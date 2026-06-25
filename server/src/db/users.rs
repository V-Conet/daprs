//! 用户仓储

use serde_json::Value;
use sqlx::{FromRow, SqlitePool};

use shared::AppError;

use super::map_db_err;

/// 用户行
#[derive(Debug, Clone, FromRow)]
pub struct User {
    pub asn: i64,
    pub display_name: Option<String>,
    pub email: Option<String>,
    pub mntner: Option<String>,
    pub userinfo: Option<String>,
    pub first_login_at: i64,
    pub last_login_at: i64,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 登录时写入/更新用户
///
/// `userinfo` 为 OIDC userinfo 原始 claims 的 JSON 字符串
pub async fn upsert_on_login(
    pool: &SqlitePool,
    asn: i64,
    display_name: Option<&str>,
    email: Option<&str>,
    mntner: Option<&str>,
    userinfo: &str,
) -> Result<(), AppError> {
    let now = super::now_unix_secs();

    sqlx::query(
        "INSERT INTO users (asn, display_name, email, mntner, userinfo,
                            first_login_at, last_login_at, created_at, updated_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(asn) DO UPDATE SET
            display_name = excluded.display_name,
            email        = excluded.email,
            mntner       = excluded.mntner,
            userinfo     = excluded.userinfo,
            last_login_at = excluded.last_login_at,
            updated_at    = excluded.updated_at",
    )
    .bind(asn)
    .bind(display_name)
    .bind(email)
    .bind(mntner)
    .bind(userinfo)
    .bind(now) // first_login_at（仅首次生效，ON CONFLICT 不覆盖）
    .bind(now)
    .bind(now)
    .bind(now)
    .execute(pool)
    .await
    .map_err(map_db_err)?;

    Ok(())
}

/// 获取用户
pub async fn get(pool: &SqlitePool, asn: i64) -> Result<Option<User>, AppError> {
    sqlx::query_as::<_, User>("SELECT * FROM users WHERE asn = ?")
        .bind(asn)
        .fetch_optional(pool)
        .await
        .map_err(map_db_err)
}

/// 解析 userinfo JSON
pub fn parse_userinfo(user: &User) -> Result<Value, AppError> {
    user.userinfo
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .ok_or_else(|| AppError::InternalError("missing userinfo".into()))
}
