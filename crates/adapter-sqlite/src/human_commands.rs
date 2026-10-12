//! Solo command metadata shares the protected operation and canonical replay path.
use crate::SqliteStore;
use app_application::{
    agent::{human_commands::*, operations::AgentOperation, tool_dispatch::ToolDispatchRequest},
    models::identity::CampaignId,
    ports::repositories::RepositoryError,
};
use sqlx::{Row, Sqlite, Transaction};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SoloPreparationStamp {
    pub campaign_revision: u64,
    pub combat: Option<SoloCombatStamp>,
}
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct SoloCombatStamp {
    pub encounter_id: Uuid,
    pub revision: u64,
    pub actor_token: Uuid,
}
pub(crate) struct SoloPreparedExecution {
    pub binding: PreparedCommandBinding,
    pub stamp: SoloPreparationStamp,
}
pub(crate) async fn has_bound_manifest(
    tx: &mut Transaction<'_, Sqlite>,
    causation: &str,
) -> Result<bool, RepositoryError> {
    let Some((request, _)) = causation
        .strip_prefix("agent:")
        .and_then(|rest| rest.split_once(':'))
    else {
        return Ok(false);
    };
    let request = Uuid::parse_str(request).map_err(|_| error("human_command_scope_invalid"))?;
    sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM solo_human_command_manifests WHERE request_id=?1)",
    )
    .bind(request.to_string())
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| error("human_command_read_failed"))
}
async fn current_stamp(
    tx: &mut Transaction<'_, Sqlite>,
    scope: &CommandScope,
) -> Result<SoloPreparationStamp, RepositoryError> {
    let state =
        crate::campaign_state::load_in_transaction(tx, CampaignId(scope.campaign_id)).await?;
    let combat = if let Some(value) = state
        .projection
        .combat
        .filter(|value| value["snapshot"]["active"] == true)
    {
        let projection: app_application::models::combat::CombatProjection =
            serde_json::from_value(value).map_err(|_| error("human_command_data_invalid"))?;
        let actors = projection
            .snapshot
            .combatants
            .iter()
            .filter(|actor| actor.character_id.map(|id| id.0) == Some(scope.actor_id))
            .collect::<Vec<_>>();
        if actors.len() != 1 {
            return Err(error("human_command_scope_invalid"));
        }
        Some(SoloCombatStamp {
            encounter_id: projection.encounter_id,
            revision: projection.revision,
            actor_token: actors[0].id.0,
        })
    } else {
        None
    };
    Ok(SoloPreparationStamp {
        campaign_revision: state.campaign.revision.0,
        combat,
    })
}

