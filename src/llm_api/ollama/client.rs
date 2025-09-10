//! # Ollama API 客户端
//!
//! 实现 Ollama API 的客户端，支持 chat 和 chat_stream 功能
//! 使用 utils 模块提供的通用基础设施

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
    tool_structure::Tool,
};

/// Ollama Chat 请求结构体
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OllamaChatRequest {
    /// 要使用的模型名称
    pub model: String,
    /// 对话消息列表
    pub messages: Vec<Message>,
    /// 是否使用流式输出
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<bool>,
    /// 模型参数选项
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<HashMap<String, Value>>,
    /// 输出格式约束（如 "json"）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    /// 可用工具列表
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tools: Option<Vec<Tool>>,
}

impl OllamaChatRequest {
    /// 创建新的聊天请求
    pub fn new(model: String, messages: Vec<Message>) -> Self {
        Self {
            model,
            messages,
            stream: None,
            options: None,
            format: None,
            tools: None,
        }
    }

    /// 设置工具列表
    pub fn with_tools(mut self, tools: Vec<Tool>) -> Self {
        self.tools = Some(tools);
        self
    }

    /// 添加单个工具
    pub fn add_tool(mut self, tool: Tool) -> Self {
        match self.tools {
            Some(ref mut tools) => tools.push(tool),
            None => self.tools = Some(vec![tool]),
        }
        self
    }
}

impl ChatRequestTrait for OllamaChatRequest {
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
    }

    fn get_options(&self) -> Option<HashMap<String, Value>> {
        self.options.clone()
    }

    fn set_options(&mut self, options: HashMap<String, Value>) {
        self.options = Some(options);
    }

    fn get_format(&self) -> Option<String> {
        self.format.clone()
    }

    fn set_format(&mut self, format: String) {
        self.format = Some(format);
    }
}

/// Ollama Chat 响应结构体
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OllamaChatResponse {
    /// 使用的模型名称
    pub model: String,
    /// 响应创建时间
    pub created_at: String,
    /// AI 生成的消息
    pub message: Option<Message>,
    /// 是否完成（流式输出中使用）
    pub done: bool,
    /// 总处理时间（纳秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_duration: Option<u64>,
    /// 加载时间（纳秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub load_duration: Option<u64>,
    /// 提示词处理时间（纳秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_duration: Option<u64>,
    /// 生成时间（纳秒）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_duration: Option<u64>,
    /// 提示词 token 数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt_eval_count: Option<u32>,
    /// 生成的 token 数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub eval_count: Option<u32>,
}

impl ChatResponseTrait for OllamaChatResponse {
    fn get_model(&self) -> &str {
        &self.model
    }

    fn get_created_at(&self) -> &str {
        &self.created_at
    }

    fn get_message(&self) -> Option<Message> {
        self.message.clone()
    }

    fn is_done(&self) -> bool {
        self.done
    }

    fn get_total_duration(&self) -> Option<u64> {
        self.total_duration
    }

    fn get_eval_count(&self) -> Option<u32> {
        self.eval_count
    }

    fn get_prompt_eval_count(&self) -> Option<u32> {
        self.prompt_eval_count
    }


}

/// Ollama 客户端错误类型
#[derive(Debug)]
pub enum OllamaError {
    Client(ClientError),
    Json(serde_json::Error),
    InvalidRequest(String),
    Api(String),
}

impl fmt::Display for OllamaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OllamaError::Client(e) => write!(f, "Client error: {}", e),
            OllamaError::Json(e) => write!(f, "JSON serialization error: {}", e),
            OllamaError::InvalidRequest(msg) => write!(f, "Invalid request: {}", msg),
            OllamaError::Api(msg) => write!(f, "API error: {}", msg),
        }
    }
}

impl std::error::Error for OllamaError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            OllamaError::Client(e) => Some(e),
            OllamaError::Json(e) => Some(e),
            _ => None,
        }
    }
}

impl From<ClientError> for OllamaError {
    fn from(error: ClientError) -> Self {
        OllamaError::Client(error)
    }
}

impl From<serde_json::Error> for OllamaError {
    fn from(error: serde_json::Error) -> Self {
        OllamaError::Json(error)
    }
}

