use async_trait::async_trait;
use futures::stream::{self, StreamExt};
use std::sync::Mutex;

use crate::provider::{Capabilities, ChatChunk, ChatRequest, ChunkStream, LlmError, LlmProvider};

pub struct MockProvider {
    scripted: Mutex<Vec<ChatChunk>>,
    capabilities: Capabilities,
    model: String,
    /// Optional thinking/reasoning chunks to emit BEFORE the scripted chunks.
    thinking_chunks: Vec<String>,
}

impl MockProvider {
    /// Construct a mock with scripted chunks. Caps default to all-true so
    /// existing tests that assume vision + tools keep working without
    /// requiring opt-in. Default active model is `"mock-default"`.
    pub fn new(scripted: Vec<ChatChunk>) -> Self {
        Self {
            scripted: Mutex::new(scripted),
            capabilities: Capabilities {
                vision_input: true,
                reasoning: true,
                tool_calls: true,
                streaming: true,
            },
            model: "mock-default".to_string(),
            thinking_chunks: Vec::new(),
        }
    }

    /// Load a fresh set of scripted chunks. Useful for multi-turn tests where
    /// callers need to set up round-2 chunks after consuming round-1 chunks.
    pub fn set_chunks(&self, chunks: Vec<ChatChunk>) {
        *self.scripted.lock().expect("mock lock poisoned") = chunks;
    }

    /// Override the capabilities reported by this mock. Chainable.
    pub fn with_capabilities(mut self, capabilities: Capabilities) -> Self {
        self.capabilities = capabilities;
        self
    }

    /// Override the active model id reported by this mock. Chainable.
    pub fn with_active_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    /// Prepend thinking/reasoning delta chunks before the scripted chunks.
    /// Each string in `chunks` becomes a `ChatChunk::ThinkingDelta`. Chainable.
    pub fn with_thinking_chunks(mut self, chunks: Vec<String>) -> Self {
        self.thinking_chunks = chunks;
        self
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    async fn stream_chat(&self, _req: ChatRequest) -> Result<ChunkStream, LlmError> {
        let mut all_chunks: Vec<ChatChunk> = self
            .thinking_chunks
            .iter()
            .map(|t| ChatChunk::ThinkingDelta { text: t.clone() })
            .collect();
        {
            let mut guard = self.scripted.lock().expect("mock provider lock poisoned");
            all_chunks.extend(std::mem::take(&mut *guard));
        }
        let stream = stream::iter(all_chunks.into_iter().map(Ok)).boxed();
        Ok(stream)
    }

    fn name(&self) -> &'static str {
        "mock"
    }

    fn capabilities_for_model(&self, _model_id: &str) -> Capabilities {
        self.capabilities
    }

    fn active_model(&self) -> &str {
        &self.model
    }
}
