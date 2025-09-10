//! # BaseClient 测试集
//!
//! 测试 BaseClient 类的各项功能：
//! - 配置管理（超时、重试、默认头等）
//! - 错误处理和类型转换
//! - 重试机制和指数退避
//! - 监控指标收集
//! - 请求上下文管理
//! - HTTP 请求发送（使用 mockito 模拟）

use project_rust_learn::llm_api::utils::client::{
    BaseClient, ClientConfig, ClientError, TimeoutConfig, RetryConfig,
    RequestContext, ClientMetrics
};
use project_rust_learn::dao::{init_sqlite_pool, init_db};
use serde_json::json;
use std::time::Duration;
use std::collections::HashMap;
use mockito::Server;


/// 确保数据库只初始化一次
async fn setup_database() {
    use std::sync::atomic::{AtomicBool, Ordering};
    static INITIALIZED: AtomicBool = AtomicBool::new(false);
    
    if !INITIALIZED.swap(true, Ordering::SeqCst) {
        // 初始化数据库连接池
        init_sqlite_pool("sqlite://data/app.db").await;
        // 初始化数据库表结构
        if let Err(e) = init_db("data/init.sql").await {
            eprintln!("Failed to initialize database: {}", e);
        }
        println!("Database initialized for tests");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== 配置类测试 ==========

    #[test]
    fn test_timeout_config_default() {
        let config = TimeoutConfig::default();
        assert_eq!(config.request_timeout, Duration::from_secs(180));
        assert_eq!(config.connect_timeout, Duration::from_secs(30));
        assert_eq!(config.read_timeout, Some(Duration::from_secs(120)));
    }

    #[test]
    fn test_timeout_config_builder() {
        let config = TimeoutConfig::new()
            .with_request_timeout(Duration::from_secs(60))
            .with_connect_timeout(Duration::from_secs(10));
        
        assert_eq!(config.request_timeout, Duration::from_secs(60));
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert_eq!(config.read_timeout, Some(Duration::from_secs(120))); // 默认值保持不变
    }

    #[test]
    fn test_retry_config_default() {
        let config = RetryConfig::default();
        assert_eq!(config.max_attempts, 3);
        assert_eq!(config.base_delay, Duration::from_millis(1000));
        assert_eq!(config.max_delay, Duration::from_secs(30));
        assert!(config.exponential_backoff);
    }

    #[test]
    fn test_retry_config_builder() {
        let config = RetryConfig::new()
            .with_max_attempts(5)
            .with_base_delay(Duration::from_millis(500));
        
        assert_eq!(config.max_attempts, 5);
        assert_eq!(config.base_delay, Duration::from_millis(500));
        assert_eq!(config.max_delay, Duration::from_secs(30)); // 默认值保持不变
        assert!(config.exponential_backoff);
    }

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.timeout.request_timeout, Duration::from_secs(180));
        assert_eq!(config.retry.max_attempts, 3);
        assert!(config.default_headers.is_empty());
        assert_eq!(config.user_agent, "LLM-Client/1.0");
    }

    #[test]
    fn test_client_config_builder() {
        let mut headers = HashMap::new();
        headers.insert("X-Test".to_string(), "test-value".to_string());
        
        let timeout_config = TimeoutConfig::new().with_request_timeout(Duration::from_secs(60));
        let retry_config = RetryConfig::new().with_max_attempts(5);
        
        let config = ClientConfig::new()
            .with_timeout(timeout_config)
            .with_retry(retry_config)
            .add_header("Authorization".to_string(), "Bearer token".to_string())
            .with_user_agent("Test-Agent/1.0".to_string());
        
        assert_eq!(config.timeout.request_timeout, Duration::from_secs(60));
        assert_eq!(config.retry.max_attempts, 5);
        assert_eq!(config.default_headers.get("Authorization"), Some(&"Bearer token".to_string()));
        assert_eq!(config.user_agent, "Test-Agent/1.0");
    }

    // ========== 错误类型测试 ==========

    #[test]
    fn test_client_error_display() {
        let timeout_error = ClientError::Timeout { duration: Duration::from_secs(30) };
        assert!(format!("{}", timeout_error).contains("Request timeout after 30s"));

        let config_error = ClientError::Config { message: "Invalid config".to_string() };
        assert!(format!("{}", config_error).contains("Configuration error: Invalid config"));

        let api_error = ClientError::LLMApi { 
            message: "Rate limit exceeded".to_string(), 
            status_code: Some(429) 
        };
        assert!(format!("{}", api_error).contains("LLM API error: Rate limit exceeded (status: Some(429))"));

        let retry_error = ClientError::RetryExhausted { 
            attempts: 3, 
            last_error: "Network error".to_string() 
        };
        assert!(format!("{}", retry_error).contains("Retry exhausted after 3 attempts: Network error"));
    }

    #[tokio::test]
    async fn test_error_conversions() {
        // 测试 reqwest::Error 转换
        let reqwest_error = reqwest::get("http://[::1]:invalid").await.unwrap_err();
        let client_error: ClientError = reqwest_error.into();
        assert!(matches!(client_error, ClientError::Network { .. }));

        // 测试 serde_json::Error 转换
        let json_error = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        let client_error: ClientError = json_error.into();
        assert!(matches!(client_error, ClientError::Serialization { .. }));
    }

    // ========== RequestContext 测试 ==========

    #[test]
    fn test_request_context_creation() {
        let ctx = RequestContext::new("https://api.example.com/chat", 3, false);
        
        assert!(!ctx.request_id.is_empty());
        assert_eq!(ctx.url, "https://api.example.com/chat");
        assert_eq!(ctx.attempt, 1);
        assert_eq!(ctx.max_attempts, 3);
        assert_eq!(ctx.tokens_output, 0);
        assert!(!ctx.is_stream);
        assert!(ctx.retry_reason.is_none());
        assert!(ctx.model_id.is_none());
        assert!(!ctx.is_final_attempt());
    }

    #[test]
    fn test_request_context_operations() {
        let mut ctx = RequestContext::new("https://api.example.com/chat", 3, true);
        
        // 测试设置模型 ID
        ctx.set_model_id("gpt-3.5-turbo".to_string());
        assert_eq!(ctx.model_id, Some("gpt-3.5-turbo".to_string()));
        
        // 测试添加 token
        ctx.add_tokens(100);
        ctx.add_tokens(50);
        assert_eq!(ctx.tokens_output, 150);
        
        // 测试重试操作
        ctx.start_retry("Network timeout".to_string());
        assert_eq!(ctx.attempt, 2);
        assert_eq!(ctx.retry_reason, Some("Network timeout".to_string()));
        
        // 测试最后一次尝试检查
        ctx.start_retry("API error".to_string());
        assert_eq!(ctx.attempt, 3);
        assert!(ctx.is_final_attempt());
    }

    #[test]
    fn test_request_context_timing() {
        let ctx = RequestContext::new("https://api.example.com/chat", 3, false);
        
        // 测试时间追踪
        let total_elapsed = ctx.total_elapsed();
        let attempt_elapsed = ctx.attempt_elapsed();
        
        // 时间应该很短（刚创建）
        assert!(total_elapsed.as_millis() < 100);
        assert!(attempt_elapsed.as_millis() < 100);
        
        // 总时间和尝试时间应该相近（第一次尝试）
        assert!((total_elapsed.as_millis() as i64 - attempt_elapsed.as_millis() as i64).abs() < 10);
    }

    // ========== ClientMetrics 测试 ==========

    #[test]
    fn test_client_metrics_default() {
        let metrics = ClientMetrics::default();
        
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 0);
        assert_eq!(metrics.retry_count, 0);
        assert_eq!(metrics.avg_response_time, Duration::ZERO);
        assert_eq!(metrics.max_response_time, Duration::ZERO);
        assert_eq!(metrics.min_response_time, Duration::ZERO);
    }

    // ========== BaseClient 构造测试 ==========

    #[test]
    fn test_base_client_creation_default() {
        let client = BaseClient::new_default();
        assert!(client.is_ok());
        
        let client = client.unwrap();
        assert_eq!(client.config().user_agent, "LLM-Client/1.0");
        assert_eq!(client.config().timeout.request_timeout, Duration::from_secs(180));
        assert_eq!(client.config().retry.max_attempts, 3);
    }

    #[test]
    fn test_base_client_creation_with_config() {
        let config = ClientConfig::new()
            .with_user_agent("Test-Client/2.0".to_string())
            .add_header("X-Custom".to_string(), "custom-value".to_string());
        
        let client = BaseClient::new(config);
        assert!(client.is_ok());
        
        let client = client.unwrap();
        assert_eq!(client.config().user_agent, "Test-Client/2.0");
        assert_eq!(client.config().default_headers.get("X-Custom"), Some(&"custom-value".to_string()));
    }

    #[test]
    fn test_base_client_metrics_access() {
        let client = BaseClient::new_default().unwrap();
        let metrics = client.metrics();
        
        // 初始状态应该是空的
        assert_eq!(metrics.total_requests, 0);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 0);
    }

    // ========== HTTP 请求测试（使用 mockito） ==========

    #[tokio::test]
    async fn test_successful_post_request() {
        setup_database().await;
        
        let mut server = Server::new_async().await;
        
        // 设置 mock 服务器响应
        let mock = server.mock("POST", "/api/chat")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message": "Hello, world!"}"#)
            .create_async().await;

        // 创建客户端
        let config = ClientConfig::new()
            .with_timeout(TimeoutConfig::new().with_request_timeout(Duration::from_secs(5)));
        let client = BaseClient::new(config).unwrap();

        // 发送请求
        let request_body = json!({
            "prompt": "Hello",
            "model": "test-model"
        });

        let response = client.post(&format!("{}/api/chat", server.url()), request_body).await;
        
        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(response.status(), 200);
        
        let body = response.text().await.unwrap();
        assert!(body.contains("Hello, world!"));
        
        // 验证 mock 被调用
        mock.assert_async().await;
        
        // 检查指标
        let metrics = client.metrics();
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 1);
        assert_eq!(metrics.failed_requests, 0);
    }

    #[tokio::test]
    async fn test_post_request_with_retry_on_server_error() {
        setup_database().await;
        
        let mut server = Server::new_async().await;
        
        // 第一次请求返回 500 错误，第二次成功
        let mock_error = server.mock("POST", "/api/chat")
            .with_status(500)
            .with_body("Internal Server Error")
            .expect(1)
            .create_async().await;

        let mock_success = server.mock("POST", "/api/chat")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message": "Success after retry"}"#)
            .expect(1)
            .create_async().await;

        // 创建支持重试的客户端
        let config = ClientConfig::new()
            .with_timeout(TimeoutConfig::new().with_request_timeout(Duration::from_secs(5)))
            .with_retry(RetryConfig::new().with_max_attempts(3).with_base_delay(Duration::from_millis(100)));
        let client = BaseClient::new(config).unwrap();

        let request_body = json!({
            "prompt": "Hello",
            "model": "test-model"
        });

        let response = client.post(&format!("{}/api/chat", server.url()), request_body).await;
        
        assert!(response.is_ok());
        let response = response.unwrap();
        assert_eq!(response.status(), 200);
        
        let body = response.text().await.unwrap();
        assert!(body.contains("Success after retry"));
        
        // 验证两个 mock 都被调用了
        mock_error.assert_async().await;
        mock_success.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_request_retry_exhausted() {
        setup_database().await;
        
        let mut server = Server::new_async().await;
        
        // 所有请求都返回 500 错误
        let mock = server.mock("POST", "/api/chat")
            .with_status(500)
            .with_body("Persistent Server Error")
            .expect(3) // 应该重试 3 次
            .create_async().await;

        // 创建最多重试 3 次的客户端
        let config = ClientConfig::new()
            .with_timeout(TimeoutConfig::new().with_request_timeout(Duration::from_secs(5)))
            .with_retry(RetryConfig::new().with_max_attempts(3).with_base_delay(Duration::from_millis(50)));
        let client = BaseClient::new(config).unwrap();

        let request_body = json!({
            "prompt": "Hello",
            "model": "test-model"
        });

        let result = client.post(&format!("{}/api/chat", server.url()), request_body).await;
        
        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(matches!(error, ClientError::RetryExhausted { attempts: 3, .. }));
        
        // 验证 mock 被调用了 3 次
        mock.assert_async().await;
        
        // 检查指标
        let metrics = client.metrics();
        assert_eq!(metrics.total_requests, 1);
        assert_eq!(metrics.successful_requests, 0);
        assert_eq!(metrics.failed_requests, 1);
    }

    #[tokio::test]
    async fn test_post_request_client_error_no_retry() {
        let mut server = Server::new_async().await;
        
        // 返回 400 客户端错误（不应该重试）
        let mock = server.mock("POST", "/api/chat")
            .with_status(400)
            .with_body("Bad Request")
            .expect(1) // 只应该调用一次，不重试
            .create_async().await;

        let config = ClientConfig::new()
            .with_timeout(TimeoutConfig::new().with_request_timeout(Duration::from_secs(5)))
            .with_retry(RetryConfig::new().with_max_attempts(3));
        let client = BaseClient::new(config).unwrap();

        let request_body = json!({
            "prompt": "Hello",
            "model": "test-model"
        });

        let result = client.post(&format!("{}/api/chat", server.url()), request_body).await;
        
        assert!(result.is_err());
        let error = result.unwrap_err();
        // 400 错误不会重试，所以应该是 LLMApi 错误而不是 RetryExhausted
        assert!(matches!(error, ClientError::LLMApi { status_code: Some(400), .. }));
        
        // 验证 mock 只被调用了一次
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_stream_request() {
        let mut server = Server::new_async().await;
        
        // 模拟流式响应
        let stream_data = "data: {\"response\": \"Hello\"}\n\ndata: {\"response\": \" world!\"}\n\ndata: {\"done\": true, \"eval_count\": 10}\n\n";
        let mock = server.mock("POST", "/api/chat/stream")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body(stream_data)
            .create_async().await;

        let config = ClientConfig::new()
            .with_timeout(TimeoutConfig::new().with_request_timeout(Duration::from_secs(5)));
        let client = BaseClient::new(config).unwrap();

        let request_body = json!({
            "prompt": "Hello",
            "model": "test-model",
            "stream": true
        });

        let mut received_chunks = Vec::new();
        let callback = |chunk: String| -> bool {
            received_chunks.push(chunk);
            true // 继续处理
        };

        let result = client.post_stream(&format!("{}/api/chat/stream", server.url()), request_body, callback).await;
        
        assert!(result.is_ok());
        
        // 验证接收到的数据块
        assert!(!received_chunks.is_empty());
        let joined = received_chunks.join("");
        assert!(joined.contains("Hello"));
        assert!(joined.contains("world!"));
        assert!(joined.contains("done"));
        
        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_post_stream_early_termination() {
        let mut server = Server::new_async().await;
        
        let stream_data = "data: {\"response\": \"Hello\"}\n\ndata: {\"response\": \" world!\"}\n\ndata: {\"response\": \" More data...\"}\n\n";
        let mock = server.mock("POST", "/api/chat/stream")
            .with_status(200)
            .with_header("content-type", "text/plain")
            .with_body(stream_data)
            .create_async().await;

        let config = ClientConfig::new()
            .with_timeout(TimeoutConfig::new().with_request_timeout(Duration::from_secs(5)));
        let client = BaseClient::new(config).unwrap();

        let request_body = json!({
            "prompt": "Hello",
            "model": "test-model",
            "stream": true
        });

        let mut chunk_count = 0;
        let callback = |_chunk: String| -> bool {
            chunk_count += 1;
            chunk_count < 2 // 只处理前两个数据块
        };

        let result = client.post_stream(&format!("{}/api/chat/stream", server.url()), request_body, callback).await;
        
        assert!(result.is_ok());
        assert_eq!(chunk_count, 2);
        
        mock.assert_async().await;
    }

    // ========== 重试机制测试 ==========

    #[test]
    fn test_calculate_backoff_delay() {
        // 由于 calculate_backoff_delay 是私有方法，我们通过公开接口间接测试
        let config = ClientConfig::new()
            .with_retry(RetryConfig::new()
                .with_base_delay(Duration::from_millis(100))
                .with_max_attempts(5));
        
        let client = BaseClient::new(config).unwrap();
        
        // 这里我们无法直接测试 calculate_backoff_delay，
        // 但可以通过重试行为来验证指数退避是否正确工作
        assert!(client.config().retry.exponential_backoff);
        assert_eq!(client.config().retry.base_delay, Duration::from_millis(100));
        assert_eq!(client.config().retry.max_delay, Duration::from_secs(30));
    }

    // ========== 边界条件测试 ==========

    #[tokio::test]
    async fn test_post_request_timeout() {
        // 使用一个不存在的地址来模拟超时
        // 这个地址保证不会立即连接成功或失败，而是会超时
        let non_existent_url = "http://10.255.255.1:80/timeout"; // 使用保留的私有地址
        
        // 设置很短的超时时间
        let config = ClientConfig::new()
            .with_timeout(TimeoutConfig::new().with_request_timeout(Duration::from_millis(100)))
            .with_retry(RetryConfig::new().with_max_attempts(1)); // 不重试，直接失败
        let client = BaseClient::new(config).unwrap();

        let request_body = json!({"prompt": "Hello"});
        let result = client.post(non_existent_url, request_body).await;
        
        assert!(result.is_err());
        let error = result.unwrap_err();
        // 可能是超时错误、网络错误或重试耗尽错误，都表示连接失败
        assert!(matches!(error, ClientError::Timeout { .. } | ClientError::Network { .. } | ClientError::RetryExhausted { .. }));
    }

    #[test]
    fn test_empty_config_values() {
        let config = ClientConfig::new()
            .with_user_agent("".to_string()); // 空用户代理
        
        let client = BaseClient::new(config);
        assert!(client.is_ok()); // 应该能正常创建
        
        let client = client.unwrap();
        assert_eq!(client.config().user_agent, "");
    }

    #[test]
    fn test_extreme_retry_config() {
        // 测试极端重试配置
        let config = ClientConfig::new()
            .with_retry(RetryConfig::new()
                .with_max_attempts(0) // 不重试
                .with_base_delay(Duration::ZERO)); // 零延迟
        
        let client = BaseClient::new(config);
        assert!(client.is_ok());
        
        let client = client.unwrap();
        assert_eq!(client.config().retry.max_attempts, 0);
        assert_eq!(client.config().retry.base_delay, Duration::ZERO);
    }

    // ========== 并发测试 ==========

    #[tokio::test]
    async fn test_concurrent_requests() {
        setup_database().await;
        
        let mut server = Server::new_async().await;
        
        // 设置多个 mock 响应
        let mock = server.mock("POST", "/api/chat")
            .with_status(200)
            .with_header("content-type", "application/json")
            .with_body(r#"{"message": "Concurrent response"}"#)
            .expect(10) // 期望 10 个请求
            .create_async().await;

        let config = ClientConfig::new()
            .with_timeout(TimeoutConfig::new().with_request_timeout(Duration::from_secs(5)));
        let client = BaseClient::new(config).unwrap();

        // 并发发送 10 个请求
        let mut handles = Vec::new();
        for i in 0..10 {
            let client_clone = client.clone();
            let url = format!("{}/api/chat", server.url());
            let handle = tokio::spawn(async move {
                let request_body = json!({
                    "prompt": format!("Request {}", i),
                    "model": "test-model"
                });
                client_clone.post(&url, request_body).await
            });
            handles.push(handle);
        }

        // 等待所有請求完成
        let mut success_count = 0;
        for handle in handles {
            let result = handle.await.unwrap();
            if result.is_ok() {
                success_count += 1;
            }
        }

        assert_eq!(success_count, 10);
        mock.assert_async().await;
        
        // 检查指标
        let metrics = client.metrics();
        assert_eq!(metrics.total_requests, 10);
        assert_eq!(metrics.successful_requests, 10);
        assert_eq!(metrics.failed_requests, 0);
    }
}
