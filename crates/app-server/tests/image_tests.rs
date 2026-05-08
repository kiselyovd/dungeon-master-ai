use app_server::image::cache::image_cache_key;
use app_server::image::provider::ImagePrompt;

#[test]
fn cache_key_is_deterministic() {
    let prompt = ImagePrompt {
        content_prompt: "Dark tavern".into(),
        style_preset: "dark_fantasy".into(),
        scene_id: Some("scene_1".into()),
        npc_ids: vec!["npc_a".into(), "npc_b".into()],
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
    };
    let prompt2 = ImagePrompt {
        content_prompt: "Dark tavern".into(),
        style_preset: "dark_fantasy".into(),
        scene_id: Some("scene_1".into()),
        npc_ids: vec!["npc_b".into()],
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
    };
    let prompt_ba = ImagePrompt {
        content_prompt: "Tavern".into(),
        style_preset: "dark_fantasy".into(),
        scene_id: Some("s1".into()),
        npc_ids: vec!["npc_b".into(), "npc_a".into()],
    };
    assert_eq!(image_cache_key(&prompt_ab), image_cache_key(&prompt_ba));
}

#[test]
fn local_sdxl_stub_cost_is_zero() {
    use app_server::image::provider::ImageProvider;
    use app_server::image::stub::LocalSdxlSidecarProvider;
    let stub = LocalSdxlSidecarProvider;
    assert_eq!(stub.cost_per_image(), 0.0);
    assert_eq!(stub.estimated_seconds(), 8);
}
