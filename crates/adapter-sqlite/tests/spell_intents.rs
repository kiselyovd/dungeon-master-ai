use adapter_sqlite::{pool, SqliteStore};
use app_application::{
    campaigns::{CreateCampaign, CreateCampaignCommand, CreateSnapshot, CreateSnapshotCommand},
    combat::{
        commands::{CombatActionCommand, ResolveCombatCommand},
        resolve::ResolveCombatAction,
    },
    models::{
        campaign_state::CommitCampaignCharacter,
        combat::{CombatProjection, CombatSnapshot, COMBAT_PROJECTION_VERSION},
        session::CampaignSeed,
        snapshot::RestoreSnapshotRecord,
    },
    ports::repositories::{CampaignStateRepository, CombatRepository, SnapshotRepository},
};
use app_domain::combat::{combatant::Combatant, initiative::InitiativeEntry, types::CombatantId};
use serde_json::json;
use std::{collections::BTreeMap, sync::Arc};
use uuid::Uuid;

#[tokio::test]
async fn spell_resources_commit_replay_and_restore_with_the_hero() {
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            title: "Magic".into(),
            premise: "Bridge".into(),
            seed: CampaignSeed::Collaborative {
                answers: BTreeMap::from([("world".into(), vec!["Bridge".into()])]),
            },
        })
        .await
        .unwrap();
    let character = Uuid::new_v4();
    let sheet = json!({"name":"Mira","heroClass":"cleric","level":1,"hp":2,"hpMax":20,"ac":16,"abilities":{"str":12,"dex":14,"con":14,"int":10,"wis":16,"cha":8},"inventory":[],"spells":{"cantrips":["sacred-flame"],"level1":["cure-wounds","healing-word"]}});
    store
        .commit_character(CommitCampaignCharacter {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            character_id: character,
            expected_revision: campaign.revision,
            causation_id: "hero".into(),
            character: sheet.clone(),
        })
        .await
        .unwrap();
    let encounter = Uuid::new_v4();
    let npc = CombatantId::new();
    let projection = store
        .create(
            campaign.last_session_id.0,
            CombatProjection {
                schema_version: COMBAT_PROJECTION_VERSION,
                encounter_id: encounter,
                revision: 0,
                authored: None,
                adventure: None,
                campaign_scope: None,
                snapshot: CombatSnapshot {
                    active: true,
                    round: 1,
                    current_combatant: Some(npc),
                    initiative: vec![InitiativeEntry {
                        id: npc,
                        roll: 0,
                        dex_tiebreak: 0,
                    }],
                    combatants: vec![Combatant::new(npc, "Foe".into(), 100, 100, 10)],
                },
                events: vec![],
            },
        )
        .await
        .unwrap();
    let hero = projection
        .snapshot
        .combatants
        .iter()
        .find(|a| a.character_id.is_some())
        .unwrap()
        .id;
    let command = ResolveCombatCommand {
        request_id: "heal".into(),
        encounter_id: encounter,
        expected_revision: 0,
        rng_seed: 42,
        action: CombatActionCommand::Spell {
            caster: hero,
            spell_id: "cure-wounds".into(),
            targets: vec![hero],
            slot_level: 1,
        },
    };
    let first = ResolveCombatAction::new(store.clone())
        .execute(command.clone())
        .await
        .unwrap();
    assert!(first.projection.snapshot.combatants[0].current_hp > 2);
    assert_eq!(
        store
            .load_current(campaign.id)
            .await
            .unwrap()
            .projection
            .character["combatResources"]["spellSlots"]["1"],
        1
    );
    assert!(
        ResolveCombatAction::new(store.clone())
            .execute(command)
            .await
            .unwrap()
            .duplicate
    );
    let current = store.load_current(campaign.id).await.unwrap();
    let saved = CreateSnapshot::new(store.clone())
        .execute(CreateSnapshotCommand {
            id: Uuid::new_v4(),
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: current.campaign.revision,
            title: "After healing".into(),
            kind: "manual".into(),
            summary: "".into(),
            tag: "combat".into(),
            causation_id: "save".into(),
            character: sheet,
        })
        .await
        .unwrap();
    assert_eq!(
        saved.snapshot.projection.character["combatResources"]["spellSlots"]["1"],
        1
    );
    ResolveCombatAction::new(store.clone())
        .execute(ResolveCombatCommand {
            request_id: "end".into(),
            encounter_id: encounter,
            expected_revision: 1,
            rng_seed: 42,
            action: CombatActionCommand::EndTurn { combatant: hero },
        })
        .await
        .unwrap();
    ResolveCombatAction::new(store.clone())
        .execute(ResolveCombatCommand {
            request_id: "heal-again".into(),
            encounter_id: encounter,
            expected_revision: 2,
            rng_seed: 43,
            action: CombatActionCommand::Spell {
                caster: hero,
                spell_id: "cure-wounds".into(),
                targets: vec![hero],
                slot_level: 1,
            },
        })
        .await
        .unwrap();
    let current = store.load_current(campaign.id).await.unwrap();
    assert_eq!(
        current.projection.character["combatResources"]["spellSlots"]["1"],
        0
    );
    store
        .restore_snapshot(RestoreSnapshotRecord {
            save_id: saved.save_id,
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: current.campaign.revision,
            causation_id: "restore".into(),
        })
        .await
        .unwrap();
    let restored = store.get(encounter).await.unwrap().unwrap();
    assert_eq!(
        restored.snapshot.combatants[0]
            .casting
            .as_ref()
            .unwrap()
            .slots[&1]
            .remaining,
        1
    );
    assert_eq!(
        store
            .load_current(campaign.id)
            .await
            .unwrap()
            .projection
            .character["combatResources"]["spellSlots"]["1"],
        1
    );
    assert!(restored.snapshot.combatants[0].budget.spell_slot_used);
}
