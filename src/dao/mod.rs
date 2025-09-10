use sqlx::SqlitePool;
use once_cell::sync::OnceCell;
use std::sync::Arc;

pub static SQLITE_POOL: OnceCell<Arc<SqlitePool>> = OnceCell::new();

/// 异步初始化全局 SqlitePool
pub async fn init_sqlite_pool(db_url: &str) {
    let pool = SqlitePool::connect(db_url).await.expect("Failed to create pool");
    SQLITE_POOL.set(Arc::new(pool)).ok();
}

pub mod cache;

pub mod model;
pub mod provider;
pub mod provider_key_pool;
pub mod system_config;
pub mod call_log;

use tokio::fs;

/// 通过 SQLITE_POOL 获取数据库连接，并异步执行 SQL 脚本
pub async fn init_db(sql_path: &str) -> anyhow::Result<()> {
    let sql = fs::read_to_string(sql_path).await?;
    let pool = SQLITE_POOL.get().expect("SQLITE_POOL not initialized").clone();
    // 支持多条 SQL 语句分号分割执行
    for statement in sql.split(';') {
        let stmt = statement.trim();
        if !stmt.is_empty() {
            sqlx::query(stmt).execute(&*pool).await?;
        }
    }
    Ok(())
}