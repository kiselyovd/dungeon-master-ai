use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use std::sync::Mutex;

use crate::provider::{ChatChunk, ChatRequest, ChunkStream, LlmError, LlmProvider};

pub struct MockProvider {
    scripted: Mutex<Vec<ChatChunk>>,
}

impl MockProvider {
    pub fn new(scripted: Vec<ChatChunk>) -> Self {
        Self {
            scripted: Mutex::new(scripted),
        }
    }

    /// Load a fresh set of scripted chunks. Useful for multi-turn tests where
    /// callers need to set up round-2 chunks after consuming round-1 chunks.
    pub fn set_chunks(&self, chunks: Vec<ChatChunk>) {
        *self.scripted.lock().expect("mock lock poisoned") = chunks;
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn stream_chat(&self, _req: ChatRequest) -> Result<ChunkStream, LlmError> {
        let chunks: Vec<ChatChunk> = {
            let mut guard = self.scripted.lock().expect("mock provider lock poisoned");
            std::mem::take(&mut *guard)
        };
        let stream = stream::iter(chunks.into_iter().map(Ok)).boxed();
        Ok(stream)
    }

    fn name(&self) -> &'static str {
        "mock"
    }

    fn supports_vision(&self) -> bool {
        true
    }
}
