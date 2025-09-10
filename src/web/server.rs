use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post, put, delete},
    Router,
};
use tower::ServiceBuilder;
use tower_http::{
    services::{ServeDir, ServeFile},
};
use std::net::SocketAddr;
use anyhow::Result;

use crate::dao::init_sqlite_pool;
use crate::web::{
    handlers::{
        health_handler::{health_check, system_info},
        provider_handler::{
            list_providers, get_provider, create_new_provider, 
            update_existing_provider, delete_existing_provider,
            list_provider_summary,
        },
        model_handler::{
            list_models, get_model, create_new_model,
            update_existing_model, delete_existing_model,
            get_model_templates,
        },
        api_key_handler::{
            list_provider_api_keys, create_api_key, update_api_key,
            delete_api_key, toggle_api_key_status,
        },
        call_log_handler::{
            list_call_logs, get_call_log_stats,
        },
    },
    middleware::cors::cors_layer,
};

pub struct WebServer {
    db_url: String,
    init_sql_path: String,
}

impl WebServer {
    pub fn new(db_url: String, init_sql_path: String) -> Self {
        Self { db_url, init_sql_path }
    }

    pub async fn start(&self, addr: SocketAddr) -> Result<()> {
        // 初始化数据库
        init_sqlite_pool(&self.db_url).await;
        
        // 执行数据库初始化脚本
        if let Err(e) = crate::dao::init_db(&self.init_sql_path).await {
            eprintln!("Failed to initialize database: {}", e);
        }

        let app = self.create_app();

        println!("🌐 Web管理界面启动中...");
        println!("📱 管理界面: http://{}", addr);
        println!("🔗 API文档: http://{}/api/health", addr);

        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }

    fn create_app(&self) -> Router {
        // API路由
        let api_routes = Router::new()
            // 健康检查
            .route("/health", get(health_check))
            .route("/system", get(system_info))
            // Provider管理
            .route("/providers", get(list_providers).post(create_new_provider))
            .route("/providers/summary", get(list_provider_summary))
            .route("/providers/:id", get(get_provider).put(update_existing_provider).delete(delete_existing_provider))
            // Model管理
            .route("/models", get(list_models).post(create_new_model))
            .route("/models/:id", get(get_model).put(update_existing_model).delete(delete_existing_model))
            .route("/models/templates/:provider", get(get_model_templates))
            // API Key管理
            .route("/providers/:id/api-keys", get(list_provider_api_keys).post(create_api_key))
            .route("/api-keys/:id", put(update_api_key).delete(delete_api_key))
            .route("/api-keys/:id/toggle/:status", put(toggle_api_key_status))
            // Call Log管理
            .route("/call-logs", get(list_call_logs))
            .route("/call-logs/stats", get(get_call_log_stats));

        // 静态文件服务
        let static_routes = Router::new()
            .route_service("/", ServeFile::new("src/web/static/index.html"))
            .nest_service("/static", ServeDir::new("src/web/static"))
            .fallback(static_fallback);

        // 组合所有路由
        Router::new()
            .nest("/api", api_routes)
            .merge(static_routes)
            .layer(
                ServiceBuilder::new()
                    .layer(cors_layer())
            )
    }
}

// 静态文件fallback - 对于SPA应用，都返回index.html
async fn static_fallback() -> impl IntoResponse {
    match tokio::fs::read_to_string("src/web/static/index.html").await {
        Ok(content) => Html(content).into_response(),
        Err(_) => (StatusCode::NOT_FOUND, "Page not found").into_response(),
    }
}
