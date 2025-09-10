use project_rust_learn::dao::{
    provider_key_pool::{
        create_provider_key_pool_from_raw_key,
        toggle_provider_key_pool_active
    }
};
use sqlx::{SqlitePool, Row};

/// 创建内存中的测试数据库
async fn setup_test_db() -> SqlitePool {
    let pool = SqlitePool::connect("sqlite::memory:").await
        .expect("Failed to create in-memory database");
    
    // 创建表结构
    let create_table_sql = r#"
        CREATE TABLE IF NOT EXISTS provider_key_pools (
            id TEXT PRIMARY KEY,
            provider TEXT NOT NULL,
            key_hash TEXT NOT NULL,
            encrypted_key_value TEXT NOT NULL,
            is_active BOOLEAN DEFAULT 1,
            usage_count INTEGER DEFAULT 0,
            last_used_at TEXT,
            rate_limit_per_minute INTEGER,
            rate_limit_per_hour INTEGER,
            created_at TEXT DEFAULT (datetime('now', 'localtime'))
        );
    "#;
    
    sqlx::query(create_table_sql)
        .execute(&pool)
        .await
        .expect("Failed to create table");
    
    pool
}

/// 初始化测试数据
async fn setup_test_data(pool: &SqlitePool) {
    // 创建 OpenAI 的测试 API Keys
    for i in 1..=3 {
        let id = format!("openai_key_{}", i);
        let api_key = format!("sk-test-openai-key-{}", i);
        
        create_provider_key_pool_from_raw_key(
            pool,
            id,
            "openai".to_string(),
            &api_key,
            true, // 活跃
            Some(100),
            Some(3600),
        ).await.expect("Failed to create OpenAI key");
    }
    
    // 创建 Anthropic 的测试 API Keys
    for i in 1..=2 {
        let id = format!("anthropic_key_{}", i);
        let api_key = format!("sk-ant-test-key-{}", i);
        
        create_provider_key_pool_from_raw_key(
            pool,
            id,
            "anthropic".to_string(),
            &api_key,
            true, // 活跃
            Some(50),
            Some(1800),
        ).await.expect("Failed to create Anthropic key");
    }
    
    // 创建一个非活跃的 OpenAI key
    let inactive_id = "openai_key_inactive".to_string();
    let inactive_api_key = "sk-test-openai-inactive";
    
    create_provider_key_pool_from_raw_key(
        pool,
        inactive_id.clone(),
        "openai".to_string(),
        inactive_api_key,
        false, // 非活跃
        Some(100),
        Some(3600),
    ).await.expect("Failed to create inactive OpenAI key");
}

#[tokio::test]
async fn test_basic_database_operations() {
    let pool = setup_test_db().await;
    setup_test_data(&pool).await;
    
    // 查询活跃的 OpenAI keys
    let active_openai_keys = sqlx::query("SELECT id FROM provider_key_pools WHERE provider = ? AND is_active = 1")
        .bind("openai")
        .fetch_all(&pool)
        .await
        .expect("Failed to query active OpenAI keys");
    
    assert_eq!(active_openai_keys.len(), 3);
    
    // 查询活跃的 Anthropic keys
    let active_anthropic_keys = sqlx::query("SELECT id FROM provider_key_pools WHERE provider = ? AND is_active = 1")
        .bind("anthropic")
        .fetch_all(&pool)
        .await
        .expect("Failed to query active Anthropic keys");
    
    assert_eq!(active_anthropic_keys.len(), 2);
    
    // 测试禁用一个 key
    toggle_provider_key_pool_active(&pool, "openai_key_2", false).await
        .expect("Failed to toggle key status");
    
    // 重新查询
    let active_openai_keys_after = sqlx::query("SELECT id FROM provider_key_pools WHERE provider = ? AND is_active = 1")
        .bind("openai")
        .fetch_all(&pool)
        .await
        .expect("Failed to query active OpenAI keys after toggle");
    
    assert_eq!(active_openai_keys_after.len(), 2);
    
    println!("✅ Basic database operations test passed");
}

#[tokio::test]
async fn test_key_creation_and_encryption() {
    let pool = setup_test_db().await;
    
    // 创建一个 API key
    let test_api_key = "sk-test-key-for-encryption";
    create_provider_key_pool_from_raw_key(
        &pool,
        "test_key_1".to_string(),
        "test_provider".to_string(),
        test_api_key,
        true,
        Some(60),
        Some(3600),
    ).await.expect("Failed to create test key");
    
    // 查询创建的 key
    let stored_key = sqlx::query("SELECT * FROM provider_key_pools WHERE id = ?")
        .bind("test_key_1")
        .fetch_one(&pool)
        .await
        .expect("Failed to fetch stored key");
    
    // 验证数据
    let provider: String = stored_key.try_get("provider").expect("Failed to get provider");
    let is_active: bool = stored_key.try_get("is_active").expect("Failed to get is_active");
    let encrypted_value: String = stored_key.try_get("encrypted_key_value").expect("Failed to get encrypted_key_value");
    
    assert_eq!(provider, "test_provider");
    assert!(is_active);
    assert!(!encrypted_value.is_empty());
    // 加密后的值应该与原始值不同
    assert_ne!(encrypted_value, test_api_key);
    
    println!("✅ Key creation and encryption test passed");
}
