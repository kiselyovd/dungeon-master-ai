use std::sync::Arc;

use adapter_http::services::{
    AgentHttpService, AgentTurnHttpCommand, CampaignHttpService, CharacterAssistHttpService,
    ChatHttpCommand, ChatHttpService, CombatActionCommand, CombatHttpService, CombatStartCommand,
    HttpOutcome, HttpServiceError, HttpServices, LocalControlHttpService, LocalControlOperation,
    LocalEventStream, MediaHttpService, ProviderDiscoveryCommand, RestoreSaveCommand, SaveCreated,
    SaveMetadataCommand, SavesHttpService, SettingsHttpService, SrdSection,
};
use app_application::models::agent::AgentEvent;
use app_application::models::chat::ChatRequest;
use app_application::models::combat::CombatProjection;
use app_application::models::settings::SettingsConfigV2;
use app_application::ports::llm::ChunkStream;
use app_application::ports::media::{ImageBytes, ImagePrompt, VideoPrompt, VideoStream};
use async_trait::async_trait;
use axum::body::{to_bytes, Body};
use axum::http::{Request, StatusCode};
use serde_json::{json, Value};
use tokio::sync::{mpsc, Mutex};
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Default)]
struct SavesStub {
    create: Mutex<Option<(Uuid, SaveMetadataCommand)>>,
    restore: Mutex<Option<RestoreSaveCommand>>,
}

#[async_trait]
impl SavesHttpService for SavesStub {
    async fn list(&self, _: Uuid) -> Result<Value, HttpServiceError> {
        Ok(json!([]))
    }
    async fn create(
        &self,
        session: Uuid,
        metadata: SaveMetadataCommand,
    ) -> Result<SaveCreated, HttpServiceError> {
        *self.create.lock().await = Some((session, metadata));
        Ok(SaveCreated {
            id: Uuid::nil(),
            revision: 5,
        })
    }
    async fn quick(&self, _: Uuid) -> Result<SaveCreated, HttpServiceError> {
        Ok(SaveCreated {
            id: Uuid::nil(),
            revision: 5,
        })
    }
    async fn get(&self, _: Uuid) -> Result<Value, HttpServiceError> {
        Err(HttpServiceError::NotFound)
    }
    async fn delete(&self, _: Uuid) -> Result<bool, HttpServiceError> {
        Ok(false)
    }
    async fn restore(&self, command: RestoreSaveCommand) -> Result<Value, HttpServiceError> {
        *self.restore.lock().await = Some(command);
        Ok(json!({
            "schema_version": 1,
            "ruleset_id": "srd-5.2.1",
            "revision": 4,
            "projection": {
                "messages": [], "character": {}, "combat": null, "scene": null,
                "journal": [], "npcs": []
            }
        }))
    }
    async fn update(&self, _: Uuid, _: SaveMetadataCommand) -> Result<bool, HttpServiceError> {
        Ok(false)
    }
}

