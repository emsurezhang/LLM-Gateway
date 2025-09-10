use axum::{
    extract::Query,
    http::StatusCode,
    response::Json,
};
use serde::{Deserialize, Serialize};

use crate::dao::{
    call_log::{
        list_call_logs_paginated, list_error_call_logs, count_call_logs, CallLog, CallLogStats,
        get_call_logs_stats,
    },
    SQLITE_POOL,
};

#[derive(Debug, Deserialize)]
pub struct CallLogQuery {
    page: Option<u32>,
    limit: Option<u32>,
    error_only: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CallLogResponse {
    pub data: Vec<CallLog>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

#[derive(Debug, Serialize)]
pub struct CallLogStatsResponse {
    pub stats: CallLogStats,
}

/// 获取调用日志列表（分页）
pub async fn list_call_logs(
    Query(params): Query<CallLogQuery>,
) -> Result<Json<CallLogResponse>, StatusCode> {
    let pool = SQLITE_POOL.get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref();
        
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(100);
    let error_only = params.error_only.unwrap_or(false);

    // 计算偏移量
    let offset = ((page - 1) * limit) as i64;
    let limit_i64 = limit as i64;

    // 获取总数
    let total = match count_call_logs(pool).await {
        Ok(count) => count,
        Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
    };

    // 获取日志数据
    let call_logs = if error_only {
        match list_error_call_logs(pool).await {
            Ok(logs) => {
                // 对于error_only，我们需要手动分页
                logs.into_iter()
                    .skip(offset as usize)
                    .take(limit as usize)
                    .collect()
            }
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    } else {
        match list_call_logs_paginated(pool, limit_i64, offset).await {
            Ok(logs) => logs,
            Err(_) => return Err(StatusCode::INTERNAL_SERVER_ERROR),
        }
    };

    let total_pages = ((total as f64) / (limit as f64)).ceil() as u32;

    Ok(Json(CallLogResponse {
        data: call_logs,
        total,
        page,
        limit,
        total_pages,
    }))
}

/// 获取调用日志统计信息
pub async fn get_call_log_stats() -> Result<Json<CallLogStatsResponse>, StatusCode> {
    let pool = SQLITE_POOL.get()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .as_ref();
        
    match get_call_logs_stats(pool).await {
        Ok(stats) => Ok(Json(CallLogStatsResponse { stats })),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}
