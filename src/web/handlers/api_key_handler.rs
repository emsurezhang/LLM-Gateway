use axum::{
    extract::Path,
    http::StatusCode,
    response::Json,
};
use serde_json::{json, Value};
use uuid::Uuid;
use sqlx::SqlitePool;

use crate::dao::{
    provider::{get_provider_by_id},
    provider_key_pool::{
        ProviderKeyPool, 
        list_provider_key_pools_by_provider, 
        create_provider_key_pool_from_raw_key,
        get_provider_key_pool_by_id,
        update_provider_key_pool,
        delete_provider_key_pool,
        toggle_provider_key_pool_active
    },
    SQLITE_POOL,
};
use crate::web::dto::api_key_dto::*;
use crate::dao::provider_key_pool::crypto::{process_api_key, decrypt_api_key};

/// 获取指定Provider的所有API Key
pub async fn list_provider_api_keys(Path(provider_id): Path<String>) -> Result<Json<ApiKeyListResponse>, StatusCode> {
    let pool = SQLITE_POOL.get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref();

    // 首先获取provider信息
    let provider = match get_provider_by_id(pool, &provider_id).await {
        Ok(Some(provider)) => provider,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // 获取该provider的所有API Key
    match list_provider_key_pools_by_provider(pool, &provider.name).await {
        Ok(keys) => {
            let api_keys: Vec<ApiKeyResponse> = keys.into_iter().map(|key| {
                let key_preview = generate_key_preview(&key.key_hash);
                
                ApiKeyResponse {
                    id: key.id,
                    provider: key.provider,
                    key_preview,
                    is_active: key.is_active,
                    usage_count: key.usage_count,
                    last_used_at: key.last_used_at,
                    rate_limit_per_minute: key.rate_limit_per_minute,
                    rate_limit_per_hour: key.rate_limit_per_hour,
                    created_at: key.created_at,
                }
            }).collect();

            Ok(Json(ApiKeyListResponse {
                provider_id: provider.id,
                provider_name: provider.display_name,
                keys: api_keys,
            }))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// 为Provider添加新的API Key
pub async fn create_api_key(Json(request): Json<CreateApiKeyRequest>) -> Result<Json<Value>, StatusCode> {
    let pool = SQLITE_POOL.get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref();

    // 验证输入
    if request.api_key.trim().is_empty() {
        return Ok(Json(json!({
            "error": "API key cannot be empty"
        })));
    }

    // 获取provider信息
    let provider = match get_provider_by_id(pool, &request.provider_id).await {
        Ok(Some(provider)) => provider,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // 生成唯一ID
    let key_id = Uuid::new_v4().to_string();

    match create_provider_key_pool_from_raw_key(
        pool,
        key_id.clone(),
        provider.name,
        &request.api_key,
        true, // 默认激活
        request.rate_limit_per_minute,
        request.rate_limit_per_hour,
    ).await {
        Ok(_) => Ok(Json(json!({
            "id": key_id,
            "message": "API key added successfully"
        }))),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// 更新API Key设置
pub async fn update_api_key(
    Path(key_id): Path<String>,
    Json(request): Json<UpdateApiKeyRequest>,
) -> Result<Json<Value>, StatusCode> {
    let pool = SQLITE_POOL.get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref();

    // 获取现有的API Key
    let existing = match get_provider_key_pool_by_id(pool, &key_id).await {
        Ok(Some(key)) => key,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // 构建更新后的API Key
    let updated_key = ProviderKeyPool {
        id: existing.id,
        provider: existing.provider,
        key_hash: existing.key_hash,
        encrypted_key_value: existing.encrypted_key_value,
        is_active: request.is_active.unwrap_or(existing.is_active),
        usage_count: existing.usage_count,
        last_used_at: existing.last_used_at,
        rate_limit_per_minute: request.rate_limit_per_minute.or(existing.rate_limit_per_minute),
        rate_limit_per_hour: request.rate_limit_per_hour.or(existing.rate_limit_per_hour),
        created_at: existing.created_at,
    };

    match update_provider_key_pool(pool, &updated_key).await {
        Ok(rows) if rows > 0 => Ok(Json(json!({
            "message": "API key updated successfully"
        }))),
        Ok(_) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// 删除API Key
pub async fn delete_api_key(Path(key_id): Path<String>) -> Result<Json<Value>, StatusCode> {
    let pool = SQLITE_POOL.get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref();

    match delete_provider_key_pool(pool, &key_id).await {
        Ok(rows) if rows > 0 => Ok(Json(json!({
            "message": "API key deleted successfully"
        }))),
        Ok(_) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// 切换API Key的激活状态
pub async fn toggle_api_key_status(
    Path((key_id, status)): Path<(String, bool)>
) -> Result<Json<Value>, StatusCode> {
    let pool = SQLITE_POOL.get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref();

    match toggle_provider_key_pool_active(pool, &key_id, status).await {
        Ok(rows) if rows > 0 => Ok(Json(json!({
            "message": format!("API key {} successfully", if status { "activated" } else { "deactivated" })
        }))),
        Ok(_) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// 生成密钥预览（显示前几位和后几位）
fn generate_key_preview(key_hash: &str) -> String {
    if key_hash.len() > 8 {
        format!("{}...{}", &key_hash[..4], &key_hash[key_hash.len()-4..])
    } else {
        format!("{}...", &key_hash[..std::cmp::min(4, key_hash.len())])
    }
}
