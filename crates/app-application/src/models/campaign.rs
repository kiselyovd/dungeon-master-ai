use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::chat::ChatMessage;
use super::combat::CombatProjection;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StoredMessage {
    pub id: Uuid,
    pub session_id: Uuid,
    pub sequence: i64,
    pub message: ChatMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JournalEntry {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub title: String,
    pub body: String,
    pub updated_at_epoch_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NpcRecord {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub name: String,
    pub description: String,
    pub updated_at_epoch_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SceneRecord {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub name: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignSave {
    pub id: Uuid,
    pub campaign_id: Uuid,
    pub session_id: Uuid,
    pub name: String,
    pub messages: Vec<StoredMessage>,
    pub combat: Option<CombatProjection>,
    pub scene: Option<SceneRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SrdReference {
    pub source_id: String,
    pub title: String,
    pub body: String,
    pub score: f32,
}
