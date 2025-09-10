use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateProviderRequest {
    pub name: String,           // provider名称 (ollama, ali, openai等)
    pub display_name: String,   // 显示名称
    pub base_url: Option<String>, // 基础URL
    pub api_key: Option<String>,  // API Key (可选)
    pub description: Option<String>, // 描述
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProviderRequest {
    pub display_name: Option<String>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,  // 如果提供，将添加新的API key到key pool
    pub description: Option<String>,
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderResponse {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub base_url: Option<String>,
    pub description: Option<String>,
    pub is_active: bool,
    pub model_count: usize,     // 关联的模型数量
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ProviderSummary {
    pub id: String,
    pub name: String,
    pub display_name: String,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AddApiKeyRequest {
    pub provider_id: String,
    pub api_key: String,
    pub rate_limit_per_minute: Option<i64>,
    pub rate_limit_per_hour: Option<i64>,
}
