//! Agent 工具模块
//!
//! 命令执行、认证等功能

use axum::{
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};

use crate::config::Config;
use shared::AppError;

/// 命令执行结果
#[derive(Debug)]
pub struct CmdOutput {
    /// 是否成功
    pub success: bool,
    /// 标准输出或错误输出
    pub text: String,
}

/// 执行外部命令（异步，带超时）
///
/// 超时时间从配置读取，默认 15 秒。
pub async fn run_cmd(config: &Config, bin: &str, args: &[&str]) -> CmdOutput {
    use tokio::process::Command;
    use tokio::time::{Duration, timeout};

    let timeout_secs = config.agent.timeout.unwrap_or(15);
    let result = timeout(
        Duration::from_secs(timeout_secs),
        Command::new(bin).args(args).output(),
    )
    .await;

    match result {
        Ok(Ok(output)) => {
            let text = if output.status.success() {
                String::from_utf8_lossy(&output.stdout).into_owned()
            } else {
                String::from_utf8_lossy(&output.stderr).into_owned()
            };
            CmdOutput {
                success: output.status.success(),
                text,
            }
        }
        Ok(Err(e)) => CmdOutput {
            success: false,
            text: format!("failed to execute {}: {}", bin, e),
        },
        Err(_) => CmdOutput {
            success: false,
            text: format!("command timed out after {} seconds", timeout_secs),
        },
    }
}

/// 认证中间件
pub async fn auth_middleware(
    State(config): State<Config>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    verify_token(&request.headers(), &config)?;
    Ok(next.run(request).await)
}

/// 验证 API Token
///
/// 空 token 拒绝所有请求，防止未配置认证时端点暴露。
pub fn verify_token(headers: &HeaderMap, config: &Config) -> Result<(), AppError> {
    if config.agent.api_token.is_empty() {
        return Err(AppError::Unauthorized);
    }

    let header_value = headers
        .get("x-api-token")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    if header_value != config.agent.api_token {
        return Err(AppError::Unauthorized);
    }

    Ok(())
}

/// 从请求头解析 ASN
pub fn parse_asn_header(headers: &HeaderMap) -> Result<u32, AppError> {
    let asn: u32 = headers
        .get("asn")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse().ok())
        .ok_or(AppError::BadRequest("invalid ASN".into()))?;

    if asn == 0 {
        return Err(AppError::BadRequest("invalid ASN: must be positive".into()));
    }

    Ok(asn)
}
