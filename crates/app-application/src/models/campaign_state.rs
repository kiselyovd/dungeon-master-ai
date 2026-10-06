use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::identity::{CampaignId, Revision, SessionId};
use super::session::CampaignDetail;
use super::snapshot::SnapshotProjection;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CampaignSessionState {
    pub campaign: CampaignDetail,
    pub session_id: SessionId,
    pub character_id: Option<Uuid>,
    pub projection: SnapshotProjection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommitCampaignCharacter {
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub character_id: Uuid,
    pub expected_revision: Revision,
    pub causation_id: String,
    pub character: Value,
}

pub const MAX_CHARACTER_METADATA_BYTES: usize = 256 * 1024;
pub const MAX_PORTRAIT_URL_BYTES: usize = 8 * 1024 * 1024;

pub fn within_character_budget(character: &Value) -> bool {
    let Some(document) = character.as_object() else {
        return false;
    };
    let portrait_ok = document.get("portraitUrl").is_none_or(|portrait| {
        portrait.is_null()
            || portrait
                .as_str()
                .is_some_and(|url| url.len() <= MAX_PORTRAIT_URL_BYTES)
    });
    let metadata: serde_json::Map<String, Value> = document
        .iter()
        .filter(|(key, _)| key.as_str() != "portraitUrl")
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    portrait_ok
        && serde_json::to_vec(&metadata)
            .is_ok_and(|bytes| bytes.len() <= MAX_CHARACTER_METADATA_BYTES)
}

/// Structural validation of a durable sheet. Combat still derives/validates actions
/// in the domain; accepting a document never authorizes client-supplied damage.
pub fn valid_character_document(value: &Value) -> bool {
    let text = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_str)
            .is_some_and(|s| !s.trim().is_empty())
    };
    let integer = |key: &str, min: i64, max: i64| {
        value
            .get(key)
            .and_then(Value::as_i64)
            .is_some_and(|n| (min..=max).contains(&n))
    };
    value.is_object()
        && text("name")
        && text("heroClass")
        && integer("level", 1, 20)
        && integer("hpMax", 1, 10000)
        && integer(
            "hp",
            0,
            value.get("hpMax").and_then(Value::as_i64).unwrap_or(-1),
        )
        && integer("ac", 1, 40)
        && ["str", "dex", "con", "int", "wis", "cha"]
            .iter()
            .all(|key| {
                value
                    .get("abilities")
                    .and_then(|a| a.get(key))
                    .and_then(Value::as_i64)
                    .is_some_and(|n| (1..=30).contains(&n))
            })
        && value
            .get("inventory")
            .and_then(Value::as_array)
            .is_some_and(|items| {
                items.iter().all(|item| {
                    item.get("id")
                        .and_then(Value::as_str)
                        .is_some_and(|id| !id.is_empty())
                        && item
                            .get("name")
                            .and_then(Value::as_str)
                            .is_some_and(|name| !name.is_empty())
                        && item.get("count").and_then(Value::as_u64).is_some()
                })
            })
}
