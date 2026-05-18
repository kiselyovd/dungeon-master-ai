pub mod agent;
pub mod character_assist;
pub mod chat;
pub mod combat;
pub mod health;
pub mod hf;
pub mod journal;
pub mod local_llm;
pub mod local_mode;
pub mod messages;
pub mod npc;
pub mod providers;
pub mod saves;
pub mod settings;
pub mod srd;
pub mod video;

use serde::Deserialize;
use uuid::Uuid;

/// Query string used by all campaign-scoped GET endpoints (`/journal`, `/npcs`, etc.).
#[derive(Deserialize)]
pub struct CampaignQuery {
    pub campaign_id: Uuid,
}