/// Ollama 客户端
pub struct OllamaClient {
    /// 基础 HTTP 客户端
    base_client: BaseClient,
    /// Ollama 服务器基础 URL
    base_url: String,
}

impl OllamaClient {
    /// 创建新的 Ollama 客户端
    pub fn new(base_url: String) -> Result<Self> {
        Self::new_with_config(base_url, ClientConfig::default())
    }

    /// 使用自定义配置创建客户端
    pub fn new_with_config(base_url: String, config: ClientConfig) -> Result<Self> {
        let base_client = BaseClient::new(config)?;
        
        Ok(Self {
            base_client,
            base_url,
        })
    }

    /// 使用自定义配置和 HTTP 客户端创建客户端（用于测试）
    pub fn new_with_client(base_url: String, config: ClientConfig, client: Client) -> Result<Self> {
        let base_client = BaseClient::new_with_client(config, Some(client))?;
        
        Ok(Self {
            base_client,
            base_url,
        })
    }

    /// 发送聊天请求（非流式）
    pub async fn chat(&self, mut request: OllamaChatRequest) -> Result<OllamaChatResponse, OllamaError> {
        // 确保不是流式请求
        request.set_stream(false);
        
        // 验证请求
        request.validate().map_err(OllamaError::InvalidRequest)?;

        // 构建完整的 URL
        let url = format!("{}/api/chat", self.base_url);

        // 发送请求
        let response = self.base_client.post(&url, &request).await?;
        
        // 解析响应
        let response_text = response.text().await.map_err(|e| {
            OllamaError::Api(format!("Failed to read response: {}", e))
        })?;

        let chat_response: OllamaChatResponse = serde_json::from_str(&response_text)?;
        
        Ok(chat_response)
    }

    /// 发送流式聊天请求
    pub async fn chat_stream<F>(&self, mut request: OllamaChatRequest, mut callback: F) -> Result<(), OllamaError>
    where
        F: FnMut(OllamaChatResponse) -> bool + Send,
    {
        // 确保是流式请求
        request.set_stream(true);
        
        // 验证请求
        request.validate().map_err(OllamaError::InvalidRequest)?;

        // 构建完整的 URL
        let url = format!("{}/api/chat", self.base_url);

        // 发送流式请求
        self.base_client.post_stream(&url, &request, |line: String| {
            // 过滤空行
            if line.trim().is_empty() {
                return true;
            }

            // 解析 JSON 响应
            match serde_json::from_str::<OllamaChatResponse>(&line) {
                Ok(response) => {
                    // 调用用户回调
                    callback(response)
                },
                Err(e) => {
                    eprintln!("Failed to parse streaming response: {}: {}", e, line);
                    true // 继续处理其他行
                }
            }
        }).await?;

        Ok(())
    }

    /// 获取可用模型列表
    pub async fn list_models(&self) -> Result<Vec<String>, OllamaError> {
        let url = format!("{}/api/tags", self.base_url);
        
        let response = self.base_client.http_client()
            .get(&url)
            .send()
            .await
            .map_err(|e| OllamaError::Api(format!("Failed to get models: {}", e)))?;

        let response_text = response.text().await.map_err(|e| {
            OllamaError::Api(format!("Failed to read models response: {}", e))
        })?;

        // 解析模型列表响应
        let models_response: serde_json::Value = serde_json::from_str(&response_text)?;
        
        let mut model_names = Vec::new();
        if let Some(models) = models_response.get("models").and_then(|v| v.as_array()) {
            for model in models {
                if let Some(name) = model.get("name").and_then(|v| v.as_str()) {
                    model_names.push(name.to_string());
                }
            }
        }

        Ok(model_names)
    }

    /// 检查模型是否可用
    pub async fn is_model_available(&self, model_name: &str) -> Result<bool, OllamaError> {
        let models = self.list_models().await?;
        Ok(models.iter().any(|name| name == model_name))
    }
}

#[async_trait]
impl LLMClientTrait for OllamaClient {
    type Request = OllamaChatRequest;
    type Response = OllamaChatResponse;
    type Error = OllamaError;

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
        request.validate().map_err(OllamaError::InvalidRequest)
    }

    fn client_name(&self) -> &'static str {
        "Ollama"
    }

    fn base_client(&self) -> &BaseClient {
        &self.base_client
    }
}
