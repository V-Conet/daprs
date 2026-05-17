//! DAPRS Server
//!
//! DN42 AutoPeering Server，提供 WebUI API 和 OAuth 认证。

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::middleware;
use axum::routing::{delete, get, post};
use clap::Parser;
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

use crate::api::{handler, oauth};
use crate::config::{AppState, Config};
use shared::rate_limiter::{self, RateLimiter};

mod api;
mod cli;
mod config;

// const INDEX_HTML: &str = r#"
// <!DOCTYPE html>
// <html>
// <head>
//     <title>DAPRS API</title>
//     <style>
//         body { font-family: system-ui, sans-serif; max-width: 720px; margin: 2rem auto; padding: 0 1rem; }
//         h1 { color: #333; }
//         a { color: #0066cc; text-decoration: none; }
//         a:hover { text-decoration: underline; }
//         ul { list-style: none; padding: 0; }
//         li { margin: 0.5rem 0; }
//     </style>
// </head>
// <body>
//     <h1>DAPRS API</h1>
//     <p>DN42 AutoPeering System</p>
//     <ul>
//         <li><a href="/api/login">Login</a></li>
//         <li><a href="/api/logout">Logout</a></li>
//         <li><a href="/api/me">My Info</a></li>
//         <li><a href="/api/nodes">View Nodes</a></li>
//         <li><a href="/api/peers">View Peers</a></li>
//     </ul>
// </body>
// </html>
// "#;

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = cli::Cli::parse();

    let config = load_config(&cli.config)?;
    let db = sled::open("daprs.db")?;
    let state = AppState { config, db };

    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind(&state.config.server.bind).await?;

    // ratelimit
    let rate_limit_config = state.config.server.rate_limit.as_ref();

    let auth_limiter: Option<Arc<RateLimiter>> =
        rate_limit_config.and_then(|rl| rl.auth.as_ref()).map(|w| {
            Arc::new(RateLimiter::new(
                Duration::from_secs(w.window_secs),
                w.max_requests,
            ))
        });

    let api_limiter: Option<Arc<RateLimiter>> =
        rate_limit_config.and_then(|rl| rl.api.as_ref()).map(|w| {
            Arc::new(RateLimiter::new(
                Duration::from_secs(w.window_secs),
                w.max_requests,
            ))
        });

    // public api
    let mut auth_routes = axum::Router::new()
        .route("/api/login", get(oauth::login))
        .route("/api/login/callback", get(oauth::login_callback))
        .route("/api/me", get(oauth::me))
        .route("/api/logout", post(oauth::logout));
    if let Some(limiter) = auth_limiter {
        auth_routes = auth_routes.route_layer(middleware::from_fn_with_state(
            limiter,
            rate_limiter::rate_limit_middleware,
        ));
    }

    // protected api
    let mut protected_api = axum::Router::new()
        .route("/api/nodes", get(handler::get_nodes))
        .route("/api/peers", get(handler::get_peers))
        .route("/api/peering", post(handler::post_peering))
        .route("/api/peering/{node}", delete(handler::delete_peering_queue))
        .route("/api/modify", post(handler::post_modify))
        .route("/api/modify/{node}", delete(handler::delete_modify_queue))
        .route("/api/remove", post(handler::post_remove))
        .route("/api/remove/{node}", delete(handler::delete_remove_queue))
        .route("/api/cmd", post(handler::post_cmd))
        .route("/api/peer/{node}/info", get(handler::get_peer_info))
        // 管理员 API
        .route("/api/admin/pending", get(handler::get_pending_requests))
        .route(
            "/api/admin/pending/{id}/approve",
            post(handler::approve_request),
        )
        .route(
            "/api/admin/pending/{id}/reject",
            post(handler::reject_request),
        )
        .route("/api/admin/peers", get(handler::get_all_peers))
        .route("/api/admin/peer/modify", post(handler::admin_modify_peer))
        .route("/api/admin/peer/delete", post(handler::admin_delete_peer))
        .route("/api/admin/check", get(handler::check_admin))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            oauth::require_auth_middleware,
        ));

    if let Some(limiter) = api_limiter {
        protected_api = protected_api.route_layer(middleware::from_fn_with_state(
            limiter,
            rate_limiter::rate_limit_middleware,
        ));
    }

    let mut app = axum::Router::new()
        .merge(protected_api)
        .merge(auth_routes)
        .layer(TraceLayer::new_for_http());

    // CORS 配置

    if let Some(origin) = &state.config.web.frontend_origin {
        let cors = CorsLayer::new()
            .allow_origin(
                origin
                    .parse::<axum::http::HeaderValue>()
                    .map_err(|_| anyhow::anyhow!("invalid frontend_origin"))?,
            )
            .allow_credentials(true)
            .allow_methods([
                axum::http::Method::GET,
                axum::http::Method::POST,
                axum::http::Method::DELETE,
                axum::http::Method::OPTIONS,
            ])
            .allow_headers([
                axum::http::header::AUTHORIZATION,
                axum::http::header::CONTENT_TYPE,
                axum::http::header::HeaderName::from_static("x-api-token"),
            ]);
        app = app.layer(cors);
    }

    let app = app.with_state(state);

    info!(
        "Server is running on http://{}",
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
