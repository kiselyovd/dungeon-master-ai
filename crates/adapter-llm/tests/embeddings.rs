use adapter_llm::embeddings::{
    embedding_dim, load_all_chunks, parse_embedding_model, DEFAULT_EMBEDDING_MODEL,
};

#[test]
fn parses_supported_model_names_and_dimensions_without_loading_weights() {
    for (name, dimension) in [
        (DEFAULT_EMBEDDING_MODEL, 384),
        ("Multilingual-E5-Small", 384),
        ("bge-small-en-v15-q", 384),
        ("multilingual-e5-base", 768),
        ("bge-large-en-v15", 1024),
    ] {
        let model = parse_embedding_model(name).unwrap();
        assert_eq!(embedding_dim(&model), dimension, "model={name}");
    }
}

#[test]
fn rejects_unknown_embedding_model_with_safe_identifier_context() {
    let error = parse_embedding_model("not-a-real-model").unwrap_err();
    assert!(error.contains("not-a-real-model"));
}

#[test]
fn loads_the_embedded_srd_corpus_without_runtime_services() {
    let chunks = load_all_chunks();
    assert!(chunks.len() >= 10);
    assert!(chunks.iter().all(|chunk| !chunk.source_key.is_empty()));
}
