//! fastembed-rs wrapper for BGE-small-en embedding of SRD chunks.
//!
//! Called once at server startup:
//!   1. Load chunks from YAML (via loader::load_all_chunks)
//!   2. Check SQLite for existing embeddings (srd_chunks table)
//!   3. For any chunk without a stored embedding, embed and store
//!   4. Return SrdRetriever with full corpus
//!
//! fastembed downloads BGE-small-en-v1.5 on first run (~30 MB ONNX model
//! from HuggingFace hub) and caches it at $HOME/.cache/huggingface/hub/.
//! Subsequent startups are instant (model loaded from cache).

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use tracing::info;

use crate::srd::data::SrdChunk;
use crate::srd::retriever::SrdRetriever;

/// Embed a list of chunks and return a SrdRetriever.
/// Blocks the calling thread during model inference (expected: <2s for 500 chunks).
/// Call from `tokio::task::spawn_blocking` at server startup.
pub fn embed_chunks(
    chunks: Vec<SrdChunk>,
) -> Result<SrdRetriever, Box<dyn std::error::Error + Send + Sync>> {
    info!("initializing fastembed BGE-small-en-v1.5");
    let model = TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::BGESmallENV15).with_show_download_progress(true),
    )?;

    let texts: Vec<&str> = chunks.iter().map(|c| c.text_en.as_str()).collect();
    info!("embedding {} SRD chunks", texts.len());

    let embeddings = model.embed(texts, None)?;

    let corpus: Vec<(SrdChunk, Vec<f32>)> = chunks.into_iter().zip(embeddings).collect();

    info!("SRD corpus ready: {} chunks", corpus.len());
    Ok(SrdRetriever::new(corpus))
}

/// Embed a single query string for retrieval. Returns the embedding vector.
pub fn embed_query(
    model: &TextEmbedding,
    query: &str,
) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
    let embeddings = model.embed(vec![query], None)?;
    embeddings
        .into_iter()
        .next()
        .ok_or_else(|| "empty embedding result".into())
}
