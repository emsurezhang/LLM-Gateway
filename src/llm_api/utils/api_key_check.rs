use crate::dao::provider_key_pool::{
    get_provider_key_pool_by_id,
    get_provider_key_pool_from_cache, 
    insert_provider_key_pool_to_cache
};
use crate::dao::SQLITE_POOL;
use sqlx::Result;

/// 根据 provider 和 id 查找特定的 API Key，优先从缓存查找
/// 
/// # Arguments
/// * `provider` - 提供商名称
/// * `id` - API Key 池 ID
/// 
/// # Returns
/// * `Ok(Some(String))` - 找到解密后的 API Key
/// * `Ok(None)` - 未找到 API Key 或 API Key 不活跃
/// * `Err(sqlx::Error)` - 数据库查询错误
pub async fn get_api_key_with_cache(provider: &str, id: &str) -> Result<Option<String>> {
    // 1. 先检查缓存中是否存在
    if let Some(cached_key_pool) = get_provider_key_pool_from_cache(provider, id).await {
        if cached_key_pool.is_active {
            println!("Cache hit for API key: {}:{}", provider, id);
            return Ok(Some(cached_key_pool.decrypted_api_key));
        } else {
            println!("Cache hit but API key is inactive: {}:{}", provider, id);
            return Ok(None);
        }
    }

    println!("Cache miss for API key: {}:{}", provider, id);

    // 2. 缓存未命中，查询数据库
    let pool = SQLITE_POOL.get()
        .expect("SQLITE_POOL not initialized")
        .clone();
    
    match get_provider_key_pool_by_id(&pool, id).await? {
        Some(key_pool) => {
            // 检查 provider 是否匹配且 API Key 是否活跃
            if key_pool.provider == provider && key_pool.is_active {
                // 3. 数据库中存在且活跃，将 ProviderKeyPool 插入到缓存中
                if let Err(e) = insert_provider_key_pool_to_cache(&key_pool).await {
                    eprintln!("Failed to cache API key {}:{}: {}", provider, id, e);
                }

                // 4. 从缓存中获取解密后的 API Key
                if let Some(cached_key_pool) = get_provider_key_pool_from_cache(provider, id).await {
                    Ok(Some(cached_key_pool.decrypted_api_key))
                } else {
                    // 如果缓存失败，直接解密返回
                    match crate::dao::provider_key_pool::crypto::decrypt_api_key(&key_pool.encrypted_key_value) {
                        Ok(api_key) => Ok(Some(api_key)),
                        Err(e) => Err(sqlx::Error::Protocol(format!("Failed to decrypt API key: {}", e))),
                    }
                }
            } else {
                Ok(None)
            }
        }
        None => Ok(None)
    }
}