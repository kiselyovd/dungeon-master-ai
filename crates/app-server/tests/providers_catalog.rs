use app_server::test_support::TestServer;
use serde_json::Value;

#[tokio::test]
async fn get_providers_catalog_returns_chat_image_video_keys() {
    let server = TestServer::start().await;
    let resp = reqwest::get(server.url("/providers/catalog"))
        .await
        .expect("request");
    assert_eq!(resp.status(), 200);
    let body: Value = resp.json().await.expect("json");
    assert!(body["chat"].is_array());
    assert!(body["image"].is_array());
    assert!(body["video"].is_array());

    let chat_ids: Vec<&str> = body["chat"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect();
    // Native Anthropic was removed in M11 Batch D.5; cloud chat is openai-compat only.
    assert!(!chat_ids.contains(&"anthropic"));
    assert!(chat_ids.contains(&"openai-compat"));
    assert!(chat_ids.contains(&"local-mistralrs"));
}

#[tokio::test]
async fn catalog_lists_5_image_providers() {
    let server = TestServer::start().await;
    let body: Value = reqwest::get(server.url("/providers/catalog"))
        .await
        .expect("request")
        .json()
        .await
        .expect("json");
    let image_ids: Vec<&str> = body["image"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["id"].as_str().unwrap())
        .collect();
    assert!(image_ids.contains(&"local-sdxl-lightning"));
    assert!(image_ids.contains(&"replicate"));
    assert_eq!(body["image"].as_array().unwrap().len(), 5);
}

#[tokio::test]
async fn catalog_lists_only_ltx_video_in_m7_dm() {
    let server = TestServer::start().await;
    let body: Value = reqwest::get(server.url("/providers/catalog"))
        .await
        .expect("request")
        .json()
        .await
        .expect("json");
    assert_eq!(body["video"].as_array().unwrap().len(), 1);
    assert_eq!(body["video"][0]["id"], "local-ltx-video");
}

#[tokio::test]
async fn catalog_openai_compat_is_the_only_cloud_chat_entry() {
    let server = TestServer::start().await;
    let body: Value = reqwest::get(server.url("/providers/catalog"))
        .await
        .expect("request")
        .json()
        .await
        .expect("json");
    // Native Anthropic was removed in M11 Batch D.5.
    let openai_compat = body["chat"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["id"] == "openai-compat")
        .expect("openai-compat entry");
    assert_eq!(openai_compat["requires_base_url"], true);
    assert_eq!(openai_compat["requires_api_key"], true);
    assert!(body["chat"]
        .as_array()
        .unwrap()
        .iter()
        .all(|e| e["id"] != "anthropic"));
}
