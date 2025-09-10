//! # Web管理界面模块
//!
//! 提供可视化的LLM模型和Provider管理界面

pub mod server;
pub mod handlers;
pub mod dto;
pub mod middleware;

pub use server::WebServer;
