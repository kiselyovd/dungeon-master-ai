//! Integration tests for the read-only /srd/* endpoints.
//!
//! Boots a real `TestServer` (mock LLM, in-memory sqlite), hits each path,
//! and asserts the supported SRD 5.2.1 catalog and identity invariants.

use app_server::test_support::TestServer;

async fn fetch_json(server: &TestServer, path: &str) -> serde_json::Value {
    let resp = reqwest::get(server.url(path)).await.expect("request");
    assert_eq!(resp.status(), 200, "expected 200 for {path}");
    resp.json().await.expect("json")
}

fn assert_current_ruleset(entries: &[serde_json::Value]) {
    for entry in entries {
        assert_eq!(entry["ruleset_id"], "srd-5.2.1");
        assert!(entry["name_key"]
            .as_str()
            .is_some_and(|key| key.starts_with("catalog.")));
    }
}

#[tokio::test]
async fn races_endpoint_returns_the_nine_supported_species() {
    let server = TestServer::start().await;
    let body = fetch_json(&server, "/srd/races").await;
    let arr = body.as_array().expect("array");
    assert_eq!(arr.len(), 9);
    assert_current_ruleset(arr);
    assert!(arr.iter().any(|r| r["id"] == "dwarf"), "dwarf missing");
}

#[tokio::test]
async fn classes_endpoint_returns_twelve_classes() {
    let server = TestServer::start().await;
    let body = fetch_json(&server, "/srd/classes").await;
    let arr = body.as_array().expect("array");
    assert_eq!(arr.len(), 12);
    assert_current_ruleset(arr);
}

#[tokio::test]
async fn backgrounds_endpoint_returns_the_four_srd_backgrounds() {
    let server = TestServer::start().await;
    let body = fetch_json(&server, "/srd/backgrounds").await;
    let arr = body.as_array().expect("array");
    assert_eq!(arr.len(), 4);
    assert_current_ruleset(arr);
}

#[tokio::test]
async fn spells_endpoint_covers_levels_zero_through_two() {
    let server = TestServer::start().await;
    let body = fetch_json(&server, "/srd/spells").await;
    let arr = body.as_array().expect("array");
    assert_eq!(arr.len(), 29);
    assert_current_ruleset(arr);
    assert!(arr.iter().any(|s| s["level"] == 0), "no cantrips");
    assert!(arr.iter().any(|s| s["level"] == 1), "no lvl 1 spells");
    assert!(arr.iter().any(|s| s["level"] == 2), "no lvl 2 spells");
}

#[tokio::test]
async fn equipment_endpoint_returns_three_buckets() {
    let server = TestServer::start().await;
    let body = fetch_json(&server, "/srd/equipment").await;
    let obj = body.as_object().expect("object");
    assert!(
        obj["weapons"].as_array().expect("weapons").len() == 16,
        "unexpected supported weapon count"
    );
    assert_eq!(obj["armor"].as_array().expect("armor").len(), 13);
    assert_eq!(
        obj["adventuring_gear"]
            .as_array()
            .expect("adventuring_gear")
            .len(),
        24
    );
    assert_current_ruleset(obj["weapons"].as_array().expect("weapons"));
    assert_current_ruleset(obj["armor"].as_array().expect("armor"));
    assert_current_ruleset(
        obj["adventuring_gear"]
            .as_array()
            .expect("adventuring_gear"),
    );
}

#[tokio::test]
async fn feats_endpoint_returns_the_supported_origin_feats() {
    let server = TestServer::start().await;
    let body = fetch_json(&server, "/srd/feats").await;
    let arr = body.as_array().expect("array");
    let ids = arr
        .iter()
        .filter_map(|entry| entry["id"].as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(
        ids,
        ["alert", "magic-initiate", "savage-attacker", "skilled"]
            .into_iter()
            .collect()
    );
    assert_current_ruleset(arr);
}

#[tokio::test]
async fn weapon_properties_endpoint_returns_at_least_ten() {
    let server = TestServer::start().await;
    let body = fetch_json(&server, "/srd/weapon-properties").await;
    let arr = body.as_array().expect("array");
    assert_eq!(arr.len(), 10);
    assert_current_ruleset(arr);
}
