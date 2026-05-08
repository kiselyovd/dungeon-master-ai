pub mod agent;
pub mod chat;
pub mod combat;
pub mod health;
pub mod journal;
pub mod local_mode;
pub mod npc;
pub mod settings;

use serde::Deserialize;
use uuid::Uuid;

/// Query string used by all campaign-scoped GET endpoints (`/journal`, `/npcs`, etc.).
#[derive(Deserialize)]
pub struct CampaignQuery {
    pub campaign_id: Uuid,
}
