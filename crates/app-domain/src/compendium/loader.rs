//! Lazy, panic-on-failure loader for the SRD 5.1 character-creation datasets.
//!
//! YAML files are embedded at compile time via `include_str!()`. The first
//! call to `compendium()` parses every file once and stores the aggregate in
//! a `OnceLock`. Parse errors are programmer bugs (we ship our own data), so
//! they panic loudly rather than threading a `Result` through every caller.

use std::sync::OnceLock;

use serde::Deserialize;

use super::types::{Background, Class, EquipmentBundle, Feat, Race, Spell, WeaponProperty};

#[derive(Clone, Debug)]
pub struct Compendium {
    pub races: Vec<Race>,
    pub classes: Vec<Class>,
    pub backgrounds: Vec<Background>,
    pub spells: Vec<Spell>,
    pub equipment: EquipmentBundle,
    pub feats: Vec<Feat>,
    pub weapon_properties: Vec<WeaponProperty>,
}

const RACES_CLASSES_BACKGROUNDS_YAML: &str =
    include_str!("data/races_classes_backgrounds.yaml");
const SPELLS_0_1_YAML: &str = include_str!("data/spells_level_0_1.yaml");
const SPELLS_2_AK_YAML: &str = include_str!("data/spells_level_2_a_k.yaml");
const SPELLS_2_LZ_YAML: &str = include_str!("data/spells_level_2_l_z.yaml");
const EQUIPMENT_YAML: &str = include_str!("data/equipment.yaml");
const FEATS_YAML: &str = include_str!("data/feats.yaml");
const WEAPON_PROPERTIES_YAML: &str = include_str!("data/weapon_properties.yaml");

#[derive(Deserialize)]
struct RcbDoc {
    races: Vec<Race>,
    classes: Vec<Class>,
    backgrounds: Vec<Background>,
}

#[derive(Deserialize)]
struct SpellsDoc {
    spells: Vec<Spell>,
}

#[derive(Deserialize)]
struct FeatsDoc {
    feats: Vec<Feat>,
}

#[derive(Deserialize)]
struct WeaponPropertiesDoc {
    weapon_properties: Vec<WeaponProperty>,
}

static CELL: OnceLock<Compendium> = OnceLock::new();

pub fn compendium() -> &'static Compendium {
    CELL.get_or_init(load)
}

fn load() -> Compendium {
    let rcb: RcbDoc = serde_yaml::from_str(RACES_CLASSES_BACKGROUNDS_YAML)
        .expect("races_classes_backgrounds.yaml must parse");
    let spells_0_1: SpellsDoc =
        serde_yaml::from_str(SPELLS_0_1_YAML).expect("spells_level_0_1.yaml must parse");
    let spells_2_ak: SpellsDoc =
        serde_yaml::from_str(SPELLS_2_AK_YAML).expect("spells_level_2_a_k.yaml must parse");
    let spells_2_lz: SpellsDoc =
        serde_yaml::from_str(SPELLS_2_LZ_YAML).expect("spells_level_2_l_z.yaml must parse");
    let equipment: EquipmentBundle =
        serde_yaml::from_str(EQUIPMENT_YAML).expect("equipment.yaml must parse");
    let feats: FeatsDoc =
        serde_yaml::from_str(FEATS_YAML).expect("feats.yaml must parse");
    let weapon_properties: WeaponPropertiesDoc =
        serde_yaml::from_str(WEAPON_PROPERTIES_YAML).expect("weapon_properties.yaml must parse");

    let mut spells = Vec::with_capacity(
        spells_0_1.spells.len() + spells_2_ak.spells.len() + spells_2_lz.spells.len(),
    );
    spells.extend(spells_0_1.spells);
    spells.extend(spells_2_ak.spells);
    spells.extend(spells_2_lz.spells);

    Compendium {
        races: rcb.races,
        classes: rcb.classes,
        backgrounds: rcb.backgrounds,
        spells,
        equipment,
        feats: feats.feats,
        weapon_properties: weapon_properties.weapon_properties,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compendium_loads_with_expected_counts() {
        let c = compendium();
        assert!(c.races.len() >= 9, "SRD 5.1 has 9 base races, got {}", c.races.len());
        assert!(
            c.classes.len() >= 12,
            "SRD 5.1 has 12 classes, got {}",
            c.classes.len()
        );
        assert!(!c.backgrounds.is_empty(), "backgrounds missing");
        assert!(
            c.spells.len() >= 120,
            "expected >=120 spells lvl 0-2, got {}",
            c.spells.len()
        );
        assert!(
            c.equipment.weapons.len() >= 30,
            "expected >=30 weapons, got {}",
            c.equipment.weapons.len()
        );
        assert!(!c.equipment.armor.is_empty(), "armor missing");
        assert!(
            !c.equipment.adventuring_gear.is_empty(),
            "adventuring gear missing"
        );
        assert_eq!(c.feats.len(), 1, "SRD 5.1 has only Grappler");
        assert!(
            c.weapon_properties.len() >= 10,
            "expected >=10 weapon properties"
        );
    }

    #[test]
    fn spells_partition_covers_levels_0_1_2() {
        let c = compendium();
        let by_level = |lvl: i32| c.spells.iter().filter(|s| s.level == lvl).count();
        assert!(by_level(0) >= 15, "cantrips count looks low: {}", by_level(0));
        assert!(by_level(1) >= 40, "lvl 1 spells low: {}", by_level(1));
        assert!(by_level(2) >= 40, "lvl 2 spells low: {}", by_level(2));
    }

    #[test]
    fn race_dwarf_has_subraces() {
        let dwarf = compendium()
            .races
            .iter()
            .find(|r| r.id == "dwarf")
            .expect("dwarf race");
        assert!(!dwarf.subraces.is_empty(), "dwarf has at least one subrace");
    }
}
