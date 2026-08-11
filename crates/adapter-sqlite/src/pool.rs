use sqlx::SqlitePool;

pub async fn migrate(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let started_at = std::time::Instant::now();
    sqlx::migrate!("./migrations").run(pool).await?;
    tracing::info!(
        elapsed_ms = started_at.elapsed().as_millis(),
        "sqlite migrations applied"
    );
    Ok(())
}
