//! # 阿里云通义千问 API 客户端
//!
//! 实现阿里云 DashScope API 的客户端，支持通义千问等模型
//! 使用 OpenAI 兼容格式的 API 接口

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use anyhow::Result;
use reqwest::Client;

use crate::llm_api::utils::{
    client::{BaseClient, ClientConfig, ClientError, LLMClientTrait},
    chat_traits::{ChatRequestTrait, ChatResponseTrait},
    msg_structure::Message,
};

/// 阿里云 Chat 请求结构体（OpenAI 兼容格式）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AliChatRequest {
    /// 要使用的模型名称，如 "qwen-plus", "qwen-turbo", "qwen-max" 等
    pub model: String,
    /// 对话消息列表
    pub messages: Vec<Message>,
    /// 是否使用流式输出
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// 生成时的随机种子
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed: Option<u32>,
    /// 输出的最大 token 数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// 温度参数，控制生成的随机性
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
    /// Top-p 参数，核采样
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    /// 停止生成的标记
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stop: Option<Vec<String>>,
    /// 结果格式，支持 "text" 或 "message"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_format: Option<String>,
    /// 是否启用增量输出（流式输出专用）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incremental_output: Option<bool>,
}

impl AliChatRequest {
    /// 创建新的聊天请求
    pub fn new(model: String, messages: Vec<Message>) -> Self {
        Self {
            model,
            messages,
            stream: None,
            seed: None,
            max_tokens: None,
            temperature: None,
            top_p: None,
            stop: None,
            result_format: None,
            incremental_output: None,
        }
    }

    /// 设置最大 token 数量
    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    /// 设置温度参数
    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    /// 设置 top_p 参数
    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    /// 设置停止标记
    pub fn with_stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }

    /// 设置增量输出（用于流式）
    pub fn with_incremental_output(mut self, incremental: bool) -> Self {
        self.incremental_output = Some(incremental);
        self
    }
}

impl ChatRequestTrait for AliChatRequest {
    fn get_model(&self) -> &str {
        &self.model
    }

    fn get_messages(&self) -> Vec<Message> {
        self.messages.clone()
    }

    fn set_messages(&mut self, messages: Vec<Message>) {
        self.messages = messages;
    }

    fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    fn message_count(&self) -> usize {
        self.messages.len()
    }

    fn is_stream(&self) -> Option<bool> {
        self.stream
    }

    fn set_stream(&mut self, stream: bool) {
        self.stream = Some(stream);
        // 流式输出时建议启用增量输出
        if stream {
            self.incremental_output = Some(true);
        }
    }

    fn get_options(&self) -> Option<HashMap<String, Value>> {
        let mut options = HashMap::new();
        
        if let Some(seed) = self.seed {
            options.insert("seed".to_string(), Value::from(seed));
        }
        if let Some(max_tokens) = self.max_tokens {
            options.insert("max_tokens".to_string(), Value::from(max_tokens));
        }
        if let Some(temperature) = self.temperature {
            options.insert("temperature".to_string(), Value::from(temperature));
        }
        if let Some(top_p) = self.top_p {
            options.insert("top_p".to_string(), Value::from(top_p));
        }
        if let Some(ref stop) = self.stop {
            options.insert("stop".to_string(), Value::from(stop.clone()));
        }
        if let Some(ref result_format) = self.result_format {
            options.insert("result_format".to_string(), Value::from(result_format.clone()));
        }
        if let Some(incremental_output) = self.incremental_output {
            options.insert("incremental_output".to_string(), Value::from(incremental_output));
        }
        
        if options.is_empty() {
            None
        } else {
            Some(options)
        }
    }

