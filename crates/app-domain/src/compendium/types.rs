//! Typed mirror of the SRD 5.1 character-creation YAML datasets.
//!
//! Top-level identifying fields are strict (`id`, `name_en`, `name_ru`, etc.)
//! so wizard code can pattern-match. Deeply nested or heterogeneous fields
//! (spell `range_ft` can be int or string, class `subclasses` carry rich
//! nested feature trees) stay as `serde_yaml::Value` for v1: the loader's
//! job is to confirm the file parses, not to over-specify a schema that the
//! upcoming wizard milestone will refine.

use serde::{Deserialize, Serialize};
use serde_yaml::Value as YamlValue;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct AbilityScores {
    #[serde(default)]
    pub str: Option<i32>,
    #[serde(default)]
    pub dex: Option<i32>,
    #[serde(default)]
    pub con: Option<i32>,
    #[serde(default)]
    pub int: Option<i32>,
    #[serde(default)]
    pub wis: Option<i32>,
    #[serde(default)]
    pub cha: Option<i32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Senses {
    #[serde(default)]
    pub darkvision_ft: Option<i32>,
    #[serde(default)]
    pub truesight_ft: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AgeRange {
    pub mature_at: i32,
    pub max_lifespan: i32,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct RaceProficiencies {
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub weapons: Vec<String>,
    #[serde(default)]
    pub tools: Vec<String>,
    #[serde(default)]
    pub saves: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct RaceTrait {
    pub id: String,
    pub name_en: String,
    pub name_ru: String,
    pub mechanical_description: String,
    #[serde(default)]
    pub flavor_description: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Subrace {
    pub id: String,
    pub name_en: String,
    pub name_ru: String,
    #[serde(default)]
    pub additional_asi: AbilityScores,
    #[serde(default)]
    pub additional_traits: Vec<RaceTrait>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Race {
    pub id: String,
    pub name_en: String,
    pub name_ru: String,
    #[serde(default)]
    pub ability_score_increases: AbilityScores,
    pub age: AgeRange,
    #[serde(default)]
    pub alignment_tendency: Option<String>,
    pub size: String,
    pub speed: i32,
    #[serde(default)]
    pub languages: Vec<String>,
    pub proficiencies: RaceProficiencies,
    pub senses: Senses,
    #[serde(default)]
    pub traits: Vec<RaceTrait>,
    #[serde(default)]
    pub subraces: Vec<Subrace>,
    pub source_url: String,
    pub srd_section: String,
}

/// Class-level entry. Free-form nested arrays (skill_proficiencies choose
/// blocks, starting_equipment options, level_1_features, spellcasting block,
/// subclasses) are kept as raw YAML values so the wizard milestone can design
/// its own typed surface without bouncing back to this module.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Class {
    pub id: String,
    pub name_en: String,
    pub name_ru: String,
    pub hit_die: i32,
    #[serde(default)]
    pub primary_ability: Vec<String>,
    #[serde(default)]
    pub saving_throw_proficiencies: Vec<String>,
    #[serde(default)]
    pub armor_proficiencies: Vec<String>,
    #[serde(default)]
    pub weapon_proficiencies: Vec<String>,
    #[serde(default)]
    pub tool_proficiencies: Vec<String>,
    #[serde(default)]
    pub skill_proficiencies: YamlValue,
    #[serde(default)]
    pub starting_equipment: YamlValue,
    #[serde(default)]
    pub level_1_features: YamlValue,
    #[serde(default)]
    pub spellcasting: YamlValue,
    #[serde(default)]
    pub subclass_at_level: Option<i32>,
    #[serde(default)]
    pub subclasses: YamlValue,
    pub source_url: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct BackgroundFeature {
    pub name_en: String,
    pub name_ru: String,
    pub description: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Background {
    pub id: String,
    pub name_en: String,
    pub name_ru: String,
    #[serde(default)]
    pub skill_proficiencies: Vec<String>,
    #[serde(default)]
    pub tool_proficiencies: Vec<String>,
    #[serde(default)]
    pub language_proficiencies: YamlValue,
    #[serde(default)]
    pub starting_equipment: YamlValue,
    #[serde(default)]
    pub starting_gold: Option<i32>,
    pub feature: BackgroundFeature,
    #[serde(default)]
    pub suggested_characteristics: YamlValue,
    #[serde(default)]
    pub source_url: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct SpellComponents {
    #[serde(default)]
    pub v: bool,
    #[serde(default)]
    pub s: bool,
    #[serde(default)]
    pub m: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpellDamage {
    pub dice: String,
    #[serde(rename = "type")]
    pub damage_type: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpellAttack {
    pub kind: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpellSave {
    pub ability: String,
    #[serde(default)]
    pub half_on_success: bool,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct BilingualText {
    #[serde(default)]
    pub en: Option<String>,
    #[serde(default)]
    pub ru: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Spell {
    pub id: String,
    pub name_en: String,
    pub name_ru: String,
    pub level: i32,
    pub school: String,
    pub casting_time: String,
    #[serde(default)]
    pub range_ft: YamlValue,
    pub components: SpellComponents,
    pub duration: String,
    #[serde(default)]
    pub ritual: bool,
    #[serde(default)]
    pub concentration: bool,
    #[serde(default)]
    pub classes: Vec<String>,
    pub description_en: String,
    pub description_ru: String,
    #[serde(default)]
    pub at_higher_levels: Option<BilingualText>,
    #[serde(default)]
    pub damage: Option<SpellDamage>,
    #[serde(default)]
    pub attack: Option<SpellAttack>,
    #[serde(default)]
    pub save: Option<SpellSave>,
    #[serde(default)]
    pub scales_with_level: Option<String>,
    pub source_url: String,
    pub srd_section: String,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Cost {
    #[serde(default)]
    pub gp: Option<i32>,
    #[serde(default)]
    pub sp: Option<i32>,
    #[serde(default)]
    pub cp: Option<i32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct WeaponRange {
    #[serde(default)]
    pub normal: Option<i32>,
    #[serde(default)]
    pub long: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WeaponDamage {
    pub dice: String,
    #[serde(rename = "type")]
    pub damage_type: String,
    #[serde(default)]
    pub versatile_dice: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Weapon {
    pub id: String,
    pub name_en: String,
    pub name_ru: String,
    pub category: String,
    #[serde(default)]
    pub cost: Cost,
    pub damage: WeaponDamage,
    pub weight_lb: f32,
    #[serde(default)]
    pub properties: Vec<String>,
    #[serde(default)]
    pub range_ft: WeaponRange,
    #[serde(default)]
    pub special_rules: Option<BilingualText>,
    pub source_url: String,
    pub srd_section: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Armor {
    pub id: String,
    pub name_en: String,
    pub name_ru: String,
    pub category: String,
    #[serde(default)]
    pub cost: Cost,
    pub ac_base: i32,
    #[serde(default)]
    pub ac_dex_bonus_cap: Option<i32>,
    #[serde(default)]
    pub stealth_disadvantage: bool,
    #[serde(default)]
    pub str_req: Option<i32>,
    pub weight_lb: f32,
    #[serde(default)]
    pub don_time: Option<String>,
    #[serde(default)]
    pub doff_time: Option<String>,
    pub source_url: String,
    pub srd_section: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct AdventuringGear {
    pub id: String,
    pub name_en: String,
    pub name_ru: String,
    #[serde(default)]
    pub cost: Cost,
    pub weight_lb: f32,
    #[serde(default)]
    pub description_en: Option<String>,
    #[serde(default)]
    pub description_ru: Option<String>,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub srd_section: Option<String>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct EquipmentBundle {
    #[serde(default)]
    pub weapons: Vec<Weapon>,
    #[serde(default)]
    pub armor: Vec<Armor>,
    #[serde(default)]
    pub adventuring_gear: Vec<AdventuringGear>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct FeatPrerequisite {
    #[serde(rename = "type")]
    pub kind: String,
    #[serde(default)]
    pub ability: Option<String>,
    #[serde(default)]
    pub minimum: Option<i32>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Feat {
    pub id: String,
    pub name_en: String,
    pub name_ru: String,
    #[serde(default)]
    pub prerequisites: Vec<FeatPrerequisite>,
    #[serde(default)]
    pub asi_grants: YamlValue,
    #[serde(default)]
    pub effects_en: Vec<String>,
    #[serde(default)]
    pub effects_ru: Vec<String>,
    #[serde(default)]
    pub mechanical_hooks: YamlValue,
    pub source_url: String,
    pub srd_section: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct WeaponProperty {
    pub id: String,
    pub name_en: String,
    pub name_ru: String,
    pub description_en: String,
    pub description_ru: String,
}
