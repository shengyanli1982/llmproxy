use crate::{
    api::v1::{
        auth::auth_middleware,
        handlers::{forward, routing, upstream, upstream_group},
    },
    config::Config,
    r#const::api,
    server::ForwardState,
};
use axum::{
    middleware,
    routing::{delete, get, patch, post, put},
    Router,
};
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock;

/// 应用状态结构体，用于替代之前的元组状态
#[derive(Clone)]
pub struct AppState {
    /// 配置
    pub config: Arc<RwLock<Config>>,
    /// 转发服务状态
    pub forward_states: Arc<HashMap<String, Arc<ForwardState>>>,
}

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
pub fn api_routes(
    config: Arc<RwLock<Config>>,
    forward_states: Arc<HashMap<String, Arc<ForwardState>>>,
) -> Router {
    // 创建应用状态
    let app_state = AppState {
        config,
        forward_states,
    };

    // 检查是否需要认证
    let auth_token = std::env::var(api::ADMIN_AUTH_TOKEN_ENV).ok();

    // 创建API路由器
    let mut api_router = Router::new()
        .route(FORWARD_PATH, get(forward::list_forwards))
        .route(FORWARD_NAME_PATH, get(forward::get_forward))
        .route(ROUTES_PATH, get(routing::list_routes))
        .route(ROUTES_PATH, post(routing::create_route))
        .route(ROUTE_PATH, get(routing::get_route))
        .route(ROUTE_PATH, put(routing::update_route))
        .route(ROUTE_PATH, delete(routing::delete_route))
        .route(
            UPSTREAM_GROUP_PATH,
            get(upstream_group::list_upstream_groups),
        )
        .route(
            UPSTREAM_GROUP_NAME_PATH,
            get(upstream_group::get_upstream_group),
        )
        .route(
            UPSTREAM_GROUP_NAME_PATH,
            patch(upstream_group::patch_upstream_group),
        )
        .route(UPSTREAM_PATH, get(upstream::list_upstreams))
        .route(UPSTREAM_NAME_PATH, get(upstream::get_upstream))
        .route(UPSTREAM_PATH, post(upstream::create_upstream))
        .route(UPSTREAM_NAME_PATH, put(upstream::update_upstream))
        .route(UPSTREAM_NAME_PATH, delete(upstream::delete_upstream))
        .with_state(app_state);

    // 如果设置了认证令牌，添加认证中间件
    if auth_token.is_some() {
        api_router = api_router.layer(middleware::from_fn_with_state(auth_token, auth_middleware));
    }

    // 返回根路由器，其中包含嵌套的API路由
    Router::new().nest(API_V1_PREFIX, api_router)
}
