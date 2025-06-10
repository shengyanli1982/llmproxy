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
use tracing::debug;
use utoipa::OpenApi;
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
    tags(
        (name = "Forwards", description = "转发规则 APIs | Forward Rule APIs"),
        (name = "UpstreamGroups", description = "上游组 APIs | Upstream Group APIs"),
        (name = "Upstreams", description = "上游服务 APIs | Upstream Service APIs"),
    ),
    info(
        title = "LLMProxy APIs",
        version = "v1",
        description = "Github: <a href='https://github.com/shengyanli1982/llmproxy'>https://github.com/shengyanli1982/llmproxy</a>
        <br>
        作者(Author): 李盛雁 | ShengYan Li
        <br>
        邮箱(Email): shengyanlee36@gmail.com
        <br><br>
        LLMProxy 是一款专为大型语言模型（LLM）设计的企业级智能代理与负载均衡器。它统一管理和编排各类 LLM 服务（如公有云 API、私有化部署的 vLLM/Ollama 等），实现高效、稳定、可扩展的 LLM 应用访问。此管理 API 提供了对 LLMProxy 核心配置资源的只读访问能力，便于监控、审计和集成自动化运维体系。
        <br><br>
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
