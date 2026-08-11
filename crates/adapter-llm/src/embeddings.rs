use app_domain::srd::data::{load_chunks_from_yaml, SrdChunk};
use app_domain::srd::retriever::SrdRetriever;
use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
use tracing::{info, warn};

pub const DEFAULT_EMBEDDING_MODEL: &str = "multilingual-e5-small";

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

pub fn embed_chunks(
    chunks: Vec<SrdChunk>,
    model: EmbeddingModel,
) -> Result<SrdRetriever, Box<dyn std::error::Error + Send + Sync>> {
    info!(model = ?model, "initializing embedding adapter");
    let model = TextEmbedding::try_new(InitOptions::new(model).with_show_download_progress(true))?;
    let texts: Vec<&str> = chunks.iter().map(|chunk| chunk.text_en.as_str()).collect();
    info!(chunk_count = texts.len(), "embedding SRD chunks");
    let embeddings = model.embed(texts, None)?;
    let corpus = chunks.into_iter().zip(embeddings).collect();
    Ok(SrdRetriever::new(corpus))
}

pub fn embed_query(
    model: &TextEmbedding,
    query: &str,
) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
    model
        .embed(vec![query], None)?
        .into_iter()
        .next()
        .ok_or_else(|| "empty embedding result".into())
}

pub fn embed_query_by_name(
    query: &str,
    model_name: &str,
) -> Result<Vec<f32>, Box<dyn std::error::Error + Send + Sync>> {
    let model = parse_embedding_model(model_name)
        .map_err(|error| -> Box<dyn std::error::Error + Send + Sync> { error.into() })?;
    let model = TextEmbedding::try_new(InitOptions::new(model))?;
    embed_query(&model, query)
}

static SPELLS_YAML: &str = include_str!("../../app-domain/srd/spells.yaml");
static MONSTERS_YAML: &str = include_str!("../../app-domain/srd/monsters.yaml");
static RULES_YAML: &str = include_str!("../../app-domain/srd/rules.yaml");
static CLASSES_YAML: &str = include_str!("../../app-domain/srd/classes.yaml");

pub fn load_all_chunks() -> Vec<SrdChunk> {
    let mut chunks = Vec::new();
    for yaml in [SPELLS_YAML, MONSTERS_YAML, RULES_YAML, CLASSES_YAML] {
        match load_chunks_from_yaml(yaml) {
            Ok(parsed) => chunks.extend(parsed),
            Err(error) => warn!(error = %error, "SRD yaml parse failed"),
        }
    }
    chunks
}
