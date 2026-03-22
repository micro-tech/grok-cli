#[tokio::main]
async fn main() -> anyhow::Result<()> {
    grok_cli::cli::app::run().await
}
