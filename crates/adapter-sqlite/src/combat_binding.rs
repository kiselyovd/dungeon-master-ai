use crate::messages::repository_error;
use app_application::models::{
    combat::{CombatCampaignScope, CombatProjection},
    identity::{CampaignId, Revision, SessionId},
};
use app_application::ports::repositories::RepositoryError;
use sqlx::{Row, Sqlite, Transaction};
use uuid::Uuid;

pub(crate) async fn recover_loadouts(
    tx: &mut Transaction<'_, Sqlite>,
    projection: &mut CombatProjection,
) -> Result<(), RepositoryError> {
    let Some(scope) = &projection.campaign_scope else {
        return Ok(());
    };
    if projection.snapshot.combatants.iter().all(|actor| {
        actor.loadout.is_some() && (actor.character_id.is_none() || actor.casting.is_some())
    }) {
        return Ok(());
    }
    let raw: Option<String> = sqlx::query_scalar("SELECT cc.character_json FROM campaign_characters cc JOIN sessions s ON s.campaign_id=cc.campaign_id WHERE cc.campaign_id=?1 AND cc.character_id=?2 AND s.id=?3")
        .bind(scope.campaign_id.to_string()).bind(scope.character_id.to_string()).bind(scope.session_id.to_string()).fetch_optional(&mut **tx).await.map_err(|_|repository_error("read_legacy_combat_sheet"))?;
    let sheet = serde_json::from_str(&raw.ok_or_else(|| repository_error("combat_hero_missing"))?)
        .map_err(|_| repository_error("decode_legacy_combat_sheet"))?;
    app_application::combat::hero::recover_bound_loadouts(scope.character_id, &sheet, projection)
        .map_err(repository_error)
}

pub(crate) async fn bind_start(
    tx: &mut Transaction<'_, Sqlite>,
    session_id: Uuid,
    projection: &mut CombatProjection,
    advance_revision: bool,
) -> Result<(), RepositoryError> {
    let campaign:Option<String>=sqlx::query_scalar("SELECT s.campaign_id FROM sessions s JOIN campaign_characters cc ON cc.campaign_id=s.campaign_id WHERE s.id=?1")
        .bind(session_id.to_string()).fetch_optional(&mut **tx).await.map_err(|_|repository_error("read_combat_hero_campaign"))?;
    let Some(campaign) = campaign else {
        return Ok(());
    };
    let campaign_id = CampaignId(
        Uuid::parse_str(&campaign).map_err(|_| repository_error("combat_campaign_invalid"))?,
    );
    let current = crate::campaign_state::load_in_transaction(tx, campaign_id).await?;
    if current.session_id.0 != session_id {
        return Err(repository_error("combat_session_mismatch"));
    }
    let active: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM combat_encounters WHERE session_id=?1 AND ended_at IS NULL)",
    )
    .bind(session_id.to_string())
    .fetch_one(&mut **tx)
    .await
    .map_err(|_| repository_error("read_active_combat"))?;
    if active {
        return Err(repository_error("combat_already_active"));
    }
    let character_id = current
        .character_id
        .ok_or_else(|| repository_error("combat_hero_missing"))?;
    app_application::combat::hero::bind_hero(
        character_id,
        &current.projection.character,
        projection,
        Uuid::new_v4().as_u128() as u64,
    )
    .map_err(repository_error)?;
    let hero = projection
        .snapshot
        .combatants
        .iter()
        .find(|actor| actor.character_id.is_some_and(|id| id.0 == character_id))
        .ok_or_else(|| repository_error("combat_hero_missing"))?;
    let mut sheet = current.projection.character.clone();
    apply_hero_resources(&mut sheet, hero)?;
    crate::campaign_state::persist_character(tx, campaign_id, character_id, sheet).await?;
    projection.campaign_scope = Some(if advance_revision {
        advance(
            tx,
            campaign_id,
            session_id,
            character_id,
            &format!("combat:{}:start", projection.encounter_id),
        )
        .await?
    } else {
        CombatCampaignScope {
            campaign_id: campaign_id.0,
            session_id,
            character_id,
            revision: current.campaign.revision.0,
        }
    });
    Ok(())
}

