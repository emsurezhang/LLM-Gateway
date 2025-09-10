//! # OllamaClient 演示程序
//!
//! 这是一个独立的演示程序，展示如何使用 OllamaClient
//! 
//! 运行方式：
//! ```bash
//! cargo run --example ollama_client_demo
//! ```

use anyhow::Result;
use std::collections::HashMap;
use serde_json::Value;

use project_rust_learn::llm_api::ollama::client::{OllamaClient, OllamaChatRequest};
use project_rust_learn::llm_api::utils::{
    client::{ClientConfig, TimeoutConfig, RetryConfig},
    msg_structure::Message,
    tool_structure::{Tool, ToolFunction},
    chat_traits::ChatRequestTrait,
};



/// 初始化模型选择
async fn initialize_model(client: &OllamaClient) -> Result<String> {
    println!("📋 检查可用模型...");
    match client.list_models().await {
        Ok(models) => {
            if models.is_empty() {
                println!("❌ 没有找到可用模型");
                println!("请运行: ollama pull llama3.2");
                return Err(anyhow::anyhow!("没有可用模型"));
            }
            println!("✅ 找到模型: {:?}", models);
            
            // 从模型列表中选择一个 llama 模型
            let llama_model = models.iter()
                .find(|model| model.to_lowercase().contains("llama"))
                .cloned();
            
            match llama_model {
                Some(model) => {
                    println!("🎯 选择模型: {}", model);
                    Ok(model)
                }
                None => {
                    println!("❌ 没有找到 llama 模型");
                    println!("请运行: ollama pull llama3.2");
                    Err(anyhow::anyhow!("没有找到 llama 模型"))
                }
            }
        }
        Err(e) => {
            println!("❌ 无法连接到 Ollama: {}", e);
            println!("请确保 Ollama 正在运行：ollama serve");
            Err(anyhow::anyhow!("连接失败: {}", e))
        }
    }
}

/// 基础使用示例
async fn basic_example(model: &str) -> Result<()> {
    println!("=== 基础使用示例 ===");
    
    // 1. 创建客户端
    let client = OllamaClient::new("http://localhost:11434".to_string())?;
    
    // 2. 构建对话
    let messages = vec![
        Message {
            role: "system".to_string(),
            content: "你是一个有用的AI助手".to_string(),
            thinking: None,
            images: None,
            tool_calls: None,
            tool_name: None,
        },
        Message {
            role: "user".to_string(),
            content: "简单介绍一下 Rust 编程语言".to_string(),
            thinking: None,
            images: None,
            tool_calls: None,
            tool_name: None,
        },
    ];
    
    let request = OllamaChatRequest::new(model.to_string(), messages);
    
    // 4. 发送请求
    println!("\n发送请求中...");
    match client.chat(request).await {
        Ok(response) => {
            println!("✅ 请求成功！");
            println!("模型: {}", response.model);
            if let Some(message) = response.message {
                println!("回复: {}", message.content);
            }
            if let Some(duration) = response.total_duration {
                println!("耗时: {:.2}ms", duration as f64 / 1_000_000.0);
            }
        }
        Err(e) => {
            println!("❌ 请求失败: {}", e);
        }
    }
    
    Ok(())
}

/// 流式输出示例
async fn streaming_example(model: &str) -> Result<()> {
    println!("\n=== 流式输出示例 ===");
    
    let client = OllamaClient::new("http://localhost:11434".to_string())?;
    
    let messages = vec![
        Message {
            role: "user".to_string(),
            content: "写一首关于编程的短诗".to_string(),
            thinking: None,
            images: None,
            tool_calls: None,
            tool_name: None,
        },
    ];
    
    let request = OllamaChatRequest::new(model.to_string(), messages);
    
    println!("开始流式输出:");
    print!("回复: ");
    
    match client.chat_stream(request, |response| {
        if let Some(message) = &response.message {
            print!("{}", message.content);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
        }
        !response.done
    }).await {
        Ok(_) => println!("\n✅ 流式输出完成！"),
        Err(e) => println!("\n❌ 流式输出失败: {}", e),
    }
    
    Ok(())
}

