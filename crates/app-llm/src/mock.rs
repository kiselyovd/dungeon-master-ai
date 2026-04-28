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
}
