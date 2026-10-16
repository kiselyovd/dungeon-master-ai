//! A native alias is meaningful only under the trusted owner's current model.
use crate::{
    Capabilities, ChatRequest, ChunkStream, LlmError, LlmProvider, MistralrsLocalProvider,
};
use app_application::{
    models::local_models::{RuntimeState, RuntimeStatus},
    ports::runtime::RuntimeControl,
};
use std::sync::Arc;

pub struct NativeRuntimeMistralProvider {
    control: Arc<dyn RuntimeControl>,
    runtime_model_id: String,
    model: String,
    identity: String,
}
impl NativeRuntimeMistralProvider {
    pub fn new(
        control: Arc<dyn RuntimeControl>,
        runtime_model_id: String,
        model: String,
        identity: String,
    ) -> Self {
        Self {
            control,
            runtime_model_id,
            model,
            identity,
        }
    }
    async fn checked_status(&self) -> Result<RuntimeStatus, LlmError> {
        let status = self
            .control
            .status()
            .await
            .map_err(|_| LlmError::RuntimeUnavailable)?;
        if !matches!(status.state, RuntimeState::Running | RuntimeState::Degraded)
            || status.llm_port.is_none_or(|port| port == 0)
        {
            return Err(LlmError::RuntimeUnavailable);
        }
        if status.model_id.as_deref() != Some(self.runtime_model_id.as_str()) {
            return Err(LlmError::RuntimeChanged);
        }
        Ok(status)
    }
    async fn current_provider(&self, requested: &str) -> Result<MistralrsLocalProvider, LlmError> {
        if requested != self.model {
            return Err(LlmError::RuntimeChanged);
        }
        let status = self.checked_status().await?;
        let port = status.llm_port.ok_or(LlmError::RuntimeUnavailable)?;
        // A restart of the same model may choose a new port. Never reuse a stale alias.
        Ok(MistralrsLocalProvider::new(port, self.model.clone()))
    }
}
#[async_trait::async_trait]
impl LlmProvider for NativeRuntimeMistralProvider {
    async fn verify_execution_environment(&self) -> Result<(), LlmError> {
        self.checked_status().await.map(|_| ())
    }
    async fn stream_chat(&self, request: ChatRequest) -> Result<ChunkStream, LlmError> {
        self.current_provider(&request.model)
            .await?
            .stream_chat(request)
            .await
    }
    async fn stream_chat_json(
        &self,
        request: ChatRequest,
        schema: serde_json::Value,
    ) -> Result<ChunkStream, LlmError> {
        self.current_provider(&request.model)
            .await?
            .stream_chat_json(request, schema)
            .await
    }
    fn name(&self) -> &'static str {
        "local-mistralrs"
    }
    fn active_model(&self) -> &str {
        &self.model
    }
    fn execution_origin(&self) -> Option<String> {
        Some(format!("local-mistralrs-native:{}", self.identity))
    }
    fn capabilities_for_model(&self, model: &str) -> Capabilities {
        MistralrsLocalProvider::new(0, self.model.clone()).capabilities_for_model(model)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use app_application::{
        models::local_models::RuntimeStartRequest, ports::runtime::RuntimeError,
    };
    struct Control(std::sync::Mutex<RuntimeStatus>);
    #[async_trait::async_trait]
    impl RuntimeControl for Control {
        async fn start(&self, _: RuntimeStartRequest) -> Result<RuntimeStatus, RuntimeError> {
            self.status().await
        }
        async fn stop(&self) -> Result<RuntimeStatus, RuntimeError> {
            self.status().await
        }
        async fn status(&self) -> Result<RuntimeStatus, RuntimeError> {
            Ok(self.0.lock().unwrap().clone())
        }
    }
    #[tokio::test]
    async fn same_model_restart_rebinds_port_but_a_different_model_or_stop_cannot_use_default_alias(
    ) {
        let control = Arc::new(Control(std::sync::Mutex::new(RuntimeStatus {
            state: RuntimeState::Running,
            model_id: Some("a".into()),
            llm_port: Some(1234),
            ..Default::default()
        })));
        let provider = NativeRuntimeMistralProvider::new(
            control.clone(),
            "a".into(),
            "model-a".into(),
            "physical-a".into(),
        );
        assert_eq!(
            provider
                .current_provider("model-a")
                .await
                .unwrap()
                .base_url(),
            "http://127.0.0.1:1234/v1/"
        );
        control.0.lock().unwrap().llm_port = Some(5678);
        assert_eq!(
            provider
                .current_provider("model-a")
                .await
                .unwrap()
                .base_url(),
            "http://127.0.0.1:5678/v1/"
        );
        control.0.lock().unwrap().model_id = Some("b".into());
        assert!(matches!(
            provider.verify_execution_environment().await,
            Err(LlmError::RuntimeChanged)
        ));
        assert!(matches!(
            provider.current_provider("model-b").await,
            Err(LlmError::RuntimeChanged)
        ));
        control.0.lock().unwrap().state = RuntimeState::Stopped;
        assert!(matches!(
            provider.verify_execution_environment().await,
            Err(LlmError::RuntimeUnavailable)
        ));
    }
    #[tokio::test]
    async fn uses_current_native_port_and_changed_or_stopped_models_make_no_http_request() {
        use futures::StreamExt;
        use wiremock::{
            matchers::{method, path},
            Mock, MockServer, ResponseTemplate,
        };
        let server = MockServer::start().await;
        let body = concat!(
            "data: {\"id\":\"fixture\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"default\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Ready\"},\"finish_reason\":null}]}\n\n",
            "data: {\"id\":\"fixture\",\"object\":\"chat.completion.chunk\",\"created\":1,\"model\":\"default\",\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"stop\"}]}\n\n",
            "data: [DONE]\n\n"
        );
        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_string(body),
            )
            .mount(&server)
            .await;
        let control = Arc::new(Control(std::sync::Mutex::new(RuntimeStatus {
            state: RuntimeState::Running,
            model_id: Some("a".into()),
            llm_port: Some(1),
            ..Default::default()
        })));
        let provider = NativeRuntimeMistralProvider::new(
            control.clone(),
            "a".into(),
            "model-a".into(),
            "physical-a".into(),
        );
        control.0.lock().unwrap().llm_port = Some(server.address().port());
        let request = ChatRequest {
            model: "model-a".into(),
            messages: vec![],
            system_prompt: None,
            max_tokens: None,
            temperature: None,
            tools: vec![],
            reasoning: None,
        };
        let mut stream = provider.stream_chat(request.clone()).await.unwrap();
        let mut text = String::new();
        while let Some(chunk) = stream.next().await {
            if let crate::ChatChunk::TextDelta { text: delta } = chunk.unwrap() {
                text.push_str(&delta);
            }
        }
        assert_eq!(text, "Ready");
        control.0.lock().unwrap().model_id = Some("b".into());
        assert!(matches!(
            provider.stream_chat(request.clone()).await,
            Err(LlmError::RuntimeChanged)
        ));
        control.0.lock().unwrap().state = RuntimeState::Stopped;
        assert!(matches!(
            provider.stream_chat(request).await,
            Err(LlmError::RuntimeUnavailable)
        ));
        assert_eq!(server.received_requests().await.unwrap().len(), 1);
    }
}