/// 自定义配置示例
async fn custom_config_example(model: &str) -> Result<()> {
    println!("\n=== 自定义配置示例 ===");
    
    // 自定义客户端配置
    let config = ClientConfig::new()
        .with_user_agent("OllamaDemo/1.0".to_string())
        .with_timeout(TimeoutConfig::new().with_request_timeout(std::time::Duration::from_secs(30)))
        .with_retry(RetryConfig::new().with_max_attempts(2).with_base_delay(std::time::Duration::from_millis(500)));
    
    let client = OllamaClient::new_with_config(
        "http://localhost:11434".to_string(),
        config
    )?;
    
    let messages = vec![
        Message {
            role: "user".to_string(),
            content: "什么是机器学习？请简短回答".to_string(),
            thinking: None,
            images: None,
            tool_calls: None,
            tool_name: None,
        },
    ];
    
    let mut request = OllamaChatRequest::new(model.to_string(), messages);
    
    // 设置模型参数
    let mut options = HashMap::new();
    options.insert("temperature".to_string(), Value::from(0.3));  // 更保守的回答
    options.insert("max_tokens".to_string(), Value::from(100));   // 限制长度
    request.set_options(options);
    
    println!("发送自定义配置请求...");
    match client.chat(request).await {
        Ok(response) => {
            println!("✅ 自定义配置成功");
            if let Some(message) = response.message {
                println!("回复: {}", message.content);
            }
        }
        Err(e) => {
            println!("❌ 请求失败: {}", e);
        }
    }
    
    Ok(())
}

/// 工具调用示例
async fn tool_example(model: &str) -> Result<()> {
    println!("\n=== 工具调用示例 ===");
    
    let client = OllamaClient::new("http://localhost:11434".to_string())?;
    
    // 定义一个计算器工具
    let calculator_tool = Tool {
        tool_type: "function".to_string(),
        function: ToolFunction {
            name: "calculate".to_string(),
            description: "执行数学计算".to_string(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "expression": {
                        "type": "string",
                        "description": "数学表达式，如：2+3*4"
                    }
                },
                "required": ["expression"]
            }),
        },
    };
    
    let messages = vec![
        Message {
            role: "system".to_string(),
            content: "你可以使用 calculate 工具来计算数学表达式".to_string(),
            thinking: None,
            images: None,
            tool_calls: None,
            tool_name: None,
        },
        Message {
            role: "user".to_string(),
            content: "请计算 15 + 27 * 3".to_string(),
            thinking: None,
            images: None,
            tool_calls: None,
            tool_name: None,
        },
    ];
    
    let request = OllamaChatRequest::new(model.to_string(), messages)
        .with_tools(vec![calculator_tool]);
    
    println!("发送工具调用请求...");
    match client.chat(request).await {
        Ok(response) => {
            println!("✅ 工具调用请求成功");
            if let Some(message) = response.message {
                println!("回复: {}", message.content);
                
                if let Some(tool_calls) = &message.tool_calls {
                    println!("工具调用:");
                    for tool_call in tool_calls {
                        println!("  函数: {}", tool_call.function.name);
                        println!("  参数: {:?}", tool_call.function.arguments);
                    }
                }
            }
        }
        Err(e) => {
            println!("❌ 工具调用失败: {}", e);
            println!("注意：需要支持工具调用的模型");
        }
    }
    
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    println!("🤖 OllamaClient 使用演示");
    println!("=======================");
    println!("请确保:");
    println!("1. Ollama 服务正在运行：ollama serve");
    println!("2. 已安装模型：ollama pull llama3.2");
    println!("3. 服务地址：http://localhost:11434");
    println!("{}", "=".repeat(40));
    
    // 初始化客户端并选择模型
    let client = OllamaClient::new("http://localhost:11434".to_string())?;
    let selected_model = match initialize_model(&client).await {
        Ok(model) => model,
        Err(e) => {
            println!("❌ 初始化失败: {}", e);
            return Ok(());
        }
    };
    
    println!("{}", "=".repeat(40));
    
    // 运行各种示例，使用选定的模型
    basic_example(&selected_model).await?;
    streaming_example(&selected_model).await?;
    custom_config_example(&selected_model).await?;
    tool_example(&selected_model).await?;
    
    println!("\n🎉 演示完成！");
    Ok(())
}
