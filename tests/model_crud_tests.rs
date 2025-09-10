use project_rust_learn::dao::{init_sqlite_pool, init_db, SQLITE_POOL};
use project_rust_learn::dao::model::{Model, create_model, list_models, update_model, delete_model, get_model_by_id};
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
async fn test_model_crud_operations() {
    let pool = setup_test_env().await;
    
    println!("=== Testing Model CRUD Operations ===");
    
    // 创建一个模型
    let model = Model {
        id: uuid::Uuid::new_v4().to_string(),
        name: "test_model".to_string(),
        provider: "test_provider".to_string(),
        model_type: "test_type".to_string(),
        base_url: None,
        is_active: true,
        health_status: None,
        last_health_check: None,
        health_check_interval_seconds: None,
        cost_per_token_input: None,
        cost_per_token_output: None,
        function_tags: None,
        config: None,
        created_at: None,
        updated_at: None,
    };

    // Test Create
    let rows = create_model(&pool, &model).await.expect("create_model failed");
    println!("✅ Created model: {} row(s)", rows);
    assert_eq!(rows, 1);

    // Test List All
    let models = list_models(&pool).await.expect("list_models failed");
    println!("✅ Listed models: {} total", models.len());
    assert!(models.len() > 0);

    // Test Get by ID
    let fetched = get_model_by_id(&pool, &model.id).await.expect("get_model_by_id failed");
    println!("✅ Fetched model by ID: {:?}", fetched.is_some());
    assert!(fetched.is_some());
    if let Some(ref fetched_model) = fetched {
        assert_eq!(fetched_model.name, model.name);
        assert_eq!(fetched_model.provider, model.provider);
    }

    // Test Update
    let mut updated_model = model.clone();
    updated_model.name = "updated_model".to_string();
    let updated_rows = update_model(&pool, &updated_model).await.expect("update_model failed");
    println!("✅ Updated model: {} row(s)", updated_rows);
    assert_eq!(updated_rows, 1);

    // Verify update
    let updated_fetched = get_model_by_id(&pool, &model.id).await.expect("get_model_by_id failed");
    if let Some(ref updated_model_data) = updated_fetched {
        assert_eq!(updated_model_data.name, "updated_model");
    }

    // Test Delete
    let deleted_rows = delete_model(&pool, &model.id).await.expect("delete_model failed");
    println!("✅ Deleted model: {} row(s)", deleted_rows);
    assert_eq!(deleted_rows, 1);

    // Verify deletion
    let deleted_fetched = get_model_by_id(&pool, &model.id).await.expect("get_model_by_id failed");
    assert!(deleted_fetched.is_none());
    
    println!("=== Model CRUD Tests Completed ===");
}
