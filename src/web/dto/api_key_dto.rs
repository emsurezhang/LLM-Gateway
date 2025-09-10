use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyResponse {
    pub id: String,
    pub provider: String,
    pub key_preview: String,  // 显示部分密钥，如 "sk-...xyz"
    pub is_active: bool,
    pub usage_count: i64,
    pub last_used_at: Option<String>,
    pub rate_limit_per_minute: Option<i64>,
    pub rate_limit_per_hour: Option<i64>,
    pub created_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateApiKeyRequest {
    pub provider_id: String,
    pub api_key: String,
    pub rate_limit_per_minute: Option<i64>,
    pub rate_limit_per_hour: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateApiKeyRequest {
    pub is_active: Option<bool>,
    pub rate_limit_per_minute: Option<i64>,
    pub rate_limit_per_hour: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKeyListResponse {
    pub provider_id: String,
    pub provider_name: String,
    pub keys: Vec<ApiKeyResponse>,
}
