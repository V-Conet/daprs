use anyhow::{Context, Result};
use axum::{
    middleware,
    routing::{delete, get, post},
};
use clap::Parser;
use std::path::Path;
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::{cmd::cmd_handler, config::Config, mw::auth_middleware};

mod api;
mod cli;
mod cmd;
mod config;
mod mw;
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

    // public API, no auth
    let app = axum::Router::new()
        .route("/health", get(|| async { "OK" }))
        .layer(TraceLayer::new_for_http());

    // protected API, require API_TOKEN
    // todo: API_TOKEN auth between server and agent
    let _app = axum::Router::new()
        .route("/config", get(api::get_config))
        .route("/cmd", post(cmd_handler))
        // TODO, more CMDs, like traceroute, wg show, etc.
        // ref: `docs/dn42-bot`
        .route("/create_peer", post(wb::create_config))
        .route("/modify_peer", post(wb::modify_config))
        .route("/delete_peer", delete(wb::delete_config))
        .route("/peer_info", get(wb::get_peer_info))
        .layer(TraceLayer::new_for_http())
        .route_layer(middleware::from_fn_with_state(
            config.clone(),
            auth_middleware,
        ))
        .with_state(config);

    info!(
        "Agent is running on http://{}",
        listener.local_addr().unwrap()
    );

    axum::serve(listener, app.merge(_app)).await?;

    Ok(())
}

fn load_config<P: AsRef<Path>>(path: P) -> Result<Config> {
    let config_content = std::fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&config_content)
        .with_context(|| format!("Failed to load config: {}", path.as_ref().display()))?;
    Ok(config)
}
