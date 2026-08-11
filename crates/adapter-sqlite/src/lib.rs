pub mod combat;
pub mod journal;
pub mod legacy;
pub mod messages;
pub mod npcs;
pub mod pool;
pub mod saves;
pub mod scenes;
pub mod srd;

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
