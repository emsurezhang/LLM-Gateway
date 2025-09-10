//! # 通用 Chat API 抽象结构
//!
//! 定义所有 LLM 客户端共用的 ChatRequest 和 ChatResponse 抽象类/trait
//! 以及相关的通用类型和方法

use serde_json::Value;
use std::collections::HashMap;
use async_trait::async_trait;
use crate::llm_api::utils::msg_structure::Message;

/// 通用 ChatRequest Trait
/// 
/// 定义所有 LLM 客户端的 Chat 请求必须实现的通用接口
/// 支持模型选择、消息列表、工具调用等核心功能
pub trait ChatRequestTrait {
    /// 获取要使用的模型名称
    fn get_model(&self) -> &str;
    
    /// 获取对话消息列表的副本
    fn get_messages(&self) -> Vec<Message>;
    
    /// 设置对话消息列表
    fn set_messages(&mut self, messages: Vec<Message>);
    
    /// 添加一条消息到对话中
    fn add_message(&mut self, message: Message);
    
    /// 获取消息数量
    fn message_count(&self) -> usize;
    
    /// 获取是否使用流式输出
    fn is_stream(&self) -> Option<bool> {
        None
    }
    
    /// 设置是否使用流式输出
    fn set_stream(&mut self, stream: bool);
    
    /// 获取模型参数选项的副本
    fn get_options(&self) -> Option<HashMap<String, Value>> {
        None
    }
    
    /// 设置模型参数选项
    fn set_options(&mut self, options: HashMap<String, Value>);
    
    /// 获取输出格式约束
    fn get_format(&self) -> Option<String> {
        None
    }
    
    /// 设置输出格式约束
    fn set_format(&mut self, format: String);
    
    /// 验证请求参数是否有效
    fn validate(&self) -> Result<(), String> {
        if self.get_model().is_empty() {
            return Err("Model name cannot be empty".to_string());
        }
        if self.message_count() == 0 {
            return Err("Messages cannot be empty".to_string());
        }
        Ok(())
    }
}

/// 通用 ChatResponse Trait
/// 
/// 定义所有 LLM 客户端的 Chat 响应必须实现的通用接口
/// 包含模型信息、生成内容、性能指标等
pub trait ChatResponseTrait {
    /// 获取实际使用的模型名称
    fn get_model(&self) -> &str;
    
    /// 获取响应创建时间戳
    fn get_created_at(&self) -> &str;
    
    /// 获取 AI 生成的消息内容的副本
    fn get_message(&self) -> Option<Message>;
    
    /// 获取生成的文本内容（便捷方法）
    fn get_content(&self) -> Option<String> {
        self.get_message().map(|msg| msg.content)
    }
    
    /// 是否为完整响应（流式模式下使用）
    fn is_done(&self) -> bool;
    
    /// 获取总处理时间（纳秒）
    fn get_total_duration(&self) -> Option<u64> {
        None
    }
    
    /// 获取生成的 token 数量
    fn get_eval_count(&self) -> Option<u32> {
        None
    }
    
    /// 获取提示词 token 数量
    fn get_prompt_eval_count(&self) -> Option<u32> {
        None
    }
    
    /// 计算生成速度（tokens/秒）
    fn get_generation_speed(&self) -> Option<f64> {
        match (self.get_eval_count(), self.get_total_duration()) {
            (Some(tokens), Some(duration_ns)) if duration_ns > 0 => {
                let duration_seconds = duration_ns as f64 / 1_000_000_000.0;
                Some(tokens as f64 / duration_seconds)
            },
            _ => None,
        }
    }
    
    /// 获取性能摘要信息
    fn get_performance_summary(&self) -> PerformanceSummary {
        PerformanceSummary {
            total_duration: self.get_total_duration(),
            eval_count: self.get_eval_count(),
            prompt_eval_count: self.get_prompt_eval_count(),
            generation_speed: self.get_generation_speed(),
        }
    }
}

/// 性能摘要结构体
/// 
/// 统一不同 LLM API 的性能指标格式
#[derive(Debug, Clone)]
pub struct PerformanceSummary {
    /// 总处理时间（纳秒）
    pub total_duration: Option<u64>,
    /// 生成的 token 数量
    pub eval_count: Option<u32>,
    /// 提示词 token 数量
    pub prompt_eval_count: Option<u32>,
    /// 生成速度（tokens/秒）
    pub generation_speed: Option<f64>,
}

