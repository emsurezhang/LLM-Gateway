//! # 阿里云通义千问客户端池演示
//!
//! 演示如何使用客户端池进行并发对话，自动轮询多个 API Key

use project_rust_learn::llm_api::ali::client::AliChatRequest;
use project_rust_learn::llm_api::utils::msg_structure::Message;
use project_rust_learn::llm_api::utils::client_pool::{init_ali_client_pool, get_ali_client_pool};
use project_rust_learn::dao::provider_key_pool::preload::preload_provider_key_pools_to_cache;
use project_rust_learn::dao::{SQLITE_POOL, init_sqlite_pool, init_db};
use project_rust_learn::dao::cache::init_global_cache;
use project_rust_learn::logger;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    let _logger = logger::init_logger(logger::LogConfig::default()).unwrap();

    println!("=== 阿里云通义千问客户端池演示 ===\n");

    // 初始化数据库连接池
    println!("🔧 正在初始化数据库连接池...");
    init_sqlite_pool("sqlite://data/app.db").await;
    
    let pool = match SQLITE_POOL.get() {
        Some(pool) => {
            println!("📦 数据库连接池已就绪");
            pool.clone()
        }
        None => {
            eprintln!("❌ 数据库连接池初始化失败");
            return Err("Database pool initialization failed".into());
        }
    };

    // 初始化数据库表结构
    println!("🏗️  正在初始化数据库表结构...");
    match init_db("data/init.sql").await {
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

    // 初始化客户端池（5个客户端实例）
    println!("🏊 正在初始化客户端池...");
    init_ali_client_pool(5).await?;
    println!("✅ 客户端池初始化完成");

    // 获取客户端池
    let client_pool = get_ali_client_pool().await?;
    println!("📊 客户端池大小: {}", client_pool.size());

    println!("\n{}\n", "=".repeat(60));

    // 测试单个请求
    println!("🤖 测试单个聊天请求:");
    test_single_chat(client_pool).await?;

    println!("\n{}\n", "=".repeat(60));

    // 测试并发请求
    println!("🚀 测试并发聊天请求:");
    test_concurrent_chat(client_pool).await?;

    println!("\n{}\n", "=".repeat(60));

    // 测试流式对话
    println!("🌊 测试流式对话:");
    test_stream_chat(client_pool).await?;

    Ok(())
}

/// 测试单个聊天请求
async fn test_single_chat(client_pool: &project_rust_learn::llm_api::utils::client_pool::GlobalAliClientPool) -> Result<(), Box<dyn std::error::Error>> {
    let messages = vec![
        Message::system("You are a helpful assistant.".to_string()),
        Message::user("请简单介绍一下你自己，不超过50字。".to_string()),
    ];

    let request = AliChatRequest::new("qwen-plus".to_string(), messages)
        .with_max_tokens(100)
        .with_temperature(0.7);

    let start_time = Instant::now();
    
    match client_pool.chat(request).await {
        Ok(response) => {
            let elapsed = start_time.elapsed();
            
            if let Some(choice) = response.choices.first() {
                println!("🤖 回复: {}", choice.message.content);
                
                if let Some(usage) = &response.usage {
                    println!("📊 Token 使用:");
                    println!("   输入: {} tokens", usage.prompt_tokens);
                    println!("   输出: {} tokens", usage.completion_tokens);
                    println!("   总计: {} tokens", usage.total_tokens);
                }
                
                println!("⏱️ 响应时间: {:.2}s", elapsed.as_secs_f64());
            }
        }
        Err(e) => {
            eprintln!("❌ 请求失败: {}", e);
        }
    }

    Ok(())
}