/// For new effects only, after canonical receipt lookup and inside the effect's
/// write transaction. No binding means legacy behavior unless a manifest exists.
pub(crate) async fn require_prepared_context(
    tx: &mut Transaction<'_, Sqlite>,
    causation: &str,
    campaign: Uuid,
    session: Uuid,
) -> Result<Option<SoloPreparedExecution>, RepositoryError> {
    let Some(rest) = causation.strip_prefix("agent:") else {
        return Ok(None);
    };
    let Some((request, call)) = rest.split_once(':') else {
        return Ok(None);
    };
    let request_id = Uuid::parse_str(request).map_err(|_| error("human_command_scope_invalid"))?;
    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM solo_human_command_manifests WHERE request_id=?1)",
    )
    .bind(request_id.to_string())
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| error("human_command_read_failed"))?;
    if !exists {
        return Ok(None);
    }
    let owner: String = sqlx::query_scalar(
        "SELECT owner_id FROM agent_turn_operations WHERE request_id=?1 AND status='running'",
    )
    .bind(request_id.to_string())
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| error("human_command_read_failed"))?
    .ok_or_else(|| error("human_command_not_owned"))?;
    let owner = Uuid::parse_str(&owner).map_err(|_| error("human_command_data_invalid"))?;
    let (scope, message) = guard_operation(tx, request_id, owner).await?;
    if scope.campaign_id != campaign || scope.session_id != session {
        return Err(error("human_command_scope_invalid"));
    }
    let row=sqlx::query("SELECT binding_json,preparation_json FROM solo_human_command_bindings WHERE request_id=?1 AND tool_call_id=?2").bind(request_id.to_string()).bind(call).fetch_optional(&mut **tx).await.map_err(|_|error("human_command_read_failed"))?.ok_or_else(||error("human_command_binding_missing"))?;
    let binding: PreparedCommandBinding =
        serde_json::from_str(&row.get::<String, _>("binding_json"))
            .map_err(|_| error("human_command_data_invalid"))?;
    let stamp: SoloPreparationStamp =
        serde_json::from_str(&row.get::<String, _>("preparation_json"))
            .map_err(|_| error("human_command_data_invalid"))?;
    let manifest_raw: String = sqlx::query_scalar(
        "SELECT manifest_json FROM solo_human_command_manifests WHERE request_id=?1",
    )
    .bind(request_id.to_string())
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| error("human_command_read_failed"))?;
    let manifest: HumanCommandManifest =
        serde_json::from_str(&manifest_raw).map_err(|_| error("human_command_data_invalid"))?;
    manifest
        .validate_structure(&message, &scope)
        .map_err(|_| error("human_command_manifest_invalid"))?;
    let ledger = load_ledger(tx, &scope, &message)
        .await?
        .ok_or_else(|| error("human_command_manifest_missing"))?;
    if !ledger.bindings.contains(&binding) {
        return Err(error("human_command_binding_invalid"));
    }
    if ledger
        .receipts
        .iter()
        .any(|receipt| receipt.binding == binding)
    {
        return Err(error("human_command_already_finished"));
    }
    let generation: i64 = sqlx::query_scalar(
        "SELECT restore_generation FROM agent_turn_operations WHERE request_id=?1",
    )
    .bind(request_id.to_string())
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| error("human_command_read_failed"))?;
    let current_generation: i64 =
        sqlx::query_scalar("SELECT restore_generation FROM sessions WHERE id=?1")
            .bind(binding.scope.session_id.to_string())
            .fetch_one(&mut **tx)
            .await
            .map_err(|_| error("human_command_read_failed"))?;
    let state =
        crate::campaign_state::load_in_transaction(tx, CampaignId(binding.scope.campaign_id))
            .await?;
    if owner.is_nil()
        || binding.scope.request_id != request_id
        || binding.tool_call_id != call
        || binding.scope.campaign_id != campaign
        || binding.scope.session_id != session
        || generation != current_generation
        || state.session_id.0 != binding.scope.session_id
        || state.character_id != Some(binding.scope.actor_id)
    {
        return Err(error("human_command_scope_invalid"));
    }
    if current_stamp(tx, &binding.scope).await? != stamp {
        return Err(error("human_command_context_stale"));
    }
    Ok(Some(SoloPreparedExecution { binding, stamp }))
}
fn error(code: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation: "human_command_ledger",
        code,
    }
}
impl SqliteStore {
    /// Read only the latest solo operation in the current accepted actor/timeline.
    /// Room-owned state is available solely through authenticated room services.
    pub async fn pending_solo_clarification(
        &self,
        campaign_id: Uuid,
        session_id: Uuid,
    ) -> Result<Option<PendingHumanCommandClarification>, RepositoryError> {
        self.pending_solo_operation(campaign_id, session_id, false)
            .await
    }

    pub async fn solo_recovery_input(
        &self,
        campaign_id: Uuid,
        session_id: Uuid,
    ) -> Result<Option<PendingHumanCommandClarification>, RepositoryError> {
        self.pending_solo_operation(campaign_id, session_id, true)
            .await
    }

