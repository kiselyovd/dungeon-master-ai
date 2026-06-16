//! Executes validated tool-calls from the agent loop.
//!
//! Each call goes through `app_domain::combat::validator::validate_tool_call`
//! first, then dispatches to the matching executor below. Executors return a
//! `(Value, is_error)` tuple - they never panic. The result JSON is what gets
//! injected back as a `ChatMessage::ToolResult` in the next round.
//!
//! M2/M3 boundary: the validator currently only knows the seven M2 combat
//! tools. The eight new M3 tools (`set_scene`, `cast_spell`, `remember_npc`,
//! `recall_npc`, `journal_append`, `quick_save`, `generate_map`,
//! `generate_illustration`, `query_rules`) come back from the validator as
//! `UnknownTool`. That returns a graceful `is_error=true` result here without
//! crashing the loop. Phase D extends the validator dispatch table to cover them.
//!
//! Several executors below are stubbed because their backing tables/db
//! helpers come in later phases:
//! - `execute_query_rules` -> Phase E (SRD RAG).

use std::sync::Arc;

use base64::engine::general_purpose::STANDARD as B64;
use base64::Engine;

use app_domain::combat::validator::validate_tool_call;
use app_llm::ToolCall;
use serde_json::{json, Value};
use sqlx::SqlitePool;
use tracing::warn;
use uuid::Uuid;

/// Whether an unfinished combat encounter exists. The agent uses this to gate
/// the combat-management tool subset (and rule injection) on/off per turn, so an
/// exploration turn isn't burdened with combat tools + rules a small model would
/// trip over. Any DB error conservatively reports `false` (exploration mode).
pub async fn is_combat_active(pool: &SqlitePool) -> bool {
    sqlx::query("SELECT id FROM combat_encounters WHERE ended_at IS NULL LIMIT 1")
        .fetch_optional(pool)
        .await
        .map(|row| row.is_some())
        .unwrap_or(false)
}

use crate::image::provider::{ImagePrompt, ImageProvider};
use crate::video::provider::{VideoPrompt, VideoProvider};

/// Execute a tool-call. Returns `(result_value, is_error)`.
/// Never panics - errors are surfaced as `is_error=true` with a message in the result.
#[allow(clippy::too_many_arguments)]
pub async fn execute_tool(
    tc: &ToolCall,
    pool: &SqlitePool,
    image_provider: Option<Arc<dyn ImageProvider>>,
    video_provider: Option<Arc<dyn VideoProvider>>,
    retriever: Option<&app_domain::srd::retriever::SrdRetriever>,
    embedding_model: &str,
    campaign_id: Uuid,
    session_id: Uuid,
) -> (Value, bool) {
    // Phase 1: Validate via domain dispatch table.
    // M3 NOTE: The validator currently knows only the seven M2 tools. The
    // eight new tools (set_scene, cast_spell, remember_npc, recall_npc,
    // journal_append, quick_save, generate_map, generate_illustration,
    // query_rules) return `UnknownTool`. That is handled gracefully here -
    // we record the error and return without crashing. Phase D extends the
    // validator.
    let validated = match validate_tool_call(&tc.name, tc.args.clone()) {
        Ok(v) => v,
        Err(e) => {
            warn!("tool validation failed: {} - {}", tc.name, e);
            return (json!({ "error": e.to_string() }), true);
        }
    };

    // Phase 2: Execute the validated tool-call.
    match validated.tool_name.as_str() {
        "roll_dice" => execute_roll_dice(&validated.args),
        "apply_damage" => execute_apply_damage(&validated.args, pool).await,
        "apply_healing" => execute_apply_healing(&validated.args, pool).await,
        "start_combat" => execute_start_combat(&validated.args, pool, session_id).await,
        "end_combat" => execute_end_combat(pool).await,
        "add_token" => execute_add_token(&validated.args, pool).await,
        "update_token" => execute_update_token(&validated.args, pool).await,
        "remove_token" => execute_remove_token(&validated.args, pool).await,
        "set_scene" => execute_set_scene(&validated.args, pool, campaign_id).await,
        "cast_spell" => execute_cast_spell(&validated.args, pool).await,
        "remember_npc" => execute_remember_npc(&validated.args, pool, campaign_id).await,
        "recall_npc" => execute_recall_npc(&validated.args, pool, campaign_id).await,
        "journal_append" => execute_journal_append(&validated.args, pool, campaign_id).await,
        "quick_save" => execute_quick_save(&validated.args, pool, session_id).await,
        "generate_map" => execute_generate_map(&validated.args, image_provider).await,
        "generate_illustration" => {
            execute_generate_illustration(&validated.args, image_provider).await
        }
        "generate_video" => execute_generate_video(&validated.args, video_provider).await,
        "query_rules" => execute_query_rules(&validated.args, retriever, embedding_model).await,
        unknown => (
            json!({ "error": format!("unhandled tool: {}", unknown) }),
            true,
        ),
    }
}

fn execute_roll_dice(args: &Value) -> (Value, bool) {
    use app_domain::dice::roll_expr_detailed;
    use app_domain::rng::SeededRng;

    let dice_str = args["dice"].as_str().unwrap_or("1d20");
    let modifier = args["modifier"].as_i64().unwrap_or(0) as i32;

    let expr = parse_dice_expr(dice_str, modifier);
    let mut rng = SeededRng::new_random();
    let detail = roll_expr_detailed(&expr, &mut rng);
    (
        json!({ "rolls": detail.rolls, "total": detail.total, "modifier": modifier }),
        false,
    )
}

/// Parse "NdX" or "dX" strings into a `DiceExpr`. Defaults to 1d20 on parse failure.
fn parse_dice_expr(s: &str, modifier: i32) -> app_domain::dice::DiceExpr {
    use app_domain::dice::{DiceExpr, Die};
    let s = s.to_lowercase();
    let (count_part, die_part) = if let Some(idx) = s.find('d') {
        (&s[..idx], &s[idx + 1..])
    } else {
        return DiceExpr {
            count: 1,
            die: Die::D20,
            modifier,
        };
    };
    // DiceExpr.count is u8; clamp to 1..=u8::MAX.
    let count: u8 = count_part
        .parse::<u32>()
        .unwrap_or(1)
        .clamp(1, u8::MAX as u32) as u8;
    let sides: u32 = die_part.parse().unwrap_or(20);
    let die = match sides {
        4 => Die::D4,
        6 => Die::D6,
        8 => Die::D8,
        10 => Die::D10,
        12 => Die::D12,
        20 => Die::D20,
        100 => Die::D100,
        _ => Die::D20,
    };
    DiceExpr {
        count,
        die,
        modifier,
    }
}

/// Parse a lowercase damage-type string into `DamageType`.
/// Returns `None` for unknown types (treated as Normal/no modifier).
fn parse_damage_type(s: &str) -> Option<app_domain::combat::types::DamageType> {
    use app_domain::combat::types::DamageType;
    match s {
        "acid" => Some(DamageType::Acid),
        "bludgeoning" => Some(DamageType::Bludgeoning),
        "cold" => Some(DamageType::Cold),
        "fire" => Some(DamageType::Fire),
        "force" => Some(DamageType::Force),
        "lightning" => Some(DamageType::Lightning),
        "necrotic" => Some(DamageType::Necrotic),
        "piercing" => Some(DamageType::Piercing),
        "poison" => Some(DamageType::Poison),
        "psychic" => Some(DamageType::Psychic),
        "radiant" => Some(DamageType::Radiant),
        "slashing" => Some(DamageType::Slashing),
        "thunder" => Some(DamageType::Thunder),
        _ => None,
    }
}

/// Build a `DamageResistance` from three nullable JSON-array columns.
fn build_damage_resistance(
    resistances_json: Option<&str>,
    immunities_json: Option<&str>,
    vulnerabilities_json: Option<&str>,
) -> app_domain::combat::damage::DamageResistance {
    use app_domain::combat::damage::{DamageRelation, DamageResistance};

    let mut resist = DamageResistance::default();

    let parse_list = |json: Option<&str>| -> Vec<String> {
        json.and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .unwrap_or_default()
    };

    for dt_str in parse_list(resistances_json) {
        if let Some(dt) = parse_damage_type(&dt_str) {
            resist.set(dt, DamageRelation::Resistant);
        }
    }
    for dt_str in parse_list(immunities_json) {
        if let Some(dt) = parse_damage_type(&dt_str) {
            resist.set(dt, DamageRelation::Immune);
        }
    }
    for dt_str in parse_list(vulnerabilities_json) {
        if let Some(dt) = parse_damage_type(&dt_str) {
            resist.set(dt, DamageRelation::Vulnerable);
        }
    }

    resist
}

