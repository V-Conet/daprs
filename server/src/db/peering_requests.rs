//! Peering 请求仓储

use serde::{Deserialize, Serialize};
use shared::PeeringPayload;
use sqlx::{FromRow, SqlitePool};

use shared::AppError;

use super::map_db_err;

/// 请求动作
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Action {
    Create,
    Modify,
    Delete,
}

impl Action {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Create => "create",
            Self::Modify => "modify",
            Self::Delete => "delete",
        }
    }

    fn from_db(s: &str) -> Result<Self, AppError> {
        match s {
            "create" => Ok(Self::Create),
            "modify" => Ok(Self::Modify),
            "delete" => Ok(Self::Delete),
            other => Err(AppError::InternalError(format!("unknown action: {other}"))),
        }
    }
}

/// 请求状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Pending,
    Approved,
    Rejected,
    Dispatched,
    Succeeded,
    Failed,
    Expired,
}

impl Status {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Approved => "approved",
            Self::Rejected => "rejected",
            Self::Dispatched => "dispatched",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Expired => "expired",
        }
    }

    fn from_db(s: &str) -> Result<Self, AppError> {
        match s {
            "pending" => Ok(Self::Pending),
            "approved" => Ok(Self::Approved),
            "rejected" => Ok(Self::Rejected),
            "dispatched" => Ok(Self::Dispatched),
            "succeeded" => Ok(Self::Succeeded),
            "failed" => Ok(Self::Failed),
            "expired" => Ok(Self::Expired),
            other => Err(AppError::InternalError(format!("unknown status: {other}"))),
        }
    }

    /// 是否终态
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Rejected | Self::Succeeded | Self::Failed | Self::Expired
        )
    }
}

/// Peering 请求行
#[derive(Debug, Clone, FromRow)]
pub struct PeeringRequest {
    pub id: String,
    pub user_asn: i64,
    pub node: String,
    pub action: String,
    pub status: String,
    pub require_approval: i64,
    pub payload: Option<String>,
    pub reviewer_asn: Option<i64>,
    pub reviewed_at: Option<i64>,
    pub dispatched_at: Option<i64>,
    pub result_error: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub expires_at: Option<i64>,
}

impl PeeringRequest {
    pub fn action_enum(&self) -> Result<Action, AppError> {
        Action::from_db(&self.action)
    }

    pub fn status_enum(&self) -> Result<Status, AppError> {
        Status::from_db(&self.status)
    }

    /// 解析 payload JSON
    pub fn payload_value(&self) -> Result<Option<PeeringPayload>, AppError> {
        match &self.payload {
            Some(s) => Ok(Some(serde_json::from_str(s).map_err(|e| {
                AppError::InternalError(format!("payload parse: {e}"))
            })?)),
            None => Ok(None),
        }
    }

    pub fn require_approval(&self) -> bool {
        self.require_approval != 0
    }
}

