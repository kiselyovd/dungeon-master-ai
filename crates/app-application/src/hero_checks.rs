//! Canonical skill checks: transport/model inputs never own hero modifiers.
use app_domain::{combat::conditions::Condition, dice::roll_d20_detailed, rng::SeededRng};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;

pub use app_domain::skills::Skill;
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HeroSkillCheck {
    pub character_id: Uuid,
    pub skill: Skill,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub dc: Option<u8>,
}
#[async_trait::async_trait]
pub trait HeroCheckSheetRepository: Send + Sync {
    async fn check_sheet(
        &self,
        campaign: Uuid,
        session: Uuid,
        character: Uuid,
    ) -> Result<Value, crate::ports::repositories::RepositoryError>;
}

pub fn roll_skill(
    sheet: &Value,
    check: &HeroSkillCheck,
    advantage: bool,
    disadvantage: bool,
    rng: &mut SeededRng,
) -> Result<Value, &'static str> {
    if check.character_id.is_nil() || check.dc.is_some_and(|dc| dc > 40) {
        return Err("hero_check_invalid");
    }
    let actor = crate::recovery::actor(sheet, check.character_id)?;
    if actor.current_hp <= 0 || actor.is_dead || actor.is_incapacitated() {
        return Err("hero_check_incapacitated");
    }
    let level = sheet["level"]
        .as_u64()
        .filter(|level| (1..=20).contains(level))
        .ok_or("hero_check_invalid")?;
    let score = sheet["abilities"][check.skill.ability()]
        .as_i64()
        .filter(|score| (1..=30).contains(score))
        .ok_or("hero_check_invalid")?;
    let proficient = match sheet
        .get("skillProfs")
        .and_then(|skills| skills.get(check.skill.key()))
    {
        None => false,
        Some(value) => value.as_bool().ok_or("hero_check_invalid")?,
    };
    let modifier = app_domain::skills::skill_modifier(score as i32, level, proficient)?;
    let defense = crate::defense::normalize_for_read(&mut sheet.clone())?;
    let armor_disadvantage = defense.is_some_and(|defense| {
        (check.skill == Skill::Stealth && defense.stealth_disadvantage)
            || (defense.untrained_armor && matches!(check.skill.ability(), "str" | "dex"))
    });
    let poisoned_disadvantage = actor.conditions.contains(&Condition::Poisoned);
    let disadvantage = disadvantage || armor_disadvantage || poisoned_disadvantage;
    let detail = roll_d20_detailed(modifier, advantage, disadvantage, rng);
    Ok(
        json!({"character_id":check.character_id,"skill":check.skill,"ability":check.skill.ability(),"rolls":detail.rolls,"modifier":detail.modifier,"total":detail.total,"dc":check.dc,"success":check.dc.map(|dc|detail.total >= i32::from(dc)),"advantage":advantage,"disadvantage":disadvantage,"armor_disadvantage":armor_disadvantage,"poisoned_disadvantage":poisoned_disadvantage}),
    )
}

/// Only identity needed to ask the server for a check; portraits/biography stay out.
pub fn context_identity(character: Uuid, sheet: &Value) -> Value {
    json!({"character_id":character,"name":sheet["name"].as_str().unwrap_or("").chars().take(120).collect::<String>()})
}

pub fn roll_skill_seeded(
    sheet: &Value,
    check: &HeroSkillCheck,
    advantage: bool,
    disadvantage: bool,
    seed: u64,
) -> Result<Value, &'static str> {
    roll_skill(
        sheet,
        check,
        advantage,
        disadvantage,
        &mut SeededRng::from_seed(seed),
    )
}
