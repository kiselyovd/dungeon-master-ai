use async_trait::async_trait;
use futures::stream::StreamExt;
use genai::Client;
use genai::ModelIden;
use genai::adapter::AdapterKind;
use genai::chat::ChatOptions;
use genai::resolver::{AuthData, AuthResolver};
use std::sync::Arc;
use tokio_stream::wrappers::ReceiverStream;

use crate::genai_common::{classify_genai_error, convert_messages, pump_genai_stream};
use crate::provider::{ChatChunk, ChatRequest, ChunkStream, LlmError, LlmProvider};

pub struct AnthropicProvider {
    client: Client,
}

impl AnthropicProvider {
    pub fn new(api_key: String) -> Self {
        let key = Arc::new(api_key);
        let resolver =
            AuthResolver::from_resolver_fn(move |_iden: ModelIden| -> Result<
                Option<AuthData>,
                genai::resolver::Error,
            > {
                Ok(Some(AuthData::from_single(key.as_str().to_string())))
            });
        let client = Client::builder().with_auth_resolver(resolver).build();
        Self { client }
    }

    fn build_options(req: &ChatRequest) -> ChatOptions {
        let mut options = ChatOptions::default();
        if let Some(max) = req.max_tokens {
            options = options.with_max_tokens(max);
        }
        if let Some(temp) = req.temperature {
            options = options.with_temperature(temp as f64);
        }
        options
    }
}

#[async_trait]
impl LlmProvider for AnthropicProvider {
    async fn stream_chat(&self, req: ChatRequest) -> Result<ChunkStream, LlmError> {
        let model = req.model.clone();
        let options = Self::build_options(&req);
        let g_req = convert_messages(req.messages, req.tools);

        let stream_response = self
            .client
            .exec_chat_stream(&model, g_req, Some(&options))
            .await
            .map_err(|e| classify_genai_error(e.to_string()))?;

        let g_stream = stream_response.stream;
        let (tx, rx) = tokio::sync::mpsc::channel::<Result<ChatChunk, LlmError>>(64);

        tokio::spawn(pump_genai_stream(g_stream, tx));

        Ok(ReceiverStream::new(rx).boxed())
    }

    fn name(&self) -> &'static str {
        "anthropic"
    }

    fn supports_tools(&self) -> bool {
        true
    }

    fn supports_vision(&self) -> bool {
        true
    }
}

#[allow(dead_code)]
fn anthropic_kind() -> AdapterKind {
    AdapterKind::Anthropic
}
