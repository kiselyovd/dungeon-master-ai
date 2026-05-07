//! Plain Rust cosine similarity retrieval over SRD embeddings.
//!
//! v1 corpus is ~500 chunks. At 384 dimensions (BGE-small-en), the full
//! corpus embedding matrix is ~750 KB. Loading all rows and computing cosine
//! in-process is fast (<5ms on any modern CPU) and avoids sqlite-vec or HNSW.

use crate::srd::data::SrdChunk;

/// Cosine similarity between two equal-length float vectors.
/// Returns 0.0 if either vector is zero-length.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "vectors must have equal length");
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

/// Return the top-K chunks sorted by descending cosine similarity to `query`.
/// Chunks without an embedding are skipped.
pub fn top_k_by_cosine<'a>(
    query: &[f32],
    corpus: &'a [(SrdChunk, Vec<f32>)],
    k: usize,
) -> Vec<(f32, &'a SrdChunk)> {
    let mut scores: Vec<(f32, &'a SrdChunk)> = corpus
        .iter()
        .map(|(chunk, emb)| (cosine_similarity(query, emb), chunk))
        .collect();
    scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
    scores.truncate(k);
    scores
}

/// A loaded and embedded SRD corpus ready for retrieval.
pub struct SrdRetriever {
    corpus: Vec<(SrdChunk, Vec<f32>)>,
}

impl SrdRetriever {
    pub fn new(corpus: Vec<(SrdChunk, Vec<f32>)>) -> Self {
        Self { corpus }
    }

    /// Return up to `k` most relevant chunks for `query_text` using the
    /// pre-computed query embedding.
    pub fn retrieve_by_embedding(&self, query_embedding: &[f32], k: usize) -> Vec<&SrdChunk> {
        top_k_by_cosine(query_embedding, &self.corpus, k)
            .into_iter()
            .map(|(_, chunk)| chunk)
            .collect()
    }

    pub fn len(&self) -> usize {
        self.corpus.len()
    }

    pub fn is_empty(&self) -> bool {
        self.corpus.is_empty()
    }
}
