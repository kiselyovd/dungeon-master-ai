//! Deterministic zero-runtime image provider backed by compiled WebP assets.

use std::sync::{Arc, Once};

use async_trait::async_trait;
use tracing::{debug, info};

use super::provider::{ImageBytes, ImageError, ImagePrompt, ImageProvider, ImageSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundledAssetCategory {
    Map,
    Illustration,
    Portrait,
}

impl BundledAssetCategory {
    fn as_str(self) -> &'static str {
        match self {
            Self::Map => "map",
            Self::Illustration => "illustration",
            Self::Portrait => "portrait",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BundledAsset {
    /// Explicit gallery identities are addressable by ID but never reshuffle
    /// the previously stable generic generation fallback pool.
    pub gallery_only: bool,
    pub id: &'static str,
    pub category: BundledAssetCategory,
    pub keywords: &'static [&'static str],
    pub mime_type: &'static str,
    pub bytes: &'static [u8],
    pub width: u32,
    pub height: u32,
}

macro_rules! asset {
    ($id:literal, $category:expr, [$($keyword:literal),+ $(,)?], $path:literal) => {
        BundledAsset {
            gallery_only: false,
            id: $id,
            category: $category,
            keywords: &[$($keyword),+],
            mime_type: "image/webp",
            bytes: include_bytes!($path),
            width: 1280,
            height: 720,
        }
    };
}

macro_rules! gallery_portrait {
    ($id:literal, $path:literal) => {
        BundledAsset {
            gallery_only: true,
            id: $id,
            category: BundledAssetCategory::Portrait,
            keywords: &[],
            mime_type: "image/webp",
            bytes: include_bytes!($path),
            width: 512,
            height: 512,
        }
    };
}

static ASSETS: &[BundledAsset] = &[
    BundledAsset {
        id: "portrait-fighter-legacy-v1",
        gallery_only: false,
        category: BundledAssetCategory::Portrait,
        keywords: &["fighter", "warrior", "paladin", "воин", "паладин"],
        mime_type: "image/webp",
        bytes: include_bytes!("../../assets/bundled/portraits/portrait-fighter-legacy.webp"),
        width: 1024,
        height: 1024,
    },
    BundledAsset {
        id: "portrait-wizard-legacy-v1",
        gallery_only: false,
        category: BundledAssetCategory::Portrait,
        keywords: &["wizard", "mage", "волшебник", "маг"],
        mime_type: "image/webp",
        bytes: include_bytes!("../../assets/bundled/portraits/portrait-wizard-legacy.webp"),
        width: 1024,
        height: 1024,
    },
    BundledAsset {
        id: "portrait-rogue-legacy-v1",
        gallery_only: false,
        category: BundledAssetCategory::Portrait,
        keywords: &["rogue", "thief", "плут", "вор"],
        mime_type: "image/webp",
        bytes: include_bytes!("../../assets/bundled/portraits/portrait-rogue-legacy.webp"),
        width: 1024,
        height: 1024,
    },
    BundledAsset {
        id: "portrait-cleric-legacy-v1",
        gallery_only: false,
        category: BundledAssetCategory::Portrait,
        keywords: &["cleric", "priest", "жрец"],
        mime_type: "image/webp",
        bytes: include_bytes!("../../assets/bundled/portraits/portrait-cleric-legacy.webp"),
        width: 1024,
        height: 1024,
    },
    BundledAsset {
        id: "portrait-fighter-01-v2",
        gallery_only: false,
        category: BundledAssetCategory::Portrait,
        keywords: &["fighter", "warrior", "воин"],
        mime_type: "image/webp",
        bytes: include_bytes!("../../assets/bundled/portraits/portrait-fighter-01-v2.webp"),
        width: 512,
        height: 512,
    },
    BundledAsset {
        id: "portrait-fighter-02-v2",
        gallery_only: true,
        category: BundledAssetCategory::Portrait,
        keywords: &[],
        mime_type: "image/webp",
        bytes: include_bytes!("../../assets/bundled/portraits/portrait-fighter-02-v2.webp"),
        width: 512,
        height: 512,
    },
    BundledAsset {
        id: "portrait-fighter-03-v2",
        gallery_only: true,
        category: BundledAssetCategory::Portrait,
        keywords: &[],
        mime_type: "image/webp",
        bytes: include_bytes!("../../assets/bundled/portraits/portrait-fighter-03-v2.webp"),
        width: 512,
        height: 512,
    },
    BundledAsset {
        id: "portrait-fighter-04-v2",
        gallery_only: true,
        category: BundledAssetCategory::Portrait,
        keywords: &[],
        mime_type: "image/webp",
        bytes: include_bytes!("../../assets/bundled/portraits/portrait-fighter-04-v2.webp"),
        width: 512,
        height: 512,
    },
    gallery_portrait!(
        "portrait-rogue-01-v2",
        "../../assets/bundled/portraits/portrait-rogue-01-v2.webp"
    ),
    gallery_portrait!(
        "portrait-rogue-02-v2",
        "../../assets/bundled/portraits/portrait-rogue-02-v2.webp"
    ),
    gallery_portrait!(
        "portrait-rogue-03-v2",
        "../../assets/bundled/portraits/portrait-rogue-03-v2.webp"
    ),
    gallery_portrait!(
        "portrait-rogue-04-v2",
        "../../assets/bundled/portraits/portrait-rogue-04-v2.webp"
    ),
    gallery_portrait!(
        "portrait-wizard-01-v2",
        "../../assets/bundled/portraits/portrait-wizard-01-v2.webp"
    ),
    gallery_portrait!(
        "portrait-wizard-02-v2",
        "../../assets/bundled/portraits/portrait-wizard-02-v2.webp"
    ),
    gallery_portrait!(
        "portrait-wizard-03-v2",
        "../../assets/bundled/portraits/portrait-wizard-03-v2.webp"
    ),
    gallery_portrait!(
        "portrait-wizard-04-v2",
        "../../assets/bundled/portraits/portrait-wizard-04-v2.webp"
    ),
    gallery_portrait!(
        "portrait-cleric-01-v2",
        "../../assets/bundled/portraits/portrait-cleric-01-v2.webp"
    ),
    gallery_portrait!(
        "portrait-cleric-02-v2",
        "../../assets/bundled/portraits/portrait-cleric-02-v2.webp"
    ),
    gallery_portrait!(
        "portrait-cleric-03-v2",
        "../../assets/bundled/portraits/portrait-cleric-03-v2.webp"
    ),
    gallery_portrait!(
        "portrait-cleric-04-v2",
        "../../assets/bundled/portraits/portrait-cleric-04-v2.webp"
    ),
    asset!(
        "map-tavern-interior",
        BundledAssetCategory::Map,
        ["tavern", "inn", "bar", "common room"],
        "../../assets/bundled/maps/map-tavern-interior.webp"
    ),
    asset!(
        "map-forest-crossing",
        BundledAssetCategory::Map,
        ["forest", "crossing", "stream", "bridge", "road"],
        "../../assets/bundled/maps/map-forest-crossing.webp"
    ),
    asset!(
        "map-ruined-courtyard",
        BundledAssetCategory::Map,
        ["ruin", "ruined", "courtyard", "fountain"],
        "../../assets/bundled/maps/map-ruined-courtyard.webp"
    ),
    asset!(
        "map-cave-chamber",
        BundledAssetCategory::Map,
        ["cave", "cavern", "underground", "pool"],
        "../../assets/bundled/maps/map-cave-chamber.webp"
    ),
    asset!(
        "map-dungeon-hall",
        BundledAssetCategory::Map,
        ["dungeon", "hall", "crypt", "prison"],
        "../../assets/bundled/maps/map-dungeon-hall.webp"
    ),
    asset!(
        "map-temple-floor",
        BundledAssetCategory::Map,
        ["temple", "shrine", "chapel", "sanctuary"],
        "../../assets/bundled/maps/map-temple-floor.webp"
    ),
    asset!(
        "map-village-square",
        BundledAssetCategory::Map,
        ["village", "town", "square", "market", "well"],
        "../../assets/bundled/maps/map-village-square.webp"
    ),
    asset!(
        "map-tower-chamber",
        BundledAssetCategory::Map,
        ["tower", "wizard", "mage", "library"],
        "../../assets/bundled/maps/map-tower-chamber.webp"
    ),
    asset!(
        "illustration-tavern",
        BundledAssetCategory::Illustration,
        ["tavern", "inn", "bar", "meeting"],
        "../../assets/bundled/illustrations/illustration-tavern.webp"
    ),
    asset!(
        "illustration-village",
        BundledAssetCategory::Illustration,
        ["village", "town", "valley", "settlement"],
        "../../assets/bundled/illustrations/illustration-village.webp"
    ),
    asset!(
        "illustration-forest-road",
        BundledAssetCategory::Illustration,
        ["forest", "road", "journey", "travel"],
        "../../assets/bundled/illustrations/illustration-forest-road.webp"
    ),
    asset!(
        "illustration-ruins",
        BundledAssetCategory::Illustration,
        ["ruin", "ruined", "battle", "fortress"],
        "../../assets/bundled/illustrations/illustration-ruins.webp"
    ),
    asset!(
        "illustration-cave",
        BundledAssetCategory::Illustration,
        ["cave", "cavern", "underground"],
        "../../assets/bundled/illustrations/illustration-cave.webp"
    ),
    asset!(
        "illustration-dungeon",
        BundledAssetCategory::Illustration,
        ["dungeon", "crypt", "corridor"],
        "../../assets/bundled/illustrations/illustration-dungeon.webp"
    ),
    asset!(
        "illustration-temple",
        BundledAssetCategory::Illustration,
        ["temple", "priest", "chapel", "letter"],
        "../../assets/bundled/illustrations/illustration-temple.webp"
    ),
    asset!(
        "illustration-wizard-tower",
        BundledAssetCategory::Illustration,
        ["wizard", "mage", "tower", "magic"],
        "../../assets/bundled/illustrations/illustration-wizard-tower.webp"
    ),
];

pub fn bundled_assets() -> &'static [BundledAsset] {
    ASSETS
}

#[derive(Debug, Default)]
pub struct BundledImageProvider;

impl BundledImageProvider {
    pub fn new() -> Self {
        static READY_LOG: Once = Once::new();
        READY_LOG.call_once(|| info!(asset_count = ASSETS.len(), "bundled image catalog ready"));
        Self
    }
}

pub fn bundled_image_provider() -> Arc<dyn ImageProvider> {
    Arc::new(BundledImageProvider::new())
}

#[async_trait]
impl ImageProvider for BundledImageProvider {
    async fn generate(&self, prompt: ImagePrompt) -> Result<ImageBytes, ImageError> {
        let category = category_for_style(&prompt.style_preset);
        let normalized_prompt = normalize(&prompt.content_prompt);
        let input_hash = fnv1a64(&format!(
            "{}\0{}\0{}",
            category.as_str(),
            normalize(&prompt.style_preset),
            normalized_prompt
        ));
        let asset = select_asset(category, &normalized_prompt, input_hash).ok_or({
            ImageError::Degraded {
                code: "bundled_catalog_empty",
            }
        })?;

        debug!(
            category = category.as_str(),
            asset_id = asset.id,
            input_hash,
            width = asset.width,
            height = asset.height,
            byte_count = asset.bytes.len(),
            "selected bundled image"
        );
        Ok(ImageBytes {
            data: asset.bytes.to_vec(),
            mime_type: asset.mime_type.to_string(),
            source: ImageSource::Bundled {
                asset_id: asset.id.to_string(),
            },
        })
    }

    fn estimated_seconds(&self) -> u32 {
        0
    }

    fn cost_per_image(&self) -> f32 {
        0.0
    }
}

fn select_asset(
    category: BundledAssetCategory,
    normalized_prompt: &str,
    input_hash: u64,
) -> Option<&'static BundledAsset> {
    let mut best_score = 0usize;
    let mut candidates = Vec::new();
    for asset in ASSETS
        .iter()
        .filter(|asset| asset.category == category && !asset.gallery_only)
    {
        let score = asset
            .keywords
            .iter()
            .filter(|keyword| normalized_prompt.contains(*keyword))
            .count();
        if score > best_score {
            best_score = score;
            candidates.clear();
        }
        if score == best_score {
            candidates.push(asset);
        }
    }
    if candidates.is_empty() {
        return None;
    }
    candidates
        .get((input_hash as usize) % candidates.len())
        .copied()
}

