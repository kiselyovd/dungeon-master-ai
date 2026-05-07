//! Assembles the system prompt injected at the top of each LLM request.
//!
//! `base_prompt` is from `AgentConfig` (user-editable in Settings -> Model tab).
//! Phase E extends this with RAG-retrieved SRD chunks.
//! Phase G wires NPC memory injection via `load_all_npc_facts`.
//!
//! For Task C1 the NPC loader is stubbed because the `npc_memory` table is
//! introduced in migration `0002_m3_journal_npc_srd.sql` (Phase E/F/G work).

use sqlx::SqlitePool;
use uuid::Uuid;

/// Build the system context string for one agent round.
pub async fn build_context(
    pool: &SqlitePool,
    campaign_id: Uuid,
    _player_message: &str,
    base_prompt: &str,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut ctx = base_prompt.to_string();

    // Append NPC memory facts for NPCs currently in scope.
    // Phase G implements scene-scoped scope; for now we'd load all facts.
    let npc_facts = load_all_npc_facts(pool, campaign_id).await.unwrap_or_default();
    if !npc_facts.is_empty() {
        ctx.push_str("\n\n## Known NPCs\n");
        ctx.push_str(&npc_facts);
    }

    // Phase E: SRD RAG chunks appended here.
    // Phase G: NPC memory scoped to scene.

    Ok(ctx)
}

async fn load_all_npc_facts(
    _pool: &SqlitePool,
    _campaign_id: Uuid,
) -> Result<String, sqlx::Error> {
    // Phase G wires this to the npc_memory table.
    // Stubbed in Task C1 to keep the build green until the table exists.
    Ok(String::new())
}
