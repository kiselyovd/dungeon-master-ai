//! Assembles the system prompt injected at the top of each LLM request.
//!
//! `base_prompt` is from `AgentConfig` (user-editable in Settings -> Model tab).
//! Phase E extends this with RAG-retrieved SRD chunks.
//! Phase G wires NPC memory injection via `load_all_npc_facts`.
//!
//! For Task C1 the NPC loader is stubbed because the `npc_memory` table is
//! introduced in migration `0002_m3_journal_npc_srd.sql` (Phase E/F/G work).

use app_domain::srd::embedder::parse_embedding_model;
use app_domain::srd::retriever::SrdRetriever;
use fastembed::{InitOptions, TextEmbedding};
use sqlx::SqlitePool;
use tracing::warn;
use uuid::Uuid;

use crate::agent::tools::ToolAvailability;

/// Prepend a concise Dungeon-Master operating directive to the user's custom
/// system prompt. Small local models (Gemma 4 E2B in particular) ship with an
/// empty/minimal user prompt, drift out of character, and either narrate a tool
/// instead of calling it or spiral into long internal deliberation that ends in
/// an empty turn. This scaffold pins the role, demands concise output + decisive
/// action, and - when image generation is available - explicitly tells the model
/// to CALL `generate_image` rather than describe a scene in prose.
pub(crate) fn compose_system_prompt(base: &str, availability: ToolAvailability) -> String {
    let mut s = String::from(
        "You are the Dungeon Master of a Dungeons & Dragons 5e game. Narrate vividly \
         but concisely in the second person (\"you see ...\"). Decide quickly and keep \
         any internal deliberation to a sentence or two - never stall or loop. Prefer \
         calling the provided tools over describing in prose what a tool would do.",
    );
    if availability.image {
        s.push_str(
            " When the party arrives somewhere new, the scene changes, or the player asks \
             to see, draw, show, or illustrate a place, character, item, or map, \
             immediately call the generate_image tool with a short concrete visual prompt \
             instead of only describing it in words.",
        );
    }
    let base = base.trim();
    if !base.is_empty() {
        s.push_str("\n\n");
        s.push_str(base);
    }
    s
}

/// D&D 5e mechanics terms. When the player's message mentions one (or combat is
/// active), the turn likely needs rules grounding, so we inject SRD chunks. Pure
/// narration/movement/dialogue ("I step into the crypt") matches none, so RAG is
/// skipped - feeding 5 irrelevant rule chunks to a small local model on every
/// turn is the noise that makes Gemma 4 E2B deliberate instead of acting.
const RULES_TERMS: &[&str] = &[
    "attack",
    "damage",
    "hit point",
    " hp",
    "armor class",
    " ac ",
    "saving throw",
    " save",
    " check",
    "roll",
    "dice",
    "d20",
    "spell",
    "cast",
    "condition",
    "grapple",
    "initiative",
    "advantage",
    "disadvantage",
    "modifier",
    "proficien",
    "ability score",
    " dc ",
    "death save",
    "concentration",
    "resistance",
    "immune",
    "stealth",
    "perception",
];

/// Decide whether this turn needs SRD rule chunks injected. True when combat is
/// active or the message references a 5e mechanic; false for plain narration.
pub fn needs_rules_context(message: &str, in_combat: bool) -> bool {
    if in_combat {
        return true;
    }
    let lower = format!(" {} ", message.to_ascii_lowercase());
    RULES_TERMS.iter().any(|t| lower.contains(t))
}

