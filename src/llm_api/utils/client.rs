//! # 通用 LLM 客户端架构
//!
//! 提供统一的客户端基础设施，包括：
//! - 超时管理和配置
//! - 重试机制和错误处理
//! - 请求/响应监控
//! - 连接池管理
//! - 统一的错误类型

use async_trait::async_trait;
use reqwest::{Client as HttpClient, Response};
use serde::Serialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::time::{sleep, timeout};
use tracing::{info, warn, error};
use uuid::Uuid;
use crate::dao::call_log::{CallLog, create_call_log};

/// 超时配置
#[derive(Debug, Clone)]
pub struct TimeoutConfig {
    /// 总请求超时时间
    pub request_timeout: Duration,
    /// 连接超时时间
    pub connect_timeout: Duration,
    /// 读取超时时间
    pub read_timeout: Option<Duration>,
}

impl Default for TimeoutConfig {
    fn default() -> Self {
        Self {
            request_timeout: Duration::from_secs(180), // 3分钟总超时
            connect_timeout: Duration::from_secs(30),  // 30秒连接超时
            read_timeout: Some(Duration::from_secs(120)), // 2分钟读取超时
        }
    }
}

impl TimeoutConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }


}

/// 重试配置
#[derive(Debug, Clone)]
pub struct RetryConfig {
    /// 最大重试次数
    pub max_attempts: u32,
    /// 基础延迟时间
    pub base_delay: Duration,
    /// 最大延迟时间
    pub max_delay: Duration,
    /// 是否启用指数退避
    pub exponential_backoff: bool,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(1000),
            max_delay: Duration::from_secs(30),
            exponential_backoff: true,
        }
    }
}

impl RetryConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_attempts(mut self, attempts: u32) -> Self {
        self.max_attempts = attempts;
        self
    }

    pub fn with_base_delay(mut self, delay: Duration) -> Self {
        self.base_delay = delay;
        self
    }


}

/// 完整的客户端配置
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// 超时配置
    pub timeout: TimeoutConfig,
    /// 重试配置
    pub retry: RetryConfig,
    /// 默认请求头
    pub default_headers: HashMap<String, String>,
    /// 用户代理
    pub user_agent: String,
}

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            timeout: TimeoutConfig::default(),
            retry: RetryConfig::default(),
            default_headers: HashMap::new(),
            user_agent: "LLM-Client/1.0".to_string(),
        }
    }
}

impl ClientConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_timeout(mut self, timeout: TimeoutConfig) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn with_retry(mut self, retry: RetryConfig) -> Self {
        self.retry = retry;
        self
    }

    pub fn add_header(mut self, key: String, value: String) -> Self {
        self.default_headers.insert(key, value);
        self
    }

    pub fn with_user_agent(mut self, user_agent: String) -> Self {
        self.user_agent = user_agent;
        self
    }


}

/// 客户端错误类型
#[derive(Debug)]
pub enum ClientError {
    /// 请求超时
    Timeout { duration: Duration },
    /// 网络错误
    Network { source: reqwest::Error },
    /// 重试次数耗尽
    RetryExhausted { attempts: u32, last_error: String },
    /// 配置错误
    Config { message: String },
    /// LLM API 错误
    LLMApi { message: String, status_code: Option<u16> },
    /// 序列化错误
    Serialization { source: serde_json::Error },
    /// 内部错误
    Internal { message: String },
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::Timeout { duration } => write!(f, "Request timeout after {:?}", duration),
            ClientError::Network { source } => write!(f, "Network error: {}", source),
            ClientError::RetryExhausted { attempts, last_error } => {
                write!(f, "Retry exhausted after {} attempts: {}", attempts, last_error)
            }
            ClientError::Config { message } => write!(f, "Configuration error: {}", message),
            ClientError::LLMApi { message, status_code } => {
                write!(f, "LLM API error: {} (status: {:?})", message, status_code)
            }
            ClientError::Serialization { source } => write!(f, "Serialization error: {}", source),
            ClientError::Internal { message } => write!(f, "Internal error: {}", message),
        }
    }
}