/// 创建请求（初始 status=pending）
pub async fn create(
    pool: &SqlitePool,
    id: &str,
    user_asn: i64,
    node: &str,
    action: Action,
    require_approval: bool,
    payload: Option<&str>,
    expires_at: Option<i64>,
) -> Result<(), AppError> {
    let now = super::now_unix_secs();
    sqlx::query(
        "INSERT INTO peering_requests
            (id, user_asn, node, action, status, require_approval, payload,
             created_at, updated_at, expires_at)
         VALUES (?, ?, ?, ?, 'pending', ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(user_asn)
    .bind(node)
    .bind(action.as_str())
    .bind(if require_approval { 1 } else { 0 })
    .bind(payload)
    .bind(now)
    .bind(now)
    .bind(expires_at)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(())
}

/// 创建一条终态请求（用于管理员直接操作的历史记录）
pub async fn create_terminal(
    pool: &SqlitePool,
    id: &str,
    user_asn: i64,
    node: &str,
    action: Action,
    payload: Option<&str>,
    status: Status,
    reviewer_asn: Option<i64>,
    error: Option<&str>,
) -> Result<(), AppError> {
    let now = super::now_unix_secs();
    let reviewed_at = if status.is_terminal() {
        Some(now)
    } else {
        None
    };
    sqlx::query(
        "INSERT INTO peering_requests
            (id, user_asn, node, action, status, require_approval, payload,
             reviewer_asn, reviewed_at, dispatched_at, result_error,
             created_at, updated_at, expires_at)
         VALUES (?, ?, ?, ?, ?, 0, ?, ?, ?, ?, ?, ?, ?, NULL)",
    )
    .bind(id)
    .bind(user_asn)
    .bind(node)
    .bind(action.as_str())
    .bind(status.as_str())
    .bind(payload)
    .bind(reviewer_asn)
    .bind(reviewed_at)
    .bind(if status == Status::Succeeded || status == Status::Failed {
        Some(now)
    } else {
        None
    })
    .bind(error)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(())
}

/// 获取单个请求
pub async fn get(pool: &SqlitePool, id: &str) -> Result<Option<PeeringRequest>, AppError> {
    sqlx::query_as::<_, PeeringRequest>("SELECT * FROM peering_requests WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await
        .map_err(map_db_err)
}

/// 用户未过期的 pending 请求
pub async fn list_pending_for_user(
    pool: &SqlitePool,
    user_asn: i64,
) -> Result<Vec<PeeringRequest>, AppError> {
    let now = super::now_unix_secs();
    sqlx::query_as::<_, PeeringRequest>(
        "SELECT * FROM peering_requests
         WHERE user_asn = ? AND status = 'pending'
           AND (expires_at IS NULL OR expires_at > ?)
         ORDER BY created_at ASC",
    )
    .bind(user_asn)
    .bind(now)
    .fetch_all(pool)
    .await
    .map_err(map_db_err)
}

/// 全部 pending 请求（管理员）
pub async fn list_all_pending(pool: &SqlitePool) -> Result<Vec<PeeringRequest>, AppError> {
    let now = super::now_unix_secs();
    sqlx::query_as::<_, PeeringRequest>(
        "SELECT * FROM peering_requests
         WHERE status = 'pending'
           AND (expires_at IS NULL OR expires_at > ?)
         ORDER BY created_at ASC",
    )
    .bind(now)
    .fetch_all(pool)
    .await
    .map_err(map_db_err)
}

/// 标记为已批准
pub async fn mark_approved(pool: &SqlitePool, id: &str, reviewer_asn: i64) -> Result<(), AppError> {
    let now = super::now_unix_secs();
    sqlx::query(
        "UPDATE peering_requests
            SET status = 'approved', reviewer_asn = ?, reviewed_at = ?, updated_at = ?
          WHERE id = ?",
    )
    .bind(reviewer_asn)
    .bind(now)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(())
}

/// 标记为已拒绝
pub async fn mark_rejected(pool: &SqlitePool, id: &str, reviewer_asn: i64) -> Result<(), AppError> {
    let now = super::now_unix_secs();
    sqlx::query(
        "UPDATE peering_requests
            SET status = 'rejected', reviewer_asn = ?, reviewed_at = ?, updated_at = ?
          WHERE id = ?",
    )
    .bind(reviewer_asn)
    .bind(now)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(())
}

/// 标记为已派发
pub async fn mark_dispatched(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let now = super::now_unix_secs();
    sqlx::query(
        "UPDATE peering_requests
            SET status = 'dispatched', dispatched_at = ?, updated_at = ?
          WHERE id = ?",
    )
    .bind(now)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(())
}

/// 标记为成功
pub async fn mark_succeeded(pool: &SqlitePool, id: &str) -> Result<(), AppError> {
    let now = super::now_unix_secs();
    sqlx::query("UPDATE peering_requests SET status = 'succeeded', updated_at = ? WHERE id = ?")
        .bind(now)
        .bind(id)
        .execute(pool)
        .await
        .map_err(map_db_err)?;
    Ok(())
}

/// 标记为失败
pub async fn mark_failed(pool: &SqlitePool, id: &str, error: &str) -> Result<(), AppError> {
    let now = super::now_unix_secs();
    sqlx::query(
        "UPDATE peering_requests
            SET status = 'failed', result_error = ?, updated_at = ?
          WHERE id = ?",
    )
    .bind(error)
    .bind(now)
    .bind(id)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(())
}

/// 取消用户对某节点的所有非终态请求（删除）
pub async fn cancel_for_user_node(
    pool: &SqlitePool,
    user_asn: i64,
    node: &str,
) -> Result<u64, AppError> {
    let res = sqlx::query(
        "DELETE FROM peering_requests
          WHERE user_asn = ? AND node = ?
            AND status IN ('pending','approved','dispatched')",
    )
    .bind(user_asn)
    .bind(node)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(res.rows_affected())
}

/// 清理过期 pending（置为 expired）
pub async fn cleanup_expired(pool: &SqlitePool) -> Result<u64, AppError> {
    let now = super::now_unix_secs();
    let res = sqlx::query(
        "UPDATE peering_requests
            SET status = 'expired', updated_at = ?
          WHERE status = 'pending' AND expires_at IS NOT NULL AND expires_at <= ?",
    )
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(res.rows_affected())
}
