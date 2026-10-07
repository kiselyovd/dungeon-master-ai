use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::chat::ChatMessage;
use super::combat::CombatProjection;
use super::identity::{CampaignId, Revision, SessionId};

pub const SNAPSHOT_SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotProjection {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shared_message_indices: Vec<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub party: Option<super::party::PartySnapshot>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub adventuring: Option<super::rest::AdventuringState>,
    #[serde(default)]
    pub adventure: Option<super::adventure::AdventureProjection>,
    pub messages: Vec<ChatMessage>,
    pub character: Value,
    pub combat: Option<Value>,
    pub scene: Option<Value>,
    pub journal: Vec<Value>,
    pub npcs: Vec<Value>,
}

impl SnapshotProjection {
    pub fn is_complete(&self) -> bool {
        self.shared_message_indices
            .iter()
            .copied()
            .collect::<std::collections::BTreeSet<_>>()
            .len()
            == self.shared_message_indices.len()
            && self.shared_message_indices.iter().all(|index| {
                self.messages.get(*index).is_some_and(|message| {
                    matches!(
                        message,
                        ChatMessage::User { .. } | ChatMessage::Assistant { .. }
                    )
                })
            })
            && self
                .party
                .as_ref()
                .is_none_or(|party| party.validate(&self.character).is_ok())
            && self
                .adventuring
                .as_ref()
                .is_none_or(|state| state.elapsed_seconds <= i64::MAX as u64)
            && self.character.is_object()
            && self.combat.as_ref().is_none_or(|value| {
                serde_json::from_value::<CombatProjection>(value.clone()).is_ok()
            })
            && self.scene.as_ref().is_none_or(|value| {
                value.get("id").is_some_and(Value::is_string)
                    && value.get("title").is_some_and(Value::is_string)
                    && value.get("mode").is_some_and(Value::is_string)
            })
            && self.journal.iter().all(|value| {
                value.get("id").is_some_and(Value::is_string)
                    && value.get("entry_html").is_some_and(Value::is_string)
                    && value.get("created_at").is_some_and(Value::is_string)
            })
            && self.npcs.iter().all(|value| {
                value.get("id").is_some_and(Value::is_string)
                    && value.get("name").is_some_and(Value::is_string)
                    && value.get("facts").is_some_and(Value::is_array)
                    && value.get("updated_at").is_some_and(Value::is_string)
            })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CampaignSnapshot {
    pub id: Uuid,
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub revision: Revision,
    pub schema_version: u16,
    pub ruleset_id: String,
    pub projection: SnapshotProjection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotCreated {
    pub save_id: Uuid,
    pub snapshot: CampaignSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SnapshotSaveSummary {
    pub id: Uuid,
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub snapshot_id: Uuid,
    pub revision: Revision,
    pub kind: String,
    pub title: String,
    pub summary: String,
    pub tag: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SnapshotSaveDetails {
    #[serde(flatten)]
    pub summary: SnapshotSaveSummary,
    pub snapshot: CampaignSnapshot,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateSnapshotRecord {
    pub id: Uuid,
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub expected_revision: Revision,
    pub schema_version: u16,
    pub ruleset_id: String,
    pub title: String,
    pub kind: String,
    pub summary: String,
    pub tag: String,
    pub causation_id: String,
    pub character: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RestoreSnapshotRecord {
    pub save_id: Uuid,
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub expected_revision: Revision,
    pub causation_id: String,
}
