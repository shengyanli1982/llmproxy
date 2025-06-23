// tests/balancer/common.rs

// This module will contain shared helper functions for the balancer tests.

use llmproxy::{balancer::ManagedUpstream, config::UpstreamRef};
use wiremock::{MockServer, ResponseTemplate};

/// Creates a list of managed upstreams for testing.
pub fn create_test_managed_upstreams() -> Vec<ManagedUpstream> {
    vec![
        ManagedUpstream {
            upstream_ref: UpstreamRef {
                name: "upstream1".to_string(),
                weight: 1,
            },
            breaker: None,
        },
        ManagedUpstream {
            upstream_ref: UpstreamRef {
                name: "upstream2".to_string(),
                weight: 2,
            },
            breaker: None,
        },
        ManagedUpstream {
            upstream_ref: UpstreamRef {
                name: "upstream3".to_string(),
                weight: 3,
            },
            breaker: None,
        },
    ]
}

/// Sets up a mock server with a given response body and delay.
pub async fn setup_mock_server(body: &str, delay_ms: u64) -> MockServer {
    let server = MockServer::start().await;
    let response = ResponseTemplate::new(200)
        .set_body_string(body)
        .set_delay(std::time::Duration::from_millis(delay_ms));
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path("/test"))
        .respond_with(response)
        .mount(&server)
        .await;
    server
}