impl PerformanceSummary {
    /// 创建空的性能摘要
    pub fn empty() -> Self {
        Self {
            total_duration: None,
            eval_count: None,
            prompt_eval_count: None,
            generation_speed: None,
        }
    }
    
    /// 是否有任何性能数据
    pub fn has_data(&self) -> bool {
        self.total_duration.is_some() || 
        self.eval_count.is_some() || 
        self.prompt_eval_count.is_some()
    }
    
    /// 格式化为可读字符串
    pub fn format(&self) -> String {
        let mut parts = Vec::new();
        
        if let Some(tokens) = self.eval_count {
            parts.push(format!("Generated: {} tokens", tokens));
        }
        
        if let Some(prompt_tokens) = self.prompt_eval_count {
            parts.push(format!("Prompt: {} tokens", prompt_tokens));
        }
        
        if let Some(speed) = self.generation_speed {
            parts.push(format!("Speed: {:.1} tokens/s", speed));
        }
        
        if let Some(duration) = self.total_duration {
            let duration_ms = duration as f64 / 1_000_000.0;
            parts.push(format!("Duration: {:.1}ms", duration_ms));
        }
        
        if parts.is_empty() {
            "No performance data".to_string()
        } else {
            parts.join(", ")
        }
    }
}

/// 通用 Chat 客户端 Trait
/// 
/// 定义所有 LLM 客户端必须实现的基本 Chat 方法
#[async_trait]
pub trait ChatClientTrait {
    type Request: ChatRequestTrait;
    type Response: ChatResponseTrait;
    type Error: std::error::Error + Send + Sync + 'static;
    
    /// 发送 Chat 请求（非流式）
    async fn chat(&self, request: Self::Request) -> Result<Self::Response, Self::Error>;
    
    /// 发送 Chat 请求（流式）
    async fn chat_stream(&self, request: Self::Request) -> Result<Box<dyn futures_util::Stream<Item = Result<Self::Response, Self::Error>> + Unpin + Send>, Self::Error>;
    
    /// 获取客户端名称/类型
    fn get_client_type(&self) -> &'static str;
    
    /// 检查客户端是否健康
    async fn health_check(&self) -> Result<bool, Self::Error>;
}

/// 构建器模式的 ChatRequest 基础实现
/// 
/// 提供通用的构建器方法，减少重复代码
pub struct ChatRequestBuilder {
    model: String,
    messages: Vec<Message>,
    stream: Option<bool>,
    options: Option<HashMap<String, Value>>,
    format: Option<String>,
}

impl ChatRequestBuilder {
    /// 创建新的构建器
    pub fn new(model: String) -> Self {
        Self {
            model,
            messages: Vec::new(),
            stream: None,
            options: None,
            format: None,
        }
    }
    
    /// 设置消息列表
    pub fn messages(mut self, messages: Vec<Message>) -> Self {
        self.messages = messages;
        self
    }
    
    /// 添加单条消息
    pub fn add_message(mut self, message: Message) -> Self {
        self.messages.push(message);
        self
    }
    
    /// 添加系统消息
    pub fn system(mut self, content: String) -> Self {
        self.messages.push(Message::system(content));
        self
    }
    
    /// 添加用户消息
    pub fn user(mut self, content: String) -> Self {
        self.messages.push(Message::user(content));
        self
    }
    
    /// 添加助手消息
    pub fn assistant(mut self, content: String) -> Self {
        self.messages.push(Message::assistant(content));
        self
    }
    
    /// 设置流式输出
    pub fn stream(mut self, stream: bool) -> Self {
        self.stream = Some(stream);
        self
    }
    
    /// 设置模型参数
    pub fn options(mut self, options: HashMap<String, Value>) -> Self {
        self.options = Some(options);
        self
    }
    
    /// 设置输出格式
    pub fn format(mut self, format: String) -> Self {
        self.format = Some(format);
        self
    }
    
    /// 获取构建的字段（子类可以使用）
    pub fn build_fields(self) -> (String, Vec<Message>, Option<bool>, Option<HashMap<String, Value>>, Option<String>) {
        (self.model, self.messages, self.stream, self.options, self.format)
    }
}
