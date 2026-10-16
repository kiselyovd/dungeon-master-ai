use std::collections::HashSet;

use adapter_media::image::{
    bundled_assets, BundledAssetCategory, BundledImageProvider, ImagePrompt, ImageProvider,
    ImageSource,
};

fn prompt(content: &str, style: &str) -> ImagePrompt {
    ImagePrompt {
        content_prompt: content.into(),
        style_preset: style.into(),
        scene_id: None,
        npc_ids: Vec::new(),
        backend_preset: None,
        width: None,
        height: None,
    }
}

#[tokio::test]
async fn all_eight_map_identities_return_their_packaged_pixels_and_export_optional_wire_fixture() {
    use base64::Engine;
    let provider = BundledImageProvider::new();
    let mut wire = Vec::new();
    for (id, content) in [
        ("map-tavern-interior", "tavern interior"),
        ("map-forest-crossing", "forest crossing"),
        ("map-ruined-courtyard", "ruined courtyard"),
        ("map-cave-chamber", "cave chamber"),
        ("map-dungeon-hall", "dungeon hall"),
        ("map-temple-floor", "temple floor"),
        ("map-village-square", "village square"),
        ("map-tower-chamber", "tower chamber"),
    ] {
        let image = provider.generate(prompt(content, "map")).await.unwrap();
        assert_eq!(
            image.source,
            ImageSource::Bundled {
                asset_id: id.into()
            }
        );
        let asset = bundled_assets()
            .iter()
            .find(|asset| asset.id == id)
            .unwrap();
        assert_eq!(image.data, asset.bytes);
        assert_eq!((asset.width, asset.height), (1280, 720));
        assert!(image.data.len() <= 512 * 1024);
        wire.push(serde_json::json!({"id":id,"prompt":content,"image_b64":base64::engine::general_purpose::STANDARD.encode(&image.data)}));
    }
    if let Ok(path) = std::env::var("DMAI_EXPORT_BUNDLED_MAPS") {
        std::fs::write(path, serde_json::to_vec(&wire).unwrap()).unwrap();
    }
}

#[tokio::test]
async fn selects_the_same_asset_for_normalized_prompt_variants() {
    let provider = BundledImageProvider::new();
    let first = provider
        .generate(prompt("forest crossing", "map"))
        .await
        .expect("bundled map");
    let second = provider
        .generate(prompt("  FOREST\n crossing  ", "map"))
        .await
        .expect("normalized bundled map");

    assert_eq!(first, second);
    assert!(matches!(first.source, ImageSource::Bundled { .. }));
    assert_eq!(first.mime_type, "image/webp");
    assert!(!first.data.is_empty());
}

#[test]
fn catalog_is_unique_valid_and_has_complete_scene_coverage() {
    let assets = bundled_assets();
    let ids = assets.iter().map(|asset| asset.id).collect::<HashSet<_>>();
    assert_eq!(ids.len(), assets.len());
    assert!(
        assets
            .iter()
            .filter(|asset| asset.category == BundledAssetCategory::Map)
            .count()
            >= 8
    );
    assert!(
        assets
            .iter()
            .filter(|asset| asset.category == BundledAssetCategory::Illustration)
            .count()
            >= 8
    );

    for asset in assets {
        match asset.category {
            BundledAssetCategory::Map => assert!(asset.id.starts_with("map-")),
            BundledAssetCategory::Illustration => assert!(asset.id.starts_with("illustration-")),
            BundledAssetCategory::Portrait => assert!(asset.id.starts_with("portrait-")),
        }
        assert_eq!(&asset.bytes[..4], b"RIFF");
        assert_eq!(&asset.bytes[8..12], b"WEBP");
        if asset.category == BundledAssetCategory::Portrait {
            assert!(asset.width >= 512 && asset.width == asset.height);
        } else {
            assert!(asset.width >= 1024 && asset.height >= 576);
        }
        assert!(asset.bytes.len() <= 512 * 1024);
        if asset.gallery_only {
            assert_eq!(asset.category, BundledAssetCategory::Portrait);
            assert!(asset.keywords.is_empty());
        } else {
            assert!(!asset.keywords.is_empty());
        }
    }
}

#[tokio::test]
async fn character_portrait_never_selects_a_map_or_scene_even_with_scenic_keywords() {
    let provider = BundledImageProvider::new();
    for content in [
        "fighter in a tavern",
        "wizard in forest",
        "cleric in temple",
        "rogue near ruins",
    ] {
        let image = provider
            .generate(prompt(content, "character_portrait"))
            .await
            .unwrap();
        let ImageSource::Bundled { asset_id } = image.source else {
            panic!("bundled portrait expected")
        };
        let asset = bundled_assets()
            .iter()
            .find(|asset| asset.id == asset_id)
            .unwrap();
        assert_eq!(asset.category, BundledAssetCategory::Portrait);
        assert_eq!(asset.width, asset.height);
    }
}

#[tokio::test]
async fn map_and_illustration_prompts_never_cross_categories() {
    let provider = BundledImageProvider::new();
    let map = provider
        .generate(prompt("tavern", "map"))
        .await
        .expect("bundled map");
    let illustration = provider
        .generate(prompt("tavern", "classic"))
        .await
        .expect("bundled illustration");

    let ImageSource::Bundled { asset_id: map_id } = map.source else {
        panic!("map source must be bundled");
    };
    let ImageSource::Bundled {
        asset_id: illustration_id,
    } = illustration.source
    else {
        panic!("illustration source must be bundled");
    };
    assert!(map_id.starts_with("map-"));
    assert!(illustration_id.starts_with("illustration-"));
}
