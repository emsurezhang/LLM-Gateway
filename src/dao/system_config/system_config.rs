use sqlx::{SqlitePool, Result};

#[allow(dead_code)]
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct SystemConfig {
    pub id: String,
    pub category: String,
    pub key_name: String,
    pub value: String,
    pub is_encrypted: bool,
    pub version: i64,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Create a new system config entry (async)
pub async fn create_system_config(pool: &SqlitePool, config: &SystemConfig) -> Result<u64> {
    let res = sqlx::query(r#"
        INSERT INTO system_configs (
            id, category, key_name, value, is_encrypted, version, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
    "#)
        .bind(&config.id)
        .bind(&config.category)
        .bind(&config.key_name)
        .bind(&config.value)
        .bind(config.is_encrypted)
        .bind(config.version)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Read a system config entry by id (async)
pub async fn get_system_config_by_id(pool: &SqlitePool, id: &str) -> Result<Option<SystemConfig>> {
    let config = sqlx::query_as::<_, SystemConfig>("SELECT * FROM system_configs WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(config)
}

/// Read a system config entry by category and key_name (async)
pub async fn get_system_config_by_key(pool: &SqlitePool, category: &str, key_name: &str) -> Result<Option<SystemConfig>> {
    let config = sqlx::query_as::<_, SystemConfig>("SELECT * FROM system_configs WHERE category = ? AND key_name = ?")
        .bind(category)
        .bind(key_name)
        .fetch_optional(pool)
        .await?;
    Ok(config)
}

/// List all system config entries (async)
pub async fn list_system_configs(pool: &SqlitePool) -> Result<Vec<SystemConfig>> {
    let configs = sqlx::query_as::<_, SystemConfig>("SELECT * FROM system_configs ORDER BY category, key_name")
        .fetch_all(pool)
        .await?;
    Ok(configs)
}

/// List system config entries by category (async)
pub async fn list_system_configs_by_category(pool: &SqlitePool, category: &str) -> Result<Vec<SystemConfig>> {
    let configs = sqlx::query_as::<_, SystemConfig>("SELECT * FROM system_configs WHERE category = ? ORDER BY key_name")
        .bind(category)
        .fetch_all(pool)
        .await?;
    Ok(configs)
}

/// List encrypted system config entries (async)
pub async fn list_encrypted_system_configs(pool: &SqlitePool) -> Result<Vec<SystemConfig>> {
    let configs = sqlx::query_as::<_, SystemConfig>("SELECT * FROM system_configs WHERE is_encrypted = 1 ORDER BY category, key_name")
        .fetch_all(pool)
        .await?;
    Ok(configs)
}

/// Update a system config entry by id (async)
pub async fn update_system_config(pool: &SqlitePool, config: &SystemConfig) -> Result<u64> {
    let res = sqlx::query(r#"
        UPDATE system_configs SET
            category = ?,
            key_name = ?,
            value = ?,
            is_encrypted = ?,
            version = version + 1,
            updated_at = datetime('now')
        WHERE id = ?
    "#)
        .bind(&config.category)
        .bind(&config.key_name)
        .bind(&config.value)
        .bind(config.is_encrypted)
        .bind(&config.id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Update system config value by category and key_name (async)
pub async fn update_system_config_value(pool: &SqlitePool, category: &str, key_name: &str, value: &str) -> Result<u64> {
    let res = sqlx::query(r#"
        UPDATE system_configs SET
            value = ?,
            version = version + 1,
            updated_at = datetime('now')
        WHERE category = ? AND key_name = ?
    "#)
        .bind(value)
        .bind(category)
        .bind(key_name)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Update system config encryption status (async)
pub async fn update_system_config_encryption(pool: &SqlitePool, id: &str, is_encrypted: bool, encrypted_value: &str) -> Result<u64> {
    let res = sqlx::query(r#"
        UPDATE system_configs SET
            value = ?,
            is_encrypted = ?,
            version = version + 1,
            updated_at = datetime('now')
        WHERE id = ?
    "#)
        .bind(encrypted_value)
        .bind(is_encrypted)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Delete a system config entry by id (async)
pub async fn delete_system_config(pool: &SqlitePool, id: &str) -> Result<u64> {
    let res = sqlx::query("DELETE FROM system_configs WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Delete system config entries by category (async)
pub async fn delete_system_configs_by_category(pool: &SqlitePool, category: &str) -> Result<u64> {
    let res = sqlx::query("DELETE FROM system_configs WHERE category = ?")
        .bind(category)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Check if a system config key exists (async)
pub async fn system_config_exists(pool: &SqlitePool, category: &str, key_name: &str) -> Result<bool> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM system_configs WHERE category = ? AND key_name = ?")
        .bind(category)
        .bind(key_name)
        .fetch_one(pool)
        .await?;
    Ok(count.0 > 0)
}

/// Get system config value directly (async)
pub async fn get_system_config_value(pool: &SqlitePool, category: &str, key_name: &str) -> Result<Option<String>> {
    let result: Option<(String,)> = sqlx::query_as("SELECT value FROM system_configs WHERE category = ? AND key_name = ?")
        .bind(category)
        .bind(key_name)
        .fetch_optional(pool)
        .await?;
    Ok(result.map(|r| r.0))
}
