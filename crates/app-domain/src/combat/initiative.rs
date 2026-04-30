use serde::{Deserialize, Serialize};

use super::types::CombatantId;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InitiativeEntry {
    pub id: CombatantId,
    pub roll: i32,
    pub dex_tiebreak: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InitiativeOrder {
    entries: Vec<InitiativeEntry>,
    current_index: usize,
    round: u32,
}

impl InitiativeOrder {
    pub fn build(mut entries: Vec<InitiativeEntry>) -> Self {
        // Sort descending by roll, then descending by DEX for ties.
        entries.sort_by(|a, b| {
            b.roll.cmp(&a.roll).then_with(|| b.dex_tiebreak.cmp(&a.dex_tiebreak))
        });
        Self { entries, current_index: 0, round: 1 }
    }

    pub fn current(&self) -> &InitiativeEntry {
        &self.entries[self.current_index]
    }

    pub fn round(&self) -> u32 {
        self.round
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn as_slice(&self) -> &[InitiativeEntry] {
        &self.entries
    }

    /// Advance to the next combatant. If wrapping past the end, increments the round.
    pub fn advance(&mut self) {
        self.current_index += 1;
        if self.current_index >= self.entries.len() {
            self.current_index = 0;
            self.round += 1;
        }
    }

    pub fn remove(&mut self, id: CombatantId) {
        if let Some(pos) = self.entries.iter().position(|e| e.id == id) {
            self.entries.remove(pos);
            // If we removed before or at current, step back so current is still valid.
            if self.entries.is_empty() { return; }
            if pos <= self.current_index && self.current_index > 0 {
                self.current_index -= 1;
            }
            // Clamp in case we were at the very end.
            if self.current_index >= self.entries.len() {
                self.current_index = 0;
            }
        }
    }
}
