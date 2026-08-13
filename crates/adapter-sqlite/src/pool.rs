use sqlx::SqlitePool;

static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

pub async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let started_at = std::time::Instant::now();
    MIGRATOR.run(pool).await?;
    tracing::info!(
        elapsed_ms = started_at.elapsed().as_millis(),
        migration_count = MIGRATOR.migrations.len(),
        "sqlite migration baseline applied"
    );
    Ok(())
}
