//! Deterministic zero-runtime image provider backed by compiled WebP assets.

use std::sync::{Arc, Once};

use async_trait::async_trait;
use tracing::{debug, info};

use super::provider::{ImageBytes, ImageError, ImagePrompt, ImageProvider, ImageSource};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BundledAssetCategory {
    Map,
    Illustration,
}

impl BundledAssetCategory {
    fn as_str(self) -> &'static str {
        match self {
            Self::Map => "map",
            Self::Illustration => "illustration",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BundledAsset {
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

static ASSETS: &[BundledAsset] = &[
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
        let category = if prompt.style_preset.eq_ignore_ascii_case("map") {
            BundledAssetCategory::Map
        } else {
            BundledAssetCategory::Illustration
        };
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
    for asset in ASSETS.iter().filter(|asset| asset.category == category) {
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
    candidates
        .get((input_hash as usize) % candidates.len())
        .copied()
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