    fn set_options(&mut self, options: HashMap<String, Value>) {
        if let Some(seed) = options.get("seed").and_then(|v| v.as_u64()) {
            self.seed = Some(seed as u32);
        }
        if let Some(max_tokens) = options.get("max_tokens").and_then(|v| v.as_u64()) {
            self.max_tokens = Some(max_tokens as u32);
        }
        if let Some(temperature) = options.get("temperature").and_then(|v| v.as_f64()) {
            self.temperature = Some(temperature as f32);
        }
        if let Some(top_p) = options.get("top_p").and_then(|v| v.as_f64()) {
            self.top_p = Some(top_p as f32);
        }
        if let Some(stop) = options.get("stop").and_then(|v| v.as_array()) {
            let stop_strings: Vec<String> = stop.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.to_string())
                .collect();
            if !stop_strings.is_empty() {
                self.stop = Some(stop_strings);
            }
        }
        if let Some(result_format) = options.get("result_format").and_then(|v| v.as_str()) {
            self.result_format = Some(result_format.to_string());
        }
        if let Some(incremental_output) = options.get("incremental_output").and_then(|v| v.as_bool()) {
            self.incremental_output = Some(incremental_output);
        }
    }

    fn get_format(&self) -> Option<String> {
        self.result_format.clone()
    }

    fn set_format(&mut self, format: String) {
        self.result_format = Some(format);
    }

    fn validate(&self) -> Result<(), String> {
        if self.get_model().is_empty() {
            return Err("Model name cannot be empty".to_string());
        }
        
        // 验证模型名称是否为支持的通义千问模型
        let supported_models = [
            "qwen-plus", "qwen-turbo", "qwen-max", "qwen-max-1201",
            "qwen-max-longcontext", "qwen2.5-72b-instruct", "qwen2.5-32b-instruct",
            "qwen2.5-14b-instruct", "qwen2.5-7b-instruct", "qwen2.5-3b-instruct",
            "qwen2.5-1.5b-instruct", "qwen2.5-0.5b-instruct",
        ];
        
        if !supported_models.contains(&self.get_model()) {
            return Err(format!("Unsupported model: {}. Supported models: {:?}", 
                self.get_model(), supported_models));
        }
        
        if self.message_count() == 0 {
            return Err("Messages cannot be empty".to_string());
        }
        
        // 验证参数范围
        if let Some(temperature) = self.temperature {
            if temperature < 0.0 || temperature > 2.0 {
                return Err("Temperature must be between 0.0 and 2.0".to_string());
            }
        }
        
        if let Some(top_p) = self.top_p {
            if top_p < 0.0 || top_p > 1.0 {
                return Err("Top_p must be between 0.0 and 1.0".to_string());
            }
        }
        
        Ok(())
    }
}

/// 阿里云使用统计信息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AliUsage {
    /// 输入 token 数量
    pub prompt_tokens: u32,
    /// 输出 token 数量
    pub completion_tokens: u32,
    /// 总 token 数量
    pub total_tokens: u32,
    /// 输入 token 详细信息（可选）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_tokens_details: Option<AliPromptTokensDetails>,
}

/// 输入 token 详细信息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AliPromptTokensDetails {
    /// 缓存的 token 数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_tokens: Option<u32>,
}

/// 阿里云 Chat 选择项
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AliChoice {
    /// 生成的消息
    pub message: Message,
    /// 完成原因：stop、length、content_filter 等
    pub finish_reason: String,
    /// 选择项索引
    pub index: usize,
    /// 概率信息（通常为 null）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub logprobs: Option<Value>,
}

/// 阿里云 Chat 响应结构体（OpenAI 兼容格式）
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AliChatResponse {
    /// 响应中的选择项列表
    pub choices: Vec<AliChoice>,
    /// 响应对象类型，通常为 "chat.completion"
    pub object: String,
    /// 使用统计信息
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<AliUsage>,
    /// 响应创建时间戳
    pub created: u64,
    /// 系统指纹（通常为 null）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_fingerprint: Option<String>,
    /// 使用的模型名称
    pub model: String,
    /// 响应 ID
    pub id: String,
}

impl ChatResponseTrait for AliChatResponse {
    fn get_model(&self) -> &str {
        &self.model
    }

    fn get_created_at(&self) -> &str {
        // 由于 trait 要求返回 &str，这里返回 ID 作为时间信息的替代
        // 在实际应用中建议修改 trait 定义返回 String 类型
        &self.id
    }

    fn get_message(&self) -> Option<Message> {
        self.choices.first().map(|choice| choice.message.clone())
    }

    fn is_done(&self) -> bool {
        // 对于非流式响应，始终为完成状态
        true
    }

    fn get_eval_count(&self) -> Option<u32> {
        self.usage.as_ref().map(|usage| usage.completion_tokens)
    }

    fn get_prompt_eval_count(&self) -> Option<u32> {
        self.usage.as_ref().map(|usage| usage.prompt_tokens)
    }
}

/// 阿里云流式响应结构体
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AliStreamResponse {
    /// 响应 ID
    pub id: String,
    /// 响应对象类型
    pub object: String,
    /// 创建时间戳
    pub created: u64,
    /// 使用的模型名称
    pub model: String,
    /// 流式选择项列表
    pub choices: Vec<AliStreamChoice>,
    /// 使用统计（仅在最后一个块中出现）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<AliUsage>,
}

