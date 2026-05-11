//! Integration tests for the read-only /srd/* endpoints.
//!
//! Boots a real `TestServer` (mock LLM, in-memory sqlite), hits each path,
//! and asserts the SRD-cardinality invariants we trust about the data.

use app_server::test_support::TestServer;

async fn fetch_json(server: &TestServer, path: &str) -> serde_json::Value {
    let resp = reqwest::get(server.url(path)).await.expect("request");
    assert_eq!(resp.status(), 200, "expected 200 for {path}");
    resp.json().await.expect("json")
}

#[tokio::test]
async fn races_endpoint_returns_at_least_nine_entries() {
    let server = TestServer::start().await;
    let body = fetch_json(&server, "/srd/races").await;
    let arr = body.as_array().expect("array");
    assert!(arr.len() >= 9, "expected >=9 races, got {}", arr.len());
    assert!(arr.iter().any(|r| r["id"] == "dwarf"), "dwarf missing");
}

#[tokio::test]
async fn classes_endpoint_returns_twelve_classes() {
    let server = TestServer::start().await;
    let body = fetch_json(&server, "/srd/classes").await;
    let arr = body.as_array().expect("array");
    assert!(arr.len() >= 12, "expected >=12 classes, got {}", arr.len());
}

#[tokio::test]
async fn backgrounds_endpoint_returns_non_empty_array() {
    let server = TestServer::start().await;
    let body = fetch_json(&server, "/srd/backgrounds").await;
    assert!(!body.as_array().expect("array").is_empty());
}

#[tokio::test]
async fn spells_endpoint_covers_levels_zero_through_two() {
    let server = TestServer::start().await;
    let body = fetch_json(&server, "/srd/spells").await;
    let arr = body.as_array().expect("array");
    assert!(arr.len() >= 120, "expected >=120 spells, got {}", arr.len());
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
        obj["weapons"].as_array().expect("weapons").len() >= 30,
        "weapons low"
    );
    assert!(!obj["armor"].as_array().expect("armor").is_empty());
    assert!(
        !obj["adventuring_gear"]
            .as_array()
            .expect("adventuring_gear")
            .is_empty()
    );
}

#[tokio::test]
async fn feats_endpoint_returns_grappler_only() {
    let server = TestServer::start().await;
    let body = fetch_json(&server, "/srd/feats").await;
    let arr = body.as_array().expect("array");
    assert_eq!(arr.len(), 1, "SRD 5.1 has only Grappler");
    assert_eq!(arr[0]["id"].as_str(), Some("grappler"));
}

#[tokio::test]
async fn weapon_properties_endpoint_returns_at_least_ten() {
    let server = TestServer::start().await;
    let body = fetch_json(&server, "/srd/weapon-properties").await;
    let arr = body.as_array().expect("array");
    assert!(arr.len() >= 10, "expected >=10 properties, got {}", arr.len());
}
