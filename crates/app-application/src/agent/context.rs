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
    /// Public scene/target labels, discovered facts and canonical references only.
    /// Never DM-only canon or other players' sheets.
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
    let public_facts = adventure
        .map(|projection| {
            crate::campaigns::adventure_context::bell_public_narration_facts(
                &projection.state,
                false,
            )
            .into_iter()
            .map(|fact| serde_json::json!({"id":fact.id,"text":fact.text}))
            .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let public_routes = adventure
        .map(|projection| {
            crate::campaigns::adventure_context::bell_public_route_references(&projection.state)
        })
        .unwrap_or_default();
    let public_actions = adventure
        .map(|projection| {
            crate::campaigns::adventure_context::bell_public_action_references(&projection.state)
        })
        .unwrap_or_default();
    let presentation_glossary = adventure
        .filter(|projection| {
            projection.state.package_id == crate::models::adventure::BELL_ADVENTURE_ID
        })
        .map(|_| {
            serde_json::json!({
                "ward":{"en":"ward","ru":"оберег"},
                "bell_clapper":{"en":"bell clapper","ru":"язык колокола"},
                "memorial_roll":{"en":"memorial roll","ru":"памятный список спасённых"},
                "land_ledger":{"en":"land ledger","ru":"земельная книга"}
            })
        })
        .unwrap_or_else(|| serde_json::json!({}));
    let targets=combat.and_then(|value|value["snapshot"]["combatants"].as_array()).map(|actors|actors.iter().map(|actor|serde_json::json!({"id":actor["id"],"name":actor["name"],"display_name_key":actor["display_name_key"],"character_id":actor["character_id"],"position":actor["position"],"current_hp":actor["current_hp"],"is_dead":actor["is_dead"]})).collect::<Vec<_>>()).unwrap_or_default();
    serde_json::json!({"hero_name":hero_name,"adventure":adventure,"public_facts":public_facts,"public_routes":public_routes,"public_actions":public_actions,"presentation_glossary":presentation_glossary,"targets":targets,"authored_map":combat.map(|value|value["authored"]["map"].clone()),"location_names":{"town":"Veresk / Вереск","quay":"old quay / старый причал","archive":"drowned archive / затопленный архив","belfry":"lower belfry / нижняя звонница","warehouse":"warehouse / склад","shrine":"shrine / святилище"}})
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::adventure::{AdventureProjection, AdventureState, Fact, Location};

    #[test]
    fn route_parameters_and_discovered_facts_reach_intention_checks_without_secrets() {
        let mut state = AdventureState::new();
        state.location = Location::Quay;
        let projection = AdventureProjection::from(state.clone());
        let before = public_intention_context(Some(&projection), None, Some("Нора"));
        assert_eq!(
            before["public_routes"][0]["args"]["action"],
            serde_json::json!({"kind":"move","location":"archive"})
        );
        assert!(before["public_routes"][0]["description"]
            .as_str()
            .unwrap()
            .contains("dry upper path"));
        assert_eq!(before["public_routes"][0]["currently_available"], true);
        assert!(!before["public_facts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["id"] == "ward_inspection"));
        state.facts.insert(Fact::WardDamaged);
        let after = public_intention_context(
            Some(&AdventureProjection::from(state.clone())),
            None,
            Some("Нора"),
        );
        assert!(after["public_facts"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f["id"] == "ward_inspection"));
        let public = after.to_string();
        for secret in [
            "sell its silver fitting",
            "Varden cut",
            "Savely died",
            "found alive",
        ] {
            assert!(
                !public.contains(secret),
                "public context contains private information"
            );
        }
        state.closed = true;
        let closed =
            public_intention_context(Some(&AdventureProjection::from(state.clone())), None, None);
        assert_eq!(closed["public_routes"][0]["currently_available"], false);
        assert_eq!(
            closed["public_routes"][0]["args"],
            after["public_routes"][0]["args"]
        );
        state.location = Location::Archive;
        let reverse =
            public_intention_context(Some(&AdventureProjection::from(state.clone())), None, None);
        assert_eq!(reverse["public_routes"][0]["to"], "quay");
        state.package_id = "other-adventure".into();
        let other = public_intention_context(Some(&AdventureProjection::from(state)), None, None);
        assert_eq!(other["public_routes"], serde_json::json!([]));
        assert_eq!(other["public_facts"], serde_json::json!([]));
        assert_eq!(other["public_actions"], serde_json::json!([]));
        assert_eq!(other["presentation_glossary"], serde_json::json!({}));
        assert_eq!(
            after["presentation_glossary"]["memorial_roll"]["ru"],
            "памятный список спасённых"
        );
    }

    #[test]
    fn action_references_describe_intentions_without_revealing_their_results() {
        let mut state = AdventureState::new();
        state.location = Location::Archive;
        let context =
            public_intention_context(Some(&AdventureProjection::from(state.clone())), None, None);
        let search = context["public_actions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["args"]["action"]["kind"] == "locate_tikhon")
            .unwrap();
        assert!(search["purpose"].as_str().unwrap().contains("Окликнуть"));
        assert!(!context.to_string().contains("found alive"));
        assert!(!context["public_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["args"]["action"]["kind"] == "rescue_tikhon"));
        state.facts.insert(Fact::TikhonLocated);
        let known =
            public_intention_context(Some(&AdventureProjection::from(state.clone())), None, None);
        assert!(known["public_actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["args"]["action"]["kind"] == "rescue_tikhon"));
        state.closed = true;
        assert_eq!(
            public_intention_context(Some(&AdventureProjection::from(state)), None, None)
                ["public_actions"],
            serde_json::json!([])
        );
        let mut belfry = AdventureState::new();
        belfry.location = Location::Belfry;
        assert!(!belfry.facts.contains(&Fact::RosterAvailable));
        let refs = public_intention_context(Some(&AdventureProjection::from(belfry)), None, None);
        let echo = refs["public_actions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["args"]["action"]["kind"] == "acknowledge_echo")
            .unwrap();
        assert!(!echo["purpose"].as_str().unwrap().contains("record"));
        assert!(!refs.to_string().contains("Savely"));
    }
}
