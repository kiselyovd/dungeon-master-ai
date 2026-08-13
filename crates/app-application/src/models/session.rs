use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use uuid::Uuid;

use super::identity::{CampaignId, Revision, SessionId};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CampaignSeed {
    Collaborative {
        answers: BTreeMap<String, Vec<String>>,
    },
    Document {
        document_id: Uuid,
    },
    Bundled {
        adventure_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignSummary {
    pub id: CampaignId,
    pub schema_version: u16,
    pub ruleset_id: String,
    pub title: String,
    pub premise: String,
    pub last_session_id: SessionId,
    pub revision: Revision,
    pub updated_at_epoch_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignDetail {
    pub id: CampaignId,
    pub schema_version: u16,
    pub ruleset_id: String,
    pub title: String,
    pub premise: String,
    pub seed: CampaignSeed,
    pub last_session_id: SessionId,
    pub revision: Revision,
    pub created_at_epoch_ms: i64,
    pub updated_at_epoch_ms: i64,
}

impl From<&CampaignDetail> for CampaignSummary {
    fn from(detail: &CampaignDetail) -> Self {
        Self {
            id: detail.id,
            schema_version: detail.schema_version,
            ruleset_id: detail.ruleset_id.clone(),
            title: detail.title.clone(),
            premise: detail.premise.clone(),
            last_session_id: detail.last_session_id,
            revision: detail.revision,
            updated_at_epoch_ms: detail.updated_at_epoch_ms,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CampaignSession {
    pub id: SessionId,
    pub campaign_id: CampaignId,
    pub revision: Revision,
    pub created_at_epoch_ms: i64,
    pub updated_at_epoch_ms: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignEventKind {
    CampaignCreated,
}

impl CampaignEventKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CampaignCreated => "campaign_created",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CampaignEvent {
    pub event_id: Uuid,
    pub campaign_id: CampaignId,
    pub revision: Revision,
    pub kind: CampaignEventKind,
    pub payload: Value,
    pub causation_id: Option<String>,
    pub created_at_epoch_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InitialCampaign {
    pub detail: CampaignDetail,
    pub session: CampaignSession,
    pub event: CampaignEvent,
}
