use app_application::combat::{
    authored,
    hero::{bind_party, recover_party_loadouts},
};
use serde_json::json;
use uuid::Uuid;

fn sheets() -> Vec<(Uuid, serde_json::Value)> {
    (0..4).map(|_|(Uuid::new_v4(),json!({"name":"Mira","heroClass":"fighter","level":1,"hp":12,"hpMax":12,"ac":16,"abilities":{"str":16,"dex":14,"con":14,"int":10,"wis":12,"cha":8},"inventory":[]}))).collect()
}
#[test]
fn authored_defeat_recovers_every_party_hero_at_the_same_confirmed_outcome() {
    let sheets = sheets();
    let references = sheets
        .iter()
        .map(|(id, sheet)| (*id, sheet))
        .collect::<Vec<_>>();
    let mut projection = authored::bell_defense(Uuid::new_v4());
    bind_party(&references, &mut projection, 42).unwrap();
    let mut positions = std::collections::BTreeSet::new();
    for actor in &mut projection.snapshot.combatants {
        if actor.character_id.is_some() {
            assert!(positions.insert((actor.position.x, actor.position.y)));
            actor.apply_raw_damage(12);
        }
    }
    assert_eq!(
        authored::finalize(&mut projection, false),
        Some(app_domain::adventure::DefenseOutcome::Defeated)
    );
    authored::recover_defeated_hero(&mut projection).unwrap();
    assert_eq!(
        projection
            .snapshot
            .combatants
            .iter()
            .filter(|actor| actor.character_id.is_some() && actor.current_hp == 1)
            .count(),
        4
    );
}
#[test]
fn recovery_rejects_a_snapshot_missing_an_accepted_party_actor() {
    let sheets = sheets();
    let references = sheets
        .iter()
        .map(|(id, sheet)| (*id, sheet))
        .collect::<Vec<_>>();
    let mut projection = authored::bell_defense(Uuid::new_v4());
    bind_party(&references, &mut projection, 42).unwrap();
    projection
        .snapshot
        .combatants
        .retain(|actor| actor.character_id.is_none_or(|id| id.0 != sheets[1].0));
    assert!(recover_party_loadouts(&references, &mut projection).is_err());
}
