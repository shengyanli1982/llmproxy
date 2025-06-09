use crate::api::v1::handlers::{forward, upstream, upstream_group};
use crate::config::Config;
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::{Arc, RwLock};

pub const UPSTREAMS_PREFIX: &str = "/upstreams";
pub const UPSTREAM_BY_NAME_PREFIX: &str = "/upstreams/{name}";
pub const UPSTREAM_GROUPS_PREFIX: &str = "/upstream-groups";
pub const UPSTREAM_GROUP_BY_NAME_PREFIX: &str = "/upstream-groups/{name}";
pub const FORWARDS_PREFIX: &str = "/forwards";
pub const FORWARD_BY_NAME_PREFIX: &str = "/forwards/{name}";

/// 创建API v1路由
pub fn api_routes(config: Arc<RwLock<Arc<Config>>>) -> Router {
    Router::new()
        // 上游API
        .route(UPSTREAMS_PREFIX, get(upstream::get_all_upstreams))
        .route(UPSTREAMS_PREFIX, post(upstream::create_upstream))
        .route(UPSTREAM_BY_NAME_PREFIX, get(upstream::get_upstream))
        .route(UPSTREAM_BY_NAME_PREFIX, put(upstream::update_upstream))
        .route(UPSTREAM_BY_NAME_PREFIX, delete(upstream::delete_upstream))
        // 上游组API
        .route(
            UPSTREAM_GROUPS_PREFIX,
            get(upstream_group::get_all_upstream_groups),
        )
        .route(
            UPSTREAM_GROUPS_PREFIX,
            post(upstream_group::create_upstream_group),
        )
        .route(
            UPSTREAM_GROUP_BY_NAME_PREFIX,
            get(upstream_group::get_upstream_group),
        )
        .route(
            UPSTREAM_GROUP_BY_NAME_PREFIX,
            put(upstream_group::update_upstream_group),
        )
        .route(
            UPSTREAM_GROUP_BY_NAME_PREFIX,
            delete(upstream_group::delete_upstream_group),
        )
        // 转发服务API
        .route(FORWARDS_PREFIX, get(forward::get_all_forwards))
        .route(FORWARDS_PREFIX, post(forward::create_forward))
        .route(FORWARD_BY_NAME_PREFIX, get(forward::get_forward))
        .route(FORWARD_BY_NAME_PREFIX, put(forward::update_forward))
        .route(FORWARD_BY_NAME_PREFIX, delete(forward::delete_forward))
        .with_state(config)
}
