use serde::{Serialize, Deserialize};
use serde_json::Value;
/// 工具定义结构体
/// 
/// 定义 AI 可以调用的工具/函数，包括功能描述和参数规范
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tool {
    /// 工具类型，通常为 "function"
    #[serde(rename = "type")]
    pub tool_type: String,
    /// 工具的具体功能定义
    pub function: ToolFunction,
}

/// 工具函数定义
/// 
/// 详细描述工具的功能、用途和参数格式
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ToolFunction {
    /// 函数名称（唯一标识符）
    pub name: String,
    /// 函数功能描述，帮助 AI 理解何时使用此工具
    pub description: String,
    /// 函数参数的 JSON Schema 定义
    pub parameters: Value,
}