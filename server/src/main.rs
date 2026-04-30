use std::path::Path;

use anyhow::{Context, Result};
use axum::middleware;
use axum::response::Html;
use axum::routing::{get, post};
use clap::Parser;
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

use crate::api::{handler, oauth};
use crate::config::{AppState, Config};

mod api;
mod cli;
mod config;

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
    let db = sled::open("daprs-server.db")?;
    let state = AppState { config, db };

    tracing_subscriber::fmt::init();

    let listener = TcpListener::bind(&state.config.server.bind).await?;

    //for index
    let index = Html(
        r#"
        <html>
            <head>
                <title>DAPRS API</title>
            </head>
            <body>
            <center>
                <h1>DAPRS API</h1>
                <p><a href="/api/login">Login</a></p>
                <p><a href="/api/logout">Logout</a></p>
                <p><a href="/api/me">My Info</a></p>
                <p><a href="/api/nodes">View Nodes</a></p>
                <p><a href="/api/peers">View Peers</a></p>
            </center>
            </body>
        </html>
        "#
        .to_string(),
    );

    // todo: API_TOKEN auth between server and agent
    let protected_api = axum::Router::new()
        .route("/", get(|| async { index }))
        .route("/api/nodes", get(handler::get_nodes))
        .route("/api/peers", get(handler::get_peers))
        .route("/api/peering", post(handler::post_peering))
        .route("/api/modify", post(handler::post_modify))
        .route("/api/remove", post(handler::post_remove))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            oauth::require_auth_middleware,
        ));

    let mut app = axum::Router::new()
        .merge(protected_api)
        .route("/api/login", get(oauth::login))
        .route("/api/login/callback", get(oauth::login_callback))
        .route("/api/me", get(oauth::me))
        .route("/api/logout", post(oauth::logout))
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    if let Some(web) = state.config.web.as_ref() {
        if let Some(origin) = web.frontend_origin.as_ref() {
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
                    axum::http::Method::OPTIONS,
                ])
                .allow_headers([
                    axum::http::header::AUTHORIZATION,
                    axum::http::header::CONTENT_TYPE,
                    axum::http::header::HeaderName::from_static("x-api-token"),
                ]);
            app = app.layer(cors);
        }
    }

    let app = app.with_state(state);

    info!(
        "Server is running on http://{}",
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
