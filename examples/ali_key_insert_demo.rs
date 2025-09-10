use tracing::{info, error, debug};
use uuid::Uuid;

// 导入项目的模块
use project_rust_learn::dao::{init_sqlite_pool, init_db, SQLITE_POOL};
use project_rust_learn::dao::cache::init_global_cache;
use project_rust_learn::dao::provider_key_pool::{
    create_provider_key_pool_from_raw_key,
    get_provider_key_pool_by_id,
    list_provider_key_pools_by_provider,
    get_decrypted_api_key_from_cache
};
use project_rust_learn::logger::init_dev_logger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    if let Err(e) = init_dev_logger() {
        eprintln!("Failed to initialize logger: {}", e);
        std::process::exit(1);
    }
    info!("Logger initialized successfully");

    // 初始化数据库连接池
    info!("Initializing database...");
    init_sqlite_pool("sqlite://data/app.db").await;
    let pool = SQLITE_POOL.get().unwrap().clone();
    
    // 初始化数据库表结构
    match init_db("data/init.sql").await {
        Ok(_) => info!("Database initialized successfully"),
        Err(e) => {
            error!("DB init failed: {}", e);
            return Err(e.into());
        }
    }

    // 初始化缓存
    info!("Initializing memory cache...");
    match init_global_cache(&pool, 3600, 1000).await {
        Ok(_) => info!("Global cache initialized successfully"),
        Err(e) => {
            error!("Cache init failed: {}", e);
            return Err(e.into());
        }
    }

    // 1. 插入阿里云API key
    info!("Inserting Ali API key into database...");
    let ali_api_key = "sk-7caebfcc5b5c4554b51d59cfd13081ae";
    let key_id = Uuid::new_v4().to_string();
    
    match create_provider_key_pool_from_raw_key(
        &pool,
        key_id.clone(),
        "ali".to_string(),
        ali_api_key,
        true,
        Some(60),    // 每分钟60次请求限制
        Some(3600),  // 每小时3600次请求限制
    ).await {
        Ok(rows_affected) => {
            info!("Successfully inserted Ali API key, rows affected: {}", rows_affected);
        }
        Err(e) => {
            error!("Failed to insert Ali API key: {}", e);
            return Err(e.into());
        }
    }

    // 2. 验证插入 - 通过ID查询
    info!("Verifying insertion by querying the key by ID...");
    match get_provider_key_pool_by_id(&pool, &key_id).await {
        Ok(Some(key_pool)) => {
            info!("Successfully retrieved key pool by ID:");
            info!("  ID: {}", key_pool.id);
            info!("  Provider: {}", key_pool.provider);
            info!("  Is Active: {}", key_pool.is_active);
            info!("  Usage Count: {}", key_pool.usage_count);
            info!("  Rate Limit Per Minute: {:?}", key_pool.rate_limit_per_minute);
            info!("  Rate Limit Per Hour: {:?}", key_pool.rate_limit_per_hour);
            info!("  Created At: {:?}", key_pool.created_at);
            debug!("  Key Hash: {}", key_pool.key_hash);
            debug!("  Encrypted Key Value: {}", key_pool.encrypted_key_value);
        }
        Ok(None) => {
            error!("Key pool not found with ID: {}", key_id);
            return Err("Key pool not found".into());
        }
        Err(e) => {
            error!("Failed to retrieve key pool: {}", e);
            return Err(e.into());
        }
    }

    // 3. 验证插入 - 通过Provider查询所有阿里云的key
    info!("Verifying insertion by querying all Ali keys...");
    match list_provider_key_pools_by_provider(&pool, "ali").await {
        Ok(ali_keys) => {
            info!("Found {} Ali API keys:", ali_keys.len());
            for (index, key_pool) in ali_keys.iter().enumerate() {
                info!("  Ali Key #{}: ID={}, Active={}, Usage={}", 
                    index + 1, key_pool.id, key_pool.is_active, key_pool.usage_count);
            }
        }
        Err(e) => {
            error!("Failed to retrieve Ali keys: {}", e);
            return Err(e.into());
        }
    }

    // 4. 重新加载缓存以包含新插入的Ali key
    info!("Reloading provider key pools to cache to include new Ali key...");
    use project_rust_learn::dao::provider_key_pool::preload_provider_key_pools_to_cache;
    match preload_provider_key_pools_to_cache(&pool).await {
        Ok(_) => info!("Successfully reloaded provider key pools to cache"),
        Err(e) => {
            error!("Failed to reload provider key pools to cache: {}", e);
            return Err(e.into());
        }
    }

    // 5. 从缓存中获取解密的API key
    info!("Testing decrypted API key retrieval from cache...");
    match get_decrypted_api_key_from_cache("ali", &key_id).await {
        Some(decrypted_key) => {
            info!("Successfully retrieved decrypted API key from cache:");
            info!("  Original key: {}", ali_api_key);
            info!("  Decrypted key: {}", decrypted_key);
            
            if decrypted_key == ali_api_key {
                info!("✅ Verification successful: Original and decrypted keys match!");
            } else {
                error!("❌ Verification failed: Keys do not match!");
                return Err("Key verification failed".into());
            }
        }
        None => {
            error!("Failed to retrieve decrypted API key from cache - key not found");
            return Err("Decrypted key not found in cache".into());
        }
    }

    info!("🎉 Ali API key insertion and verification demo completed successfully!");
    Ok(())
}
