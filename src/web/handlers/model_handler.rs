use axum::{
    extract::{Path, Query},
    http::StatusCode,
    response::Json,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use uuid::Uuid;

use crate::dao::{
    model::{Model, get_model_by_id, create_model, update_model, delete_model},
    provider::{get_provider_by_id},
    SQLITE_POOL,
};
use crate::web::dto::model_dto::*;

/// 获取所有models
pub async fn list_models(Query(params): Query<HashMap<String, String>>) -> Result<Json<Vec<ModelResponse>>, StatusCode> {
    let pool = SQLITE_POOL.get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref();

    match crate::dao::model::list_models(pool).await {
        Ok(models) => {
            let mut responses = Vec::new();
            
            for model in models {
                // 过滤条件
                if let Some(provider_filter) = params.get("provider") {
                    if model.provider != *provider_filter {
                        continue;
                    }
                }
                
                if let Some(active_filter) = params.get("active") {
                    if active_filter == "true" && !model.is_active {
                        continue;
                    }
                    if active_filter == "false" && model.is_active {
                        continue;
                    }
                }

                // 获取provider显示名称
                let provider_name = match get_provider_by_id(pool, &model.provider).await {
                    Ok(Some(provider)) => provider.display_name,
                    _ => model.provider.clone(), // 如果找不到provider，使用原始名称
                };
                
                responses.push(ModelResponse {
                    id: model.id,
                    name: model.name,
                    display_name: None, // TODO: 添加到Model结构体
                    provider: model.provider,
                    provider_name,
                    model_type: model.model_type,
                    base_url: model.base_url,
                    is_active: model.is_active,
                    health_status: model.health_status,
                    last_health_check: model.last_health_check,
                    cost_per_token_input: model.cost_per_token_input,
                    cost_per_token_output: model.cost_per_token_output,
                    created_at: model.created_at,
                    updated_at: model.updated_at,
                });
            }
            
            Ok(Json(responses))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// 获取单个model
pub async fn get_model(Path(id): Path<String>) -> Result<Json<ModelResponse>, StatusCode> {
    let pool = SQLITE_POOL.get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref();

    match get_model_by_id(pool, &id).await {
        Ok(Some(model)) => {
            // 获取provider显示名称
            let provider_name = match get_provider_by_id(pool, &model.provider).await {
                Ok(Some(provider)) => provider.display_name,
                _ => model.provider.clone(),
            };
            
            Ok(Json(ModelResponse {
                id: model.id,
                name: model.name,
                display_name: None,
                provider: model.provider,
                provider_name,
                model_type: model.model_type,
                base_url: model.base_url,
                is_active: model.is_active,
                health_status: model.health_status,
                last_health_check: model.last_health_check,
                cost_per_token_input: model.cost_per_token_input,
                cost_per_token_output: model.cost_per_token_output,
                created_at: model.created_at,
                updated_at: model.updated_at,
            }))
        }
        Ok(None) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// 创建新的model
pub async fn create_new_model(
    Json(request): Json<CreateModelRequest>,
) -> Result<Json<Value>, StatusCode> {
    let pool = SQLITE_POOL.get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref();

    // 验证输入
    if request.name.trim().is_empty() || request.provider_id.trim().is_empty() {
        return Err(StatusCode::BAD_REQUEST);
    }

    // 验证provider存在
    match get_provider_by_id(pool, &request.provider_id).await {
        Ok(Some(_)) => {},
        Ok(None) => return Err(StatusCode::BAD_REQUEST), // Provider不存在
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    }

    // 生成ID
    let id = Uuid::new_v4().to_string();

    let model_type_str = match request.model_type {
        ModelType::Llm => "llm",
        ModelType::Vllm => "vllm",
    };

    let model = Model {
        id: id.clone(),
        name: request.name.trim().to_string(),
        provider: request.provider_id,
        model_type: model_type_str.to_string(),
        base_url: request.base_url,
        is_active: request.auto_start,
        health_status: Some("unknown".to_string()),
        last_health_check: None,
        health_check_interval_seconds: Some(300), // 5分钟默认间隔
        cost_per_token_input: Some(request.cost_per_token_input),
        cost_per_token_output: Some(request.cost_per_token_output),
        function_tags: None,
        config: request.config,
        created_at: None,
        updated_at: None,
    };

    match create_model(pool, &model).await {
        Ok(_) => {
            Ok(Json(json!({
                "id": id,
                "message": "Model created successfully"
            })))
        }
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// 更新model
pub async fn update_existing_model(
    Path(id): Path<String>,
    Json(request): Json<UpdateModelRequest>,
) -> Result<Json<Value>, StatusCode> {
    let pool = SQLITE_POOL.get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref();

    // 先获取现有model
    let existing = match get_model_by_id(pool, &id).await {
        Ok(Some(model)) => model,
        Ok(None) => return Err(StatusCode::NOT_FOUND),
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // 构建更新后的model
    let updated_model = Model {
        id: existing.id,
        name: existing.name, // name不允许修改
        provider: existing.provider, // provider不允许修改
        model_type: existing.model_type, // model_type不允许修改
        base_url: request.base_url.or(existing.base_url),
        is_active: request.is_active.unwrap_or(existing.is_active),
        health_status: existing.health_status,
        last_health_check: existing.last_health_check,
        health_check_interval_seconds: existing.health_check_interval_seconds,
        cost_per_token_input: request.cost_per_token_input.or(existing.cost_per_token_input),
        cost_per_token_output: request.cost_per_token_output.or(existing.cost_per_token_output),
        function_tags: existing.function_tags,
        config: request.config.or(existing.config),
        created_at: existing.created_at,
        updated_at: None, // 数据库会自动更新
    };

    match update_model(pool, &updated_model).await {
        Ok(rows) if rows > 0 => {
            Ok(Json(json!({
                "message": "Model updated successfully"
            })))
        }
        Ok(_) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// 删除model（软删除）
pub async fn delete_existing_model(Path(id): Path<String>) -> Result<Json<Value>, StatusCode> {
    let pool = SQLITE_POOL.get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref();

    match delete_model(pool, &id).await {
        Ok(rows) if rows > 0 => {
            Ok(Json(json!({
                "message": "Model deleted successfully"
            })))
        }
        Ok(_) => Err(StatusCode::NOT_FOUND),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// 获取模型模板（针对特定provider）
pub async fn get_model_templates(Path(provider): Path<String>) -> Result<Json<ModelTemplateResponse>, StatusCode> {
    // 预定义的模型模板
    let templates = match provider.as_str() {
        "ollama" => vec![
            ModelTemplate {
                name: "llama3.1:latest".to_string(),
                display_name: "Llama 3.1 (Latest)".to_string(),
                description: "Meta的开源大语言模型，最新版本".to_string(),
                model_type: ModelType::Llm,
                recommended_cost_input: 0.0,
                recommended_cost_output: 0.0,
            },
            ModelTemplate {
                name: "llama3.1:8b".to_string(),
                display_name: "Llama 3.1 8B".to_string(),
                description: "Llama 3.1 8B参数版本".to_string(),
                model_type: ModelType::Llm,
                recommended_cost_input: 0.0,
                recommended_cost_output: 0.0,
            },
            ModelTemplate {
                name: "qwen2:7b".to_string(),
                display_name: "Qwen2 7B".to_string(),
                description: "阿里巴巴开源的Qwen2模型".to_string(),
                model_type: ModelType::Llm,
                recommended_cost_input: 0.0,
                recommended_cost_output: 0.0,
            },
        ],
        "ali" => vec![
            ModelTemplate {
                name: "qwen-turbo".to_string(),
                display_name: "通义千问 Turbo".to_string(),
                description: "快速响应版本，适合对话场景".to_string(),
                model_type: ModelType::Llm,
                recommended_cost_input: 0.0008,
                recommended_cost_output: 0.002,
            },
            ModelTemplate {
                name: "qwen-plus".to_string(),
                display_name: "通义千问 Plus".to_string(),
                description: "增强版本，更强的推理能力".to_string(),
                model_type: ModelType::Llm,
                recommended_cost_input: 0.004,
                recommended_cost_output: 0.012,
            },
            ModelTemplate {
                name: "qwen-max".to_string(),
                display_name: "通义千问 Max".to_string(),
                description: "最强版本，适合复杂任务".to_string(),
                model_type: ModelType::Llm,
                recommended_cost_input: 0.02,
                recommended_cost_output: 0.06,
            },
        ],
        "openai" => vec![
            ModelTemplate {
                name: "gpt-3.5-turbo".to_string(),
                display_name: "GPT-3.5 Turbo".to_string(),
                description: "性价比高的对话模型".to_string(),
                model_type: ModelType::Llm,
                recommended_cost_input: 0.0015,
                recommended_cost_output: 0.002,
            },
            ModelTemplate {
                name: "gpt-4".to_string(),
                display_name: "GPT-4".to_string(),
                description: "更强的推理和创作能力".to_string(),
                model_type: ModelType::Llm,
                recommended_cost_input: 0.03,
                recommended_cost_output: 0.06,
            },
            ModelTemplate {
                name: "gpt-4-vision-preview".to_string(),
                display_name: "GPT-4 Vision".to_string(),
                description: "支持图像理解的多模态模型".to_string(),
                model_type: ModelType::Vllm,
                recommended_cost_input: 0.01,
                recommended_cost_output: 0.03,
            },
        ],
        _ => vec![],
    };

    Ok(Json(ModelTemplateResponse {
        provider,
        templates,
    }))
}
