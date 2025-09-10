use sqlx::{SqlitePool, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Provider {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub base_url: Option<String>,
    pub description: Option<String>,
    pub is_active: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Create a new provider
pub async fn create_provider(pool: &SqlitePool, provider: &Provider) -> Result<u64> {
    let res = sqlx::query(r#"
        INSERT INTO providers (
            id, name, display_name, base_url, description, is_active, created_at, updated_at
        ) VALUES (?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
    "#)
        .bind(&provider.id)
        .bind(&provider.name)
        .bind(&provider.display_name)
        .bind(&provider.base_url)
        .bind(&provider.description)
        .bind(provider.is_active)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Get provider by id
pub async fn get_provider_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Provider>> {
    let provider = sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE id = ?")
        .bind(id)
        .fetch_optional(pool)
        .await?;
    Ok(provider)
}

/// Get provider by name
pub async fn get_provider_by_name(pool: &SqlitePool, name: &str) -> Result<Option<Provider>> {
    let provider = sqlx::query_as::<_, Provider>("SELECT * FROM providers WHERE name = ?")
        .bind(name)
        .fetch_optional(pool)
        .await?;
    Ok(provider)
}

/// Get all providers
pub async fn get_all_providers(pool: &SqlitePool) -> Result<Vec<Provider>> {
    let providers = sqlx::query_as::<_, Provider>("SELECT * FROM providers ORDER BY created_at DESC")
        .fetch_all(pool)
        .await?;
    Ok(providers)
}

/// Update provider
pub async fn update_provider(pool: &SqlitePool, id: &str, provider: &Provider) -> Result<u64> {
    let res = sqlx::query(r#"
        UPDATE providers 
        SET display_name = ?, base_url = ?, description = ?, is_active = ?, updated_at = datetime('now')
        WHERE id = ?
    "#)
        .bind(&provider.display_name)
        .bind(&provider.base_url)
        .bind(&provider.description)
        .bind(provider.is_active)
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Delete provider (soft delete by setting is_active = false)
pub async fn delete_provider(pool: &SqlitePool, id: &str) -> Result<u64> {
    let res = sqlx::query("UPDATE providers SET is_active = 0, updated_at = datetime('now') WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Hard delete provider (only if no models are associated)
pub async fn hard_delete_provider(pool: &SqlitePool, id: &str) -> Result<u64> {
    // First check if there are any models associated with this provider
    let model_count = count_models_for_provider(pool, id).await?;
    if model_count > 0 {
        return Err(sqlx::Error::RowNotFound); // Use this to indicate constraint violation
    }
    
    // If no models, proceed with deletion
    let res = sqlx::query("DELETE FROM providers WHERE id = ?")
        .bind(id)
        .execute(pool)
        .await?;
    Ok(res.rows_affected())
}

/// Count models for provider
pub async fn count_models_for_provider(pool: &SqlitePool, provider_id: &str) -> Result<i64> {
    let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM models WHERE provider = ? AND is_active = 1")
        .bind(provider_id)
        .fetch_one(pool)
        .await?;
    Ok(count.0)
}
