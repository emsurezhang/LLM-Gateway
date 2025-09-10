//! # OllamaClient 快速开始示例
//!
//! 这是一个最简单的 OllamaClient 使用示例
//! 
//! 运行前请确保：
//! 1. Ollama 服务正在运行：`ollama serve`
//! 2. 已安装模型：`ollama pull llama3.2`
//!
//! 运行方式：
//! ```bash
//! cargo run --example ollama_quickstart
//! ```

use anyhow::Result;
use project_rust_learn::llm_api::ollama::client::{OllamaClient, OllamaChatRequest};
use project_rust_learn::llm_api::utils::msg_structure::Message;

#[tokio::main]
async fn main() -> Result<()> {
    println!("🚀 OllamaClient 快速开始");
    println!("=====================");
    
    // 1. 创建客户端（使用默认配置）
    let client = OllamaClient::new("http://localhost:11434".to_string())?;
    println!("✅ 客户端创建成功");
    
    // 2. 测试连接并获取模型列表
    println!("\n📋 获取可用模型...");
    let selected_model = match client.list_models().await {
        Ok(models) => {
            if models.is_empty() {
                println!("❌ 没有找到可用模型");
                println!("请运行: ollama pull llama3.2");
                return Ok(());
            }
            println!("✅ 找到模型: {:?}", models);
            
            // 从模型列表中选择一个 llama3.x 模型
            let llama_model = models.iter()
                .find(|model| model.to_lowercase().contains("llama"))
                .cloned();
            
            match llama_model {
                Some(model) => {
                    println!("🎯 选择模型: {}", model);
                    model
                }
                None => {
                    println!("❌ 没有找到 llama3.x 模型");
                    println!("请运行: ollama pull llama3.2");
                    return Ok(());
                }
            }
        }
        Err(e) => {
            println!("❌ 连接失败: {}", e);
            println!("请确保 Ollama 正在运行: ollama serve");
            return Ok(());
        }
    };
    
    // 3. 创建简单对话
    let messages = vec![
        Message {
            role: "user".to_string(),
            content: "你好！请用一句话介绍你自己。".to_string(),
            thinking: None,
            images: None,
            tool_calls: None,
            tool_name: None,
        },
    ];
    
    // 4. 发送请求
    let request = OllamaChatRequest::new(selected_model, messages);
    
    println!("\n💬 发送消息中...");
    match client.chat(request).await {
        Ok(response) => {
            println!("✅ 收到回复:");
            if let Some(message) = response.message {
                println!("🤖 {}", message.content);
            }
            
            // 显示一些统计信息
            if let Some(duration) = response.total_duration {
                println!("\n📊 耗时: {:.2} 秒", duration as f64 / 1_000_000_000.0);
            }
            if let Some(tokens) = response.eval_count {
                println!("📊 生成 tokens: {}", tokens);
            }
        }
        Err(e) => {
            println!("❌ 请求失败: {}", e);
        }
    }
    
    println!("\n🎉 示例完成！");
    println!("\n💡 接下来你可以尝试:");
    println!("   - 修改消息内容");
    println!("   - 尝试不同的模型");
    println!("   - 查看完整示例: cargo run --example ollama_client_demo");
    
    Ok(())
}
