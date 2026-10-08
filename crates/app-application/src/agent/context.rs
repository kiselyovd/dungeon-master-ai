use async_trait::async_trait;
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct AgentContextRequest {
    pub campaign_id: Uuid,
    pub player_message: String,
    pub base_system_prompt: String,
    pub embedding_model: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentContext {
    /// Trusted DM knowledge must not be published through provider reasoning.
    pub reasoning_is_private: bool,
    pub adventure_active: bool,
    pub system_prompt: String,
    pub combat_active: bool,
    /// Only visible scene/target labels; never DM-only canon or other players' sheets.
    pub public_intention_context: serde_json::Value,
}

#[derive(Debug, Error)]
#[error("agent context port failed with code {code}")]
pub struct AgentContextError {
    pub code: &'static str,
}

#[async_trait]
pub trait AgentContextPort: Send + Sync {
    async fn build(&self, request: AgentContextRequest) -> Result<AgentContext, AgentContextError>;
}

pub fn public_intention_context(
    adventure: Option<&crate::models::adventure::AdventureProjection>,
    combat: Option<&serde_json::Value>,
    hero_name: Option<&str>,
) -> serde_json::Value {
    let targets=combat.and_then(|value|value["snapshot"]["combatants"].as_array()).map(|actors|actors.iter().map(|actor|serde_json::json!({"id":actor["id"],"name":actor["name"],"display_name_key":actor["display_name_key"],"character_id":actor["character_id"],"position":actor["position"],"current_hp":actor["current_hp"],"is_dead":actor["is_dead"]})).collect::<Vec<_>>()).unwrap_or_default();
    serde_json::json!({"hero_name":hero_name,"adventure":adventure,"targets":targets,"authored_map":combat.map(|value|value["authored"]["map"].clone()),"location_names":{"town":"Veresk / Вереск","quay":"old quay / старый причал","archive":"drowned archive / затопленный архив","belfry":"lower belfry / нижняя звонница","warehouse":"warehouse / склад","shrine":"shrine / святилище"}})
}
