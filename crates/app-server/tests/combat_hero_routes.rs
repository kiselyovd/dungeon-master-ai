use adapter_sqlite::SqliteStore;
use app_application::{
    campaigns::{CreateCampaign, CreateCampaignCommand},
    models::{campaign_state::CommitCampaignCharacter, session::CampaignSeed},
    ports::repositories::{CampaignStateRepository, CombatRepository},
};
use app_server::test_support::TestServer;
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

async fn campaign(
    store: Arc<SqliteStore>,
    title: &str,
) -> app_application::models::session::CampaignDetail {
    CreateCampaign::new(store)
        .execute(CreateCampaignCommand {
            request_id: None,
            title: title.into(),
            premise: "Bridge".into(),
            seed: CampaignSeed::Collaborative {
                answers: std::collections::BTreeMap::from([(
                    "world".into(),
                    vec!["Bridge".into()],
                )]),
            },
        })
        .await
        .unwrap()
}
async fn hero(
    store: &SqliteStore,
    campaign: &app_application::models::session::CampaignDetail,
) -> Uuid {
    let id = Uuid::new_v4();
    store.commit_character(CommitCampaignCharacter{campaign_id:campaign.id,session_id:campaign.last_session_id,character_id:id,expected_revision:campaign.revision,causation_id:"hero".into(),character:json!({"name":"Mira","heroClass":"fighter","level":1,"hp":7,"hpMax":12,"ac":16,"abilities":{"str":16,"dex":14,"con":14,"int":10,"wis":12,"cha":8},"inventory":[]})}).await.unwrap();
    id
}

#[tokio::test]
async fn combat_start_rejects_another_campaigns_session_before_mutation() {
    let server = TestServer::start().await;
    let store = Arc::new(SqliteStore::new(server.state.db().clone()));
    let a = campaign(store.clone(), "A").await;
    let b = campaign(store.clone(), "B").await;
    hero(&store, &b).await;
    let response = reqwest::Client::new()
        .post(server.url("/combat/start"))
        .json(
            &json!({"campaign_id":a.id.0,"session_id":b.last_session_id.0,"initiative_entries":[]}),
        )
        .send()
        .await
        .unwrap();
    assert!(
        response.status().is_client_error(),
        "mismatched identity must not mutate B under A's lane"
    );
    assert!(store
        .load_current(b.id)
        .await
        .unwrap()
        .projection
        .combat
        .is_none());
}

#[tokio::test]
async fn model_start_uses_an_explicit_npc_contract_without_a_second_hero() {
    use app_llm::ToolCall;
    use app_server::agent::tool_executor::execute_tool;
    let server = TestServer::start().await;
    let store = Arc::new(SqliteStore::new(server.state.db().clone()));
    let campaign = campaign(store.clone(), "Role contract").await;
    let character = hero(&store, &campaign).await;
    let ambiguous = ToolCall {
        id: "old-start".into(),
        name: "start_combat".into(),
        args: json!({"initiative_entries":[{"name":"Mira"},{"name":"Goblin"}]}),
    };
    let (_, error) = execute_tool(
        &ambiguous,
        &store,
        None,
        None,
        None,
        "mock",
        campaign.id.0,
        campaign.last_session_id.0,
    )
    .await;
    assert!(
        error,
        "an ambiguous legacy list must not create another owned hero"
    );
    assert!(store
        .load_current(campaign.id)
        .await
        .unwrap()
        .projection
        .combat
        .is_none());
    let call = ToolCall {
        id: "npc-start".into(),
        name: "start_combat".into(),
        args: json!({"initiative_entries":[{"role":"npc","name":"Goblin"}]}),
    };
    let (outcome, error) = execute_tool(
        &call,
        &store,
        None,
        None,
        None,
        "mock",
        campaign.id.0,
        campaign.last_session_id.0,
    )
    .await;
    assert!(!error, "{outcome}");
    let id = Uuid::parse_str(outcome["encounter_id"].as_str().unwrap()).unwrap();
    let stored = store.get(id).await.unwrap().unwrap();
    assert_eq!(stored.snapshot.combatants.len(), 2);
    assert_eq!(
        stored
            .snapshot
            .combatants
            .iter()
            .filter(|actor| actor.character_id.is_some_and(|id| id.0 == character))
            .count(),
        1
    );
    assert_eq!(outcome["projection"], serde_json::to_value(stored).unwrap());
}

