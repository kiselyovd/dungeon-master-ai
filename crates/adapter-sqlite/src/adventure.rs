use crate::{messages::repository_error, SqliteStore};
use app_application::models::adventure::{AdventureCommand, AdventureProjection};
use app_application::models::adventure::{AdventureState, BELL_ADVENTURE_ID};
use app_application::models::campaign_state::CampaignSessionState;
use app_application::models::identity::CampaignId;
use app_application::ports::repositories::{AdventureRepository, RepositoryError};
use async_trait::async_trait;
use serde_json::json;
use sqlx::{Row, Sqlite, Transaction};
use uuid::Uuid;

fn rejected(code: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation: "validate_adventure",
        code,
    }
}

pub(crate) async fn read(
    tx: &mut Transaction<'_, Sqlite>,
    id: CampaignId,
) -> Result<Option<AdventureProjection>, RepositoryError> {
    let raw: Option<String> =
        sqlx::query_scalar("SELECT state_json FROM campaign_adventures WHERE campaign_id = ?1")
            .bind(id.0.to_string())
            .fetch_optional(&mut **tx)
            .await
            .map_err(|_| repository_error("read_adventure"))?;
    raw.map(|raw| {
        serde_json::from_str::<AdventureState>(&raw)
            .map(Into::into)
            .map_err(|_| repository_error("decode_adventure"))
    })
    .transpose()
}

pub(crate) async fn persist(
    tx: &mut Transaction<'_, Sqlite>,
    id: CampaignId,
    state: Option<&AdventureState>,
) -> Result<(), RepositoryError> {
    match state {
        Some(state) => {
            if state.package_id != BELL_ADVENTURE_ID {
                return Err(repository_error("adventure_package_unknown"));
            }
            sqlx::query("INSERT INTO campaign_adventures(campaign_id,state_json) VALUES(?1,?2) ON CONFLICT(campaign_id) DO UPDATE SET state_json=excluded.state_json")
                .bind(id.0.to_string()).bind(serde_json::to_string(state).map_err(|_|repository_error("encode_adventure"))?)
                .execute(&mut **tx).await.map_err(|_|repository_error("persist_adventure"))?;
        }
        None => {
            sqlx::query("DELETE FROM campaign_adventures WHERE campaign_id=?1")
                .bind(id.0.to_string())
                .execute(&mut **tx)
                .await
                .map_err(|_| repository_error("clear_adventure"))?;
        }
    }
    Ok(())
}

