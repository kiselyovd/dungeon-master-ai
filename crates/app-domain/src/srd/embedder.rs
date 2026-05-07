//! fastembed-rs wrapper for embedding SRD chunks.
//!
//! Called once at server startup:
//!   1. Load chunks from YAML (via loader::load_all_chunks)
//!   2. Check SQLite for existing embeddings (srd_chunks table)
//!   3. For any chunk without a stored embedding, embed and store
//!   4. Return SrdRetriever with full corpus
//!
//! fastembed downloads the chosen ONNX model on first run from HuggingFace
//! hub and caches it at `$HOME/.cache/huggingface/hub/`. Subsequent startups
//! are instant (model loaded from cache).
//!
//! The default model is `MultilingualE5Small` (384d, ~470MB) which supports
//! Russian, English, and 100+ other languages - aligning with the project's
//! bilingual RU+EN scope. Override via the `DMAI_EMBEDDING_MODEL` env var or
//! via `AgentConfig.embedding_model`.

use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use tracing::info;

use crate::srd::data::SrdChunk;
use crate::srd::retriever::SrdRetriever;

/// Default embedding model used when no override is provided.
///
/// Multilingual E5 Small: 384 dim, ~470MB download, supports 100+ languages
/// including Russian and English. Matches the project's bilingual RU+EN scope.
pub const DEFAULT_EMBEDDING_MODEL: &str = "multilingual-e5-small";

/// Parse a kebab-case model name into a `fastembed::EmbeddingModel` variant.
/// Returns Err with the rejected name if unrecognised.
///
/// Recognised names (subset of fastembed 4.x; expandable later):
/// - "multilingual-e5-small" (DEFAULT, 384d, RU+EN+100 langs)
/// - "multilingual-e5-base" (768d, RU+EN+100 langs)
/// - "multilingual-e5-large" (1024d, RU+EN+100 langs)
/// - "bge-small-en-v15" (384d, EN only)
/// - "bge-small-en-v15-q" (384d, EN only, quantized ~33MB)
/// - "bge-base-en-v15" (768d, EN only)
/// - "bge-large-en-v15" (1024d, EN only)
/// - "all-minilm-l6-v2" (384d, EN only, fastest)
/// - "all-minilm-l12-v2" (384d, EN only)
/// - "paraphrase-ml-mpnet-base-v2" (768d, multilingual paraphrase)
/// - "paraphrase-ml-minilm-l12-v2" (384d, multilingual paraphrase)
pub fn parse_embedding_model(name: &str) -> Result<EmbeddingModel, String> {
    match name.to_lowercase().as_str() {
        "multilingual-e5-small" => Ok(EmbeddingModel::MultilingualE5Small),
        "multilingual-e5-base" => Ok(EmbeddingModel::MultilingualE5Base),
        "multilingual-e5-large" => Ok(EmbeddingModel::MultilingualE5Large),
        "bge-small-en-v15" | "bge-small-en" => Ok(EmbeddingModel::BGESmallENV15),
        "bge-small-en-v15-q" | "bge-small-en-q" => Ok(EmbeddingModel::BGESmallENV15Q),
        "bge-base-en-v15" | "bge-base-en" => Ok(EmbeddingModel::BGEBaseENV15),
        "bge-large-en-v15" | "bge-large-en" => Ok(EmbeddingModel::BGELargeENV15),
        "all-minilm-l6-v2" => Ok(EmbeddingModel::AllMiniLML6V2),
        "all-minilm-l12-v2" => Ok(EmbeddingModel::AllMiniLML12V2),
        "paraphrase-ml-mpnet-base-v2" => Ok(EmbeddingModel::ParaphraseMLMpnetBaseV2),
        "paraphrase-ml-minilm-l12-v2" => Ok(EmbeddingModel::ParaphraseMLMiniLML12V2),
        other => Err(format!(
            "unknown embedding model: '{other}' (try 'multilingual-e5-small' or 'bge-small-en-v15')"
        )),
    }
}

/// Return the embedding dimension for a model. Used for cache validation
/// (a stored embedding with a different dim than the active model means the
/// cache must be cleared and re-built).
pub fn embedding_dim(model: &EmbeddingModel) -> usize {
    match model {
        EmbeddingModel::AllMiniLML6V2
        | EmbeddingModel::AllMiniLML6V2Q
        | EmbeddingModel::AllMiniLML12V2
        | EmbeddingModel::AllMiniLML12V2Q
        | EmbeddingModel::BGESmallENV15
        | EmbeddingModel::BGESmallENV15Q
        | EmbeddingModel::MultilingualE5Small
        | EmbeddingModel::ParaphraseMLMiniLML12V2
        | EmbeddingModel::ParaphraseMLMiniLML12V2Q
        | EmbeddingModel::BGESmallZHV15 => 384,
        EmbeddingModel::BGEBaseENV15
        | EmbeddingModel::BGEBaseENV15Q
        | EmbeddingModel::NomicEmbedTextV1
        | EmbeddingModel::NomicEmbedTextV15
        | EmbeddingModel::NomicEmbedTextV15Q
        | EmbeddingModel::ParaphraseMLMpnetBaseV2
        | EmbeddingModel::MultilingualE5Base
        | EmbeddingModel::GTEBaseENV15
        | EmbeddingModel::GTEBaseENV15Q
        | EmbeddingModel::JinaEmbeddingsV2BaseCode => 768,
        EmbeddingModel::BGELargeENV15
        | EmbeddingModel::BGELargeENV15Q
        | EmbeddingModel::BGELargeZHV15
        | EmbeddingModel::MultilingualE5Large
        | EmbeddingModel::MxbaiEmbedLargeV1
        | EmbeddingModel::MxbaiEmbedLargeV1Q
        | EmbeddingModel::GTELargeENV15
        | EmbeddingModel::GTELargeENV15Q
        | EmbeddingModel::ModernBertEmbedLarge => 1024,
        EmbeddingModel::ClipVitB32 => 512,
    }
}

/// Embed a list of chunks using the given model and return a SrdRetriever.
/// Blocks the calling thread during model inference (expected: <2s for 500 chunks).
/// Call from `tokio::task::spawn_blocking` at server startup.
pub fn embed_chunks(
    chunks: Vec<SrdChunk>,
    model: EmbeddingModel,
) -> Result<SrdRetriever, Box<dyn std::error::Error + Send + Sync>> {
    info!("initializing fastembed model: {:?}", model);
    let model = TextEmbedding::try_new(
        InitOptions::new(model).with_show_download_progress(true),
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
