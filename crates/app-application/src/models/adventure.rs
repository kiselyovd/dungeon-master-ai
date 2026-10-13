use super::identity::{CampaignId, Revision, SessionId};
pub use app_domain::adventure::{
    Action, AdventureState, DefenseOutcome, Fact, Location, BELL_ADVENTURE_ID,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AdventureProjection {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map_skin_asset_id: Option<String>,
    /// Atmosphere for the current public location, never tactical geometry.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scene_asset_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map: Option<app_domain::combat::geometry::BattleMap>,
    pub state: AdventureState,
    pub available_actions: Vec<Action>,
    /// Explicit objectives offered by the current public scene. No future or
    /// private-canon objective is inferred by a presentation adapter.
    #[serde(default)]
    pub public_goals: Vec<Action>,
    /// Portraits permitted by confirmed meetings/discoveries; never private DM canon.
    #[serde(default)]
    pub public_npcs: Vec<PublicAdventureNpc>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PublicAdventureNpc {
    pub id: String,
    pub portrait_asset_id: String,
}

fn public_npcs(state: &AdventureState) -> Vec<PublicAdventureNpc> {
    if state.package_id != BELL_ADVENTURE_ID {
        return Vec::new();
    }
    let visited =
        |location| state.location == location || state.history.contains(&Action::Move(location));
    let met = [
        ("mila", true),
        ("agata", true),
        (
            "leon",
            visited(Location::Shrine) || state.facts.contains(&Fact::WardUnderstood),
        ),
        ("varden", visited(Location::Warehouse)),
        (
            "tikhon",
            state.facts.contains(&Fact::TikhonLocated)
                || state.facts.contains(&Fact::TikhonRescued),
        ),
        (
            "savely",
            state.facts.contains(&Fact::RosterAvailable)
                || state.facts.contains(&Fact::EchoAcknowledged),
        ),
    ];
    met.into_iter()
        .filter(|(_, visible)| *visible)
        .map(|(id, _)| PublicAdventureNpc {
            id: id.into(),
            portrait_asset_id: format!("npc.bell.{id}.v2"),
        })
        .collect()
}

impl From<AdventureState> for AdventureProjection {
    fn from(state: AdventureState) -> Self {
        let available_actions = state.available_actions();
        let public_goals = available_actions
            .iter()
            .filter(|action| !matches!(action, Action::Move(_) | Action::Finish))
            .copied()
            .collect();
        Self {
            map_skin_asset_id: (state.package_id == BELL_ADVENTURE_ID
                && state.location == Location::Archive)
                .then(|| "map.bell_archive.v2".into()),
            scene_asset_id: (state.package_id == BELL_ADVENTURE_ID).then(|| {
                let location = match state.location {
                    Location::Town => "town",
                    Location::Quay => "quay",
                    Location::Shrine => "shrine",
                    Location::Warehouse => "warehouse",
                    Location::Archive => "archive",
                    Location::Belfry => "belfry",
                };
                format!("scene.bell.{location}.v2")
            }),
            public_npcs: public_npcs(&state),
            map: if state.location == Location::Archive {
                Some(app_domain::combat::geometry::bell_archive_map())
            } else {
                None
            },
            state,
            available_actions,
            public_goals,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdventureCommand {
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub character_id: Uuid,
    pub expected_revision: Revision,
    pub causation_id: String,
    pub action: Action,
}

#[derive(Debug, Clone)]
pub struct AdventureIntent {
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub causation_id: String,
    pub action: Action,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_scene_art_is_public_location_bound_and_legacy_optional() {
        for (location, id) in [
            (Location::Town, "town"),
            (Location::Quay, "quay"),
            (Location::Shrine, "shrine"),
            (Location::Warehouse, "warehouse"),
            (Location::Archive, "archive"),
            (Location::Belfry, "belfry"),
        ] {
            let mut state = AdventureState::new();
            state.location = location;
            let projection = AdventureProjection::from(state);
            assert_eq!(
                projection.scene_asset_id,
                Some(format!("scene.bell.{id}.v2"))
            );
            assert_eq!(projection.map.is_some(), location == Location::Archive);
            assert_eq!(
                projection.map_skin_asset_id.as_deref(),
                if location == Location::Archive {
                    Some("map.bell_archive.v2")
                } else {
                    None
                }
            );
            let mut value = serde_json::to_value(&projection).unwrap();
            value.as_object_mut().unwrap().remove("scene_asset_id");
            value.as_object_mut().unwrap().remove("map_skin_asset_id");
            let legacy: AdventureProjection = serde_json::from_value(value).unwrap();
            assert_eq!(legacy.scene_asset_id, None);
            assert_eq!(legacy.map_skin_asset_id, None);
        }
        let mut unknown = AdventureState::new();
        unknown.package_id = "another-package".into();
        assert_eq!(AdventureProjection::from(unknown).scene_asset_id, None);
    }

    #[test]
    fn authored_portraits_reveal_only_confirmed_people() {
        let state = AdventureState::new();
        let ids = |state: AdventureState| {
            AdventureProjection::from(state)
                .public_npcs
                .into_iter()
                .map(|npc| npc.id)
                .collect::<Vec<_>>()
        };
        assert_eq!(ids(state.clone()), vec!["mila", "agata"]);
        let mut found = state.clone();
        found.facts.insert(Fact::TikhonLocated);
        assert!(ids(found).contains(&"tikhon".into()));
        assert!(!ids(state.clone()).contains(&"savely".into()));
        let mut known = state;
        known.history.push(Action::Move(Location::Shrine));
        known.history.push(Action::Move(Location::Warehouse));
        known.facts.insert(Fact::RosterAvailable);
        known.facts.insert(Fact::TikhonRescued);
        assert_eq!(
            ids(known.clone()),
            vec!["mila", "agata", "leon", "varden", "tikhon", "savely"]
        );
        let restored: AdventureState =
            serde_json::from_str(&serde_json::to_string(&known).unwrap()).unwrap();
        assert_eq!(ids(restored), ids(known));
    }

    #[test]
    fn public_portrait_metadata_has_no_private_canon_or_future_identity() {
        let state = AdventureState::new();
        let json = serde_json::to_value(AdventureProjection::from(state).public_npcs).unwrap();
        for npc in json.as_array().unwrap() {
            assert_eq!(npc.as_object().unwrap().len(), 2);
            assert!(npc["portrait_asset_id"]
                .as_str()
                .unwrap()
                .starts_with("npc.bell."));
        }
        let serialized = json.to_string();
        for hidden in [
            "savely",
            "varden",
            "tikhon",
            "silver",
            "culprit",
            "biography",
        ] {
            assert!(!serialized.contains(hidden));
        }
    }

    #[test]
    fn public_goals_only_offer_current_scene_actions() {
        let projection = AdventureProjection::from(AdventureState::new());
        assert!(!projection.public_goals.is_empty());
        assert!(projection.public_goals.iter().all(|goal| {
            projection.available_actions.contains(goal)
                && !matches!(goal, Action::Move(_) | Action::Finish)
        }));
        assert!(!projection.public_goals.contains(&Action::RecoverLedger));
        let mut closed = projection.state;
        closed.closed = true;
        assert!(AdventureProjection::from(closed).public_goals.is_empty());
    }
}
