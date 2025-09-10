//! # 阿里云通义千问客户端演示
//!
//! 演示如何使用阿里云通义千问客户端进行对话
//! 支持两种模式：环境变量模式 和 数据库模式

use std::env;
use project_rust_learn::llm_api::ali::client::{AliClient, AliChatRequest};
use project_rust_learn::llm_api::utils::msg_structure::Message;
use project_rust_learn::llm_api::utils::client_pool::{init_ali_client_pool, get_ali_client_pool};
use project_rust_learn::dao::provider_key_pool::preload::preload_provider_key_pools_to_cache;
use project_rust_learn::dao::SQLITE_POOL;
use project_rust_learn::logger;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    let _logger = logger::init_logger(logger::LogConfig::default()).unwrap();

    // 检查是否有环境变量，如果有就使用单客户端模式，否则使用客户端池模式
    if let Ok(api_key) = env::var("DASHSCOPE_API_KEY") {
        println!("=== 阿里云通义千问客户端演示（环境变量模式）===\n");
        run_single_client_mode(api_key).await?;
    } else {
        println!("=== 阿里云通义千问客户端演示（数据库池模式）===\n");
        run_client_pool_mode().await?;
    }

    Ok(())
}

/// 单客户端模式（使用环境变量）
async fn run_single_client_mode(api_key: String) -> Result<(), Box<dyn std::error::Error>> {
    // 创建客户端
    let client = AliClient::new(api_key)?;

    // 测试非流式对话
    println!("🤖 测试非流式对话:");
    test_chat_single(&client).await?;

    println!("\n{}\n", "=".repeat(50));

    // 测试流式对话
    println!("🌊 测试流式对话:");
    test_stream_chat_single(&client).await?;

    Ok(())
}

/// 客户端池模式（使用数据库中的多个 API Key）
async fn run_client_pool_mode() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化数据库连接池
    match SQLITE_POOL.get() {
        Some(pool) => {
            println!("📦 数据库连接池已就绪");
            
            // 预加载 API Key 到内存
            println!("🔄 正在预加载 API Key 到内存...");
            preload_provider_key_pools_to_cache(pool).await?;
            println!("✅ API Key 预加载完成");
        }
        None => {
            eprintln!("❌ 数据库连接池未初始化");
            return Err("Database pool not initialized".into());
        }
    }

    // 初始化客户端池
    println!("🏊 正在初始化客户端池...");
    init_ali_client_pool(3).await?;
    println!("✅ 客户端池初始化完成");

    // 获取客户端池
    let client_pool = get_ali_client_pool().await?;

    // 测试非流式对话
    println!("🤖 测试非流式对话（自动轮询 API Key）:");
    test_chat_pool(client_pool).await?;

    println!("\n{}\n", "=".repeat(50));

    // 测试流式对话
    println!("🌊 测试流式对话（自动轮询 API Key）:");
    test_stream_chat_pool(client_pool).await?;

    Ok(())
}

/// 测试非流式对话（单客户端）
async fn test_chat_single(client: &AliClient) -> Result<(), Box<dyn std::error::Error>> {
    let messages = vec![
        Message::system("You are a helpful assistant.".to_string()),
        Message::user("你是谁？请简单介绍一下自己。".to_string()),
    ];

    let request = AliChatRequest::new("qwen-plus".to_string(), messages)
        .with_max_tokens(100)
        .with_temperature(0.7);

    match client.chat(request).await {
        Ok(response) => {
            if let Some(choice) = response.choices.first() {
                println!("🤖 回复: {}", choice.message.content);
                
                if let Some(usage) = &response.usage {
                    println!("📊 Token 使用:");
                    println!("   输入: {} tokens", usage.prompt_tokens);
                    println!("   输出: {} tokens", usage.completion_tokens);
                    println!("   总计: {} tokens", usage.total_tokens);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ 请求失败: {}", e);
        }
    }

    Ok(())
}

/// 测试流式对话（单客户端）
async fn test_stream_chat_single(client: &AliClient) -> Result<(), Box<dyn std::error::Error>> {
    let messages = vec![
        Message::system("You are a helpful assistant.".to_string()),
        Message::user("请用50字左右介绍一下人工智能的发展历程。".to_string()),
    ];

    let request = AliChatRequest::new("qwen-plus".to_string(), messages)
        .with_max_tokens(200)
        .with_temperature(0.7);

    print!("🤖 流式回复: ");
    
    let mut full_content = String::new();
    let mut token_count = 0;

    match client.chat_stream(request, |response| {
        if let Some(choice) = response.choices.first() {
            if let Some(content) = &choice.delta.content {
                print!("{}", content);
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
                full_content.push_str(content);
                token_count += 1;
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
            println!("\n✅ 流式对话完成");
            println!("📝 完整回复: {}", full_content);
            println!("🔢 收到 {} 个数据块", token_count);
        }
        Err(e) => {
            eprintln!("\n❌ 流式请求失败: {}", e);
        }
    }

    Ok(())
}

/// 测试非流式对话（客户端池）
async fn test_chat_pool(client_pool: &project_rust_learn::llm_api::utils::client_pool::GlobalAliClientPool) -> Result<(), Box<dyn std::error::Error>> {
    let messages = vec![
        Message::system("You are a helpful assistant.".to_string()),
        Message::user("你是谁？请简单介绍一下自己。".to_string()),
    ];

    let request = AliChatRequest::new("qwen-plus".to_string(), messages)
        .with_max_tokens(100)
        .with_temperature(0.7);

    match client_pool.chat(request).await {
        Ok(response) => {
            if let Some(choice) = response.choices.first() {
                println!("🤖 回复: {}", choice.message.content);
                
                if let Some(usage) = &response.usage {
                    println!("📊 Token 使用:");
                    println!("   输入: {} tokens", usage.prompt_tokens);
                    println!("   输出: {} tokens", usage.completion_tokens);
                    println!("   总计: {} tokens", usage.total_tokens);
                }
            }
        }
        Err(e) => {
            eprintln!("❌ 请求失败: {}", e);
        }
    }

    Ok(())
}

/// 测试流式对话（客户端池）
async fn test_stream_chat_pool(client_pool: &project_rust_learn::llm_api::utils::client_pool::GlobalAliClientPool) -> Result<(), Box<dyn std::error::Error>> {
    let messages = vec![
        Message::system("You are a helpful assistant.".to_string()),
        Message::user("请用50字左右介绍一下人工智能的发展历程。".to_string()),
    ];

    let request = AliChatRequest::new("qwen-plus".to_string(), messages)
        .with_max_tokens(200)
        .with_temperature(0.7);

    print!("🤖 流式回复: ");
    
    let mut full_content = String::new();
    let mut token_count = 0;

    match client_pool.chat_stream(request, |response| {
        if let Some(choice) = response.choices.first() {
            if let Some(content) = &choice.delta.content {
                print!("{}", content);
                std::io::Write::flush(&mut std::io::stdout()).unwrap();
                full_content.push_str(content);
                token_count += 1;
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
            println!("\n✅ 流式对话完成");
            println!("📝 完整回复: {}", full_content);
            println!("🔢 收到 {} 个数据块", token_count);
        }
        Err(e) => {
            eprintln!("\n❌ 流式请求失败: {}", e);
        }
    }

    Ok(())
}