/// 测试并发聊天请求
async fn test_concurrent_chat(client_pool: &'static project_rust_learn::llm_api::utils::client_pool::GlobalAliClientPool) -> Result<(), Box<dyn std::error::Error>> {
    let concurrent_requests = 8;
    println!("🔄 启动 {} 个并发请求...", concurrent_requests);
    
    let start_time = Instant::now();
    let mut handles = vec![];
    
    for i in 0..concurrent_requests {
        let pool = client_pool;
        let handle = tokio::spawn(async move {
            let messages = vec![
                Message::system("You are a helpful assistant.".to_string()),
                Message::user(format!("这是第 {} 个并发请求，请回复一个数字 {} 和简短问候。", i + 1, i + 1)),
            ];

            let request = AliChatRequest::new("qwen-plus".to_string(), messages)
                .with_max_tokens(50)
                .with_temperature(0.5);

            let req_start = Instant::now();
            
            match pool.chat(request).await {
                Ok(response) => {
                    let req_elapsed = req_start.elapsed();
                    
                    if let Some(choice) = response.choices.first() {
                        println!("✅ 请求 {} 成功 ({:.2}s): {}", 
                            i + 1, 
                            req_elapsed.as_secs_f64(),
                            choice.message.content.chars().take(50).collect::<String>()
                        );
                        
                        if let Some(usage) = &response.usage {
                            println!("   📊 tokens: {}输入 + {}输出 = {}总计", 
                                usage.prompt_tokens, 
                                usage.completion_tokens, 
                                usage.total_tokens
                            );
                        }
                    }
                    true
                }
                Err(e) => {
                    eprintln!("❌ 请求 {} 失败: {}", i + 1, e);
                    false
                }
            }
        });
        handles.push(handle);
    }

    // 等待所有请求完成
    let mut success_count = 0;
    for handle in handles {
        if handle.await.unwrap_or(false) {
            success_count += 1;
        }
    }
    
    let total_elapsed = start_time.elapsed();
    
    println!("\n📈 并发测试结果:");
    println!("   ✅ 成功: {}/{}", success_count, concurrent_requests);
    println!("   ❌ 失败: {}/{}", concurrent_requests - success_count, concurrent_requests);
    println!("   ⏱️ 总时间: {:.2}s", total_elapsed.as_secs_f64());
    println!("   📊 平均 QPS: {:.2}", concurrent_requests as f64 / total_elapsed.as_secs_f64());

    Ok(())
}

/// 测试流式对话
async fn test_stream_chat(client_pool: &project_rust_learn::llm_api::utils::client_pool::GlobalAliClientPool) -> Result<(), Box<dyn std::error::Error>> {
    let messages = vec![
        Message::system("You are a helpful assistant.".to_string()),
        Message::user("请用大约100字介绍一下人工智能的应用领域。".to_string()),
    ];

    let request = AliChatRequest::new("qwen-plus".to_string(), messages)
        .with_max_tokens(200)
        .with_temperature(0.7);

    print!("🤖 流式回复: ");
    
    let mut full_content = String::new();
    let mut chunk_count = 0;
    let start_time = Instant::now();

    match client_pool.chat_stream(request, |response| {
        if let Some(choice) = response.choices.first() {
            if let Some(content) = &choice.delta.content {
                print!("{}", content);
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
                full_content.push_str(content);
                chunk_count += 1;
            }
            
            // 检查是否完成
            if choice.finish_reason.is_some() {
                println!(); // 换行
                if let Some(usage) = &response.usage {
                    println!("📊 Token 使用:");
                    println!("   输入: {} tokens", usage.prompt_tokens);
                    println!("   输出: {} tokens", usage.completion_tokens);
                    println!("   总计: {} tokens", usage.total_tokens);
                }
                return false; // 停止流式输出
            }
        }
        true // 继续接收
    }).await {
        Ok(_) => {
            let elapsed = start_time.elapsed();
            println!("\n✅ 流式对话完成");
            println!("📝 完整回复长度: {} 字符", full_content.chars().count());
            println!("🔢 收到数据块: {} 个", chunk_count);
            println!("⏱️ 总时间: {:.2}s", elapsed.as_secs_f64());
            
            if chunk_count > 0 {
                println!("📊 平均块间隔: {:.0}ms", elapsed.as_millis() as f64 / chunk_count as f64);
            }
        }
        Err(e) => {
            eprintln!("\n❌ 流式请求失败: {}", e);
        }
    }

    Ok(())
}
