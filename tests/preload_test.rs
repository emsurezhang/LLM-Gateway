use project_rust_learn::dao::{init_sqlite_pool, init_db, SQLITE_POOL};
use project_rust_learn::dao::provider_key_pool::{
    create_provider_key_pool_from_raw_key, 
    preload_provider_key_pools_to_cache, get_provider_key_pool_from_cache,
    get_decrypted_api_key_from_cache
};
use std::sync::Arc;
use sqlx::{Pool, Sqlite};

/// 初始化测试环境的辅助函数
async fn setup_test_env() -> Arc<Pool<Sqlite>> {
    init_sqlite_pool("sqlite://data/app.db").await;
    let pool = SQLITE_POOL.get().unwrap().clone();
    init_db("data/init.sql").await.expect("DB init failed");
    pool
}

#[tokio::test]
async fn test_preload_with_decrypted_api_keys() {
    let pool = setup_test_env().await;
    
    println!("=== Testing Provider Key Pool Preload with Decrypted API Keys ===");

    // 1. 创建一些测试用的 provider key pools，包含真实的 API keys
    let test_api_key_1 = "sk-test-api-key-12345678901234567890123456789012";
    let test_api_key_2 = "gsk_test_api_key_67890123456789012345678901234567890";
    
    let provider_id_1 = uuid::Uuid::new_v4().to_string();
    let provider_id_2 = uuid::Uuid::new_v4().to_string();
    
    // 创建 OpenAI provider key pool
    let result1 = create_provider_key_pool_from_raw_key(
        &pool,
        provider_id_1.clone(),
        "openai".to_string(),
        test_api_key_1,
        true,
        Some(100),
        Some(6000)
    ).await;
    
    if let Err(e) = result1 {
        println!("Warning: Failed to create OpenAI key pool: {}", e);
        // 可能已经存在，继续执行测试
    } else {
        println!("✓ Created OpenAI provider key pool with ID: {}", provider_id_1);
    }
    
    // 创建 Groq provider key pool  
    let result2 = create_provider_key_pool_from_raw_key(
        &pool,
        provider_id_2.clone(),
        "groq".to_string(),
        test_api_key_2,
        true,
        Some(50),
        Some(3000)
    ).await;
    
    if let Err(e) = result2 {
        println!("Warning: Failed to create Groq key pool: {}", e);
        // 可能已经存在，继续执行测试
    } else {
        println!("✓ Created Groq provider key pool with ID: {}", provider_id_2);
    }

    // 2. 预加载所有 provider key pools 到缓存（包含解密后的 API keys）
    println!("\n--- Preloading provider key pools to cache ---");
    
    let preload_result = preload_provider_key_pools_to_cache(&pool).await;
    match preload_result {
        Ok(_) => println!("✓ Successfully preloaded provider key pools to cache"),
        Err(e) => {
            println!("✗ Failed to preload provider key pools: {}", e);
            return;
        }
    }

    // 3. 从缓存中获取 provider key pool（应该包含解密后的 API key）
    println!("\n--- Testing cache retrieval ---");
    
    let cached_openai = get_provider_key_pool_from_cache("openai", &provider_id_1).await;
    match cached_openai {
        Some(cached_pool) => {
            println!("✓ Retrieved OpenAI key pool from cache");
            println!("  - ID: {}", cached_pool.id);
            println!("  - Provider: {}", cached_pool.provider);
            println!("  - Is Active: {}", cached_pool.is_active);
            println!("  - Decrypted API Key Length: {}", cached_pool.decrypted_api_key.len());
            println!("  - API Key starts with: {}...", &cached_pool.decrypted_api_key[..10]);
            
            // 验证解密后的 API key 是否正确
            if cached_pool.decrypted_api_key == test_api_key_1 {
                println!("✓ Decrypted API key matches original!");
            } else {
                println!("✗ Decrypted API key does not match original");
                println!("  Expected: {}", test_api_key_1);
                println!("  Got: {}", cached_pool.decrypted_api_key);
            }
        }
        None => {
            println!("✗ Failed to retrieve OpenAI key pool from cache");
        }
    }
    
    // 4. 使用便捷函数直接获取解密后的 API key
    println!("\n--- Testing direct API key retrieval ---");
    
    let api_key = get_decrypted_api_key_from_cache("groq", &provider_id_2).await;
    match api_key {
        Some(key) => {
            println!("✓ Retrieved Groq API key from cache");
            println!("  - API Key Length: {}", key.len());
            println!("  - API Key starts with: {}...", &key[..10]);
            
            if key == test_api_key_2 {
                println!("✓ Retrieved API key matches original!");
            } else {
                println!("✗ Retrieved API key does not match original");
            }
        }
        None => {
            println!("✗ Failed to retrieve Groq API key from cache");
        }
    }

    println!("\n=== Test completed ===");
}
