use crate::api::v1::handlers::{forward, upstream, upstream_group};
use crate::api::v1::models::{ApiError, ApiResponse, ErrorDetail, ErrorInfo, ErrorType};
use crate::config::{
    AuthConfig, AuthType, BalanceConfig, BalanceStrategy, BreakerConfig, ForwardConfig,
    HeaderOpType, HeaderOperation, HttpClientConfig, ProxyConfig, RateLimitConfig, RetryConfig,
    TimeoutConfig, UpstreamConfig, UpstreamGroupConfig, UpstreamRef,
};
use axum::Router;
use utoipa::OpenApi;
use utoipa_scalar::{Scalar, Servable};

/// API 文档
#[derive(OpenApi)]
#[openapi(
    paths(
        upstream::get_all_upstreams,
        upstream::get_upstream,
        upstream::create_upstream,
        upstream::update_upstream,
        upstream::delete_upstream,
        upstream_group::get_all_upstream_groups,
        upstream_group::get_upstream_group,
        upstream_group::create_upstream_group,
        upstream_group::update_upstream_group,
        upstream_group::delete_upstream_group,
        forward::get_all_forwards,
        forward::get_forward,
        forward::create_forward,
        forward::update_forward,
        forward::delete_forward,
    ),
    components(
        schemas(
            ApiResponse<Vec<UpstreamConfig>>,
            ApiResponse<UpstreamConfig>,
            ApiResponse<Vec<UpstreamGroupConfig>>,
            ApiResponse<UpstreamGroupConfig>,
            ApiResponse<Vec<ForwardConfig>>,
            ApiResponse<ForwardConfig>,
            ApiError,
            ErrorInfo,
            ErrorType,
            ErrorDetail,
            UpstreamConfig,
            UpstreamGroupConfig,
            UpstreamRef,
            ForwardConfig,
            AuthConfig,
            AuthType,
            HeaderOperation,
            HeaderOpType,
            BalanceConfig,
            BalanceStrategy,
            BreakerConfig,
            HttpClientConfig,
            TimeoutConfig,
            RateLimitConfig,
            RetryConfig,
            ProxyConfig,
        )
    ),
    tags(
        (name = "Upstreams", description = "Upstream Management API | 上游服务管理API"),
        (name = "UpstreamGroups", description = "Upstream Group Management API | 上游服务组管理API"),
        (name = "Forwards", description = "Forward Service Management API | 转发服务管理API")
    ),
    info(
        title = "LLMProxy API",
        version = "1.0.0",
        description = "LLMProxy is an intelligent load balancer that supports unified management and scheduling of multiple Large Language Model (LLM) services.\
        This API provides management functions for upstream services, upstream groups, and forward services.\
        <br><br>\
        LLMProxy是一个智能负载均衡器，支持对多个大型语言模型(LLM)服务进行统一管理和调度。\
        该API提供上游服务、上游服务组和转发服务的管理功能。",
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        ),
        contact(
            name = "LLMProxy Team",
            url = "https://github.com/shengyanli1982/llmproxy"
        )
    ),
    servers(
        (url = "/api/v1/admin", description = "Local server | 本地服务器")
    )
)]
pub struct ApiDoc;

/// 创建 OpenAPI 文档路由
pub fn scalar_routes(path: &'static str) -> Router {
    // 使用 Servable trait 的 with_url 方法创建 Scalar 实例
    let scalar = Scalar::with_url(path, ApiDoc::openapi());

    // 返回路由
    Router::new().merge(scalar)
}