pub(crate) fn category_for_style(style: &str) -> BundledAssetCategory {
    if style.eq_ignore_ascii_case("map") {
        BundledAssetCategory::Map
    } else if style.eq_ignore_ascii_case("character_portrait")
        || style.eq_ignore_ascii_case("portrait")
    {
        BundledAssetCategory::Portrait
    } else {
        BundledAssetCategory::Illustration
    }
}

fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

fn fnv1a64(value: &str) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_gallery_choices_do_not_change_existing_unknown_portrait_fallbacks() {
        let original = [
            "portrait-fighter-legacy-v1",
            "portrait-wizard-legacy-v1",
            "portrait-rogue-legacy-v1",
            "portrait-cleric-legacy-v1",
            "portrait-fighter-01-v2",
        ];
        for seed in 0..32 {
            assert_eq!(
                select_asset(BundledAssetCategory::Portrait, "", seed)
                    .unwrap()
                    .id,
                original[seed as usize % original.len()]
            );
        }
        for id in [
            "portrait-rogue-01-v2",
            "portrait-rogue-02-v2",
            "portrait-rogue-03-v2",
            "portrait-rogue-04-v2",
        ] {
            let asset = ASSETS.iter().find(|asset| asset.id == id).unwrap();
            assert!(asset.gallery_only);
            assert_eq!((asset.width, asset.height), (512, 512));
        }
    }
}