#[tokio::test]
async fn solo_recovery_http_rejects_roll_injection_and_cannot_bypass_a_live_room() {
    use adapter_secrets::InMemorySecretsStore;
    use app_application::{
        campaigns::{GAME_SCHEMA_VERSION, RULESET_ID},
        models::room::CreateLocalRoom,
        rooms::{RoomService, SystemRoomClock},
    };
    let server = TestServer::start().await;
    let store = Arc::new(SqliteStore::new(server.state.db().clone()));
    let campaign = campaign(store.clone(), "Solo recovery").await;
    let id = hero(&store, &campaign).await;
    let mut state = store.load_current(campaign.id).await.unwrap();
    let mut actor = app_application::recovery::actor(&state.projection.character, id).unwrap();
    actor.current_hp = 0;
    actor.death_saves.stabilize();
    app_application::vitality::store_actor(&mut state.projection.character, &actor).unwrap();
    sqlx::query("UPDATE campaign_characters SET character_json=?1 WHERE campaign_id=?2")
        .bind(state.projection.character.to_string())
        .bind(campaign.id.0.to_string())
        .execute(store.pool())
        .await
        .unwrap();
    let url = server.url(&format!("/campaigns/{}/recovery", campaign.id.0));
    let client = reqwest::Client::new();
    let status = client.get(&url).send().await.unwrap();
    assert!(status.status().is_success());
    let status: serde_json::Value = status.json().await.unwrap();
    assert_eq!(status["hp"], 0);
    assert_eq!(status["recovery"]["status"], "stable");
    let wire_before = if std::env::var("DMAI_SOLO_RECOVERY_WIRE_FIXTURE").is_ok() {
        Some(
            json!({"status":status,"rest":client.get(server.url(&format!("/campaigns/{}/rest",campaign.id.0))).send().await.unwrap().json::<serde_json::Value>().await.unwrap(),"state":client.get(server.url(&format!("/campaigns/{}/state",campaign.id.0))).send().await.unwrap().json::<serde_json::Value>().await.unwrap()}),
        )
    } else {
        None
    };
    let body = json!({"schema_version":GAME_SCHEMA_VERSION,"ruleset_id":RULESET_ID,"session_id":campaign.last_session_id.0,"character_id":id,"expected_revision":state.campaign.revision.0,"causation_id":"solo-ui-recovery","intent":{"kind":"wait","target_character_id":id}});
    for field in ["rng_seed", "roll"] {
        let mut forged = body.clone();
        forged[field] = json!(20);
        assert_eq!(
            client
                .post(&url)
                .json(&forged)
                .send()
                .await
                .unwrap()
                .status()
                .as_u16(),
            422
        );
    }
    let mut forged = body.clone();
    forged["intent"] = json!({"kind":"death_save","roll":20});
    assert_eq!(
        client
            .post(&url)
            .json(&forged)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        422
    );
    let response = client.post(&url).json(&body).send().await.unwrap();
    assert!(response.status().is_success());
    let result: serde_json::Value = response.json().await.unwrap();
    assert_eq!(result["status"]["hp"], 1);
    assert_eq!(result["receipt"]["recovered"], true);
    let retry: serde_json::Value = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(retry["receipt"]["duplicate"], true);
    assert_eq!(retry["status"], result["status"]);
    if let (Ok(path), Some(before)) = (
        std::env::var("DMAI_SOLO_RECOVERY_WIRE_FIXTURE"),
        wire_before,
    ) {
        let after = json!({"result":result,"rest":client.get(server.url(&format!("/campaigns/{}/rest",campaign.id.0))).send().await.unwrap().json::<serde_json::Value>().await.unwrap(),"state":client.get(server.url(&format!("/campaigns/{}/state",campaign.id.0))).send().await.unwrap().json::<serde_json::Value>().await.unwrap()});
        let rested=client.post(server.url(&format!("/campaigns/{}/rest",campaign.id.0))).json(&json!({"schema_version":GAME_SCHEMA_VERSION,"ruleset_id":RULESET_ID,"session_id":campaign.last_session_id.0,"character_id":id,"expected_revision":result["status"]["revision"],"causation_id":"solo-after-recovery-rest","intent":{"kind":"long"}})).send().await.unwrap();
        assert!(rested.status().is_success());
        let rested: serde_json::Value = rested.json().await.unwrap();
        let after_rest = json!({"result":rested,"recovery":client.get(&url).send().await.unwrap().json::<serde_json::Value>().await.unwrap(),"state":client.get(server.url(&format!("/campaigns/{}/state",campaign.id.0))).send().await.unwrap().json::<serde_json::Value>().await.unwrap()});
        std::fs::write(
            path,
            serde_json::to_string_pretty(
                &json!({"before":before,"after":after,"afterRest":after_rest}),
            )
            .unwrap(),
        )
        .unwrap();
    }
    let current = store.load_current(campaign.id).await.unwrap();
    let rooms = RoomService::new(
        store.clone(),
        store.clone(),
        Arc::new(InMemorySecretsStore::default()),
        Arc::new(SystemRoomClock),
    );
    rooms
        .create_local_room(CreateLocalRoom {
            campaign_id: campaign.id,
            session_id: campaign.last_session_id,
            expected_revision: current.campaign.revision,
            causation_id: "room-after-recovery".into(),
        })
        .await
        .unwrap();
    // Even an old exact receipt must not let the local actor bypass room authority.
    assert_eq!(
        client
            .post(&url)
            .json(&body)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        409
    );
    assert_eq!(
        client.get(&url).send().await.unwrap().status().as_u16(),
        409
    );
}

