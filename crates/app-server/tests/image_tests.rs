use app_server::image::cache::image_cache_key;
use app_server::image::provider::ImagePrompt;
use app_server::test_support::TestServer;

#[test]
fn cache_key_is_deterministic() {
    let prompt = ImagePrompt {
        content_prompt: "Dark tavern".into(),
        style_preset: "dark_fantasy".into(),
        scene_id: Some("scene_1".into()),
        npc_ids: vec!["npc_a".into(), "npc_b".into()],
        backend_preset: None,
        width: None,
        height: None,
    };
    let key1 = image_cache_key(&prompt);
    let key2 = image_cache_key(&prompt);
    assert_eq!(key1, key2);
}

#[test]
fn cache_key_differs_for_different_npcs() {
    let prompt1 = ImagePrompt {
        content_prompt: "Dark tavern".into(),
        style_preset: "dark_fantasy".into(),
        scene_id: Some("scene_1".into()),
        npc_ids: vec!["npc_a".into()],
        backend_preset: None,
        width: None,
        height: None,
    };
    let prompt2 = ImagePrompt {
        content_prompt: "Dark tavern".into(),
        style_preset: "dark_fantasy".into(),
        scene_id: Some("scene_1".into()),
        npc_ids: vec!["npc_b".into()],
        backend_preset: None,
        width: None,
        height: None,
    };
    assert_ne!(image_cache_key(&prompt1), image_cache_key(&prompt2));
}

#[test]
fn cache_key_npc_order_is_sorted() {
    let prompt_ab = ImagePrompt {
        content_prompt: "Tavern".into(),
        style_preset: "dark_fantasy".into(),
        scene_id: Some("s1".into()),
        npc_ids: vec!["npc_a".into(), "npc_b".into()],
        backend_preset: None,
        width: None,
        height: None,
    };
    let prompt_ba = ImagePrompt {
        content_prompt: "Tavern".into(),
        style_preset: "dark_fantasy".into(),
        scene_id: Some("s1".into()),
        npc_ids: vec!["npc_b".into(), "npc_a".into()],
        backend_preset: None,
        width: None,
        height: None,
    };
    assert_eq!(image_cache_key(&prompt_ab), image_cache_key(&prompt_ba));
}

#[test]
fn local_image_sidecar_cost_is_zero() {
    use app_server::image::provider::ImageProvider;
    use app_server::image::stub::LocalImageSidecarProvider;
    let stub = LocalImageSidecarProvider::new("http://127.0.0.1:0");
    assert_eq!(stub.cost_per_image(), 0.0);
    assert_eq!(stub.estimated_seconds(), 8);
}

#[tokio::test]
async fn fresh_server_generates_separate_bundled_map_and_illustration_without_media_sidecar() {
    let server = TestServer::start().await;
    assert!(server.state.media_sidecar_url().is_none());

    let client = reqwest::Client::new();
    let map_response = client
        .post(server.url("/image/generate"))
        .json(&serde_json::json!({
            "prompt": "forest crossing",
            "style_preset": "map"
        }))
        .send()
        .await
        .expect("image request");

    assert_eq!(map_response.status(), 200);
    let map: serde_json::Value = map_response.json().await.expect("map response");
    let illustration_response = client
        .post(server.url("/image/generate"))
        .json(&serde_json::json!({
            "prompt": "forest crossing",
            "style_preset": "classic"
        }))
        .send()
        .await
        .expect("illustration request");
    assert_eq!(illustration_response.status(), 200);
    let illustration: serde_json::Value = illustration_response
        .json()
        .await
        .expect("illustration response");

    assert_eq!(map["source"], "bundled");
    assert_eq!(illustration["source"], "bundled");
    let map_id = map["asset_id"].as_str().expect("map asset id");
    let illustration_id = illustration["asset_id"]
        .as_str()
        .expect("illustration asset id");
    assert!(map_id.starts_with("map-"));
    assert!(illustration_id.starts_with("illustration-"));
    assert_ne!(map_id, illustration_id);
    for body in [&map, &illustration] {
        assert!(body["url"]
            .as_str()
            .is_some_and(|url| url.starts_with("data:image/webp;base64,")));
    }
}
