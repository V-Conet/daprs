//! 数据库仓储层
//!
//! 封装所有 SQLite 访问逻辑，按实体拆分为子模块。
//! handler/oauth 层只调用本模块的函数，不直接写 SQL。
//!
//! 约定：
//! - 时间戳统一 unix 秒（i64）
//! - ASN 以 i64 存储（SQLite INTEGER），在调用层与 u32 互转
//! - JSON 字段（payload、userinfo）以 TEXT 存储序列化结果
//!
//! 仓储层对外提供完整 API 表面，部分成员当前未被调用但保留供后续使用。
#![allow(dead_code)]

use std::time::{SystemTime, UNIX_EPOCH};

use shared::AppError;

pub mod audit;
pub mod oauth_states;
pub mod peer_cache;
pub mod peering_requests;
pub mod sessions;
pub mod users;

/// 连接 SQLite 并启用外键约束
pub async fn connect(db_path: &str) -> Result<sqlx::SqlitePool, AppError> {
    // 兼容 "sqlite:path?..." / "path" 两种写法，取文件名部分
    let filename = db_path.strip_prefix("sqlite:").unwrap_or(db_path);
    let filename = filename.split('?').next().unwrap_or(filename);

    let options = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(filename)
        .create_if_missing(true)
        .foreign_keys(true);

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(options)
        .await
        .map_err(map_db_err)?;

    Ok(pool)
}

/// 运行内嵌迁移
pub async fn migrate(pool: &sqlx::SqlitePool) -> Result<(), AppError> {
    sqlx::migrate!("../migrations")
        .run(pool)
        .await
        .map_err(|e| AppError::InternalError(format!("migration error: {e}")))?;
    Ok(())
}

/// 当前 unix 秒
pub fn now_unix_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// 将 sqlx 错误映射为 AppError
pub(crate) fn map_db_err(e: sqlx::Error) -> AppError {
    AppError::InternalError(format!("db error: {e}"))
}