#[tokio::test]
async fn solo_death_save_http_records_a_roll_without_renderer_dice() {
    use app_application::campaigns::{GAME_SCHEMA_VERSION, RULESET_ID};
    let server = TestServer::start().await;
    let store = Arc::new(SqliteStore::new(server.state.db().clone()));
    let campaign = campaign(store.clone(), "Solo death save").await;
    let id = hero(&store, &campaign).await;
    let mut state = store.load_current(campaign.id).await.unwrap();
    let mut actor = app_application::recovery::actor(&state.projection.character, id).unwrap();
    actor.current_hp = 0;
    app_application::vitality::store_actor(&mut state.projection.character, &actor).unwrap();
    sqlx::query("UPDATE campaign_characters SET character_json=?1 WHERE campaign_id=?2")
        .bind(state.projection.character.to_string())
        .bind(campaign.id.0.to_string())
        .execute(store.pool())
        .await
        .unwrap();
    let client = reqwest::Client::new();
    let url = server.url(&format!("/campaigns/{}/recovery", campaign.id.0));
    let before_status = client
        .get(&url)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let before_rest = client
        .get(server.url(&format!("/campaigns/{}/rest", campaign.id.0)))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let before_state = client
        .get(server.url(&format!("/campaigns/{}/state", campaign.id.0)))
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    let body = json!({"schema_version":GAME_SCHEMA_VERSION,"ruleset_id":RULESET_ID,"session_id":campaign.last_session_id.0,"character_id":id,"expected_revision":state.campaign.revision.0,"causation_id":"solo-death-save-ui","intent":{"kind":"death_save"}});
    let response = client.post(&url).json(&body).send().await.unwrap();
    assert!(response.status().is_success());
    let result = response.json::<serde_json::Value>().await.unwrap();
    assert_eq!(
        result["receipt"]["death_saves"].as_array().unwrap().len(),
        1
    );
    assert_eq!(result["status"]["elapsed_seconds"], 6);
    let retry = client
        .post(&url)
        .json(&body)
        .send()
        .await
        .unwrap()
        .json::<serde_json::Value>()
        .await
        .unwrap();
    assert_eq!(
        retry["receipt"]["death_saves"],
        result["receipt"]["death_saves"]
    );
    if let Ok(path) = std::env::var("DMAI_SOLO_DEATH_SAVE_WIRE_FIXTURE") {
        let after_rest = client
            .get(server.url(&format!("/campaigns/{}/rest", campaign.id.0)))
            .send()
            .await
            .unwrap()
            .json::<serde_json::Value>()
            .await
            .unwrap();
        std::fs::write(path,serde_json::to_string_pretty(&json!({"before":{"state":before_state,"status":before_status,"rest":before_rest},"after":{"result":result,"rest":after_rest}})).unwrap()).unwrap();
    }
}

#[tokio::test]
async fn owned_armor_http_uses_canonical_defense_and_rejects_unavailable_equipment() {
    use app_application::campaigns::{GAME_SCHEMA_VERSION, RULESET_ID};
    let server = TestServer::start().await;
    let store = Arc::new(SqliteStore::new(server.state.db().clone()));
    let campaign = campaign(store.clone(), "Armor HTTP").await;
    let id = hero(&store, &campaign).await;
    let state = store.load_current(campaign.id).await.unwrap();
    let mut sheet = state.projection.character;
    sheet["inventory"] = json!([{"id":"chain-mail","name":"Chain Mail","count":1}]);
    sheet["defense"] = json!({"schema_version":1,"armor_id":"chain-mail","shield":false});
    sheet["ac"] = json!(1);
    let body = json!({"schema_version":GAME_SCHEMA_VERSION,"ruleset_id":RULESET_ID,"session_id":campaign.last_session_id.0,"character_id":id,"expected_revision":state.campaign.revision.0,"causation_id":"equip-chain","character":sheet});
    let url = server.url(&format!("/campaigns/{}/character", campaign.id.0));
    let client = reqwest::Client::new();
    let response = client.put(&url).json(&body).send().await.unwrap();
    assert!(response.status().is_success());
    let result: serde_json::Value = response.json().await.unwrap();
    assert_eq!(result["projection"]["character"]["ac"], 16);
    let mut invalid = body;
    invalid["expected_revision"] = result["campaign"]["revision"].clone();
    invalid["causation_id"] = json!("equip-unowned");
    invalid["character"]["defense"]["armor_id"] = json!("plate-armor");
    assert_eq!(
        client
            .put(&url)
            .json(&invalid)
            .send()
            .await
            .unwrap()
            .status()
            .as_u16(),
        422
    );
    let after = store.load_current(campaign.id).await.unwrap();
    assert_eq!(
        after.campaign.revision.0,
        result["campaign"]["revision"].as_u64().unwrap()
    );
    assert_eq!(after.projection.character["ac"], 16);
}
