use app_domain::srd::data::load_chunks_from_yaml;

#[test]
fn spells_yaml_loads_nonzero_chunks() {
    let yaml = include_str!("../srd/spells.yaml");
    let chunks = load_chunks_from_yaml(yaml).unwrap();
    assert!(!chunks.is_empty(), "spells.yaml must have at least one chunk");
    for chunk in &chunks {
        assert!(!chunk.source_key.is_empty());
        assert!(!chunk.text_en.is_empty());
    }
}

#[test]
fn monsters_yaml_loads_nonzero_chunks() {
    let yaml = include_str!("../srd/monsters.yaml");
    let chunks = load_chunks_from_yaml(yaml).unwrap();
    assert!(!chunks.is_empty());
}

#[test]
fn rules_yaml_loads_nonzero_chunks() {
    let yaml = include_str!("../srd/rules.yaml");
    let chunks = load_chunks_from_yaml(yaml).unwrap();
    assert!(chunks.len() >= 10, "rules.yaml should have at least 10 chunks");
}

#[test]
fn classes_yaml_loads_nonzero_chunks() {
    let yaml = include_str!("../srd/classes.yaml");
    let chunks = load_chunks_from_yaml(yaml).unwrap();
    assert_eq!(chunks.len(), 4, "expected exactly 4 class chunks");
}

#[test]
fn chunk_text_is_non_trivially_long() {
    let yaml = include_str!("../srd/spells.yaml");
    let chunks = load_chunks_from_yaml(yaml).unwrap();
    // Each chunk should have enough text to be meaningful for embedding.
    for chunk in &chunks {
        assert!(chunk.text_en.len() >= 30, "chunk '{}' text too short", chunk.source_key);
    }
}
