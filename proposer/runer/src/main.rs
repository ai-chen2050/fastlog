mod handler;
mod node_factory;
mod proposer;
mod storage;
mod api;
mod cli;

use cli::proposer::run_cli;
use tools::tokio_static;
use tracing::*;
use tracing_subscriber::EnvFilter;

fn main() {
    tokio_static::block_forever_on(async_main());
}

async fn async_main() {
    // set default log level: INFO
    let rust_log = std::env::var("RUST_LOG").unwrap_or_else(|_| "info".to_string());
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::new(rust_log))
        .init();

    info!("start proposer server");
    run_cli().await;
}
