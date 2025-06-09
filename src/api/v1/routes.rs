use crate::api::v1::handlers::{forward, upstream, upstream_group};
use crate::config::Config;
use axum::{
    routing::{delete, get, post, put},
    Router,
};
use std::sync::{Arc, RwLock};

/// 创建API v1路由
pub fn api_routes(config: Arc<RwLock<Arc<Config>>>) -> Router {
    Router::new()
        // 上游API
        .route("/upstreams", get(upstream::get_all_upstreams))
        .route("/upstreams", post(upstream::create_upstream))
        .route("/upstreams/{name}", get(upstream::get_upstream))
        .route("/upstreams/{name}", put(upstream::update_upstream))
        .route("/upstreams/{name}", delete(upstream::delete_upstream))
        // 上游组API
        .route(
            "/upstream-groups",
            get(upstream_group::get_all_upstream_groups),
        )
        .route(
            "/upstream-groups",
            post(upstream_group::create_upstream_group),
        )
        .route(
            "/upstream-groups/{name}",
            get(upstream_group::get_upstream_group),
        )
        .route(
            "/upstream-groups/{name}",
            put(upstream_group::update_upstream_group),
        )
        .route(
            "/upstream-groups/{name}",
            delete(upstream_group::delete_upstream_group),
        )
        // 转发服务API
        .route("/forwards", get(forward::get_all_forwards))
        .route("/forwards", post(forward::create_forward))
        .route("/forwards/{name}", get(forward::get_forward))
        .route("/forwards/{name}", put(forward::update_forward))
        .route("/forwards/{name}", delete(forward::delete_forward))
        .with_state(config)
}
