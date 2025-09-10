use crate::dao::model::{Model, get_model_by_provider_and_name};
use crate::dao::model::{get_model_from_cache, insert_model_to_cache};
use crate::dao::SQLITE_POOL;
use sqlx::Result;

/// 根据 provider 和 name 查找 Model，优先从缓存查找，缓存未命中则查询数据库
/// 
/// # Arguments
/// * `provider` - 提供商名称
/// * `name` - 模型名称
/// 
/// # Returns
/// * `Ok(Some(Model))` - 找到模型
/// * `Ok(None)` - 未找到模型
/// * `Err(sqlx::Error)` - 数据库查询错误
pub async fn get_model_with_cache(provider: &str, name: &str) -> Result<Option<Model>> {
    // 1. 先检查缓存中是否存在
    if let Some(model) = get_model_from_cache(provider, name).await {
        println!("Cache hit for model: {}:{}", provider, name);
        return Ok(Some(model));
    }

    println!("Cache miss for model: {}:{}", provider, name);

    // 2. 缓存未命中，查询数据库
    let pool = SQLITE_POOL.get()
        .expect("SQLITE_POOL not initialized")
        .clone();
    
    match get_model_by_provider_and_name(&pool, provider, name).await? {
        Some(model) => {
            // 3. 数据库中存在，将Model插入到缓存中
            if let Err(_) = insert_model_to_cache(&model).await {
                eprintln!("Failed to cache model {}:{}", provider, name);
            }
            Ok(Some(model))
        }
        None => {
            // 4. 数据库中也不存在
            Ok(None)
        }
    }
}