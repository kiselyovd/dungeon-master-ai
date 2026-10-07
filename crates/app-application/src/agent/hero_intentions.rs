use crate::combat::commands::CombatActionCommand;
use crate::models::identity::{CampaignId, SessionId};
use app_domain::combat::loadout::WeaponMode;
use app_domain::combat::types::{CombatantId, Position};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum HeroCombatIntent {
    EndTurn,
    Retreat,
    Move {
        x: i32,
        y: i32,
    },
    Weapon {
        target: Uuid,
        weapon_id: String,
        #[serde(default)]
        mode: WeaponMode,
        #[serde(default)]
        two_handed: bool,
    },
    Spell {
        spell_id: String,
        targets: Vec<Uuid>,
        slot_level: u8,
    },
}
impl HeroCombatIntent {
    pub fn for_actor(&self, actor: CombatantId) -> CombatActionCommand {
        match self {
            Self::EndTurn => CombatActionCommand::EndTurn { combatant: actor },
            Self::Retreat => CombatActionCommand::Retreat { combatant: actor },
            Self::Move { x, y } => CombatActionCommand::Move {
                combatant: actor,
                to: Position { x: *x, y: *y },
            },
            Self::Weapon {
                target,
                weapon_id,
                mode,
                two_handed,
            } => CombatActionCommand::WeaponAttack {
                attacker: actor,
                target: CombatantId(*target),
                weapon_id: Some(weapon_id.clone()),
                mode: *mode,
                two_handed: *two_handed,
            },
            Self::Spell {
                spell_id,
                targets,
                slot_level,
            } => CombatActionCommand::Spell {
                caster: actor,
                spell_id: spell_id.clone(),
                targets: targets.iter().copied().map(CombatantId).collect(),
                slot_level: *slot_level,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct HeroIntention<T> {
    pub campaign_id: CampaignId,
    pub session_id: SessionId,
    pub causation_id: String,
    pub action: T,
}

#[async_trait::async_trait]
pub trait HeroIntentionRepository: Send + Sync {
    async fn combat_as_dm(
        &self,
        intent: HeroIntention<HeroCombatIntent>,
        rng_seed: u64,
    ) -> Result<
        crate::models::campaign_state::CampaignSessionState,
        crate::ports::repositories::RepositoryError,
    >;
    async fn rest_as_dm(
        &self,
        intent: HeroIntention<crate::models::rest::RestIntent>,
        rng_seed: u64,
    ) -> Result<
        crate::models::campaign_state::CampaignSessionState,
        crate::ports::repositories::RepositoryError,
    >;
}
