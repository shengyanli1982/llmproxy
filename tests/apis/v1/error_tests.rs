use axum::{body::to_bytes, http::StatusCode, response::IntoResponse};
use llmproxy::apis::v1::error::{codes, ApiError, ApiResponse};
use tokio::test;

/// 测试ApiResponse正确格式化各种响应
#[test]
async fn test_api_response_formatting() {
    // 测试成功响应
    let ok_response: ApiResponse<String> =
        ApiResponse::ok("Success message", Some("data".to_string()));
    assert_eq!(ok_response.code, codes::OK);
    assert_eq!(ok_response.message, "Success message");
    assert_eq!(ok_response.data, Some("data".to_string()));

    // 测试创建成功响应
    let created_response: ApiResponse<String> =
        ApiResponse::created("Created message", Some("data".to_string()));
    assert_eq!(created_response.code, codes::CREATED);
    assert_eq!(created_response.message, "Created message");
    assert_eq!(created_response.data, Some("data".to_string()));

    // 测试接受响应
    let accepted_response: ApiResponse<String> =
        ApiResponse::accepted("Accepted message", Some("data".to_string()));
    assert_eq!(accepted_response.code, codes::ACCEPTED);
    assert_eq!(accepted_response.message, "Accepted message");
    assert_eq!(accepted_response.data, Some("data".to_string()));

    // 测试无数据响应
    let no_data_response: ApiResponse<String> = ApiResponse::ok("No data message", None);
    assert_eq!(no_data_response.code, codes::OK);
    assert_eq!(no_data_response.message, "No data message");
    assert_eq!(no_data_response.data, None);
}

/// 测试ApiError转换为HTTP响应
#[test]
async fn test_api_error_into_response() {
    // 测试NotFound错误
    let not_found_error = ApiError::NotFound("Resource not found".to_string());
    let response = not_found_error.into_response();
    assert_eq!(response.status(), StatusCode::NOT_FOUND);

    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["code"], codes::NOT_FOUND);
    assert!(body["message"].as_str().unwrap().contains("not found"));

    // 测试AlreadyExists错误
    let already_exists_error = ApiError::AlreadyExists("Resource already exists".to_string());
    let response = already_exists_error.into_response();
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["code"], codes::ALREADY_EXISTS);
    assert!(body["message"].as_str().unwrap().contains("already exists"));

    // 测试StillReferenced错误
    let still_referenced_error = ApiError::StillReferenced {
        resource_type: "Upstream".to_string(),
        name: "test_upstream".to_string(),
        referenced_by: vec!["group1".to_string(), "group2".to_string()],
    };
    let response = still_referenced_error.into_response();
    assert_eq!(response.status(), StatusCode::CONFLICT);

    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["code"], codes::STILL_REFERENCED);
    assert!(body["message"]
        .as_str()
        .unwrap()
        .contains("still referenced"));
    assert!(body["data"]["referenced_by"].is_array());
    assert_eq!(body["data"]["referenced_by"][0], "group1");
    assert_eq!(body["data"]["referenced_by"][1], "group2");

    // 测试ReferenceNotFound错误
    let reference_not_found_error = ApiError::ReferenceNotFound {
        resource_type: "UpstreamGroup".to_string(),
        name: "nonexistent_group".to_string(),
    };
    let response = reference_not_found_error.into_response();
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["code"], codes::UNPROCESSABLE);
    assert!(body["message"].as_str().unwrap().contains("not found"));

    // 测试ValidationError错误
    let validation_error = ApiError::ValidationError("Invalid input".to_string());
    let response = validation_error.into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["code"], codes::BAD_REQUEST);
    assert!(body["message"].as_str().unwrap().contains("Invalid input"));

    // 测试InternalError错误
    let internal_error = ApiError::InternalError("Server error".to_string());
    let response = internal_error.into_response();
    assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);

    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["code"], codes::INTERNAL_ERROR);
    assert!(body["message"].as_str().unwrap().contains("Server error"));

    // 测试JsonParseError错误
    let json_parse_error = ApiError::JsonParseError("Invalid JSON".to_string());
    let response = json_parse_error.into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["code"], codes::INVALID_JSON);
    assert!(body["message"].as_str().unwrap().contains("Invalid JSON"));

    // 测试MissingParameter错误
    let missing_parameter_error = ApiError::MissingParameter("Required field".to_string());
    let response = missing_parameter_error.into_response();
    assert_eq!(response.status(), StatusCode::BAD_REQUEST);

    let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(body["code"], codes::MISSING_FIELD);
    assert!(body["message"].as_str().unwrap().contains("Required field"));
}

/// 测试错误代码常量是否正确定义
#[test]
async fn test_error_codes() {
    // 成功代码
    assert_eq!(codes::OK, 20000);
    assert_eq!(codes::CREATED, 20100);
    assert_eq!(codes::ACCEPTED, 20200);

    // 客户端错误代码
    assert_eq!(codes::BAD_REQUEST, 40000);
    assert_eq!(codes::INVALID_JSON, 40001);
    assert_eq!(codes::MISSING_FIELD, 40002);
    assert_eq!(codes::UNAUTHORIZED, 40100);
    assert_eq!(codes::FORBIDDEN, 40300);
    assert_eq!(codes::NOT_FOUND, 40400);
    assert_eq!(codes::CONFLICT, 40900);
    assert_eq!(codes::ALREADY_EXISTS, 40901);
    assert_eq!(codes::STILL_REFERENCED, 40902);
    assert_eq!(codes::UNPROCESSABLE, 42200);

    // 服务端错误代码
    assert_eq!(codes::INTERNAL_ERROR, 50000);
    assert_eq!(codes::SERVICE_UNAVAILABLE, 50300);
}
