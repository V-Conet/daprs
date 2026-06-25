//! OAuth 登录中间态仓储（PKCE/CSRF，TTL 600s）

use sqlx::{FromRow, SqlitePool};

use shared::AppError;

use super::map_db_err;

/// 登录中间态行
#[derive(Debug, Clone, FromRow)]
pub struct OAuthLoginState {
    pub state: String,
    pub pkce_verifier: String,
    pub provider: String,
    pub created_at: i64,
    pub expires_at: i64,
}

/// 创建登录中间态
pub async fn create(
    pool: &SqlitePool,
    state: &str,
    pkce_verifier: &str,
    provider: &str,
    expires_at: i64,
) -> Result<(), AppError> {
    let now = super::now_unix_secs();
    sqlx::query(
        "INSERT INTO oauth_login_states (state, pkce_verifier, provider, created_at, expires_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(state)
    .bind(pkce_verifier)
    .bind(provider)
    .bind(now)
    .bind(expires_at)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(())
}

/// 消费登录中间态（删除并返回），校验过期
///
/// 过期或不存在返回 None
pub async fn consume(pool: &SqlitePool, state: &str) -> Result<Option<OAuthLoginState>, AppError> {
    let row = sqlx::query_as::<_, OAuthLoginState>(
        "DELETE FROM oauth_login_states WHERE state = ? RETURNING *",
    )
    .bind(state)
    .fetch_optional(pool)
    .await
    .map_err(map_db_err)?;

    if let Some(ref r) = row && super::now_unix_secs() > r.expires_at {
        return Ok(None);
    }
    Ok(row)
}

/// 清理过期登录中间态
pub async fn cleanup_expired(pool: &SqlitePool) -> Result<u64, AppError> {
    let now = super::now_unix_secs();
    let res = sqlx::query("DELETE FROM oauth_login_states WHERE expires_at <= ?")
        .bind(now)
        .execute(pool)
        .await
        .map_err(map_db_err)?;
    Ok(res.rows_affected())
}
