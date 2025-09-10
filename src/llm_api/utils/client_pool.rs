//! # LLM 客户端池管理
//!
//! 提供客户端池管理功能，支持并发访问和 API Key 轮询

use std::sync::Arc;
use tokio::sync::{Mutex, Semaphore, OnceCell};
use anyhow::Result;
use tracing::{info, warn, error};

use crate::llm_api::ali::client::{AliClient, AliChatRequest, AliChatResponse, AliStreamResponse, AliError};
use crate::llm_api::utils::client::{BaseClient, ClientConfig};
use crate::dao::provider_key_pool::preload::get_api_key_round_robin;

/// 客户端池管理器
pub struct ClientPool<T> {
    clients: Vec<Arc<Mutex<T>>>,
    semaphore: Arc<Semaphore>,
    current_index: std::sync::atomic::AtomicUsize,
}

impl<T> ClientPool<T> {
    pub fn new(clients: Vec<T>) -> Self {
        let size = clients.len();
        Self {
            clients: clients.into_iter().map(|c| Arc::new(Mutex::new(c))).collect(),
            semaphore: Arc::new(Semaphore::new(size)),
            current_index: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    /// 获取可用的客户端
    pub async fn acquire(&self) -> ClientGuard<T> {
        let permit = self.semaphore.clone().acquire_owned().await.unwrap();
        let index = self.current_index.fetch_add(1, std::sync::atomic::Ordering::Relaxed) % self.clients.len();
        let client = self.clients[index].clone();
        
        ClientGuard {
            client,
            _permit: permit,
        }
    }

    /// 获取池大小
    pub fn size(&self) -> usize {
        self.clients.len()
    }
}

/// 客户端守护，自动归还到池中
pub struct ClientGuard<T> {
    client: Arc<Mutex<T>>,
    _permit: tokio::sync::OwnedSemaphorePermit,
}

impl<T> ClientGuard<T> {
    pub async fn lock(&self) -> tokio::sync::MutexGuard<T> {
        self.client.lock().await
    }
}

/// 动态 API Key 的阿里云客户端
pub struct DynamicAliClient {
    base_client: BaseClient,
    base_url: String,
}

impl DynamicAliClient {
    pub fn new() -> Result<Self> {
        let config = ClientConfig::new()
            .add_header("Content-Type".to_string(), "application/json".to_string());
        
        let base_client = BaseClient::new(config)?;
        
        Ok(Self {
            base_client,
            base_url: AliClient::DEFAULT_BASE_URL.to_string(),
        })
    }

    /// 执行聊天请求（自动获取和切换 Key）
    pub async fn chat_with_auto_key(&self, request: AliChatRequest) -> Result<AliChatResponse, AliError> {
        const MAX_RETRIES: usize = 3;
        let mut last_error = None;

        for attempt in 0..MAX_RETRIES {
            // 获取下一个可用的 API Key
            if let Some((api_key, key_id)) = get_api_key_round_robin("ali").await {
                info!("Using API key {} for attempt {}", key_id, attempt + 1);
                
                // 创建临时的 Ali 客户端进行请求
                match AliClient::new(api_key) {
                    Ok(temp_client) => {
                        match temp_client.chat(request.clone()).await {
                            Ok(response) => {
                                info!("Request succeeded with API key {}", key_id);
                                return Ok(response);
                            }
                            Err(e) => {
                                warn!("API Key {} 调用失败 (attempt {}): {}", key_id, attempt + 1, e);
                                
                                // 如果是频率限制错误，标记这个 key（可以扩展实现）
                                let error_msg = e.to_string();
                                if error_msg.contains("rate") || error_msg.contains("quota") {
                                    warn!("API Key {} reached rate limit", key_id);
                                    // TODO: 可以在这里标记 key 为暂时不可用
                                }
                                
                                last_error = Some(e);
                            }
                        }
                    }
                    Err(e) => {
                        error!("Failed to create Ali client with key {}: {}", key_id, e);
                        last_error = Some(AliError::Api(format!("Failed to create client: {}", e)));
                    }
                }
            } else {
                let error = AliError::Api("No available API keys for provider 'ali'".to_string());
                error!("No available API keys for provider 'ali'");
                return Err(error);
            }
        }

        Err(last_error.unwrap_or_else(|| AliError::Api("All retries failed".to_string())))
    }

    /// 执行流式聊天请求（自动获取和切换 Key）
    pub async fn chat_stream_with_auto_key<F>(&self, request: AliChatRequest, callback: F) -> Result<(), AliError>
    where
        F: FnMut(AliStreamResponse) -> bool + Send,
    {
        // 获取 API Key 并创建临时客户端进行流式调用
        if let Some((api_key, key_id)) = get_api_key_round_robin("ali").await {
            info!("Using API key {} for stream request", key_id);
            
            match AliClient::new(api_key) {
                Ok(temp_client) => {
                    match temp_client.chat_stream(request, callback).await {
                        Ok(()) => {
                            info!("Stream request succeeded with API key {}", key_id);
                            Ok(())
                        }
                        Err(e) => {
                            warn!("Stream request failed with API key {}: {}", key_id, e);
                            Err(e)
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to create Ali client for stream with key {}: {}", key_id, e);
                    Err(AliError::Api(format!("Failed to create client for stream: {}", e)))
                }
            }
        } else {
            let error = AliError::Api("No available API keys for provider 'ali'".to_string());
            error!("No available API keys for provider 'ali'");
            Err(error)
        }
    }
}

/// 全局阿里云客户端池
pub struct GlobalAliClientPool {
    pool: ClientPool<DynamicAliClient>,
}

impl GlobalAliClientPool {
    /// 初始化全局客户端池
    pub async fn init(pool_size: usize) -> Result<Self> {
        info!("Initializing global Ali client pool with size: {}", pool_size);
        
        let mut clients = Vec::with_capacity(pool_size);
        
        for i in 0..pool_size {
            match DynamicAliClient::new() {
                Ok(client) => {
                    clients.push(client);
                    info!("Created dynamic Ali client {}/{}", i + 1, pool_size);
                }
                Err(e) => {
                    error!("Failed to create dynamic Ali client {}/{}: {}", i + 1, pool_size, e);
                    return Err(e);
                }
            }
        }

        let pool = ClientPool::new(clients);
        info!("Successfully initialized global Ali client pool with {} clients", pool.size());

        Ok(Self { pool })
    }

    /// 获取客户端进行聊天
    pub async fn chat(&self, request: AliChatRequest) -> Result<AliChatResponse, AliError> {
        let guard = self.pool.acquire().await;
        let client = guard.lock().await;
        client.chat_with_auto_key(request).await
    }

    /// 获取客户端进行流式聊天
    pub async fn chat_stream<F>(&self, request: AliChatRequest, callback: F) -> Result<(), AliError>
    where
        F: FnMut(AliStreamResponse) -> bool + Send,
    {
        let guard = self.pool.acquire().await;
        let client = guard.lock().await;
        client.chat_stream_with_auto_key(request, callback).await
    }

    /// 获取池大小
    pub fn size(&self) -> usize {
        self.pool.size()
    }
}

// 全局单例
static GLOBAL_ALI_POOL: OnceCell<GlobalAliClientPool> = OnceCell::const_new();

/// 初始化全局阿里云客户端池
pub async fn init_ali_client_pool(pool_size: usize) -> Result<()> {
    let pool = GlobalAliClientPool::init(pool_size).await?;
    GLOBAL_ALI_POOL.set(pool).map_err(|_| anyhow::anyhow!("Global Ali client pool already initialized"))?;
    info!("Global Ali client pool initialized successfully");
    Ok(())
}

/// 获取全局阿里云客户端池
pub async fn get_ali_client_pool() -> Result<&'static GlobalAliClientPool> {
    GLOBAL_ALI_POOL.get().ok_or_else(|| {
        anyhow::anyhow!("Global Ali client pool not initialized. Call init_ali_client_pool() first.")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dynamic_ali_client_creation() {
        let client = DynamicAliClient::new();
        assert!(client.is_ok());
    }

    #[tokio::test]
    async fn test_client_pool_creation() {
        let clients = vec![
            DynamicAliClient::new().unwrap(),
            DynamicAliClient::new().unwrap(),
        ];
        
        let pool = ClientPool::new(clients);
        assert_eq!(pool.size(), 2);
    }
}
