use anyhow::{Context, Result};
use axum::routing::{delete, get, post};
use clap::Parser;
use std::path::Path;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::{config::Config};

mod api;
mod cmd;
mod cli;
mod config;
mod wb;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {:?}", e);
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = cli::Cli::parse();

    let config = load_config(&cli.config)?;

    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind(&config.agent.bind).await?;
    
    // todo: API_TOKEN auth between server and agent
    let app = axum::Router::new()
        .route("/health", get(|| async { "OK" }))
        .route("/config", get(api::get_config))

        // TODO: we need to use post with custom headers instead of /CMD/stuff for better security
        // so that, the variant of CMD can be handled by only one CMD route
        // for simplicity and flexibility, different CMD will be indicated by path
        .route("/ping/{arg}", get(cmd::get_ping))
        .route("/ping4/{arg}", get(cmd::get_ping4))
        .route("/ping6/{arg}", get(cmd::get_ping6))

        // TODO, more CMDs, like traceroute, wg show, etc.
        // ref: `docs/dn42-bot`
        .route("/create_peer", post(wb::create_config))
        .route("/modify_peer", post(wb::modify_config))
        .route("/delete_peer", delete(wb::delete_config))
        .layer(TraceLayer::new_for_http())
        .with_state(config);

    info!(
        "Agent is running on http://{}",
        listener.local_addr().unwrap()
    );

    axum::serve(listener, app).await?;

    Ok(())
}

fn load_config<P: AsRef<Path>>(path: P) -> Result<Config> {
    let config_content = std::fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&config_content)
        .with_context(|| format!("Failed to load config: {}", path.as_ref().display()))?;
    Ok(config)
}
