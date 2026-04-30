use super::combatant::Combatant;
use crate::dice::{Die, roll_one};
use crate::rng::SeededRng;

/// Apply healing to a combatant. Healing at 0 HP revives and clears death saves.
pub fn apply_healing(combatant: &mut Combatant, amount: i32) {
    combatant.apply_healing(amount);
}

/// Roll a death saving throw for a combatant at 0 HP.
/// Returns `true` if the roll is a success (>= 10).
/// Updates the combatant's DeathSaves accordingly.
/// Panics in debug builds if called on a non-downed combatant (programming error).
pub fn roll_death_save(combatant: &mut Combatant, rng: &mut SeededRng) -> bool {
    debug_assert!(combatant.current_hp == 0, "death save on non-downed combatant");
    let roll = roll_one(Die::D20, rng);
    if roll >= 10 {
        combatant.death_saves.record_success();
        true
    } else {
        combatant.death_saves.record_failure();
        false
    }
}
