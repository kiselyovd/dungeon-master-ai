use app_domain::combat::types::{CombatantId, DamageType, Position};
use app_domain::dice::DiceExpr;
use uuid::Uuid;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum CombatActionCommand {
    Retreat {
        combatant: CombatantId,
    },
    Spell {
        caster: CombatantId,
        spell_id: String,
        targets: Vec<CombatantId>,
        slot_level: u8,
    },
    WeaponAttack {
        attacker: CombatantId,
        target: CombatantId,
        weapon_id: Option<String>,
        mode: app_domain::combat::loadout::WeaponMode,
        two_handed: bool,
    },
    Attack {
        attacker: CombatantId,
        target: CombatantId,
        attack_modifier: i32,
        damage_expr: DiceExpr,
        damage_type: DamageType,
    },
    Cast {
        combatant: CombatantId,
    },
    Move {
        combatant: CombatantId,
        to: Position,
    },
    EndTurn {
        combatant: CombatantId,
    },
}

#[derive(Debug, Clone)]
pub struct ResolveCombatCommand {
    pub request_id: String,
    pub encounter_id: Uuid,
    pub expected_revision: u64,
    pub rng_seed: u64,
    pub action: CombatActionCommand,
}
