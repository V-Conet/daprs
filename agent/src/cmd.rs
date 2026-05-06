use std::process::Command;

use axum::{Json, http::StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "op", content = "args")]
#[serde(rename_all = "lowercase")]
pub enum Cmd {
    /// Ping CMD, target:
    /// ```
    /// Usage:
    ///     /ping [options] <dest>
    /// Options:
    ///     -c <count>    number of echo requests to send (default: 4)
    ///     -s <size>     use <size> as number of data bytes to be sent
    ///     -F            do not fragment packets
    ///     -t <timeout>  time to wait for response (default: 2000ms)
    ///     -i <interval> ms between sending each packet (default: 500ms)
    ///     -4            use IPv4
    ///     -6            use IPv6
    /// ```
    Ping {
        protocol: Option<u16>,
        count: Option<u16>,
        size: Option<u16>,
        dfrag: Option<bool>,
        timeout: Option<u32>,
        target: String,
    },
    /// Dig CMD, target:
    /// e.g. 1
    /// ```
    /// Usage: /dig domain {type} {@dns_server}
    /// 用法：/dig domain {type} {@dns_server}
    /// Only accept following types
    /// 只接受以下类型的查询
    /// ANY, A, AAAA, CNAME, MX, TXT, NS, SOA, SRV, PTR
    /// ```
    Dig {
        qtype: Option<String>,
        /// included port
        server: Option<String>,
        target: String,
    },
    /// Raw `wg show` output for a single interface.
    WgShow {
        interface: String,
    },
    /// Raw `birdc show protocol` output for a single protocol.
    BirdShow {
        protocol: String,
    },
}
#[derive(Serialize, Deserialize)]
pub struct CmdRequest {
    pub cmd: Cmd,
}
pub async fn cmd_handler(Json(paylod): Json<CmdRequest>) -> Result<String, StatusCode> {
    // TODO: all args from request, require validation
    // so far, this is a proto
    match paylod.cmd {
        Cmd::Ping {
            protocol,
            count,
            size,
            dfrag,
            timeout,
            target,
        } => {
            // basic args
            let count = count.unwrap_or(4).to_string();
            let timeout = timeout.unwrap_or(2000).to_string();

            let mut args = vec!["-c".to_string(), count, "-w".to_string(), timeout];

            if let Some(size) = size {
                args.push("-s".to_string());
                args.push(size.to_string());
            }
            if dfrag.unwrap_or(false) {
                args.push("-F".to_string());
            }
            if let Some(protocol) = protocol {
                match protocol {
                    4 => args.push("-4".to_string()),
                    6 => args.push("-6".to_string()),
                    _ => return Err(StatusCode::BAD_REQUEST),
                }
            }
            run_ping(args, target).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        }
        // TODO: DANGER: VALIDATION REQUIRED
        // so far, proto
        Cmd::Dig {
            qtype,
            server,
            target,
        } => {
            let mut args = vec![qtype.unwrap_or("A".to_string())];

            if let Some(server) = server {
                args.push(format!("@{}", server));
            }
            run_dig(args, target).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
        }
        Cmd::WgShow { interface } => run_cmd("wg", &["show", &interface])
            .map(|output| output.text)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR),
        Cmd::BirdShow { protocol } => run_cmd("birdc", &["show", "protocol", &protocol])
            .map(|output| output.text)
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn run_ping(args: Vec<String>, target: String) -> Result<String, axum::http::StatusCode> {
    let mut cmd_args = args;
    cmd_args.push(target.to_lowercase());
    let cmd_args = cmd_args.iter().map(String::as_str).collect::<Vec<_>>();

    run_cmd("ping", &cmd_args)
        .and_then(|output| {
            if output.success {
                Ok(output.text)
            } else {
                Err(axum::http::StatusCode::BAD_REQUEST)
            }
        })
}

fn run_dig(args: Vec<String>, target: String) -> Result<String, axum::http::StatusCode> {
    let mut cmd_args = args;
    cmd_args.push(target.to_lowercase());
    let cmd_args = cmd_args.iter().map(String::as_str).collect::<Vec<_>>();

    run_cmd("dig", &cmd_args)
        .and_then(|output| {
            if output.success {
                Ok(output.text)
            } else {
                Err(axum::http::StatusCode::BAD_REQUEST)
            }
        })
}

struct CmdOutput {
    success: bool,
    text: String,
}

fn run_cmd(bin: &str, args: &[&str]) -> Result<CmdOutput, axum::http::StatusCode> {
    let output = Command::new(bin)
        .args(args)
        .output()
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)?;

    let text = if output.status.success() {
        String::from_utf8_lossy(&output.stdout).into_owned()
    } else {
        String::from_utf8_lossy(&output.stderr).into_owned()
    };

    Ok(CmdOutput {
        success: output.status.success(),
        text,
    })
}