impl std::error::Error for ClientError {}

impl From<reqwest::Error> for ClientError {
    fn from(error: reqwest::Error) -> Self {
        ClientError::Network { source: error }
    }
}

impl From<serde_json::Error> for ClientError {
    fn from(error: serde_json::Error) -> Self {
        ClientError::Serialization { source: error }
    }
}

/// 请求上下文信息，用于日志记录和问题追踪
#[derive(Debug, Clone)]
pub struct RequestContext {
    /// 请求唯一标识符
    pub request_id: String,
    /// 请求 URL
    pub url: String,
    /// 当前尝试次数
    pub attempt: u32,
    /// 最大重试次数
    pub max_attempts: u32,
    /// 请求开始时间
    pub start_time: Instant,
    /// 当前尝试的开始时间
    pub attempt_start_time: Instant,
    /// 重试原因
    pub retry_reason: Option<String>,
    /// 模型 ID（用于调用记录）
    pub model_id: Option<String>,
    /// 输出 token 数量
    pub tokens_output: i64,
    /// 是否为流式请求
    pub is_stream: bool,
}

impl RequestContext {
    /// 创建新的请求上下文
    pub fn new(url: &str, max_attempts: u32, is_stream: bool) -> Self {
        let now = Instant::now();
        Self {
            request_id: Uuid::new_v4().to_string(),
            url: url.to_string(),
            attempt: 1,
            max_attempts,
            start_time: now,
            attempt_start_time: now,
            retry_reason: None,
            model_id: None,
            tokens_output: 0,
            is_stream,
        }
    }

    /// 设置模型 ID
    pub fn set_model_id(&mut self, model_id: String) {
        self.model_id = Some(model_id);
    }

    /// 增加输出 token 数量
    pub fn add_tokens(&mut self, tokens: i64) {
        self.tokens_output += tokens;
    }

    /// 开始新的重试尝试
    pub fn start_retry(&mut self, reason: String) {
        self.attempt += 1;
        self.attempt_start_time = Instant::now();
        self.retry_reason = Some(reason);
    }

    /// 获取总耗时
    pub fn total_elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// 获取当前尝试耗时
    pub fn attempt_elapsed(&self) -> Duration {
        self.attempt_start_time.elapsed()
    }

    /// 检查是否为最后一次尝试
    pub fn is_final_attempt(&self) -> bool {
        self.attempt >= self.max_attempts
    }
}

/// 客户端监控指标
#[derive(Debug, Clone, Default)]
pub struct ClientMetrics {
    /// 总请求数
    pub total_requests: u64,
    /// 成功请求数
    pub successful_requests: u64,
    /// 失败请求数
    pub failed_requests: u64,
    /// 重试次数
    pub retry_count: u64,
    /// 平均响应时间
    pub avg_response_time: Duration,
    /// 最长响应时间
    pub max_response_time: Duration,
    /// 最短响应时间
    pub min_response_time: Duration,
}

/// 通用 HTTP 客户端
/// 
/// 提供带有超时、重试和监控功能的 HTTP 客户端封装
#[derive(Debug, Clone)]
pub struct BaseClient {
    /// HTTP 客户端
    client: HttpClient,
    /// 客户端配置
    config: ClientConfig,
    /// 监控指标
    metrics: Arc<Mutex<ClientMetrics>>,
}

impl BaseClient {
    /// 创建新的基础客户端
    pub fn new(config: ClientConfig) -> Result<Self, ClientError> {
        Self::new_with_client(config, None)
    }

