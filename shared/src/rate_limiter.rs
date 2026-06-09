//! 通用速率限制中间件
//!
//! 基于滑动窗口算法的内存速率限制器，可用于 axum 服务端和 agent 端。

use std::collections::{HashMap, VecDeque};
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use axum::{
    extract::{ConnectInfo, Request, State},
    middleware::Next,
    response::Response,
};
use serde::{Deserialize, Serialize};

use crate::AppError;

/// 清理间隔
const CLEANUP_INTERVAL: Duration = Duration::from_secs(60);

/// 速率限制窗口配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitWindow {
    /// 时间窗口（秒）
    pub window_secs: u64,
    /// 窗口内最大请求数
    pub max_requests: u64,
}

/// 顶层速率限制配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    /// 认证端点限速（登录、回调等）
    #[serde(default)]
    pub auth: Option<RateLimitWindow>,
    /// 通用 API 端点限速
    #[serde(default)]
    pub api: Option<RateLimitWindow>,
}

// RateLimiter
/// 内存滑动窗口速率限制器
///
/// 使用 `Arc<RateLimiter>` 在多个路由层之间共享同一个实例。
pub struct RateLimiter {
    window: Duration,
    max_requests: u64,
    inner: Mutex<Inner>,
}

struct Inner {
    clients: HashMap<String, VecDeque<Instant>>,
    last_cleanup: Instant,
}

impl RateLimiter {
    /// 创建新的速率限制器
    ///
    /// * `window` - 滑动时间窗口
    /// * `max_requests` - 窗口内允许的最大请求数（达到此数后拒绝）
    pub fn new(window: Duration, max_requests: u64) -> Self {
        Self {
            window,
            max_requests,
            inner: Mutex::new(Inner {
                clients: HashMap::new(),
                last_cleanup: Instant::now(),
            }),
        }
    }

    /// 检查客户端是否被允许发送请求
    ///
    /// 返回 `Ok(())` 表示未超限，`Err(AppError::TooManyRequests)` 表示被限速。
    pub fn check(&self, client_id: &str) -> Result<(), AppError> {
        let mut inner = self.inner.lock().unwrap();
        let now = Instant::now();
        let cutoff = now - self.window;

        let timestamps = inner.clients.entry(client_id.to_string()).or_default();

        // 淘汰窗口外的旧时间戳
        while timestamps.front().is_some_and(|t| *t < cutoff) {
            timestamps.pop_front();
        }

        if timestamps.len() as u64 >= self.max_requests {
            return Err(AppError::TooManyRequests);
        }

        timestamps.push_back(now);

        // 定期全局清理
        if now - inner.last_cleanup > CLEANUP_INTERVAL {
            inner.clients.retain(|_, ts| {
                while ts.front().is_some_and(|t| *t < cutoff) {
                    ts.pop_front();
                }
                !ts.is_empty()
            });
            inner.last_cleanup = now;
        }

        Ok(())
    }
}

// 中间件
/// 限速中间件
///
/// 从请求中提取客户端标识，检查速率限制。
///
/// # 用法
///
/// ```ignore
/// use std::sync::Arc;
/// let limiter = Arc::new(RateLimiter::new(window, max));
/// router.route_layer(middleware::from_fn_with_state(
///     limiter,
///     rate_limiter::rate_limit_middleware,
/// ));
/// ```
pub async fn rate_limit_middleware(
    State(limiter): State<std::sync::Arc<RateLimiter>>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let client_id = extract_client_id(&request);
    limiter.check(&client_id)?;
    Ok(next.run(request).await)
}

// 客户端识别
/// 从请求中提取客户端标识
///
/// 按优先级从以下来源提取：
/// 1. `X-Forwarded-For` 头（取第一个 IP）
/// 2. `X-Real-IP` 头
/// 3. `ConnectInfo<SocketAddr>` 扩展（需要 `into_make_service_with_connect_info`）
/// 4. 回退到 `"unknown"`
fn extract_client_id(request: &Request) -> String {
    if let Some(ip) = request
        .headers()
        .get("x-forwarded-for")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.split(',').next())
        .map(|s| s.trim().to_string())
    {
        return ip;
    }

    if let Some(ip) = request
        .headers()
        .get("x-real-ip")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
    {
        return ip;
    }

    if let Some(ConnectInfo(addr)) = request.extensions().get::<ConnectInfo<SocketAddr>>() {
        return addr.ip().to_string();
    }

    "unknown".to_string()
}
// tests
#[cfg(test)]
mod tests {
    use super::*;
    use axum::{body::Body, http::Request};

    #[test]
    fn test_allows_requests_under_limit() {
        let limiter = RateLimiter::new(Duration::from_secs(10), 3);
        assert!(limiter.check("client-a").is_ok());
        assert!(limiter.check("client-a").is_ok());
        assert!(limiter.check("client-a").is_ok());
    }

    #[test]
    fn test_rejects_at_limit() {
        let limiter = RateLimiter::new(Duration::from_secs(10), 2);
        assert!(limiter.check("client-b").is_ok());
        assert!(limiter.check("client-b").is_ok());
        assert!(limiter.check("client-b").is_err());
    }

    #[test]
    fn test_independent_client_buckets() {
        let limiter = RateLimiter::new(Duration::from_secs(10), 1);
        assert!(limiter.check("client-a").is_ok());
        assert!(limiter.check("client-a").is_err());
        assert!(limiter.check("client-b").is_ok());
    }

    #[test]
    fn test_window_expiry() {
        let limiter = RateLimiter::new(Duration::from_millis(50), 1);
        assert!(limiter.check("client-c").is_ok());
        assert!(limiter.check("client-c").is_err());
        std::thread::sleep(Duration::from_millis(60));
        assert!(limiter.check("client-c").is_ok());
    }

    #[test]
    fn test_stale_timestamps_pruned_on_check() {
        let limiter = RateLimiter::new(Duration::from_millis(10), 5);
        limiter.check("test-client").unwrap();
        // 等待窗口过期后，旧时间戳被淘汰
        std::thread::sleep(Duration::from_millis(30));
        // 过期后请求应该被允许（旧时间戳已被淘汰）
        assert!(limiter.check("test-client").is_ok());
    }

    #[test]
    fn test_extract_x_forwarded_for() {
        let req = Request::builder()
            .header("x-forwarded-for", "10.0.0.1, 10.0.0.2")
            .body(Body::empty())
            .unwrap();
        assert_eq!(extract_client_id(&req), "10.0.0.1");
    }

    #[test]
    fn test_extract_x_real_ip() {
        let req = Request::builder()
            .header("x-real-ip", "10.0.0.3")
            .body(Body::empty())
            .unwrap();
        assert_eq!(extract_client_id(&req), "10.0.0.3");
    }

    #[test]
    fn test_extract_x_forwarded_for_precedes_x_real_ip() {
        let req = Request::builder()
            .header("x-forwarded-for", "10.0.0.1")
            .header("x-real-ip", "10.0.0.3")
            .body(Body::empty())
            .unwrap();
        assert_eq!(extract_client_id(&req), "10.0.0.1");
    }

    #[test]
    fn test_extract_fallback_unknown() {
        let req = Request::builder().body(Body::empty()).unwrap();
        assert_eq!(extract_client_id(&req), "unknown");
    }
}
