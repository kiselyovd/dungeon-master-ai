//! Assembles the system prompt injected at the top of each LLM request.
//!
//! `base_prompt` is from `AgentConfig` (user-editable in Settings -> Model tab).
//! Phase E extends this with RAG-retrieved SRD chunks.
//! Phase G wires NPC memory injection via `load_all_npc_facts`.
//!
//! For Task C1 the NPC loader is stubbed because the `npc_memory` table is
//! introduced in migration `0002_m3_journal_npc_srd.sql` (Phase E/F/G work).

use app_domain::srd::retriever::SrdRetriever;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use sqlx::SqlitePool;
use tracing::warn;
use uuid::Uuid;

/// Build the system context string for one agent round.
pub async fn build_context(
    pool: &SqlitePool,
    campaign_id: Uuid,
    player_message: &str,
    base_prompt: &str,
    retriever: Option<&SrdRetriever>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut ctx = base_prompt.to_string();

    // RAG: inject top-5 SRD chunks relevant to the player's message.
    if let Some(ret) = retriever {
        if !ret.is_empty() {
            match embed_player_message(player_message) {
                Ok(query_emb) => {
                    let chunks = ret.retrieve_by_embedding(&query_emb, 5);
                    if !chunks.is_empty() {
                        ctx.push_str("\n\n## Relevant D&D 5e Rules\n");
                        for chunk in chunks {
                            ctx.push_str(&format!("- {}: {}\n", chunk.source_key, chunk.text_en));
                        }
                    }
                }
                Err(e) => warn!("query embedding failed: {e}"),
            }
        }
    }

    // Append NPC memory facts for NPCs currently in scope.
    // Phase G implements scene-scoped scope; for now we'd load all facts.
    let npc_facts = load_all_npc_facts(pool, campaign_id).await.unwrap_or_default();
    if !npc_facts.is_empty() {
        ctx.push_str("\n\n## Known NPCs\n");
        ctx.push_str(&npc_facts);
    }

    Ok(ctx)
}

/// Re-init on each call is wasteful in production. Phase I will cache the
/// `TextEmbedding` handle on `AppState`. For M3 correctness this is sufficient
/// because the model is cached on disk - only the ONNX session setup runs (~100ms).
fn embed_player_message(text: &str) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
    let model = TextEmbedding::try_new(InitOptions::new(EmbeddingModel::BGESmallENV15))?;
    let embeddings = model.embed(vec![text], None)?;
    embeddings
        .into_iter()
        .next()
        .ok_or_else(|| "empty embedding".into())
}

async fn load_all_npc_facts(
    _pool: &SqlitePool,
    _campaign_id: Uuid,
) -> Result<String, sqlx::Error> {
    // Phase G wires this to the npc_memory table.
    // Stubbed in Task C1 to keep the build green until the table exists.
    Ok(String::new())
}
