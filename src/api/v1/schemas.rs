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
    r#const::api,
};
use axum::{http::header, Router};
use std::env;
use tracing::debug;
use utoipa::{
    openapi::security::{ApiKey, ApiKeyValue, SecurityScheme},
    Modify, OpenApi,
};
use utoipa_scalar::{Scalar, Servable};

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
        description = "LLMProxy 是一款专为大型语言模型（LLM）设计的企业级智能代理与负载均衡器。它统一管理和编排各类 LLM 服务（如公有云 API、私有化部署的 vLLM/Ollama 等），实现高效、稳定、可扩展的 LLM 应用访问。此管理 API 提供了对 LLMProxy 核心配置资源的只读访问能力，便于监控、审计和集成自动化运维体系。
        <br>
        LLMProxy is an enterprise-grade intelligent proxy and load balancer designed for Large Language Models (LLMs). It unifies the management and orchestration of various LLM services (e.g., public cloud APIs, privately deployed vLLM/Ollama) to enable efficient, stable, and scalable LLM application access. This Admin API provides read-only access to LLMProxy's core configuration resources, facilitating monitoring, auditing, and integration with automated operational systems.",
        license(
            name = "MIT",
            url = "https://opensource.org/licenses/MIT"
        ),
    )
)]
pub struct ApiDoc;

const OPENAPI_PATH: &str = "/docs";

pub fn openapi_routes() -> Router {
    let openapi_path = format!("{}{}", API_V1_PREFIX, OPENAPI_PATH);
    debug!(
        "OpenAPI UI is enabled in debug mode, visit \"{}\"",
        openapi_path
    );
    Router::new().merge(Scalar::with_url(openapi_path, ApiDoc::openapi()))
}

/// 安全方案修改器
struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        // 检查是否设置了认证令牌环境变量
        let auth_enabled = env::var(api::ADMIN_AUTH_TOKEN_ENV).is_ok();

        if auth_enabled {
            // 如果启用认证，添加安全方案
            if let Some(components) = &mut openapi.components {
                components.add_security_scheme(
                    api::auth::BEARER_SECURITY_SCHEME,
                    SecurityScheme::ApiKey(ApiKey::Header(ApiKeyValue::new(
                        header::AUTHORIZATION.as_str(),
                    ))),
                );
            }
        } else {
            // 如果未启用认证，从所有路径操作中移除安全要求
            for (_, path_item) in openapi.paths.paths.iter_mut() {
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
