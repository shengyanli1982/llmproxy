use crate::{
    api::v1::{
        auth::auth_middleware,
        handlers::{forward, routing, upstream, upstream_group},
    },
    config::Config,
    r#const::api,
};
use axum::{
    middleware,
    routing::{delete, get, patch, post, put},
    Router,
};
use std::sync::Arc;
use tokio::sync::RwLock;

pub const API_V1_PREFIX: &str = "/api/v1";
const FORWARD_PATH: &str = "/forwards";
const FORWARD_NAME_PATH: &str = "/forwards/{name}";
const UPSTREAM_GROUP_PATH: &str = "/upstream-groups";
const UPSTREAM_GROUP_NAME_PATH: &str = "/upstream-groups/{name}";
const UPSTREAM_PATH: &str = "/upstreams";
const UPSTREAM_NAME_PATH: &str = "/upstreams/{name}";
const ROUTES_PATH: &str = "/forwards/{name}/routes";
const ROUTE_PATH: &str = "/forwards/{name}/routes/{path}";

/// 创建 API v1 路由
pub fn api_routes(config: Arc<RwLock<Config>>) -> Router {
    // 检查是否需要认证
    let auth_token = std::env::var(api::ADMIN_AUTH_TOKEN_ENV).ok();

    let mut api_router = Router::new()
        // 转发规则路由
        .route(FORWARD_PATH, get(forward::list_forwards))
        .route(FORWARD_NAME_PATH, get(forward::get_forward))
        // 路由规则路由
        .route(ROUTES_PATH, get(routing::list_routes))
        .route(ROUTES_PATH, post(routing::create_route))
        .route(ROUTE_PATH, get(routing::get_route))
        .route(ROUTE_PATH, put(routing::update_route))
        .route(ROUTE_PATH, delete(routing::delete_route))
        // 上游组路由
        .route(
            UPSTREAM_GROUP_PATH,
            get(upstream_group::list_upstream_groups),
        )
        .route(
            UPSTREAM_GROUP_NAME_PATH,
            get(upstream_group::get_upstream_group),
        )
        // 上游组修改路由
        .route(
            UPSTREAM_GROUP_NAME_PATH,
            patch(upstream_group::patch_upstream_group),
        )
        // 上游服务路由
        .route(UPSTREAM_PATH, get(upstream::list_upstreams))
        .route(UPSTREAM_NAME_PATH, get(upstream::get_upstream))
        // 上游服务修改路由
        .route(UPSTREAM_PATH, post(upstream::create_upstream))
        .route(UPSTREAM_NAME_PATH, put(upstream::update_upstream))
        .route(UPSTREAM_NAME_PATH, delete(upstream::delete_upstream))
        .with_state(config);

    // 如果设置了认证令牌，添加认证中间件
    if auth_token.is_some() {
        api_router = api_router.layer(middleware::from_fn_with_state(auth_token, auth_middleware));
    }

    Router::new().nest(API_V1_PREFIX, api_router)
}
