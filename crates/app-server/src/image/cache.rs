//! Image cache key computation.
//!
//! `key = fnv1a_64(scene_id + sorted(npc_ids) + style_preset)` rendered as 16-char hex.
//! FNV-1a 64-bit is collision-rare for the small key space we expect (~thousands of
//! distinct prompts per campaign) and avoids pulling in the `sha2` crate just for this.
//! M5 can upgrade to SHA-256 if collisions ever matter at scale.

use crate::image::provider::ImagePrompt;

/// Compute a deterministic cache key for an image prompt.
/// NPC ids are sorted so the order in which the LLM lists them does not affect the key.
pub fn image_cache_key(prompt: &ImagePrompt) -> String {
    let mut npc_ids = prompt.npc_ids.clone();
    npc_ids.sort();
    let raw = format!(
        "{}|{}|{}",
        prompt.scene_id.as_deref().unwrap_or(""),
        npc_ids.join(","),
        prompt.style_preset,
    );
    let hash = fnv1a_64(raw.as_bytes());
    format!("{:016x}", hash)
}

fn fnv1a_64(data: &[u8]) -> u64 {
    const OFFSET: u64 = 14695981039346656037;
    const PRIME: u64 = 1099511628211;
    let mut hash = OFFSET;
    for byte in data {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(PRIME);
    }
    hash
}
