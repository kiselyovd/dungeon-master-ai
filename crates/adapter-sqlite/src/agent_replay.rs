//! Read-only recovery of previously resolved player tools. Never prepares or executes a new effect.
use crate::SqliteStore;
use app_application::{
    agent::{
        commands::AgentToolCommand,
        tool_dispatch::{ToolDispatchRequest, ToolExecution},
    },
    models::{
        adventure::AdventureCommand, campaign_state::CampaignSessionState,
        combat::CombatProjection, identity::CampaignId, rest::RestCommand,
    },
    ports::repositories::RepositoryError,
};
use serde_json::{json, Value};
use sqlx::Row;
fn error(code: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation: "replay_player_tool",
        code,
    }
}
impl SqliteStore {
    pub async fn replay_player_tool(
        &self,
        request: &ToolDispatchRequest,
    ) -> Result<Option<ToolExecution>, RepositoryError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|_| error("tool_replay_begin_failed"))?;
        let result = replay_in_transaction(&mut tx, request).await?;
        tx.commit()
            .await
            .map_err(|_| error("tool_replay_commit_failed"))?;
        Ok(result)
    }
    /// Repair only durable delivery metadata; canonical effects are never executed.
    pub async fn recover_player_tool(
        &self,
        request: &ToolDispatchRequest,
    ) -> Result<Option<ToolExecution>, RepositoryError> {
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| error("tool_replay_begin_failed"))?;
        let result = replay_in_transaction(&mut tx, request).await?;
        if let Some(result) = &result {
            repair_checkpoint(&mut tx, request, result).await?;
        }
        tx.commit()
            .await
            .map_err(|_| error("tool_replay_commit_failed"))?;
        Ok(result)
    }
}
pub(crate) async fn repair_checkpoint(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    request: &ToolDispatchRequest,
    result: &ToolExecution,
) -> Result<(), RepositoryError> {
    let registered: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM agent_turn_operations WHERE request_id=?1)",
    )
    .bind(&request.request_id)
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| error("tool_replay_read_failed"))?;
    if !registered {
        return Ok(());
    }
    let command = json!({"tool_name":request.command.tool_name(),"args":request.command.to_args()})
        .to_string();
    let raw = serde_json::to_string(result).map_err(|_| error("tool_replay_invalid"))?;
    sqlx::query("INSERT INTO agent_tool_steps(request_id,tool_call_id,command_json,result_json) VALUES(?1,?2,?3,?4) ON CONFLICT(request_id,tool_call_id) DO UPDATE SET result_json=excluded.result_json WHERE agent_tool_steps.result_json IS NULL AND agent_tool_steps.command_json=excluded.command_json")
        .bind(&request.request_id).bind(&request.tool_call_id).bind(command).bind(raw).execute(&mut **tx).await.map_err(|_|error("tool_replay_checkpoint_failed"))?;
    Ok(())
}
pub(crate) async fn replay_in_transaction(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    request: &ToolDispatchRequest,
) -> Result<Option<ToolExecution>, RepositoryError> {
    let mut state =
        crate::campaign_state::load_in_transaction(tx, CampaignId(request.campaign_id)).await?;
    if state.session_id.0 != request.session_id || state.character_id.is_none() {
        return Err(error("tool_replay_scope_mismatch"));
    }
    let operation=sqlx::query("SELECT campaign_id,session_id,restore_generation FROM agent_turn_operations WHERE request_id=?1")
            .bind(&request.request_id).fetch_optional(&mut **tx).await.map_err(|_|error("tool_replay_read_failed"))?;
    if let Some(operation) = operation {
        if operation.get::<String, _>("campaign_id") != request.campaign_id.to_string()
            || operation.get::<String, _>("session_id") != request.session_id.to_string()
        {
            return Err(error("tool_replay_scope_mismatch"));
        }
        let generation: i64 =
            sqlx::query_scalar("SELECT restore_generation FROM sessions WHERE id=?1")
                .bind(request.session_id.to_string())
                .fetch_one(&mut **tx)
                .await
                .map_err(|_| error("tool_replay_read_failed"))?;
        if operation.get::<i64, _>("restore_generation") != generation {
            return Err(error("tool_replay_timeline_mismatch"));
        }
    }
    let key = format!("agent:{}:{}", request.request_id, request.tool_call_id);
    let expected =
        json!({"tool_name":request.command.tool_name(),"args":request.command.to_args()})
            .to_string();
    let step=sqlx::query("SELECT command_json,result_json FROM agent_tool_steps WHERE request_id=?1 AND tool_call_id=?2")
            .bind(&request.request_id).bind(&request.tool_call_id).fetch_optional(&mut **tx).await.map_err(|_|error("tool_replay_read_failed"))?;
    let pending_step_exists = step.is_some();
    let recorded = if let Some(step) = step {
        if step.get::<String, _>("command_json") != expected {
            return Err(error("tool_replay_signature_mismatch"));
        }
        step.get::<Option<String>, _>("result_json")
            .map(|raw| {
                serde_json::from_str::<ToolExecution>(&raw)
                    .map_err(|_| error("tool_replay_invalid"))
            })
            .transpose()?
    } else {
        None
    };
    if matches!(
        request.command,
        AgentToolCommand::ApplyDamage(_)
            | AgentToolCommand::ApplyHealing(_)
            | AgentToolCommand::CastSpell(_)
            | AgentToolCommand::UpdateToken(_)
            | AgentToolCommand::RemoveToken(_)
    ) {
        if recorded.is_none() && pending_step_exists {
            return Err(error("tool_replay_effect_uncertain"));
        }
        return Ok(recorded);
    }
    if let AgentToolCommand::RollDice(dice) = &request.command {
        if dice
            .check
            .as_ref()
            .is_some_and(|check| Some(check.character_id) != state.character_id)
        {
            return Err(error("tool_replay_scope_mismatch"));
        }
        return Ok(recorded);
    }
    if matches!(request.command, AgentToolCommand::EndCombat(_)) {
        if recorded.as_ref().is_some_and(|result| !result.is_error)
            && state
                .projection
                .combat
                .as_ref()
                .is_some_and(|combat| combat["snapshot"]["active"] == true)
        {
            return Err(error("tool_replay_scope_mismatch"));
        }
        return Ok(recorded);
    }
    let confirmed = match &request.command {
        AgentToolCommand::AdventureAction(action) => {
            let row=sqlx::query("SELECT request_json FROM adventure_operations WHERE campaign_id=?1 AND causation_id=?2")
                    .bind(request.campaign_id.to_string()).bind(&key).fetch_optional(&mut **tx).await.map_err(|_|error("tool_replay_read_failed"))?;
            if let Some(row) = row {
                let old: AdventureCommand =
                    serde_json::from_str(&row.get::<String, _>("request_json"))
                        .map_err(|_| error("tool_replay_invalid"))?;
                if old.campaign_id.0 != request.campaign_id
                    || old.session_id.0 != request.session_id
                    || Some(old.character_id) != state.character_id
                    || old.action != action.action
                {
                    return Err(error("tool_replay_signature_mismatch"));
                }
                true
            } else {
                false
            }
        }
        AgentToolCommand::RestAction(action) => {
            let row = sqlx::query(
                "SELECT request_json FROM rest_operations WHERE session_id=?1 AND causation_id=?2",
            )
            .bind(request.session_id.to_string())
            .bind(&key)
            .fetch_optional(&mut **tx)
            .await
            .map_err(|_| error("tool_replay_read_failed"))?;
            if let Some(row) = row {
                let old: RestCommand = serde_json::from_str(&row.get::<String, _>("request_json"))
                    .map_err(|_| error("tool_replay_invalid"))?;
                if old.campaign_id.0 != request.campaign_id
                    || old.session_id.0 != request.session_id
                    || Some(old.character_id) != state.character_id
                    || old.intent != *action
                {
                    return Err(error("tool_replay_signature_mismatch"));
                }
                true
            } else {
                false
            }
        }
        AgentToolCommand::CombatAction(action) => {
            let row=sqlx::query("SELECT r.signature,r.projection_json FROM combat_action_receipts r JOIN combat_encounters e ON e.id=r.encounter_id WHERE e.session_id=?1 AND r.request_id=?2")
                    .bind(request.session_id.to_string()).bind(&key).fetch_optional(&mut **tx).await.map_err(|_|error("tool_replay_read_failed"))?;
            if let Some(row) = row {
                let old: CombatProjection =
                    serde_json::from_str(&row.get::<String, _>("projection_json"))
                        .map_err(|_| error("tool_replay_invalid"))?;
                let signature: Value = serde_json::from_str(&row.get::<String, _>("signature"))
                    .map_err(|_| error("tool_replay_invalid"))?;
                let scope = old
                    .campaign_scope
                    .ok_or_else(|| error("tool_replay_scope_mismatch"))?;
                if scope.campaign_id != request.campaign_id
                    || scope.session_id != request.session_id
                    || Some(scope.character_id) != state.character_id
                {
                    return Err(error("tool_replay_scope_mismatch"));
                }
                let actors = old
                    .snapshot
                    .combatants
                    .iter()
                    .filter(|actor| actor.character_id.map(|id| id.0) == state.character_id)
                    .collect::<Vec<_>>();
                if actors.len() != 1
                    || signature["action"]
                        != serde_json::to_value(action.for_actor(actors[0].id))
                            .map_err(|_| error("tool_replay_invalid"))?
                {
                    return Err(error("tool_replay_signature_mismatch"));
                }
                if state.projection.combat.is_none() {
                    fill_terminal(tx, &mut state).await?;
                }
                true
            } else {
                false
            }
        }
        _ => return Err(error("tool_replay_unsupported")),
    };
    let result = if confirmed {
        let mut result = json!({"schema_version":1,"ruleset_id":"srd-5.2.1","campaign_id":state.campaign.id.0,"session_id":state.session_id.0,"character_id":state.character_id,"revision":state.campaign.revision.0,"adventure":state.projection.adventure,"inventory":state.projection.character["inventory"],"projection":state.projection.combat});
        if !matches!(request.command, AgentToolCommand::AdventureAction(_)) {
            let mut status = app_application::rest::status_for_state(&state)
                .map_err(|_| error("tool_replay_status_invalid"))?;
            status.combat_active = state
                .projection
                .combat
                .as_ref()
                .is_some_and(|combat| combat["snapshot"]["active"] == true);
            result["rest_status"] =
                serde_json::to_value(status).map_err(|_| error("tool_replay_invalid"))?;
        }
        Some(ToolExecution {
            result,
            is_error: false,
            handled_by: "engine".into(),
            media: vec![],
        })
    } else if let Some(recorded) = recorded {
        if !recorded.is_error {
            return Err(error("tool_replay_receipt_missing"));
        }
        Some(recorded)
    } else {
        None
    };
    Ok(result)
}
async fn fill_terminal(
    tx: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    state: &mut CampaignSessionState,
) -> Result<(), RepositoryError> {
    let raw:String=sqlx::query_scalar("SELECT projection FROM combat_encounters WHERE session_id=?1 AND ended_at IS NOT NULL ORDER BY rowid DESC LIMIT 1")
        .bind(state.session_id.0.to_string()).fetch_one(&mut **tx).await.map_err(|_|error("tool_replay_terminal_missing"))?;
    let mut terminal: CombatProjection =
        serde_json::from_str(&raw).map_err(|_| error("tool_replay_invalid"))?;
    if terminal.snapshot.active {
        return Err(error("tool_replay_invalid"));
    }
    if let Some(scope) = terminal.campaign_scope.as_mut() {
        scope.revision = state.campaign.revision.0;
    }
    terminal.adventure = state.projection.adventure.clone();
    for actor in &mut terminal.snapshot.combatants {
        if actor.character_id.map(|id| id.0) == state.character_id {
            let mut current = app_application::combat::hero::hero_combatant(
                state
                    .character_id
                    .ok_or_else(|| error("tool_replay_invalid"))?,
                &state.projection.character,
            )
            .map_err(|_| error("tool_replay_invalid"))?;
            current.id = actor.id;
            current.position = actor.position;
            current.budget = actor.budget.clone();
            current.withdrawn = actor.withdrawn;
            *actor = current;
        }
    }
    state.projection.combat =
        Some(serde_json::to_value(terminal).map_err(|_| error("tool_replay_invalid"))?);
    Ok(())
}
