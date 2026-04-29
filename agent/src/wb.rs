// Write Wireguard & Bird configuration files

use std::path::Path;

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

// TODO!
// probably should we replac Json<bool> with StatusCode?
// to better indicate the result of the operation, like 200 for success, 400 for bad request, 500 for server error, etc.


// or should even the modify action could be handled by this func?
// 'casze we only pass the String for both configs,
// if so, the full modified config String will be given by server
// we can just take it, veirfy it wont contain bad code, then writen to fs.
pub async fn create_config(
    headers: HeaderMap,
    State(config_state): State<Config>,
    Json(config): Json<WbConfig>,
) -> Result<axum::http::StatusCode, axum::http::StatusCode> {
    require_api_token(&headers, &config_state)?;
    // write_config_files(&config_state, &config)?;

    // todo: this is a validation in C/S
    println!("config writen:\n{}\n{}", config.wg_config, config.bird_config);

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
    println!("config writen:\n{}\n{}", config.wg_config, config.bird_config);
    Ok(StatusCode::OK)
}


// whather above func will handle both create and modify or not
// delect func will only implement in this
pub async fn delete_config(
    headers: HeaderMap,
    State(config_state): State<Config>,
) -> Result<StatusCode, StatusCode> {
    require_api_token(&headers, &config_state)?;

    print!("deleted config: {}, {}",config_state.agent.wg_config_path,config_state.agent.bird_config_path);

    Ok(StatusCode::OK)

    //todo!();
    // remove_file_if_exists(&config_state.agent.wg_config_path)?;
    // remove_file_if_exists(&config_state.agent.bird_config_path)?;
    // Ok(Json(true))
}

// fn write_config_files(config_state: &Config, config: &WbConfig) -> Result<(), StatusCode> {
//     write_file_with_parent(&config_state.agent.wg_config_path, &config.wg_config)?;
//     write_file_with_parent(&config_state.agent.bird_config_path, &config.bird_config)?;
//     Ok(())
// }

// fn write_file_with_parent(path: &str, content: &str) -> Result<(), StatusCode> {
//     let p = Path::new(path);
//     if let Some(parent) = p.parent() {
//         if !parent.as_os_str().is_empty() {
//             std::fs::create_dir_all(parent).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
//         }
//     }

//     std::fs::write(p, content).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
//     Ok(())
// }

// fn remove_file_if_exists(path: &str) -> Result<(), StatusCode> {
//     let p = Path::new(path);
//     match std::fs::remove_file(p) {
//         Ok(_) => Ok(()),
//         Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
//         Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
//     }
// }