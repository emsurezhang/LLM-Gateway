use sqlx::SqlitePool;
use crate::dao::model::{list_models, Model};
use crate::dao::cache::get_global_cache;
use anyhow::Result;
use tracing::{info, error, debug, warn};
/// 从数据库预加载所有模型数据到全局缓存
pub async fn preload_models_to_cache(pool: &SqlitePool) -> anyhow::Result<()> {
    info!("Starting to preload models to cache");
    
    // 1. 从数据库读取所有模型
    let models = list_models(pool).await
        .map_err(|e| anyhow::anyhow!("Failed to load models from database: {}", e))?;
    
    info!(model_count = models.len(), "Loaded models from database");
    
    // 2. 获取全局缓存实例
    let cache = get_global_cache();
    
    // 3. 将每个模型数据加载到缓存中
    for model in models {
        // 使用模型ID作为缓存key
        let cache_key = format!("model:{}:{}", model.provider, model.name);
        
        // 将模型序列化为JSON字符串作为缓存值
        let cache_value = serde_json::to_string(&model)
            .map_err(|e| anyhow::anyhow!("Failed to serialize model {}: {}", model.id, e))?;
        
        // 插入到缓存
        cache.insert(cache_key.clone(), cache_value).await;
        
        debug!(
            model_name = %model.name,
            model_id = %model.id,
            provider = %model.provider,
            cache_key = %cache_key,
            "Cached model successfully"
        );
    }
    
    info!("Successfully preloaded all models to cache");
    Ok(())
}

/// 从缓存中获取模型（通过 provider 和 name）
pub async fn get_model_from_cache(provider: &str, name: &str) -> Option<Model> {
    let cache = get_global_cache();
    let cache_key = format!("model:{}:{}", provider, name);

    // 尝试从缓存获取，如果不存在则返回None
    let cached_value = cache.get(&cache_key).await?;
    
    // 反序列化JSON字符串为模型对象
    match serde_json::from_str::<Model>(&cached_value) {
        Ok(model) => Some(model),
        Err(e) => {
            error!(
                cache_key = %cache_key,
                error = %e,
                "Failed to deserialize cached model"
            );
            None
        }
    }
}

/// 将Model插入到缓存
pub async fn insert_model_to_cache(model: &Model) -> Result<()> {
    let cache = get_global_cache();
    let cache_key = format!("model:{}:{}", model.provider, model.name);
    
    let cache_value = serde_json::to_string(model)?;
    cache.insert(cache_key, cache_value).await;
    
    Ok(())
}