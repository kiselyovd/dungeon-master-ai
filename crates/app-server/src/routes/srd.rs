//! Read-only GET endpoints exposing the SRD 5.1 character-creation compendium.
//!
//! Each handler returns a JSON-serialised clone of the `&'static` slice from
//! `app_domain::compendium::compendium()`. The parse cost is paid once at the
//! first call site (anywhere in-process), then cached for every subsequent
//! request.

use app_domain::compendium::{
    compendium,
    types::{AdventuringGear, Armor, Background, Class, Feat, Race, Spell, Weapon, WeaponProperty},
};
use axum::Json;
use serde::Serialize;

pub async fn get_races() -> Json<Vec<Race>> {
    Json(compendium().races.clone())
}

pub async fn get_classes() -> Json<Vec<Class>> {
    Json(compendium().classes.clone())
}

pub async fn get_backgrounds() -> Json<Vec<Background>> {
    Json(compendium().backgrounds.clone())
}

pub async fn get_spells() -> Json<Vec<Spell>> {
    Json(compendium().spells.clone())
}

#[derive(Clone, Debug, Serialize)]
pub struct EquipmentResponse {
    pub weapons: Vec<Weapon>,
    pub armor: Vec<Armor>,
    pub adventuring_gear: Vec<AdventuringGear>,
}

pub async fn get_equipment() -> Json<EquipmentResponse> {
    let e = &compendium().equipment;
    Json(EquipmentResponse {
        weapons: e.weapons.clone(),
        armor: e.armor.clone(),
        adventuring_gear: e.adventuring_gear.clone(),
    })
}

pub async fn get_feats() -> Json<Vec<Feat>> {
    Json(compendium().feats.clone())
}

pub async fn get_weapon_properties() -> Json<Vec<WeaponProperty>> {
    Json(compendium().weapon_properties.clone())
}
