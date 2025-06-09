use axum::Router;

/// API文档
pub struct ApiDoc;

pub fn scalar_routes(_path: &str) -> Router {
    // 暂时禁用 OpenAPI 功能，返回一个空的路由
    Router::new()
}
