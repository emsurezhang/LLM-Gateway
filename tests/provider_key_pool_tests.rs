use project_rust_learn::dao::{init_sqlite_pool, init_db, SQLITE_POOL};
use project_rust_learn::dao::provider_key_pool::{
    ProviderKeyPool, create_provider_key_pool, get_provider_key_pool_by_id,
    list_provider_key_pools, list_provider_key_pools_by_provider, list_active_provider_key_pools,
    update_provider_key_pool, update_key_pool_usage, delete_provider_key_pool,
    toggle_provider_key_pool_active, preload_provider_key_pools_to_cache,
    get_provider_key_pool_from_cache, get_decrypted_api_key_from_cache
};
use project_rust_learn::dao::provider_key_pool::crypto::{process_api_key};
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
async fn test_provider_key_pool_crud_operations() {
    let pool = setup_test_env().await;
    
    println!("=== Testing Provider Key Pool CRUD Operations ===");

    // Test 1: Create provider key pool entries
    let key_pool1 = ProviderKeyPool {
        id: uuid::Uuid::new_v4().to_string(),
        provider: "openai".to_string(),
        key_hash: "hash_openai_key_1".to_string(),
        encrypted_key_value: "encrypted_openai_key_1".to_string(),
        is_active: true,
        usage_count: 0,
        last_used_at: None,
        rate_limit_per_minute: Some(60),
        rate_limit_per_hour: Some(3600),
        created_at: None,
    };

    let key_pool2 = ProviderKeyPool {
        id: uuid::Uuid::new_v4().to_string(),
        provider: "anthropic".to_string(),
        key_hash: "hash_anthropic_key_1".to_string(),
        encrypted_key_value: "encrypted_anthropic_key_1".to_string(),
        is_active: true,
        usage_count: 5,
        last_used_at: Some("2024-01-01 10:00:00".to_string()),
        rate_limit_per_minute: Some(30),
        rate_limit_per_hour: Some(1800),
        created_at: None,
    };

    let key_pool3 = ProviderKeyPool {
        id: uuid::Uuid::new_v4().to_string(),
        provider: "openai".to_string(),
        key_hash: "hash_openai_key_2".to_string(),
        encrypted_key_value: "encrypted_openai_key_2".to_string(),
        is_active: false,
        usage_count: 100,
        last_used_at: Some("2024-01-01 09:00:00".to_string()),
        rate_limit_per_minute: Some(60),
        rate_limit_per_hour: Some(3600),
        created_at: None,
    };

    println!("Creating provider key pool entries...");
    let rows1 = create_provider_key_pool(&pool, &key_pool1).await.expect("create_provider_key_pool failed");
    println!("✅ Created key pool 1: {} row(s)", rows1);
    assert_eq!(rows1, 1);

    let rows2 = create_provider_key_pool(&pool, &key_pool2).await.expect("create_provider_key_pool failed");
    println!("✅ Created key pool 2: {} row(s)", rows2);
    assert_eq!(rows2, 1);

    let rows3 = create_provider_key_pool(&pool, &key_pool3).await.expect("create_provider_key_pool failed");
    println!("✅ Created key pool 3: {} row(s)", rows3);
    assert_eq!(rows3, 1);

    // Test 2: List all provider key pools
    println!("\nListing all provider key pools...");
    let all_key_pools = list_provider_key_pools(&pool).await.expect("list_provider_key_pools failed");
    println!("✅ Total key pools: {}", all_key_pools.len());
    assert!(all_key_pools.len() >= 3);

    // Test 3: Get provider key pool by ID
    println!("\nGetting provider key pool by ID...");
    let fetched_key_pool = get_provider_key_pool_by_id(&pool, &key_pool1.id).await.expect("get_provider_key_pool_by_id failed");
    println!("✅ Fetched key pool by id: {:?}", fetched_key_pool.is_some());
    assert!(fetched_key_pool.is_some());

    // Test 4: List provider key pools by provider
    println!("\nListing provider key pools by provider (openai)...");
    let openai_key_pools = list_provider_key_pools_by_provider(&pool, "openai").await.expect("list_provider_key_pools_by_provider failed");
    println!("✅ OpenAI key pools: {}", openai_key_pools.len());
    assert_eq!(openai_key_pools.len(), 2);

    // Test 5: List active provider key pools
    println!("\nListing active provider key pools...");
    let active_key_pools = list_active_provider_key_pools(&pool).await.expect("list_active_provider_key_pools failed");
    println!("✅ Active key pools: {}", active_key_pools.len());
    assert_eq!(active_key_pools.len(), 2); // key_pool1 and key_pool2 are active

    // Test 6: Update key pool usage
    println!("\nUpdating key pool usage...");
    let usage_rows = update_key_pool_usage(&pool, &key_pool1.id).await.expect("update_key_pool_usage failed");
    println!("✅ Updated usage: {} row(s)", usage_rows);
    assert_eq!(usage_rows, 1);

    // Test 7: Update provider key pool
    println!("\nUpdating provider key pool...");
    let mut updated_key_pool2 = key_pool2.clone();
    updated_key_pool2.encrypted_key_value = "new_encrypted_anthropic_key".to_string();
    updated_key_pool2.rate_limit_per_minute = Some(90);
    let update_rows = update_provider_key_pool(&pool, &updated_key_pool2).await.expect("update_provider_key_pool failed");
    println!("✅ Updated key pool: {} row(s)", update_rows);
    assert_eq!(update_rows, 1);

    // Test 8: Toggle active status
    println!("\nToggling active status...");
    let toggle_rows = toggle_provider_key_pool_active(&pool, &key_pool3.id, true).await.expect("toggle_provider_key_pool_active failed");
    println!("✅ Toggled active status: {} row(s)", toggle_rows);
    assert_eq!(toggle_rows, 1);

    // Test 9: Delete provider key pools
    println!("\nDeleting provider key pools...");
    let delete_rows1 = delete_provider_key_pool(&pool, &key_pool1.id).await.expect("delete_provider_key_pool failed");
    println!("✅ Deleted key pool 1: {} row(s)", delete_rows1);
    assert_eq!(delete_rows1, 1);

    let delete_rows2 = delete_provider_key_pool(&pool, &key_pool2.id).await.expect("delete_provider_key_pool failed");
    println!("✅ Deleted key pool 2: {} row(s)", delete_rows2);
    assert_eq!(delete_rows2, 1);

    let delete_rows3 = delete_provider_key_pool(&pool, &key_pool3.id).await.expect("delete_provider_key_pool failed");
    println!("✅ Deleted key pool 3: {} row(s)", delete_rows3);
    assert_eq!(delete_rows3, 1);

    println!("\n=== Provider Key Pool Tests Completed ===");
}
