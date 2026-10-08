//! Skill ability mapping and level-based proficiency.
use crate::combat::types::ability_modifier;
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Skill {
    Acrobatics,
    AnimalHandling,
    Arcana,
    Athletics,
    Deception,
    History,
    Insight,
    Intimidation,
    Investigation,
    Medicine,
    Nature,
    Perception,
    Performance,
    Persuasion,
    Religion,
    SleightOfHand,
    Stealth,
    Survival,
}
impl Skill {
    pub fn ability(self) -> &'static str {
        match self {
            Self::Acrobatics | Self::SleightOfHand | Self::Stealth => "dex",
            Self::Athletics => "str",
            Self::Arcana | Self::History | Self::Investigation | Self::Nature | Self::Religion => {
                "int"
            }
            Self::AnimalHandling
            | Self::Insight
            | Self::Medicine
            | Self::Perception
            | Self::Survival => "wis",
            Self::Deception | Self::Intimidation | Self::Performance | Self::Persuasion => "cha",
        }
    }
    pub fn key(self) -> &'static str {
        match self {
            Self::Acrobatics => "acrobatics",
            Self::AnimalHandling => "animal_handling",
            Self::Arcana => "arcana",
            Self::Athletics => "athletics",
            Self::Deception => "deception",
            Self::History => "history",
            Self::Insight => "insight",
            Self::Intimidation => "intimidation",
            Self::Investigation => "investigation",
            Self::Medicine => "medicine",
            Self::Nature => "nature",
            Self::Perception => "perception",
            Self::Performance => "performance",
            Self::Persuasion => "persuasion",
            Self::Religion => "religion",
            Self::SleightOfHand => "sleight_of_hand",
            Self::Stealth => "stealth",
            Self::Survival => "survival",
        }
    }
}

pub fn skill_modifier(score: i32, level: u64, proficient: bool) -> Result<i32, &'static str> {
    if !(1..=30).contains(&score) || !(1..=20).contains(&level) {
        return Err("hero_check_invalid");
    }
    Ok(ability_modifier(score)
        + if proficient {
            2 + ((level - 1) / 4) as i32
        } else {
            0
        })
}