/// Build the system context string for one agent round.
///
/// `model_name` is the kebab-case embedding model id (e.g. "multilingual-e5-small")
/// from `AgentConfig.embedding_model`. It must agree with the model used to
/// build the retriever's corpus, otherwise query/corpus vectors live in
/// different spaces and similarity is meaningless.
///
/// `inject_rules` gates RAG: SRD chunks are only retrieved when the turn needs
/// rules (see `needs_rules_context`), keeping the context lean for small models.
pub async fn build_context(
    pool: &SqlitePool,
    campaign_id: Uuid,
    player_message: &str,
    base_prompt: &str,
    model_name: &str,
    retriever: Option<&SrdRetriever>,
    inject_rules: bool,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut ctx = base_prompt.to_string();

    // RAG: inject top-3 SRD chunks relevant to the player's message, but only on
    // rules-relevant turns - skipping it on narration turns keeps small local
    // models from over-deliberating on irrelevant rule text.
    if inject_rules {
        if let Some(ret) = retriever {
            if !ret.is_empty() {
                match embed_player_message(player_message, model_name) {
                    Ok(query_emb) => {
                        let chunks = ret.retrieve_by_embedding(&query_emb, 3);
                        if !chunks.is_empty() {
                            ctx.push_str("\n\n## Relevant D&D 5e Rules\n");
                            for chunk in chunks {
                                ctx.push_str(&format!(
                                    "- {}: {}\n",
                                    chunk.source_key, chunk.text_en
                                ));
                            }
                        }
                    }
                    Err(e) => warn!("query embedding failed: {e}"),
                }
            }
        }
    }

    // Append NPC memory facts for NPCs currently in scope.
    // Phase G implements scene-scoped scope; for now we'd load all facts.
    let npc_facts = load_all_npc_facts(pool, campaign_id)
        .await
        .unwrap_or_default();
    if !npc_facts.is_empty() {
        ctx.push_str("\n\n## Known NPCs\n");
        ctx.push_str(&npc_facts);
    }

    Ok(ctx)
}

/// Re-init on each call is wasteful in production. Phase I will cache the
/// `TextEmbedding` handle on `AppState`. For M3 correctness this is sufficient
/// because the model is cached on disk - only the ONNX session setup runs (~100ms).
fn embed_player_message(
    text: &str,
    model_name: &str,
) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
    let parsed = parse_embedding_model(model_name)
        .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> { e.into() })?;
    let model = TextEmbedding::try_new(InitOptions::new(parsed))?;
    let embeddings = model.embed(vec![text], None)?;
    embeddings
        .into_iter()
        .next()
        .ok_or_else(|| "empty embedding".into())
}

async fn load_all_npc_facts(_pool: &SqlitePool, _campaign_id: Uuid) -> Result<String, sqlx::Error> {
    // Phase G wires this to the npc_memory table.
    // Stubbed in Task C1 to keep the build green until the table exists.
    Ok(String::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn avail(image: bool) -> ToolAvailability {
        ToolAvailability {
            image,
            video: false,
        }
    }

    #[test]
    fn compose_pins_dm_role_and_concision() {
        let s = compose_system_prompt("", avail(false));
        assert!(s.contains("Dungeon Master"));
        assert!(s.contains("concisely"));
    }

    #[test]
    fn compose_adds_generate_image_directive_only_when_image_enabled() {
        assert!(compose_system_prompt("", avail(true)).contains("generate_image"));
        assert!(!compose_system_prompt("", avail(false)).contains("generate_image"));
    }

    #[test]
    fn compose_appends_user_prompt_after_scaffold() {
        let s = compose_system_prompt("Be a grim, terse DM.", avail(false));
        let scaffold_end = s.find("Be a grim").expect("user prompt present");
        assert!(scaffold_end > 0, "user prompt must come AFTER the scaffold");
        assert!(s.contains("Dungeon Master"));
    }

    #[test]
    fn compose_ignores_blank_user_prompt() {
        let s = compose_system_prompt("   \n  ", avail(false));
        assert!(
            !s.contains("\n\n"),
            "blank base must not add a trailing block"
        );
    }

    #[test]
    fn needs_rules_skips_plain_narration() {
        assert!(!needs_rules_context(
            "I step into the ancient torchlit crypt and look around.",
            false
        ));
        assert!(!needs_rules_context(
            "I greet the innkeeper and ask for a room.",
            false
        ));
    }

    #[test]
    fn needs_rules_fires_on_mechanics_terms() {
        assert!(needs_rules_context(
            "I attack the goblin with my sword.",
            false
        ));
        assert!(needs_rules_context(
            "Do I need a saving throw for that?",
            false
        ));
        assert!(needs_rules_context("I cast fireball at the group.", false));
    }

    #[test]
    fn needs_rules_always_true_in_combat() {
        assert!(needs_rules_context("I look around nervously.", true));
    }
}