    /// 创建新的基础客户端，可注入自定义 HTTP 客户端（用于测试）
    pub fn new_with_client(config: ClientConfig, custom_client: Option<HttpClient>) -> Result<Self, ClientError> {
        let client = if let Some(client) = custom_client {
            client
        } else {
            let mut client_builder = HttpClient::builder()
                .no_proxy()
                .timeout(config.timeout.request_timeout)
                .connect_timeout(config.timeout.connect_timeout)
                .user_agent(&config.user_agent);

            // 添加默认请求头
            let mut default_headers = reqwest::header::HeaderMap::new();
            for (key, value) in &config.default_headers {
                if let (Ok(header_name), Ok(header_value)) = (
                    reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                    reqwest::header::HeaderValue::from_str(value),
                ) {
                    default_headers.insert(header_name, header_value);
                }
            }
            client_builder = client_builder.default_headers(default_headers);

            client_builder.build().map_err(|e| ClientError::Config {
                message: format!("Failed to build HTTP client: {}", e),
            })?
        };

        Ok(Self {
            client,
            config,
            metrics: Arc::new(Mutex::new(ClientMetrics::default())),
        })
    }

    /// 使用默认配置创建客户端
    pub fn new_default() -> Result<Self, ClientError> {
        Self::new(ClientConfig::default())
    }



    /// 获取内部 HTTP 客户端
    pub fn http_client(&self) -> &HttpClient {
        &self.client
    }

    /// 获取配置
    pub fn config(&self) -> &ClientConfig {
        &self.config
    }

    /// 获取监控指标
    pub fn metrics(&self) -> ClientMetrics {
        self.metrics.lock().unwrap().clone()
    }

    /// 发送 POST 请求（非流式）
    pub async fn post<T>(&self, url: &str, body: T) -> Result<Response, ClientError>
    where
        T: Serialize + Clone,
    {
        let mut ctx = RequestContext::new(url, self.config.retry.max_attempts, false);
        self.log_request_start(&ctx);

        let mut last_error: Option<ClientError> = None;

        for _ in 1..=self.config.retry.max_attempts {
            // 如果不是第一次尝试，计算延迟并记录重试日志
            if ctx.attempt > 1 {
                let delay = self.calculate_backoff_delay(ctx.attempt - 1);
                self.log_retry_attempt(&ctx, delay);
                sleep(delay).await;
            }

            // 发送请求
            match timeout(
                self.config.timeout.request_timeout,
                self.client.post(url).json(&body).send()
            ).await {
                Ok(Ok(response)) => {
                    let status_code = response.status().as_u16();
                    
                    // 检查响应状态码，如果是错误状态码则处理为错误
                    if !response.status().is_success() {
                        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        
                        // 记录 API 错误
                        self.log_api_error(&ctx, &error_text, Some(status_code));
                        
                        let api_error = ClientError::LLMApi {
                            message: error_text,
                            status_code: Some(status_code),
                        };
                        
                        // 检查是否应该重试
                        if !self.should_retry(&api_error, ctx.attempt) {
                            self.log_request_failure(&ctx, &api_error);
                            self.update_failure_metrics();
                            
                            // 创建失败的调用记录
                            self.create_call_record(&ctx, status_code as i64, Some(format!("{}", api_error))).await;
                            
                            return Err(api_error);
                        }
                        
                        // 检查是否还能重试
                        if ctx.is_final_attempt() {
                            last_error = Some(api_error);
                            break;
                        }
                        
                        // 准备重试
                        ctx.start_retry(format!("API error: {}", status_code));
                        last_error = Some(api_error);
                        continue;
                    } else {
                        // 成功响应
                        let status_code = status_code as i64;
                        self.log_request_success(&ctx);
                        self.update_success_metrics(ctx.total_elapsed());
                        
                        // 创建调用记录（非流式请求完成）
                        self.create_call_record(&ctx, status_code, None).await;
                        
                        return Ok(response);
                    }
                }
                Ok(Err(error)) => {
                    // 记录网络错误详细信息
                    self.log_network_error(&ctx, &error);
                    
                    let client_error = ClientError::Network { source: error };
                    
                    // 检查是否应该重试
                    if !self.should_retry(&client_error, ctx.attempt) {
                        self.log_request_failure(&ctx, &client_error);
                        self.update_failure_metrics();
                        
                        // 创建失败的调用记录
                        self.create_call_record(&ctx, 0, Some(format!("{}", client_error))).await;
                        
                        return Err(client_error);
                    }
                    
                    // 检查是否还能重试
                    if ctx.is_final_attempt() {
                        last_error = Some(client_error);
                        break;
                    }
                    
                    // 准备重试
                    ctx.start_retry("Network error".to_string());
                    last_error = Some(client_error);
                }
                Err(_) => {
                    // 超时错误
                    self.log_timeout_error(&ctx, self.config.timeout.request_timeout);
                    
                    let timeout_error = ClientError::Timeout {
                        duration: self.config.timeout.request_timeout,
                    };
                    
                    // 检查是否还能重试
                    if ctx.is_final_attempt() {
                        last_error = Some(timeout_error);
                        break;
                    }
                    
                    // 准备重试
                    ctx.start_retry("Request timeout".to_string());
                    last_error = Some(timeout_error);
                }
            }
        }

        // 所有重试都失败了
        let final_error = last_error.unwrap_or_else(|| ClientError::Internal {
            message: "Request failed without specific error".to_string(),
        });
        
        self.log_retry_exhausted(&ctx, &format!("{}", final_error));
        self.update_failure_metrics();
        
        let retry_error = ClientError::RetryExhausted {
            attempts: ctx.attempt,
            last_error: format!("{}", final_error),
        };
        
        // 创建重试耗尽的调用记录
        self.create_call_record(&ctx, 0, Some(format!("{}", retry_error))).await;
        
        Err(retry_error)
    }

