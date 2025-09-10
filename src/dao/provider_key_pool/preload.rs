use sqlx::{SqlitePool, Row};
use crate::dao::provider_key_pool::{list_provider_key_pools, ProviderKeyPool};
use crate::dao::cache::get_global_cache;
use crate::dao::provider_key_pool::crypto::decrypt_api_key;
use anyhow::Result;
use tracing::{info, error, debug, warn};
use serde::{Deserialize, Serialize};
use std::sync::atomic::AtomicUsize;
use std::collections::HashMap;
use tokio::sync::RwLock;
use lazy_static::lazy_static;

// 全局轮询计数器，每个 provider 一个
lazy_static! {
    static ref ROUND_ROBIN_COUNTERS: RwLock<HashMap<String, AtomicUsize>> = RwLock::new(HashMap::new());
    // 内存中的活跃 API Key 池，按 provider 分组
    static ref ACTIVE_KEY_POOLS: RwLock<HashMap<String, Vec<String>>> = RwLock::new(HashMap::new());
}

/// 用于缓存的 Provider Key Pool 结构体，包含解密后的 API KEY
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedProviderKeyPool {
    pub id: String,
    pub provider: String,
    pub key_hash: String,
    pub decrypted_api_key: String,  // 解密后的真实 API KEY
    pub is_active: bool,
    pub usage_count: i64,
    pub last_used_at: Option<String>,
    pub rate_limit_per_minute: Option<i64>,
    pub rate_limit_per_hour: Option<i64>,
    pub created_at: Option<String>,
}

impl From<&ProviderKeyPool> for CachedProviderKeyPool {
    fn from(key_pool: &ProviderKeyPool) -> Self {
        Self {
            id: key_pool.id.clone(),
            provider: key_pool.provider.clone(),
            key_hash: key_pool.key_hash.clone(),
            decrypted_api_key: String::new(), // 这里会在预加载时设置
            is_active: key_pool.is_active,
            usage_count: key_pool.usage_count,
            last_used_at: key_pool.last_used_at.clone(),
            rate_limit_per_minute: key_pool.rate_limit_per_minute,
            rate_limit_per_hour: key_pool.rate_limit_per_hour,
            created_at: key_pool.created_at.clone(),
        }
    }
}

/// 从数据库预加载所有 provider key pool 数据到全局缓存，同时构建轮询计数器
pub async fn preload_provider_key_pools_to_cache(pool: &SqlitePool) -> anyhow::Result<()> {
    info!("Starting to preload provider key pools to cache");
    
    // 1. 从数据库读取所有 provider key pools
    let key_pools = list_provider_key_pools(pool).await
        .map_err(|e| anyhow::anyhow!("Failed to load provider key pools from database: {}", e))?;
    
    info!(key_pool_count = key_pools.len(), "Loaded provider key pools from database");
    
    // 2. 获取全局缓存实例
    let cache = get_global_cache();
    
    // 3. 构建内存中的活跃 API Key 池和轮询计数器
    let mut provider_active_keys: HashMap<String, Vec<String>> = HashMap::new();
    let mut provider_counters: HashMap<String, AtomicUsize> = HashMap::new();
    
    // 4. 将每个 provider key pool 数据加载到缓存中
    for key_pool in key_pools {
        // 解密 API KEY
        let decrypted_api_key = match decrypt_api_key(&key_pool.encrypted_key_value) {
            Ok(api_key) => api_key,
            Err(e) => {
                error!(
                    key_pool_id = %key_pool.id,
                    provider = %key_pool.provider,
                    error = %e,
                    "Failed to decrypt API key for provider key pool, skipping"
                );
                continue; // 跳过这个无法解密的 key pool
            }
        };
        
        // 创建缓存对象，包含解密后的 API KEY
        let mut cached_key_pool = CachedProviderKeyPool::from(&key_pool);
        cached_key_pool.decrypted_api_key = decrypted_api_key;
        
        // 使用 provider key pool ID 作为缓存key
        let cache_key = format!("provider_key_pool:{}:{}", key_pool.provider, key_pool.id);
        
        // 将缓存对象序列化为JSON字符串作为缓存值
        let cache_value = serde_json::to_string(&cached_key_pool)
            .map_err(|e| anyhow::anyhow!("Failed to serialize cached provider key pool {}: {}", key_pool.id, e))?;
        
        // 插入到缓存
        cache.insert(cache_key.clone(), cache_value).await;
        
        // 如果是活跃的 API Key，添加到内存池中
        if key_pool.is_active {
            provider_active_keys
                .entry(key_pool.provider.clone())
                .or_insert_with(Vec::new)
                .push(key_pool.id.clone());
            
            // 初始化该 provider 的轮询计数器
            provider_counters
                .entry(key_pool.provider.clone())
                .or_insert_with(|| AtomicUsize::new(0));
        }
        
        debug!(
            key_pool_id = %key_pool.id,
            provider = %key_pool.provider,
            is_active = %key_pool.is_active,
            cache_key = %cache_key,
            api_key_length = %cached_key_pool.decrypted_api_key.len(),
            "Cached provider key pool with decrypted API key successfully"
        );
    }
    
    // 5. 更新全局的活跃 API Key 池和轮询计数器
    {
        let mut active_pools = ACTIVE_KEY_POOLS.write().await;
        *active_pools = provider_active_keys.clone();
    }
    
    {
        let mut counters = ROUND_ROBIN_COUNTERS.write().await;
        *counters = provider_counters;
    }
    
    info!("Successfully preloaded all provider key pools to cache");
    info!("Initialized round robin counters for {} providers", provider_active_keys.len());
    
    for (provider, keys) in provider_active_keys {
        info!("  {}: {} active keys", provider, keys.len());
    }
    
    Ok(())
}

