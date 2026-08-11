use axum::extract::Extension;
use axum::Json;
use serde_json::Value;

use crate::services::{HttpServiceError, HttpServices, SrdSection};

macro_rules! srd_handler {
    ($name:ident, $section:expr) => {
        pub async fn $name(
            Extension(services): Extension<HttpServices>,
        ) -> Result<Json<Value>, HttpServiceError> {
            services.campaign.srd($section).await.map(Json)
        }
    };
}

srd_handler!(get_races, SrdSection::Races);
srd_handler!(get_classes, SrdSection::Classes);
srd_handler!(get_backgrounds, SrdSection::Backgrounds);
srd_handler!(get_spells, SrdSection::Spells);
srd_handler!(get_equipment, SrdSection::Equipment);
srd_handler!(get_feats, SrdSection::Feats);
srd_handler!(get_weapon_properties, SrdSection::WeaponProperties);