    /// 发送 POST 流式请求
    pub async fn post_stream<T, F>(&self, url: &str, body: T, mut callback: F) -> Result<(), ClientError>
    where
        T: Serialize + Clone,
        F: FnMut(String) -> bool + Send,
    {
        use futures_util::StreamExt;
        
        let mut ctx = RequestContext::new(url, self.config.retry.max_attempts, true);
        self.log_request_start(&ctx);
        
        let mut stream_completed = false;

        let mut last_error: Option<ClientError> = None;

        for _ in 1..=self.config.retry.max_attempts {
            // 如果不是第一次尝试，计算延迟并记录重试日志
            if ctx.attempt > 1 {
                let delay = self.calculate_backoff_delay(ctx.attempt - 1);
                self.log_retry_attempt(&ctx, delay);
                sleep(delay).await;
            }

            // 发送流式请求
            match timeout(
                self.config.timeout.request_timeout,
                self.client.post(url).json(&body).send()
            ).await {
                Ok(Ok(response)) => {
                    // 检查响应状态
                    if !response.status().is_success() {
                        let status_code = response.status().as_u16();
                        let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                        
                        // 记录 API 错误
                        self.log_api_error(&ctx, &error_text, Some(status_code));
                        
                        let api_error = ClientError::LLMApi {
                            message: error_text,
                            status_code: Some(status_code),
                        };
                        
                        if !self.should_retry(&api_error, ctx.attempt) || ctx.is_final_attempt() {
                            self.log_request_failure(&ctx, &api_error);
                            self.update_failure_metrics();
                            return Err(api_error);
                        }
                        
                        // 准备重试
                        ctx.start_retry(format!("API error: {}", status_code));
                        last_error = Some(api_error);
                        continue;
                    }

                    // 处理流式响应
                    let mut stream = response.bytes_stream();
                    let mut buffer = String::new();
                    let mut total_chunks = 0;
                    
                    info!(
                        request_id = %ctx.request_id,
                        "Starting to process stream response"
                    );
                    
                    while let Some(chunk_result) = stream.next().await {
                        match chunk_result {
                            Ok(chunk) => {
                                total_chunks += 1;
                                let chunk_str = String::from_utf8_lossy(&chunk);
                                buffer.push_str(&chunk_str);
                                
                                // 按行处理数据
                                while let Some(line_end) = buffer.find('\n') {
                                    let line = buffer[..line_end].trim().to_string();
                                    buffer = buffer[line_end + 1..].to_string();
                                    
                                    if !line.is_empty() {
                                        // 检查是否为完成标记（针对 Ollama 等支持 done 字段的响应）
                                        if line.contains("\"done\":true") || line.contains("\"done\": true") {
                                            stream_completed = true;
                                            
                                            // 尝试解析 JSON 以获取 token 信息
                                            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(&line) {
                                                if let Some(eval_count) = json_value.get("eval_count").and_then(|v| v.as_i64()) {
                                                    ctx.add_tokens(eval_count);
                                                }
                                            }
                                        }
                                        
                                        // 调用回调函数，如果返回 false 则停止
                                        if !callback(line) {
                                            info!(
                                                request_id = %ctx.request_id,
                                                total_chunks = total_chunks,
                                                "Stream processing stopped by callback"
                                            );
                                            self.log_request_success(&ctx);
                                            self.update_success_metrics(ctx.total_elapsed());
                                            
                                            // 如果流式请求完成，创建调用记录
                                            if stream_completed {
                                                self.create_call_record(&ctx, 200, None).await;
                                            }
                                            
                                            return Ok(());
                                        }
                                    }
                                }
                            }
                            Err(error) => {
                                error!(
                                    request_id = %ctx.request_id,
                                    total_chunks = total_chunks,
                                    error = %error,
                                    "Stream chunk processing error"
                                );
                                
                                self.log_network_error(&ctx, &error);
                                let client_error = ClientError::Network { source: error };
                                
                                if !self.should_retry(&client_error, ctx.attempt) || ctx.is_final_attempt() {
                                    self.log_request_failure(&ctx, &client_error);
                                    self.update_failure_metrics();
                                    return Err(client_error);
                                }
                                
                                // 准备重试
                                ctx.start_retry("Stream chunk error".to_string());                                
                                break;
                            }
                        }
                    }
                    
                    // 处理剩余的缓冲区内容
                    if !buffer.trim().is_empty() {
                        callback(buffer.trim().to_string());
                    }
                    
                    info!(
                        request_id = %ctx.request_id,
                        total_chunks = total_chunks,
                        stream_completed = stream_completed,
                        "Stream processing completed successfully"
                    );
                    
                    self.log_request_success(&ctx);
                    self.update_success_metrics(ctx.total_elapsed());
                    
                    // 如果流式请求完成，创建调用记录
                    if stream_completed {
                        self.create_call_record(&ctx, 200, None).await;
                    }
                    
                    return Ok(());
                }
                Ok(Err(error)) => {
                    self.log_network_error(&ctx, &error);
                    let client_error = ClientError::Network { source: error };
                    
                    if !self.should_retry(&client_error, ctx.attempt) || ctx.is_final_attempt() {
                        self.log_request_failure(&ctx, &client_error);
                        self.update_failure_metrics();
                        return Err(client_error);
                    }
                    
                    // 准备重试
                    ctx.start_retry("Network error".to_string());
                    last_error = Some(client_error);
                }
                Err(_) => {
                    // 超时错误
                    self.log_timeout_error(&ctx, self.config.timeout.request_timeout);
                    
                    let timeout_error = ClientError::Timeout {
                        duration: self.config.timeout.request_timeout,
                    };
                    
                    if ctx.is_final_attempt() {
                        self.log_request_failure(&ctx, &timeout_error);
                        self.update_failure_metrics();
                        return Err(timeout_error);
                    }
                    
                    // 准备重试
                    ctx.start_retry("Request timeout".to_string());
                    last_error = Some(timeout_error);
                }
            }
        }

        // 所有重试都失败了
        let final_error = last_error.unwrap_or_else(|| ClientError::Internal {
            message: "Stream request failed without specific error".to_string(),
        });
        
        self.log_retry_exhausted(&ctx, &format!("{}", final_error));
        self.update_failure_metrics();
        
        let retry_error = ClientError::RetryExhausted {
            attempts: ctx.attempt,
            last_error: format!("{}", final_error),
        };
        
        // 创建流式请求重试耗尽的调用记录
        self.create_call_record(&ctx, 0, Some(format!("{}", retry_error))).await;
        
        Err(retry_error)
    }

