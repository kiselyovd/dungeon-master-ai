pub mod agent;
pub mod character_assist;
pub mod chat;
pub mod combat;
pub mod health;
pub mod image;
pub mod journal;
pub mod local_control;
pub mod messages;
pub mod npc;
pub mod providers;
pub mod saves;
pub mod settings;
pub mod srd;
pub mod video;

use serde::Deserialize;
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CampaignQuery {
    pub campaign_id: Uuid,
}
