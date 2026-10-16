#[tokio::main]
async fn main() -> anyhow::Result<()> {
    app_bootstrap::run().await
}
