// API v1 模块
pub mod auth;
pub mod handlers;
pub mod models;
pub mod routes;
pub mod schemas;

// 重新导出常用类型
pub use self::models::{ApiResponse, ErrorDetail, ErrorResponse};
pub use self::routes::api_routes;
pub use self::schemas::{openapi_routes, ApiDoc};
