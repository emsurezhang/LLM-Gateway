# LLM Dispatcher 使用指南

LLM Dispatcher 是一个统一的LLM API调度器，支持多个供应商的智能路由和负载均衡。

## 特性

- 🔀 **统一接口**: 一个接口调用多种LLM供应商
- 🔄 **自动Fallback**: 当主供应商失败时自动切换到备选供应商  
- ⚡ **异步支持**: 全异步设计，支持高并发
- 🎛️ **参数统一**: 统一的参数格式，自动适配不同供应商
- 🔧 **灵活配置**: 支持超时、重试、温度等参数配置
- 📊 **使用统计**: 返回Token使用量等统计信息

## 支持的供应商

- **Ollama**: 本地LLM服务 (llama3.2, qwen2.5, gemma2等)
- **阿里云**: 通义千问系列 (qwen-plus, qwen-turbo, qwen-max等)
- **OpenAI**: GPT系列 (即将支持)
- **Claude**: Anthropic Claude (即将支持)

## 快速开始

### 1. 基础使用

```rust
use project_rust_learn::llm_api::{
    dispatcher::{LLMDispatcher, DispatchRequest, Provider, OllamaAdapter},
    utils::msg_structure::Message,
    ollama::client::OllamaClient,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 创建dispatcher
    let dispatcher = LLMDispatcher::new(None);

    // 注册Ollama客户端
    let ollama_client = OllamaClient::new("http://localhost:11434".to_string())?;
    dispatcher.register_client(Box::new(OllamaAdapter::new(ollama_client))).await;

    // 准备消息
    let messages = vec![Message {
        role: "user".to_string(),
        content: "Hello, world!".to_string(),
        thinking: None,
        images: None, 
        tool_calls: None,
        tool_name: None,
    }];

    // 发送请求
    let request = DispatchRequest::new(
        Provider::Ollama,
        "llama3.2".to_string(),
        messages,
    ).with_temperature(0.7);

    let response = dispatcher.dispatch(request).await?;
    println!("回复: {}", response.content);

    Ok(())
}
```

### 2. 多供应商配置

```rust
use std::env;

// 创建dispatcher配置
let config = DispatchConfig {
    default_timeout_ms: 30000,
    default_retry_count: 3,
    default_temperature: 0.7,
    enable_fallback: true,
    fallback_providers: vec![Provider::Ollama, Provider::Ali],
};

let dispatcher = LLMDispatcher::new(Some(config));

// 注册多个客户端
let ollama_client = OllamaClient::new("http://localhost:11434".to_string())?;
dispatcher.register_client(Box::new(OllamaAdapter::new(ollama_client))).await;

if let Ok(api_key) = env::var("DASHSCOPE_API_KEY") {
    let ali_client = AliClient::new(api_key)?;
    dispatcher.register_client(Box::new(AliAdapter::new(ali_client))).await;
}
```

### 3. 参数配置

```rust
let request = DispatchRequest::new(
    Provider::Ali,
    "qwen-turbo".to_string(),
    messages,
)
.with_temperature(0.8)           // 创造性控制
.with_max_tokens(1000)          // 最大输出长度
.with_top_p(0.9)                // nucleus sampling
.with_stop(vec!["END".to_string()]); // 停止词

let response = dispatcher.dispatch(request).await?;
```

### 4. 流式响应 (开发中)

```rust
// 注意：流式功能还在开发中
let request = DispatchRequest::new(
    Provider::Ollama,
    "llama3.2".to_string(),
    messages,
).with_stream(true);

let mut stream = dispatcher.dispatch_stream(request).await?;
while let Some(chunk) = stream.recv().await {
    match chunk {
        Ok(content) => print!("{}", content),
        Err(e) => eprintln!("错误: {}", e),
    }
}
```

## 环境设置

### Ollama设置

```bash
# 安装Ollama
curl -fsSL https://ollama.com/install.sh | sh

# 启动服务
ollama serve

# 下载模型
ollama pull llama3.2
ollama pull qwen2.5
```

### 阿里云设置

```bash
# 设置API Key环境变量
export DASHSCOPE_API_KEY="your-dashscope-api-key"
```

## 运行示例

```bash
# 基础示例
cargo run --example simple_dispatcher_demo

# 完整功能示例  
cargo run --example dispatcher_demo

# 设置阿里云API Key后运行
DASHSCOPE_API_KEY=your-key cargo run --example simple_dispatcher_demo
```

## API参考

### DispatchRequest 参数

| 参数 | 类型 | 说明 | 默认值 |
|------|------|------|--------|
| provider | Provider | 供应商选择 | - |
| model | String | 模型名称 | - |
| messages | Vec<Message> | 对话消息 | - |
| stream | Option<bool> | 是否流式 | false |
| temperature | Option<f32> | 随机性(0.0-2.0) | 0.7 |
| max_tokens | Option<u32> | 最大输出token | - |
| top_p | Option<f32> | nucleus sampling | - |
| stop | Option<Vec<String>> | 停止词 | - |
| timeout_ms | Option<u64> | 超时(毫秒) | 30000 |
| retry_count | Option<u32> | 重试次数 | 3 |

### DispatchResponse 字段

| 字段 | 类型 | 说明 |
|------|------|------|
| content | String | AI生成的内容 |
| provider | Provider | 实际使用的供应商 |
| model | String | 实际使用的模型 |
| usage | Option<TokenUsage> | Token使用统计 |
| finish_reason | Option<String> | 完成原因 |
| request_id | Option<String> | 请求ID |
| created_at | String | 创建时间 |
| total_duration | Option<u64> | 总耗时(纳秒) |

## 错误处理

```rust
use project_rust_learn::llm_api::dispatcher::LLMError;

match dispatcher.dispatch(request).await {
    Ok(response) => {
        println!("成功: {}", response.content);
    }
    Err(LLMError::UnsupportedProvider(provider)) => {
        println!("不支持的供应商: {:?}", provider);
    }
    Err(LLMError::ModelNotAvailable(model)) => {
        println!("模型不可用: {}", model);
    }
    Err(LLMError::ApiError(msg)) => {
        println!("API错误: {}", msg);
    }
    Err(e) => {
        println!("其他错误: {}", e);
    }
}
```

## 最佳实践

1. **供应商选择**: 优先使用本地Ollama做开发测试，生产环境使用云服务
2. **Fallback配置**: 启用fallback机制，提高系统可用性
3. **参数调优**: 根据任务类型调整temperature和top_p参数
4. **错误处理**: 充分处理各种错误情况，提供用户友好的提示
5. **监控统计**: 利用usage信息监控API使用情况和成本

## 故障排除

### Ollama连接失败
- 确保Ollama服务正在运行: `ollama serve`
- 检查端口是否正确: 默认11434
- 确认模型已下载: `ollama list`

### 阿里云API失败  
- 检查API Key是否正确设置
- 确认账户余额充足
- 检查模型名称是否正确

### 模型不可用
- Ollama: `ollama pull <model-name>`
- 阿里云: 参考官方文档确认支持的模型列表

需要帮助？提交Issue或查看更多示例代码。
