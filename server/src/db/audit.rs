//! 审计日志仓储

use shared::{ActionResult, ActionType, AuditLog};
use sqlx::{FromRow, SqlitePool};

use shared::AppError;

use super::map_db_err;

/// 审计日志行
#[derive(Debug, Clone, FromRow)]
pub struct AuditRow {
    pub id: String,
    pub timestamp: i64,
    pub actor_asn: i64,
    pub action: String,
    pub target_asn: i64,
    pub node: String,
    pub result: String,
    pub error: Option<String>,
    pub created_at: i64,
}

impl AuditRow {
    fn action_enum(&self) -> Result<ActionType, AppError> {
        match self.action.as_str() {
            "create" => Ok(ActionType::Create),
            "approve" => Ok(ActionType::Approve),
            "reject" => Ok(ActionType::Reject),
            "modify" => Ok(ActionType::Modify),
            "delete" => Ok(ActionType::Delete),
            other => Err(AppError::InternalError(format!("unknown action: {other}"))),
        }
    }

    fn result_enum(&self) -> ActionResult {
        match self.result.as_str() {
            "success" => ActionResult::Success,
            _ => ActionResult::Failed(self.error.clone().unwrap_or_default()),
        }
    }

    /// 转为对外共享类型
    pub fn to_audit_log(&self) -> Result<AuditLog, AppError> {
        Ok(AuditLog {
            id: self.id.clone(),
            timestamp: self.timestamp as u64,
            actor_asn: self.actor_asn as u32,
            action: self.action_enum()?,
            target_asn: self.target_asn as u32,
            node: self.node.clone(),
            result: self.result_enum(),
        })
    }
}

/// 记录一条审计日志
pub async fn insert(
    pool: &SqlitePool,
    id: &str,
    actor_asn: i64,
    action: ActionType,
    target_asn: i64,
    node: &str,
    result: &ActionResult,
) -> Result<(), AppError> {
    let now = super::now_unix_secs();
    let (result_str, error) = match result {
        ActionResult::Success => ("success", None),
        ActionResult::Failed(msg) => ("failed", Some(msg.as_str())),
    };

    sqlx::query(
        "INSERT INTO audit_logs (id, timestamp, actor_asn, action, target_asn, node, result, error, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(id)
    .bind(now)
    .bind(actor_asn)
    .bind(action.name()) // create/approve/reject/modify/delete
    .bind(target_asn)
    .bind(node)
    .bind(result_str)
    .bind(error)
    .bind(now)
    .execute(pool)
    .await
    .map_err(map_db_err)?;
    Ok(())
}

/// 分页查询审计日志（按时间倒序）
pub async fn list_paginated(
    pool: &SqlitePool,
    limit: i64,
    offset: i64,
) -> Result<Vec<AuditRow>, AppError> {
    sqlx::query_as::<_, AuditRow>(
        "SELECT * FROM audit_logs ORDER BY timestamp DESC, id DESC LIMIT ? OFFSET ?",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await
    .map_err(map_db_err)
}

/// 按保留天数清理
pub async fn prune_older_than(pool: &SqlitePool, retention_secs: i64) -> Result<u64, AppError> {
    let cutoff = super::now_unix_secs() - retention_secs;
    let res = sqlx::query("DELETE FROM audit_logs WHERE timestamp < ?")
        .bind(cutoff)
        .execute(pool)
        .await
        .map_err(map_db_err)?;
    Ok(res.rows_affected())
}