/// 从缓存中获取 provider key pool（通过 provider 和 id）
/// 返回的是包含解密后 API KEY 的缓存对象
pub async fn get_provider_key_pool_from_cache(provider: &str, id: &str) -> Option<CachedProviderKeyPool> {
    let cache = get_global_cache();
    let cache_key = format!("provider_key_pool:{}:{}", provider, id);

    // 尝试从缓存获取，如果不存在则返回None
    let cached_value = cache.get(&cache_key).await?;
    
    // 反序列化JSON字符串为缓存的 provider key pool 对象
    match serde_json::from_str::<CachedProviderKeyPool>(&cached_value) {
        Ok(cached_key_pool) => Some(cached_key_pool),
        Err(e) => {
            error!(
                cache_key = %cache_key,
                error = %e,
                "Failed to deserialize cached provider key pool"
            );
            None
        }
    }
}

/// 将 ProviderKeyPool 插入到缓存（会解密 API KEY）
pub async fn insert_provider_key_pool_to_cache(key_pool: &ProviderKeyPool) -> Result<()> {
    let cache = get_global_cache();
    let cache_key = format!("provider_key_pool:{}:{}", key_pool.provider, key_pool.id);
    
    // 解密 API KEY
    let decrypted_api_key = decrypt_api_key(&key_pool.encrypted_key_value)?;
    
    // 创建缓存对象
    let mut cached_key_pool = CachedProviderKeyPool::from(key_pool);
    cached_key_pool.decrypted_api_key = decrypted_api_key;
    
    let cache_value = serde_json::to_string(&cached_key_pool)?;
    cache.insert(cache_key, cache_value).await;
    
    Ok(())
}

/// 直接插入已解密的 CachedProviderKeyPool 到缓存
pub async fn insert_cached_provider_key_pool_to_cache(cached_key_pool: &CachedProviderKeyPool) -> Result<()> {
    let cache = get_global_cache();
    let cache_key = format!("provider_key_pool:{}:{}", cached_key_pool.provider, cached_key_pool.id);
    
    let cache_value = serde_json::to_string(cached_key_pool)?;
    cache.insert(cache_key, cache_value).await;
    
    Ok(())
}

/// 从缓存中获取解密后的 API KEY
pub async fn get_decrypted_api_key_from_cache(provider: &str, id: &str) -> Option<String> {
    let cached_key_pool = get_provider_key_pool_from_cache(provider, id).await?;
    Some(cached_key_pool.decrypted_api_key)
}

