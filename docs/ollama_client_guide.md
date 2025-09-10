# OllamaClient 使用指南

本指南将详细介绍如何使用 `OllamaClient` 进行各种类型的 AI 对话交互。

## 目录

1. [环境准备](#环境准备)
2. [基础用法](#基础用法)
3. [流式输出](#流式输出)
4. [自定义配置](#自定义配置)
5. [工具调用](#工具调用)
6. [错误处理](#错误处理)
7. [最佳实践](#最佳实践)

## 环境准备

### 1. 安装 Ollama

```bash
# macOS
brew install ollama

# Linux
curl -fsSL https://ollama.ai/install.sh | sh

# Windows
# 从 https://ollama.ai 下载安装包
```

### 2. 启动 Ollama 服务

```bash
ollama serve
```

### 3. 下载模型

```bash
# 下载 Llama 3.2 模型（推荐）
ollama pull llama3.2

# 其他可选模型
ollama pull mistral
ollama pull codellama
ollama pull llava  # 支持图像的多模态模型
```

### 4. 验证安装

```bash
# 列出已安装的模型
ollama list

# 测试模型
ollama run llama3.2 "Hello, world!"
```

## 基础用法

### 最简单的例子

```rust
use anyhow::Result;
use project_rust_learn::llm_api::ollama::client::{OllamaClient, OllamaChatRequest};
use project_rust_learn::llm_api::utils::msg_structure::Message;

#[tokio::main]
async fn main() -> Result<()> {
    // 1. 创建客户端
    let client = OllamaClient::new("http://localhost:11434".to_string())?;
    
    // 2. 构建消息
    let messages = vec![
        Message {
            role: "user".to_string(),
            content: "你好，请介绍一下你自己".to_string(),
            thinking: None,
            images: None,
            tool_calls: None,
            tool_name: None,
        },
    ];
    
    // 3. 创建请求
    let request = OllamaChatRequest::new("llama3.2".to_string(), messages);
    
    // 4. 发送请求
    let response = client.chat(request).await?;
    
    // 5. 处理响应
    if let Some(message) = response.message {
        println!("AI: {}", message.content);
    }
    
    Ok(())
}
```

### 多轮对话

```rust
async fn multi_turn_conversation() -> Result<()> {
    let client = OllamaClient::new("http://localhost:11434".to_string())?;
    
    let mut messages = vec![
        Message {
            role: "system".to_string(),
            content: "你是一个有用的编程助手，专门帮助用户学习 Rust".to_string(),
            thinking: None,
            images: None,
            tool_calls: None,
            tool_name: None,
        },
    ];
    
    // 第一轮对话
    messages.push(Message {
        role: "user".to_string(),
        content: "什么是 Rust 的所有权系统？".to_string(),
        thinking: None,
        images: None,
        tool_calls: None,
        tool_name: None,
    });
    
    let request = OllamaChatRequest::new("llama3.2".to_string(), messages.clone());
    let response = client.chat(request).await?;
    
    if let Some(ai_message) = response.message {
        println!("AI: {}", ai_message.content);
        // 将 AI 的回复添加到对话历史
        messages.push(ai_message);
    }
    
    // 第二轮对话
    messages.push(Message {
        role: "user".to_string(),
        content: "能给我一个所有权的代码示例吗？".to_string(),
        thinking: None,
        images: None,
        tool_calls: None,
        tool_name: None,
    });
    
    let request = OllamaChatRequest::new("llama3.2".to_string(), messages);
    let response = client.chat(request).await?;
    
    if let Some(ai_message) = response.message {
        println!("AI: {}", ai_message.content);
    }
    
    Ok(())
}
```

## 流式输出

流式输出允许您实时接收 AI 的回复，而不必等待完整响应：

```rust
async fn streaming_chat() -> Result<()> {
    let client = OllamaClient::new("http://localhost:11434".to_string())?;
    
    let messages = vec![
        Message {
            role: "user".to_string(),
            content: "请写一个关于人工智能的故事".to_string(),
            thinking: None,
            images: None,
            tool_calls: None,
            tool_name: None,
        },
    ];
    
    let request = OllamaChatRequest::new("llama3.2".to_string(), messages);
    
    print!("AI: ");
    
    client.chat_stream(request, |response| {
        // 实时输出每个 token
        if let Some(message) = &response.message {
            print!("{}", message.content);
            std::io::Write::flush(&mut std::io::stdout()).unwrap();
        }
        
        // 返回 true 继续接收，false 停止
        !response.done
    }).await?;
    
    println!(); // 换行
    Ok(())
}
```

## 自定义配置

### 客户端配置

```rust
use project_rust_learn::llm_api::utils::client::{ClientConfig, TimeoutConfig, RetryConfig};
use std::time::Duration;

async fn custom_client_config() -> Result<()> {
    // 创建自定义配置
    let config = ClientConfig::new()
        .with_user_agent("MyApp/1.0".to_string())
        .with_timeout(
            TimeoutConfig::new()
                .with_request_timeout(Duration::from_secs(60))  // 60秒超时
                .with_connect_timeout(Duration::from_secs(10))  // 10秒连接超时
        )
        .with_retry(
            RetryConfig::new()
                .with_max_attempts(5)                           // 最多重试5次
                .with_base_delay(Duration::from_millis(2000))   // 重试间隔2秒
        );
    
    let client = OllamaClient::new_with_config(
        "http://localhost:11434".to_string(),
        config
    )?;
    
    // 使用配置的客户端...
    Ok(())
}
```

### 模型参数配置

```rust
use std::collections::HashMap;
use serde_json::Value;

async fn model_parameters() -> Result<()> {
    let client = OllamaClient::new("http://localhost:11434".to_string())?;
    
    let messages = vec![
        Message {
            role: "user".to_string(),
            content: "写一首诗".to_string(),
            thinking: None,
            images: None,
            tool_calls: None,
            tool_name: None,
        },
    ];
    
    let mut request = OllamaClient::new("llama3.2".to_string(), messages);
    
    // 设置模型参数
    let mut options = HashMap::new();
    options.insert("temperature".to_string(), Value::from(0.8));    // 创造性
    options.insert("top_p".to_string(), Value::from(0.9));         // 核采样
    options.insert("max_tokens".to_string(), Value::from(200));    // 最大长度
    options.insert("repeat_penalty".to_string(), Value::from(1.1)); // 重复惩罚
    
    request.set_options(options);
    
    // 可选：设置输出格式
    // request.set_format("json".to_string());
    
    let response = client.chat(request).await?;
    
    if let Some(message) = response.message {
        println!("AI: {}", message.content);
    }
    
    Ok(())
}
```

## 工具调用

工具调用允许 AI 使用外部函数和 API：

```rust
use project_rust_learn::llm_api::utils::tool_structure::{Tool, ToolFunction, ToolFunctionParameters};

async fn tool_calling_example() -> Result<()> {
    let client = OllamaClient::new("http://localhost:11434".to_string())?;
    
    // 定义天气查询工具
    let weather_tool = Tool {
        tool_type: "function".to_string(),
        function: ToolFunction {
            name: "get_weather".to_string(),
            description: Some("获取指定城市的天气信息".to_string()),
            parameters: ToolFunctionParameters {
                schema_type: "object".to_string(),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("city".to_string(), serde_json::json!({
                        "type": "string",
                        "description": "城市名称"
                    }));
                    props.insert("unit".to_string(), serde_json::json!({
                        "type": "string",
                        "enum": ["celsius", "fahrenheit"],
                        "description": "温度单位",
                        "default": "celsius"
                    }));
                    props
                },
                required: vec!["city".to_string()],
            },
        },
    };
    
    // 定义计算器工具
    let calculator_tool = Tool {
        tool_type: "function".to_string(),
        function: ToolFunction {
            name: "calculate".to_string(),
            description: Some("执行数学计算".to_string()),
            parameters: ToolFunctionParameters {
                schema_type: "object".to_string(),
                properties: {
                    let mut props = HashMap::new();
                    props.insert("expression".to_string(), serde_json::json!({
                        "type": "string",
                        "description": "要计算的数学表达式，如：2+3*4"
                    }));
                    props
                },
                required: vec!["expression".to_string()],
            },
        },
    };
    
    let messages = vec![
        Message {
            role: "system".to_string(),
            content: "你是一个助手，可以帮助用户查询天气和进行计算。使用提供的工具来回答用户问题。".to_string(),
            thinking: None,
            images: None,
            tool_calls: None,
            tool_name: None,
        },
        Message {
            role: "user".to_string(),
            content: "请查询北京的天气，并计算 25 * 4 + 10 的结果".to_string(),
            thinking: None,
            images: None,
            tool_calls: None,
            tool_name: None,
        },
    ];
    
    let request = OllamaChatRequest::new("llama3.2".to_string(), messages)
        .with_tools(vec![weather_tool, calculator_tool]);
    
    let response = client.chat(request).await?;
    
    if let Some(message) = response.message {
        println!("AI: {}", message.content);
        
        // 检查工具调用
        if let Some(tool_calls) = &message.tool_calls {
            println!("\n工具调用:");
            for tool_call in tool_calls {
                println!("  函数: {}", tool_call.function.name);
                println!("  参数: {}", tool_call.function.arguments);
                
                // 这里您需要实现实际的工具执行逻辑
                // 例如：
                match tool_call.function.name.as_str() {
                    "get_weather" => {
                        println!("  -> 执行天气查询...");
                        // 实际的天气 API 调用
                    }
                    "calculate" => {
                        println!("  -> 执行数学计算...");
                        // 实际的计算逻辑
                    }
                    _ => println!("  -> 未知工具"),
                }
            }
        }
    }
    
    Ok(())
}
```

## 错误处理

良好的错误处理是生产环境的关键：

```rust
use project_rust_learn::llm_api::ollama::client::OllamaError;

async fn error_handling_example() -> Result<()> {
    let client = OllamaClient::new("http://localhost:11434".to_string())?;
    
    let messages = vec![
        Message {
            role: "user".to_string(),
            content: "测试消息".to_string(),
            thinking: None,
            images: None,
            tool_calls: None,
            tool_name: None,
        },
    ];
    
    let request = OllamaChatRequest::new("nonexistent-model".to_string(), messages);
    
    match client.chat(request).await {
        Ok(response) => {
            if let Some(message) = response.message {
                println!("成功: {}", message.content);
            }
        }
        Err(e) => {
            match e {
                OllamaError::Client(client_error) => {
                    println!("客户端错误: {}", client_error);
                    // 可能是网络问题，可以重试
                }
                OllamaError::Api(api_error) => {
                    println!("API 错误: {}", api_error);
                    // 可能是模型不存在或服务器错误
                }
                OllamaError::Json(json_error) => {
                    println!("JSON 解析错误: {}", json_error);
                    // 响应格式问题
                }
                OllamaError::InvalidRequest(msg) => {
                    println!("无效请求: {}", msg);
                    // 请求参数有问题
                }
            }
        }
    }
    
    Ok(())
}

// 带重试的错误处理
async fn resilient_chat(client: &OllamaClient, request: OllamaChatRequest, max_retries: u32) -> Result<Option<String>> {
    let mut attempts = 0;
    
    loop {
        match client.chat(request.clone()).await {
            Ok(response) => {
                return Ok(response.message.map(|m| m.content));
            }
            Err(e) => {
                attempts += 1;
                if attempts >= max_retries {
                    return Err(e.into());
                }
                
                println!("尝试 {} 失败: {}，等待重试...", attempts, e);
                tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
            }
        }
    }
}
```

## 最佳实践

### 1. 资源管理

```rust
// 复用客户端实例
lazy_static::lazy_static! {
    static ref OLLAMA_CLIENT: OllamaClient = {
        OllamaClient::new("http://localhost:11434".to_string())
            .expect("Failed to create Ollama client")
    };
}

async fn use_shared_client() -> Result<()> {
    let response = OLLAMA_CLIENT.chat(request).await?;
    // ...
    Ok(())
}
```

### 2. 配置管理

```rust
#[derive(serde::Deserialize)]
struct AppConfig {
    ollama_url: String,
    default_model: String,
    timeout_seconds: u64,
}

fn load_config() -> AppConfig {
    // 从环境变量或配置文件加载
    AppConfig {
        ollama_url: std::env::var("OLLAMA_URL")
            .unwrap_or_else(|_| "http://localhost:11434".to_string()),
        default_model: std::env::var("DEFAULT_MODEL")
            .unwrap_or_else(|_| "llama3.2".to_string()),
        timeout_seconds: std::env::var("TIMEOUT_SECONDS")
            .unwrap_or_else(|_| "60".to_string())
            .parse()
            .unwrap_or(60),
    }
}
```

### 3. 性能优化

```rust
// 批量处理
async fn batch_chat(requests: Vec<OllamaChatRequest>) -> Result<Vec<OllamaChatResponse>> {
    let client = OllamaClient::new("http://localhost:11434".to_string())?;
    
    // 并发处理（但要控制并发数）
    let futures: Vec<_> = requests.into_iter()
        .map(|req| client.chat(req))
        .collect();
    
    let results = futures::future::join_all(futures).await;
    
    // 收集成功的结果
    let mut responses = Vec::new();
    for result in results {
        match result {
            Ok(response) => responses.push(response),
            Err(e) => eprintln!("请求失败: {}", e),
        }
    }
    
    Ok(responses)
}
```

### 4. 监控和日志

```rust
use tracing::{info, warn, error};

async fn monitored_chat(client: &OllamaClient, request: OllamaChatRequest) -> Result<OllamaChatResponse> {
    let start = std::time::Instant::now();
    
    info!("开始聊天请求，模型: {}", request.get_model());
    
    match client.chat(request).await {
        Ok(response) => {
            let duration = start.elapsed();
            info!(
                "聊天请求成功，耗时: {:.2}ms，tokens: {:?}",
                duration.as_millis(),
                response.eval_count
            );
            Ok(response)
        }
        Err(e) => {
            let duration = start.elapsed();
            error!("聊天请求失败，耗时: {:.2}ms，错误: {}", duration.as_millis(), e);
            Err(e)
        }
    }
}
```

## 运行示例

1. **快速开始**：
   ```bash
   cargo run --example ollama_quickstart
   ```

2. **完整演示**：
   ```bash
   cargo run --example ollama_client_demo
   ```

3. **在自己的项目中使用**：
   ```rust
   // 在 Cargo.toml 中添加依赖
   [dependencies]
   project_rust_learn = { path = "." }
   tokio = { version = "1", features = ["full"] }
   anyhow = "1"
   ```

## 故障排除

### 常见问题

1. **连接失败**：
   - 确保 Ollama 服务正在运行：`ollama serve`
   - 检查端口是否正确（默认 11434）

2. **模型不存在**：
   - 列出可用模型：`ollama list`
   - 下载所需模型：`ollama pull llama3.2`

3. **超时错误**：
   - 增加超时时间配置
   - 检查网络连接和服务器性能

4. **内存不足**：
   - 使用较小的模型
   - 调整 Ollama 的内存限制

### 调试技巧

```rust
// 启用详细日志
RUST_LOG=debug cargo run --example ollama_quickstart

// 检查 Ollama 状态
curl http://localhost:11434/api/tags

// 测试基本连接
curl http://localhost:11434/api/generate -d '{
  "model": "llama3.2",
  "prompt": "Hello, world!",
  "stream": false
}'
```

这个指南涵盖了 OllamaClient 的所有主要功能和最佳实践。根据您的具体需求，您可以选择相应的示例进行参考和修改。