pub(crate) async fn sync_hero(
    tx: &mut Transaction<'_, Sqlite>,
    projection: &mut CombatProjection,
) -> Result<(), RepositoryError> {
    let heroes = projection
        .snapshot
        .combatants
        .iter()
        .filter(|actor| actor.character_id.is_some())
        .collect::<Vec<_>>();
    if heroes.is_empty() {
        if projection.campaign_scope.is_some() {
            return Err(repository_error("combat_hero_binding_missing"));
        }
        return Ok(());
    }
    if heroes.len() != 1 {
        return Err(repository_error("combat_multiple_hero_bindings"));
    }
    let hero = heroes[0];
    let character_id = hero
        .character_id
        .ok_or_else(|| repository_error("combat_hero_missing"))?
        .0;
    let row=sqlx::query("SELECT ce.session_id,ce.round,s.campaign_id,cc.character_json FROM combat_encounters ce JOIN sessions s ON s.id=ce.session_id JOIN campaign_characters cc ON cc.campaign_id=s.campaign_id AND cc.character_id=?2 WHERE ce.id=?1")
        .bind(projection.encounter_id.to_string()).bind(character_id.to_string()).fetch_optional(&mut **tx).await.map_err(|_|repository_error("read_bound_combat_sheet"))?.ok_or_else(||repository_error("combat_hero_identity_mismatch"))?;
    let session_id = Uuid::parse_str(&row.get::<String, _>("session_id"))
        .map_err(|_| repository_error("combat_session_invalid"))?;
    let campaign_id = CampaignId(
        Uuid::parse_str(&row.get::<String, _>("campaign_id"))
            .map_err(|_| repository_error("combat_campaign_invalid"))?,
    );
    let mut sheet: serde_json::Value =
        serde_json::from_str(&row.get::<String, _>("character_json"))
            .map_err(|_| repository_error("decode_bound_combat_sheet"))?;
    if hero.current_hp < 0
        || hero.current_hp > hero.max_hp
        || sheet["hpMax"].as_i64() != Some(hero.max_hp as i64)
    {
        return Err(repository_error("combat_hero_stats_changed"));
    }
    apply_hero_resources(&mut sheet, hero)?;
    let previous_round = u32::try_from(row.get::<i64, _>("round"))
        .map_err(|_| repository_error("combat_round_invalid"))?;
    let elapsed =
        app_application::combat::projection::elapsed_seconds(previous_round, &projection.snapshot);
    crate::rest::advance_clock(tx, SessionId(session_id), elapsed).await?;
    crate::campaign_state::persist_character(tx, campaign_id, character_id, sheet).await?;
    projection.campaign_scope = Some(
        advance(
            tx,
            campaign_id,
            session_id,
            character_id,
            &format!("combat:{}:{}", projection.encounter_id, projection.revision),
        )
        .await?,
    );
    Ok(())
}

fn apply_hero_resources(
    sheet: &mut serde_json::Value,
    hero: &app_application::models::combat::Combatant,
) -> Result<(), RepositoryError> {
    sheet["hp"] = serde_json::json!(hero.current_hp);
    if let Some(casting) = &hero.casting {
        if !sheet
            .get("combatResources")
            .is_some_and(serde_json::Value::is_object)
        {
            sheet["combatResources"] = serde_json::json!({});
        }
        sheet["combatResources"]["spellSlots"] = serde_json::to_value(
            casting
                .slots
                .iter()
                .map(|(level, slots)| (level.to_string(), slots.remaining))
                .collect::<std::collections::BTreeMap<_, _>>(),
        )
        .map_err(|_| repository_error("encode_spell_slots"))?;
    }
    if let Some(loadout) = &hero.loadout {
        if !sheet
            .get("combatResources")
            .is_some_and(serde_json::Value::is_object)
        {
            sheet["combatResources"] = serde_json::json!({});
        }
        sheet["combatResources"]["arrows"] = serde_json::json!(loadout.arrows);
        sheet["combatResources"]["quantities"] = serde_json::to_value(&loadout.quantities)
            .map_err(|_| repository_error("encode_combat_resources"))?;
    }
    Ok(())
}

async fn advance(
    tx: &mut Transaction<'_, Sqlite>,
    campaign_id: CampaignId,
    session_id: Uuid,
    character_id: Uuid,
    cause: &str,
) -> Result<CombatCampaignScope, RepositoryError> {
    let revision: i64 =
        sqlx::query_scalar("SELECT revision FROM sessions WHERE id=?1 AND campaign_id=?2")
            .bind(session_id.to_string())
            .bind(campaign_id.0.to_string())
            .fetch_one(&mut **tx)
            .await
            .map_err(|_| repository_error("read_combat_campaign_revision"))?;
    let revision = u64::try_from(revision)
        .map_err(|_| repository_error("combat_campaign_revision_invalid"))?;
    crate::campaigns::advance_session_revision(
        tx,
        campaign_id,
        SessionId(session_id),
        Revision(revision),
        "combat_changed",
        cause,
    )
    .await?;
    Ok(CombatCampaignScope {
        campaign_id: campaign_id.0,
        session_id,
        character_id,
        revision: revision
            .checked_add(1)
            .ok_or_else(|| repository_error("combat_campaign_revision_invalid"))?,
    })
}