/// 使用轮询策略从内存中获取指定 provider 的一个活跃 API Key
/// 
/// # Arguments
/// * `provider` - 提供商名称
/// 
/// # Returns
/// * `Some((String, String))` - 找到的 API Key 和对应的 ID
/// * `None` - 未找到活跃的 API Key
pub async fn get_api_key_round_robin(provider: &str) -> Option<(String, String)> {
    // 1. 从内存中获取该 provider 的活跃 API Key 列表
    let active_key_ids = {
        let active_pools = ACTIVE_KEY_POOLS.read().await;
        match active_pools.get(provider) {
            Some(keys) => keys.clone(),
            None => {
                info!("No active API keys found in memory for provider: {}", provider);
                return None;
            }
        }
    };

    if active_key_ids.is_empty() {
        info!("No active API keys found for provider: {}", provider);
        return None;
    }

    // 2. 获取该 provider 的轮询计数器
    let counter = {
        let counters = ROUND_ROBIN_COUNTERS.read().await;
        counters.get(provider)?.load(std::sync::atomic::Ordering::Relaxed)
    };

    // 3. 使用轮询策略选择 API Key
    let selected_index = counter % active_key_ids.len();
    let selected_key_id = &active_key_ids[selected_index];

    // 4. 更新计数器
    {
        let counters = ROUND_ROBIN_COUNTERS.read().await;
        if let Some(counter) = counters.get(provider) {
            counter.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        }
    }

    info!("Round robin selected API key {}:{} (index: {}/{})", 
          provider, selected_key_id, selected_index, active_key_ids.len());

    // 5. 从缓存获取解密后的 API Key
    if let Some(cached_key_pool) = get_provider_key_pool_from_cache(provider, selected_key_id).await {
        if cached_key_pool.is_active {
            return Some((cached_key_pool.decrypted_api_key, selected_key_id.clone()));
        } else {
            warn!("Selected API key {}:{} is not active", provider, selected_key_id);
        }
    } else {
        warn!("Selected API key {}:{} not found in cache", provider, selected_key_id);
    }

    None
}

/// 重新加载指定 provider 的活跃 API Key
/// 
/// # Arguments
/// * `pool` - 数据库连接池
/// * `provider` - 提供商名称
pub async fn reload_provider_api_keys(pool: &SqlitePool, provider: &str) -> anyhow::Result<()> {
    info!("Reloading API keys for provider: {}", provider);
    
    // 查询指定 provider 的所有活跃 API Key
    let query = "SELECT id FROM provider_key_pools WHERE provider = ? AND is_active = 1 ORDER BY id";
    let rows = sqlx::query(query)
        .bind(provider)
        .fetch_all(pool)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to query active keys for provider {}: {}", provider, e))?;

    let key_ids: Vec<String> = rows.into_iter()
        .map(|row| row.get::<String, _>("id"))
        .collect();

    // 更新内存中的活跃 key 池
    {
        let mut active_pools = ACTIVE_KEY_POOLS.write().await;
        if key_ids.is_empty() {
            active_pools.remove(provider);
        } else {
            active_pools.insert(provider.to_string(), key_ids.clone());
        }
    }

    // 重置该 provider 的轮询计数器
    reset_round_robin_counter(provider).await;

    info!("Reloaded {} active API keys for provider: {}", key_ids.len(), provider);
    Ok(())
}

/// 重置指定 provider 的轮询计数器
/// 
/// # Arguments
/// * `provider` - 提供商名称
pub async fn reset_round_robin_counter(provider: &str) {
    let counters = ROUND_ROBIN_COUNTERS.read().await;
    if let Some(counter) = counters.get(provider) {
        counter.store(0, std::sync::atomic::Ordering::Relaxed);
        info!("Reset round robin counter for provider: {}", provider);
    }
}

/// 获取指定 provider 当前的轮询计数器值
/// 
/// # Arguments
/// * `provider` - 提供商名称
/// 
/// # Returns
/// * 当前计数器值
pub async fn get_round_robin_counter(provider: &str) -> usize {
    let counters = ROUND_ROBIN_COUNTERS.read().await;
    counters.get(provider)
        .map(|counter| counter.load(std::sync::atomic::Ordering::Relaxed))
        .unwrap_or(0)
}

/// 获取指定 provider 在内存中的活跃 API Key 数量
/// 
/// # Arguments
/// * `provider` - 提供商名称
/// 
/// # Returns
/// * API Key 数量
pub async fn get_active_key_count(provider: &str) -> usize {
    let active_pools = ACTIVE_KEY_POOLS.read().await;
    active_pools.get(provider)
        .map(|keys| keys.len())
        .unwrap_or(0)
}