//! # LLM API Dispatcher
//!
//! 统一的LLM API调度器，支持多个供应商的智能路由和负载均衡
//! 支持Ollama、阿里云、OpenAI等多种LLM供应商

use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;
use std::sync::Arc;
use async_trait::async_trait;
use anyhow::Result;
use std::fmt;

use crate::llm_api::utils::{
    client::ClientError,
    msg_structure::Message,
    chat_traits::{ChatRequestTrait, ChatResponseTrait},
    client_pool::{ClientPool, DynamicAliClient},
};
use crate::llm_api::ali::client::{AliClient, AliChatRequest};
use crate::llm_api::ollama::client::{OllamaClient, OllamaChatRequest};
use crate::dao::{init_sqlite_pool, init_db, SQLITE_POOL};
use crate::dao::cache::init_global_cache;
use crate::dao::provider_key_pool::preload::preload_provider_key_pools_to_cache;

// 定义供应商枚举
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum Provider {
    Ollama,
    Ali,
    OpenAI,
    Claude,
    Gemini,
}

// 定义请求参数
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchRequest {
    pub provider: Provider,
    pub model: String,
    pub messages: Vec<Message>,
    pub stream: Option<bool>,               // 是否流式，默认false
    pub temperature: Option<f32>,           // 控制随机性，0.0-2.0
    pub max_tokens: Option<u32>,           // 最大生成token数
    pub top_p: Option<f32>,                // nucleus sampling参数
    pub frequency_penalty: Option<f32>,     // 频率惩罚
    pub presence_penalty: Option<f32>,      // 存在惩罚
    pub stop: Option<Vec<String>>,         // 停止词
    pub timeout_ms: Option<u64>,           // 请求超时时间(毫秒)
    pub retry_count: Option<u32>,          // 重试次数
    pub context_window: Option<u32>,       // 上下文窗口大小
}

// 定义响应结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DispatchResponse {
    pub content: String,
    pub provider: Provider,
    pub model: String,
    pub usage: Option<TokenUsage>,
    pub finish_reason: Option<String>,
    pub request_id: Option<String>,
    pub created_at: String,
    pub total_duration: Option<u64>,
}

// Token使用统计
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// 定义客户端适配器trait
#[async_trait]
pub trait LLMClientAdapter: Send + Sync {
    async fn generate(&self, request: &DispatchRequest) -> Result<DispatchResponse, LLMError>;
    async fn generate_stream(&self, request: &DispatchRequest) -> Result<tokio::sync::mpsc::Receiver<Result<String, LLMError>>, LLMError>;
    fn supported_models(&self) -> Vec<String>;
    fn provider_name(&self) -> Provider;
}

// 错误定义
#[derive(Debug)]
pub enum LLMError {
    UnsupportedProvider(Provider),
    ModelNotAvailable(String),
    Timeout,
    RateLimit,
    Network(String),
    ApiError(String),
    InvalidParameters(String),
    ClientError(ClientError),
    AnyhowError(anyhow::Error),
}

impl fmt::Display for LLMError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LLMError::UnsupportedProvider(provider) => write!(f, "Provider not supported: {:?}", provider),
            LLMError::ModelNotAvailable(model) => write!(f, "Model not available: {}", model),
            LLMError::Timeout => write!(f, "Request timeout"),
            LLMError::RateLimit => write!(f, "Rate limited"),
            LLMError::Network(msg) => write!(f, "Network error: {}", msg),
            LLMError::ApiError(msg) => write!(f, "API error: {}", msg),
            LLMError::InvalidParameters(msg) => write!(f, "Invalid parameters: {}", msg),
            LLMError::ClientError(e) => write!(f, "Client error: {}", e),
            LLMError::AnyhowError(e) => write!(f, "Anyhow error: {}", e),
        }
    }
}

impl std::error::Error for LLMError {}

impl From<ClientError> for LLMError {
    fn from(err: ClientError) -> Self {
        LLMError::ClientError(err)
    }
}

impl From<anyhow::Error> for LLMError {
    fn from(err: anyhow::Error) -> Self {
        LLMError::AnyhowError(err)
    }
}

// Ollama客户端适配器
pub struct OllamaAdapter {
    client: OllamaClient,
}

impl OllamaAdapter {
    pub fn new(client: OllamaClient) -> Self {
        Self { client }
    }
}