    async fn pending_solo_operation(
        &self,
        campaign_id: Uuid,
        session_id: Uuid,
        include_unplanned: bool,
    ) -> Result<Option<PendingHumanCommandClarification>, RepositoryError> {
        let mut tx = self
            .pool()
            .begin()
            .await
            .map_err(|_| error("human_command_begin_failed"))?;
        let current =
            crate::campaign_state::load_in_transaction(&mut tx, CampaignId(campaign_id)).await?;
        if current.session_id.0 != session_id || current.character_id.is_none() {
            return Ok(None);
        }
        let room: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM coop_rooms WHERE campaign_id=?1 AND active=1)",
        )
        .bind(campaign_id.to_string())
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| error("human_command_read_failed"))?;
        if room {
            return Ok(None);
        }
        let row=sqlx::query("SELECT o.request_id,o.request_json,o.status,o.restore_generation,o.actor_id,m.manifest_json,m.clarifications_json,m.final_narration_json FROM agent_turn_operations o LEFT JOIN solo_human_command_manifests m ON m.request_id=o.request_id WHERE o.campaign_id=?1 AND o.session_id=?2 ORDER BY o.updated_at DESC,o.created_at DESC,o.rowid DESC LIMIT 1").bind(campaign_id.to_string()).bind(session_id.to_string()).fetch_optional(&mut *tx).await.map_err(|_|error("human_command_read_failed"))?;
        let Some(row) = row else {
            return Ok(None);
        };
        if !matches!(
            row.get::<String, _>("status").as_str(),
            "running" | "interrupted"
        ) {
            return Ok(None);
        }
        let generation: i64 =
            sqlx::query_scalar("SELECT restore_generation FROM sessions WHERE id=?1")
                .bind(session_id.to_string())
                .fetch_one(&mut *tx)
                .await
                .map_err(|_| error("human_command_read_failed"))?;
        if generation != row.get::<i64, _>("restore_generation") {
            return Ok(None);
        }
        let operation: AgentOperation = serde_json::from_str(&row.get::<String, _>("request_json"))
            .map_err(|_| error("human_command_data_invalid"))?;
        if row.get::<Option<String>, _>("actor_id") != current.character_id.map(|id| id.to_string())
        {
            return Ok(None);
        }
        if operation.request_id.to_string() != row.get::<String, _>("request_id")
            || operation.request.campaign_id != campaign_id
            || operation.request.session_id != session_id
        {
            return Err(error("human_command_scope_invalid"));
        }
        let Some(raw) = row.get::<Option<String>, _>("manifest_json") else {
            if !include_unplanned {
                return Ok(None);
            }
            tx.commit()
                .await
                .map_err(|_| error("human_command_commit_failed"))?;
            return Ok(Some(PendingHumanCommandClarification {
                operation,
                clarification: None,
            }));
        };
        let manifest: HumanCommandManifest =
            serde_json::from_str(&raw).map_err(|_| error("human_command_data_invalid"))?;
        let scope = CommandScope {
            request_id: operation.request_id,
            campaign_id,
            session_id,
            actor_id: current
                .character_id
                .ok_or_else(|| error("human_command_actor_required"))?,
            authority: CommandAuthority::Solo {
                restore_generation: u64::try_from(generation)
                    .map_err(|_| error("human_command_data_invalid"))?,
            },
        };
        if operation.request_id.to_string() != row.get::<String, _>("request_id")
            || operation.request.campaign_id != campaign_id
            || operation.request.session_id != session_id
            || manifest.scope != scope
        {
            return Ok(None);
        }
        manifest
            .validate_structure(&operation.request.player_message, &scope)
            .map_err(|_| error("human_command_manifest_invalid"))?;
        let questions: Vec<HumanCommandClarification> = serde_json::from_str(
            &row.get::<Option<String>, _>("clarifications_json")
                .ok_or_else(|| error("human_command_data_invalid"))?,
        )
        .map_err(|_| error("human_command_data_invalid"))?;
        let clarification = questions.last().cloned();
        if clarification.as_ref().is_some_and(|clarification| {
            clarification.id.is_nil()
                || clarification.question.trim().is_empty()
                || !manifest.commands.iter().any(|command| {
                    command.id == clarification.command_id
                        && (clarification.reply.is_some()
                            || matches!(command.goal, HumanCommandGoal::NeedsResolution { .. }))
                })
        }) {
            return Err(error("human_command_data_invalid"));
        }
        tx.commit()
            .await
            .map_err(|_| error("human_command_commit_failed"))?;
        Ok(Some(PendingHumanCommandClarification {
            operation,
            clarification,
        }))
    }
}

