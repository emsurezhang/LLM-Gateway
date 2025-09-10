use sqlx::{SqlitePool, Result};
use serde::{Serialize, Deserialize};

#[allow(dead_code)]
#[derive(Debug, Clone, sqlx::FromRow, Serialize, Deserialize)]
pub struct Model {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub model_type: String,
    pub base_url: Option<String>,
    pub is_active: bool,
    pub health_status: Option<String>,
    pub last_health_check: Option<String>,
    pub health_check_interval_seconds: Option<i64>,
    pub cost_per_token_input: Option<f64>,
    pub cost_per_token_output: Option<f64>,
    pub function_tags: Option<String>,
    pub config: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

/// Create a new model (async)
pub async fn create_model(pool: &SqlitePool, model: &Model) -> Result<u64> {
	let res = sqlx::query(r#"
		INSERT INTO models (
			id, name, provider, model_type, base_url, is_active, health_status, last_health_check,
			health_check_interval_seconds, cost_per_token_input, cost_per_token_output, function_tags, config, created_at, updated_at
		) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, datetime('now'), datetime('now'))
	"#)
		.bind(&model.id)
		.bind(&model.name)
		.bind(&model.provider)
		.bind(&model.model_type)
		.bind(&model.base_url)
		.bind(model.is_active)
		.bind(&model.health_status)
		.bind(&model.last_health_check)
		.bind(&model.health_check_interval_seconds)
		.bind(&model.cost_per_token_input)
		.bind(&model.cost_per_token_output)
		.bind(&model.function_tags)
		.bind(&model.config)
		.execute(pool)
		.await?;
	Ok(res.rows_affected())
}

/// Read a model by id (async)
pub async fn get_model_by_id(pool: &SqlitePool, id: &str) -> Result<Option<Model>> {
	let model = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE id = ?")
		.bind(id)
		.fetch_optional(pool)
		.await?;
	Ok(model)
}

pub async fn get_model_by_provider_and_name(pool: &SqlitePool, provider: &str, name: &str) -> Result<Option<Model>> {
    let model = sqlx::query_as::<_, Model>("SELECT * FROM models WHERE provider = ? AND name = ?")
        .bind(provider)
        .bind(name)
        .fetch_optional(pool)
        .await?;
    Ok(model)
}

/// List all models (async)
pub async fn list_models(pool: &SqlitePool) -> Result<Vec<Model>> {
	let models = sqlx::query_as::<_, Model>("SELECT * FROM models")
		.fetch_all(pool)
		.await?;
	Ok(models)
}

/// Update a model by id (async)
pub async fn update_model(pool: &SqlitePool, model: &Model) -> Result<u64> {
	let res = sqlx::query(r#"
		UPDATE models SET
			name = ?,
			provider = ?,
			model_type = ?,
			base_url = ?,
			is_active = ?,
			health_status = ?,
			last_health_check = ?,
			health_check_interval_seconds = ?,
			cost_per_token_input = ?,
			cost_per_token_output = ?,
			function_tags = ?,
			config = ?,
			updated_at = datetime('now')
		WHERE id = ?
	"#)
		.bind(&model.name)
		.bind(&model.provider)
		.bind(&model.model_type)
		.bind(&model.base_url)
		.bind(model.is_active)
		.bind(&model.health_status)
		.bind(&model.last_health_check)
		.bind(&model.health_check_interval_seconds)
		.bind(&model.cost_per_token_input)
		.bind(&model.cost_per_token_output)
		.bind(&model.function_tags)
		.bind(&model.config)
		.bind(&model.id)
		.execute(pool)
		.await?;
	Ok(res.rows_affected())
}

/// Delete a model by id (async)
pub async fn delete_model(pool: &SqlitePool, id: &str) -> Result<u64> {
	let res = sqlx::query("DELETE FROM models WHERE id = ?")
		.bind(id)
		.execute(pool)
		.await?;
	Ok(res.rows_affected())
}
