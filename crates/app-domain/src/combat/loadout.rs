use super::types::{ability_modifier, DamageType};
use crate::compendium::character_catalog;
use crate::dice::{DiceExpr, Die};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WeaponMode {
    #[default]
    Default,
    Melee,
    Thrown,
    Ranged,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponAttackProfile {
    pub weapon_id: String,
    pub attack_bonus: i32,
    pub damage: DiceExpr,
    pub damage_type: DamageType,
    pub reach_ft: i32,
    pub normal_range_ft: i32,
    pub long_range_ft: i32,
    pub ranged: bool,
    pub thrown: bool,
    pub ammunition: bool,
    pub two_handed: bool,
    pub versatile_damage: Option<DiceExpr>,
    pub heavy_disadvantage: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatLoadout {
    pub default_weapon: String,
    pub attacks: Vec<WeaponAttackProfile>,
    pub quantities: BTreeMap<String, u32>,
    pub arrows: u32,
    pub shield: bool,
}

pub struct LoadoutInput<'a> {
    pub class_id: &'a str,
    pub level: u8,
    pub strength: i32,
    pub dexterity: i32,
    pub owned: Vec<(String, u32)>,
    pub arrows: u32,
}

impl CombatLoadout {
    pub fn derive(input: LoadoutInput<'_>) -> Result<Self, &'static str> {
        let catalog = character_catalog();
        let class = catalog
            .classes
            .iter()
            .find(|class| class.id == input.class_id)
            .ok_or("combat_class_unknown")?;
        if !(1..=20).contains(&input.level) {
            return Err("combat_level_invalid");
        }
        let strength = ability_modifier(input.strength);
        let dexterity = ability_modifier(input.dexterity);
        let proficiency = 2 + (i32::from(input.level) - 1) / 4;
        let prefix = format!("{}-", input.class_id);
        let owns_shield = input.owned.iter().any(|(id, count)| {
            *count > 0 && (id == "shield" || id == &format!("{}shield", prefix))
        }) && class.armor_proficiencies.iter().any(|p| p == "shield");
        let mut attacks = Vec::new();
        let mut quantities = BTreeMap::<String, u32>::new();
        for (id, count) in input.owned {
            if count == 0 {
                continue;
            }
            let normalized = id.strip_prefix(&prefix).unwrap_or(&id);
            let Some(weapon) = catalog
                .equipment
                .weapons
                .iter()
                .find(|weapon| weapon.id == normalized)
            else {
                continue;
            };
            let quantity = quantities.entry(weapon.id.clone()).or_default();
            *quantity = quantity
                .checked_add(count)
                .ok_or("combat_quantity_invalid")?;
            if attacks
                .iter()
                .any(|attack: &WeaponAttackProfile| attack.weapon_id == weapon.id)
            {
                continue;
            }
            let finesse = weapon.properties.iter().any(|p| p == "finesse");
            let ranged = weapon.category.ends_with("_ranged");
            let ability = if finesse {
                strength.max(dexterity)
            } else if ranged {
                dexterity
            } else {
                strength
            };
            let trained = class.weapon_proficiencies.iter().any(|p| {
                p == &weapon.id
                    || (p == "simple" && weapon.category.starts_with("simple"))
                    || (p == "martial" && weapon.category.starts_with("martial"))
                    || (p == "martial-finesse-or-light"
                        && (finesse || weapon.properties.iter().any(|p| p == "light")))
            });
            let mut damage = parse_weapon_dice(&weapon.damage.dice)?;
            damage.modifier = ability;
            let damage_type = match weapon.damage.damage_type.as_str() {
                "slashing" => DamageType::Slashing,
                "piercing" => DamageType::Piercing,
                "bludgeoning" => DamageType::Bludgeoning,
                _ => return Err("combat_weapon_damage_invalid"),
            };
            let mut versatile = weapon
                .damage
                .versatile_dice
                .as_deref()
                .map(parse_weapon_dice)
                .transpose()?;
            if let Some(damage) = &mut versatile {
                damage.modifier = ability;
            }
            attacks.push(WeaponAttackProfile {
                weapon_id: weapon.id.clone(),
                attack_bonus: ability + if trained { proficiency } else { 0 },
                damage,
                damage_type,
                reach_ft: if weapon.properties.iter().any(|p| p == "reach") {
                    10
                } else {
                    5
                },
                normal_range_ft: weapon.range_ft.normal.unwrap_or(0),
                long_range_ft: weapon.range_ft.long.unwrap_or(0),
                ranged,
                thrown: weapon.properties.iter().any(|p| p == "thrown"),
                ammunition: weapon.properties.iter().any(|p| p == "ammunition"),
                two_handed: weapon.properties.iter().any(|p| p == "two-handed"),
                versatile_damage: versatile,
                heavy_disadvantage: weapon.properties.iter().any(|p| p == "heavy")
                    && if ranged {
                        input.dexterity < 13
                    } else {
                        input.strength < 13
                    },
            });
        }
        let default_weapon = attacks
            .first()
            .map(|a| a.weapon_id.clone())
            .unwrap_or_else(|| "unarmed".into());
        let shield = owns_shield && !attacks.first().is_some_and(|attack| attack.two_handed);
        attacks.push(WeaponAttackProfile::unarmed(strength, proficiency));
        Ok(Self {
            default_weapon,
            attacks,
            quantities,
            arrows: input.arrows,
            shield,
        })
    }
}

impl WeaponAttackProfile {
    pub fn unarmed(strength_modifier: i32, proficiency: i32) -> Self {
        Self {
            weapon_id: "unarmed".into(),
            attack_bonus: strength_modifier + proficiency,
            damage: DiceExpr {
                count: 0,
                die: Die::D4,
                modifier: (1 + strength_modifier).max(0),
            },
            damage_type: DamageType::Bludgeoning,
            reach_ft: 5,
            normal_range_ft: 0,
            long_range_ft: 0,
            ranged: false,
            thrown: false,
            ammunition: false,
            two_handed: false,
            versatile_damage: None,
            heavy_disadvantage: false,
        }
    }
}

fn parse_weapon_dice(raw: &str) -> Result<DiceExpr, &'static str> {
    let (count, sides) = raw.split_once('d').ok_or("combat_weapon_dice_invalid")?;
    let count = count
        .parse::<u8>()
        .map_err(|_| "combat_weapon_dice_invalid")?;
    let die = match sides {
        "4" => Die::D4,
        "6" => Die::D6,
        "8" => Die::D8,
        "10" => Die::D10,
        "12" => Die::D12,
        _ => return Err("combat_weapon_dice_invalid"),
    };
    if count == 0 || count > 2 {
        return Err("combat_weapon_dice_invalid");
    }
    Ok(DiceExpr {
        count,
        die,
        modifier: 0,
    })
}