#[tokio::test]
async fn restore_accepts_one_typed_body_and_returns_the_complete_projection() {
    let saves = Arc::new(SavesStub::default());
    let save_id = Uuid::new_v4();
    let campaign_id = Uuid::new_v4();
    let session_id = Uuid::new_v4();
    let app = adapter_http::router(services(saves.clone()));
    let response = app
        .oneshot(
            Request::post(format!("/saves/{save_id}/restore"))
                .header("content-type", "application/json")
                .body(Body::from(
                    json!({
                        "campaign_id": campaign_id,
                        "session_id": session_id,
                        "expected_revision": 4,
                        "causation_id": "restore-http-1"
                    })
                    .to_string(),
                ))
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: Value =
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap();
    assert_eq!(body["ruleset_id"], "srd-5.2.1");
    assert!(body["projection"]["messages"].is_array());
    let command = saves.restore.lock().await.clone().unwrap();
    assert_eq!(command.campaign_id, campaign_id);
    assert_eq!(command.session_id, session_id);
    assert_eq!(command.expected_revision, 4);
}

macro_rules! unimplemented_service {
    ($name:ident, $trait_name:ident) => {
        struct $name;
    };
}
unimplemented_service!(MediaStub, MediaHttpService);
unimplemented_service!(CampaignStub, CampaignHttpService);
unimplemented_service!(CombatStub, CombatHttpService);
unimplemented_service!(SettingsStub, SettingsHttpService);
unimplemented_service!(AgentStub, AgentHttpService);
unimplemented_service!(ChatStub, ChatHttpService);
unimplemented_service!(CharacterStub, CharacterAssistHttpService);
unimplemented_service!(LocalStub, LocalControlHttpService);

#[async_trait]
impl MediaHttpService for MediaStub {
    async fn generate_image(&self, _: ImagePrompt) -> Result<ImageBytes, HttpServiceError> {
        unimplemented!()
    }
    async fn generate_video(&self, _: VideoPrompt) -> Result<VideoStream, HttpServiceError> {
        unimplemented!()
    }
}
#[async_trait]
impl CampaignHttpService for CampaignStub {
    async fn create_campaign(
        &self,
        _: app_application::campaigns::CreateCampaignCommand,
    ) -> Result<adapter_http::routes::campaigns::CampaignResponse, HttpServiceError> {
        unimplemented!()
    }
    async fn list_campaigns(
        &self,
    ) -> Result<Vec<adapter_http::routes::campaigns::CampaignSummaryResponse>, HttpServiceError>
    {
        unimplemented!()
    }
    async fn get_campaign(
        &self,
        _: Uuid,
    ) -> Result<adapter_http::routes::campaigns::CampaignResponse, HttpServiceError> {
        unimplemented!()
    }
    async fn journal(&self, _: Uuid) -> Result<Value, HttpServiceError> {
        unimplemented!()
    }
    async fn npcs(&self, _: Uuid) -> Result<Value, HttpServiceError> {
        unimplemented!()
    }
    async fn messages(&self, _: String) -> Result<Value, HttpServiceError> {
        unimplemented!()
    }
    async fn srd(&self, _: SrdSection) -> Result<Value, HttpServiceError> {
        unimplemented!()
    }
}
#[async_trait]
impl CombatHttpService for CombatStub {
    async fn start(&self, _: CombatStartCommand) -> Result<CombatProjection, HttpServiceError> {
        unimplemented!()
    }
    async fn action(
        &self,
        _: CombatActionCommand,
    ) -> Result<Option<CombatProjection>, HttpServiceError> {
        unimplemented!()
    }
    async fn end(&self, _: Uuid) -> Result<Option<CombatProjection>, HttpServiceError> {
        unimplemented!()
    }
}
#[async_trait]
impl SettingsHttpService for SettingsStub {
    async fn provider_info(&self) -> Result<Value, HttpServiceError> {
        unimplemented!()
    }
    async fn update(&self, _: SettingsConfigV2) -> Result<Value, HttpServiceError> {
        unimplemented!()
    }
    async fn catalog(&self) -> Result<Value, HttpServiceError> {
        unimplemented!()
    }
    async fn capabilities(&self, _: String, _: String) -> Result<Option<Value>, HttpServiceError> {
        unimplemented!()
    }
    async fn discover(&self, _: ProviderDiscoveryCommand) -> Result<Value, HttpServiceError> {
        unimplemented!()
    }
}
#[async_trait]
impl AgentHttpService for AgentStub {
    async fn turn(
        &self,
        _: AgentTurnHttpCommand,
    ) -> Result<mpsc::Receiver<AgentEvent>, HttpServiceError> {
        unimplemented!()
    }
}
#[async_trait]
impl ChatHttpService for ChatStub {
    async fn stream(&self, _: ChatHttpCommand) -> Result<ChunkStream, HttpServiceError> {
        unimplemented!()
    }
}
#[async_trait]
impl CharacterAssistHttpService for CharacterStub {
    async fn stream(&self, _: ChatRequest) -> Result<ChunkStream, HttpServiceError> {
        unimplemented!()
    }
}
#[async_trait]
impl LocalControlHttpService for LocalStub {
    async fn execute(&self, _: LocalControlOperation) -> Result<HttpOutcome, HttpServiceError> {
        unimplemented!()
    }
    async fn events(&self, _: LocalEventStream) -> Result<mpsc::Receiver<Value>, HttpServiceError> {
        unimplemented!()
    }
}

fn services(saves: Arc<SavesStub>) -> HttpServices {
    HttpServices {
        media: Arc::new(MediaStub),
        campaign: Arc::new(CampaignStub),
        combat: Arc::new(CombatStub),
        saves,
        settings: Arc::new(SettingsStub),
        agent: Arc::new(AgentStub),
        chat: Arc::new(ChatStub),
        character_assist: Arc::new(CharacterStub),
        local_control: Arc::new(LocalStub),
    }
}
