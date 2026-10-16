use serde::de::DeserializeOwned;
use serde_json::Value;
use thiserror::Error;

use super::commands::*;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum ToolDecodeError {
    #[error("unknown tool: {0}")]
    UnknownTool(String),
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("validation failed: {0}")]
    ValidationFailed(String),
}

pub fn decode_tool_call(
    tool_name: &str,
    args: &Value,
) -> Result<AgentToolCommand, ToolDecodeError> {
    if !args.is_object() {
        return Err(ToolDecodeError::InvalidArgs(
            "tool arguments must be an object".into(),
        ));
    }

    let command = match tool_name {
        "roll_dice" => AgentToolCommand::RollDice(decode(args)?),
        "apply_damage" => AgentToolCommand::ApplyDamage(decode(args)?),
        "apply_healing" => AgentToolCommand::ApplyHealing(decode(args)?),
        "start_combat" => AgentToolCommand::StartCombat(decode(args)?),
        "end_combat" => AgentToolCommand::EndCombat(decode(args)?),
        "add_token" => AgentToolCommand::AddToken(decode(args)?),
        "update_token" => AgentToolCommand::UpdateToken(decode(args)?),
        "remove_token" => AgentToolCommand::RemoveToken(decode(args)?),
        "set_scene" => AgentToolCommand::SetScene(decode_scene(args)?),
        "cast_spell" => AgentToolCommand::CastSpell(decode(args)?),
        "remember_npc" => AgentToolCommand::RememberNpc(decode(args)?),
        "recall_npc" => AgentToolCommand::RecallNpc(decode(args)?),
        "journal_append" => AgentToolCommand::JournalAppend(decode(args)?),
        "quick_save" => AgentToolCommand::QuickSave(decode(args)?),
        "query_rules" => AgentToolCommand::QueryRules(decode(args)?),
        "generate_map" => AgentToolCommand::GenerateMap(decode(args)?),
        "generate_illustration" => AgentToolCommand::GenerateIllustration(decode(args)?),
        "generate_video" => AgentToolCommand::GenerateVideo(decode(args)?),
        _ => return Err(ToolDecodeError::UnknownTool(tool_name.to_owned())),
    };

    validate(&command)?;
    Ok(command)
}

fn decode<T: DeserializeOwned>(args: &Value) -> Result<T, ToolDecodeError> {
    serde_json::from_value(args.clone()).map_err(|error| {
        ToolDecodeError::InvalidArgs(format!("payload does not match tool schema: {error}"))
    })
}

fn decode_scene(args: &Value) -> Result<SetSceneCommand, ToolDecodeError> {
    #[derive(serde::Deserialize)]
    struct ScenePayload {
        title: String,
        mode: String,
        #[serde(default)]
        subtitle: Option<String>,
        #[serde(default)]
        image_prompt: Option<String>,
    }

    let payload: ScenePayload = decode(args)?;
    let mode = match payload.mode.as_str() {
        "exploration" => SceneMode::Exploration,
        "combat" => SceneMode::Combat,
        other => {
            return Err(ToolDecodeError::ValidationFailed(format!(
                "invalid mode '{other}'; must be 'exploration' or 'combat'"
            )))
        }
    };
    Ok(SetSceneCommand {
        title: payload.title,
        mode,
        subtitle: payload.subtitle,
        image_prompt: payload.image_prompt,
    })
}

fn require_text(value: &str, field: &str) -> Result<(), ToolDecodeError> {
    if value.trim().is_empty() {
        return Err(ToolDecodeError::ValidationFailed(format!(
            "'{field}' must not be empty"
        )));
    }
    Ok(())
}

fn validate(command: &AgentToolCommand) -> Result<(), ToolDecodeError> {
    match command {
        AgentToolCommand::RollDice(command) => require_text(&command.dice, "dice"),
        AgentToolCommand::ApplyDamage(command) => {
            require_text(&command.token_id, "token_id")?;
            if command.amount < 0 {
                return Err(ToolDecodeError::ValidationFailed(
                    "damage amount must be >= 0".into(),
                ));
            }
            Ok(())
        }
        AgentToolCommand::ApplyHealing(command) => {
            require_text(&command.token_id, "token_id")?;
            if command.amount < 0 {
                return Err(ToolDecodeError::ValidationFailed(
                    "healing amount must be >= 0".into(),
                ));
            }
            Ok(())
        }
        AgentToolCommand::StartCombat(command) => {
            for entry in &command.initiative_entries {
                require_text(&entry.name, "initiative_entries.name")?;
            }
            Ok(())
        }
        AgentToolCommand::EndCombat(_) | AgentToolCommand::QuickSave(_) => Ok(()),
        AgentToolCommand::AddToken(command) => {
            require_text(&command.id, "id")?;
            require_text(&command.name, "name")
        }
        AgentToolCommand::UpdateToken(command) => require_text(&command.id, "id"),
        AgentToolCommand::RemoveToken(command) => require_text(&command.id, "id"),
        AgentToolCommand::SetScene(command) => require_text(&command.title, "title"),
        AgentToolCommand::CastSpell(command) => {
            require_text(&command.caster_id, "caster_id")?;
            require_text(&command.spell, "spell")?;
            if command.targets.is_empty() {
                return Err(ToolDecodeError::ValidationFailed(
                    "cast_spell requires at least one target".into(),
                ));
            }
            for target in &command.targets {
                require_text(target, "targets")?;
            }
            if command
                .slot_level
                .is_some_and(|level| !(1..=9).contains(&level))
            {
                return Err(ToolDecodeError::ValidationFailed(
                    "slot_level must be between 1 and 9".into(),
                ));
            }
            Ok(())
        }
        AgentToolCommand::RememberNpc(command) => {
            require_text(&command.name, "name")?;
            require_text(&command.fact, "fact")
        }
        AgentToolCommand::RecallNpc(command) => require_text(&command.name, "name"),
        AgentToolCommand::JournalAppend(command) => require_text(&command.entry_html, "entry_html"),
        AgentToolCommand::QueryRules(command) => require_text(&command.question, "question"),
        AgentToolCommand::GenerateMap(command) => require_text(&command.prompt, "prompt"),
        AgentToolCommand::GenerateIllustration(command) => require_text(&command.prompt, "prompt"),
        AgentToolCommand::GenerateVideo(command) => {
            require_text(&command.prompt, "prompt")?;
            if command
                .seconds
                .is_some_and(|seconds| !seconds.is_finite() || !(3.0..=8.0).contains(&seconds))
            {
                return Err(ToolDecodeError::ValidationFailed(
                    "seconds must be between 3 and 8".into(),
                ));
            }
            if command.frame_count == Some(0) {
                return Err(ToolDecodeError::ValidationFailed(
                    "frame_count must be greater than 0".into(),
                ));
            }
            Ok(())
        }
    }
}