#[async_trait]
impl LLMClientAdapter for OllamaAdapter {
    async fn generate(&self, request: &DispatchRequest) -> Result<DispatchResponse, LLMError> {
        // 构建Ollama请求
        let mut ollama_request = OllamaChatRequest::new(
            request.model.clone(),
            request.messages.clone(),
        );
        
        if let Some(stream) = request.stream {
            ollama_request.set_stream(stream);
        }
        
        // 设置参数
        if request.temperature.is_some() || request.max_tokens.is_some() || request.top_p.is_some() {
            let mut options = std::collections::HashMap::new();
            if let Some(temp) = request.temperature {
                options.insert("temperature".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(temp as f64).unwrap()));
            }
            if let Some(max_tokens) = request.max_tokens {
                options.insert("num_predict".to_string(), serde_json::Value::Number(serde_json::Number::from(max_tokens)));
            }
            if let Some(top_p) = request.top_p {
                options.insert("top_p".to_string(), serde_json::Value::Number(serde_json::Number::from_f64(top_p as f64).unwrap()));
            }
            ollama_request.set_options(options);
        }

        // 执行请求
        let response = self.client.chat(ollama_request).await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;

        // 转换响应
        let content = response.get_content().unwrap_or_default();
        
        Ok(DispatchResponse {
            content,
            provider: Provider::Ollama,
            model: response.get_model().to_string(),
            usage: Some(TokenUsage {
                prompt_tokens: response.get_prompt_eval_count().unwrap_or(0),
                completion_tokens: response.get_eval_count().unwrap_or(0),
                total_tokens: response.get_prompt_eval_count().unwrap_or(0) + response.get_eval_count().unwrap_or(0),
            }),
            finish_reason: if response.is_done() { Some("stop".to_string()) } else { None },
            request_id: None,
            created_at: response.get_created_at().to_string(),
            total_duration: response.get_total_duration(),
        })
    }

    async fn generate_stream(&self, _request: &DispatchRequest) -> Result<tokio::sync::mpsc::Receiver<Result<String, LLMError>>, LLMError> {
        // 简化实现，暂时不支持流式
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let _ = tx.send(Err(LLMError::InvalidParameters("Stream not implemented yet".to_string()))).await;
        Ok(rx)
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            "llama3.2".to_string(),
            "llama3.1:latest".to_string(),
            "llama3".to_string(),
            "qwen-turbo".to_string(),
            "qwen-plus".to_string(),
            "gemma2".to_string(),
            "mistral".to_string(),
            "codellama".to_string(),
        ]
    }

    fn provider_name(&self) -> Provider {
        Provider::Ollama
    }
}

// Ali客户端适配器
pub struct AliAdapter {
    client: AliClient,
}

impl AliAdapter {
    pub fn new(client: AliClient) -> Self {
        Self { client }
    }
}

// Ali客户端池适配器
pub struct AliPoolAdapter {
    pool: Arc<ClientPool<DynamicAliClient>>,
}

impl AliPoolAdapter {
    pub fn new(pool: Arc<ClientPool<DynamicAliClient>>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LLMClientAdapter for AliPoolAdapter {
    async fn generate(&self, request: &DispatchRequest) -> Result<DispatchResponse, LLMError> {
        // 构建Ali请求
        let mut ali_request = AliChatRequest::new(
            request.model.clone(),
            request.messages.clone(),
        );
        
        if let Some(stream) = request.stream {
            ali_request.set_stream(stream);
        }
        
        // 设置参数
        if let Some(temp) = request.temperature {
            ali_request.temperature = Some(temp);
        }
        if let Some(max_tokens) = request.max_tokens {
            ali_request.max_tokens = Some(max_tokens);
        }
        if let Some(top_p) = request.top_p {
            ali_request.top_p = Some(top_p);
        }
        if let Some(stop) = &request.stop {
            ali_request.stop = Some(stop.clone());
        }

        // 从池中获取客户端并执行请求
        let client_guard = self.pool.acquire().await;
        let client = client_guard.lock().await;
        
        let response = client.chat_with_auto_key(ali_request).await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;

        // 转换响应
        let content = response.get_content().unwrap_or_default();
        let model = response.model.clone();
        let usage = response.usage.as_ref().map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });
        let finish_reason = response.choices.first().map(|c| c.finish_reason.clone());
        let request_id = response.id.clone();
        let created_at = response.get_created_at().to_string();
        
        Ok(DispatchResponse {
            content,
            provider: Provider::Ali,
            model,
            usage,
            finish_reason,
            request_id: Some(request_id),
            created_at,
            total_duration: None,
        })
    }

