use crate::{
    models::{
        campaign_state::{valid_character_document, CampaignSessionState},
        rest::{RestCommand, RestResult, RestStatus},
    },
    ports::repositories::{RepositoryError, RestRepository},
};
use app_domain::{
    combat::types::ability_modifier,
    rest::{RestInput, RestResources},
};
use std::sync::Arc;
pub struct RestCampaign<R>(Arc<R>);
impl<R: RestRepository> RestCampaign<R> {
    pub fn new(repository: Arc<R>) -> Self {
        Self(repository)
    }
    #[tracing::instrument(skip_all,fields(campaign_id=%command.campaign_id.0,causation_id=%command.causation_id,expected_revision=command.expected_revision.0))]
    pub async fn execute(&self, command: RestCommand) -> Result<RestResult, RepositoryError> {
        if command.campaign_id.0.is_nil()
            || command.session_id.0.is_nil()
            || command.character_id.is_nil()
            || command.causation_id.trim().is_empty()
            || command.causation_id.len() > 256
        {
            return Err(rejected("rest_intention_invalid"));
        }
        self.0
            .rest(command, uuid::Uuid::new_v4().as_u128() as u64)
            .await
    }
    pub async fn status(
        &self,
        id: crate::models::identity::CampaignId,
    ) -> Result<RestStatus, RepositoryError> {
        self.0.status(id).await
    }
}
fn rejected(code: &'static str) -> RepositoryError {
    RepositoryError::Operation {
        operation: "rest",
        code,
    }
}

pub fn resolve_for_state(
    state: &CampaignSessionState,
    intent: crate::models::rest::RestIntent,
    rng_seed: u64,
) -> Result<crate::models::rest::RestOutcome, RepositoryError> {
    app_domain::rest::resolve_rest(
        input_for_state(state)?,
        intent,
        &mut app_domain::rng::SeededRng::from_seed(rng_seed),
    )
    .map_err(|error| {
        rejected(match error {
            app_domain::rest::RestError::Unsafe => "rest_unsafe",
            app_domain::rest::RestError::Downed => "rest_character_downed",
            app_domain::rest::RestError::TooSoon => "rest_too_soon",
            app_domain::rest::RestError::Invalid => "rest_intention_invalid",
        })
    })
}

pub fn input_for_state(state: &CampaignSessionState) -> Result<RestInput, RepositoryError> {
    let sheet = &state.projection.character;
    crate::vitality::read(
        sheet,
        state
            .character_id
            .ok_or_else(|| rejected("rest_identity_mismatch"))?,
    )
    .map_err(rejected)?;
    if state.character_id.is_none() || !valid_character_document(sheet) {
        return Err(rejected("rest_character_required"));
    }
    let class = sheet["heroClass"]
        .as_str()
        .ok_or_else(|| rejected("rest_character_invalid"))?;
    let level = sheet["level"]
        .as_u64()
        .ok_or_else(|| rejected("rest_character_invalid"))? as u8;
    let resources = sheet.get("combatResources");
    let hit_dice_remaining = match resources.and_then(|r| r.get("hitDiceRemaining")) {
        None => level,
        Some(value) => u8::try_from(
            value
                .as_u64()
                .ok_or_else(|| rejected("rest_resources_invalid"))?,
        )
        .map_err(|_| rejected("rest_resources_invalid"))?
        .min(level),
    };
    let last = resources
        .and_then(|r| r.get("lastLongRestCompletedSeconds"))
        .filter(|v| !v.is_null())
        .map(|v| v.as_u64().ok_or_else(|| rejected("rest_resources_invalid")))
        .transpose()?;
    let casting = crate::combat::hero::casting_from_sheet(sheet).map_err(rejected)?;
    let elapsed_seconds = state
        .projection
        .adventuring
        .as_ref()
        .map_or(0, |state| state.elapsed_seconds);
    Ok(RestInput {
        class_id: class.into(),
        level,
        hp: sheet["hp"].as_i64().unwrap() as i32,
        max_hp: sheet["hpMax"].as_i64().unwrap() as i32,
        constitution_modifier: ability_modifier(sheet["abilities"]["con"].as_i64().unwrap() as i32),
        elapsed_seconds,
        resources: RestResources {
            hit_dice_remaining,
            last_long_rest_completed: last,
            spell_slots: casting
                .slots
                .into_iter()
                .map(|(level, slots)| (level, slots.remaining))
                .collect(),
        },
        safe: state.projection.adventure.as_ref().is_none_or(|adventure| {
            adventure.state.location == app_domain::adventure::Location::Town
        }),
        combat_active: state.projection.combat.is_some(),
    })
}

pub fn status_for_state(state: &CampaignSessionState) -> Result<RestStatus, RepositoryError> {
    let input = input_for_state(state)?;
    let hit_die = app_domain::compendium::compendium()
        .classes
        .iter()
        .find(|class| class.id == input.class_id)
        .ok_or_else(|| rejected("rest_character_invalid"))?
        .hit_die as u8;
    let slots = crate::combat::hero::casting_from_sheet(&state.projection.character)
        .map_err(rejected)?
        .slots;
    let ready = input.resources.last_long_rest_completed.map_or(0, |last| {
        last.saturating_add(16 * 60 * 60)
            .saturating_sub(input.elapsed_seconds)
    });
    Ok(RestStatus {
        campaign_id: state.campaign.id.0,
        session_id: state.session_id.0,
        character_id: state.character_id.unwrap(),
        revision: state.campaign.revision.0,
        hp: input.hp,
        max_hp: input.max_hp,
        hit_die,
        hit_dice_remaining: input.resources.hit_dice_remaining,
        slots,
        elapsed_seconds: input.elapsed_seconds,
        long_rest_ready_in_seconds: ready,
        safe: input.safe,
        combat_active: input.combat_active,
    })
}

pub fn apply_rest_to_sheet(
    sheet: &mut serde_json::Value,
    outcome: &app_domain::rest::RestOutcome,
) -> Result<(), &'static str> {
    crate::vitality::clear_after_healing(sheet, outcome.hp)?;
    sheet["hp"] = serde_json::json!(outcome.hp);
    if !sheet
        .get("combatResources")
        .is_some_and(serde_json::Value::is_object)
    {
        sheet["combatResources"] = serde_json::json!({});
    }
    sheet["combatResources"]["hitDiceRemaining"] =
        serde_json::json!(outcome.resources.hit_dice_remaining);
    sheet["combatResources"]["lastLongRestCompletedSeconds"] =
        serde_json::json!(outcome.resources.last_long_rest_completed);
    sheet["combatResources"]["spellSlots"] = serde_json::json!(outcome.resources.spell_slots);
    Ok(())
}
