//! Peering 缓存仓储（非真相源，以 agent 文件系统为准）

use shared::PeeringPayload;
use sqlx::{FromRow, SqlitePool};

use shared::AppError;

use super::map_db_err;

/// 缓存行
#[derive(Debug, Clone, FromRow)]
pub struct PeerCacheRow {
    pub id: i64,
    pub user_asn: i64,
    pub node: String,
    pub payload: String,
    pub status: String,
    pub last_synced_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

impl PeerCacheRow {
    pub fn payload_value(&self) -> Result<PeeringPayload, AppError> {
        serde_json::from_str(&self.payload)
            .map_err(|e| AppError::InternalError(format!("payload parse: {e}")))
    }
}

/// 写入/更新为 active
pub async fn upsert_active(
    pool: &SqlitePool,
    user_asn: i64,
    node: &str,
    payload: &str,
) -> Result<(), AppError> {
    let now = super::now_unix_secs();
    sqlx::query(
        "INSERT INTO peer_cache (user_asn, node, payload, status, last_synced_at, created_at, updated_at)
         VALUES (?, ?, ?, 'active', ?, ?, ?)
         ON CONFLICT(user_asn, node) DO UPDATE SET
            payload = excluded.payload,
            status = 'active',
            last_synced_at = excluded.last_synced_at,
            updated_at = excluded.updated_at",
    )
    .bind(user_asn)
    .bind(node)
    .bind(payload)
    .bind(now)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(())
}

/// 更新 payload（modify 成功后）
pub async fn update_payload(
    pool: &SqlitePool,
    user_asn: i64,
    node: &str,
    payload: &str,
) -> Result<(), AppError> {
    let now = super::now_unix_secs();
    sqlx::query(
        "UPDATE peer_cache
            SET payload = ?, status = 'active', last_synced_at = ?, updated_at = ?
          WHERE user_asn = ? AND node = ?",
    )
    .bind(payload)
    .bind(now)
    .bind(now)
    .bind(user_asn)
    .bind(node)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(())
}

/// 标记为已移除
pub async fn mark_removed(pool: &SqlitePool, user_asn: i64, node: &str) -> Result<(), AppError> {
    let now = super::now_unix_secs();
    sqlx::query(
        "UPDATE peer_cache SET status = 'removed', last_synced_at = ?, updated_at = ?
          WHERE user_asn = ? AND node = ?",
    )
    .bind(now)
    .bind(now)
    .bind(user_asn)
    .bind(node)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(())
}

/// 用户的 active peer 列表
pub async fn list_active_for_user(
    pool: &SqlitePool,
    user_asn: i64,
) -> Result<Vec<PeerCacheRow>, AppError> {
    sqlx::query_as::<_, PeerCacheRow>(
        "SELECT * FROM peer_cache WHERE user_asn = ? AND status = 'active' ORDER BY node ASC",
    )
    .bind(user_asn)
    .fetch_all(pool)
    .await
    .map_err(map_db_err)
}
