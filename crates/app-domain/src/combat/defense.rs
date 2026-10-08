//! Defense from an explicit worn loadout; owned equipment alone is not a declaration.
use super::types::ability_modifier;
use crate::compendium::character_catalog;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DefenseChoice {
    pub schema_version: u16,
    pub armor_id: Option<String>,
    pub shield: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DefenseStats {
    pub choice: DefenseChoice,
    pub ac: i32,
    pub speed_penalty_ft: i32,
    pub stealth_disadvantage: bool,
    pub untrained_armor: bool,
}
pub struct DefenseInput<'a> {
    pub class_id: &'a str,
    pub strength: i32,
    pub dexterity: i32,
    pub constitution: i32,
    pub wisdom: i32,
    pub owned: &'a [(String, u32)],
}
fn owns(input: &DefenseInput<'_>, id: &str) -> bool {
    let prefix = format!("{}-", input.class_id);
    input
        .owned
        .iter()
        .any(|(owned, count)| *count > 0 && owned.strip_prefix(&prefix).unwrap_or(owned) == id)
}
pub fn derive(
    input: &DefenseInput<'_>,
    choice: &DefenseChoice,
) -> Result<DefenseStats, &'static str> {
    if choice.schema_version != 1
        || [
            input.strength,
            input.dexterity,
            input.constitution,
            input.wisdom,
        ]
        .iter()
        .any(|s| !(1..=30).contains(s))
    {
        return Err("hero_defense_invalid");
    }
    let catalog = character_catalog();
    let class = catalog
        .classes
        .iter()
        .find(|c| c.id == input.class_id)
        .ok_or("combat_class_unknown")?;
    let shield_trained = class.armor_proficiencies.iter().any(|p| p == "shield");
    if choice.shield && !owns(input, "shield") {
        return Err("hero_shield_unavailable");
    }
    let dex = ability_modifier(input.dexterity);
    let (base, speed_penalty_ft, stealth_disadvantage, untrained_armor) =
        if let Some(id) = &choice.armor_id {
            let armor = catalog
                .equipment
                .armor
                .iter()
                .find(|a| a.id == *id && a.category != "shield")
                .ok_or("hero_armor_unknown")?;
            if !owns(input, id) {
                return Err("hero_armor_unavailable");
            }
            let dex_bonus = match armor.ac_dex_bonus_cap {
                Some(0) => 0,
                Some(cap) => dex.min(cap),
                None => dex,
            };
            (
                armor.ac_base + dex_bonus,
                if armor
                    .str_req
                    .is_some_and(|required| input.strength < required)
                {
                    10
                } else {
                    0
                },
                armor.stealth_disadvantage,
                !class.armor_proficiencies.contains(&armor.category),
            )
        } else {
            let base = 10 + dex;
            (
                match input.class_id {
                    "barbarian" => base + ability_modifier(input.constitution),
                    "monk" if !choice.shield => base + ability_modifier(input.wisdom),
                    _ => base,
                },
                0,
                false,
                false,
            )
        };
    Ok(DefenseStats {
        choice: choice.clone(),
        ac: base
            + if choice.shield && shield_trained {
                2
            } else {
                0
            },
        speed_penalty_ft,
        stealth_disadvantage,
        untrained_armor,
    })
}
/// Recommend trained starting gear; the player can explicitly declare another owned option.
pub fn starting_choice(input: &DefenseInput<'_>) -> Result<DefenseStats, &'static str> {
    let class = character_catalog()
        .classes
        .iter()
        .find(|c| c.id == input.class_id)
        .ok_or("combat_class_unknown")?;
    let shield = owns(input, "shield") && class.armor_proficiencies.iter().any(|p| p == "shield");
    let mut best = derive(
        input,
        &DefenseChoice {
            schema_version: 1,
            armor_id: None,
            shield,
        },
    )?;
    for armor in &character_catalog().equipment.armor {
        if armor.category == "shield"
            || !owns(input, &armor.id)
            || !class.armor_proficiencies.contains(&armor.category)
        {
            continue;
        }
        let candidate = derive(
            input,
            &DefenseChoice {
                schema_version: 1,
                armor_id: Some(armor.id.clone()),
                shield,
            },
        )?;
        if candidate.ac > best.ac {
            best = candidate;
        }
    }
    Ok(best)
}
pub fn has_owned_body_armor(input: &DefenseInput<'_>) -> bool {
    let Some(class) = character_catalog()
        .classes
        .iter()
        .find(|c| c.id == input.class_id)
    else {
        return false;
    };
    character_catalog().equipment.armor.iter().any(|a| {
        a.category != "shield"
            && owns(input, &a.id)
            && class.armor_proficiencies.contains(&a.category)
    })
}
