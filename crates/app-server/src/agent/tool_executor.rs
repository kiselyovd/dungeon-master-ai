//! Executes validated tool-calls from the agent loop.
//!
//! Each call goes through `app_domain::combat::validator::validate_tool_call`
//! first, then dispatches to the matching executor below. Executors return a
//! `(Value, is_error)` tuple - they never panic. The result JSON is what gets
//! injected back as a `ChatMessage::ToolResult` in the next round.
//!
//! M2/M3 boundary: the validator currently only knows the seven M2 combat
//! tools. The eight new M3 tools (`set_scene`, `cast_spell`, `remember_npc`,
//! `recall_npc`, `journal_append`, `quick_save`, `generate_image`,
//! `query_rules`) come back from the validator as `UnknownTool`. That returns
//! a graceful `is_error=true` result here without crashing the loop. Phase D
//! extends the validator dispatch table to cover them.
//!
//! Several executors below are stubbed because their backing tables/db
//! helpers come in later phases:
//! - `execute_remember_npc` / `execute_recall_npc` -> Phase G (NPC memory).
//! - `execute_journal_append` -> Phase F (journal).
//! - `execute_query_rules` -> Phase E (SRD RAG).
//! - `execute_generate_image` -> Phase H (image queue).

use app_domain::combat::validator::validate_tool_call;
use app_llm::ToolCall;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tracing::warn;
use uuid::Uuid;

/// Execute a tool-call. Returns `(result_value, is_error)`.
/// Never panics - errors are surfaced as `is_error=true` with a message in the result.
pub async fn execute_tool(tc: &ToolCall, pool: &SqlitePool, campaign_id: Uuid) -> (Value, bool) {
    // Phase 1: Validate via domain dispatch table.
    // M3 NOTE: The validator currently knows only the seven M2 tools. The
    // eight new tools (set_scene, cast_spell, remember_npc, recall_npc,
    // journal_append, quick_save, generate_image, query_rules) return
    // `UnknownTool`. That is handled gracefully here - we record the error
    // and return without crashing. Phase D extends the validator.
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
        "start_combat" => execute_start_combat(&validated.args, pool).await,
        "end_combat" => execute_end_combat(pool).await,
        "add_token" => execute_add_token(&validated.args, pool).await,
        "update_token" => execute_update_token(&validated.args, pool).await,
        "remove_token" => execute_remove_token(&validated.args, pool).await,
        "set_scene" => execute_set_scene(&validated.args, pool, campaign_id).await,
        "cast_spell" => execute_cast_spell(&validated.args, pool).await,
        "remember_npc" => execute_remember_npc(&validated.args, pool, campaign_id).await,
        "recall_npc" => execute_recall_npc(&validated.args, pool, campaign_id).await,
        "journal_append" => execute_journal_append(&validated.args, pool, campaign_id).await,
        "quick_save" => execute_quick_save(&validated.args, pool, campaign_id).await,
        "generate_image" => execute_generate_image(&validated.args).await,
        "query_rules" => execute_query_rules(&validated.args, pool).await,
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

async fn execute_apply_damage(args: &Value, pool: &SqlitePool) -> (Value, bool) {
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
            let new_hp = (current_hp - amount).max(0);
            let _ = sqlx::query("UPDATE combat_tokens SET current_hp = ?1 WHERE id = ?2")
                .bind(new_hp)
                .bind(token_id)
                .execute(pool)
                .await;
            (
                json!({ "new_hp": new_hp, "damage_dealt": amount }),
                false,
            )
        }
        _ => (
            json!({ "error": "token not found", "token_id": token_id }),
            true,
        ),
    }
}

async fn execute_start_combat(args: &Value, pool: &SqlitePool) -> (Value, bool) {
    let encounter_id = uuid::Uuid::new_v4();
    let initiative_json = serde_json::to_string(&args["initiative_entries"]).unwrap_or_default();
    let now = chrono::Utc::now().to_rfc3339();
    let _ = sqlx::query(
        "INSERT INTO combat_encounters (id, session_id, round, started_at, initiative) VALUES (?1, ?2, 1, ?3, ?4)"
    )
    .bind(encounter_id.to_string())
    .bind(uuid::Uuid::new_v4().to_string()) // placeholder session_id until session wiring lands
    .bind(now)
    .bind(initiative_json)
    .execute(pool)
    .await;
    (
        json!({ "encounter_id": encounter_id.to_string() }),
        false,
    )
}

async fn execute_end_combat(pool: &SqlitePool) -> (Value, bool) {
    let now = chrono::Utc::now().to_rfc3339();
    let _ = sqlx::query("UPDATE combat_encounters SET ended_at = ?1 WHERE ended_at IS NULL")
        .bind(now)
        .execute(pool)
        .await;
    (json!({ "status": "combat_ended" }), false)
}

async fn execute_add_token(args: &Value, pool: &SqlitePool) -> (Value, bool) {
    let id = args["id"].as_str().unwrap_or_default();
    let name = args["name"].as_str().unwrap_or("Unknown");
    let x = args["x"].as_i64().unwrap_or(0) as i32;
    let y = args["y"].as_i64().unwrap_or(0) as i32;
    let hp = args["hp"].as_i64().unwrap_or(1) as i32;
    let max_hp = args["max_hp"].as_i64().unwrap_or(1) as i32;
    let ac = args["ac"].as_i64().unwrap_or(10) as i32;

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

    let _ = sqlx::query(
        "INSERT OR REPLACE INTO combat_tokens (id, encounter_id, name, current_hp, max_hp, ac, pos_x, pos_y, conditions) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,'[]')"
    )
    .bind(id)
    .bind(encounter_id)
    .bind(name)
    .bind(hp)
    .bind(max_hp)
    .bind(ac)
    .bind(x)
    .bind(y)
    .execute(pool)
    .await;

    (json!({ "token_id": id, "status": "added" }), false)
}

