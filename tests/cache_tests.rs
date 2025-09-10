use project_rust_learn::dao::{init_sqlite_pool, init_db, SQLITE_POOL};
use project_rust_learn::dao::cache::{init_global_cache, get_global_cache};

/// 初始化测试环境的辅助函数
async fn setup_test_env() {
    init_sqlite_pool("sqlite://data/app.db").await;
    let pool = SQLITE_POOL.get().unwrap().clone();
    init_db("data/init.sql").await.expect("DB init failed");
    init_global_cache(&pool, 3600, 1000).await.expect("Cache init failed");
}

#[tokio::test]
async fn test_cache_operations() {
    setup_test_env().await;
    
    println!("=== Testing Cache Operations ===");
    
    // Demo: Using global cache
    let cache = get_global_cache();
    
    // Insert some data into cache
    cache.insert("test_key".to_string(), "test_value".to_string()).await;
    println!("Inserted data into cache");
    
    // Load data from cache with fallback
    let cached_value = cache.get_or_load("test_key".to_string(), |_key| async {
        "fallback_value".to_string()
    }).await;
    println!("Cached value: {}", cached_value);
    assert_eq!(cached_value, "test_value");
    
    // Load data that doesn't exist (will use loader)
    let new_value = cache.get_or_load("new_key".to_string(), |key| async move {
        format!("generated_value_for_{}", key)
    }).await;
    println!("New value: {}", new_value);
    assert_eq!(new_value, "generated_value_for_new_key");
    
    // Invalidate a key
    cache.invalidate(&"test_key".to_string()).await;
    println!("Invalidated test_key from cache");
    
    // Try to get the invalidated key again (should use loader)
    let reloaded_value = cache.get_or_load("test_key".to_string(), |_key| async {
        "reloaded_value".to_string()
    }).await;
    println!("Reloaded value: {}", reloaded_value);
    assert_eq!(reloaded_value, "reloaded_value");
    
    println!("=== Cache Operations Tests Completed ===");
}