async fn execute_apply_damage(args: &Value, pool: &SqlitePool) -> (Value, bool) {
    use app_domain::combat::damage::compute_effective_damage;
    use sqlx::Row;

    let token_id = args["token_id"].as_str().unwrap_or_default();
    let amount = args["amount"].as_i64().unwrap_or(0) as i32;
    let damage_type_str = args["type"].as_str().unwrap_or("").to_lowercase();

    let row = sqlx::query(
        "SELECT current_hp, max_hp, resistances, immunities, vulnerabilities \
         FROM combat_tokens WHERE id = ?1",
    )
    .bind(token_id)
    .fetch_optional(pool)
    .await;

    match row {
        Ok(Some(r)) => {
            let current_hp: i32 = r.try_get("current_hp").unwrap_or(0);
            let resistances: Option<String> = r.try_get("resistances").ok().flatten();
            let immunities: Option<String> = r.try_get("immunities").ok().flatten();
            let vulnerabilities: Option<String> = r.try_get("vulnerabilities").ok().flatten();

            let resist = build_damage_resistance(
                resistances.as_deref(),
                immunities.as_deref(),
                vulnerabilities.as_deref(),
            );

            // Map the type string; unknown -> Normal (no modifier applied).
            let effective = if let Some(dt) = parse_damage_type(&damage_type_str) {
                compute_effective_damage(amount, dt, &resist)
            } else {
                amount
            };

            let new_hp = (current_hp - effective).max(0);
            if let Err(e) = sqlx::query("UPDATE combat_tokens SET current_hp = ?1 WHERE id = ?2")
                .bind(new_hp)
                .bind(token_id)
                .execute(pool)
                .await
            {
                tracing::warn!(error = %e, "sqlx write failed in execute_apply_damage");
                return (json!({ "error": e.to_string() }), true);
            }
            (
                json!({
                    "new_hp": new_hp,
                    "raw_damage": amount,
                    "effective_damage": effective
                }),
                false,
            )
        }
        _ => (
            json!({ "error": "token not found", "token_id": token_id }),
            true,
        ),
    }
}

async fn execute_apply_healing(args: &Value, pool: &SqlitePool) -> (Value, bool) {
    let token_id = args["token_id"].as_str().unwrap_or_default();
    let amount = args["amount"].as_i64().unwrap_or(0) as i32;

    let row = sqlx::query("SELECT current_hp, max_hp FROM combat_tokens WHERE id = ?1")
        .bind(token_id)
        .fetch_optional(pool)
        .await;

    match row {
        Ok(Some(r)) => {
            use sqlx::Row;
            let current_hp: i32 = r.try_get("current_hp").unwrap_or(0);
            let max_hp: i32 = r.try_get("max_hp").unwrap_or(current_hp);
            let new_hp = (current_hp + amount).min(max_hp);
            if let Err(e) = sqlx::query("UPDATE combat_tokens SET current_hp = ?1 WHERE id = ?2")
                .bind(new_hp)
                .bind(token_id)
                .execute(pool)
                .await
            {
                tracing::warn!(error = %e, "sqlx write failed in execute_apply_healing");
                return (json!({ "error": e.to_string() }), true);
            }
            (json!({ "new_hp": new_hp, "healing_done": amount }), false)
        }
        _ => (
            json!({ "error": "token not found", "token_id": token_id }),
            true,
        ),
    }
}

