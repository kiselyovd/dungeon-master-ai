//! Lazy, panic-on-failure loader for the normalized SRD 5.2.1 character catalog.
//!
//! The checked-in catalog contains identifiers and game mechanics required by
//! character creation. Long-form rules prose stays in the separately indexed
//! SRD retrieval corpus. Parse or identity failures are packaging defects and
//! therefore fail loudly during development.

use std::sync::OnceLock;

use serde::Deserialize;

use super::types::{
    Background, Class, EquipmentBundle, Feat, Race, RulesetProvenance, Spell, WeaponMastery,
    WeaponProperty,
};

pub const CURRENT_RULESET_ID: &str = "srd-5.2.1";

#[derive(Clone, Debug, Deserialize)]
pub struct CharacterCatalog {
    pub provenance: RulesetProvenance,
    #[serde(default)]
    pub races: Vec<Race>,
    #[serde(default)]
    pub classes: Vec<Class>,
    #[serde(default)]
    pub backgrounds: Vec<Background>,
    #[serde(default)]
    pub spells: Vec<Spell>,
    #[serde(default)]
    pub equipment: EquipmentBundle,
    #[serde(default)]
    pub feats: Vec<Feat>,
    #[serde(default)]
    pub weapon_properties: Vec<WeaponProperty>,
    #[serde(default)]
    pub weapon_masteries: Vec<WeaponMastery>,
}

/// Compatibility name retained while HTTP and wizard callers migrate.
pub type Compendium = CharacterCatalog;

const CATALOG_YAML: &str = include_str!("data/srd_5_2_1.yaml");

static CELL: OnceLock<CharacterCatalog> = OnceLock::new();

pub fn character_catalog() -> &'static CharacterCatalog {
    CELL.get_or_init(load)
}

pub fn compendium() -> &'static Compendium {
    character_catalog()
}

fn load() -> CharacterCatalog {
    let mut catalog: CharacterCatalog =
        serde_yaml::from_str(CATALOG_YAML).expect("srd_5_2_1.yaml must parse");
    assert_eq!(
        catalog.provenance.ruleset_id, CURRENT_RULESET_ID,
        "character catalog ruleset identity must match the application ruleset"
    );

    let ruleset = catalog.provenance.ruleset_id.clone();
    for entry in &mut catalog.races {
        entry.ruleset_id.clone_from(&ruleset);
    }
    for entry in &mut catalog.classes {
        entry.ruleset_id.clone_from(&ruleset);
    }
    for entry in &mut catalog.backgrounds {
        entry.ruleset_id.clone_from(&ruleset);
    }
    for entry in &mut catalog.spells {
        entry.ruleset_id.clone_from(&ruleset);
    }
    for entry in &mut catalog.equipment.weapons {
        entry.ruleset_id.clone_from(&ruleset);
    }
    for entry in &mut catalog.equipment.armor {
        entry.ruleset_id.clone_from(&ruleset);
    }
    for entry in &mut catalog.equipment.adventuring_gear {
        entry.ruleset_id.clone_from(&ruleset);
    }
    for entry in &mut catalog.feats {
        entry.ruleset_id.clone_from(&ruleset);
    }
    for entry in &mut catalog.weapon_properties {
        entry.ruleset_id.clone_from(&ruleset);
    }
    for entry in &mut catalog.weapon_masteries {
        entry.ruleset_id.clone_from(&ruleset);
    }

    catalog
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_loads_with_the_supported_srd_521_surface() {
        let catalog = character_catalog();
        assert_eq!(catalog.provenance.ruleset_id, CURRENT_RULESET_ID);
        assert_eq!(catalog.races.len(), 9);
        assert_eq!(catalog.classes.len(), 12);
        assert_eq!(catalog.backgrounds.len(), 4);
        assert!(catalog
            .spells
            .iter()
            .any(|spell| spell.id == "magic-missile"));
        assert!(catalog
            .equipment
            .weapons
            .iter()
            .any(|weapon| weapon.mastery.as_deref() == Some("vex")));
        assert_eq!(catalog.weapon_masteries.len(), 8);
    }
}
