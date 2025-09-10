use project_rust_learn::dao::{init_sqlite_pool, init_db, SQLITE_POOL};
use project_rust_learn::dao::system_config::{
    SystemConfig, create_system_config, get_system_config_by_id, get_system_config_by_key,
    list_system_configs, list_system_configs_by_category, list_encrypted_system_configs,
    update_system_config, update_system_config_value, update_system_config_encryption,
    delete_system_config, delete_system_configs_by_category, system_config_exists,
    get_system_config_value
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
async fn test_system_config_crud_operations() {
    let pool = setup_test_env().await;
    
    println!("=== Testing System Config CRUD Operations ===");

    // Test 1: Create system config entries
    let config1 = SystemConfig {
        id: uuid::Uuid::new_v4().to_string(),
        category: "database".to_string(),
        key_name: "max_connections".to_string(),
        value: "100".to_string(),
        is_encrypted: false,
        version: 1,
        created_at: None,
        updated_at: None,
    };

    let config2 = SystemConfig {
        id: uuid::Uuid::new_v4().to_string(),
        category: "api".to_string(),
        key_name: "secret_key".to_string(),
        value: "encrypted_secret_value_123".to_string(),
        is_encrypted: true,
        version: 1,
        created_at: None,
        updated_at: None,
    };

    let config3 = SystemConfig {
        id: uuid::Uuid::new_v4().to_string(),
        category: "api".to_string(),
        key_name: "rate_limit".to_string(),
        value: "1000".to_string(),
        is_encrypted: false,
        version: 1,
        created_at: None,
        updated_at: None,
    };

    let config4 = SystemConfig {
        id: uuid::Uuid::new_v4().to_string(),
        category: "logging".to_string(),
        key_name: "log_level".to_string(),
        value: "info".to_string(),
        is_encrypted: false,
        version: 1,
        created_at: None,
        updated_at: None,
    };

    println!("Creating system config entries...");
    let rows1 = create_system_config(&pool, &config1).await.expect("create_system_config failed");
    println!("✅ Created config 1: {} row(s)", rows1);
    assert_eq!(rows1, 1);

    let rows2 = create_system_config(&pool, &config2).await.expect("create_system_config failed");
    println!("✅ Created config 2: {} row(s)", rows2);
    assert_eq!(rows2, 1);

    let rows3 = create_system_config(&pool, &config3).await.expect("create_system_config failed");
    println!("✅ Created config 3: {} row(s)", rows3);
    assert_eq!(rows3, 1);

    let rows4 = create_system_config(&pool, &config4).await.expect("create_system_config failed");
    println!("✅ Created config 4: {} row(s)", rows4);
    assert_eq!(rows4, 1);

    // Test 2: List all system configs
    println!("\nListing all system configs...");
    let all_configs = list_system_configs(&pool).await.expect("list_system_configs failed");
    println!("✅ Total configs: {}", all_configs.len());
    assert!(all_configs.len() >= 4);

    // Test 3: Get system config by ID
    println!("\nGetting system config by ID...");
    let fetched_config = get_system_config_by_id(&pool, &config1.id).await.expect("get_system_config_by_id failed");
    println!("✅ Fetched config by id: {:?}", fetched_config.is_some());
    assert!(fetched_config.is_some());

    // Test 4: Get system config by key
    println!("\nGetting system config by key...");
    let config_by_key = get_system_config_by_key(&pool, "api", "rate_limit").await.expect("get_system_config_by_key failed");
    println!("✅ Fetched config by key (api.rate_limit): {:?}", config_by_key.is_some());
    assert!(config_by_key.is_some());

    // Test 5: List system configs by category
    println!("\nListing system configs by category (api)...");
    let api_configs = list_system_configs_by_category(&pool, "api").await.expect("list_system_configs_by_category failed");
    println!("✅ API configs: {}", api_configs.len());
    assert_eq!(api_configs.len(), 2);

    // Test 6: List encrypted system configs
    println!("\nListing encrypted system configs...");
    let encrypted_configs = list_encrypted_system_configs(&pool).await.expect("list_encrypted_system_configs failed");
    println!("✅ Encrypted configs: {}", encrypted_configs.len());
    assert_eq!(encrypted_configs.len(), 1);

    // Test 7: Check if system config exists
    println!("\nChecking if system config exists...");
    let exists1 = system_config_exists(&pool, "database", "max_connections").await.expect("system_config_exists failed");
    let exists2 = system_config_exists(&pool, "nonexistent", "key").await.expect("system_config_exists failed");
    println!("✅ database.max_connections exists: {}", exists1);
    println!("✅ nonexistent.key exists: {}", exists2);
    assert_eq!(exists1, true);
    assert_eq!(exists2, false);

    // Test 8: Get system config value directly
    println!("\nGetting system config value directly...");
    let value1 = get_system_config_value(&pool, "logging", "log_level").await.expect("get_system_config_value failed");
    let value2 = get_system_config_value(&pool, "nonexistent", "key").await.expect("get_system_config_value failed");
    println!("✅ logging.log_level value: {:?}", value1);
    println!("✅ nonexistent.key value: {:?}", value2);
    assert_eq!(value1, Some("info".to_string()));
    assert_eq!(value2, None);

    // Test 9: Update system config value
    println!("\nUpdating system config value...");
    let update_rows1 = update_system_config_value(&pool, "database", "max_connections", "200").await.expect("update_system_config_value failed");
    println!("✅ Updated database.max_connections: {} row(s)", update_rows1);
    assert_eq!(update_rows1, 1);

    // Test 10: Update full system config
    println!("\nUpdating full system config...");
    let mut updated_config3 = config3.clone();
    updated_config3.value = "2000".to_string();
    let update_rows2 = update_system_config(&pool, &updated_config3).await.expect("update_system_config failed");
    println!("✅ Updated full config: {} row(s)", update_rows2);
    assert_eq!(update_rows2, 1);

    // Test 11: Update system config encryption
    println!("\nUpdating system config encryption...");
    let encrypt_rows = update_system_config_encryption(&pool, &config4.id, true, "encrypted_log_level_value").await.expect("update_system_config_encryption failed");
    println!("✅ Updated encryption status: {} row(s)", encrypt_rows);
    assert_eq!(encrypt_rows, 1);

    // Test 12: Delete system config by category
    println!("\nDeleting system configs by category (api)...");
    let delete_category_rows = delete_system_configs_by_category(&pool, "api").await.expect("delete_system_configs_by_category failed");
    println!("✅ Deleted configs by category: {} row(s)", delete_category_rows);
    assert_eq!(delete_category_rows, 2);

    // Test 13: Delete individual system configs
    println!("\nDeleting individual system configs...");
    let delete_rows1 = delete_system_config(&pool, &config1.id).await.expect("delete_system_config failed");
    println!("✅ Deleted config 1: {} row(s)", delete_rows1);
    assert_eq!(delete_rows1, 1);

    let delete_rows4 = delete_system_config(&pool, &config4.id).await.expect("delete_system_config failed");
    println!("✅ Deleted config 4: {} row(s)", delete_rows4);
    assert_eq!(delete_rows4, 1);

    println!("\n=== System Config Tests Completed ===");
}
