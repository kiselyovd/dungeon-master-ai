//! Shared domain types for the dungeon-master-ai backend.
//!
//! This crate is intentionally thin in M1; it grows substantially in M2
//! when the rules engine introduces Action, ResultEvents, GameState, etc.

pub mod placeholder {
    pub fn ping() -> &'static str {
        "domain alive"
    }
}
