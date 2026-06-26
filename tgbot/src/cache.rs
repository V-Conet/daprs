//! 命令结果缓存
//!
//! 用于 inline keyboard 节点切换：缓存一次命令在所有节点上的结果，
//! callback 时按 cache_id 取回，避免重复执行。

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use tokio::sync::Mutex;

use crate::error::CmdOutput;

pub type Cache = Arc<Mutex<HashMap<String, CacheEntry>>>;

/// 一次命令在所有节点上的结果
#[derive(Clone)]
pub struct CacheEntry {
    /// 缓存 ID（callback data 用），存入 entry 以保证端到端一致
    pub cache_id: String,
    /// 命令名（shared::Cmd::name()）
    pub cmd_name: String,
    /// 命令目标
    pub target: String,
    /// 节点名列表
    pub nodes: Vec<String>,
    /// 各节点结果（与 nodes 同序）
    pub results: Vec<NodeResult>,
    /// 创建时间（用于 TTL）
    pub created_at: Instant,
}

/// 单个节点的命令结果
#[derive(Clone)]
pub struct NodeResult {
    pub node: String,
    pub output: CmdOutput,
}

/// 生成缓存 ID（去掉连字符的 uuid）
pub fn gen_id() -> String {
    uuid::Uuid::new_v4().to_string().replace('-', "")
}

const MAX_ENTRIES: usize = 100;
const TTL_SECS: u64 = 1800; // 30 分钟

/// 机会式清理：先按 TTL 淘汰过期，再按最旧淘汰至上限
pub fn sweep_if_needed(map: &mut HashMap<String, CacheEntry>) {
    let now = Instant::now();
    map.retain(|_, e| now.duration_since(e.created_at).as_secs() < TTL_SECS);

    if map.len() > MAX_ENTRIES {
        let mut by_age: Vec<(String, Instant)> =
            map.iter().map(|(k, v)| (k.clone(), v.created_at)).collect();
        by_age.sort_by_key(|(_, t)| *t);
        let to_remove = map.len() - MAX_ENTRIES;
        for (k, _) in by_age.into_iter().take(to_remove) {
            map.remove(&k);
        }
    }
}
