// Write Wireguard & Bird configuration files

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
};
use serde::{Deserialize, Serialize};

use crate::{api::require_api_token, config::Config};

#[derive(Serialize, Deserialize)]
pub struct WbConfig {
    pub wg_config: String,
    pub bird_config: String,
}

// TODO! Peering ASN should be passed in request,
// therefore, the config file path can be generated
pub async fn create_config(
    headers: HeaderMap,
    State(config_state): State<Config>,
    Json(config): Json<WbConfig>,
) -> Result<StatusCode, StatusCode> {
    require_api_token(&headers, &config_state)?;
    // write_config_files(&config_state, &config)?;

    // todo: this is a validation in C/S
    println!(
        "config writen:\n{}\n{}",
        config.wg_config, config.bird_config
    );

    //return prober status code
    Ok(StatusCode::OK)
}

pub async fn modify_config(
    headers: HeaderMap,
    State(config_state): State<Config>,
    Json(config): Json<WbConfig>,
) -> Result<StatusCode, StatusCode> {
    require_api_token(&headers, &config_state)?;
    // write_config_files(&config_state, &config)?;

    // todo: this is a validation in C/S
    println!(
        "config writen:\n{}\n{}",
        config.wg_config, config.bird_config
    );
    Ok(StatusCode::OK)
}

// whather above func will handle both create and modify or not
// delect func will only implement in this
pub async fn delete_config(
    headers: HeaderMap,
    State(config_state): State<Config>,
) -> Result<StatusCode, StatusCode> {
    require_api_token(&headers, &config_state)?;

    // print!("deleted config: {}, {}",config_state.agent.wg_path,config_state.agent.bird_path);

    Ok(StatusCode::OK)
}
