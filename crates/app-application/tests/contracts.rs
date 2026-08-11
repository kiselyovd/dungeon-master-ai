use app_application::models::agent::{AgentEvent, AgentTurnRequest};
use app_application::models::chat::{
    Capabilities, ChatChunk, ChatMessage, ChatRequest, FinishReason, MessagePart, ReasoningSpec,
    Tool, ToolCall, ToolResult,
};
use app_application::models::combat::{CombatProjection, COMBAT_PROJECTION_VERSION};
use app_application::models::local_models::{SystemEntry, UserEntry};
use app_application::ports::llm::{ChunkStream, LlmError, LlmProvider};
use app_application::ports::runtime::{SidecarError, SidecarHandle, SidecarLauncher, SpawnSpec};
use async_trait::async_trait;
use futures::stream;
use serde_json::json;
use uuid::Uuid;

struct ContractProvider;

#[async_trait]
impl LlmProvider for ContractProvider {
    async fn stream_chat(&self, _request: ChatRequest) -> Result<ChunkStream, LlmError> {
        Ok(Box::pin(stream::iter([Ok(ChatChunk::Done {
            reason: FinishReason::Stop,
        })])))
    }

    fn name(&self) -> &'static str {
        "contract"
    }

    fn capabilities_for_model(&self, _model_id: &str) -> Capabilities {
        Capabilities::default()
    }

    fn active_model(&self) -> &str {
        "contract-model"
    }
}

#[test]
fn chat_contract_keeps_the_stable_serde_shape() {
    let message = ChatMessage::User {
        parts: vec![
            MessagePart::Text {
                text: "look".into(),
            },
            MessagePart::Image {
                mime: "image/png".into(),
                data_b64: "AA==".into(),
                name: Some("map.png".into()),
            },
        ],
    };

    assert_eq!(
        serde_json::to_value(message).unwrap(),
        json!({
            "role": "user",
            "parts": [
                {"type": "text", "text": "look"},
                {"type": "image", "mime": "image/png", "data_b64": "AA==", "name": "map.png"}
            ]
        })
    );
}

#[test]
fn public_contract_types_are_owned_by_the_application_crate() {
    fn assert_provider<T: LlmProvider>() {}
    assert_provider::<ContractProvider>();

    let _request_type: Option<AgentTurnRequest> = None;
    let _event_type: Option<AgentEvent> = None;
    let _tool_types: Option<(Tool, ToolCall, ToolResult)> = None;
    let _reasoning = ReasoningSpec::Medium;
    let _runtime_types: Option<(
        SpawnSpec,
        SidecarHandle,
        SidecarError,
        Box<dyn SidecarLauncher>,
    )> = None;
}

#[test]
fn combat_projection_has_an_explicit_version_and_monotonic_revision() {
    let projection = CombatProjection::empty(Uuid::nil(), 7);
    assert_eq!(projection.schema_version, COMBAT_PROJECTION_VERSION);
    assert_eq!(projection.revision, 7);
    assert!(projection.snapshot.combatants.is_empty());
    assert!(projection.events.is_empty());
}

#[test]
fn local_model_manifest_keeps_flattened_wire_compatibility() {
    let entry = UserEntry {
        system: SystemEntry {
            id: "custom/foo".into(),
            hf_repo: "custom/foo".into(),
            hf_filename: "foo.gguf".into(),
            arch: "llama".into(),
            quant: "gguf-q4_k_m".into(),
            size_gb: 1.0,
            license: "mit".into(),
            display_name: "Foo".into(),
        },
        added_at: "2026-05-19T00:00:00Z".into(),
        source: "hf-search".into(),
    };
    let value = serde_json::to_value(&entry).unwrap();
    assert_eq!(value["hf_repo"], "custom/foo");
    assert!(value.get("system").is_none());
    assert_eq!(serde_json::from_value::<UserEntry>(value).unwrap(), entry);
}
