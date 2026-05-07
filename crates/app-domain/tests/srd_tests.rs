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

#[test]
fn cosine_similarity_orthogonal_vectors_is_zero() {
    use app_domain::srd::retriever::cosine_similarity;
    let a = vec![1.0f32, 0.0, 0.0];
    let b = vec![0.0f32, 1.0, 0.0];
    let sim = cosine_similarity(&a, &b);
    assert!((sim - 0.0).abs() < 1e-6, "orthogonal should be ~0, got {sim}");
}

#[test]
fn cosine_similarity_identical_is_one() {
    use app_domain::srd::retriever::cosine_similarity;
    let a = vec![0.3f32, 0.5, 0.8];
    let sim = cosine_similarity(&a, &a);
    assert!((sim - 1.0).abs() < 1e-5, "identical should be ~1, got {sim}");
}

#[test]
fn top_k_returns_at_most_k_results() {
    use app_domain::srd::data::SrdChunk;
    use app_domain::srd::retriever::top_k_by_cosine;

    let query = vec![1.0f32, 0.0, 0.0];
    let chunks: Vec<(SrdChunk, Vec<f32>)> = (0..10)
        .map(|i| {
            let mut emb = vec![0.0f32; 3];
            emb[i % 3] = 1.0;
            (SrdChunk::new(format!("chunk_{i}"), format!("text {i}")), emb)
        })
        .collect();

    let results = top_k_by_cosine(&query, &chunks, 3);
    assert!(results.len() <= 3, "should return at most 3 results");
}

#[test]
fn parse_embedding_model_recognises_default() {
    use app_domain::srd::embedder::{DEFAULT_EMBEDDING_MODEL, parse_embedding_model};
    assert!(parse_embedding_model(DEFAULT_EMBEDDING_MODEL).is_ok());
}

#[test]
fn parse_embedding_model_recognises_multilingual() {
    use app_domain::srd::embedder::parse_embedding_model;
    assert!(parse_embedding_model("multilingual-e5-small").is_ok());
    assert!(parse_embedding_model("multilingual-e5-base").is_ok());
    assert!(parse_embedding_model("multilingual-e5-large").is_ok());
}

#[test]
fn parse_embedding_model_recognises_bge() {
    use app_domain::srd::embedder::parse_embedding_model;
    assert!(parse_embedding_model("bge-small-en-v15").is_ok());
    assert!(parse_embedding_model("bge-small-en-v15-q").is_ok());
    assert!(parse_embedding_model("bge-base-en-v15").is_ok());
    assert!(parse_embedding_model("bge-large-en-v15").is_ok());
}

#[test]
fn parse_embedding_model_is_case_insensitive() {
    use app_domain::srd::embedder::parse_embedding_model;
    assert!(parse_embedding_model("Multilingual-E5-Small").is_ok());
    assert!(parse_embedding_model("BGE-SMALL-EN-V15").is_ok());
}

#[test]
fn parse_embedding_model_unknown_errors() {
    use app_domain::srd::embedder::parse_embedding_model;
    let err = parse_embedding_model("not-a-real-model").unwrap_err();
    assert!(err.contains("not-a-real-model"));
}

#[test]
fn embedding_dim_matches_known_dims() {
    use app_domain::srd::embedder::{embedding_dim, parse_embedding_model};
    assert_eq!(
        embedding_dim(&parse_embedding_model("multilingual-e5-small").unwrap()),
        384
    );
    assert_eq!(
        embedding_dim(&parse_embedding_model("bge-small-en-v15").unwrap()),
        384
    );
    assert_eq!(
        embedding_dim(&parse_embedding_model("multilingual-e5-base").unwrap()),
        768
    );
    assert_eq!(
        embedding_dim(&parse_embedding_model("multilingual-e5-large").unwrap()),
        1024
    );
}
