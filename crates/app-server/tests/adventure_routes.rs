use app_server::test_support::TestServer;
use serde_json::{json, Value};
use uuid::Uuid;

#[tokio::test]
async fn bundled_adventure_routes_commit_only_available_owned_actions() {
    let server = TestServer::start().await;
    let client = reqwest::Client::new();
    let campaign:Value=client.post(server.url("/campaigns")).json(&json!({
        "schema_version":1,"ruleset_id":"srd-5.2.1","title":"Колокол под водой","premise":"Тихон пропал",
        "seed":{"kind":"bundled","adventure_id":"bell-beneath-water"}
    })).send().await.unwrap().error_for_status().unwrap().json().await.unwrap();
    let id = campaign["id"].as_str().unwrap();
    let hero = Uuid::new_v4();
    let character_url = server.url(&format!("/campaigns/{id}/character"));
    client.put(character_url).json(&json!({"schema_version":1,"ruleset_id":"srd-5.2.1",
        "session_id":campaign["last_session_id"],"character_id":hero,"expected_revision":1,"causation_id":"hero",
        "character":{"name":"Мира","heroClass":"fighter","level":1,"hp":12,"hpMax":12,"ac":14,
        "abilities":{"str":15,"dex":14,"con":13,"int":12,"wis":10,"cha":8},"inventory":[]}
    })).send().await.unwrap().error_for_status().unwrap();
    let url = server.url(&format!("/campaigns/{id}/adventure/actions"));
    let request = json!({"schema_version":1,"ruleset_id":"srd-5.2.1","session_id":campaign["last_session_id"],
        "character_id":hero,"expected_revision":2,"causation_id":"first-step","action":{"kind":"move","location":"quay"}});
    let response = client.post(&url).json(&request).send().await.unwrap();
    assert_eq!(response.status(), 200);
    let state: Value = response.json().await.unwrap();
    assert_eq!(
        state["projection"]["adventure"]["state"]["location"],
        "quay"
    );
    assert_eq!(state["campaign"]["revision"], 3);
    assert!(!state.to_string().contains("cut the bell"));
    let retry: Value = client
        .post(&url)
        .json(&request)
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(retry, state);
    let mut invalid = request;
    invalid["causation_id"] = json!("illegal-action");
    invalid["expected_revision"] = json!(3);
    invalid["action"] = json!({"kind":"restore_ward"});
    assert_eq!(
        client
            .post(&url)
            .json(&invalid)
            .send()
            .await
            .unwrap()
            .status(),
        422
    );
    let current: Value = client
        .get(server.url(&format!("/campaigns/{id}/state")))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(current["campaign"]["revision"], 3);
    let unknown = client
        .post(server.url("/campaigns"))
        .json(&json!({"schema_version":1,"ruleset_id":"srd-5.2.1",
        "title":"Unknown","premise":"?","seed":{"kind":"bundled","adventure_id":"invented"}}))
        .send()
        .await
        .unwrap();
    assert_eq!(unknown.status(), 422);
}
