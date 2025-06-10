use crate::{api::v1::models::ErrorResponse, r#const::api};
use axum::{
    body::Body,
    extract::State,
    http::{header, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use thiserror::Error;
use tracing::{info, warn};

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("Bearer authentication required")]
    MissingAuthHeader,
    #[error("Invalid token format")]
    InvalidTokenFormat,
    #[error("Invalid token")]
    TokenMismatch,
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = (
            StatusCode::UNAUTHORIZED,
            api::error_types::UNAUTHORIZED,
            self.to_string(),
        );

        let body = ErrorResponse::error(status, error_type, &message);
        let mut response = body.into_response();
        response.headers_mut().insert(
            header::WWW_AUTHENTICATE,
            api::auth::BEARER_SCHEME.parse().unwrap(),
        );
        response
    }
}

/// Bearer 令牌认证中间件
pub async fn auth_middleware(
    State(auth_token): State<Option<String>>,
    request: Request<Body>,
    next: Next,
) -> Result<Response, AuthError> {
    // 如果没有设置认证令牌，则跳过认证
    let Some(expected_token) = auth_token else {
        return Ok(next.run(request).await);
    };

    let method = request.method().clone();
    let uri = request.uri().clone();

    // 获取 Authorization 头
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok());

    match auth_header {
        Some(header_value) if header_value.starts_with(api::auth::BEARER_PREFIX) => {
            let token = header_value
                .trim_start_matches(api::auth::BEARER_PREFIX)
                .trim();
            if token == expected_token {
                // 认证成功，继续处理请求
                info!(
                    "Authentication successful for request: \"{}\" \"{}\"",
                    method, uri
                );
                Ok(next.run(request).await)
            } else {
                // 令牌无效
                warn!(
                    "Invalid token provided for request: \"{}\" \"{}\"",
                    method, uri
                );
                Err(AuthError::TokenMismatch)
            }
        }
        Some(_) => {
            warn!(
                "Invalid token format in Authorization header for request: \"{}\" \"{}\"",
                method, uri
            );
            Err(AuthError::InvalidTokenFormat)
        }
        None => {
            warn!(
                "Missing Authorization header for request: \"{}\" \"{}\"",
                method, uri
            );
            Err(AuthError::MissingAuthHeader)
        }
    }
}
