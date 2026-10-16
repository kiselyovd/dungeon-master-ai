//! Canonical defense projection for versioned sheets and recognized legacy armor kits.
use app_domain::{
    combat::defense::{self, DefenseChoice, DefenseInput, DefenseStats},
    compendium::character_catalog,
};
use serde_json::{json, Value};
pub fn normalize(sheet: &mut Value) -> Result<Option<DefenseStats>, &'static str> {
    let class = sheet["heroClass"].as_str().ok_or("hero_defense_invalid")?;
    let owned = sheet["inventory"]
        .as_array()
        .ok_or("hero_defense_invalid")?
        .iter()
        .map(|item| {
            Ok((
                item["id"]
                    .as_str()
                    .ok_or("hero_defense_invalid")?
                    .to_owned(),
                u32::try_from(item["count"].as_u64().ok_or("hero_defense_invalid")?)
                    .map_err(|_| "hero_defense_invalid")?,
            ))
        })
        .collect::<Result<Vec<_>, &'static str>>()?;
    let score = |key: &str| -> Result<i32, &'static str> {
        i32::try_from(
            sheet["abilities"][key]
                .as_i64()
                .ok_or("hero_defense_invalid")?,
        )
        .map_err(|_| "hero_defense_invalid")
    };
    let input = DefenseInput {
        class_id: class,
        strength: score("str")?,
        dexterity: score("dex")?,
        constitution: score("con")?,
        wisdom: score("wis")?,
        owned: &owned,
    };
    let result = if let Some(raw) = sheet.get("defense") {
        let choice: DefenseChoice =
            serde_json::from_value(raw.clone()).map_err(|_| "hero_defense_invalid")?;
        defense::derive(&input, &choice)?
    } else if defense::has_owned_body_armor(&input) {
        defense::starting_choice(&input)?
    } else {
        return Ok(None);
    };
    if !(1..=40).contains(&result.ac) {
        return Err("hero_defense_invalid");
    }
    let base_speed = character_catalog()
        .races
        .iter()
        .find(|r| Some(r.id.as_str()) == sheet["race"].as_str())
        .map(|r| u64::try_from(r.speed))
        .transpose()
        .map_err(|_| "hero_defense_invalid")?
        .or_else(|| sheet["baseSpeedFt"].as_u64())
        .or_else(|| sheet["speedFt"].as_u64())
        .unwrap_or(30);
    if !(1..=120).contains(&base_speed) {
        return Err("hero_defense_invalid");
    }
    sheet["ac"] = json!(result.ac);
    sheet["defense"] = serde_json::to_value(&result.choice).map_err(|_| "hero_defense_invalid")?;
    sheet["baseSpeedFt"] = json!(base_speed);
    sheet["speedFt"] = json!(base_speed.saturating_sub(result.speed_penalty_ft as u64));
    Ok(Some(result))
}

/// Keep an already valid legacy document readable when an implicit schema upgrade
/// would exceed its storage budget. An explicit edit can free space for the upgrade.
pub fn normalize_for_read(sheet: &mut Value) -> Result<Option<DefenseStats>, &'static str> {
    let mut candidate = sheet.clone();
    let result = normalize(&mut candidate)?;
    if !crate::models::campaign_state::within_character_budget(&candidate)
        && sheet.get("defense").is_none()
    {
        return Ok(None);
    }
    *sheet = candidate;
    Ok(result)
}

pub fn repository_error(code: &'static str) -> crate::ports::repositories::RepositoryError {
    crate::ports::repositories::RepositoryError::Operation {
        operation: "character_defense",
        code,
    }
}
