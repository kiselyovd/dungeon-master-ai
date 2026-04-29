use thiserror::Error;

use super::combatant::ActionBudget;
use super::types::ActionKind;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum ActionEconomyError {
    #[error("action already used this turn")]
    ActionExhausted,
    #[error("bonus action already used this turn")]
    BonusActionExhausted,
    #[error("reaction already used this turn")]
    ReactionExhausted,
    #[error("not enough movement remaining (have {have}ft, need {need}ft)")]
    InsufficientMovement { have: i32, need: i32 },
}

/// Attempt to consume the given action kind from the budget.
/// For Movement, this consumes the minimum 5ft increment; callers
/// that know exact feet should call `consume_movement_ft` directly.
pub fn consume_action(
    budget: &mut ActionBudget,
    kind: ActionKind,
) -> Result<(), ActionEconomyError> {
    match kind {
        ActionKind::Action => {
            if !budget.action {
                return Err(ActionEconomyError::ActionExhausted);
            }
            budget.action = false;
            Ok(())
        }
        ActionKind::BonusAction => {
            if !budget.bonus_action {
                return Err(ActionEconomyError::BonusActionExhausted);
            }
            budget.bonus_action = false;
            Ok(())
        }
        ActionKind::Reaction => {
            if !budget.reaction {
                return Err(ActionEconomyError::ReactionExhausted);
            }
            budget.reaction = false;
            Ok(())
        }
        ActionKind::Movement => consume_movement_ft(budget, 5),
    }
}

pub fn consume_movement_ft(
    budget: &mut ActionBudget,
    feet: i32,
) -> Result<(), ActionEconomyError> {
    if budget.movement_ft < feet {
        return Err(ActionEconomyError::InsufficientMovement {
            have: budget.movement_ft,
            need: feet,
        });
    }
    budget.movement_ft -= feet;
    Ok(())
}
