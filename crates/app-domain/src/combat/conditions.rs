use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Condition {
    Prone,
    Grappled,
    Frightened,
    Poisoned,
    Unconscious,
    /// Blinded - v2+; stub present for forward compat
    Blinded,
}

impl std::fmt::Display for Condition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Condition::Prone => "prone",
            Condition::Grappled => "grappled",
            Condition::Frightened => "frightened",
            Condition::Poisoned => "poisoned",
            Condition::Unconscious => "unconscious",
            Condition::Blinded => "blinded",
        };
        write!(f, "{s}")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttackModifier {
    Advantage,
    Disadvantage,
    Normal,
}

/// Modifier for attacks *made against* a creature with this condition.
/// `melee` = true means the attack is a melee attack.
pub fn condition_attack_modifier(condition: Condition, melee: bool) -> AttackModifier {
    match condition {
        Condition::Prone => {
            if melee { AttackModifier::Advantage } else { AttackModifier::Disadvantage }
        }
        Condition::Unconscious => AttackModifier::Advantage,
        _ => AttackModifier::Normal,
    }
}

/// Modifier applied to attacks *made by* a creature that has this condition.
pub fn attacker_condition_modifier(condition: Condition) -> AttackModifier {
    match condition {
        Condition::Poisoned | Condition::Frightened | Condition::Blinded => {
            AttackModifier::Disadvantage
        }
        Condition::Prone => AttackModifier::Disadvantage,
        _ => AttackModifier::Normal,
    }
}
