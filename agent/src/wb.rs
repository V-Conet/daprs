// Write Wireguard & Bird configuration files

use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode, header},
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

    // FIXME: ALWAYS VALIDATE BEFORE WRITING
    let asn = headers.get("asn").unwrap().to_str().unwrap();
    let wg_path = format!("{}/dn42-{}.conf", config_state.agent.wg_path, asn);
    match write_wg(&config.wg_config, &wg_path) {
        Ok(_) => {
            // debug
            println!("config writen: {}", wg_path);

            Ok(StatusCode::OK)
        }
        Err(e) => {
            eprintln!("Error writing config: {:?}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }

    // ...
}

pub async fn modify_config(
    headers: HeaderMap,
    State(config_state): State<Config>,
    Json(config): Json<WbConfig>,
) -> Result<StatusCode, StatusCode> {
    require_api_token(&headers, &config_state)?;

    todo!();
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

fn write_wg(conf: &String, path: &str) -> Result<(), std::io::Error> {
    std::fs::write(path, conf)?;

    Ok(())
}

fn delete_conf(path: &str) -> Result<(), std::io::Error> {
    std::fs::remove_file(path)?;

    Ok(())
}