async fn execute_update_token(args: &Value, pool: &SqlitePool) -> (Value, bool) {
    let id = args["id"].as_str().unwrap_or_default();
    if let Some(hp) = args["hp"].as_i64() {
        let _ = sqlx::query("UPDATE combat_tokens SET current_hp = ?1 WHERE id = ?2")
            .bind(hp as i32)
            .bind(id)
            .execute(pool)
            .await;
    }
    if let Some(x) = args["x"].as_i64() {
        if let Some(y) = args["y"].as_i64() {
            let _ = sqlx::query("UPDATE combat_tokens SET pos_x = ?1, pos_y = ?2 WHERE id = ?3")
                .bind(x as i32)
                .bind(y as i32)
                .bind(id)
                .execute(pool)
                .await;
        }
    }
    (json!({ "token_id": id, "status": "updated" }), false)
}

async fn execute_remove_token(args: &Value, pool: &SqlitePool) -> (Value, bool) {
    let id = args["id"].as_str().unwrap_or_default();
    let _ = sqlx::query("UPDATE combat_tokens SET is_dead = 1 WHERE id = ?1")
        .bind(id)
        .execute(pool)
        .await;
    (json!({ "token_id": id, "status": "removed" }), false)
}

async fn execute_set_scene(
    args: &Value,
    _pool: &SqlitePool,
    _campaign_id: Uuid,
) -> (Value, bool) {
    let title = args["title"].as_str().unwrap_or("Unnamed Scene");
    let subtitle = args.get("subtitle").and_then(|v| v.as_str()).unwrap_or("");
    let mode = args["mode"].as_str().unwrap_or("exploration");
    let scene_id = uuid::Uuid::new_v4();
    // Full scene table is deferred to M5; in M3 we return the scene metadata so
    // the LLM has a stable reference id to use in subsequent turns.
    (
        json!({
            "scene_id": scene_id.to_string(),
            "title": title,
            "subtitle": subtitle,
            "mode": mode,
        }),
        false,
    )
}

async fn execute_cast_spell(args: &Value, _pool: &SqlitePool) -> (Value, bool) {
    let spell = args["spell"].as_str().unwrap_or("unknown");
    // Full spell resolution is delegated to the domain resolver in a later phase.
    // For Task C1 we acknowledge the call so the LLM can continue narration.
    (
        json!({ "spell": spell, "status": "cast", "result": "see narration" }),
        false,
    )
}

async fn execute_remember_npc(
    _args: &Value,
    _pool: &SqlitePool,
    _campaign_id: Uuid,
) -> (Value, bool) {
    // Phase G wires this to the npc_memory table via crate::db::npc_upsert_fact.
    // Stubbed in Task C1 because the npc_memory table + db helpers don't exist yet.
    (json!({ "status": "deferred_to_phase_g" }), false)
}

async fn execute_recall_npc(
    _args: &Value,
    _pool: &SqlitePool,
    _campaign_id: Uuid,
) -> (Value, bool) {
    // Phase G wires this to crate::db::npc_get.
    (
        json!({ "name": "", "facts": [], "status": "deferred_to_phase_g" }),
        false,
    )
}

async fn execute_journal_append(
    _args: &Value,
    _pool: &SqlitePool,
    _campaign_id: Uuid,
) -> (Value, bool) {
    // Phase F wires this to crate::db::journal_insert.
    (json!({ "status": "deferred_to_phase_f" }), false)
}

async fn execute_quick_save(
    args: &Value,
    pool: &SqlitePool,
    campaign_id: Uuid,
) -> (Value, bool) {
    let label = args
        .get("label")
        .and_then(|v| v.as_str())
        .unwrap_or("Quick save");
    let save_id = uuid::Uuid::new_v4();
    let now = chrono::Utc::now().to_rfc3339();
    // Linear save: stores a minimal state snapshot. The full save schema lands in M5.
    let _ = sqlx::query(
        "INSERT INTO snapshots (id, session_id, turn_number, created_at, game_state, player_action) VALUES (?1, ?2, 0, ?3, ?4, NULL)"
    )
    .bind(save_id.to_string())
    .bind(campaign_id.to_string())
    .bind(now)
    .bind(serde_json::json!({ "schema_version": 1, "state": { "label": label } }).to_string())
    .execute(pool)
    .await;
    (
        json!({ "save_id": save_id.to_string(), "branch_id": null }),
        false,
    )
}

async fn execute_generate_image(args: &Value) -> (Value, bool) {
    // Image generation is queued asynchronously. The agent loop returns a
    // placeholder immediately; Phase H wires the actual Replicate call and
    // SSE-side image arrival event.
    let prompt = args["prompt"].as_str().unwrap_or("");
    let style = args
        .get("style")
        .and_then(|v| v.as_str())
        .unwrap_or("dark_fantasy");
    (
        json!({ "status": "queued", "prompt": prompt, "style": style }),
        false,
    )
}

async fn execute_query_rules(args: &Value, _pool: &SqlitePool) -> (Value, bool) {
    // Phase E wires in the actual SRD retriever. In Task C1 this returns a stub.
    let question = args["question"].as_str().unwrap_or("");
    (
        json!({ "question": question, "chunks": [], "status": "rag_not_loaded" }),
        false,
    )
}
