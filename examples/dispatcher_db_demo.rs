//! # LLM Dispatcher 数据库版本演示
//!
//! 演示如何使用带数据库支持的LLMDispatcher，从数据库中读取API Key

use tokio;
use project_rust_learn::{
    llm_api::{
        dispatcher::{
            LLMDispatcher, DispatchRequest, DispatchConfig, Provider,
            OllamaAdapter,
        },
        utils::{
            msg_structure::Message,
        },
        ollama::client::OllamaClient,
    },
    logger,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    let _logger = logger::init_logger(logger::LogConfig::default()).unwrap();

    println!("🚀 LLM Dispatcher 数据库版本演示开始");

    // 创建 dispatcher 配置
    let config = DispatchConfig {
        default_timeout_ms: 30000,
        default_retry_count: 2,
        default_temperature: 0.8,
        enable_fallback: true,
        fallback_providers: vec![Provider::Ollama, Provider::Ali],
    };

    // 使用数据库版本创建dispatcher
    println!("📊 正在初始化数据库版本的Dispatcher...");
    let dispatcher = LLMDispatcher::new_with_database(
        Some(config),
        "sqlite://data/app.db",
        "data/init.sql"
    ).await?;

    // 注册 Ollama 客户端
    let ollama_client = OllamaClient::new("http://localhost:11434".to_string())?;
    let ollama_adapter = OllamaAdapter::new(ollama_client);
    dispatcher.register_client(Box::new(ollama_adapter)).await;
    println!("✅ Ollama 客户端已注册");

    // 注册阿里云客户端池（从数据库读取API Key）
    println!("🏊 注册阿里云客户端池...");
    match dispatcher.register_ali_pool(5).await {
        Ok(_) => println!("✅ 阿里云客户端池已注册（池大小：5）"),
        Err(e) => {
            println!("⚠️  阿里云客户端池注册失败: {}", e);
            println!("💡 确保数据库中已配置阿里云API Key");
        }
    }

    // 准备测试消息
    let messages = vec![
        Message {
            role: "user".to_string(),
            content: "请简单介绍一下人工智能的发展历程".to_string(),
            thinking: None,
            images: None,
            tool_calls: None,
            tool_name: None,
        }
    ];

    println!("\n📝 开始测试不同供应商...");

    // 测试 Ollama
    println!("\n🦙 测试 Ollama (llama3.1:latest):");
    if dispatcher.is_provider_available(&Provider::Ollama).await {
        let request = DispatchRequest::new(
            Provider::Ollama,
            "llama3.1:latest".to_string(),
            messages.clone(),
        )
        .with_temperature(0.7)
        .with_max_tokens(200);

        match dispatcher.dispatch(request).await {
            Ok(response) => {
                println!("✅ Ollama 响应:");
                println!("   模型: {}", response.model);
                println!("   内容: {}", response.content);
                if let Some(usage) = response.usage {
                    println!("   Token使用: {}+{}={}", usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
                }
            }
            Err(e) => {
                println!("❌ Ollama 请求失败: {}", e);
                println!("   💡 确保Ollama正在运行且已安装llama3.1模型");
            }
        }
    } else {
        println!("❌ Ollama 不可用");
    }

    // 测试阿里云客户端池
    println!("\n🌟 测试阿里云客户端池 (qwen-turbo):");
    if dispatcher.is_provider_available(&Provider::Ali).await {
        let request = DispatchRequest::new(
            Provider::Ali,
            "qwen-turbo".to_string(),
            messages.clone(),
        )
        .with_temperature(0.7)
        .with_max_tokens(200);

        match dispatcher.dispatch(request).await {
            Ok(response) => {
                println!("✅ 阿里云响应:");
                println!("   模型: {}", response.model);
                println!("   内容: {}", response.content);
                if let Some(usage) = response.usage {
                    println!("   Token使用: {}+{}={}", usage.prompt_tokens, usage.completion_tokens, usage.total_tokens);
                }
                if let Some(request_id) = response.request_id {
                    println!("   请求ID: {}", request_id);
                }
            }
            Err(e) => {
                println!("❌ 阿里云请求失败: {}", e);
                println!("   💡 检查数据库中的API Key配置");
            }
        }
    } else {
        println!("❌ 阿里云不可用");
    }

    // 连续测试多次阿里云请求，验证API Key轮询
    println!("\n🔄 测试API Key轮询机制（连续5次请求）:");
    if dispatcher.is_provider_available(&Provider::Ali).await {
        for i in 1..=5 {
            println!("   第{}次请求:", i);
            let request = DispatchRequest::new(
                Provider::Ali,
                "qwen-turbo".to_string(),
                vec![Message {
                    role: "user".to_string(),
                    content: format!("这是第{}次测试请求，请简单回复", i),
                    thinking: None,
                    images: None,
                    tool_calls: None,
                    tool_name: None,
                }],
            ).with_temperature(0.5).with_max_tokens(50);

            match dispatcher.dispatch(request).await {
                Ok(response) => {
                    println!("     ✅ 成功: {}", response.content.chars().take(50).collect::<String>());
                }
                Err(e) => {
                    println!("     ❌ 失败: {}", e);
                }
            }
        }
    }

    // 列出所有支持的模型
    println!("\n📋 支持的模型列表:");
    let models = dispatcher.list_models(None).await;
    for (provider, model_list) in models {
        println!("   {:?}: {:?}", provider, model_list);
    }

    println!("\n🎉 LLM Dispatcher 数据库版本演示完成!");
    println!("\n💡 使用说明:");
    println!("   1. 确保数据库中已配置阿里云API Key");
    println!("   2. 启动Ollama服务并下载相应模型");
    println!("   3. 客户端池会自动轮询使用不同的API Key");
    println!("   4. 当API Key用完或失效时会自动切换到下一个");
    
    Ok(())
}
