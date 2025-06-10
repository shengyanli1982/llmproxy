use crate::api::v1::models::ApiResponse;
use axum::{
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};

/// Bearer 令牌认证中间件
pub async fn auth_middleware<B>(
    State(auth_token): State<Option<String>>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, ApiResponse<()>> {
    // 如果没有设置认证令牌，则跳过认证
    let Some(expected_token) = auth_token else {
        return Ok(next.run(request).await);
    };

    // 获取 Authorization 头
    let auth_header = request
        .headers()
        .get("Authorization")
        .and_then(|header| header.to_str().ok());

    // 验证 Bearer 令牌
    match auth_header {
        Some(auth) if auth.starts_with("Bearer ") => {
            let token = auth.trim_start_matches("Bearer ").trim();
            if token == expected_token {
                // 认证成功，继续处理请求
                Ok(next.run(request).await)
            } else {
                // 令牌无效
                Err(ApiResponse::error(
                    StatusCode::UNAUTHORIZED,
                    "Unauthorized",
                    "无效的认证令牌",
                ))
            }
        }
        _ => {
            // 缺少 Authorization 头或格式不正确
            Err(ApiResponse::error(
                StatusCode::UNAUTHORIZED,
                "Unauthorized",
                "需要 Bearer 认证",
            ))
        }
    }
}
