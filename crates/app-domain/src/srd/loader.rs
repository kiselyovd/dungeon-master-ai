//! Loads all SRD YAML files bundled at compile time via `include_str!`.
//! On first startup `load_all_chunks()` is called and the results are
//! passed to `embedder::embed_chunks` then stored in SQLite.

use crate::srd::data::{load_chunks_from_yaml, SrdChunk};

static SPELLS_YAML: &str = include_str!("../../srd/spells.yaml");
static MONSTERS_YAML: &str = include_str!("../../srd/monsters.yaml");
static RULES_YAML: &str = include_str!("../../srd/rules.yaml");
static CLASSES_YAML: &str = include_str!("../../srd/classes.yaml");

/// Load all SRD chunks from embedded YAML data.
pub fn load_all_chunks() -> Vec<SrdChunk> {
    let mut chunks = Vec::new();
    for yaml in &[SPELLS_YAML, MONSTERS_YAML, RULES_YAML, CLASSES_YAML] {
        match load_chunks_from_yaml(yaml) {
            Ok(c) => chunks.extend(c),
            Err(e) => tracing::warn!("SRD yaml parse error: {e}"),
        }
    }
    chunks
}