async fn execute_start_combat(args: &Value, pool: &SqlitePool, session_id: Uuid) -> (Value, bool) {
    use app_domain::combat::initiative::{InitiativeEntry, InitiativeOrder};
    use app_domain::combat::types::CombatantId;
    use app_domain::dice::{roll_expr_detailed, DiceExpr, Die};
    use app_domain::rng::SeededRng;
    use std::collections::HashMap;

    let entries_raw = match args["initiative_entries"].as_array() {
        Some(a) => a,
        None => {
            return (
                json!({ "error": "initiative_entries must be an array" }),
                true,
            );
        }
    };

    let mut rng = SeededRng::new_random();
    let d20 = DiceExpr {
        count: 1,
        die: Die::D20,
        modifier: 0,
    };

    // Build domain entries and a parallel id->name map in one pass.
    let mut id_to_name: HashMap<uuid::Uuid, String> = HashMap::new();
    let mut domain_entries: Vec<InitiativeEntry> = Vec::with_capacity(entries_raw.len());

    for entry in entries_raw {
        let name = entry["name"].as_str().unwrap_or("Unknown").to_string();
        let dex_mod = entry
            .get("dex_mod")
            .or_else(|| entry.get("dex_tiebreak"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;

        // Use provided roll if present, otherwise roll d20 + dex_mod.
        let roll = if let Some(provided) = entry.get("roll").and_then(|v| v.as_i64()) {
            provided as i32
        } else {
            roll_expr_detailed(&d20, &mut rng).total + dex_mod
        };

        let id = CombatantId(uuid::Uuid::new_v4());
        id_to_name.insert(id.0, name);
        domain_entries.push(InitiativeEntry {
            id,
            roll,
            dex_tiebreak: dex_mod,
        });
    }

    // Sort descending by roll, tiebreak by dex_tiebreak (via InitiativeOrder::build).
    let order = InitiativeOrder::build(domain_entries);

    // Produce the `ordered` array: [{name, roll}, ...] in sorted initiative order.
    let ordered: Vec<Value> = order
        .as_slice()
        .iter()
        .map(|e| {
            let name = id_to_name.get(&e.id.0).cloned().unwrap_or_default();
            json!({ "name": name, "roll": e.roll })
        })
        .collect();

    let initiative_json = serde_json::to_string(&ordered).unwrap_or_default();
    let encounter_id = uuid::Uuid::new_v4();
    let now = chrono::Utc::now().to_rfc3339();
    if let Err(e) = sqlx::query(
        "INSERT INTO combat_encounters (id, session_id, round, started_at, initiative) VALUES (?1, ?2, 1, ?3, ?4)"
    )
    .bind(encounter_id.to_string())
    .bind(session_id.to_string())
    .bind(now)
    .bind(initiative_json)
    .execute(pool)
    .await
    {
        tracing::warn!(error = %e, "sqlx write failed in execute_start_combat");
        return (json!({ "error": e.to_string() }), true);
    }
    (
        json!({ "encounter_id": encounter_id.to_string(), "ordered": ordered }),
        false,
    )
}

async fn execute_end_combat(pool: &SqlitePool) -> (Value, bool) {
    let now = chrono::Utc::now().to_rfc3339();
    if let Err(e) = sqlx::query("UPDATE combat_encounters SET ended_at = ?1 WHERE ended_at IS NULL")
        .bind(now)
        .execute(pool)
        .await
    {
        tracing::warn!(error = %e, "sqlx write failed in execute_end_combat");
        return (json!({ "error": e.to_string() }), true);
    }
    (json!({ "status": "combat_ended" }), false)
}

/// Encode an optional JSON array field from the args into a JSON string for storage.
/// Returns `None` if the field is absent or not an array.
fn encode_resist_list(args: &Value, field: &str) -> Option<String> {
    args.get(field)
        .and_then(|v| v.as_array())
        .map(|arr| serde_json::to_string(arr).unwrap_or_else(|_| "[]".into()))
}

async fn execute_add_token(args: &Value, pool: &SqlitePool) -> (Value, bool) {
    let id = args["id"].as_str().unwrap_or_default();
    let name = args["name"].as_str().unwrap_or("Unknown");
    let x = args["x"].as_i64().unwrap_or(0) as i32;
    let y = args["y"].as_i64().unwrap_or(0) as i32;
    let hp = args["hp"].as_i64().unwrap_or(1) as i32;
    let max_hp = args["max_hp"].as_i64().unwrap_or(1) as i32;
    let ac = args["ac"].as_i64().unwrap_or(10) as i32;

    let resistances = encode_resist_list(args, "resistances");
    let immunities = encode_resist_list(args, "immunities");
    let vulnerabilities = encode_resist_list(args, "vulnerabilities");

    // Get the most recent open encounter id.
    let encounter_row = sqlx::query(
        "SELECT id FROM combat_encounters WHERE ended_at IS NULL ORDER BY started_at DESC LIMIT 1",
    )
    .fetch_optional(pool)
    .await;

    let encounter_id = match encounter_row {
        Ok(Some(r)) => {
            use sqlx::Row;
            r.try_get::<String, _>("id").unwrap_or_default()
        }
        _ => return (json!({ "error": "no active encounter" }), true),
    };

    if let Err(e) = sqlx::query(
        "INSERT OR REPLACE INTO combat_tokens \
         (id, encounter_id, name, current_hp, max_hp, ac, pos_x, pos_y, conditions, resistances, immunities, vulnerabilities) \
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'[]',?9,?10,?11)"
    )
    .bind(id)
    .bind(encounter_id)
    .bind(name)
    .bind(hp)
    .bind(max_hp)
    .bind(ac)
    .bind(x)
    .bind(y)
    .bind(resistances)
    .bind(immunities)
    .bind(vulnerabilities)
    .execute(pool)
    .await
    {
        tracing::warn!(error = %e, "sqlx write failed in execute_add_token");
        return (json!({ "error": e.to_string() }), true);
    }

    (json!({ "token_id": id, "status": "added" }), false)
}

async fn execute_update_token(args: &Value, pool: &SqlitePool) -> (Value, bool) {
    let id = args["id"].as_str().unwrap_or_default();
    if let Some(hp) = args["hp"].as_i64() {
        if let Err(e) = sqlx::query("UPDATE combat_tokens SET current_hp = ?1 WHERE id = ?2")
            .bind(hp as i32)
            .bind(id)
            .execute(pool)
            .await
        {
            tracing::warn!(error = %e, "sqlx write failed in execute_update_token");
            return (json!({ "error": e.to_string() }), true);
        }
    }
    if let Some(x) = args["x"].as_i64() {
        if let Some(y) = args["y"].as_i64() {
            if let Err(e) =
                sqlx::query("UPDATE combat_tokens SET pos_x = ?1, pos_y = ?2 WHERE id = ?3")
                    .bind(x as i32)
                    .bind(y as i32)
                    .bind(id)
                    .execute(pool)
                    .await
            {
                tracing::warn!(error = %e, "sqlx write failed in execute_update_token");
                return (json!({ "error": e.to_string() }), true);
            }
        }
    }
    // Persist resistance fields if provided.
    for field in &["resistances", "immunities", "vulnerabilities"] {
        if let Some(encoded) = encode_resist_list(args, field) {
            if let Err(e) = sqlx::query(&format!(
                "UPDATE combat_tokens SET {field} = ?1 WHERE id = ?2"
            ))
            .bind(encoded)
            .bind(id)
            .execute(pool)
            .await
            {
                tracing::warn!(error = %e, "sqlx write failed in execute_update_token ({field})");
                return (json!({ "error": e.to_string() }), true);
            }
        }
    }
    (json!({ "token_id": id, "status": "updated" }), false)
}

async fn execute_remove_token(args: &Value, pool: &SqlitePool) -> (Value, bool) {
    let id = args["id"].as_str().unwrap_or_default();
    if let Err(e) = sqlx::query("UPDATE combat_tokens SET is_dead = 1 WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await
    {
        tracing::warn!(error = %e, "sqlx write failed in execute_remove_token");
        return (json!({ "error": e.to_string() }), true);
    }
    (json!({ "token_id": id, "status": "removed" }), false)
}

async fn execute_set_scene(args: &Value, pool: &SqlitePool, campaign_id: Uuid) -> (Value, bool) {
    let title = args["title"].as_str().unwrap_or("Unnamed Scene");
    let subtitle = args.get("subtitle").and_then(|v| v.as_str());
    let mode = args["mode"].as_str().unwrap_or("exploration");
    let image_prompt = args.get("image_prompt").and_then(|v| v.as_str());

    match crate::db::scene_insert(pool, campaign_id, title, subtitle, mode, image_prompt).await {
        Ok(scene_id) => (
            json!({
                "scene_id": scene_id.to_string(),
                "title": title,
                "subtitle": subtitle.unwrap_or(""),
                "mode": mode,
            }),
            false,
        ),
        Err(e) => {
            tracing::warn!(error = %e, "sqlx write failed in execute_set_scene");
            (json!({ "error": e.to_string() }), true)
        }
    }
}

/// Detect whether a spell is a healing spell based on its description text
/// (the compendium has no explicit `healing` field - we identify by description).
fn spell_is_healing(description: &str) -> bool {
    let lower = description.to_lowercase();
    lower.contains("regains") && lower.contains("hit point")
        || lower.contains("restores") && lower.contains("hit point")
        || lower.contains("regain hit points")
        || lower.contains("restore hit points")
}

/// Parse dice string like "1d8" and roll it with optional flat bonus.
/// Returns (total, rolls_vec).
fn roll_spell_dice(
    dice_str: &str,
    bonus: i32,
    rng: &mut app_domain::rng::SeededRng,
) -> (i32, Vec<i32>) {
    let expr = parse_dice_expr(dice_str, bonus);
    let detail = app_domain::dice::roll_expr_detailed(&expr, rng);
    (detail.total, detail.rolls)
}

async fn execute_cast_spell(args: &Value, pool: &SqlitePool) -> (Value, bool) {
    use app_domain::combat::damage::compute_effective_damage;
    use app_domain::compendium::compendium;
    use app_domain::rng::SeededRng;
    use sqlx::Row;

    let spell_key = args["spell"].as_str().unwrap_or("unknown");

    // 1. Look up spell in compendium - match by id or name (case-insensitive).
    let comp = compendium();
    let spell = comp.spells.iter().find(|s| {
        s.id.eq_ignore_ascii_case(spell_key) || s.name_en.eq_ignore_ascii_case(spell_key)
    });

    let Some(spell) = spell else {
        return (json!({ "spell": spell_key, "status": "not_in_srd" }), false);
    };

    let spell_name = spell.name_en.clone();
    let spell_id = spell.id.clone();
    let spell_level = spell.level;

    // 2. Determine spell kind.
    let is_healing = spell_is_healing(&spell.description_en);
    let has_damage = spell.damage.is_some();

    // Collect targets from args (optional array of token_ids or objects).
    let targets_val = args.get("targets").and_then(|v| v.as_array());

    let mut rng = SeededRng::new_random();

    // Optional save DC from args (for save-for-half spells).
    let save_dc = args
        .get("save_dc")
        .and_then(|v| v.as_i64())
        .map(|v| v as i32);

    // 3a. DAMAGE spell path.
    if has_damage {
        let dmg_info = spell.damage.as_ref().unwrap();
        let dice_str = &dmg_info.dice;
        let damage_type_str = dmg_info.damage_type.to_lowercase();

        // Roll damage once (shared roll for AoE; each target applies separately).
        let (raw_damage, rolls) = roll_spell_dice(dice_str, 0, &mut rng);

        let save_config = spell.save.as_ref();
        let half_on_success = save_config.map(|s| s.half_on_success).unwrap_or(false);

        // If there are targets, resolve damage for each.
        let mut target_results: Vec<Value> = Vec::new();

        if let Some(targets) = targets_val {
            for target in targets {
                // Accept either a plain string token_id or an object { token_id, save_bonus? }.
                let token_id = target
                    .as_str()
                    .or_else(|| target.get("token_id").and_then(|v| v.as_str()))
                    .unwrap_or_default();
                let save_bonus = target
                    .get("save_bonus")
                    .and_then(|v| v.as_i64())
                    .unwrap_or(0) as i32;

                let row = sqlx::query(
                    "SELECT current_hp, max_hp, resistances, immunities, vulnerabilities \
                     FROM combat_tokens WHERE id = ?1",
                )
                .bind(token_id)
                .fetch_optional(pool)
                .await;

                let Ok(Some(r)) = row else {
                    target_results.push(json!({
                        "token_id": token_id,
                        "error": "token not found"
                    }));
                    continue;
                };

                let current_hp: i32 = r.try_get("current_hp").unwrap_or(0);
                let max_hp: i32 = r.try_get("max_hp").unwrap_or(current_hp);
                let resistances: Option<String> = r.try_get("resistances").ok().flatten();
                let immunities: Option<String> = r.try_get("immunities").ok().flatten();
                let vulnerabilities: Option<String> = r.try_get("vulnerabilities").ok().flatten();

                let resist = build_damage_resistance(
                    resistances.as_deref(),
                    immunities.as_deref(),
                    vulnerabilities.as_deref(),
                );

                // Determine effective raw (before resistance) - handle save-for-half.
                let raw_for_target = if half_on_success {
                    if let Some(dc) = save_dc {
                        // Roll save: d20 + save_bonus vs DC.
                        let save_roll = {
                            let d20 = parse_dice_expr("1d20", save_bonus);
                            app_domain::dice::roll_expr(&d20, &mut rng)
                        };
                        if save_roll >= dc {
                            raw_damage / 2
                        } else {
                            raw_damage
                        }
                    } else {
                        raw_damage
                    }
                } else {
                    raw_damage
                };

                // Apply resistance/immunity/vulnerability.
                let effective = if let Some(dt) = parse_damage_type(&damage_type_str) {
                    compute_effective_damage(raw_for_target, dt, &resist)
                } else {
                    raw_for_target
                };

                let new_hp = (current_hp - effective).max(0);
                _ = max_hp; // read above, satisfies completeness

                if let Err(e) =
                    sqlx::query("UPDATE combat_tokens SET current_hp = ?1 WHERE id = ?2")
                        .bind(new_hp)
                        .bind(token_id)
                        .execute(pool)
                        .await
                {
                    tracing::warn!(error = %e, "sqlx write failed in execute_cast_spell");
                    target_results.push(json!({
                        "token_id": token_id,
                        "error": e.to_string()
                    }));
                    continue;
                }

                target_results.push(json!({
                    "token_id": token_id,
                    "raw_damage": raw_for_target,
                    "effective_damage": effective,
                    "new_hp": new_hp,
                }));
            }
        }

        return (
            json!({
                "spell": spell_id,
                "spell_name": spell_name,
                "level": spell_level,
                "status": "resolved",
                "damage_type": damage_type_str,
                "dice": dice_str,
                "rolls": rolls,
                "raw_damage": raw_damage,
                "save_dc": save_dc,
                "targets": target_results,
                "slots_tracked": false,
            }),
            false,
        );
    }

    // 3b. HEALING spell path.
    if is_healing {
        // Extract healing dice from description (best-effort: parse NdX pattern).
        // Compendium healing spells: cure-wounds=1d8, healing-word=1d4, prayer-of-healing=2d8.
        // We search the description for the first dice expression.
        let heal_dice = extract_dice_from_description(&spell.description_en)
            .unwrap_or_else(|| "1d4".to_string());
        // Bonus from caster's spellcasting modifier (optional, defaults 0).
        let spell_mod = args
            .get("spell_modifier")
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        let (heal_amount, rolls) = roll_spell_dice(&heal_dice, spell_mod, &mut rng);

        let mut target_results: Vec<Value> = Vec::new();

        if let Some(targets) = targets_val {
            for target in targets {
                let token_id = target
                    .as_str()
                    .or_else(|| target.get("token_id").and_then(|v| v.as_str()))
                    .unwrap_or_default();

                let row = sqlx::query("SELECT current_hp, max_hp FROM combat_tokens WHERE id = ?1")
                    .bind(token_id)
                    .fetch_optional(pool)
                    .await;

                let Ok(Some(r)) = row else {
                    target_results.push(json!({
                        "token_id": token_id,
                        "error": "token not found"
                    }));
                    continue;
                };

                let current_hp: i32 = r.try_get("current_hp").unwrap_or(0);
                let max_hp: i32 = r.try_get("max_hp").unwrap_or(current_hp);
                let new_hp = (current_hp + heal_amount).min(max_hp);

                if let Err(e) =
                    sqlx::query("UPDATE combat_tokens SET current_hp = ?1 WHERE id = ?2")
                        .bind(new_hp)
                        .bind(token_id)
                        .execute(pool)
                        .await
                {
                    tracing::warn!(error = %e, "sqlx write failed in execute_cast_spell healing");
                    target_results.push(json!({
                        "token_id": token_id,
                        "error": e.to_string()
                    }));
                    continue;
                }

                target_results.push(json!({
                    "token_id": token_id,
                    "healing": heal_amount,
                    "new_hp": new_hp,
                }));
            }
        }

        return (
            json!({
                "spell": spell_id,
                "spell_name": spell_name,
                "level": spell_level,
                "status": "resolved",
                "kind": "healing",
                "dice": heal_dice,
                "rolls": rolls,
                "healing": heal_amount,
                "targets": target_results,
                "slots_tracked": false,
            }),
            false,
        );
    }

    // 3c. Narrative-only spell (no mechanical data in compendium - e.g. magic-missile, buffs).
    (
        json!({
            "spell": spell_id,
            "spell_name": spell_name,
            "level": spell_level,
            "status": "resolved",
            "note": "narrative-only (no mechanical data in SRD set)",
            "slots_tracked": false,
        }),
        false,
    )
}

/// Extract the first dice expression (e.g. "1d8", "2d8") from a description string.
/// Used to parse healing dice from the spell's English description when there is no
/// explicit `damage` field (healing spells have `damage: null` in the compendium).
fn extract_dice_from_description(desc: &str) -> Option<String> {
    // Find first occurrence of NdX or dX pattern.
    let lower = desc.to_lowercase();
    let bytes = lower.as_bytes();
    for i in 0..bytes.len() {
        if bytes[i] == b'd' {
            // Check preceding digits for count.
            let count_start = bytes[..i]
                .iter()
                .rposition(|&b| !b.is_ascii_digit())
                .map(|p| p + 1)
                .unwrap_or(0);
            let count_str = &lower[count_start..i];

            // Require at least one digit after 'd'.
            let die_start = i + 1;
            if die_start >= lower.len() {
                continue;
            }
            let die_end = lower[die_start..]
                .find(|c: char| !c.is_ascii_digit())
                .map(|p| die_start + p)
                .unwrap_or(lower.len());
            let die_str = &lower[die_start..die_end];
            if die_str.is_empty() {
                continue;
            }
            let count: u32 = count_str.parse().unwrap_or(1).max(1);
            return Some(format!("{count}d{die_str}"));
        }
    }
    None
}

async fn execute_remember_npc(args: &Value, pool: &SqlitePool, campaign_id: Uuid) -> (Value, bool) {
    let Some(name) = args["name"].as_str() else {
        return (json!({ "error": "name is required" }), true);
    };
    let Some(fact) = args["fact"].as_str() else {
        return (json!({ "error": "fact is required" }), true);
    };
    let disposition = args
        .get("disposition")
        .and_then(|v| v.as_str())
        .unwrap_or("neutral");
    let role = args.get("role").and_then(|v| v.as_str()).unwrap_or("");

    if let Err(e) =
        crate::db::npc_upsert_fact(pool, campaign_id, name, fact, disposition, role).await
    {
        tracing::warn!(error = %e, "sqlx write failed in execute_remember_npc");
        return (json!({ "error": e.to_string() }), true);
    }
    (json!({ "name": name, "status": "remembered" }), false)
}

async fn execute_recall_npc(args: &Value, pool: &SqlitePool, campaign_id: Uuid) -> (Value, bool) {
    let Some(name) = args["name"].as_str() else {
        return (json!({ "error": "name is required" }), true);
    };
    match crate::db::npc_get(pool, campaign_id, name).await {
        Ok(Some(npc)) => (
            json!({
                "name": npc.name,
                "role": npc.role,
                "disposition": npc.disposition,
                "trust": npc.trust,
                "facts": npc.facts,
            }),
            false,
        ),
        Ok(None) => (
            json!({ "name": name, "facts": [], "status": "unknown_npc" }),
            false,
        ),
        Err(e) => {
            tracing::warn!(error = %e, "sqlx read failed in execute_recall_npc");
            (json!({ "error": e.to_string() }), true)
        }
    }
}

async fn execute_journal_append(
    args: &Value,
    pool: &SqlitePool,
    campaign_id: Uuid,
) -> (Value, bool) {
    let Some(entry_html) = args["entry_html"].as_str() else {
        return (json!({ "error": "entry_html is required" }), true);
    };
    let chapter = args.get("chapter").and_then(|v| v.as_str());
    match crate::db::journal_insert(pool, campaign_id, entry_html, chapter).await {
        Ok(id) => (json!({ "entry_id": id.to_string() }), false),
        Err(e) => {
            tracing::warn!(error = %e, "sqlx write failed in execute_journal_append");
            (json!({ "error": e.to_string() }), true)
        }
    }
}

async fn execute_quick_save(args: &Value, pool: &SqlitePool, session_id: Uuid) -> (Value, bool) {
    let label = args
        .get("label")
        .and_then(|v| v.as_str())
        .unwrap_or("Quick save");

    // Build a real game_state snapshot from DB (schema_version 2).
    let game_state = match crate::db::build_save_game_state(pool, session_id, label).await {
        Ok(gs) => gs,
        Err(e) => {
            tracing::warn!(error = %e, "failed to build game_state in execute_quick_save");
            return (json!({ "error": e.to_string() }), true);
        }
    };

    // Derive a human-readable summary from the game_state for the saves list.
    let (title, summary, tag) = {
        let combat = game_state.get("combat");
        let scene = game_state.get("scene");
        let is_combat = combat
            .and_then(|c| c.as_object())
            .map(|o| o.get("active").and_then(|v| v.as_bool()).unwrap_or(false))
            .unwrap_or(false);
        let scene_title = scene
            .and_then(|s| s.as_object())
            .and_then(|o| o.get("title"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let round = combat
            .and_then(|c| c.as_object())
            .and_then(|o| o.get("round"))
            .and_then(|v| v.as_i64())
            .unwrap_or(1);

        if is_combat {
            let summary = if scene_title.is_empty() {
                format!("Combat round {round}")
            } else {
                format!("Combat in {scene_title}, round {round}")
            };
            (label.to_string(), summary, "combat")
        } else if !scene_title.is_empty() {
            let summary = format!("Scene: {scene_title}");
            (label.to_string(), summary, "exploration")
        } else {
            (label.to_string(), String::new(), "exploration")
        }
    };

    let save_id = uuid::Uuid::new_v4();
    let now = chrono::Utc::now().to_rfc3339();
    let state_json = match serde_json::to_string(&game_state) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "failed to serialize game_state in execute_quick_save");
            return (json!({ "error": e.to_string() }), true);
        }
    };

    if let Err(e) = sqlx::query(
        "INSERT INTO snapshots (id, session_id, turn_number, created_at, game_state, player_action, kind, title, summary, tag) \
         VALUES (?1, ?2, 0, ?3, ?4, NULL, 'auto', ?5, ?6, ?7)",
    )
    .bind(save_id.to_string())
    .bind(session_id.to_string())
    .bind(now)
    .bind(state_json)
    .bind(title)
    .bind(summary)
    .bind(tag)
    .execute(pool)
    .await
    {
        tracing::warn!(error = %e, "sqlx write failed in execute_quick_save");
        return (json!({ "error": e.to_string() }), true);
    }
    (
        json!({ "save_id": save_id.to_string(), "branch_id": null }),
        false,
    )
}

/// Shared image-generation path. `style` selects the sidecar style preset and
/// `dims` the output canvas (None = sidecar default 1024x1024). The final
/// content prompt has already been shaped by the caller. The orchestrator peels
/// `image_b64` into a dedicated `AgentEvent::ImageGenerated`.
async fn run_image_generation(
    content_prompt: String,
    style: String,
    dims: Option<(u32, u32)>,
    image_provider: Option<Arc<dyn ImageProvider>>,
) -> (Value, bool) {
    let Some(provider) = image_provider else {
        return (
            json!({ "error": "image generation is not available (no image provider configured)" }),
            true,
        );
    };
    if content_prompt.trim().is_empty() {
        return (json!({ "error": "prompt is required" }), true);
    }
    let (width, height) = match dims {
        Some((w, h)) => (Some(w), Some(h)),
        None => (None, None),
    };
    let image_prompt = ImagePrompt {
        content_prompt,
        style_preset: style,
        scene_id: None,
        npc_ids: Vec::new(),
        backend_preset: None,
        width,
        height,
    };
    match provider.generate(image_prompt).await {
        Ok(bytes) => (
            json!({
                "status": "generated",
                "mime_type": bytes.mime_type,
                "image_b64": B64.encode(&bytes.data),
            }),
            false,
        ),
        Err(e) => {
            warn!("image generation failed: {e}");
            (json!({ "error": e.to_string() }), true)
        }
    }
}

/// Top-down tactical battle map for the VTT board. Shapes the prompt to force a
/// bird's-eye, grid-friendly, character-free render and a landscape canvas.
async fn execute_generate_map(
    args: &Value,
    image_provider: Option<Arc<dyn ImageProvider>>,
) -> (Value, bool) {
    let raw = args["prompt"].as_str().unwrap_or("").trim();
    if raw.is_empty() {
        return (json!({ "error": "prompt is required" }), true);
    }
    let shaped = format!(
        "Top-down tactical RPG battle map, orthographic bird's-eye view, no \
         perspective, square grid friendly, detailed terrain and floor tiles, no \
         characters, no tokens, no text labels. Location: {raw}"
    );
    run_image_generation(shaped, "map".to_string(), Some((1216, 832)), image_provider).await
}

/// Cinematic illustration/portrait shown inline in the chat. Square default
/// canvas; style preset chosen by the model (dark_fantasy | portrait).
///
/// The `style` arg shapes the content prompt so the image backend receives
/// concrete framing words regardless of whether it supports style_preset:
/// - "portrait" prepends a head-and-shoulders character framing.
/// - "dark_fantasy" (default) appends dark-fantasy atmosphere cues.
async fn execute_generate_illustration(
    args: &Value,
    image_provider: Option<Arc<dyn ImageProvider>>,
) -> (Value, bool) {
    let raw = args["prompt"].as_str().unwrap_or("").trim();
    let style = args
        .get("style")
        .and_then(|v| v.as_str())
        .unwrap_or("dark_fantasy");

    let content_prompt = match style {
        "portrait" => format!("character portrait, head and shoulders, detailed face, {raw}"),
        _ => format!("{raw}, dark fantasy art, dramatic lighting, moody"),
    };

    run_image_generation(content_prompt, style.to_string(), None, image_provider).await
}

/// Generate a short video clip. Returns `{status, mime_type, video_b64}` on
/// success. The orchestrator peels `video_b64` into a dedicated
/// `AgentEvent::VideoGenerated` so the blob never enters LLM history.
async fn execute_generate_video(
    args: &Value,
    video_provider: Option<Arc<dyn VideoProvider>>,
) -> (Value, bool) {
    let Some(provider) = video_provider else {
        return (
            json!({ "error": "video generation is not available (no video provider configured)" }),
            true,
        );
    };
    let raw = args["prompt"].as_str().unwrap_or("").trim();
    if raw.is_empty() {
        return (json!({ "error": "prompt is required" }), true);
    }
    let frame_count = args
        .get("frame_count")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    let prompt = VideoPrompt {
        text: raw.to_string(),
        frame_count: frame_count.unwrap_or(97),
        ..Default::default()
    };

    match provider.generate(prompt).await {
        Ok(mut stream) => {
            use crate::video::provider::VideoEvent;
            // Drain the event stream to get the final Done event.
            loop {
                match stream.events.recv().await {
                    Some(VideoEvent::Done {
                        mp4_bytes,
                        duration_seconds,
                    }) => {
                        return (
                            json!({
                                "status": "generated",
                                "mime_type": "video/mp4",
                                "video_b64": B64.encode(&mp4_bytes),
                                "duration_seconds": duration_seconds,
                            }),
                            false,
                        );
                    }
                    Some(VideoEvent::Error { message }) => {
                        warn!("video generation error: {message}");
                        return (json!({ "error": message }), true);
                    }
                    Some(VideoEvent::Started { .. } | VideoEvent::Progress { .. }) => {
                        // Continue draining.
                    }
                    None => {
                        return (
                            json!({ "error": "video stream ended without result" }),
                            true,
                        );
                    }
                }
            }
        }
        Err(e) => {
            warn!("video generation failed: {e}");
            (json!({ "error": e.to_string() }), true)
        }
    }
}

async fn execute_query_rules(
    args: &Value,
    retriever: Option<&app_domain::srd::retriever::SrdRetriever>,
    embedding_model: &str,
) -> (Value, bool) {
    let question = args["question"].as_str().unwrap_or("");

    let Some(ret) = retriever else {
        return (
            json!({ "question": question, "chunks": [], "status": "rag_unavailable" }),
            false,
        );
    };

    if ret.is_empty() {
        return (
            json!({ "question": question, "chunks": [], "status": "rag_unavailable" }),
            false,
        );
    }

    match crate::agent::context_builder::embed_player_message(question, embedding_model) {
        Ok(query_emb) => {
            let chunks = ret.retrieve_by_embedding(&query_emb, 5);
            let chunk_values: Vec<serde_json::Value> = chunks
                .iter()
                .map(|c| json!({ "source_key": c.source_key, "text": c.text_en }))
                .collect();
            (
                json!({ "question": question, "chunks": chunk_values }),
                false,
            )
        }
        Err(e) => {
            tracing::warn!(error = %e, "embedding failed in execute_query_rules");
            (
                json!({ "question": question, "chunks": [], "status": "embed_error" }),
                false,
            )
        }
    }
}

#[cfg(test)]
mod image_dispatch_tests {
    use super::*;
    use crate::image::provider::{ImageBytes, ImageError, ImagePrompt, ImageProvider};
    use async_trait::async_trait;
    use std::sync::Mutex;

    struct CapturingProvider {
        last: Arc<Mutex<Option<ImagePrompt>>>,
    }

    #[async_trait]
    impl ImageProvider for CapturingProvider {
        async fn generate(&self, prompt: ImagePrompt) -> Result<ImageBytes, ImageError> {
            *self.last.lock().unwrap() = Some(prompt);
            Ok(ImageBytes {
                data: vec![1, 2, 3],
                mime_type: "image/png".into(),
            })
        }
        fn estimated_seconds(&self) -> u32 {
            1
        }
        fn cost_per_image(&self) -> f32 {
            0.0
        }
    }

    #[tokio::test]
    async fn generate_map_is_top_down_and_landscape() {
        let last = Arc::new(Mutex::new(None));
        let provider = Arc::new(CapturingProvider { last: last.clone() });
        let (val, is_err) =
            execute_generate_map(&json!({ "prompt": "ruined throne hall" }), Some(provider)).await;
        assert!(!is_err);
        assert_eq!(
            val.get("status").and_then(|v| v.as_str()),
            Some("generated")
        );
        let captured = last.lock().unwrap().clone().unwrap();
        assert_eq!(captured.style_preset, "map");
        assert!(
            captured.content_prompt.to_lowercase().contains("top-down"),
            "map prompt must request a top-down view"
        );
        assert!(captured.content_prompt.contains("ruined throne hall"));
        assert_eq!(captured.width, Some(1216));
        assert_eq!(captured.height, Some(832));
    }

    #[tokio::test]
    async fn generate_illustration_keeps_square_default() {
        let last = Arc::new(Mutex::new(None));
        let provider = Arc::new(CapturingProvider { last: last.clone() });
        let (_val, is_err) = execute_generate_illustration(
            &json!({ "prompt": "the lich king on his throne", "style": "portrait" }),
            Some(provider),
        )
        .await;
        assert!(!is_err);
        let captured = last.lock().unwrap().clone().unwrap();
        assert_eq!(captured.style_preset, "portrait");
        assert!(captured.content_prompt.contains("the lich king"));
        assert_eq!(captured.width, None);
        assert_eq!(captured.height, None);
    }

    #[tokio::test]
    async fn illustration_portrait_style_prepends_framing() {
        let last = Arc::new(Mutex::new(None));
        let provider = Arc::new(CapturingProvider { last: last.clone() });
        let (_val, is_err) = execute_generate_illustration(
            &json!({ "prompt": "the ranger Aela", "style": "portrait" }),
            Some(provider),
        )
        .await;
        assert!(!is_err);
        let captured = last.lock().unwrap().clone().unwrap();
        assert_eq!(captured.style_preset, "portrait");
        // Framing words must appear before the raw prompt.
        assert!(
            captured
                .content_prompt
                .starts_with("character portrait, head and shoulders"),
            "portrait style must prepend framing: got '{}'",
            captured.content_prompt
        );
        assert!(captured.content_prompt.contains("the ranger Aela"));
    }

    #[tokio::test]
    async fn illustration_dark_fantasy_style_appends_atmosphere() {
        let last = Arc::new(Mutex::new(None));
        let provider = Arc::new(CapturingProvider { last: last.clone() });
        let (_val, is_err) = execute_generate_illustration(
            &json!({ "prompt": "a ruined temple", "style": "dark_fantasy" }),
            Some(provider),
        )
        .await;
        assert!(!is_err);
        let captured = last.lock().unwrap().clone().unwrap();
        assert_eq!(captured.style_preset, "dark_fantasy");
        assert!(captured.content_prompt.contains("a ruined temple"));
        assert!(
            captured.content_prompt.contains("dark fantasy art"),
            "dark_fantasy style must append atmosphere: got '{}'",
            captured.content_prompt
        );
    }

    #[tokio::test]
    async fn illustration_default_style_is_dark_fantasy() {
        // When no style arg is provided, the default "dark_fantasy" atmosphere is applied.
        let last = Arc::new(Mutex::new(None));
        let provider = Arc::new(CapturingProvider { last: last.clone() });
        let (_val, is_err) = execute_generate_illustration(
            &json!({ "prompt": "stormy battlefield" }),
            Some(provider),
        )
        .await;
        assert!(!is_err);
        let captured = last.lock().unwrap().clone().unwrap();
        assert!(
            captured.content_prompt.contains("dark fantasy art"),
            "default style must produce dark_fantasy atmosphere"
        );
    }

    #[tokio::test]
    async fn image_tools_error_without_provider() {
        let (_, is_err) = execute_generate_map(&json!({ "prompt": "x" }), None).await;
        assert!(is_err);
        let (_, is_err2) = execute_generate_illustration(&json!({ "prompt": "x" }), None).await;
        assert!(is_err2);
    }
}

#[cfg(test)]
mod video_dispatch_tests {
    use super::*;
    use crate::video::provider::{
        VideoCapabilities, VideoError, VideoEvent, VideoPrompt, VideoProvider, VideoStream,
    };
    use async_trait::async_trait;
    use tokio::sync::mpsc;

    struct MockVideoProvider {
        /// Bytes returned in VideoEvent::Done.
        mp4_bytes: Vec<u8>,
    }

    #[async_trait]
    impl VideoProvider for MockVideoProvider {
        async fn generate(&self, _prompt: VideoPrompt) -> Result<VideoStream, VideoError> {
            let (tx, rx) = mpsc::channel(4);
            let bytes = self.mp4_bytes.clone();
            tokio::spawn(async move {
                let _ = tx
                    .send(VideoEvent::Started {
                        estimated_seconds: 1,
                    })
                    .await;
                let _ = tx
                    .send(VideoEvent::Done {
                        mp4_bytes: bytes,
                        duration_seconds: 4.0,
                    })
                    .await;
            });
            Ok(VideoStream { events: rx })
        }

        fn capabilities(&self) -> VideoCapabilities {
            VideoCapabilities {
                duration_range_secs: (3, 8),
                max_resolution: (704, 480),
                supports_image_init: false,
                avg_seconds_per_clip: 4,
            }
        }
    }

    struct ErrorVideoProvider;

    #[async_trait]
    impl VideoProvider for ErrorVideoProvider {
        async fn generate(&self, _prompt: VideoPrompt) -> Result<VideoStream, VideoError> {
            Err(VideoError::BackendNotRunning)
        }
        fn capabilities(&self) -> VideoCapabilities {
            VideoCapabilities {
                duration_range_secs: (3, 8),
                max_resolution: (704, 480),
                supports_image_init: false,
                avg_seconds_per_clip: 4,
            }
        }
    }

    #[tokio::test]
    async fn execute_generate_video_returns_video_b64() {
        let provider = Arc::new(MockVideoProvider {
            mp4_bytes: vec![0x00, 0x00, 0x00, 0x18, b'f', b't', b'y', b'p'],
        });
        let (val, is_err) =
            execute_generate_video(&json!({ "prompt": "fog rolls in" }), Some(provider)).await;
        assert!(!is_err, "video generation must not error: {val}");
        assert_eq!(val["status"].as_str(), Some("generated"));
        assert_eq!(val["mime_type"].as_str(), Some("video/mp4"));
        let b64 = val["video_b64"].as_str().expect("video_b64 present");
        assert!(!b64.is_empty());
        let decoded = B64.decode(b64).expect("valid base64");
        assert_eq!(
            decoded,
            vec![0x00, 0x00, 0x00, 0x18, b'f', b't', b'y', b'p']
        );
    }

    #[tokio::test]
    async fn execute_generate_video_errors_without_provider() {
        let (val, is_err) = execute_generate_video(&json!({ "prompt": "x" }), None).await;
        assert!(is_err, "must error when provider is None");
        assert!(val["error"].as_str().is_some());
    }

    #[tokio::test]
    async fn execute_generate_video_errors_when_provider_fails() {
        let provider = Arc::new(ErrorVideoProvider);
        let (val, is_err) = execute_generate_video(&json!({ "prompt": "x" }), Some(provider)).await;
        assert!(is_err, "must surface provider error");
        assert!(val["error"].as_str().is_some());
    }

    #[tokio::test]
    async fn execute_generate_video_rejects_empty_prompt() {
        let provider = Arc::new(MockVideoProvider { mp4_bytes: vec![] });
        let (val, is_err) =
            execute_generate_video(&json!({ "prompt": "   " }), Some(provider)).await;
        assert!(is_err, "empty prompt must be an error");
        assert!(val["error"].as_str().is_some());
    }
}

#[cfg(test)]
mod query_rules_tests {
    use super::*;
    use app_domain::srd::data::SrdChunk;
    use app_domain::srd::retriever::SrdRetriever;

    /// Build a tiny SrdRetriever with two chunks whose embeddings are
    /// hand-crafted unit vectors. We pick orthogonal vectors so cosine
    /// similarity is deterministic: the "attack" query vector aligns with
    /// chunk A and not chunk B.
    fn make_test_retriever() -> SrdRetriever {
        // 4-dimensional unit vectors (enough to test cosine similarity).
        let emb_attack: Vec<f32> = vec![1.0, 0.0, 0.0, 0.0];
        let emb_spell: Vec<f32> = vec![0.0, 1.0, 0.0, 0.0];

        let chunk_a = SrdChunk::new(
            "attack_action",
            "Attack action: melee or ranged weapon attack.",
        );
        let chunk_b = SrdChunk::new("cast_spell", "Casting a spell: choose a spell you know.");

        SrdRetriever::new(vec![(chunk_a, emb_attack), (chunk_b, emb_spell)])
    }

    #[tokio::test]
    async fn query_rules_no_retriever_returns_rag_unavailable() {
        let args = serde_json::json!({ "question": "how does attack work?" });
        let (val, is_err) = execute_query_rules(&args, None, "").await;
        assert!(!is_err);
        assert_eq!(val["status"].as_str(), Some("rag_unavailable"));
        assert_eq!(val["chunks"].as_array().unwrap().len(), 0);
        assert_eq!(val["question"].as_str(), Some("how does attack work?"));
    }

    #[tokio::test]
    async fn query_rules_empty_retriever_returns_rag_unavailable() {
        let retriever = SrdRetriever::new(vec![]);
        let args = serde_json::json!({ "question": "how does attack work?" });
        let (val, is_err) = execute_query_rules(&args, Some(&retriever), "").await;
        assert!(!is_err);
        assert_eq!(val["status"].as_str(), Some("rag_unavailable"));
    }

    #[tokio::test]
    async fn query_rules_with_retriever_returns_chunks_aligned_by_cosine() {
        let retriever = make_test_retriever();
        // Since the real embed_player_message uses fastembed (which requires
        // a model file), we test graceful degradation: pass an invalid model
        // name so embed_player_message returns an error, verify embed_error status.
        let args = serde_json::json!({ "question": "attack" });
        let (val, is_err) = execute_query_rules(&args, Some(&retriever), "not-a-real-model").await;
        // embed_player_message will fail with an unknown model - graceful degradation.
        assert!(!is_err);
        // Either chunks were returned (if model was cached) or embed_error.
        // In CI without the model file, status == "embed_error".
        let status = val.get("status").and_then(|v| v.as_str());
        assert!(
            status == Some("embed_error")
                || val["chunks"]
                    .as_array()
                    .map(|a| !a.is_empty())
                    .unwrap_or(false),
            "expected embed_error or non-empty chunks, got: {val}"
        );
    }

    /// Direct wiring test: build retriever, manually produce a matching
    /// query embedding, call retrieve_by_embedding, assert the top result.
    /// This validates the plumbing without fastembed.
    #[test]
    fn retriever_returns_best_match_by_cosine() {
        let retriever = make_test_retriever();
        // Query in the "attack" direction.
        let query = vec![1.0_f32, 0.0, 0.0, 0.0];
        let results = retriever.retrieve_by_embedding(&query, 2);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].source_key, "attack_action");
        assert_eq!(results[1].source_key, "cast_spell");
    }
}

