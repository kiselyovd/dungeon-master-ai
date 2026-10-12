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
        assert!(!asset.keywords.is_empty());
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
