use crate::{
    api::v1::handlers::{forward, upstream, upstream_group},
    api::v1::models::{ApiResponse, ErrorDetail, ErrorResponse, UpstreamGroupDetail},
    api::v1::routes::API_V1_PREFIX,
    config::{
        AuthConfig, AuthType, BalanceConfig, BalanceStrategy, BreakerConfig, ForwardConfig,
        HeaderOp, HeaderOpType, HttpClientConfig, HttpClientTimeoutConfig, ProxyConfig,
        RateLimitConfig, RetryConfig, TimeoutConfig, UpstreamConfig, UpstreamGroupConfig,
        UpstreamRef,
    },
};
use axum::Router;
use std::env;
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_scalar::Scalar;

/// OpenAPI 文档结构
#[derive(OpenApi)]
#[openapi(
    paths(
        // 转发规则
        forward::list_forwards,
        forward::get_forward,
        // 上游组
        upstream_group::list_upstream_groups,
        upstream_group::get_upstream_group,
        // 上游服务
        upstream::list_upstreams,
        upstream::get_upstream,
    ),
    components(
        schemas(
            // 响应模型
            ApiResponse<Vec<ForwardConfig>>,
            ApiResponse<ForwardConfig>,
            ApiResponse<Vec<UpstreamGroupDetail>>,
            ApiResponse<UpstreamGroupDetail>,
            ApiResponse<Vec<UpstreamConfig>>,
            ApiResponse<UpstreamConfig>,
            ErrorResponse,
            ErrorDetail,
            // 配置模型
            ForwardConfig,
            UpstreamConfig,
            UpstreamGroupConfig,
            UpstreamGroupDetail,
            // 配置相关类型
            AuthConfig,
            AuthType,
            BalanceConfig,
            BalanceStrategy,
            BreakerConfig,
            HeaderOp,
            HeaderOpType,
            HttpClientConfig,
            HttpClientTimeoutConfig,
            ProxyConfig,
            RateLimitConfig,
            RetryConfig,
            TimeoutConfig,
            UpstreamRef,
        ),
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "Forward", description = "转发规则 API | Forward Rule API"),
        (name = "UpstreamGroup", description = "上游组 API | Upstream Group API"),
        (name = "Upstream", description = "上游服务 API | Upstream Service API"),
    ),
    info(
        title = "LLMProxy Admin API",
        version = "v1",
        description = "LLMProxy 管理 API，提供对配置资源的只读访问 | LLMProxy Admin API, providing read-only access to configuration resources",
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        ),
    )
)]
pub struct ApiDoc;

pub fn openapi_routes() -> Router {
    Router::new().merge(Scalar::with_url(
        format!("{}/docs", API_V1_PREFIX),
        ApiDoc::openapi(),
    ))
}

/// 安全方案修改器
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        // 检查是否设置了认证令牌环境变量
        let auth_enabled = env::var("LLMPROXY_ADMIN_AUTH_TOKEN").is_ok();

        if auth_enabled {
            // 如果启用认证，添加安全方案
            if let Some(components) = &mut openapi.components {
                components.add_security_scheme(
                    "bearer_auth",
                    SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new("Authorization"))),
                );
            }
        } else {
            // 如果未启用认证，从所有路径操作中移除安全要求
            if let Some(paths) = &mut openapi.paths {
                for (_, path_item) in paths.iter_mut() {
                    // 检查并清除各种 HTTP 方法的安全要求
                    if let Some(op) = &mut path_item.get {
                        op.security = None;
                    }
                    if let Some(op) = &mut path_item.post {
                        op.security = None;
                    }
                    if let Some(op) = &mut path_item.put {
                        op.security = None;
                    }
                    if let Some(op) = &mut path_item.delete {
                        op.security = None;
                    }
                    if let Some(op) = &mut path_item.options {
                        op.security = None;
                    }
                    if let Some(op) = &mut path_item.head {
                        op.security = None;
                    }
                    if let Some(op) = &mut path_item.patch {
                        op.security = None;
                    }
                    if let Some(op) = &mut path_item.trace {
                        op.security = None;
                    }
                }
            }
        }
    }
}
