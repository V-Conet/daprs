//! DAPRS Agent
//!
//! DN42 AutoPeering Agent

use std::sync::Arc;
use std::time::Duration;

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

use crate::config::Config;
use crate::utils::auth_middleware;
use shared::rate_limiter::{self, RateLimiter};

mod api;
mod cli;
mod cmd;
mod config;
mod utils;
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

    // ratelimit
    let api_limiter: Option<Arc<RateLimiter>> = config
        .agent
        .rate_limit
        .as_ref()
        .and_then(|rl| rl.api.as_ref())
        .map(|w| {
            Arc::new(RateLimiter::new(
                Duration::from_secs(w.window_secs),
                w.max_requests,
            ))
        });

    // public api
    let public_routes = axum::Router::new()
        .route("/health", get(|| async { "OK" }))
        .layer(TraceLayer::new_for_http());

    // protected api
    let mut protected_routes = axum::Router::new()
        // 配置查询
        .route("/config", get(api::get_config))
        // 命令执行
        .route("/cmd", post(cmd::cmd_handler))
        // Peering 管理
        .route("/create_peer", post(wb::create_config))
        .route("/modify_peer", post(wb::modify_config))
        .route("/delete_peer", delete(wb::delete_config))
        .route("/peer_info", get(wb::get_peer_info))
        .layer(TraceLayer::new_for_http())
        .route_layer(middleware::from_fn_with_state(
            config.clone(),
            auth_middleware,
        ))
        .with_state(config.clone());

    // ratelimit
    if let Some(limiter) = api_limiter {
        protected_routes = protected_routes.route_layer(middleware::from_fn_with_state(
            limiter,
            rate_limiter::rate_limit_middleware,
        ));
    }

    let app = public_routes.merge(protected_routes);

    info!(
        "Agent is running on http://{}",
        listener.local_addr().unwrap()
    );

    axum::serve(listener, app).await?;

    Ok(())
}

/// 加载配置文件
fn load_config<P: AsRef<Path>>(path: P) -> Result<Config> {
    let config_content = std::fs::read_to_string(&path)?;
    let config: Config = toml::from_str(&config_content)
        .with_context(|| format!("Failed to load config: {}", path.as_ref().display()))?;
    Ok(config)
}