async fn guard_operation(
    tx: &mut Transaction<'_, Sqlite>,
    request_id: Uuid,
    owner_id: Uuid,
) -> Result<(CommandScope, String), RepositoryError> {
    if request_id.is_nil() || owner_id.is_nil() {
        return Err(error("human_command_scope_invalid"));
    }
    let row=sqlx::query("SELECT campaign_id,session_id,request_json,restore_generation FROM agent_turn_operations WHERE request_id=?1 AND owner_id=?2 AND status='running'").bind(request_id.to_string()).bind(owner_id.to_string()).fetch_optional(&mut **tx).await.map_err(|_|error("human_command_read_failed"))?.ok_or_else(||error("human_command_not_owned"))?;
    let input: AgentOperation = serde_json::from_str(&row.get::<String, _>("request_json"))
        .map_err(|_| error("human_command_data_invalid"))?;
    if input.request_id != request_id
        || input.request.campaign_id.to_string() != row.get::<String, _>("campaign_id")
        || input.request.session_id.to_string() != row.get::<String, _>("session_id")
    {
        return Err(error("human_command_scope_invalid"));
    }
    let state =
        crate::campaign_state::load_in_transaction(tx, CampaignId(input.request.campaign_id))
            .await?;
    if state.session_id.0 != input.request.session_id {
        return Err(error("human_command_scope_invalid"));
    }
    let actor_id = state
        .character_id
        .ok_or_else(|| error("human_command_actor_required"))?;
    let generation: i64 = sqlx::query_scalar("SELECT restore_generation FROM sessions WHERE id=?1")
        .bind(input.request.session_id.to_string())
        .fetch_one(&mut **tx)
        .await
        .map_err(|_| error("human_command_read_failed"))?;
    if generation != row.get::<i64, _>("restore_generation") {
        return Err(error("human_command_superseded"));
    }
    let room_active: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM coop_rooms WHERE campaign_id=?1 AND active=1)",
    )
    .bind(input.request.campaign_id.to_string())
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| error("human_command_read_failed"))?;
    if room_active {
        return Err(error("human_command_room_required"));
    }
    Ok((
        CommandScope {
            request_id,
            campaign_id: input.request.campaign_id,
            session_id: input.request.session_id,
            actor_id,
            authority: CommandAuthority::Solo {
                restore_generation: u64::try_from(generation)
                    .map_err(|_| error("human_command_data_invalid"))?,
            },
        },
        input.request.player_message,
    ))
}

async fn load_ledger(
    tx: &mut Transaction<'_, Sqlite>,
    scope: &CommandScope,
    message: &str,
) -> Result<Option<StoredHumanCommandLedger>, RepositoryError> {
    load_ledger_mode(tx, scope, message, true).await
}

pub(crate) async fn read_ledger(
    tx: &mut Transaction<'_, Sqlite>,
    scope: &CommandScope,
    message: &str,
) -> Result<Option<StoredHumanCommandLedger>, RepositoryError> {
    load_ledger_mode(tx, scope, message, false).await
}