#[async_trait]
impl AdventureRepository for SqliteStore {
    async fn act_as_dm(
        &self,
        intent: app_application::models::adventure::AdventureIntent,
    ) -> Result<CampaignSessionState, RepositoryError> {
        use app_application::ports::repositories::CampaignStateRepository;
        if intent.causation_id.trim().is_empty() || intent.causation_id.len() > 256 {
            return Err(rejected("adventure_command_invalid"));
        }
        let current = self.load_current(intent.campaign_id).await?;
        if current.projection.adventure.is_none() {
            return Err(rejected("adventure_not_initialized"));
        }
        if current.session_id != intent.session_id {
            return Err(rejected("adventure_session_mismatch"));
        }
        let character_id = current
            .character_id
            .ok_or_else(|| rejected("adventure_character_mismatch"))?;
        let original:Option<String>=sqlx::query_scalar("SELECT request_json FROM adventure_operations WHERE campaign_id=?1 AND causation_id=?2")
            .bind(intent.campaign_id.0.to_string()).bind(&intent.causation_id).fetch_optional(&self.pool).await.map_err(|_|repository_error("read_adventure_intent_receipt"))?;
        let command = match original {
            Some(raw) => {
                let original: AdventureCommand = serde_json::from_str(&raw)
                    .map_err(|_| repository_error("decode_adventure_intent_receipt"))?;
                if original.campaign_id != intent.campaign_id
                    || original.session_id != intent.session_id
                    || original.character_id != character_id
                    || original.action != intent.action
                {
                    return Err(rejected("adventure_causation_mismatch"));
                }
                original
            }
            None => AdventureCommand {
                campaign_id: intent.campaign_id,
                session_id: intent.session_id,
                character_id,
                expected_revision: current.campaign.revision,
                causation_id: intent.causation_id,
                action: intent.action,
            },
        };
        self.act(command).await?;
        // A replay after a rewind acknowledges the old effect without sending
        // its obsolete inventory/scene back into a new renderer timeline.
        self.load_current(intent.campaign_id).await
    }
    async fn act(
        &self,
        command: AdventureCommand,
    ) -> Result<CampaignSessionState, RepositoryError> {
        let mut tx = self
            .pool
            .begin_with("BEGIN IMMEDIATE")
            .await
            .map_err(|_| repository_error("begin_adventure"))?;
        let state = act_in_transaction(&mut tx, command, None).await?;
        tx.commit()
            .await
            .map_err(|_| repository_error("commit_adventure"))?;
        Ok(state)
    }
}
pub(crate) async fn act_in_transaction(
    tx: &mut Transaction<'_, Sqlite>,
    command: AdventureCommand,
    actor: Option<serde_json::Value>,
) -> Result<CampaignSessionState, RepositoryError> {
    let request = serde_json::to_string(&command)
        .map_err(|_| repository_error("encode_adventure_command"))?;
    let receipt=sqlx::query("SELECT request_json,state_json FROM adventure_operations WHERE campaign_id=?1 AND causation_id=?2")
            .bind(command.campaign_id.0.to_string()).bind(&command.causation_id).fetch_optional(&mut **tx).await.map_err(|_|repository_error("read_adventure_receipt"))?;
    if let Some(row) = receipt {
        if row
            .try_get::<String, _>("request_json")
            .map_err(|_| repository_error("decode_adventure_receipt"))?
            != request
        {
            return Err(rejected("adventure_causation_mismatch"));
        }
        let state = serde_json::from_str(
            &row.try_get::<String, _>("state_json")
                .map_err(|_| repository_error("decode_adventure_receipt"))?,
        )
        .map_err(|_| repository_error("decode_adventure_receipt"))?;
        return Ok(state);
    }
    let mut current = crate::campaign_state::load_in_transaction(tx, command.campaign_id).await?;
    if let Some(actor) = actor {
        current.character_id = Some(command.character_id);
        current.projection.character = actor;
    }
    if current.session_id != command.session_id {
        return Err(rejected("adventure_session_mismatch"));
    }
    if current.character_id != Some(command.character_id) {
        return Err(rejected("adventure_character_mismatch"));
    }
    if current.campaign.revision != command.expected_revision {
        return Err(RepositoryError::Conflict {
            expected: command.expected_revision,
            actual: current.campaign.revision,
        });
    }
    if current.projection.combat.is_some()
        || crate::rest::active_combat(tx, command.session_id).await?
    {
        return Err(rejected("adventure_combat_active"));
    }
    if current.projection.character["hp"].as_i64().unwrap_or(0) <= 0 {
        return Err(rejected("adventure_character_downed"));
    }
    let adventure = current
        .projection
        .adventure
        .ok_or_else(|| rejected("adventure_not_initialized"))?;
    let mut outcome = adventure
        .state
        .apply(command.action)
        .map_err(|_| rejected("adventure_action_unavailable"))?;
    if command.action == app_application::models::adventure::Action::StartDefense {
        outcome.state.defense_encounter_id = Some(Uuid::new_v4());
    }
    if outcome.gold_reward > 0 {
        let mut character = current.projection.character;
        let items = character["inventory"]
            .as_array_mut()
            .ok_or_else(|| repository_error("character_document_invalid"))?;
        items.push(json!({"id":"bell-rescue-gold","name":"Gold pieces","count":outcome.gold_reward,"kind":"coin"}));
        crate::party::persist_sheet(tx, command.campaign_id, command.character_id, character)
            .await?;
    }
    persist(tx, command.campaign_id, Some(&outcome.state)).await?;
    crate::rest::advance_clock(tx, command.session_id, command.action.elapsed_seconds()).await?;
    crate::campaigns::advance_session_revision(
        tx,
        command.campaign_id,
        command.session_id,
        command.expected_revision,
        "adventure_action",
        &command.causation_id,
    )
    .await?;
    if command.action == app_application::models::adventure::Action::StartDefense {
        let encounter = outcome
            .state
            .defense_encounter_id
            .ok_or_else(|| repository_error("authored_encounter_missing"))?;
        crate::combat::create_in_transaction(
            tx,
            command.session_id.0,
            app_application::combat::authored::bell_defense(encounter),
            false,
        )
        .await?;
    }
    let state = crate::campaign_state::load_in_transaction(tx, command.campaign_id).await?;
    sqlx::query("INSERT INTO adventure_operations(campaign_id,causation_id,request_json,state_json) VALUES(?1,?2,?3,?4)")
            .bind(command.campaign_id.0.to_string()).bind(&command.causation_id).bind(request)
            .bind(serde_json::to_string(&state).map_err(|_|repository_error("encode_adventure_receipt"))?).execute(&mut **tx).await.map_err(|_|repository_error("record_adventure_receipt"))?;
    Ok(state)
}