    async fn generate_stream(&self, _request: &DispatchRequest) -> Result<tokio::sync::mpsc::Receiver<Result<String, LLMError>>, LLMError> {
        // 简化实现，暂时不支持流式
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let _ = tx.send(Err(LLMError::InvalidParameters("Stream not implemented yet for pool".to_string()))).await;
        Ok(rx)
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            "qwen-plus".to_string(),
            "qwen-turbo".to_string(),
            "qwen-max".to_string(),
            "qwen-max-longcontext".to_string(),
            "qwen2.5-72b-instruct".to_string(),
            "qwen2.5-32b-instruct".to_string(),
            "qwen2.5-14b-instruct".to_string(),
            "qwen2.5-7b-instruct".to_string(),
        ]
    }

    fn provider_name(&self) -> Provider {
        Provider::Ali
    }
}

#[async_trait]
impl LLMClientAdapter for AliAdapter {
    async fn generate(&self, request: &DispatchRequest) -> Result<DispatchResponse, LLMError> {
        // 构建Ali请求
        let mut ali_request = AliChatRequest::new(
            request.model.clone(),
            request.messages.clone(),
        );
        
        if let Some(stream) = request.stream {
            ali_request.set_stream(stream);
        }
        
        // 设置参数
        if let Some(temp) = request.temperature {
            ali_request.temperature = Some(temp);
        }
        if let Some(max_tokens) = request.max_tokens {
            ali_request.max_tokens = Some(max_tokens);
        }
        if let Some(top_p) = request.top_p {
            ali_request.top_p = Some(top_p);
        }
        if let Some(stop) = &request.stop {
            ali_request.stop = Some(stop.clone());
        }

        // 执行请求
        let response = self.client.chat(ali_request).await
            .map_err(|e| LLMError::ApiError(e.to_string()))?;

        // 转换响应
        let content = response.get_content().unwrap_or_default();
        let model = response.model.clone();
        let usage = response.usage.as_ref().map(|u| TokenUsage {
            prompt_tokens: u.prompt_tokens,
            completion_tokens: u.completion_tokens,
            total_tokens: u.total_tokens,
        });
        let finish_reason = response.choices.first().map(|c| c.finish_reason.clone());
        let request_id = response.id.clone();
        let created_at = response.get_created_at().to_string();
        
        Ok(DispatchResponse {
            content,
            provider: Provider::Ali,
            model,
            usage,
            finish_reason,
            request_id: Some(request_id),
            created_at,
            total_duration: None,
        })
    }

    async fn generate_stream(&self, _request: &DispatchRequest) -> Result<tokio::sync::mpsc::Receiver<Result<String, LLMError>>, LLMError> {
        // 简化实现，暂时不支持流式
        let (tx, rx) = tokio::sync::mpsc::channel(1);
        let _ = tx.send(Err(LLMError::InvalidParameters("Stream not implemented yet".to_string()))).await;
        Ok(rx)
    }

    fn supported_models(&self) -> Vec<String> {
        vec![
            "qwen-plus".to_string(),
            "qwen-turbo".to_string(),
            "qwen-max".to_string(),
            "qwen-max-longcontext".to_string(),
            "qwen2.5-72b-instruct".to_string(),
            "qwen2.5-32b-instruct".to_string(),
            "qwen2.5-14b-instruct".to_string(),
            "qwen2.5-7b-instruct".to_string(),
        ]
    }

    fn provider_name(&self) -> Provider {
        Provider::Ali
    }
}

// Dispatcher主体
pub struct LLMDispatcher {
    clients: Arc<RwLock<HashMap<Provider, Box<dyn LLMClientAdapter>>>>,
    default_config: DispatchConfig,
}

#[derive(Debug, Clone)]
pub struct DispatchConfig {
    pub default_timeout_ms: u64,
    pub default_retry_count: u32,
    pub default_temperature: f32,
    pub enable_fallback: bool,
    pub fallback_providers: Vec<Provider>,
}

