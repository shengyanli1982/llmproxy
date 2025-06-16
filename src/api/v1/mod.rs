// API v1 模块
pub mod auth;
pub mod handlers;
pub mod models;
pub mod routes;
pub mod schemas;

// 公共类型重新导出
pub use models::{ErrorDetail, ErrorResponse, SuccessResponse};
pub use routes::api_routes;
pub use schemas::{openapi_routes, ApiDoc};
