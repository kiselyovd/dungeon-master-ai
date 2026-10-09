//! Authored, deterministic progression. Identifiers are public discoveries;
//! private canon and translated prose are supplied by presentation adapters.
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use thiserror::Error;

pub const BELL_ADVENTURE_ID: &str = "bell-beneath-water";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Location {
    Town,
    Quay,
    Shrine,
    Warehouse,
    Archive,
    Belfry,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Fact {
    FerryRoute,
    PorterObservation,
    WardDamaged,
    WardUnderstood,
    RosterAvailable,
    ClapperRecovered,
    ClapperRepaired,
    TikhonLocated,
    TikhonRescued,
    LedgerRecovered,
    LedgerPublished,
    EchoAcknowledged,
    WardRestored,
    LowerSluiceSealed,
    RewardGranted,
    DefenseHeld,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ending {
    RememberedRiver,
    ClosedCrossing,
    UnfinishedDebt,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(
    tag = "kind",
    content = "location",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum Action {
    StartDefense,
    Move(Location),
    InspectQuay,
    TalkAgata,
    ConsultLeon,
    NegotiateClapper,
    RetrieveClapperPublicly,
    RepairClapper,
    LocateTikhon,
    RescueTikhon,
    EscortTikhonHome,
    RecoverRoster,
    RecoverLedger,
    AcknowledgeEcho,
    RestoreWard,
    SealSluice,
    PublishLedger,
    Finish,
}

impl Action {
    /// Authored scene time; retries reuse the receipt instead of advancing it.
    pub fn elapsed_seconds(self) -> u64 {
        match self {
            Self::Finish | Self::StartDefense => 0,
            Self::ConsultLeon | Self::RepairClapper | Self::EscortTikhonHome => 600,
            _ => 300,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AdventureState {
    #[serde(default)]
    pub defense: Option<DefenseOutcome>,
    #[serde(default)]
    pub defense_encounter_id: Option<uuid::Uuid>,
    pub package_id: String,
    pub location: Location,
    pub facts: BTreeSet<Fact>,
    pub ending: Option<Ending>,
    pub closed: bool,
    pub history: Vec<Action>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DefenseOutcome {
    Active,
    Held,
    Retreated,
    Defeated,
}

pub struct Transition {
    pub state: AdventureState,
    pub gold_reward: u32,
}

#[derive(Debug, Error, PartialEq, Eq)]
#[error("adventure action is unavailable")]
pub struct AdventureError;

impl Default for AdventureState {
    fn default() -> Self {
        Self::new()
    }
}

impl AdventureState {
    pub fn new() -> Self {
        Self {
            defense: None,
            defense_encounter_id: None,
            package_id: BELL_ADVENTURE_ID.into(),
            location: Location::Town,
            facts: BTreeSet::from([Fact::FerryRoute]),
            ending: None,
            closed: false,
            history: vec![],
        }
    }

    pub fn available_actions(&self) -> Vec<Action> {
        use Action::*;
        use Fact::*;
        use Location::*;
        if self.closed
            || self.package_id != BELL_ADVENTURE_ID
            || self.defense == Some(DefenseOutcome::Active)
        {
            return vec![];
        }
        let neighbors: &[Location] = match self.location {
            Town => &[Quay, Shrine, Warehouse],
            Quay => &[Town, Archive],
            Shrine | Warehouse => &[Town],
            Archive => &[Quay, Belfry],
            Belfry => &[Archive],
        };
        let mut actions: Vec<Action> = neighbors.iter().copied().map(Move).collect();
        let mut offer = |action, condition| {
            if condition {
                actions.push(action);
            }
        };
        let has = |fact| self.facts.contains(&fact);
        match self.location {
            Town => {
                offer(TalkAgata, !has(PorterObservation));
                offer(PublishLedger, has(LedgerRecovered) && !has(LedgerPublished));
                offer(Finish, true);
            }
            Quay => offer(InspectQuay, !has(WardDamaged)),
            Shrine => offer(ConsultLeon, !has(WardUnderstood) || !has(RosterAvailable)),
            Warehouse => {
                offer(NegotiateClapper, !has(ClapperRecovered));
                offer(
                    RetrieveClapperPublicly,
                    has(WardDamaged) && !has(ClapperRecovered),
                );
                offer(
                    RepairClapper,
                    has(ClapperRecovered) && !has(ClapperRepaired),
                );
            }
            Archive => {
                offer(
                    StartDefense,
                    has(TikhonLocated) && !has(TikhonRescued) && !has(DefenseHeld),
                );
                offer(LocateTikhon, !has(TikhonLocated));
                offer(RescueTikhon, has(TikhonLocated) && !has(TikhonRescued));
                offer(EscortTikhonHome, has(TikhonLocated) && !has(TikhonRescued));
                offer(RecoverRoster, has(TikhonLocated) && !has(RosterAvailable));
                offer(RecoverLedger, has(TikhonRescued) && !has(LedgerRecovered));
            }
            Belfry => {
                offer(AcknowledgeEcho, !has(EchoAcknowledged));
                offer(
                    RestoreWard,
                    has(ClapperRepaired)
                        && has(RosterAvailable)
                        && has(EchoAcknowledged)
                        && !has(WardRestored)
                        && !has(LowerSluiceSealed),
                );
                offer(SealSluice, !has(WardRestored) && !has(LowerSluiceSealed));
            }
        }
        actions
    }

    pub fn apply(&self, action: Action) -> Result<Transition, AdventureError> {
        use Action::*;
        use Fact::*;
        if !self.available_actions().contains(&action) {
            return Err(AdventureError);
        }
        let mut state = self.clone();
        let mut gold_reward = 0;
        state.ending = None;
        match action {
            StartDefense => {
                state.defense = Some(DefenseOutcome::Active);
                state.defense_encounter_id = None;
            }
            Move(location) => state.location = location,
            InspectQuay => {
                state.facts.insert(WardDamaged);
            }
            TalkAgata => {
                state.facts.insert(PorterObservation);
            }
            ConsultLeon => {
                state.facts.extend([WardUnderstood, RosterAvailable]);
            }
            NegotiateClapper | RetrieveClapperPublicly => {
                state.facts.insert(ClapperRecovered);
            }
            RepairClapper => {
                state.facts.insert(ClapperRepaired);
            }
            LocateTikhon => {
                state.facts.insert(TikhonLocated);
            }
            RescueTikhon | EscortTikhonHome => {
                state.facts.insert(TikhonRescued);
                if action == EscortTikhonHome {
                    state.location = Location::Town;
                }
                if state.facts.insert(RewardGranted) {
                    gold_reward = 10;
                }
            }
            RecoverRoster => {
                state.facts.insert(RosterAvailable);
            }
            RecoverLedger => {
                state.facts.insert(LedgerRecovered);
            }
            AcknowledgeEcho => {
                state.facts.insert(EchoAcknowledged);
            }
            RestoreWard => {
                state.facts.insert(WardRestored);
            }
            SealSluice => {
                state.facts.insert(LowerSluiceSealed);
            }
            PublishLedger => {
                state.facts.insert(LedgerPublished);
            }
            Finish => {
                state.ending = Some(if !state.facts.contains(&TikhonRescued) {
                    Ending::UnfinishedDebt
                } else if state.facts.contains(&WardRestored) {
                    Ending::RememberedRiver
                } else if state.facts.contains(&LowerSluiceSealed) {
                    Ending::ClosedCrossing
                } else {
                    Ending::UnfinishedDebt
                });
                state.closed = state.ending != Some(Ending::UnfinishedDebt);
            }
        }
        state.history.push(action);
        if state.history.len() > 100 {
            state.history.remove(0);
        }
        Ok(Transition { state, gold_reward })
    }

    pub fn resolve_defense(
        &self,
        id: uuid::Uuid,
        outcome: DefenseOutcome,
    ) -> Result<Self, AdventureError> {
        if self.defense != Some(DefenseOutcome::Active)
            || self.defense_encounter_id != Some(id)
            || outcome == DefenseOutcome::Active
        {
            return Err(AdventureError);
        }
        let mut state = self.clone();
        state.defense = Some(outcome);
        match outcome {
            DefenseOutcome::Held => {
                state.facts.insert(Fact::DefenseHeld);
            }
            DefenseOutcome::Retreated => state.location = Location::Quay,
            DefenseOutcome::Defeated => state.location = Location::Town,
            DefenseOutcome::Active => return Err(AdventureError),
        }
        Ok(state)
    }
}
