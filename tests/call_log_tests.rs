use project_rust_learn::dao::{init_sqlite_pool, init_db, SQLITE_POOL};
use project_rust_learn::dao::call_log::{
    CallLog, create_call_log, get_call_log_by_id, list_call_logs,
    list_call_logs_paginated, list_call_logs_by_model, list_call_logs_by_status,
    list_error_call_logs, list_call_logs_by_date_range, get_call_logs_stats,
    get_call_logs_stats_by_model, update_call_log,
    delete_call_logs_by_model, delete_old_call_logs, count_call_logs,
    count_call_logs_by_model
};
use project_rust_learn::dao::model::{Model, create_model, delete_model};
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
async fn test_call_log_crud_operations() {
    let pool = setup_test_env().await;
    
    println!("=== Testing Call Log CRUD Operations ===");

    // First, we need to create a model to use as foreign key
    let test_model = Model {
        id: uuid::Uuid::new_v4().to_string(),
        name: "test_call_log_model".to_string(),
        provider: "openai".to_string(),
        model_type: "chat".to_string(),
        base_url: None,
        is_active: true,
        health_status: None,
        last_health_check: None,
        health_check_interval_seconds: None,
        cost_per_token_input: Some(0.001),
        cost_per_token_output: Some(0.002),
        function_tags: None,
        config: None,
        created_at: None,
        updated_at: None,
    };
    create_model(&pool, &test_model).await.expect("create test model failed");

    // Test 1: Create call log entries
    let call_log1 = CallLog {
        id: uuid::Uuid::new_v4().to_string(),
        model_id: Some(test_model.id.clone()),
        status_code: 200,
        total_duration: 150,
        tokens_output: 50,
        error_message: None,
        created_at: None,
    };

    let call_log2 = CallLog {
        id: uuid::Uuid::new_v4().to_string(),
        model_id: Some(test_model.id.clone()),
        status_code: 500,
        total_duration: 5000,
        tokens_output: 0,
        error_message: Some("Internal server error".to_string()),
        created_at: None,
    };

    let call_log3 = CallLog {
        id: uuid::Uuid::new_v4().to_string(),
        model_id: Some(test_model.id.clone()),
        status_code: 200,
        total_duration: 300,
        tokens_output: 120,
        error_message: None,
        created_at: None,
    };

    let call_log4 = CallLog {
        id: uuid::Uuid::new_v4().to_string(),
        model_id: None,
        status_code: 404,
        total_duration: 100,
        tokens_output: 0,
        error_message: Some("Model not found".to_string()),
        created_at: None,
    };

    println!("Creating call log entries...");
    let rows1 = create_call_log(&pool, &call_log1).await.expect("create_call_log failed");
    println!("✅ Created call log 1: {} row(s)", rows1);
    assert_eq!(rows1, 1);

    let rows2 = create_call_log(&pool, &call_log2).await.expect("create_call_log failed");
    println!("✅ Created call log 2: {} row(s)", rows2);
    assert_eq!(rows2, 1);

    let rows3 = create_call_log(&pool, &call_log3).await.expect("create_call_log failed");
    println!("✅ Created call log 3: {} row(s)", rows3);
    assert_eq!(rows3, 1);

    let rows4 = create_call_log(&pool, &call_log4).await.expect("create_call_log failed");
    println!("✅ Created call log 4: {} row(s)", rows4);
    assert_eq!(rows4, 1);

    // Test 2: List all call logs
    println!("\nListing all call logs...");
    let all_call_logs = list_call_logs(&pool).await.expect("list_call_logs failed");
    println!("✅ Total call logs: {}", all_call_logs.len());
    assert!(all_call_logs.len() >= 4);

    // Test 3: Get call log by ID
    println!("\nGetting call log by ID...");
    let fetched_call_log = get_call_log_by_id(&pool, &call_log1.id).await.expect("get_call_log_by_id failed");
    println!("✅ Fetched call log by id: {:?}", fetched_call_log.is_some());
    assert!(fetched_call_log.is_some());

    // Test 4: List call logs with pagination
    println!("\nListing call logs with pagination (limit 2, offset 0)...");
    let paginated_logs = list_call_logs_paginated(&pool, 2, 0).await.expect("list_call_logs_paginated failed");
    println!("✅ Paginated call logs (page 1): {}", paginated_logs.len());
    assert!(paginated_logs.len() <= 2);

    // Test 5: List call logs by model
    println!("\nListing call logs by model...");
    let model_logs = list_call_logs_by_model(&pool, &test_model.id).await.expect("list_call_logs_by_model failed");
    println!("✅ Call logs for model {}: {}", test_model.id, model_logs.len());
    assert_eq!(model_logs.len(), 3);

    // Test 6: List call logs by status
    println!("\nListing call logs by status (200)...");
    let success_logs = list_call_logs_by_status(&pool, 200).await.expect("list_call_logs_by_status failed");
    println!("✅ Successful call logs (200): {}", success_logs.len());
    assert_eq!(success_logs.len(), 2);

    // Test 7: List error call logs
    println!("\nListing error call logs...");
    let error_logs = list_error_call_logs(&pool).await.expect("list_error_call_logs failed");
    println!("✅ Error call logs: {}", error_logs.len());
    assert_eq!(error_logs.len(), 2);

    // Test 8: List call logs by date range
    println!("\nListing call logs by date range...");
    let date_logs = list_call_logs_by_date_range(&pool, "2024-01-01", "2025-12-31").await.expect("list_call_logs_by_date_range failed");
    println!("✅ Call logs in date range: {}", date_logs.len());
    assert!(date_logs.len() >= 4);

    // Test 9: Get call logs statistics
    println!("\nGetting call logs statistics...");
    let stats = get_call_logs_stats(&pool).await.expect("get_call_logs_stats failed");
    println!("✅ Overall stats: {:?}", stats);

    // Test 10: Get call logs statistics by model
    println!("\nGetting call logs statistics by model...");
    let model_stats = get_call_logs_stats_by_model(&pool, &test_model.id).await.expect("get_call_logs_stats_by_model failed");
    println!("✅ Model stats: {:?}", model_stats);

    // Test 11: Count call logs
    println!("\nCounting call logs...");
    let total_count = count_call_logs(&pool).await.expect("count_call_logs failed");
    println!("✅ Total call logs count: {}", total_count);
    assert!(total_count >= 4);

    let model_count = count_call_logs_by_model(&pool, &test_model.id).await.expect("count_call_logs_by_model failed");
    println!("✅ Call logs count for model: {}", model_count);
    assert_eq!(model_count, 3);

    // Test 12: Update call log
    println!("\nUpdating call log...");
    let mut updated_call_log = call_log2.clone();
    updated_call_log.status_code = 503;
    updated_call_log.error_message = Some("Service temporarily unavailable".to_string());
    let update_rows = update_call_log(&pool, &updated_call_log).await.expect("update_call_log failed");
    println!("✅ Updated call log: {} row(s)", update_rows);
    assert_eq!(update_rows, 1);

    // Test 13: Delete call logs by model
    println!("\nDeleting call logs by model...");
    let delete_model_rows = delete_call_logs_by_model(&pool, &test_model.id).await.expect("delete_call_logs_by_model failed");
    println!("✅ Deleted call logs by model: {} row(s)", delete_model_rows);
    assert_eq!(delete_model_rows, 3);

    // Test 14: Delete old call logs (this will delete the remaining log without model)
    println!("\nDeleting old call logs...");
    let delete_old_rows = delete_old_call_logs(&pool, "2025-12-31").await.expect("delete_old_call_logs failed");
    println!("✅ Deleted old call logs: {} row(s)", delete_old_rows);
    assert_eq!(delete_old_rows, 1);

    // Clean up test model
    delete_model(&pool, &test_model.id).await.expect("delete test model failed");

    println!("\n=== Call Log Tests Completed ===");
}
