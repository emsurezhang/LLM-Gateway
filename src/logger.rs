use tracing_subscriber::{
    fmt::{self, time::ChronoUtc},
    layer::SubscriberExt,
    util::SubscriberInitExt,
    EnvFilter,
};
use tracing_appender::{non_blocking, rolling};
use anyhow::Result;

/// 日志级别枚举
#[derive(Debug, Clone)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl From<LogLevel> for &'static str {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Error => "error",
            LogLevel::Warn => "warn",
            LogLevel::Info => "info",
            LogLevel::Debug => "debug",
            LogLevel::Trace => "trace",
        }
    }
}

/// 日志配置结构体
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// 日志级别
    pub level: LogLevel,
    /// 日志文件目录
    pub log_dir: String,
    /// 日志文件名前缀
    pub file_prefix: String,
    /// 是否启用控制台输出
    pub console_output: bool,
    /// 是否启用JSON格式
    pub json_format: bool,
    /// 日志文件滚动策略 (daily, hourly)
    pub rotation: String,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Info,
            log_dir: "logs".to_string(),
            file_prefix: "app".to_string(),
            console_output: true,
            json_format: false,
            rotation: "daily".to_string(),
        }
    }
}

/// 初始化日志系统
pub fn init_logger(config: LogConfig) -> Result<()> {
    // 确保日志目录存在
    std::fs::create_dir_all(&config.log_dir)?;

    // 创建文件appender
    let file_appender = match config.rotation.as_str() {
        "hourly" => rolling::hourly(&config.log_dir, &config.file_prefix),
        "daily" => rolling::daily(&config.log_dir, &config.file_prefix),
        _ => rolling::daily(&config.log_dir, &config.file_prefix),
    };

    let (non_blocking_file, _guard) = non_blocking(file_appender);

    // 创建环境过滤器
    let env_filter = EnvFilter::new(format!("{}={}", env!("CARGO_PKG_NAME").replace("-", "_"), <&str>::from(config.level)));

    // 创建格式化器
    let file_layer = fmt::layer()
        .with_writer(non_blocking_file)
        .with_timer(ChronoUtc::rfc_3339())
        .with_ansi(false)
        .with_target(true)
        .with_thread_ids(true)
        .with_file(true)
        .with_line_number(true);

    // 如果启用控制台输出
    if config.console_output {
        let console_layer = fmt::layer()
            .with_timer(ChronoUtc::rfc_3339())
            .with_ansi(true)
            .with_target(false)
            .with_thread_ids(false)
            .with_file(false)
            .with_line_number(false);
        
        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .with(console_layer)
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(file_layer)
            .init();
    }

    // 防止guard被丢弃
    std::mem::forget(_guard);

    Ok(())
}

/// 快速初始化开发环境日志
pub fn init_dev_logger() -> Result<()> {
    let config = LogConfig {
        level: LogLevel::Debug,
        log_dir: "logs".to_string(),
        file_prefix: "dev".to_string(),
        console_output: true,
        json_format: false,
        rotation: "daily".to_string(),
    };
    init_logger(config)
}

/// 快速初始化生产环境日志
pub fn init_prod_logger() -> Result<()> {
    let config = LogConfig {
        level: LogLevel::Info,
        log_dir: "/var/log/project_rust_learn".to_string(),
        file_prefix: "app".to_string(),
        console_output: false,
        json_format: true,
        rotation: "daily".to_string(),
    };
    init_logger(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tracing::{debug, error, info, warn};

    #[tokio::test]
    async fn test_logging() {
        init_dev_logger().unwrap();
        
        error!("This is an error message");
        warn!("This is a warning message");
        info!("This is an info message");
        debug!("This is a debug message");
        
        // 测试结构化日志
        info!(
            user_id = 123,
            action = "login",
            ip = "192.168.1.1",
            "User logged in successfully"
        );
    }
}
