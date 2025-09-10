//! # 通用的 LLM API 消息结构体
//!
//! 定义所有 LLM 客户端共用的消息结构体和相关类型

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// 工具调用结构体
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolCall {
    /// 工具调用 ID（OpenAI 格式需要）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub id: Option<String>,
    /// 工具类型，通常为 "function"
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub tool_type: Option<String>,
    /// 要调用的函数信息
    pub function: Function,
}

/// 通用聊天消息结构体
/// 
/// 兼容多种 LLM API 格式，包括 OpenAI、Ollama、阿里云等
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Message {
    /// 消息角色：system、user、assistant、tool
    pub role: String,
    /// 消息内容文本
    pub content: String,
    /// 可选的思维过程内容（Ollama Thinking 模式）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking: Option<String>,
    /// 可选的图像列表，支持多模态对话（Ollama/GPT-4V）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub images: Option<Vec<String>>,
    /// 可选的工具调用列表（OpenAI/Ollama Tool Calling）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// 工具名称（当角色为 tool 时使用）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
}

/// 函数调用信息
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Function {
    /// 函数名称
    pub name: String,
    /// 函数参数（JSON 格式）
    pub arguments: HashMap<String, Value>,
}


impl Message {
    /// 创建系统消息
    pub fn system(content: String) -> Self {
        Self {
            role: "system".to_string(),
            content,
            thinking: None,
            images: None,
            tool_calls: None,
            tool_name: None,
        }
    }

    /// 创建用户消息
    pub fn user(content: String) -> Self {
        Self {
            role: "user".to_string(),
            content,
            thinking: None,
            images: None,
            tool_calls: None,
            tool_name: None,
        }
    }

    /// 创建助手消息
    pub fn assistant(content: String) -> Self {
        Self {
            role: "assistant".to_string(),
            content,
            thinking: None,
            images: None,
            tool_calls: None,
            tool_name: None,
        }
    }

    /// 创建工具消息
    pub fn tool(content: String, tool_name: String) -> Self {
        Self {
            role: "tool".to_string(),
            content,
            thinking: None,
            images: None,
            tool_calls: None,
            tool_name: Some(tool_name),
        }
    }

    /// 为消息添加图像（多模态支持）
    pub fn with_images(mut self, images: Vec<String>) -> Self {
        self.images = Some(images);
        self
    }

    /// 为消息添加思维过程（Ollama Thinking 模式）
    pub fn with_thinking(mut self, thinking: String) -> Self {
        self.thinking = Some(thinking);
        self
    }

    /// 为消息添加工具调用
    pub fn with_tool_calls(mut self, tool_calls: Vec<ToolCall>) -> Self {
        self.tool_calls = Some(tool_calls);
        self
    }
}