//! Tool-call validator surface.
//!
//! M2/M3 BOUNDARY: This module defines the validator dispatch table mapping
//! tool-call names to validator functions. The validator surface is complete
//! in M2; the LLM agent loop that invokes these validators is M3 work.
//! No LLM calls happen in this module.
//!
//! The contract: the LLM may never mutate game state directly. Every proposed
//! action must pass through a validator here before `CombatResolver::resolve`
//! is called. Validators return `Ok(ValidatedToolCall)` or `Err(ToolCallError)`.

use serde_json::Value;
use thiserror::Error;

#[derive(Debug, Clone, Error)]
pub enum ToolCallError {
    #[error("unknown tool: {0}")]
    UnknownTool(String),
    #[error("invalid arguments: {0}")]
    InvalidArgs(String),
    #[error("validation failed: {0}")]
    ValidationFailed(String),
}

/// A validated, ready-to-execute action. Created only by validator functions.
/// The engine is the only consumer of this type.
#[derive(Debug, Clone)]
pub struct ValidatedToolCall {
    pub tool_name: String,
    pub args: Value,
}

/// Validate a raw tool-call JSON payload from the LLM.
/// M3 will call this from the agent loop; M2 ships the function signatures.
pub fn validate_tool_call(
    tool_name: &str,
    args: Value,
) -> Result<ValidatedToolCall, ToolCallError> {
    match tool_name {
        "roll_dice" => validate_roll_dice(args),
        "apply_damage" => validate_apply_damage(args),
        "start_combat" => validate_start_combat(args),
        "end_combat" => validate_end_combat(args),
        "add_token" => validate_add_token(args),
        "update_token" => validate_update_token(args),
        "remove_token" => validate_remove_token(args),
        _ => Err(ToolCallError::UnknownTool(tool_name.to_string())),
    }
}

fn validate_roll_dice(args: Value) -> Result<ValidatedToolCall, ToolCallError> {
    args.get("dice")
        .ok_or_else(|| ToolCallError::InvalidArgs("missing 'dice' field".into()))?;
    Ok(ValidatedToolCall { tool_name: "roll_dice".into(), args })
}

fn validate_apply_damage(args: Value) -> Result<ValidatedToolCall, ToolCallError> {
    for field in &["token_id", "amount", "type"] {
        args.get(field)
            .ok_or_else(|| ToolCallError::InvalidArgs(format!("missing '{field}' field")))?;
    }
    let amount = args["amount"]
        .as_i64()
        .ok_or_else(|| ToolCallError::InvalidArgs("amount must be integer".into()))?;
    if amount < 0 {
        return Err(ToolCallError::ValidationFailed(
            "damage amount must be >= 0".into(),
        ));
    }
    Ok(ValidatedToolCall { tool_name: "apply_damage".into(), args })
}

fn validate_start_combat(args: Value) -> Result<ValidatedToolCall, ToolCallError> {
    args.get("initiative_entries")
        .ok_or_else(|| ToolCallError::InvalidArgs("missing 'initiative_entries'".into()))?;
    Ok(ValidatedToolCall { tool_name: "start_combat".into(), args })
}

fn validate_end_combat(args: Value) -> Result<ValidatedToolCall, ToolCallError> {
    Ok(ValidatedToolCall { tool_name: "end_combat".into(), args })
}

fn validate_add_token(args: Value) -> Result<ValidatedToolCall, ToolCallError> {
    for field in &["id", "name", "x", "y", "hp", "max_hp", "ac"] {
        args.get(field)
            .ok_or_else(|| ToolCallError::InvalidArgs(format!("missing '{field}'")))?;
    }
    Ok(ValidatedToolCall { tool_name: "add_token".into(), args })
}

fn validate_update_token(args: Value) -> Result<ValidatedToolCall, ToolCallError> {
    args.get("id")
        .ok_or_else(|| ToolCallError::InvalidArgs("missing 'id'".into()))?;
    Ok(ValidatedToolCall { tool_name: "update_token".into(), args })
}

fn validate_remove_token(args: Value) -> Result<ValidatedToolCall, ToolCallError> {
    args.get("id")
        .ok_or_else(|| ToolCallError::InvalidArgs("missing 'id'".into()))?;
    Ok(ValidatedToolCall { tool_name: "remove_token".into(), args })
}
