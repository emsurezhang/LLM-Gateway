use axum::{
    http::StatusCode,
    response::Json,
};
use serde_json::{json, Value};

/// 健康检查端点
pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "service": "llm-admin-web"
    }))
}

/// 获取系统信息
pub async fn system_info() -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "name": env!("CARGO_PKG_NAME"),
        "rust_version": "unknown",
        "build_time": "unknown" // 可以通过build.rs添加编译时间
    })))
}