impl Default for DispatchConfig {
    fn default() -> Self {
        Self {
            default_timeout_ms: 30000,
            default_retry_count: 3,
            default_temperature: 0.7,
            enable_fallback: true,
            fallback_providers: vec![Provider::Ollama, Provider::Ali],
        }
    }
}

impl LLMDispatcher {
    pub fn new(config: Option<DispatchConfig>) -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
            default_config: config.unwrap_or_default(),
        }
    }

    /// 创建支持数据库的dispatcher，自动初始化数据库和客户端池
    pub async fn new_with_database(config: Option<DispatchConfig>, db_url: &str, init_sql_path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // 初始化数据库连接池
        println!("🔧 正在初始化数据库连接池...");
        init_sqlite_pool(db_url).await;
        
        let pool = match SQLITE_POOL.get() {
            Some(pool) => {
                println!("📦 数据库连接池已就绪");
                pool.clone()
            }
            None => {
                return Err("数据库连接池初始化失败".into());
            }
        };

        // 初始化数据库表结构
        println!("🏗️  正在初始化数据库表结构...");
        match init_db(init_sql_path).await {
            Ok(_) => println!("✅ 数据库表结构初始化完成"),
            Err(e) => {
                eprintln!("❌ 数据库表结构初始化失败: {}", e);
                return Err(e.into());
            }
        }

        // 初始化缓存
        println!("💾 正在初始化内存缓存...");
        match init_global_cache(&pool, 3600, 1000).await {
            Ok(_) => println!("✅ 内存缓存初始化完成"),
            Err(e) => {
                eprintln!("❌ 内存缓存初始化失败: {}", e);
                return Err(e.into());
            }
        }
        
        // 预加载 API Key 到内存
        println!("🔄 正在预加载 API Key 到内存...");
        preload_provider_key_pools_to_cache(&pool).await?;
        println!("✅ API Key 预加载完成");

        // 创建dispatcher
        let dispatcher = Self::new(config);
        
        Ok(dispatcher)
    }

    /// 注册Ali客户端池
    pub async fn register_ali_pool(&self, pool_size: usize) -> Result<(), Box<dyn std::error::Error>> {
        println!("🏊 正在初始化阿里云客户端池...");
        
        // 创建多个DynamicAliClient实例
        let mut clients = Vec::new();
        for _ in 0..pool_size {
            let client = DynamicAliClient::new()?;
            clients.push(client);
        }
        
        let pool = Arc::new(ClientPool::new(clients));
        let adapter = AliPoolAdapter::new(pool);
        
        self.register_client(Box::new(adapter)).await;
        println!("✅ 阿里云客户端池初始化完成 (大小: {})", pool_size);
        
        Ok(())
    }

    // 注册客户端
    pub async fn register_client(&self, client: Box<dyn LLMClientAdapter>) {
        let provider = client.provider_name();
        let mut clients = self.clients.write().await;
        clients.insert(provider, client);
    }

    // 批量注册客户端
    pub async fn register_clients(&self, clients: Vec<Box<dyn LLMClientAdapter>>) {
        for client in clients {
            self.register_client(client).await;
        }
    }

    // 主要的dispatch方法
    pub async fn dispatch(&self, mut request: DispatchRequest) -> Result<DispatchResponse, LLMError> {
        // 应用默认配置
        self.apply_defaults(&mut request);

        // 验证请求参数
        self.validate_request(&request)?;

        // 获取客户端并执行
        let result = self.dispatch_internal(&request).await;

        // 如果启用了fallback且请求失败，尝试备选供应商
        match result {
            Err(e) if self.default_config.enable_fallback => {
                self.try_fallback(request, e).await
            }
            other => other,
        }
    }

    // 流式dispatch
    pub async fn dispatch_stream(&self, mut request: DispatchRequest) -> Result<tokio::sync::mpsc::Receiver<Result<String, LLMError>>, LLMError> {
        self.apply_defaults(&mut request);
        self.validate_request(&request)?;

        let clients = self.clients.read().await;
        let client = clients.get(&request.provider)
            .ok_or_else(|| LLMError::UnsupportedProvider(request.provider.clone()))?;

        client.generate_stream(&request).await
    }

    // 获取所有支持的模型
    pub async fn list_models(&self, provider: Option<Provider>) -> HashMap<Provider, Vec<String>> {
        let clients = self.clients.read().await;
        let mut models = HashMap::new();

        if let Some(p) = provider {
            if let Some(client) = clients.get(&p) {
                models.insert(p, client.supported_models());
            }
        } else {
            for (provider, client) in clients.iter() {
                models.insert(provider.clone(), client.supported_models());
            }
        }

        models
    }

    // 检查供应商是否可用
    pub async fn is_provider_available(&self, provider: &Provider) -> bool {
        let clients = self.clients.read().await;
        clients.contains_key(provider)
    }

    // 内部dispatch实现
    async fn dispatch_internal(&self, request: &DispatchRequest) -> Result<DispatchResponse, LLMError> {
        let clients = self.clients.read().await;
        let client = clients.get(&request.provider)
            .ok_or_else(|| LLMError::UnsupportedProvider(request.provider.clone()))?;

        // 检查模型是否支持
        if !client.supported_models().contains(&request.model) {
            return Err(LLMError::ModelNotAvailable(request.model.clone()));
        }

        // 执行请求，带重试逻辑
        let retry_count = request.retry_count.unwrap_or(self.default_config.default_retry_count);
        let mut last_error = None;

        for attempt in 0..=retry_count {
            match client.generate(request).await {
                Ok(response) => return Ok(response),
                Err(e) => {
                    last_error = Some(e);
                    if attempt < retry_count {
                        // 简单的退避策略
                        tokio::time::sleep(tokio::time::Duration::from_millis(1000 * (attempt + 1) as u64)).await;
                    }
                }
            }
        }

        Err(last_error.unwrap())
    }

    // 尝试备选供应商
    async fn try_fallback(&self, mut request: DispatchRequest, original_error: LLMError) -> Result<DispatchResponse, LLMError> {
        for fallback_provider in &self.default_config.fallback_providers {
            if *fallback_provider == request.provider {
                continue; // 跳过原始供应商
            }

            request.provider = fallback_provider.clone();
            if let Ok(response) = self.dispatch_internal(&request).await {
                return Ok(response);
            }
        }

        // 所有备选都失败，返回原始错误
        Err(original_error)
    }

    // 应用默认配置
    fn apply_defaults(&self, request: &mut DispatchRequest) {
        if request.temperature.is_none() {
            request.temperature = Some(self.default_config.default_temperature);
        }
        if request.timeout_ms.is_none() {
            request.timeout_ms = Some(self.default_config.default_timeout_ms);
        }
        if request.retry_count.is_none() {
            request.retry_count = Some(self.default_config.default_retry_count);
        }
    }

    // 验证请求参数
    fn validate_request(&self, request: &DispatchRequest) -> Result<(), LLMError> {
        if request.messages.is_empty() {
            return Err(LLMError::InvalidParameters("Messages cannot be empty".to_string()));
        }

        if request.model.is_empty() {
            return Err(LLMError::InvalidParameters("Model cannot be empty".to_string()));
        }

        if let Some(temp) = request.temperature {
            if temp < 0.0 || temp > 2.0 {
                return Err(LLMError::InvalidParameters("Temperature must be between 0.0 and 2.0".to_string()));
            }
        }

        Ok(())
    }
}

// 便捷方法
impl DispatchRequest {
    pub fn new(provider: Provider, model: String, messages: Vec<Message>) -> Self {
        Self {
            provider,
            model,
            messages,
            stream: None,
            temperature: None,
            max_tokens: None,
            top_p: None,
            frequency_penalty: None,
            presence_penalty: None,
            stop: None,
            timeout_ms: None,
            retry_count: None,
            context_window: None,
        }
    }

    pub fn with_stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }

    pub fn with_temperature(mut self, temperature: f32) -> Self {
        self.temperature = Some(temperature);
        self
    }

    pub fn with_max_tokens(mut self, max_tokens: u32) -> Self {
        self.max_tokens = Some(max_tokens);
        self
    }

    pub fn with_top_p(mut self, top_p: f32) -> Self {
        self.top_p = Some(top_p);
        self
    }

    pub fn with_stop(mut self, stop: Vec<String>) -> Self {
        self.stop = Some(stop);
        self
    }
}