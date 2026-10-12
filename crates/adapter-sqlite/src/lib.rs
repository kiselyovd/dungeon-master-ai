pub mod adventure;
pub mod agent_operations;
pub mod agent_recovery;
mod agent_replay;
mod authored_combat;
pub mod campaign_state;
pub mod campaigns;
pub mod combat;
mod combat_binding;
pub mod events;
mod hero_checks;
mod hero_intentions;
mod human_command_clarifications;
pub mod human_commands;
pub mod journal;
pub mod messages;
pub mod npcs;
pub mod party;
pub mod pool;
pub mod rest;
mod room_agent;
mod room_characters;
mod room_combat;
pub mod room_human_commands;
mod room_state;
mod room_world;
pub mod rooms;
pub mod saves;
pub mod scenes;
pub mod sessions;
pub mod srd;
pub mod tool_support;

use sqlx::SqlitePool;

#[derive(Clone)]
pub struct SqliteStore {
    pool: SqlitePool,
}

impl SqliteStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

mod room_dm_turns;

mod recovery_clock;
mod room_checks;
mod room_recovery;

mod solo_recovery;
