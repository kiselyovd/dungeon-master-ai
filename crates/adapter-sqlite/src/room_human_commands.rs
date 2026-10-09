//! Owner/lease-scoped human progress, separate from old room tool signatures.
use crate::SqliteStore;
use app_application::{
    agent::{human_commands::*, operations::AgentOperation},
    models::{combat::CombatProjection, room::RoomLease, room_agent::PreparedRoomTool},
    ports::repositories::RepositoryError,
};
use sqlx::{Row, Sqlite, Transaction};
use uuid::Uuid;
fn error(code: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation: "room_human_commands",
        code,
    }
}
/// Called after canonical replay, before a new effect in the same transaction.
/// Unbound manual/legacy nonces retain their existing repository contracts.
pub(crate) async fn require_pending_binding(
    store: &SqliteStore,
    tx: &mut Transaction<'_, Sqlite>,
    lease: &RoomLease,
    nonce: &str,
    action: &HumanAction,
    revision: u64,
    encounter: Option<Uuid>,
) -> Result<(), RepositoryError> {
    let row = sqlx::query("SELECT b.turn_id,b.tool_id,t.process_owner FROM room_human_command_bindings b JOIN room_dm_turns t ON t.room_id=b.room_id AND t.member_id=b.member_id AND t.turn_id=b.turn_id WHERE b.room_id=?1 AND b.member_id=?2 AND b.nonce=?3")
        .bind(lease.room_id().to_string()).bind(lease.member_id().to_string()).bind(nonce)
        .fetch_optional(&mut **tx).await.map_err(|_|error("human_command_read_failed"))?;
    let Some(row) = row else {
        // The old preparation API may still serve legacy turns, but must not
        // manufacture an unbound tool for a turn that already has a manifest.
        let bound_turn: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM room_agent_tools t JOIN room_human_command_manifests m ON m.room_id=t.room_id AND m.member_id=t.member_id AND m.turn_id=t.turn_id WHERE t.room_id=?1 AND t.member_id=?2 AND json_extract(t.prepared_json,'$.nonce')=?3)")
            .bind(lease.room_id().to_string()).bind(lease.member_id().to_string()).bind(nonce)
            .fetch_one(&mut **tx).await.map_err(|_|error("human_command_read_failed"))?;
        return if bound_turn {
            Err(error("human_command_binding_missing"))
        } else {
            Ok(())
        };
    };
    let parse = |name| {
        Uuid::parse_str(&row.get::<String, _>(name))
            .map_err(|_| error("human_command_data_invalid"))
    };
    let journal = RoomHumanCommandLedger::new(
        store.clone(),
        lease.clone(),
        parse("turn_id")?,
        parse("process_owner")?,
    );
    let (scope, message) = journal.guard(tx).await?;
    let ledger = journal
        .load(tx, &scope, &message)
        .await?
        .ok_or_else(|| error("human_command_manifest_missing"))?;
    let tool = row.get::<String, _>("tool_id");
    let binding = ledger
        .bindings
        .iter()
        .find(|binding| binding.tool_call_id == tool)
        .ok_or_else(|| error("human_command_binding_missing"))?;
    if binding.signature != *action
        || ledger
            .receipts
            .iter()
            .any(|receipt| receipt.binding == *binding)
    {
        return Err(error("human_command_binding_conflict"));
    }
    let raw:String = sqlx::query_scalar("SELECT prepared_json FROM room_agent_tools WHERE room_id=?1 AND member_id=?2 AND turn_id=?3 AND tool_id=?4")
        .bind(lease.room_id().to_string()).bind(lease.member_id().to_string()).bind(journal.turn_id.to_string()).bind(tool).fetch_one(&mut **tx).await.map_err(|_|error("human_command_read_failed"))?;
    let prepared: PreparedRoomTool =
        serde_json::from_str(&raw).map_err(|_| error("human_command_data_invalid"))?;
    if prepared.expected_revision != revision
        || matches!(action, HumanAction::Combat(_)) && prepared.encounter_id != encounter
    {
        return Err(error("human_command_binding_conflict"));
    }
    Ok(())
}
#[derive(Clone)]
pub struct RoomHumanCommandLedger {
    store: SqliteStore,
    lease: RoomLease,
    turn_id: Uuid,
    owner_id: Uuid,
}
impl RoomHumanCommandLedger {
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
        let (scope, message) = self.guard(&mut tx).await?;
        let mut ledger = self
            .load(&mut tx, &scope, &message)
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
    pub fn new(store: SqliteStore, lease: RoomLease, turn_id: Uuid, owner_id: Uuid) -> Self {
        Self {
            store,
            lease,
            turn_id,
            owner_id,
        }
    }
    async fn guard(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
    ) -> Result<(CommandScope, String), RepositoryError> {
        if self.turn_id.is_nil() || self.owner_id.is_nil() {
            return Err(error("human_command_scope_invalid"));
        }
        crate::room_agent::guard(tx, &self.lease).await?;
        let reset = crate::room_agent::timeline(tx, &self.lease, self.turn_id).await?;
        let row=sqlx::query("SELECT campaign_id,room_epoch,reset_cursor,request_json FROM room_dm_turns WHERE room_id=?1 AND member_id=?2 AND turn_id=?3 AND process_owner=?4 AND status='running'").bind(self.lease.room_id().to_string()).bind(self.lease.member_id().to_string()).bind(self.turn_id.to_string()).bind(self.owner_id.to_string()).fetch_optional(&mut **tx).await.map_err(|_|error("human_command_read_failed"))?.ok_or_else(||error("human_command_not_owned"))?;
        let input: AgentOperation = serde_json::from_str(&row.get::<String, _>("request_json"))
            .map_err(|_| error("human_command_data_invalid"))?;
        let (_, _, actor, _) = crate::room_world::context(tx, &self.lease).await?;
        if input.request_id != self.turn_id
            || input.request.campaign_id != self.lease.campaign_id().0
            || input.request.session_id != self.lease.session_id().0
            || row.get::<String, _>("campaign_id") != input.request.campaign_id.to_string()
            || row.get::<i64, _>("room_epoch")
                != i64::try_from(self.lease.epoch())
                    .map_err(|_| error("human_command_scope_invalid"))?
            || row.get::<i64, _>("reset_cursor") != reset
        {
            return Err(error("human_command_superseded"));
        }
        Ok((
            CommandScope {
                request_id: self.turn_id,
                campaign_id: input.request.campaign_id,
                session_id: input.request.session_id,
                actor_id: actor,
                authority: CommandAuthority::Room {
                    room_id: self.lease.room_id(),
                    member_id: self.lease.member_id(),
                    epoch: self.lease.epoch(),
                    reset_cursor: u64::try_from(reset)
                        .map_err(|_| error("human_command_data_invalid"))?,
                },
            },
            input.request.player_message,
        ))
    }
    async fn current_token(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        scope: &CommandScope,
    ) -> Result<(Option<Uuid>, Option<Uuid>, u64), RepositoryError> {
        let (_, _, _, state) = crate::room_world::context(tx, &self.lease).await?;
        if let Some(value) = state
            .projection
            .combat
            .filter(|value| value["snapshot"]["active"] == true)
        {
            let projection: CombatProjection =
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
            Ok((
                Some(projection.encounter_id),
                Some(actors[0].id.0),
                projection.revision,
            ))
        } else {
            Ok((None, None, state.campaign.revision.0))
        }
    }
    async fn load(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        scope: &CommandScope,
        message: &str,
    ) -> Result<Option<StoredHumanCommandLedger>, RepositoryError> {
        let raw:Option<String>=sqlx::query_scalar("SELECT manifest_json FROM room_human_command_manifests WHERE room_id=?1 AND member_id=?2 AND turn_id=?3").bind(self.lease.room_id().to_string()).bind(self.lease.member_id().to_string()).bind(self.turn_id.to_string()).fetch_optional(&mut **tx).await.map_err(|_|error("human_command_read_failed"))?;
        let Some(raw) = raw else {
            return Ok(None);
        };
        let manifest: HumanCommandManifest =
            serde_json::from_str(&raw).map_err(|_| error("human_command_data_invalid"))?;
        manifest
            .validate_structure(message, scope)
            .map_err(|_| error("human_command_manifest_invalid"))?;
        let rows=sqlx::query("SELECT b.tool_id,b.command_id,b.step,b.nonce,b.binding_json,b.actor_token,b.refusal_code,t.prepared_json FROM room_human_command_bindings b JOIN room_agent_tools t ON t.room_id=b.room_id AND t.member_id=b.member_id AND t.turn_id=b.turn_id AND t.tool_id=b.tool_id WHERE b.room_id=?1 AND b.member_id=?2 AND b.turn_id=?3 ORDER BY b.command_id,b.step").bind(self.lease.room_id().to_string()).bind(self.lease.member_id().to_string()).bind(self.turn_id.to_string()).fetch_all(&mut **tx).await.map_err(|_|error("human_command_read_failed"))?;
        let mut bindings = vec![];
        let mut receipts = vec![];
        for row in rows {
            let binding: PreparedCommandBinding =
                serde_json::from_str(&row.get::<String, _>("binding_json"))
                    .map_err(|_| error("human_command_data_invalid"))?;
            let prepared: PreparedRoomTool =
                serde_json::from_str(&row.get::<String, _>("prepared_json"))
                    .map_err(|_| error("human_command_data_invalid"))?;
            let nonce = Uuid::parse_str(&prepared.nonce)
                .map_err(|_| error("human_command_data_invalid"))?;
            if binding.scope != *scope
                || binding.tool_call_id != row.get::<String, _>("tool_id")
                || binding.command_id as i64 != row.get::<i64, _>("command_id")
                || binding.step as i64 != row.get::<i64, _>("step")
                || binding.canonical_nonce != Some(nonce)
                || prepared.nonce != row.get::<String, _>("nonce")
            {
                return Err(error("human_command_binding_invalid"));
            }
            let result = crate::room_agent::replay_in_transaction(
                tx,
                &self.lease,
                self.turn_id,
                &binding.tool_call_id,
                binding.signature.to_tool(),
            )
            .await?;
            if let Some(result) = result {
                if row.get::<Option<String>, _>("refusal_code").is_some() {
                    return Err(error("human_command_binding_invalid"));
                }
                receipts.push(ConfirmedCommandAttempt {
                    binding: binding.clone(),
                    outcome: CommandAttemptOutcome::Performed {
                        check_success: result.result["success"].as_bool(),
                    },
                });
            } else if let Some(code) = row.get::<Option<String>, _>("refusal_code") {
                if !app_application::agent::intentions::definitive_rejection(&code) {
                    return Err(error("human_command_outcome_uncertain"));
                }
                receipts.push(ConfirmedCommandAttempt {
                    binding: binding.clone(),
                    outcome: CommandAttemptOutcome::Refused { code },
                });
            } else {
                self.check_pending_context(
                    tx,
                    scope,
                    &binding,
                    &prepared,
                    row.get::<Option<String>, _>("actor_token"),
                )
                .await?;
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
    async fn check_pending_context(
        &self,
        tx: &mut Transaction<'_, Sqlite>,
        scope: &CommandScope,
        binding: &PreparedCommandBinding,
        prepared: &PreparedRoomTool,
        actor_token: Option<String>,
    ) -> Result<(), RepositoryError> {
        let (encounter, token, combat_revision) = self.current_token(tx, scope).await?;
        if matches!(binding.signature, HumanAction::Combat(_)) {
            if prepared.encounter_id != encounter
                || prepared.expected_revision != combat_revision
                || actor_token != token.map(|id| id.to_string())
            {
                return Err(error("human_command_context_stale"));
            }
        } else {
            let (_, _, _, current) = crate::room_world::context(tx, &self.lease).await?;
            if prepared.expected_revision != current.campaign.revision.0 {
                return Err(error("human_command_context_stale"));
            }
        }
        Ok(())
    }
    pub async fn record_definitive_refusal(
        &self,
        tool_id: &str,
        code: &str,
    ) -> Result<(), RepositoryError> {
        if !app_application::agent::intentions::definitive_rejection(code) {
            return Err(error("human_command_outcome_uncertain"));
        }
        let mut tx = self
            .store
            .pool()
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| error("human_command_begin_failed"))?;
        let (scope, message) = self.guard(&mut tx).await?;
        let ledger = self
            .load(&mut tx, &scope, &message)
            .await?
            .ok_or_else(|| error("human_command_manifest_missing"))?;
        let binding = ledger
            .bindings
            .iter()
            .find(|binding| binding.tool_call_id == tool_id)
            .ok_or_else(|| error("human_command_binding_missing"))?;
        if let Some(receipt) = ledger
            .receipts
            .iter()
            .find(|receipt| receipt.binding == *binding)
        {
            if receipt.outcome != (CommandAttemptOutcome::Refused { code: code.into() }) {
                return Err(error("human_command_binding_conflict"));
            }
        }
        sqlx::query("UPDATE room_human_command_bindings SET refusal_code=?5 WHERE room_id=?1 AND member_id=?2 AND turn_id=?3 AND tool_id=?4 AND (refusal_code IS NULL OR refusal_code=?5)").bind(self.lease.room_id().to_string()).bind(self.lease.member_id().to_string()).bind(self.turn_id.to_string()).bind(tool_id).bind(code).execute(&mut *tx).await.map_err(|_|error("human_command_write_failed"))?;
        tx.commit()
            .await
            .map_err(|_| error("human_command_commit_failed"))?;
        Ok(())
    }
}
#[async_trait::async_trait]
impl HumanCommandLedgerRepository for RoomHumanCommandLedger {
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
        self.record_definitive_refusal(tool_id, code).await
    }
    async fn original_request(&self) -> Result<(CommandScope, String), RepositoryError> {
        let mut tx = self
            .store
            .pool()
            .begin()
            .await
            .map_err(|_| error("human_command_begin_failed"))?;
        let input = self.guard(&mut tx).await?;
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
        let (scope, message) = self.guard(&mut tx).await?;
        let result = self.load(&mut tx, &scope, &message).await?;
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
        let (scope, message) = self.guard(&mut tx).await?;
        manifest
            .validate_structure(&message, &scope)
            .map_err(|_| error("human_command_manifest_invalid"))?;
        if let Some(existing) = self.load(&mut tx, &scope, &message).await? {
            if existing.manifest != manifest {
                return Err(error("human_command_manifest_conflict"));
            }
            tx.commit()
                .await
                .map_err(|_| error("human_command_commit_failed"))?;
            return Ok(existing);
        }
        let prepared:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM room_agent_tools WHERE room_id=?1 AND member_id=?2 AND turn_id=?3)").bind(self.lease.room_id().to_string()).bind(self.lease.member_id().to_string()).bind(self.turn_id.to_string()).fetch_one(&mut *tx).await.map_err(|_|error("human_command_read_failed"))?;
        if prepared {
            return Err(error("human_command_legacy_unbound"));
        }
        sqlx::query("INSERT INTO room_human_command_manifests(room_id,member_id,turn_id,manifest_json) VALUES(?1,?2,?3,?4)").bind(self.lease.room_id().to_string()).bind(self.lease.member_id().to_string()).bind(self.turn_id.to_string()).bind(serde_json::to_string(&manifest).map_err(|_|error("human_command_data_invalid"))?).execute(&mut *tx).await.map_err(|_|error("human_command_write_failed"))?;
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
        let (scope, message) = self.guard(&mut tx).await?;
        let ledger = self
            .load(&mut tx, &scope, &message)
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
        let orphan:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM room_agent_tools WHERE room_id=?1 AND member_id=?2 AND turn_id=?3 AND tool_id=?4)").bind(self.lease.room_id().to_string()).bind(self.lease.member_id().to_string()).bind(self.turn_id.to_string()).bind(&proposal.tool_call_id).fetch_one(&mut *tx).await.map_err(|_|error("human_command_read_failed"))?;
        if orphan {
            return Err(error("human_command_legacy_unbound"));
        }
        let prepared = crate::room_agent::prepare_in_transaction(
            &mut tx,
            &self.lease,
            self.turn_id,
            &proposal.tool_call_id,
            proposal.signature.to_tool(),
        )
        .await?;
        let (_, token, _) = self.current_token(&mut tx, &scope).await?;
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
            canonical_nonce: Some(
                Uuid::parse_str(&prepared.nonce)
                    .map_err(|_| error("human_command_data_invalid"))?,
            ),
        };
        ledger
            .manifest
            .prepare_binding(&binding, &ledger.bindings, &ledger.receipts)
            .map_err(|_| error("human_command_binding_invalid"))?;
        sqlx::query("INSERT INTO room_human_command_bindings(room_id,member_id,turn_id,tool_id,command_id,step,nonce,binding_json,actor_token) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9)").bind(self.lease.room_id().to_string()).bind(self.lease.member_id().to_string()).bind(self.turn_id.to_string()).bind(&binding.tool_call_id).bind(i64::from(binding.command_id)).bind(i64::from(binding.step)).bind(prepared.nonce).bind(serde_json::to_string(&binding).map_err(|_|error("human_command_data_invalid"))?).bind(token.map(|id|id.to_string())).execute(&mut *tx).await.map_err(|_|error("human_command_write_failed"))?;
        tx.commit()
            .await
            .map_err(|_| error("human_command_commit_failed"))?;
        Ok(binding)
    }
}
