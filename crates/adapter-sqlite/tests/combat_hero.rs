use adapter_sqlite::{pool, SqliteStore};
use app_application::campaigns::{CreateCampaign, CreateCampaignCommand};
use app_application::models::{
    campaign_state::CommitCampaignCharacter,
    combat::{CombatProjection, CombatSnapshot, COMBAT_PROJECTION_VERSION},
    identity::Revision,
    session::CampaignSeed,
};
use app_application::ports::repositories::{CampaignStateRepository, CombatRepository};
use app_domain::combat::{combatant::Combatant, initiative::InitiativeEntry, types::CombatantId};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

async fn fixture() -> (Arc<SqliteStore>, CommitCampaignCharacter, Uuid) {
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            title: "Bell".into(),
            premise: "Ferry".into(),
            seed: CampaignSeed::Bundled {
                adventure_id: "bell-beneath-water".into(),
            },
        })
        .await
        .unwrap();
    let command = CommitCampaignCharacter {
        campaign_id: campaign.id,
        session_id: campaign.last_session_id,
        character_id: Uuid::new_v4(),
        expected_revision: campaign.revision,
        causation_id: "hero".into(),
        character: json!({"name":"Mira","heroClass":"fighter","level":1,"hp":7,"hpMax":12,"ac":16,"speedFt":35,"initiative":99,"abilities":{"str":16,"dex":14,"con":14,"int":10,"wis":12,"cha":8},"inventory":[]}),
    };
    store.commit_character(command.clone()).await.unwrap();
    let id = Uuid::new_v4();
    let npc = CombatantId::new();
    let fake_hero = Combatant::new(
        CombatantId(command.character_id),
        "Invented replacement".into(),
        999,
        999,
        40,
    );
    let same_name = Combatant::new(npc, "Mira".into(), 8, 8, 12);
    let projection = CombatProjection {
        campaign_scope: None,
        schema_version: COMBAT_PROJECTION_VERSION,
        encounter_id: id,
        revision: 0,
        snapshot: CombatSnapshot {
            active: true,
            round: 1,
            current_combatant: Some(npc),
            initiative: vec![
                InitiativeEntry {
                    id: npc,
                    roll: 10,
                    dex_tiebreak: 1,
                },
                InitiativeEntry {
                    id: fake_hero.id,
                    roll: 99,
                    dex_tiebreak: 99,
                },
            ],
            combatants: vec![same_name, fake_hero],
        },
        events: vec![],
    };
    store
        .create(command.session_id.0, projection)
        .await
        .unwrap();
    (store, command, id)
}

#[tokio::test]
async fn start_binds_the_durable_hero_and_damage_commits_sheet_revision_and_receipt_together() {
    let (store, command, id) = fixture().await;
    let mut projection = store.get(id).await.unwrap().unwrap();
    let hero = projection
        .snapshot
        .combatants
        .iter()
        .find(|actor| actor.character_id.is_some())
        .unwrap();
    assert_eq!(hero.character_id.unwrap().0, command.character_id);
    assert_ne!(hero.id.0, command.character_id);
    assert_eq!(
        (
            hero.current_hp,
            hero.max_hp,
            hero.ac,
            hero.dex_mod,
            hero.speed_ft
        ),
        (7, 12, 16, 2, 35)
    );
    assert_eq!(projection.snapshot.combatants.len(), 2);
    assert!(projection
        .snapshot
        .combatants
        .iter()
        .all(|actor| actor.name == "Mira"));
    assert_eq!(projection.campaign_scope.as_ref().unwrap().revision, 3);
    projection
        .snapshot
        .combatants
        .iter_mut()
        .find(|actor| actor.character_id.is_some())
        .unwrap()
        .apply_raw_damage(3);
    projection.revision = 1;
    let committed = store
        .commit_action(0, projection, "hit", "{}")
        .await
        .unwrap();
    assert_eq!(committed.projection.campaign_scope.unwrap().revision, 4);
    let current = store.load_current(command.campaign_id).await.unwrap();
    assert_eq!(current.projection.character["hp"], 4);
    assert_eq!(current.campaign.revision, Revision(4));
    assert!(store.find_action(id, "hit", "{}").await.unwrap().is_some());
    assert_eq!(
        store
            .load_current(command.campaign_id)
            .await
            .unwrap()
            .campaign
            .revision,
        Revision(4)
    );
}

#[tokio::test]
async fn failed_receipt_rolls_back_the_sheet_and_campaign_revision() {
    let (store, command, id) = fixture().await;
    sqlx::query("CREATE TRIGGER reject_hero_receipt BEFORE INSERT ON combat_action_receipts BEGIN SELECT RAISE(ABORT,'forced'); END").execute(store.pool()).await.unwrap();
    let mut projection = store.get(id).await.unwrap().unwrap();
    projection.revision = 1;
    projection
        .snapshot
        .combatants
        .iter_mut()
        .find(|actor| actor.character_id.is_some())
        .unwrap()
        .apply_raw_damage(3);
    assert!(store
        .commit_action(0, projection, "hit", "{}")
        .await
        .is_err());
    let current = store.load_current(command.campaign_id).await.unwrap();
    assert_eq!(current.projection.character["hp"], 7);
    assert_eq!(current.campaign.revision, Revision(3));
    assert_eq!(store.get(id).await.unwrap().unwrap().revision, 0);
}

#[tokio::test]
async fn a_sheet_cannot_be_replaced_during_its_bound_encounter() {
    let (store, mut command, _) = fixture().await;
    command.expected_revision = Revision(3);
    command.causation_id = "overwrite".into();
    command.character["hp"] = json!(12);
    assert!(
        store.commit_character(command).await.is_err(),
        "editing the sheet cannot reset combat damage or replace the bound actor"
    );
}

#[tokio::test]
async fn compatibility_token_writes_cannot_bypass_the_bound_projection() {
    let (store, command, id) = fixture().await;
    let projection = store.get(id).await.unwrap().unwrap();
    let hero = projection
        .snapshot
        .combatants
        .iter()
        .find(|actor| actor.character_id.is_some())
        .unwrap();
    assert!(store.set_token_hp(&hero.id.0.to_string(), 1).await.is_err());
    assert!(store.mark_token_dead(&hero.id.0.to_string()).await.is_err());
    assert_eq!(
        store
            .load_current(command.campaign_id)
            .await
            .unwrap()
            .projection
            .character["hp"],
        7
    );
}

#[tokio::test]
async fn saving_during_bound_combat_uses_the_canonical_sheet() {
    use app_application::campaigns::{CreateSnapshot, CreateSnapshotCommand};
    let (store, command, _) = fixture().await;
    let mut invented = command.character.clone();
    invented["characterId"] = json!(Uuid::new_v4());
    invented["hpMax"] = json!(999);
    invented["hp"] = json!(999);
    let saved = CreateSnapshot::new(store.clone())
        .execute(CreateSnapshotCommand {
            id: Uuid::new_v4(),
            campaign_id: command.campaign_id,
            session_id: command.session_id,
            expected_revision: Revision(3),
            title: "Bound checkpoint".into(),
            kind: "manual".into(),
            summary: "".into(),
            tag: "combat".into(),
            causation_id: "bound-save".into(),
            character: invented,
        })
        .await
        .unwrap();
    assert_eq!(
        saved.snapshot.projection.character["characterId"],
        json!(command.character_id)
    );
    assert_eq!(saved.snapshot.projection.character["hp"], 7);
    assert_eq!(
        store
            .load_current(command.campaign_id)
            .await
            .unwrap()
            .character_id,
        Some(command.character_id)
    );
}
