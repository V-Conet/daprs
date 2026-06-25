-- 回滚：按依赖反序删除
DROP TABLE IF EXISTS audit_logs;
DROP TABLE IF EXISTS peer_cache;
DROP TABLE IF EXISTS peering_requests;
DROP TABLE IF EXISTS oauth_login_states;
DROP TABLE IF EXISTS sessions;
DROP TABLE IF EXISTS users;
