use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub enum ModelType {
    #[serde(rename = "llm")]
    Llm,    // 大语言模型
    #[serde(rename = "vllm")]
    Vllm,   // 视觉语言模型
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CreateModelRequest {
    pub provider_id: String,
    pub name: String,
    pub display_name: Option<String>,
    pub model_type: ModelType,
    pub base_url: Option<String>,
    pub cost_per_token_input: f64,
    pub cost_per_token_output: f64,
    pub auto_start: bool,       // 是否立即启动
    pub custom_model: bool,     // 是否为自定义模型
    pub config: Option<String>, // 额外配置JSON
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateModelRequest {
    pub display_name: Option<String>,
    pub base_url: Option<String>,
    pub is_active: Option<bool>,
    pub cost_per_token_input: Option<f64>,
    pub cost_per_token_output: Option<f64>,
    pub config: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelResponse {
    pub id: String,
    pub name: String,
    pub display_name: Option<String>,
    pub provider: String,
    pub provider_name: String,  // Provider的显示名称
    pub model_type: String,
    pub base_url: Option<String>,
    pub is_active: bool,
    pub health_status: Option<String>,
    pub last_health_check: Option<String>,
    pub cost_per_token_input: Option<f64>,
    pub cost_per_token_output: Option<f64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelSummary {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub model_type: String,
    pub is_active: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelTemplate {
    pub name: String,
    pub display_name: String,
    pub description: String,
    pub model_type: ModelType,
    pub recommended_cost_input: f64,
    pub recommended_cost_output: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModelTemplateResponse {
    pub provider: String,
    pub templates: Vec<ModelTemplate>,
}
