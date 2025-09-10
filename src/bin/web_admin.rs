//! # LLM Web管理界面启动程序
//!
//! 启动可视化的LLM模型和Provider管理界面

use std::net::SocketAddr;
use project_rust_learn::{
    web::WebServer,
    logger,
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 初始化日志
    let _logger = logger::init_logger(logger::LogConfig::default())?;

    println!("🚀 启动 LLM Web管理界面...");

    // 配置参数
    let db_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite://data/app.db".to_string());
    let init_sql_path = std::env::var("INIT_SQL_PATH")
        .unwrap_or_else(|_| "data/init.sql".to_string());
    let bind_addr = std::env::var("BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:8080".to_string());

    println!("📊 数据库: {}", db_url);
    println!("📄 初始化脚本: {}", init_sql_path);
    println!("🌐 绑定地址: {}", bind_addr);

    // 解析地址
    let addr: SocketAddr = bind_addr.parse()
        .map_err(|e| format!("Invalid bind address: {}", e))?;

    // 创建并启动Web服务器
    let web_server = WebServer::new(db_url, init_sql_path);
    web_server.start(addr).await?;

    Ok(())
}
