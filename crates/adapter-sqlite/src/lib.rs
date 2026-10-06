pub mod campaign_state;
pub mod campaigns;
pub mod combat;
pub mod events;
pub mod journal;
pub mod messages;
pub mod npcs;
pub mod pool;
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
