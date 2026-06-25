-- 用户表：ASN 即身份（不含 is_admin，管理员由 config.toml admins 列表决定）
CREATE TABLE users (
    asn            INTEGER PRIMARY KEY,          -- DN42 ASN
    display_name   TEXT,                          -- OIDC claims 提取的显示名
    email          TEXT,                          -- OIDC claims 提取的邮箱
    mntner         TEXT,                          -- DN42 mnt-by（可选）
    userinfo       TEXT,                          -- JSON: OIDC userinfo 原始 claims
    first_login_at INTEGER NOT NULL,              -- unix 秒
    last_login_at  INTEGER NOT NULL,
    created_at     INTEGER NOT NULL,
    updated_at     INTEGER NOT NULL
);

-- 会话（取代 sled oauth_sessions；userinfo 已上移到 users）
CREATE TABLE sessions (
    id         TEXT PRIMARY KEY,                  -- cookie 随机 token
    user_asn   INTEGER NOT NULL REFERENCES users(asn) ON DELETE CASCADE,
    issued_at  INTEGER NOT NULL,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL
);
CREATE INDEX idx_sessions_user   ON sessions(user_asn);
CREATE INDEX idx_sessions_expire ON sessions(expires_at);

-- OAuth 登录中间态（取代 sled oauth_login_state，TTL 600s）
CREATE TABLE oauth_login_states (
    state         TEXT PRIMARY KEY,               -- CSRF secret
    pkce_verifier TEXT NOT NULL,
    provider      TEXT NOT NULL,
    created_at    INTEGER NOT NULL,
    expires_at    INTEGER NOT NULL
);
CREATE INDEX idx_ols_expire ON oauth_login_states(expires_at);

-- Peering 请求：统一队列 + 历史
-- 取代 sled peering_queue / modify_queue / remove_queue / pending_queue
CREATE TABLE peering_requests (
    id               TEXT PRIMARY KEY,            -- UUID
    user_asn         INTEGER NOT NULL REFERENCES users(asn),
    node             TEXT NOT NULL,               -- 目标节点名（来自 TOML）
    action           TEXT NOT NULL CHECK(action IN ('create','modify','delete')),
    status           TEXT NOT NULL CHECK(status IN
                        ('pending','approved','rejected','dispatched','succeeded','failed','expired')),
    require_approval INTEGER NOT NULL DEFAULT 0,  -- 快照 = 建请求时的节点 is_verify
    payload          TEXT,                         -- JSON PeeringPayload（delete 为 NULL）
    reviewer_asn     INTEGER REFERENCES users(asn),-- 审批管理员
    reviewed_at      INTEGER,
    dispatched_at    INTEGER,
    result_error     TEXT,
    created_at       INTEGER NOT NULL,
    updated_at       INTEGER NOT NULL,
    expires_at       INTEGER                       -- pending 的 TTL = created_at + 7d
);
CREATE INDEX idx_pr_user   ON peering_requests(user_asn, status);
CREATE INDEX idx_pr_status ON peering_requests(status, created_at);
CREATE INDEX idx_pr_node   ON peering_requests(node, status);

-- Peering 缓存（非真相源）：以 agent 文件系统为准，此表仅加速 admin 列表与校验
-- 命中→快速返回；校验失败或需要强一致→回源 agent
CREATE TABLE peer_cache (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    user_asn        INTEGER NOT NULL REFERENCES users(asn),
    node            TEXT NOT NULL,
    payload         TEXT NOT NULL,                 -- JSON 当前 PeeringPayload
    status          TEXT NOT NULL CHECK(status IN ('active','removed')),
    last_synced_at  INTEGER NOT NULL,              -- 上次与 agent 对账时间
    created_at      INTEGER NOT NULL,
    updated_at      INTEGER NOT NULL,
    UNIQUE(user_asn, node)                         -- 每对 (asn,node) 一行
);
CREATE INDEX idx_pc_user ON peer_cache(user_asn, status);

-- 审计日志（取代 sled audit_log；去掉 1000 条硬上限，改保留策略 + 分页）
CREATE TABLE audit_logs (
    id         TEXT PRIMARY KEY,                  -- UUID
    timestamp  INTEGER NOT NULL,
    actor_asn  INTEGER NOT NULL,                  -- 可能是普通用户或管理员
    action     TEXT NOT NULL,                     -- create/approve/reject/modify/delete
    target_asn INTEGER NOT NULL,
    node       TEXT NOT NULL,
    result     TEXT NOT NULL,                     -- 'success' | 'failed'
    error      TEXT,
    created_at INTEGER NOT NULL
);
CREATE INDEX idx_audit_ts     ON audit_logs(timestamp DESC);
CREATE INDEX idx_audit_actor  ON audit_logs(actor_asn, timestamp DESC);
CREATE INDEX idx_audit_target ON audit_logs(target_asn, timestamp DESC);
