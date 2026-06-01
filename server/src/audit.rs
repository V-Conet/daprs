//! Audit Log 模块
//!
//! 记录所有操作历史，供管理员查看

use shared::{ActionResult, ActionType, AppError, AuditLog};
use sled::Db;

const AUDIT_LOG_TREE: &str = "audit_log";
const MAX_AUDIT_LOGS: usize = 1000;

/// 记录操作日志
pub fn log_operation(
    db: &Db,
    actor_asn: u32,
    action: ActionType,
    target_asn: u32,
    node: &str,
    result: ActionResult,
) -> Result<(), AppError> {
    let tree = db
        .open_tree(AUDIT_LOG_TREE)
        .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;

    let id = uuid::Uuid::new_v4().to_string();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let entry = AuditLog {
        id,
        timestamp,
        actor_asn,
        action,
        target_asn,
        node: node.to_string(),
        result,
    };

    let key = format!("{:020}-{}", timestamp, entry.id);
    let value = serde_json::to_vec(&entry)
        .map_err(|e| AppError::InternalError(format!("json error: {e}")))?;

    tree.insert(key.as_bytes(), value)
        .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;
    tree.flush()
        .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;

    prune_old_logs(db)?;

    Ok(())
}

/// 清理旧日志，保留最近 MAX_AUDIT_LOGS 条
fn prune_old_logs(db: &Db) -> Result<(), AppError> {
    let tree = db
        .open_tree(AUDIT_LOG_TREE)
        .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;

    let count = tree.len();
    if count <= MAX_AUDIT_LOGS {
        return Ok(());
    }

    let to_remove = count - MAX_AUDIT_LOGS;
    let mut keys_to_remove = Vec::new();

    for item in tree.iter() {
        if keys_to_remove.len() >= to_remove {
            break;
        }
        let (key, _) = item.map_err(|e| AppError::InternalError(format!("db error: {e}")))?;
        keys_to_remove.push(key);
    }

    for key in keys_to_remove {
        tree.remove(key)
            .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;
    }

    tree.flush()
        .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;

    Ok(())
}

/// 获取所有审计日志（按时间倒序）
pub fn get_all_logs(db: &Db) -> Result<Vec<AuditLog>, AppError> {
    let tree = db
        .open_tree(AUDIT_LOG_TREE)
        .map_err(|e| AppError::InternalError(format!("db error: {e}")))?;

    let mut logs: Vec<AuditLog> = tree
        .iter()
        .map(|item| {
            item.map_err(|e| AppError::InternalError(format!("db error: {e}")))
                .and_then(|(_, value)| {
                    serde_json::from_slice::<AuditLog>(&value)
                        .map_err(|e| AppError::InternalError(format!("json error: {e}")))
                })
        })
        .collect::<Result<Vec<_>, _>>()?;

    logs.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    Ok(logs)
}
