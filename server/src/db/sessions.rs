//! 会话仓储

use sqlx::{FromRow, SqlitePool};

use shared::AppError;

use super::map_db_err;

/// 会话行
#[derive(Debug, Clone, FromRow)]
pub struct Session {
    pub id: String,
    pub user_asn: i64,
    pub issued_at: i64,
    pub expires_at: i64,
    pub created_at: i64,
}

/// 创建会话
pub async fn create(
    pool: &SqlitePool,
    id: &str,
    user_asn: i64,
    issued_at: i64,
    expires_at: i64,
) -> Result<(), AppError> {
    let now = super::now_unix_secs();
    sqlx::query(
        "INSERT INTO sessions (id, user_asn, issued_at, expires_at, created_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(user_asn)
    .bind(issued_at)
    .bind(expires_at)
    .bind(now)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(())
}

/// 获取会话
pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<Session>, AppError> {
    sqlx::query_as::<_, Session>("SELECT * FROM sessions WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(map_db_err)
}

/// 删除会话
pub async fn delete(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    sqlx::query("DELETE FROM sessions WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await
        .map_err(map_db_err)?;
    Ok(())
}

/// 清理过期会话
pub async fn cleanup_expired(pool: &SqlitePool) -> Result<u64, AppError> {
    let now = super::now_unix_secs();
    let res = sqlx::query("DELETE FROM sessions WHERE expires_at <= ?")
        .bind(now)
        .execute(pool)
        .await
        .map_err(map_db_err)?;
    Ok(res.rows_affected())
}
