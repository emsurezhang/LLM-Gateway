use sqlx::{SqlitePool, Result};
use serde::{Deserialize, Serialize};
use crate::dao::provider_key_pool::crypto::{process_api_key, verify_key_integrity};

#[allow(dead_code)]
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct ProviderKeyPool {
    pub id: String,
    pub provider: String,
    pub key_hash: String,
    pub encrypted_key_value: String,
    pub is_active: bool,
    pub usage_count: i64,
    pub last_used_at: Option<String>,
    pub rate_limit_per_minute: Option<i64>,
    pub rate_limit_per_hour: Option<i64>,
    pub created_at: Option<String>,
}

/// Create a new provider key pool entry (async)
pub async fn create_provider_key_pool(pool: &SqlitePool, key_pool: &ProviderKeyPool) -> Result<u64> {
    let res = sqlx::query(r#"
        INSERT INTO provider_key_pools (
            id, provider, key_hash, encrypted_key_value, is_active, usage_count, 
            last_used_at, rate_limit_per_minute, rate_limit_per_hour, created_at
        ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'))
    "#)
        .bind(&key_pool.id)
        .bind(&key_pool.provider)
        .bind(&key_pool.key_hash)
        .bind(&key_pool.encrypted_key_value)
        .bind(key_pool.is_active)
        .bind(key_pool.usage_count)
        .bind(&key_pool.last_used_at)
        .bind(&key_pool.rate_limit_per_minute)
        .bind(&key_pool.rate_limit_per_hour)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Read a provider key pool entry by id (async)
pub async fn get_provider_key_pool_by_id(pool: &SqlitePool, id: &str) -> Result<Option<ProviderKeyPool>> {
    let key_pool = sqlx::query_as::<_, ProviderKeyPool>("SELECT * FROM provider_key_pools WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(key_pool)
}

/// List all provider key pool entries (async)
pub async fn list_provider_key_pools(pool: &SqlitePool) -> Result<Vec<ProviderKeyPool>> {
    let key_pools = sqlx::query_as::<_, ProviderKeyPool>("SELECT * FROM provider_key_pools")
        .fetch_all(pool)
        .await?;
    Ok(key_pools)
}

/// List provider key pool entries by provider (async)
pub async fn list_provider_key_pools_by_provider(pool: &SqlitePool, provider: &str) -> Result<Vec<ProviderKeyPool>> {
    let key_pools = sqlx::query_as::<_, ProviderKeyPool>("SELECT * FROM provider_key_pools WHERE provider = ?")
        .bind(provider)
        .fetch_all(pool)
        .await?;
    Ok(key_pools)
}

/// List active provider key pool entries (async)
pub async fn list_active_provider_key_pools(pool: &SqlitePool) -> Result<Vec<ProviderKeyPool>> {
    let key_pools = sqlx::query_as::<_, ProviderKeyPool>("SELECT * FROM provider_key_pools WHERE is_active = 1")
        .fetch_all(pool)
        .await?;
    Ok(key_pools)
}

/// Update a provider key pool entry by id (async)
pub async fn update_provider_key_pool(pool: &SqlitePool, key_pool: &ProviderKeyPool) -> Result<u64> {
    let res = sqlx::query(r#"
        UPDATE provider_key_pools SET
            provider = ?,
            key_hash = ?,
            encrypted_key_value = ?,
            is_active = ?,
            usage_count = ?,
            last_used_at = ?,
            rate_limit_per_minute = ?,
            rate_limit_per_hour = ?
        WHERE id = ?
    "#)
        .bind(&key_pool.provider)
        .bind(&key_pool.key_hash)
        .bind(&key_pool.encrypted_key_value)
        .bind(key_pool.is_active)
        .bind(key_pool.usage_count)
        .bind(&key_pool.last_used_at)
        .bind(&key_pool.rate_limit_per_minute)
        .bind(&key_pool.rate_limit_per_hour)
        .bind(&key_pool.id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Update usage count and last used time for a provider key pool entry (async)
pub async fn update_key_pool_usage(pool: &SqlitePool, id: &str) -> Result<u64> {
    let res = sqlx::query(r#"
        UPDATE provider_key_pools SET
            usage_count = usage_count + 1,
            last_used_at = datetime('now')
        WHERE id = ?
    "#)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Delete a provider key pool entry by id (async)
pub async fn delete_provider_key_pool(pool: &SqlitePool, id: &str) -> Result<u64> {
    let res = sqlx::query("DELETE FROM provider_key_pools WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Toggle active status of a provider key pool entry (async)
pub async fn toggle_provider_key_pool_active(pool: &SqlitePool, id: &str, is_active: bool) -> Result<u64> {
    let res = sqlx::query("UPDATE provider_key_pools SET is_active = ? WHERE id = ?")
        .bind(is_active)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Create a new provider key pool entry from raw API key (async)
/// This function automatically handles encryption and hashing
/// 
/// # Arguments
/// * `pool` - SQLite connection pool
/// * `id` - Unique identifier for the key pool entry
/// * `provider` - Provider name (e.g., "openai", "anthropic")
/// * `raw_api_key` - The original, unencrypted API key
/// * `is_active` - Whether the key is active
/// * `rate_limit_per_minute` - Optional rate limit per minute
/// * `rate_limit_per_hour` - Optional rate limit per hour
/// 
/// # Returns
/// * `Ok(u64)` - Number of rows affected
/// * `Err(sqlx::Error)` - Database error
pub async fn create_provider_key_pool_from_raw_key(
    pool: &SqlitePool,
    id: String,
    provider: String,
    raw_api_key: &str,
    is_active: bool,
    rate_limit_per_minute: Option<i64>,
    rate_limit_per_hour: Option<i64>,
) -> Result<u64> {
    let (key_hash, encrypted_key_value) = process_api_key(raw_api_key)
        .map_err(|e| sqlx::Error::Protocol(format!("Failed to process API key: {}", e)))?;

    let key_pool = ProviderKeyPool {
        id,
        provider,
        key_hash,
        encrypted_key_value,
        is_active,
        usage_count: 0,
        last_used_at: None,
        rate_limit_per_minute,
        rate_limit_per_hour,
        created_at: None,
    };

    create_provider_key_pool(pool, &key_pool).await
}