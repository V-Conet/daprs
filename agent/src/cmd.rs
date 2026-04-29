use std::process::Command;

use axum::extract::Path;

fn run_ping(args: &[&str], target: String) -> Result<String, axum::http::StatusCode> {
    let output = Command::new("ping")
        .args(args)
        .arg(target.to_lowercase())
        .output()
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(axum::http::StatusCode::BAD_REQUEST)
    }
}

pub async fn get_ping(Path(arg): Path<String>) -> Result<String, axum::http::StatusCode> {
    run_ping(&["-c", "5", "-w", "6"], arg)
}

pub async fn get_ping4(Path(arg): Path<String>) -> Result<String, axum::http::StatusCode> {
    run_ping(&["-4", "-c", "5", "-w", "6"], arg)
}
pub async fn get_ping6(Path(arg): Path<String>) -> Result<String, axum::http::StatusCode> {
    run_ping(&["-6", "-c", "5", "-w", "6"], arg)
}