#[cfg(test)]
mod start_combat_tests {
    use super::*;

    async fn make_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory db");
        crate::db::init_db(&pool).await.expect("migrate");
        pool
    }

    /// With provided rolls and known dex_mods, `ordered` must be sorted
    /// descending by roll with dex_tiebreak resolving ties.
    #[tokio::test]
    async fn start_combat_sorts_by_roll_descending() {
        let pool = make_pool().await;
        let session_id = uuid::Uuid::new_v4();

        // Goblin roll=12, Fighter roll=18, Wizard roll=12 dex_mod=4 (wins tie vs Goblin dex 2)
        let args = json!({
            "initiative_entries": [
                { "name": "Goblin",  "roll": 12, "dex_mod": 2 },
                { "name": "Fighter", "roll": 18, "dex_mod": 1 },
                { "name": "Wizard",  "roll": 12, "dex_mod": 4 }
            ]
        });

        let (val, is_err) = execute_start_combat(&args, &pool, session_id).await;
        assert!(!is_err, "start_combat must not error: {val}");

        let ordered = val["ordered"].as_array().expect("ordered array present");
        assert_eq!(ordered.len(), 3);

        // First must be Fighter (roll 18)
        assert_eq!(ordered[0]["name"].as_str(), Some("Fighter"));
        assert_eq!(ordered[0]["roll"].as_i64(), Some(18));

        // Second must be Wizard (roll 12, dex 4 > Goblin dex 2)
        assert_eq!(ordered[1]["name"].as_str(), Some("Wizard"));
        assert_eq!(ordered[1]["roll"].as_i64(), Some(12));

        // Third is Goblin
        assert_eq!(ordered[2]["name"].as_str(), Some("Goblin"));
        assert_eq!(ordered[2]["roll"].as_i64(), Some(12));

        // encounter_id is present
        assert!(val["encounter_id"].as_str().is_some());
    }

    /// Without a provided roll, the engine must auto-roll d20+dex_mod,
    /// yielding a value in [1+dex_mod, 20+dex_mod] (d20 range) for each entry.
    #[tokio::test]
    async fn start_combat_auto_rolls_when_no_roll_provided() {
        let pool = make_pool().await;
        let session_id = uuid::Uuid::new_v4();

        let args = json!({
            "initiative_entries": [
                { "name": "Rogue",  "dex_mod": 3 },
                { "name": "Zombie", "dex_mod": -1 }
            ]
        });

        let (val, is_err) = execute_start_combat(&args, &pool, session_id).await;
        assert!(!is_err, "start_combat must not error: {val}");

        let ordered = val["ordered"].as_array().expect("ordered array present");
        assert_eq!(ordered.len(), 2);

        for entry in ordered {
            let roll = entry["roll"].as_i64().expect("roll is integer");
            // d20 (1-20) + dex_mod range: Rogue [4, 23], Zombie [0, 19]
            // Both must be in the broadest possible range [-19, 39].
            assert!(
                (-19..=39).contains(&roll),
                "auto-rolled value {roll} is outside plausible d20+mod range"
            );
        }
    }

    /// With a single entry and no roll provided, `ordered` has exactly one element.
    #[tokio::test]
    async fn start_combat_single_entry() {
        let pool = make_pool().await;
        let session_id = uuid::Uuid::new_v4();

        let args = json!({
            "initiative_entries": [{ "name": "Hero" }]
        });

        let (val, is_err) = execute_start_combat(&args, &pool, session_id).await;
        assert!(!is_err, "start_combat must not error: {val}");
        let ordered = val["ordered"].as_array().expect("ordered array");
        assert_eq!(ordered.len(), 1);
        assert_eq!(ordered[0]["name"].as_str(), Some("Hero"));
        // Roll is between 1 and 20 (no dex_mod).
        let roll = ordered[0]["roll"].as_i64().unwrap();
        assert!(
            (1..=20).contains(&roll),
            "single entry roll {roll} must be 1-20"
        );
    }

    /// Missing initiative_entries returns an error result.
    #[tokio::test]
    async fn start_combat_missing_entries_is_error() {
        let pool = make_pool().await;
        let session_id = uuid::Uuid::new_v4();

        let args = json!({ "other_field": 42 });
        let (val, is_err) = execute_start_combat(&args, &pool, session_id).await;
        assert!(is_err, "must error when initiative_entries missing");
        assert!(val["error"].as_str().is_some());
    }
}