async fn load_ledger_mode(
    tx: &mut Transaction<'_, Sqlite>,
    scope: &CommandScope,
    message: &str,
    repair_delivery: bool,
) -> Result<Option<StoredHumanCommandLedger>, RepositoryError> {
    let raw: Option<String> = sqlx::query_scalar(
        "SELECT manifest_json FROM solo_human_command_manifests WHERE request_id=?1",
    )
    .bind(scope.request_id.to_string())
    .fetch_optional(&mut **tx)
    .await
    .map_err(|_| error("human_command_read_failed"))?;
    let Some(raw) = raw else {
        return Ok(None);
    };
    let manifest: HumanCommandManifest =
        serde_json::from_str(&raw).map_err(|_| error("human_command_data_invalid"))?;
    manifest
        .validate_structure(message, scope)
        .map_err(|_| error("human_command_manifest_invalid"))?;
    let rows=sqlx::query("SELECT b.tool_call_id,b.command_id,b.step,b.binding_json,b.preparation_json,s.command_json FROM solo_human_command_bindings b JOIN agent_tool_steps s ON s.request_id=b.request_id AND s.tool_call_id=b.tool_call_id WHERE b.request_id=?1 ORDER BY b.command_id,b.step").bind(scope.request_id.to_string()).fetch_all(&mut **tx).await.map_err(|_|error("human_command_read_failed"))?;
    let mut bindings = Vec::new();
    let mut receipts = Vec::new();
    for row in rows {
        let binding: PreparedCommandBinding =
            serde_json::from_str(&row.get::<String, _>("binding_json"))
                .map_err(|_| error("human_command_data_invalid"))?;
        let stamp: SoloPreparationStamp =
            serde_json::from_str(&row.get::<String, _>("preparation_json"))
                .map_err(|_| error("human_command_data_invalid"))?;
        if binding.scope != *scope
            || binding.tool_call_id != row.get::<String, _>("tool_call_id")
            || i64::from(binding.command_id) != row.get::<i64, _>("command_id")
            || i64::from(binding.step) != row.get::<i64, _>("step")
            || binding.canonical_nonce.is_some()
        {
            return Err(error("human_command_binding_invalid"));
        }
        let command = binding.signature.to_tool();
        let signature =
            serde_json::json!({"tool_name":command.tool_name(),"args":command.to_args()})
                .to_string();
        if signature != row.get::<String, _>("command_json") {
            return Err(error("human_command_signature_mismatch"));
        }
        let request = ToolDispatchRequest {
            request_id: scope.request_id.to_string(),
            tool_call_id: binding.tool_call_id.clone(),
            campaign_id: scope.campaign_id,
            session_id: scope.session_id,
            command,
        };
        if let Some(result) = crate::agent_replay::replay_in_transaction(tx, &request).await? {
            if !result.is_error && matches!(binding.signature, HumanAction::Combat(_)) {
                let expected = stamp
                    .combat
                    .as_ref()
                    .ok_or_else(|| error("human_command_context_invalid"))?;
                let receipt=sqlx::query("SELECT r.signature FROM combat_action_receipts r JOIN combat_encounters e ON e.id=r.encounter_id WHERE e.session_id=?1 AND r.encounter_id=?2 AND r.request_id=?3").bind(scope.session_id.to_string()).bind(expected.encounter_id.to_string()).bind(format!("agent:{}:{}",scope.request_id,binding.tool_call_id)).fetch_optional(&mut **tx).await.map_err(|_|error("human_command_read_failed"))?.ok_or_else(||error("human_command_context_invalid"))?;
                let signature: serde_json::Value =
                    serde_json::from_str(&receipt.get::<String, _>("signature"))
                        .map_err(|_| error("human_command_data_invalid"))?;
                let HumanAction::Combat(action) = &binding.signature else {
                    unreachable!()
                };
                if signature
                    != serde_json::json!({"encounter_id":expected.encounter_id,"expected_revision":expected.revision,"action":action.for_actor_id(expected.actor_token)})
                {
                    return Err(error("human_command_context_invalid"));
                }
            }
            let outcome = if result.is_error {
                let code = result.result["error"]
                    .as_str()
                    .filter(|code| app_application::agent::intentions::definitive_rejection(code))
                    .ok_or_else(|| error("human_command_outcome_uncertain"))?;
                CommandAttemptOutcome::Refused { code: code.into() }
            } else {
                CommandAttemptOutcome::Performed {
                    check_success: result.result["success"].as_bool(),
                }
            };
            if repair_delivery {
                crate::agent_replay::repair_checkpoint(tx, &request, &result).await?;
            }
            receipts.push(ConfirmedCommandAttempt {
                binding: binding.clone(),
                outcome,
            });
        } else if current_stamp(tx, scope).await? != stamp {
            return Err(error("human_command_context_stale"));
        }
        bindings.push(binding);
    }
    manifest
        .progress(&bindings, &receipts)
        .map_err(|_| error("human_command_binding_invalid"))?;
    let ledger = StoredHumanCommandLedger {
        manifest,
        bindings,
        receipts,
        clarifications: crate::human_command_clarifications::load(tx, scope).await?,
        final_narration: crate::human_command_clarifications::load_final(tx, scope).await?,
    };
    ledger
        .validate_clarifications()
        .map_err(|_| error("human_command_data_invalid"))?;
    Ok(Some(ledger))
}
#[derive(Clone)]
pub struct SoloHumanCommandLedger {
    store: SqliteStore,
    request_id: Uuid,
    owner_id: Uuid,
}
impl SoloHumanCommandLedger {
    async fn update_metadata<T: Send>(
        &self,
        change: impl FnOnce(&mut StoredHumanCommandLedger) -> Result<T, CommandLedgerError> + Send,
    ) -> Result<T, RepositoryError> {
        let mut tx = self
            .store
            .pool()
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| error("human_command_begin_failed"))?;
        let (scope, message) = guard_operation(&mut tx, self.request_id, self.owner_id).await?;
        let mut ledger = load_ledger(&mut tx, &scope, &message)
            .await?
            .ok_or_else(|| error("human_command_manifest_missing"))?;
        let result =
            change(&mut ledger).map_err(|_| error("human_command_clarification_invalid"))?;
        crate::human_command_clarifications::persist(&mut tx, &message, &ledger).await?;
        tx.commit()
            .await
            .map_err(|_| error("human_command_commit_failed"))?;
        Ok(result)
    }
    pub fn new(store: SqliteStore, request_id: Uuid, owner_id: Uuid) -> Self {
        Self {
            store,
            request_id,
            owner_id,
        }
    }
}
#[async_trait::async_trait]
impl HumanCommandLedgerRepository for SoloHumanCommandLedger {
    async fn record_final_narration(
        &self,
        text: String,
    ) -> Result<HumanCommandFinalNarration, RepositoryError> {
        self.update_metadata(move |ledger| ledger.record_final(text))
            .await
    }
    async fn record_clarification(
        &self,
        id: u16,
        question: String,
    ) -> Result<HumanCommandClarification, RepositoryError> {
        self.update_metadata(move |ledger| ledger.record_question(id, question))
            .await
    }
    async fn submit_clarification_reply(
        &self,
        id: Uuid,
        reply: String,
    ) -> Result<(), RepositoryError> {
        self.update_metadata(move |ledger| ledger.record_reply(id, reply))
            .await
    }
    async fn resolve_verified_goal(
        &self,
        id: u16,
        action: HumanAction,
    ) -> Result<(), RepositoryError> {
        self.update_metadata(move |ledger| ledger.resolve_goal(id, action))
            .await
    }
    async fn record_refusal(&self, tool_id: &str, code: &str) -> Result<(), RepositoryError> {
        let ledger = self
            .load_current()
            .await?
            .ok_or_else(|| error("human_command_manifest_missing"))?;
        if ledger.receipts.iter().any(|receipt| {
            receipt.binding.tool_call_id == tool_id
                && receipt.outcome == (CommandAttemptOutcome::Refused { code: code.into() })
        }) {
            Ok(())
        } else {
            Err(error("human_command_outcome_uncertain"))
        }
    }
    async fn original_request(&self) -> Result<(CommandScope, String), RepositoryError> {
        let mut tx = self
            .store
            .pool()
            .begin()
            .await
            .map_err(|_| error("human_command_begin_failed"))?;
        let input = guard_operation(&mut tx, self.request_id, self.owner_id).await?;
        tx.commit()
            .await
            .map_err(|_| error("human_command_commit_failed"))?;
        Ok(input)
    }
    async fn load_current(&self) -> Result<Option<StoredHumanCommandLedger>, RepositoryError> {
        let mut tx = self
            .store
            .pool()
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| error("human_command_begin_failed"))?;
        let (scope, message) = guard_operation(&mut tx, self.request_id, self.owner_id).await?;
        let result = load_ledger(&mut tx, &scope, &message).await?;
        tx.commit()
            .await
            .map_err(|_| error("human_command_commit_failed"))?;
        Ok(result)
    }
    async fn save_verified_manifest(
        &self,
        manifest: HumanCommandManifest,
    ) -> Result<StoredHumanCommandLedger, RepositoryError> {
        let mut tx = self
            .store
            .pool()
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| error("human_command_begin_failed"))?;
        let (scope, message) = guard_operation(&mut tx, self.request_id, self.owner_id).await?;
        manifest
            .validate_structure(&message, &scope)
            .map_err(|_| error("human_command_manifest_invalid"))?;
        if let Some(existing) = load_ledger(&mut tx, &scope, &message).await? {
            if existing.manifest != manifest {
                return Err(error("human_command_manifest_conflict"));
            }
            tx.commit()
                .await
                .map_err(|_| error("human_command_commit_failed"))?;
            return Ok(existing);
        }
        let prepared: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM agent_tool_steps WHERE request_id=?1)")
                .bind(self.request_id.to_string())
                .fetch_one(&mut *tx)
                .await
                .map_err(|_| error("human_command_read_failed"))?;
        if prepared {
            return Err(error("human_command_legacy_unbound"));
        }
        let raw =
            serde_json::to_string(&manifest).map_err(|_| error("human_command_data_invalid"))?;
        sqlx::query(
            "INSERT INTO solo_human_command_manifests(request_id,manifest_json) VALUES(?1,?2)",
        )
        .bind(self.request_id.to_string())
        .bind(raw)
        .execute(&mut *tx)
        .await
        .map_err(|_| error("human_command_write_failed"))?;
        tx.commit()
            .await
            .map_err(|_| error("human_command_commit_failed"))?;
        Ok(StoredHumanCommandLedger {
            manifest,
            bindings: vec![],
            receipts: vec![],
            clarifications: vec![],
            final_narration: None,
        })
    }
    async fn prepare_after_permission(
        &self,
        proposal: HumanCommandStepProposal,
    ) -> Result<PreparedCommandBinding, RepositoryError> {
        let mut tx = self
            .store
            .pool()
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| error("human_command_begin_failed"))?;
        let (scope, message) = guard_operation(&mut tx, self.request_id, self.owner_id).await?;
        let ledger = load_ledger(&mut tx, &scope, &message)
            .await?
            .ok_or_else(|| error("human_command_manifest_missing"))?;
        if let Some(existing) = ledger
            .bindings
            .iter()
            .find(|binding| binding.tool_call_id == proposal.tool_call_id)
        {
            if existing.command_id != proposal.command_id
                || existing.signature != proposal.signature
                || existing.terminal != proposal.terminal
            {
                return Err(error("human_command_binding_conflict"));
            }
            let existing = existing.clone();
            tx.commit()
                .await
                .map_err(|_| error("human_command_commit_failed"))?;
            return Ok(existing);
        }
        let step = ledger
            .bindings
            .iter()
            .filter(|binding| binding.command_id == proposal.command_id)
            .map(|binding| binding.step)
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| error("human_command_binding_invalid"))?;
        let binding = PreparedCommandBinding {
            scope,
            command_id: proposal.command_id,
            step,
            tool_call_id: proposal.tool_call_id,
            signature: proposal.signature,
            terminal: proposal.terminal,
            canonical_nonce: None,
        };
        let stamp = current_stamp(&mut tx, &binding.scope).await?;
        ledger
            .manifest
            .prepare_binding(&binding, &ledger.bindings, &ledger.receipts)
            .map_err(|_| error("human_command_binding_invalid"))?;
        let command = binding.signature.to_tool();
        let raw = serde_json::json!({"tool_name":command.tool_name(),"args":command.to_args()})
            .to_string();
        let orphan: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM agent_tool_steps WHERE request_id=?1 AND tool_call_id=?2)",
        )
        .bind(self.request_id.to_string())
        .bind(&binding.tool_call_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|_| error("human_command_read_failed"))?;
        if orphan {
            return Err(error("human_command_legacy_unbound"));
        }
        sqlx::query(
            "INSERT INTO agent_tool_steps(request_id,tool_call_id,command_json) VALUES(?1,?2,?3)",
        )
        .bind(self.request_id.to_string())
        .bind(&binding.tool_call_id)
        .bind(raw)
        .execute(&mut *tx)
        .await
        .map_err(|_| error("human_command_write_failed"))?;
        sqlx::query("INSERT INTO solo_human_command_bindings(request_id,tool_call_id,command_id,step,binding_json,preparation_json) VALUES(?1,?2,?3,?4,?5,?6)").bind(self.request_id.to_string()).bind(&binding.tool_call_id).bind(i64::from(binding.command_id)).bind(i64::from(binding.step)).bind(serde_json::to_string(&binding).map_err(|_|error("human_command_data_invalid"))?).bind(serde_json::to_string(&stamp).map_err(|_|error("human_command_data_invalid"))?).execute(&mut *tx).await.map_err(|_|error("human_command_write_failed"))?;
        tx.commit()
            .await
            .map_err(|_| error("human_command_commit_failed"))?;
        Ok(binding)
    }
}