    /// 计算回退延迟时间
    fn calculate_backoff_delay(&self, attempt: u32) -> Duration {
        let base_delay = self.config.retry.base_delay;
        let max_delay = self.config.retry.max_delay;

        let delay = if self.config.retry.exponential_backoff {
            let exponential = base_delay * (2_u32.pow(attempt.saturating_sub(1)));
            std::cmp::min(exponential, max_delay)
        } else {
            base_delay
        };

        std::cmp::min(delay, max_delay)
    }

    /// 判断错误类型是否可以重试（不考虑重试次数限制）
    fn should_retry(&self, error: &ClientError, _attempt: u32) -> bool {
        match error {
            ClientError::Timeout { .. } => true,
            ClientError::Network { source } => {
                source.is_timeout() || source.is_connect() || source.is_request()
            }
            ClientError::LLMApi { status_code, .. } => {
                // 5xx 服务器错误可以重试，4xx 客户端错误不重试
                status_code.map_or(false, |code| code >= 500)
            }
            _ => false,
        }
    }

    /// 更新成功指标
    fn update_success_metrics(&self, response_time: Duration) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.total_requests += 1;
            metrics.successful_requests += 1;
            
            // 更新响应时间统计
            if metrics.successful_requests == 1 {
                metrics.min_response_time = response_time;
                metrics.max_response_time = response_time;
                metrics.avg_response_time = response_time;
            } else {
                if response_time < metrics.min_response_time {
                    metrics.min_response_time = response_time;
                }
                if response_time > metrics.max_response_time {
                    metrics.max_response_time = response_time;
                }
                
                // 计算平均响应时间
                let total_time = metrics.avg_response_time * (metrics.successful_requests - 1) as u32 + response_time;
                metrics.avg_response_time = total_time / metrics.successful_requests as u32;
            }
        }
    }

    /// 更新失败指标
    fn update_failure_metrics(&self) {
        if let Ok(mut metrics) = self.metrics.lock() {
            metrics.total_requests += 1;
            metrics.failed_requests += 1;
        }
    }

    /// 记录请求开始日志
    fn log_request_start(&self, ctx: &RequestContext) {
        info!(
            request_id = %ctx.request_id,
            url = %ctx.url,
            attempt = ctx.attempt,
            max_attempts = ctx.max_attempts,
            "Starting HTTP request"
        );
    }

    /// 记录重试日志
    fn log_retry_attempt(&self, ctx: &RequestContext, delay: Duration) {
        warn!(
            request_id = %ctx.request_id,
            url = %ctx.url,
            attempt = ctx.attempt,
            max_attempts = ctx.max_attempts,
            delay_ms = delay.as_millis(),
            retry_reason = ctx.retry_reason.as_deref().unwrap_or("unknown"),
            total_elapsed_ms = ctx.total_elapsed().as_millis(),
            "Retrying request after error"
        );
    }

    /// 记录请求成功日志
    fn log_request_success(&self, ctx: &RequestContext) {
        info!(
            request_id = %ctx.request_id,
            url = %ctx.url,
            attempt = ctx.attempt,
            total_elapsed_ms = ctx.total_elapsed().as_millis(),
            attempt_elapsed_ms = ctx.attempt_elapsed().as_millis(),
            "Request completed successfully"
        );
    }

    /// 记录请求失败日志
    fn log_request_failure(&self, ctx: &RequestContext, error: &ClientError) {
        error!(
            request_id = %ctx.request_id,
            url = %ctx.url,
            attempt = ctx.attempt,
            max_attempts = ctx.max_attempts,
            total_elapsed_ms = ctx.total_elapsed().as_millis(),
            error = %error,
            "Request failed"
        );
    }

    /// 记录网络错误详细信息
    fn log_network_error(&self, ctx: &RequestContext, error: &reqwest::Error) {
        let error_details = format!(
            "is_timeout: {}, is_connect: {}, is_request: {}, status: {:?}",
            error.is_timeout(),
            error.is_connect(),
            error.is_request(),
            error.status()
        );

        error!(
            request_id = %ctx.request_id,
            url = %ctx.url,
            attempt = ctx.attempt,
            error_type = "network_error",
            error_details = %error_details,
            error_message = %error,
            "Network error occurred"
        );
    }

    /// 记录超时错误
    fn log_timeout_error(&self, ctx: &RequestContext, timeout_duration: Duration) {
        error!(
            request_id = %ctx.request_id,
            url = %ctx.url,
            attempt = ctx.attempt,
            timeout_duration_ms = timeout_duration.as_millis(),
            actual_elapsed_ms = ctx.attempt_elapsed().as_millis(),
            error_type = "timeout_error",
            "Request timeout occurred"
        );
    }

    /// 记录 API 错误
    fn log_api_error(&self, ctx: &RequestContext, message: &str, status_code: Option<u16>) {
        error!(
            request_id = %ctx.request_id,
            url = %ctx.url,
            attempt = ctx.attempt,
            status_code = status_code,
            error_type = "api_error",
            error_message = %message,
            "LLM API error occurred"
        );
    }

    /// 记录重试耗尽错误
    fn log_retry_exhausted(&self, ctx: &RequestContext, final_error: &str) {
        error!(
            request_id = %ctx.request_id,
            url = %ctx.url,
            total_attempts = ctx.attempt,
            total_elapsed_ms = ctx.total_elapsed().as_millis(),
            final_error = %final_error,
            error_type = "retry_exhausted",
            "All retry attempts exhausted"
        );
    }

    /// 创建调用记录
    async fn create_call_record(&self, ctx: &RequestContext, status_code: i64, error_message: Option<String>) {
        use crate::dao::SQLITE_POOL;
        
        // 获取数据库连接池
        if let Some(pool) = SQLITE_POOL.get() {
            let call_log = CallLog {
                id: ctx.request_id.clone(),
                model_id: ctx.model_id.clone(),
                status_code,
                total_duration: ctx.total_elapsed().as_millis() as i64,
                tokens_output: ctx.tokens_output,
                error_message,
                created_at: None, // 将在数据库中设置为当前时间
            };

            if let Err(e) = create_call_log(pool, &call_log).await {
                error!(
                    request_id = %ctx.request_id,
                    error = %e,
                    "Failed to create call log record"
                );
            } else {
                info!(
                    request_id = %ctx.request_id,
                    model_id = ctx.model_id.as_deref().unwrap_or("unknown"),
                    status_code = status_code,
                    total_duration_ms = call_log.total_duration,
                    tokens_output = call_log.tokens_output,
                    "Call log record created successfully"
                );
            }
        } else {
            warn!(
                request_id = %ctx.request_id,
                "Database pool not available, cannot create call log record"
            );
        }
    }
}

/// LLM 客户端特征 trait
/// 
/// 定义所有 LLM 客户端必须实现的核心接口
#[async_trait]
pub trait LLMClientTrait {
    type Request: Send + Sync;
    type Response: Send + Sync;
    type Error: Send + Sync + std::error::Error;

    /// 发送单次请求
    async fn send_request(&self, request: Self::Request) -> Result<Self::Response, Self::Error>;

    /// 发送流式请求
    async fn send_stream_request<F>(
        &self,
        request: Self::Request,
        callback: F,
    ) -> Result<(), Self::Error>
    where
        F: Fn(String) -> bool + Send + Sync;

    /// 验证请求
    fn validate_request(&self, request: &Self::Request) -> Result<(), Self::Error>;

    /// 获取客户端名称
    fn client_name(&self) -> &'static str;

    /// 获取基础 HTTP 客户端
    fn base_client(&self) -> &BaseClient;
}


