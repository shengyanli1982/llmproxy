use crate::{
    api::v1::{
        auth::auth_middleware,
        handlers::{forward, upstream, upstream_group},
    },
    config::Config,
};
use axum::{middleware, routing::get, Router};
use std::sync::Arc;

const API_V1_PREFIX: &str = "/api/v1";
const FORWARD_PATH: &str = "/forwards";
const FORWARD_NAME_PATH: &str = "/forwards/{name}";
const UPSTREAM_GROUP_PATH: &str = "/upstream-groups";
const UPSTREAM_GROUP_NAME_PATH: &str = "/upstream-groups/{name}";
const UPSTREAM_PATH: &str = "/upstreams";
const UPSTREAM_NAME_PATH: &str = "/upstreams/{name}";

/// 创建 API v1 路由
pub fn api_routes(config: Arc<Config>) -> Router {
    // 检查是否需要认证
    let auth_token = std::env::var("LLMPROXY_ADMIN_AUTH_TOKEN").ok();

    let mut api_router = Router::new()
        // 转发规则路由
        .route(FORWARD_PATH, get(forward::list_forwards))
        .route(FORWARD_NAME_PATH, get(forward::get_forward))
        // 上游组路由
        .route(
            UPSTREAM_GROUP_PATH,
            get(upstream_group::list_upstream_groups),
        )
        .route(
            UPSTREAM_GROUP_NAME_PATH,
            get(upstream_group::get_upstream_group),
        )
        // 上游服务路由
        .route(UPSTREAM_PATH, get(upstream::list_upstreams))
        .route(UPSTREAM_NAME_PATH, get(upstream::get_upstream))
        .with_state(config);

    // 如果设置了认证令牌，添加认证中间件
    if auth_token.is_some() {
        api_router = api_router.layer(middleware::from_fn_with_state(auth_token, auth_middleware));
    }

    Router::new().nest(API_V1_PREFIX, api_router)
}