/// 阿里云流式选择项
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AliStreamChoice {
    /// 索引
    pub index: usize,
    /// 增量消息内容
    pub delta: AliDelta,
    /// 完成原因（仅在最后出现）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

/// 阿里云增量内容
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AliDelta {
    /// 角色（仅在第一个块中出现）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub role: Option<String>,
    /// 增量内容
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
}

/// 阿里云客户端错误类型
#[derive(Debug)]
pub enum AliError {
    Client(ClientError),
    Json(serde_json::Error),
    InvalidRequest(String),
    Api(String),
    Auth(String),
}

impl fmt::Display for AliError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AliError::Client(e) => write!(f, "Client error: {}", e),
            AliError::Json(e) => write!(f, "JSON serialization error: {}", e),
            AliError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            AliError::Api(msg) => write!(f, "API error: {}", msg),
            AliError::Auth(msg) => write!(f, "Authentication error: {}", msg),
        }
    }
}

impl std::error::Error for AliError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AliError::Client(e) => Some(e),
            AliError::Json(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ClientError> for AliError {
    fn from(error: ClientError) -> Self {
        AliError::Client(error)
    }
}

impl From<serde_json::Error> for AliError {
    fn from(error: serde_json::Error) -> Self {
        AliError::Json(error)
    }
}

/// 阿里云通义千问客户端
pub struct AliClient {
    /// 基础 HTTP 客户端
    base_client: BaseClient,
    /// API Key
    api_key: String,
    /// API 基础 URL
    base_url: String,
}

impl AliClient {
    /// DashScope API 的默认基础 URL
    pub const DEFAULT_BASE_URL: &'static str = "https://dashscope.aliyuncs.com";

    /// 创建新的阿里云客户端
    pub fn new(api_key: String) -> Result<Self> {
        Self::new_with_base_url(api_key, Self::DEFAULT_BASE_URL.to_string())
    }

    /// 使用自定义基础 URL 创建客户端
    pub fn new_with_base_url(api_key: String, base_url: String) -> Result<Self> {
        let config = ClientConfig::new()
            .add_header("Authorization".to_string(), format!("Bearer {}", api_key))
            .add_header("Content-Type".to_string(), "application/json".to_string());
        
        Self::new_with_config(api_key, base_url, config)
    }

    /// 使用自定义配置创建客户端
    pub fn new_with_config(api_key: String, base_url: String, mut config: ClientConfig) -> Result<Self> {
        // 确保设置了正确的认证头
        config = config
            .add_header("Authorization".to_string(), format!("Bearer {}", api_key))
            .add_header("Content-Type".to_string(), "application/json".to_string());

        let base_client = BaseClient::new(config)?;
        
        Ok(Self {
            base_client,
            api_key,
            base_url,
        })
    }

    /// 使用自定义配置和 HTTP 客户端创建客户端（用于测试）
    pub fn new_with_client(api_key: String, base_url: String, mut config: ClientConfig, client: Client) -> Result<Self> {
        // 确保设置了正确的认证头
        config = config
            .add_header("Authorization".to_string(), format!("Bearer {}", api_key))
            .add_header("Content-Type".to_string(), "application/json".to_string());

        let base_client = BaseClient::new_with_client(config, Some(client))?;
        
        Ok(Self {
            base_client,
            api_key,
            base_url,
        })
    }

    /// 发送聊天请求（非流式）
    pub async fn chat(&self, mut request: AliChatRequest) -> Result<AliChatResponse, AliError> {
        // 确保不是流式请求
        request.set_stream(false);
        
        // 验证请求
        request.validate().map_err(AliError::InvalidRequest)?;

        // 构建完整的 URL
        let url = format!("{}/compatible-mode/v1/chat/completions", self.base_url);

        // 发送请求
        let response = self.base_client.post(&url, &request).await?;
        
        // 解析响应
        let response_text = response.text().await.map_err(|e| {
            AliError::Api(format!("Failed to read response: {}", e))
        })?;

        // 尝试解析错误响应
        if let Ok(error_response) = serde_json::from_str::<Value>(&response_text) {
            if let Some(error) = error_response.get("error") {
                if let Some(message) = error.get("message").and_then(|v| v.as_str()) {
                    return Err(AliError::Api(message.to_string()));
                }
            }
        }

        let chat_response: AliChatResponse = serde_json::from_str(&response_text)?;
        
        Ok(chat_response)
    }

    /// 发送流式聊天请求
    pub async fn chat_stream<F>(&self, mut request: AliChatRequest, mut callback: F) -> Result<(), AliError>
    where
        F: FnMut(AliStreamResponse) -> bool + Send,
    {
        // 确保是流式请求
        request.set_stream(true);
        
        // 验证请求
        request.validate().map_err(AliError::InvalidRequest)?;

        // 构建完整的 URL
        let url = format!("{}/compatible-mode/v1/chat/completions", self.base_url);

        // 发送流式请求
        self.base_client.post_stream(&url, &request, |line: String| {
            // 过滤空行和非数据行
            let line = line.trim();
            if line.is_empty() || !line.starts_with("data: ") {
                return true;
            }

            // 移除 "data: " 前缀
            let json_str = &line[6..];
            
            // 检查是否为结束标记
            if json_str == "[DONE]" {
                return false; // 结束流式处理
            }

            // 解析 JSON 响应
            match serde_json::from_str::<AliStreamResponse>(json_str) {
                Ok(response) => {
                    // 调用用户回调
                    callback(response)
                },
                Err(e) => {
                    eprintln!("Failed to parse streaming response: {}: {}", e, json_str);
                    true // 继续处理其他行
                }
            }
        }).await?;

        Ok(())
    }

    /// 获取 API Key（用于调试，生产环境中应避免暴露）
    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// 获取基础 URL
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[async_trait]
impl LLMClientTrait for AliClient {
    type Request = AliChatRequest;
    type Response = AliChatResponse;
    type Error = AliError;

    async fn send_request(&self, request: Self::Request) -> Result<Self::Response, Self::Error> {
        self.chat(request).await
    }

    async fn send_stream_request<F>(
        &self,
        request: Self::Request,
        callback: F,
    ) -> Result<(), Self::Error>
    where
        F: Fn(String) -> bool + Send + Sync,
    {
        self.chat_stream(request, |response| {
            // 将响应转换为 JSON 字符串
            match serde_json::to_string(&response) {
                Ok(json_str) => callback(json_str),
                Err(_) => false, // 解析失败时停止
            }
        }).await
    }

    fn validate_request(&self, request: &Self::Request) -> Result<(), Self::Error> {
        request.validate().map_err(AliError::InvalidRequest)
    }

    fn client_name(&self) -> &'static str {
        "Ali-DashScope"
    }

    fn base_client(&self) -> &BaseClient {
        &self.base_client
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::llm_api::utils::msg_structure::Message;

    #[test]
    fn test_ali_chat_request_creation() {
        let messages = vec![
            Message::system("You are a helpful assistant.".to_string()),
            Message::user("你是谁？".to_string()),
        ];
        
        let request = AliChatRequest::new("qwen-plus".to_string(), messages);
        
        assert_eq!(request.model, "qwen-plus");
        assert_eq!(request.message_count(), 2);
        assert!(request.validate().is_ok());
    }

    #[test]
    fn test_ali_chat_request_validation() {
        // 测试空模型名称
        let request = AliChatRequest::new("".to_string(), vec![]);
        assert!(request.validate().is_err());
        
        // 测试不支持的模型
        let request = AliChatRequest::new("unsupported-model".to_string(), vec![Message::user("test".to_string())]);
        assert!(request.validate().is_err());
        
        // 测试空消息列表
        let request = AliChatRequest::new("qwen-plus".to_string(), vec![]);
        assert!(request.validate().is_err());
        
        // 测试参数范围
        let mut request = AliChatRequest::new("qwen-plus".to_string(), vec![Message::user("test".to_string())]);
        request.temperature = Some(3.0); // 超出范围
        assert!(request.validate().is_err());
        
        request.temperature = Some(1.0);
        request.top_p = Some(1.5); // 超出范围
        assert!(request.validate().is_err());
    }

    #[test]
    fn test_ali_chat_request_options() {
        let mut request = AliChatRequest::new("qwen-plus".to_string(), vec![Message::user("test".to_string())]);
        
        request = request
            .with_max_tokens(1000)
            .with_temperature(0.7)
            .with_top_p(0.9);
        
        let options = request.get_options().unwrap();
        assert_eq!(options.get("max_tokens").unwrap().as_u64().unwrap(), 1000);
        assert_eq!(options.get("temperature").unwrap().as_f64().unwrap(), 0.7);
        assert_eq!(options.get("top_p").unwrap().as_f64().unwrap(), 0.9);
    }
}