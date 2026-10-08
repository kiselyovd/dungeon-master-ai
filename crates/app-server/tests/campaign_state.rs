use app_server::test_support::TestServer;
use serde_json::{json, Value};
use uuid::Uuid;

#[tokio::test]
async fn campaign_state_persists_a_character_and_reports_conflicts() {
    let server = TestServer::start().await;
    let client = reqwest::Client::new();
    let campaign: Value = client.post(server.url("/campaigns")).json(&json!({
        "schema_version": 1, "ruleset_id": "srd-5.2.1", "title": "The Bell", "premise": "A missing ferryman",
        "seed": {"kind": "collaborative", "answers": {"genre": ["fantasy"]}}
    })).send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
    let id = campaign["id"].as_str().unwrap();
    let state_url = server.url(&format!("/campaigns/{id}/state"));
    let empty_response = client.get(&state_url).send().await.unwrap();
    assert_eq!(empty_response.status(), 200);
    let empty: Value = empty_response.json().await.unwrap();
    assert!(empty["character_id"].is_null());
    let request = json!({
        "schema_version": 1, "ruleset_id": "srd-5.2.1", "session_id": campaign["last_session_id"],
        "character_id": Uuid::new_v4(), "expected_revision": campaign["revision"], "causation_id": "confirm-hero",
        "character": {"heroClass": "fighter", "name": "Дарен", "level": 1, "hp": 12, "hpMax": 12, "ac": 14,
            "abilities": {"str": 15, "dex": 14, "con": 13, "int": 12, "wis": 10, "cha": 8},
            "inventory": [], "backstory": "An old promise"}
    });
    let character_url = server.url(&format!("/campaigns/{id}/character"));
    let committed: Value = client
        .put(&character_url)
        .json(&request)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(committed["campaign"]["revision"], 2);
    assert_eq!(committed["projection"]["character"]["name"], "Дарен");
    let retried: Value = client
        .put(&character_url)
        .json(&request)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(retried, committed);
    let current: Value = client
        .get(&state_url)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(current, committed);
    let mut portrait = request.clone();
    portrait["expected_revision"] = json!(2);
    portrait["causation_id"] = json!("portrait-confirmation");
    portrait["character"]["portraitUrl"] =
        json!(format!("data:image/png;base64,{}", "A".repeat(400 * 1024)));
    let portrait_response = client
        .put(&character_url)
        .json(&portrait)
        .send()
        .await
        .unwrap();
    assert_eq!(
        portrait_response.status(),
        200,
        "ordinary generated portrait must not block confirmation"
    );
    let portrait_saved: Value = portrait_response.json().await.unwrap();
    assert_eq!(
        portrait_saved["projection"]["character"]["portraitUrl"],
        portrait["character"]["portraitUrl"]
    );
    let mut stale = request.clone();
    stale["causation_id"] = json!("another-commit");
    assert_eq!(
        client
            .put(&character_url)
            .json(&stale)
            .send()
            .await
            .unwrap()
            .status(),
        409
    );
    let mut invalid = request;
    invalid["ruleset_id"] = json!("unknown");
    assert_eq!(
        client
            .put(&character_url)
            .json(&invalid)
            .send()
            .await
            .unwrap()
            .status(),
        422
    );
}

#[tokio::test]
async fn untrained_owned_defense_commits_and_reloads_with_canonical_ac() {
    let server = TestServer::start().await;
    let client = reqwest::Client::new();
    let campaign:Value=client.post(server.url("/campaigns")).json(&json!({"schema_version":1,"ruleset_id":"srd-5.2.1","title":"Armor","premise":"Training","seed":{"kind":"collaborative","answers":{"genre":["fantasy"]}}})).send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
    let character_id = Uuid::new_v4();
    let character = json!({"heroClass":"wizard","name":"Mage","level":1,"hp":8,"hpMax":8,"ac":1,"abilities":{"str":12,"dex":14,"con":14,"int":16,"wis":12,"cha":10},"inventory":[{"id":"chain-mail","name":"Chain Mail","count":1},{"id":"shield","name":"Shield","count":1}],"defense":{"schema_version":1,"armor_id":"chain-mail","shield":true}});
    let payload = json!({"schema_version":1,"ruleset_id":"srd-5.2.1","session_id":campaign["last_session_id"],"character_id":character_id,"expected_revision":campaign["revision"],"causation_id":"untrained-defense","character":character});
    let url = server.url(&format!(
        "/campaigns/{}/character",
        campaign["id"].as_str().unwrap()
    ));
    let committed: Value = client
        .put(&url)
        .json(&payload)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(committed["projection"]["character"]["ac"], 16);
    assert_eq!(committed["projection"]["character"]["speedFt"], 20);
    let retried: Value = client
        .put(&url)
        .json(&payload)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(retried, committed);
    let loaded: Value = client
        .get(server.url(&format!(
            "/campaigns/{}/state",
            campaign["id"].as_str().unwrap()
        )))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(
        loaded["projection"]["character"],
        committed["projection"]["character"]
    );
}
