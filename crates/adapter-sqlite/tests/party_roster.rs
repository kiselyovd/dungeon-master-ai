use adapter_sqlite::{pool, SqliteStore};
use app_application::{
    campaigns::{CreateCampaign, CreateCampaignCommand},
    models::{
        campaign_state::CommitCampaignCharacter,
        party::{EnrollPartyCharacter, InitializeParty},
        session::CampaignSeed,
    },
    ports::repositories::{CampaignStateRepository, PartyRepository},
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

async fn fixture() -> (Arc<SqliteStore>, InitializeParty) {
    let db = sqlx::SqlitePool::connect("sqlite::memory:").await.unwrap();
    fixture_with_pool(db).await
}
async fn fixture_with_pool(db: sqlx::SqlitePool) -> (Arc<SqliteStore>, InitializeParty) {
    pool::migrate(&db).await.unwrap();
    let store = Arc::new(SqliteStore::new(db));
    let campaign = CreateCampaign::new(store.clone())
        .execute(CreateCampaignCommand {
            request_id: None,
            title: "Party".into(),
            premise: "Two heroes".into(),
            seed: CampaignSeed::Bundled {
                adventure_id: "bell-beneath-water".into(),
            },
        })
        .await
        .unwrap();
    let current=store.commit_character(CommitCampaignCharacter{campaign_id:campaign.id,session_id:campaign.last_session_id,character_id:Uuid::new_v4(),expected_revision:campaign.revision,causation_id:"host-sheet".into(),character:json!({"name":"Mira","heroClass":"fighter","level":1,"hp":12,"hpMax":12,"ac":14,"abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[],"backstory":"HOST_PRIVATE"})}).await.unwrap();
    (
        store,
        InitializeParty {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: current.campaign.revision,
            causation_id: "initialize".into(),
        },
    )
}

async fn save(
    store: &SqliteStore,
    party: &app_application::models::party::PartyState,
    id: &str,
) -> app_application::models::snapshot::SnapshotCreated {
    use app_application::ports::repositories::SnapshotRepository;
    store
        .create_snapshot(app_application::models::snapshot::CreateSnapshotRecord {
            id: Uuid::new_v4(),
            campaign_id: party.campaign_id,
            session_id: party.session_id,
            expected_revision: party.revision,
            schema_version: 1,
            ruleset_id: "srd-5.2.1".into(),
            title: "Party checkpoint".into(),
            kind: "manual".into(),
            summary: String::new(),
            tag: String::new(),
            causation_id: id.into(),
            character: json!({}),
        })
        .await
        .unwrap()
}
#[tokio::test]
async fn party_snapshot_restores_guest_state_and_rejects_a_different_roster() {
    use app_application::models::snapshot::RestoreSnapshotRecord;
    use app_application::ports::repositories::SnapshotRepository;
    let (store, init) = fixture().await;
    let party = store.initialize_party(init.clone()).await.unwrap();
    let solo_save = save(&store, &party, "before-guest").await;
    let party = store.party(init.campaign_id).await.unwrap().unwrap();
    let command = guest(&party);
    let guest_id = command.character_id;
    let joined = store.enroll_party_character(command).await.unwrap();
    let checkpoint = save(&store, &joined, "with-guest").await;
    let saved = store.party(init.campaign_id).await.unwrap().unwrap();
    sqlx::query("UPDATE party_characters SET character_json=json_set(character_json,'$.hp',2) WHERE campaign_id=?1 AND character_id=?2").bind(init.campaign_id.0.to_string()).bind(guest_id.to_string()).execute(store.pool()).await.unwrap();
    let restore = RestoreSnapshotRecord {
        save_id: checkpoint.save_id,
        campaign_id: init.campaign_id,
        session_id: init.session_id,
        expected_revision: saved.revision,
        causation_id: "restore-party".into(),
    };
    store.restore_snapshot(restore).await.unwrap();
    let restored = store.party(init.campaign_id).await.unwrap().unwrap();
    assert_eq!(restored.characters[&guest_id]["hp"], 7);
    assert!(store
        .restore_snapshot(RestoreSnapshotRecord {
            save_id: solo_save.save_id,
            campaign_id: init.campaign_id,
            session_id: init.session_id,
            expected_revision: restored.revision,
            causation_id: "lose-guest".into()
        })
        .await
        .is_err());
    assert_eq!(
        store.party(init.campaign_id).await.unwrap().unwrap(),
        restored
    );
}
#[tokio::test]
async fn membership_and_receipts_survive_pool_close_and_reopen() {
    let path = std::env::temp_dir().join(format!("dmai-party-{}.sqlite", Uuid::new_v4()));
    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(&path)
        .create_if_missing(true);
    let db = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options.clone())
        .await
        .unwrap();
    let (store, init) = fixture_with_pool(db).await;
    let party = store.initialize_party(init.clone()).await.unwrap();
    let command = guest(&party);
    let joined = store.enroll_party_character(command.clone()).await.unwrap();
    store.pool().close().await;
    drop(store);
    let db = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(1)
        .connect_with(options)
        .await
        .unwrap();
    let reopened = SqliteStore::new(db);
    assert_eq!(
        reopened.party(init.campaign_id).await.unwrap().unwrap(),
        joined
    );
    assert_eq!(
        reopened.enroll_party_character(command).await.unwrap(),
        joined
    );
    reopened.pool().close().await;
    drop(reopened);
    std::fs::remove_file(path).unwrap();
}

#[tokio::test]
async fn malformed_duplicate_guest_snapshot_cannot_overwrite_current_party() {
    use app_application::ports::repositories::SnapshotRepository;
    let (store, init) = fixture().await;
    let party = store.initialize_party(init.clone()).await.unwrap();
    let joined = store.enroll_party_character(guest(&party)).await.unwrap();
    let checkpoint = save(&store, &joined, "snapshot").await;
    let before = store.party(init.campaign_id).await.unwrap().unwrap();
    let mut document = serde_json::to_value(&checkpoint.snapshot.projection).unwrap();
    let mut duplicate = document["party"]["members"][0].clone();
    duplicate["character"]["hp"] = json!(1);
    document["party"]["members"]
        .as_array_mut()
        .unwrap()
        .push(duplicate);
    sqlx::query("UPDATE campaign_snapshots SET projection_json=?2 WHERE id=?1")
        .bind(checkpoint.snapshot.id.to_string())
        .bind(document.to_string())
        .execute(store.pool())
        .await
        .unwrap();
    assert!(store
        .restore_snapshot(app_application::models::snapshot::RestoreSnapshotRecord {
            save_id: checkpoint.save_id,
            campaign_id: init.campaign_id,
            session_id: init.session_id,
            expected_revision: before.revision,
            causation_id: "malformed-restore".into()
        })
        .await
        .is_err());
    assert_eq!(
        store.party(init.campaign_id).await.unwrap().unwrap(),
        before
    );
}

#[tokio::test]
async fn failed_second_guest_restore_rolls_back_the_first_guest_and_revision() {
    use app_application::ports::repositories::SnapshotRepository;
    let (store, init) = fixture().await;
    let party = store.initialize_party(init.clone()).await.unwrap();
    let first = store.enroll_party_character(guest(&party)).await.unwrap();
    let mut command = guest(&first);
    command.causation_id = "second-guest".into();
    let joined = store.enroll_party_character(command).await.unwrap();
    let checkpoint = save(&store, &joined, "snapshot").await;
    sqlx::query("UPDATE party_characters SET character_json=json_set(character_json,'$.hp',2) WHERE campaign_id=?1").bind(init.campaign_id.0.to_string()).execute(store.pool()).await.unwrap();
    let before = store.party(init.campaign_id).await.unwrap().unwrap();
    sqlx::raw_sql("CREATE TRIGGER fail_second_guest BEFORE UPDATE ON party_characters WHEN NEW.member_id=(SELECT member_id FROM party_characters WHERE campaign_id=NEW.campaign_id ORDER BY rowid DESC LIMIT 1) BEGIN SELECT RAISE(ABORT,'injected'); END")
        .execute(store.pool())
        .await
        .unwrap();
    assert!(store
        .restore_snapshot(app_application::models::snapshot::RestoreSnapshotRecord {
            save_id: checkpoint.save_id,
            campaign_id: init.campaign_id,
            session_id: init.session_id,
            expected_revision: before.revision,
            causation_id: "failed-restore".into()
        })
        .await
        .is_err());
    assert_eq!(
        store.party(init.campaign_id).await.unwrap().unwrap(),
        before
    );
}
fn guest(party: &app_application::models::party::PartyState) -> EnrollPartyCharacter {
    EnrollPartyCharacter {
        campaign_id: party.campaign_id,
        session_id: party.session_id,
        expected_revision: party.revision,
        causation_id: "guest".into(),
        verified_host_member_id: party.roster.host_member_id,
        member_id: Uuid::new_v4(),
        character_id: Uuid::new_v4(),
        character: json!({"name":"Mira","heroClass":"wizard","level":1,"hp":7,"hpMax":8,"ac":12,"abilities":{"str":8,"dex":14,"con":14,"int":16,"wis":12,"cha":10},"inventory":[],"backstory":"GUEST_PRIVATE"}),
    }
}
#[tokio::test]
async fn party_persists_two_same_name_heroes_and_returns_private_sheet_only_to_its_owner() {
    let (store, init) = fixture().await;
    let host_before = store
        .load_current(init.campaign_id)
        .await
        .unwrap()
        .projection
        .character;
    let party = store.initialize_party(init.clone()).await.unwrap();
    let command = guest(&party);
    let member = command.member_id;
    let joined = store.enroll_party_character(command.clone()).await.unwrap();
    assert_eq!(joined.roster.members.len(), 2);
    assert_eq!(joined.revision.0, party.revision.0 + 1);
    let host_view = joined.view_for(joined.roster.host_member_id).unwrap();
    assert!(!serde_json::to_string(&host_view)
        .unwrap()
        .contains("GUEST_PRIVATE"));
    let guest_view = joined.view_for(member).unwrap();
    assert_eq!(guest_view.own_character["backstory"], "GUEST_PRIVATE");
    assert!(!serde_json::to_string(&guest_view)
        .unwrap()
        .contains("HOST_PRIVATE"));
    assert_eq!(guest_view.actors[0].name, guest_view.actors[1].name);
    assert_eq!(
        store
            .load_current(init.campaign_id)
            .await
            .unwrap()
            .projection
            .character,
        host_before
    );
    assert_eq!(
        store.enroll_party_character(command.clone()).await.unwrap(),
        joined
    );
    let mut changed = command;
    changed.character["name"] = json!("Changed");
    assert!(store.enroll_party_character(changed).await.is_err());
    assert_eq!(
        store.party(init.campaign_id).await.unwrap().unwrap(),
        joined
    );
}
#[tokio::test]
async fn non_host_and_receipt_failure_cannot_change_party_or_campaign_revision() {
    let (store, init) = fixture().await;
    let party = store.initialize_party(init.clone()).await.unwrap();
    let mut command = guest(&party);
    command.verified_host_member_id = Uuid::new_v4();
    assert!(store.enroll_party_character(command.clone()).await.is_err());
    command.verified_host_member_id = party.roster.host_member_id;
    sqlx::query("CREATE TRIGGER fail_party_receipt BEFORE INSERT ON party_operation_receipts WHEN NEW.causation_id='guest' BEGIN SELECT RAISE(ABORT,'injected'); END").execute(store.pool()).await.unwrap();
    assert!(store.enroll_party_character(command).await.is_err());
    assert_eq!(store.party(init.campaign_id).await.unwrap().unwrap(), party);
}

#[tokio::test]
async fn solo_character_endpoint_cannot_take_a_guest_character_identity() {
    let (store, init) = fixture().await;
    let party = store.initialize_party(init.clone()).await.unwrap();
    let command = guest(&party);
    let guest_id = command.character_id;
    let joined = store.enroll_party_character(command).await.unwrap();
    let host = store.load_current(init.campaign_id).await.unwrap();
    let mut sheet = host.projection.character.clone();
    sheet["characterId"] = json!(guest_id);
    let replaced = store
        .commit_character(CommitCampaignCharacter {
            campaign_id: init.campaign_id,
            session_id: init.session_id,
            character_id: guest_id,
            expected_revision: joined.revision,
            causation_id: "steal-guest-id".into(),
            character: sheet,
        })
        .await;
    assert!(replaced.is_err());
    assert_eq!(store.load_current(init.campaign_id).await.unwrap(), host);
    assert_eq!(
        store.party(init.campaign_id).await.unwrap().unwrap(),
        joined
    );
}
