use axum::{
    extract::Path,
    http::StatusCode,
    response::Json,
};
use serde_json::{json, Value};
use uuid::Uuid;
use sqlx::SqlitePool;

use crate::dao::{
    provider::{Provider, get_all_providers, get_provider_by_id, create_provider, update_provider, hard_delete_provider, count_models_for_provider},
    provider_key_pool::{ProviderKeyPool, create_provider_key_pool},
    SQLITE_POOL,
};
use crate::dao::provider_key_pool::crypto::process_api_key;
use crate::web::dto::provider_dto::*;

/// 获取所有providers
pub async fn list_providers() -> Result<Json<Vec<ProviderResponse>>, StatusCode> {
    let pool = SQLITE_POOL.get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref();

    match get_all_providers(pool).await {
        Ok(providers) => {
            let mut responses = Vec::new();
            
            for provider in providers {
                let model_count = count_models_for_provider(pool, &provider.id)
                    .await
                    .unwrap_or(0) as usize;
                    
                responses.push(ProviderResponse {
                    id: provider.id,
                    name: provider.name,
                    display_name: provider.display_name,
                    base_url: provider.base_url,
                    description: provider.description,
                    is_active: provider.is_active,
                    model_count,
                    created_at: provider.created_at.unwrap_or_default(),
                });
            }
            
            Ok(Json(responses))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// 获取单个provider
pub async fn get_provider(Path(id): Path<String>) -> Result<Json<ProviderResponse>, StatusCode> {
    let pool = SQLITE_POOL.get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref();

    match get_provider_by_id(pool, &id).await {
        Ok(Some(provider)) => {
            let model_count = count_models_for_provider(pool, &provider.id)
                .await
                .unwrap_or(0) as usize;
                
            Ok(Json(ProviderResponse {
                id: provider.id,
                name: provider.name,
                display_name: provider.display_name,
                base_url: provider.base_url,
                description: provider.description,
                is_active: provider.is_active,
                model_count,
                created_at: provider.created_at.unwrap_or_default(),
            }))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// 创建新的provider
pub async fn create_new_provider(
    Json(request): Json<CreateProviderRequest>,
) -> Result<Json<Value>, StatusCode> {
    let pool = SQLITE_POOL.get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref();

    // 验证输入
    if request.name.trim().is_empty() || request.display_name.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // 生成ID
    let id = Uuid::new_v4().to_string();

    let provider = Provider {
        id: id.clone(),
        name: request.name.trim().to_lowercase(),
        display_name: request.display_name.trim().to_string(),
        base_url: request.base_url,
        description: request.description,
        is_active: true,
        created_at: None, // 数据库会自动设置
        updated_at: None,
    };

    match create_provider(pool, &provider).await {
        Ok(_) => {
            // 如果提供了API Key，则添加到key pool
            if let Some(api_key) = request.api_key {
                if !api_key.trim().is_empty() {
                    match add_api_key_to_pool(pool, &provider.name, &api_key).await {
                        Ok(_) => {},
                        Err(e) => {
                            tracing::error!("Failed to add API key to pool: {:?}", e);
                            // 不阻止provider创建，只是记录错误
                        }
                    }
                }
            }
            
            Ok(Json(json!({
                "id": id,
                "message": "Provider created successfully"
            })))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// 更新provider
pub async fn update_existing_provider(
    Path(id): Path<String>,
    Json(request): Json<UpdateProviderRequest>,
) -> Result<Json<Value>, StatusCode> {
    let pool = SQLITE_POOL.get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref();

    // 先获取现有provider
    let existing = match get_provider_by_id(pool, &id).await {
        Ok(Some(provider)) => provider,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // 保存provider名称用于后续API key操作
    let provider_name = existing.name.clone();
    
    // 构建更新后的provider
    let updated_provider = Provider {
        id: existing.id,
        name: existing.name, // name不允许修改
        display_name: request.display_name.unwrap_or(existing.display_name),
        base_url: request.base_url.or(existing.base_url),
        description: request.description.or(existing.description),
        is_active: request.is_active.unwrap_or(existing.is_active),
        created_at: existing.created_at,
        updated_at: None, // 数据库会自动更新
    };

    match update_provider(pool, &id, &updated_provider).await {
        Ok(rows) if rows > 0 => {
            // 如果提供了新的API Key，则添加到key pool
            if let Some(api_key) = request.api_key {
                if !api_key.trim().is_empty() {
                    match add_api_key_to_pool(pool, &provider_name, &api_key).await {
                        Ok(_) => {},
                        Err(e) => {
                            tracing::error!("Failed to add API key to pool: {:?}", e);
                            // 不阻止provider更新，只是记录错误
                        }
                    }
                }
            }
            
            Ok(Json(json!({
                "message": "Provider updated successfully"
            })))
        }
        Ok(_) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// 删除provider（检查关联模型后删除）
pub async fn delete_existing_provider(Path(id): Path<String>) -> Result<Json<Value>, StatusCode> {
    let pool = SQLITE_POOL.get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref();

    // First check if provider exists
    match get_provider_by_id(pool, &id).await {
        Ok(Some(_)) => {},
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    }

    // Check if there are associated models
    match count_models_for_provider(pool, &id).await {
        Ok(count) if count > 0 => {
            return Ok(Json(json!({
                "error": "Cannot delete provider with associated models",
                "message": format!("This provider has {} associated model(s). Please delete all models first.", count)
            })));
        }
        Ok(_) => {
            // No models, safe to delete
            match hard_delete_provider(pool, &id).await {
                Ok(rows) if rows > 0 => {
                    Ok(Json(json!({
                        "message": "Provider deleted successfully"
                    })))
                }
                Ok(_) => Err(StatusCode::NOT_FOUND),
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// 添加API Key到provider key pool的辅助函数
async fn add_api_key_to_pool(
    pool: &SqlitePool,
    provider_name: &str,
    api_key: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // 处理API密钥（哈希和加密）
    let (key_hash, encrypted_key_value) = process_api_key(api_key)
        .map_err(|e| format!("Failed to process API key: {}", e))?;
    
    // 生成唯一ID
    let key_id = Uuid::new_v4().to_string();
    
    // 创建ProviderKeyPool实例
    let key_pool = ProviderKeyPool {
        id: key_id,
        provider: provider_name.to_string(),
        key_hash,
        encrypted_key_value,
        is_active: true,
        usage_count: 0,
        last_used_at: None,
        rate_limit_per_minute: None,
        rate_limit_per_hour: None,
        created_at: None, // 数据库会自动设置
    };
    
    // 保存到数据库
    create_provider_key_pool(pool, &key_pool)
        .await
        .map_err(|e| format!("Failed to save API key: {}", e))?;
    
    Ok(())
}

/// 获取provider摘要（用于下拉框等）
pub async fn list_provider_summary() -> Result<Json<Vec<ProviderSummary>>, StatusCode> {
    let pool = SQLITE_POOL.get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref();

    match get_all_providers(pool).await {
        Ok(providers) => {
            let summaries: Vec<ProviderSummary> = providers
                .into_iter()
                .filter(|p| p.is_active)
                .map(|p| ProviderSummary {
                    id: p.id,
                    name: p.name,
                    display_name: p.display_name,
                    is_active: p.is_active,
                })
                .collect();
            
            Ok(Json(summaries))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
