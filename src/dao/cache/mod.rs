use once_cell::sync::OnceCell;
use std::time::Duration;
use std::sync::Arc;
use sqlx::SqlitePool;
use crate::dao::model::{preload_models_to_cache};
use crate::dao::provider_key_pool::{preload_provider_key_pools_to_cache};
pub mod cache;

use cache::CacheService;

/// 全局缓存实例，使用 String 作为 key 和 value
pub static GLOBAL_CACHE: OnceCell<Arc<CacheService<String, String>>> = OnceCell::new();

/// 初始化全局缓存
pub async fn init_global_cache(pool: &SqlitePool, ttl_seconds: u64, max_capacity: u64) -> anyhow::Result<()> {
    let cache_service = CacheService::new(
        Duration::from_secs(ttl_seconds),
        max_capacity,
    );
    GLOBAL_CACHE.set(Arc::new(cache_service)).ok();

    // 预加载模型
    preload_models_to_cache(pool).await.expect("Failed to preload models");

    // 预加载 Provider Key Pool
    preload_provider_key_pools_to_cache(pool).await.expect("Failed to preload provider key pools");

    Ok(())
}

/// 获取全局缓存实例
pub fn get_global_cache() -> Arc<CacheService<String, String>> {
    GLOBAL_CACHE
        .get()
        .expect("Global cache not initialized")
        .clone()
}