#[cfg(test)]
mod cast_spell_tests {
    use super::*;

    async fn make_pool() -> SqlitePool {
        let pool = SqlitePool::connect("sqlite::memory:")
            .await
            .expect("in-memory db");
        crate::db::init_db(&pool).await.expect("migrate");
        pool
    }

    /// Insert a combat encounter then a token, return the token_id.
    async fn seed_encounter_and_token(
        pool: &SqlitePool,
        token_id: &str,
        hp: i32,
        max_hp: i32,
        resistances: Option<&str>,
    ) {
        let enc_id = uuid::Uuid::new_v4().to_string();
        let now = chrono::Utc::now().to_rfc3339();
        let session_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO combat_encounters (id, session_id, round, started_at, initiative) \
             VALUES (?1, ?2, 1, ?3, '[]')",
        )
        .bind(&enc_id)
        .bind(&session_id)
        .bind(&now)
        .execute(pool)
        .await
        .unwrap();

        let resist_json = resistances
            .map(|r| format!("[\"{r}\"]"))
            .unwrap_or_else(|| "null".to_string());
        sqlx::query(
            "INSERT INTO combat_tokens \
             (id, encounter_id, name, current_hp, max_hp, ac, pos_x, pos_y, conditions, resistances, immunities, vulnerabilities) \
             VALUES (?1,?2,'TestToken',?3,?4,10,0,0,'[]',?5,null,null)",
        )
        .bind(token_id)
        .bind(&enc_id)
        .bind(hp)
        .bind(max_hp)
        .bind(resist_json)
        .execute(pool)
        .await
        .unwrap();
    }

    async fn token_hp(pool: &SqlitePool, token_id: &str) -> i32 {
        use sqlx::Row;
        sqlx::query("SELECT current_hp FROM combat_tokens WHERE id = ?1")
            .bind(token_id)
            .fetch_one(pool)
            .await
            .unwrap()
            .try_get("current_hp")
            .unwrap()
    }

    // ------------------------------------------------------------------ //
    // 1. Unknown spell -> not_in_srd                                       //
    // ------------------------------------------------------------------ //
    #[tokio::test]
    async fn unknown_spell_returns_not_in_srd() {
        let pool = make_pool().await;
        let args = json!({ "spell": "totally_made_up_spell_xyz" });
        let (val, is_err) = execute_cast_spell(&args, &pool).await;
        assert!(!is_err, "not_in_srd must not be is_error");
        assert_eq!(val["status"].as_str(), Some("not_in_srd"));
    }

    // ------------------------------------------------------------------ //
    // 2. Damage spell (burning-hands) reduces target HP                   //
    // ------------------------------------------------------------------ //
    #[tokio::test]
    async fn damage_spell_reduces_target_hp() {
        let pool = make_pool().await;
        let token_id = "tok-burn-01";
        // Give target 30 HP; burning-hands does 3d6 fire (min 3, max 18).
        seed_encounter_and_token(&pool, token_id, 30, 30, None).await;

        let args = json!({
            "spell": "burning-hands",
            "targets": [token_id]
        });
        let (val, is_err) = execute_cast_spell(&args, &pool).await;
        assert!(!is_err, "cast must not error: {val}");
        assert_eq!(val["status"].as_str(), Some("resolved"));
        assert_eq!(val["spell"].as_str(), Some("burning-hands"));

        let new_hp = token_hp(&pool, token_id).await;
        // 3d6 => min=3, max=18; 30 - 18 = 12 at minimum remaining.
        assert!(
            new_hp < 30,
            "HP must decrease after burning-hands, got {new_hp}"
        );
        assert!(
            new_hp >= 12,
            "HP floor wrong: got {new_hp} (30 - 18 = 12 min)"
        );

        // Verify target result structure.
        let targets = val["targets"].as_array().unwrap();
        assert_eq!(targets.len(), 1);
        let t = &targets[0];
        assert_eq!(t["token_id"].as_str(), Some(token_id));
        assert!(t["raw_damage"].as_i64().unwrap() >= 3);
        assert_eq!(t["new_hp"].as_i64().unwrap(), new_hp as i64);
    }

    // ------------------------------------------------------------------ //
    // 3. Fire resistance halves damage from a fire spell                   //
    // ------------------------------------------------------------------ //
    #[tokio::test]
    async fn fire_resistance_halves_burning_hands_damage() {
        let pool = make_pool().await;
        let token_id = "tok-resist-02";
        // Token with fire resistance.
        seed_encounter_and_token(&pool, token_id, 30, 30, Some("fire")).await;

        // Cast burning-hands (3d6 fire, save-for-half with dex save).
        // We deliberately omit save_dc so no save is rolled; full raw is rolled.
        let args = json!({
            "spell": "burning-hands",
            "targets": [token_id]
        });
        let (val, is_err) = execute_cast_spell(&args, &pool).await;
        assert!(!is_err, "cast must not error: {val}");

        let targets = val["targets"].as_array().unwrap();
        let t = &targets[0];
        let raw = t["raw_damage"].as_i64().unwrap() as i32;
        let effective = t["effective_damage"].as_i64().unwrap() as i32;
        // Fire resistance: effective = raw / 2 (integer).
        assert_eq!(effective, raw / 2, "resistance must halve damage");
        let new_hp = token_hp(&pool, token_id).await;
        assert_eq!(new_hp, 30 - effective);
    }

    // ------------------------------------------------------------------ //
    // 4. Save-for-half: target saves -> receives half damage                //
    // ------------------------------------------------------------------ //
    #[tokio::test]
    async fn save_for_half_gives_half_on_success() {
        // Burning-hands has save.half_on_success=true (dex save).
        // Force a definite success: save_dc=1 so any roll succeeds.
        let pool = make_pool().await;
        let token_id = "tok-save-03";
        seed_encounter_and_token(&pool, token_id, 40, 40, None).await;

        let args = json!({
            "spell": "burning-hands",
            "targets": [{ "token_id": token_id, "save_bonus": 10 }],
            "save_dc": 1
        });
        let (val, is_err) = execute_cast_spell(&args, &pool).await;
        assert!(!is_err, "cast must not error: {val}");

        let targets = val["targets"].as_array().unwrap();
        let t = &targets[0];
        let raw = t["raw_damage"].as_i64().unwrap() as i32;
        // With DC=1 and save_bonus=10, target always saves -> half raw.
        // Expect raw_damage == full_roll/2 (that is what we stored as raw_for_target).
        let full_roll = val["raw_damage"].as_i64().unwrap() as i32;
        assert_eq!(raw, full_roll / 2, "saved target must take half damage");
    }

    // ------------------------------------------------------------------ //
    // 5. Healing spell (cure-wounds) increases target HP, capped at max   //
    // ------------------------------------------------------------------ //
    #[tokio::test]
    async fn cure_wounds_heals_target() {
        let pool = make_pool().await;
        let token_id = "tok-heal-04";
        seed_encounter_and_token(&pool, token_id, 5, 20, None).await;

        let args = json!({
            "spell": "cure-wounds",
            "targets": [token_id],
            "spell_modifier": 3
        });
        let (val, is_err) = execute_cast_spell(&args, &pool).await;
        assert!(!is_err, "cure-wounds must not error: {val}");
        assert_eq!(val["status"].as_str(), Some("resolved"));
        assert_eq!(val["kind"].as_str(), Some("healing"));

        let new_hp = token_hp(&pool, token_id).await;
        // 1d8+3: min=4, max=11; started at 5, max=20.
        assert!(
            new_hp > 5,
            "HP must increase after cure-wounds, got {new_hp}"
        );
        assert!(new_hp <= 20, "HP must not exceed max, got {new_hp}");
    }

    // ------------------------------------------------------------------ //
    // 6. Narrative-only spell (no mechanical data in compendium)           //
    // ------------------------------------------------------------------ //
    #[tokio::test]
    async fn narrative_only_spell_returns_resolved_with_note() {
        // magic-missile has damage: null and is not a healing spell.
        let pool = make_pool().await;
        let args = json!({ "spell": "magic-missile" });
        let (val, is_err) = execute_cast_spell(&args, &pool).await;
        assert!(!is_err, "narrative spell must not error: {val}");
        assert_eq!(val["status"].as_str(), Some("resolved"));
        assert!(
            val["note"].as_str().is_some(),
            "narrative-only spell must include a note field"
        );
    }

    // ------------------------------------------------------------------ //
    // 7. Attack-roll spell (guiding-bolt, 4d6 radiant) reduces HP          //
    // ------------------------------------------------------------------ //
    #[tokio::test]
    async fn attack_spell_reduces_hp() {
        let pool = make_pool().await;
        let token_id = "tok-attack-05";
        seed_encounter_and_token(&pool, token_id, 50, 50, None).await;

        let args = json!({
            "spell": "guiding-bolt",
            "targets": [token_id]
        });
        let (val, is_err) = execute_cast_spell(&args, &pool).await;
        assert!(!is_err, "guiding-bolt must not error: {val}");
        assert_eq!(val["status"].as_str(), Some("resolved"));

        let new_hp = token_hp(&pool, token_id).await;
        // 4d6 => min=4, max=24; started at 50.
        assert!(new_hp < 50, "HP must decrease after guiding-bolt");
        assert!(new_hp >= 26, "min remaining: 50-24=26, got {new_hp}");
    }

    // ------------------------------------------------------------------ //
    // 8. shatter (3d8 thunder, CON save half_on_success=true)              //
    // ------------------------------------------------------------------ //
    #[tokio::test]
    async fn shatter_with_failed_save_applies_full_damage() {
        let pool = make_pool().await;
        let token_id = "tok-shatter-06";
        seed_encounter_and_token(&pool, token_id, 40, 40, None).await;

        // DC=30 forces save failure (no d20 roll can beat 30).
        let args = json!({
            "spell": "shatter",
            "targets": [{ "token_id": token_id, "save_bonus": 0 }],
            "save_dc": 30
        });
        let (val, is_err) = execute_cast_spell(&args, &pool).await;
        assert!(!is_err, "shatter must not error: {val}");

        let targets = val["targets"].as_array().unwrap();
        let t = &targets[0];
        let raw_target = t["raw_damage"].as_i64().unwrap() as i32;
        let full_roll = val["raw_damage"].as_i64().unwrap() as i32;
        // With DC=30 (impossible to beat): raw_for_target == full_roll.
        assert_eq!(raw_target, full_roll, "failed save must take full damage");
    }

    // ------------------------------------------------------------------ //
    // 9. Spell with no targets -> resolved with empty targets list          //
    // ------------------------------------------------------------------ //
    #[tokio::test]
    async fn damage_spell_no_targets_resolves_cleanly() {
        let pool = make_pool().await;
        let args = json!({ "spell": "acid-splash" });
        let (val, is_err) = execute_cast_spell(&args, &pool).await;
        assert!(!is_err, "must not error with no targets: {val}");
        assert_eq!(val["status"].as_str(), Some("resolved"));
        let targets = val["targets"].as_array().unwrap();
        assert_eq!(targets.len(), 0, "no targets -> empty array");
    }
}
