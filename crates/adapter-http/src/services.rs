use std::sync::Arc;

use app_application::models::agent::{AgentEvent, AgentTurnRequest};
use app_application::models::chat::{ChatMessage, ChatRequest};
use app_application::models::combat::CombatProjection;
use app_application::models::settings::SettingsConfigV2;
use app_application::ports::llm::ChunkStream;
use app_application::ports::media::{ImageBytes, ImagePrompt, VideoPrompt, VideoStream};
use async_trait::async_trait;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::Value;
use tokio::sync::mpsc;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpServiceError {
    NotFound,
    BadRequest { code: &'static str },
    PayloadTooLarge { code: &'static str },
    Unauthorized { code: &'static str },
    RateLimit { code: &'static str },
    BadGateway { code: &'static str },
    Internal { code: &'static str },
}

impl IntoResponse for HttpServiceError {
    fn into_response(self) -> Response {
        let (status, code) = match self {
            Self::NotFound => (StatusCode::NOT_FOUND, "not_found"),
            Self::BadRequest { code } => (StatusCode::BAD_REQUEST, code),
            Self::PayloadTooLarge { code } => (StatusCode::PAYLOAD_TOO_LARGE, code),
            Self::Unauthorized { code } => (StatusCode::UNAUTHORIZED, code),
            Self::RateLimit { code } => (StatusCode::TOO_MANY_REQUESTS, code),
            Self::BadGateway { code } => (StatusCode::BAD_GATEWAY, code),
            Self::Internal { code } => (StatusCode::INTERNAL_SERVER_ERROR, code),
        };
        (
            status,
            Json(serde_json::json!({
                "error": { "code": code, "message": code }
            })),
        )
            .into_response()
    }
}

#[async_trait]
pub trait MediaHttpService: Send + Sync {
    async fn generate_image(&self, prompt: ImagePrompt) -> Result<ImageBytes, HttpServiceError>;
    async fn generate_video(&self, prompt: VideoPrompt) -> Result<VideoStream, HttpServiceError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SrdSection {
    Races,
    Classes,
    Backgrounds,
    Spells,
    Equipment,
    Feats,
    WeaponProperties,
}

#[async_trait]
pub trait CampaignHttpService: Send + Sync {
    async fn journal(&self, campaign_id: Uuid) -> Result<Value, HttpServiceError>;
    async fn npcs(&self, campaign_id: Uuid) -> Result<Value, HttpServiceError>;
    async fn messages(&self, session_id: String) -> Result<Value, HttpServiceError>;
    async fn srd(&self, section: SrdSection) -> Result<Value, HttpServiceError>;
}

#[derive(Debug, Clone)]
pub struct CombatStartCommand {
    pub campaign_id: Uuid,
    pub session_id: Uuid,
    pub initiative_entries: Vec<crate::routes::combat::InitiativeEntryDto>,
}

#[derive(Debug, Clone)]
pub struct CombatActionCommand {
    pub encounter_id: Uuid,
    pub action_type: String,
    pub args: Value,
    pub request_id: Option<String>,
    pub expected_revision: Option<u64>,
    pub rng_seed: Option<u64>,
}

#[async_trait]
pub trait CombatHttpService: Send + Sync {
    async fn start(&self, command: CombatStartCommand) -> Result<Uuid, HttpServiceError>;
    async fn action(
        &self,
        command: CombatActionCommand,
    ) -> Result<Option<CombatProjection>, HttpServiceError>;
    async fn end(&self, encounter_id: Uuid) -> Result<(), HttpServiceError>;
}

#[derive(Debug, Clone)]
pub struct SaveMetadataCommand {
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub tag: String,
}

#[async_trait]
pub trait SavesHttpService: Send + Sync {
    async fn list(&self, session_id: Uuid) -> Result<Value, HttpServiceError>;
    async fn create(
        &self,
        session_id: Uuid,
        metadata: SaveMetadataCommand,
    ) -> Result<Uuid, HttpServiceError>;
    async fn quick(&self, session_id: Uuid) -> Result<Uuid, HttpServiceError>;
    async fn get(&self, save_id: Uuid) -> Result<Value, HttpServiceError>;
    async fn delete(&self, save_id: Uuid) -> Result<bool, HttpServiceError>;
    async fn restore(&self, session_id: Uuid, save_id: Uuid) -> Result<Value, HttpServiceError>;
    async fn update(
        &self,
        save_id: Uuid,
        metadata: SaveMetadataCommand,
    ) -> Result<bool, HttpServiceError>;
}

#[derive(Debug, Clone)]
pub struct ProviderDiscoveryCommand {
    pub provider_id: String,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
    pub search_query: Option<String>,
    pub cursor: Option<String>,
}

#[async_trait]
pub trait SettingsHttpService: Send + Sync {
    async fn provider_info(&self) -> Result<Value, HttpServiceError>;
    async fn update(&self, config: SettingsConfigV2) -> Result<Value, HttpServiceError>;
    async fn catalog(&self) -> Result<Value, HttpServiceError>;
    async fn capabilities(
        &self,
        provider_id: String,
        model_id: String,
    ) -> Result<Option<Value>, HttpServiceError>;
    async fn discover(&self, command: ProviderDiscoveryCommand) -> Result<Value, HttpServiceError>;
}

#[derive(Debug, Clone)]
pub struct AgentTurnHttpCommand {
    pub request: AgentTurnRequest,
    pub model: Option<String>,
}

#[async_trait]
pub trait AgentHttpService: Send + Sync {
    async fn turn(
        &self,
        command: AgentTurnHttpCommand,
    ) -> Result<mpsc::Receiver<AgentEvent>, HttpServiceError>;
}

#[derive(Debug, Clone)]
pub struct ChatHttpCommand {
    pub messages: Vec<ChatMessage>,
    pub session_id: Option<String>,
    pub model: Option<String>,
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}

#[async_trait]
pub trait ChatHttpService: Send + Sync {
    async fn stream(&self, command: ChatHttpCommand) -> Result<ChunkStream, HttpServiceError>;
}

#[async_trait]
pub trait CharacterAssistHttpService: Send + Sync {
    async fn stream(&self, request: ChatRequest) -> Result<ChunkStream, HttpServiceError>;
}

#[derive(Debug, Clone)]
pub enum LocalControlOperation {
    HfSetToken { token: String },
    HfDeleteToken,
    HfTokenStatus,
    HfSearch { query: Value },
    HfLicense { repo_id: String },
    HfManifestAdd { body: Value },
    HfManifestDelete { id: String },
    LocalLlmManifest,
    LocalLlmSetActive { id: String },
    LocalLlmStartDownload { id: String },
    LocalLlmCancelOrDelete { id: String },
    LocalModeGetConfig,
    LocalModeSetConfig { config: Value },
    LocalModeStartDownload { id: String },
    LocalModeDeleteDownload { id: String },
    LocalRuntimeStart,
    LocalRuntimeStop,
    LocalRuntimeStatus,
}

#[derive(Debug, Clone)]
pub enum LocalEventStream {
    LocalLlmDownloads,
    LocalModeDownload { id: String },
}

#[derive(Debug, Clone)]
pub struct HttpOutcome {
    pub status: u16,
    pub body: Option<Value>,
}

#[async_trait]
pub trait LocalControlHttpService: Send + Sync {
    async fn execute(
        &self,
        operation: LocalControlOperation,
    ) -> Result<HttpOutcome, HttpServiceError>;
    async fn events(
        &self,
        stream: LocalEventStream,
    ) -> Result<mpsc::Receiver<Value>, HttpServiceError>;
}

#[derive(Clone)]
pub struct HttpServices {
    pub media: Arc<dyn MediaHttpService>,
    pub campaign: Arc<dyn CampaignHttpService>,
    pub combat: Arc<dyn CombatHttpService>,
    pub saves: Arc<dyn SavesHttpService>,
    pub settings: Arc<dyn SettingsHttpService>,
    pub agent: Arc<dyn AgentHttpService>,
    pub chat: Arc<dyn ChatHttpService>,
    pub character_assist: Arc<dyn CharacterAssistHttpService>,
    pub local_control: Arc<dyn LocalControlHttpService>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typed_errors_map_to_stable_statuses_without_dynamic_content() {
        let cases = [
            (HttpServiceError::NotFound, StatusCode::NOT_FOUND),
            (
                HttpServiceError::BadRequest { code: "invalid" },
                StatusCode::BAD_REQUEST,
            ),
            (
                HttpServiceError::PayloadTooLarge { code: "too_large" },
                StatusCode::PAYLOAD_TOO_LARGE,
            ),
            (
                HttpServiceError::Unauthorized {
                    code: "unauthorized",
                },
                StatusCode::UNAUTHORIZED,
            ),
            (
                HttpServiceError::RateLimit { code: "rate_limit" },
                StatusCode::TOO_MANY_REQUESTS,
            ),
            (
                HttpServiceError::BadGateway { code: "upstream" },
                StatusCode::BAD_GATEWAY,
            ),
            (
                HttpServiceError::Internal { code: "internal" },
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
        ];
        for (error, expected) in cases {
            assert_eq!(error.into_response().status(), expected);
        }
    }
}
