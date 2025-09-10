use sqlx::{SqlitePool, Result};
use serde::Serialize;

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CallLog {
    pub id: String,
    pub model_id: Option<String>,    
    pub status_code: i64,
    pub total_duration: i64,
    pub tokens_output: i64,
    pub error_message: Option<String>,
    pub created_at: Option<String>,
}

/// Create a new call log entry (async)
pub async fn create_call_log(pool: &SqlitePool, call_log: &CallLog) -> Result<u64> {
    let res = sqlx::query(r#"
        INSERT INTO call_logs (
            id, model_id, status_code, total_duration, tokens_output, error_message, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, datetime('now'))
    "#)
        .bind(&call_log.id)
        .bind(&call_log.model_id)
        .bind(call_log.status_code)
        .bind(call_log.total_duration)
        .bind(call_log.tokens_output)
        .bind(&call_log.error_message)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Read a call log entry by id (async)
pub async fn get_call_log_by_id(pool: &SqlitePool, id: &str) -> Result<Option<CallLog>> {
    let call_log = sqlx::query_as::<_, CallLog>("SELECT * FROM call_logs WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(call_log)
}

/// List all call log entries (async)
pub async fn list_call_logs(pool: &SqlitePool) -> Result<Vec<CallLog>> {
    let call_logs = sqlx::query_as::<_, CallLog>("SELECT * FROM call_logs ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?;
    Ok(call_logs)
}

/// List call logs with pagination (async)
pub async fn list_call_logs_paginated(pool: &SqlitePool, limit: i64, offset: i64) -> Result<Vec<CallLog>> {
    let call_logs = sqlx::query_as::<_, CallLog>("SELECT * FROM call_logs ORDER BY created_at DESC LIMIT ? OFFSET ?")
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;
    Ok(call_logs)
}

/// List call logs by model_id (async)
pub async fn list_call_logs_by_model(pool: &SqlitePool, model_id: &str) -> Result<Vec<CallLog>> {
    let call_logs = sqlx::query_as::<_, CallLog>("SELECT * FROM call_logs WHERE model_id = ? ORDER BY created_at DESC")
        .bind(model_id)
        .fetch_all(pool)
        .await?;
    Ok(call_logs)
}

/// List call logs by status code (async)
pub async fn list_call_logs_by_status(pool: &SqlitePool, status_code: i64) -> Result<Vec<CallLog>> {
    let call_logs = sqlx::query_as::<_, CallLog>("SELECT * FROM call_logs WHERE status_code = ? ORDER BY created_at DESC")
        .bind(status_code)
        .fetch_all(pool)
        .await?;
    Ok(call_logs)
}

/// List error call logs (non-200 status codes) (async)
pub async fn list_error_call_logs(pool: &SqlitePool) -> Result<Vec<CallLog>> {
    let call_logs = sqlx::query_as::<_, CallLog>("SELECT * FROM call_logs WHERE status_code != 200 ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?;
    Ok(call_logs)
}

/// List call logs within date range (async)
pub async fn list_call_logs_by_date_range(pool: &SqlitePool, start_date: &str, end_date: &str) -> Result<Vec<CallLog>> {
    let call_logs = sqlx::query_as::<_, CallLog>(
        "SELECT * FROM call_logs WHERE created_at >= ? AND created_at <= ? ORDER BY created_at DESC"
    )
        .bind(start_date)
        .bind(end_date)
        .fetch_all(pool)
        .await?;
    Ok(call_logs)
}

/// Get call logs statistics (async)
pub async fn get_call_logs_stats(pool: &SqlitePool) -> Result<CallLogStats> {
    let stats = sqlx::query_as::<_, CallLogStats>(r#"
        SELECT 
            COUNT(*) as total_calls,
            AVG(total_duration) as avg_latency_ms,
            0 as total_tokens_input,
            SUM(tokens_output) as total_tokens_output,
            0.0 as total_cost,
            COUNT(CASE WHEN status_code != 200 THEN 1 END) as error_count
        FROM call_logs
    "#)
        .fetch_one(pool)
        .await?;
    Ok(stats)
}

/// Get call logs statistics by model (async)
pub async fn get_call_logs_stats_by_model(pool: &SqlitePool, model_id: &str) -> Result<CallLogStats> {
    let stats = sqlx::query_as::<_, CallLogStats>(r#"
        SELECT 
            COUNT(*) as total_calls,
            AVG(total_duration) as avg_latency_ms,
            0 as total_tokens_input,
            SUM(tokens_output) as total_tokens_output,
            0.0 as total_cost,
            COUNT(CASE WHEN status_code != 200 THEN 1 END) as error_count
        FROM call_logs WHERE model_id = ?
    "#)
        .bind(model_id)
        .fetch_one(pool)
        .await?;
    Ok(stats)
}

/// Update a call log entry by id (async)
pub async fn update_call_log(pool: &SqlitePool, call_log: &CallLog) -> Result<u64> {
    let res = sqlx::query(r#"
        UPDATE call_logs SET
            model_id = ?,
            status_code = ?,
            total_duration = ?,
            tokens_output = ?,
            error_message = ?
        WHERE id = ?
    "#)
        .bind(&call_log.model_id)
        .bind(call_log.status_code)
        .bind(call_log.total_duration)
        .bind(call_log.tokens_output)
        .bind(&call_log.error_message)
        .bind(&call_log.id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Delete a call log entry by id (async)
pub async fn delete_call_log(pool: &SqlitePool, id: &str) -> Result<u64> {
    let res = sqlx::query("DELETE FROM call_logs WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Delete call logs by model_id (async)
pub async fn delete_call_logs_by_model(pool: &SqlitePool, model_id: &str) -> Result<u64> {
    let res = sqlx::query("DELETE FROM call_logs WHERE model_id = ?")
        .bind(model_id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Delete call logs older than specified date (async)
pub async fn delete_old_call_logs(pool: &SqlitePool, before_date: &str) -> Result<u64> {
    let res = sqlx::query("DELETE FROM call_logs WHERE created_at < ?")
        .bind(before_date)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Get count of call logs (async)
pub async fn count_call_logs(pool: &SqlitePool) -> Result<i64> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM call_logs")
        .fetch_one(pool)
        .await?;
    Ok(count.0)
}

/// Get count of call logs by model (async)
pub async fn count_call_logs_by_model(pool: &SqlitePool, model_id: &str) -> Result<i64> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM call_logs WHERE model_id = ?")
        .bind(model_id)
        .fetch_one(pool)
        .await?;
    Ok(count.0)
}

/// Statistics struct for call logs
#[derive(Debug, Clone, Serialize, sqlx::FromRow)]
pub struct CallLogStats {
    pub total_calls: i64,
    pub avg_latency_ms: Option<f64>,
    pub total_tokens_input: i64,
    pub total_tokens_output: i64,
    pub total_cost: f64,
    pub error_count: i64,
}
