use crate::messages::repository_error;
use app_application::models::{
    combat::{CombatCampaignScope, CombatProjection},
    identity::{CampaignId, Revision, SessionId},
};
use app_application::ports::repositories::RepositoryError;
use sqlx::{Row, Sqlite, Transaction};
use uuid::Uuid;

type Sheets = std::collections::BTreeMap<Uuid, serde_json::Value>;
async fn sheets(
    tx: &mut Transaction<'_, Sqlite>,
    campaign: CampaignId,
    session: Uuid,
    starting: bool,
) -> Result<(Uuid, Sheets), RepositoryError> {
    // Raw rows avoid capture_projection -> recover_loadouts recursion.
    let root=sqlx::query("SELECT cc.character_id,cc.character_json FROM campaign_characters cc JOIN sessions s ON s.campaign_id=cc.campaign_id WHERE cc.campaign_id=?1 AND s.id=?2 AND s.id=(SELECT latest.id FROM sessions latest WHERE latest.campaign_id=?1 ORDER BY CAST(latest.created_at AS INTEGER) DESC,latest.id DESC LIMIT 1)")
        .bind(campaign.0.to_string()).bind(session.to_string()).fetch_optional(&mut **tx).await.map_err(|_|repository_error("read_combat_party"))?.ok_or_else(||repository_error("combat_hero_missing"))?;
    let root_id = Uuid::parse_str(&root.get::<String, _>("character_id"))
        .map_err(|_| repository_error("combat_hero_identity_mismatch"))?;
    let mut result = Sheets::new();
    let mut rows = vec![root];
    let present = if starting {
        crate::party::active_members(tx, campaign, SessionId(session)).await?
    } else {
        None
    };
    let retained=sqlx::query("SELECT member_id,character_id,character_json FROM party_characters WHERE campaign_id=?1 ORDER BY rowid").bind(campaign.0.to_string()).fetch_all(&mut **tx).await.map_err(|_|repository_error("read_combat_party"))?;
    for row in retained {
        let member = Uuid::parse_str(&row.get::<String, _>("member_id"))
            .map_err(|_| repository_error("combat_hero_identity_mismatch"))?;
        if present
            .as_ref()
            .is_none_or(|members| members.contains(&member))
        {
            rows.push(row);
        }
    }
    for row in rows {
        let id = Uuid::parse_str(&row.get::<String, _>("character_id"))
            .map_err(|_| repository_error("combat_hero_identity_mismatch"))?;
        let mut sheet: serde_json::Value =
            serde_json::from_str(&row.get::<String, _>("character_json"))
                .map_err(|_| repository_error("decode_combat_party"))?;
        if id.is_nil()
            || !app_application::models::campaign_state::valid_character_document(&sheet)
            || !app_application::models::campaign_state::within_character_budget(&sheet)
        {
            return Err(repository_error("combat_hero_invalid"));
        }
        sheet["characterId"] = serde_json::json!(id);
        if result.insert(id, sheet).is_some() {
            return Err(repository_error("combat_hero_identity_mismatch"));
        }
    }
    if result.len() > app_application::models::party::MAX_PARTY_MEMBERS {
        return Err(repository_error("combat_party_invalid"));
    }
    Ok((root_id, result))
}
fn retain_bound_sheets(projection: &CombatProjection, sheets: &mut Sheets) {
    let bound = projection
        .snapshot
        .combatants
        .iter()
        .filter_map(|actor| actor.character_id.map(|id| id.0))
        .collect::<std::collections::BTreeSet<_>>();
    sheets.retain(|id, _| bound.contains(id));
}
fn verify_bindings(
    projection: &CombatProjection,
    sheets: &Sheets,
    root: Uuid,
) -> Result<(), RepositoryError> {
    let bound = projection
        .snapshot
        .combatants
        .iter()
        .filter_map(|actor| actor.character_id.map(|id| id.0))
        .collect::<Vec<_>>();
    let identities = bound
        .iter()
        .copied()
        .collect::<std::collections::BTreeSet<_>>();
    if !identities.contains(&root)
        || bound.len() != sheets.len()
        || identities != sheets.keys().copied().collect()
    {
        return Err(repository_error("combat_hero_identity_mismatch"));
    }
    Ok(())
}
pub(crate) async fn recover_loadouts(
    tx: &mut Transaction<'_, Sqlite>,
    projection: &mut CombatProjection,
) -> Result<(), RepositoryError> {
    let Some(scope) = projection.campaign_scope.clone() else {
        return Ok(());
    };
    let (root, mut sheets) =
        sheets(tx, CampaignId(scope.campaign_id), scope.session_id, false).await?;
    if root != scope.character_id {
        return Err(repository_error("combat_hero_identity_mismatch"));
    }
    retain_bound_sheets(projection, &mut sheets);
    verify_bindings(projection, &sheets, root)?;
    app_application::combat::hero::recover_party_loadouts(
        &sheets
            .iter()
            .map(|(id, sheet)| (*id, sheet))
            .collect::<Vec<_>>(),
        projection,
    )
    .map_err(repository_error)
}
pub(crate) async fn bind_start(
    tx: &mut Transaction<'_, Sqlite>,
    session_id: Uuid,
    projection: &mut CombatProjection,
    advance_revision: bool,
) -> Result<(), RepositoryError> {
    let campaign:Option<String>=sqlx::query_scalar("SELECT s.campaign_id FROM sessions s JOIN campaign_characters cc ON cc.campaign_id=s.campaign_id WHERE s.id=?1").bind(session_id.to_string()).fetch_optional(&mut **tx).await.map_err(|_|repository_error("read_combat_hero_campaign"))?;
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
    let (root, mut sheets) = sheets(tx, campaign_id, session_id, true).await?;
    let mut ordered = vec![(root, &sheets[&root])];
    ordered.extend(
        sheets
            .iter()
            .filter(|(id, _)| **id != root)
            .map(|(id, sheet)| (*id, sheet)),
    );
    app_application::combat::hero::bind_party(
        &ordered,
        projection,
        Uuid::new_v4().as_u128() as u64,
    )
    .map_err(repository_error)?;
    let now = crate::rest::read_clock(tx, SessionId(session_id))
        .await?
        .unwrap_or_default()
        .elapsed_seconds;
    for hero in projection
        .snapshot
        .combatants
        .iter()
        .filter(|actor| actor.character_id.is_some())
    {
        let id = hero.character_id.unwrap().0;
        let mut sheet = sheets
            .remove(&id)
            .ok_or_else(|| repository_error("combat_hero_identity_mismatch"))?;
        apply_hero_resources(&mut sheet, hero, now)?;
        crate::party::persist_sheet(tx, campaign_id, id, sheet).await?;
    }
    projection.campaign_scope = Some(if advance_revision {
        advance(
            tx,
            campaign_id,
            session_id,
            root,
            &format!("combat:{}:start", projection.encounter_id),
        )
        .await?
    } else {
        CombatCampaignScope {
            campaign_id: campaign_id.0,
            session_id,
            character_id: root,
            revision: current.campaign.revision.0,
        }
    });
    Ok(())
}
pub(crate) async fn sync_opening_heroes(
    tx: &mut Transaction<'_, Sqlite>,
    projection: &CombatProjection,
) -> Result<(), RepositoryError> {
    let Some(scope) = &projection.campaign_scope else {
        return Ok(());
    };
    let campaign = CampaignId(scope.campaign_id);
    let (root, mut sheets) = sheets(tx, campaign, scope.session_id, false).await?;
    if root != scope.character_id {
        return Err(repository_error("combat_hero_identity_mismatch"));
    }
    retain_bound_sheets(projection, &mut sheets);
    verify_bindings(projection, &sheets, root)?;
    let now = crate::rest::read_clock(tx, SessionId(scope.session_id))
        .await?
        .unwrap_or_default()
        .elapsed_seconds;
    for actor in &projection.snapshot.combatants {
        let Some(id) = actor.character_id else {
            continue;
        };
        let mut sheet = sheets
            .remove(&id.0)
            .ok_or_else(|| repository_error("combat_hero_identity_mismatch"))?;
        if sheet["hpMax"].as_i64() != Some(i64::from(actor.max_hp)) {
            return Err(repository_error("combat_hero_stats_changed"));
        }
        apply_hero_resources(&mut sheet, actor, now)?;
        crate::party::persist_sheet(tx, campaign, id.0, sheet).await?;
    }
    Ok(())
}
pub(crate) async fn sync_hero(
    tx: &mut Transaction<'_, Sqlite>,
    projection: &mut CombatProjection,
) -> Result<(), RepositoryError> {
    if !projection
        .snapshot
        .combatants
        .iter()
        .any(|actor| actor.character_id.is_some())
    {
        if projection.campaign_scope.is_some() {
            return Err(repository_error("combat_hero_binding_missing"));
        }
        return Ok(());
    }
    let row=sqlx::query("SELECT ce.session_id,ce.round,s.campaign_id FROM combat_encounters ce JOIN sessions s ON s.id=ce.session_id WHERE ce.id=?1").bind(projection.encounter_id.to_string()).fetch_optional(&mut **tx).await.map_err(|_|repository_error("read_bound_combat_sheet"))?.ok_or_else(||repository_error("combat_hero_identity_mismatch"))?;
    let session = Uuid::parse_str(&row.get::<String, _>("session_id"))
        .map_err(|_| repository_error("combat_session_invalid"))?;
    let campaign = CampaignId(
        Uuid::parse_str(&row.get::<String, _>("campaign_id"))
            .map_err(|_| repository_error("combat_campaign_invalid"))?,
    );
    let (root, mut sheets) = sheets(tx, campaign, session, false).await?;
    retain_bound_sheets(projection, &mut sheets);
    verify_bindings(projection, &sheets, root)?;
    let previous_round = u32::try_from(row.get::<i64, _>("round"))
        .map_err(|_| repository_error("combat_round_invalid"))?;
    crate::rest::advance_combat_clock(
        tx,
        SessionId(session),
        app_application::combat::projection::elapsed_seconds(previous_round, &projection.snapshot),
    )
    .await?;
    let now = crate::rest::read_clock(tx, SessionId(session))
        .await?
        .unwrap_or_default()
        .elapsed_seconds;
    for hero in projection
        .snapshot
        .combatants
        .iter()
        .filter(|actor| actor.character_id.is_some())
    {
        let id = hero.character_id.unwrap().0;
        let mut sheet = sheets
            .remove(&id)
            .ok_or_else(|| repository_error("combat_hero_identity_mismatch"))?;
        if hero.current_hp < 0
            || hero.current_hp > hero.max_hp
            || sheet["hpMax"].as_i64() != Some(hero.max_hp as i64)
        {
            return Err(repository_error("combat_hero_stats_changed"));
        }
        apply_hero_resources(&mut sheet, hero, now)?;
        crate::party::persist_sheet(tx, campaign, id, sheet).await?;
    }
    projection.campaign_scope = Some(
        advance(
            tx,
            campaign,
            session,
            root,
            &format!("combat:{}:{}", projection.encounter_id, projection.revision),
        )
        .await?,
    );
    Ok(())
}

fn apply_hero_resources(
    sheet: &mut serde_json::Value,
    hero: &app_application::models::combat::Combatant,
    now: u64,
) -> Result<(), RepositoryError> {
    app_application::vitality::store_actor(sheet, hero).map_err(repository_error)?;
    let id = hero
        .character_id
        .ok_or_else(|| repository_error("combat_hero_binding_missing"))?
        .0;
    let vitality = app_application::vitality::read(sheet, id)
        .map_err(repository_error)?
        .ok_or_else(|| repository_error("hero_vitality_invalid"))?;
    if hero.current_hp == 0
        && hero.death_saves.stable
        && !hero.is_dead
        && vitality.natural_recovery.is_none()
    {
        let deadline = app_application::recovery::RecoveryEngine::random()
            .natural_deadline(hero, now)
            .map_err(repository_error)?;
        app_application::vitality::schedule_natural_recovery(sheet, id, deadline)
            .map_err(repository_error)?;
    }
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
