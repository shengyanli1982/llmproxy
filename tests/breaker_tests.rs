use circuitbreaker_rs::State;
use llmproxy::{
    balancer::{create_load_balancer, ManagedUpstream},
    breaker::{create_upstream_circuit_breaker, UpstreamCircuitBreaker, UpstreamError},
    config::{BalanceStrategy, BreakerConfig, UpstreamRef},
    error::AppError,
};
use std::time::Duration;
use tokio::time::sleep;

use wiremock::{
    matchers::{method, path},
    Mock, MockServer, ResponseTemplate,
};

// 辅助函数：创建测试用的熔断器配置
fn create_test_breaker_config(threshold: f64, cooldown: u64) -> BreakerConfig {
    BreakerConfig {
        threshold,
        cooldown,
    }
}

#[tokio::test]
async fn test_breaker_basic_functionality() {
    // 创建熔断器
    let name = "test_upstream".to_string();
    let group = "test_group".to_string();
    let config = create_test_breaker_config(0.5, 1); // 50% 失败率阈值，1秒冷却时间

    let breaker = create_upstream_circuit_breaker(name, group, &config);

    // 初始状态应为关闭
    assert_eq!(breaker.current_state(), State::Closed);
    assert!(breaker.is_call_permitted());

    // 测试成功调用
    let result = breaker
        .call_async(|| async { Ok::<_, UpstreamError>("success".to_string()) })
        .await;
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "success");

    // 测试失败调用
    let result = breaker
        .call_async(|| async { Err::<String, _>(UpstreamError("test failure".to_string())) })
        .await;
    assert!(result.is_err());

    // 多次失败调用，触发熔断
    for _ in 0..10 {
        let _ = breaker
            .call_async(|| async { Err::<String, _>(UpstreamError("test failure".to_string())) })
            .await;
    }

    // 熔断器应该打开
    assert_eq!(breaker.current_state(), State::Open);
    assert!(!breaker.is_call_permitted());

    // 等待冷却时间（增加等待时间）
    sleep(Duration::from_secs(3)).await;

    // 尝试一次成功调用，这应该触发状态转换
    let _ = breaker
        .call_async(|| async { Ok::<_, UpstreamError>("success".to_string()) })
        .await;

    // 再次等待一小段时间，确保状态转换完成
    sleep(Duration::from_millis(100)).await;

    // 熔断器应该不再处于开启状态
    assert_ne!(breaker.current_state(), State::Open);
    assert!(breaker.is_call_permitted());
}

#[tokio::test]
async fn test_breaker_with_mock_server() {
    // 启动模拟服务器
    let mock_server = MockServer::start().await;
    let mock_url = mock_server.uri();

    // 创建熔断器
    let name = "mock_upstream".to_string();
    let group = "mock_group".to_string();
    let config = create_test_breaker_config(0.5, 1); // 50% 失败率阈值，1秒冷却时间

    let breaker = create_upstream_circuit_breaker(name.clone(), group.clone(), &config);

    // 设置成功响应的模拟
    Mock::given(method("GET"))
        .and(path("/success"))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
        .mount(&mock_server)
        .await;

    // 设置失败响应的模拟
    Mock::given(method("GET"))
        .and(path("/fail"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Server Error"))
        .mount(&mock_server)
        .await;

    // 创建reqwest客户端
    let client = reqwest::Client::new();

    // 测试成功请求
    let success_url = format!("{}/success", mock_url);
    let result = breaker
        .call_async(|| async {
            match client.get(&success_url).send().await {
                Ok(_) => Ok(()),
                Err(e) => Err(UpstreamError(e.to_string())),
            }
        })
        .await;
    assert!(result.is_ok());

    // 多次失败请求，触发熔断
    let fail_url = format!("{}/fail", mock_url);
    for _ in 0..10 {
        let _ = breaker
            .call_async(|| async {
                match client.get(&fail_url).send().await {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            Ok(())
                        } else {
                            Err(UpstreamError(format!(
                                "Failed with status: {}",
                                resp.status()
                            )))
                        }
                    }
                    Err(e) => Err(UpstreamError(e.to_string())),
                }
            })
            .await;
    }

    // 熔断器应该打开
    assert_eq!(breaker.current_state(), State::Open);
    assert!(!breaker.is_call_permitted());

    // 等待冷却时间（增加等待时间）
    sleep(Duration::from_secs(3)).await;

    // 尝试一次成功调用，这应该触发状态转换
    let _ = breaker
        .call_async(|| async {
            match client.get(&success_url).send().await {
                Ok(_) => Ok(()),
                Err(e) => Err(UpstreamError(e.to_string())),
            }
        })
        .await;

    // 再次等待一小段时间，确保状态转换完成
    sleep(Duration::from_millis(100)).await;

    // 熔断器应该不再处于开启状态
    assert_ne!(breaker.current_state(), State::Open);
    assert!(breaker.is_call_permitted());
}

#[tokio::test]
async fn test_load_balancer_with_circuit_breaker() {
    // 启动两个模拟服务器
    let mock_server1 = MockServer::start().await;
    let mock_server2 = MockServer::start().await;

    let mock_url1 = mock_server1.uri();
    let mock_url2 = mock_server2.uri();

    // 创建两个熔断器
    let config = create_test_breaker_config(0.5, 1);

    let breaker1 =
        create_upstream_circuit_breaker("upstream1".to_string(), "test_group".to_string(), &config);

    let breaker2 =
        create_upstream_circuit_breaker("upstream2".to_string(), "test_group".to_string(), &config);

    // 创建托管上游
    let managed_upstream1 = ManagedUpstream {
        upstream_ref: UpstreamRef {
            name: "upstream1".to_string(),
            weight: 1,
        },
        breaker: Some(breaker1),
    };

    let managed_upstream2 = ManagedUpstream {
        upstream_ref: UpstreamRef {
            name: "upstream2".to_string(),
            weight: 1,
        },
        breaker: Some(breaker2),
    };

    let upstreams = vec![managed_upstream1, managed_upstream2];

    // 创建负载均衡器
    let balancer = create_load_balancer(&BalanceStrategy::RoundRobin, upstreams);

    // 设置服务器1的响应
    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Server Error"))
        .mount(&mock_server1)
        .await;

    // 设置服务器2的响应
    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
        .mount(&mock_server2)
        .await;

    // 创建reqwest客户端
    let client = reqwest::Client::new();

    // 第一次选择应该是upstream1
    let selected = balancer.select_upstream().await.unwrap();
    assert_eq!(selected.upstream_ref.name, "upstream1");

    // 多次失败请求，触发upstream1的熔断
    let fail_url = format!("{}/test", mock_url1);
    let breaker = selected.breaker.as_ref().unwrap();
    for _ in 0..10 {
        let _ = breaker
            .call_async(|| async {
                match client.get(&fail_url).send().await {
                    Ok(resp) => {
                        if resp.status().is_success() {
                            Ok(())
                        } else {
                            Err(UpstreamError(format!(
                                "Failed with status: {}",
                                resp.status()
                            )))
                        }
                    }
                    Err(e) => Err(UpstreamError(e.to_string())),
                }
            })
            .await;
    }

    // 报告失败
    balancer.report_failure(selected).await;

    // 下一次选择应该是upstream2，因为upstream1已熔断
    let selected = balancer.select_upstream().await.unwrap();
    assert_eq!(selected.upstream_ref.name, "upstream2");

    // 测试成功请求
    let success_url = format!("{}/test", mock_url2);
    let breaker = selected.breaker.as_ref().unwrap();
    let result = breaker
        .call_async(|| async {
            match client.get(&success_url).send().await {
                Ok(resp) => {
                    if resp.status().is_success() {
                        Ok(())
                    } else {
                        Err(UpstreamError(format!(
                            "Failed with status: {}",
                            resp.status()
                        )))
                    }
                }
                Err(e) => Err(UpstreamError(e.to_string())),
            }
        })
        .await;
    assert!(result.is_ok());
}

#[tokio::test]
async fn test_all_upstreams_circuit_open() {
    // 创建两个上游，但都将是熔断状态
    let breaker1 = UpstreamCircuitBreaker::new("test1".to_string(), "group1".to_string(), 0.5, 10);
    let breaker2 = UpstreamCircuitBreaker::new("test2".to_string(), "group1".to_string(), 0.5, 10);

    // 手动将两个熔断器都设置为开启状态
    for _ in 0..10 {
        let _ = breaker1
            .call_async(|| async { Err::<(), _>(UpstreamError("test failure".to_string())) })
            .await;
        let _ = breaker2
            .call_async(|| async { Err::<(), _>(UpstreamError("test failure".to_string())) })
            .await;
    }

    // 确认熔断器状态
    assert_eq!(breaker1.current_state(), State::Open);
    assert_eq!(breaker2.current_state(), State::Open);

    // 创建托管上游
    let upstream_ref1 = UpstreamRef {
        name: "upstream1".to_string(),
        weight: 1,
    };

    let upstream_ref2 = UpstreamRef {
        name: "upstream2".to_string(),
        weight: 1,
    };

    let managed_upstream1 = ManagedUpstream {
        upstream_ref: upstream_ref1,
        breaker: Some(breaker1.clone()),
    };

    let managed_upstream2 = ManagedUpstream {
        upstream_ref: upstream_ref2,
        breaker: Some(breaker2.clone()),
    };

    let upstreams = vec![managed_upstream1, managed_upstream2];

    // 创建负载均衡器
    let balancer = create_load_balancer(&BalanceStrategy::RoundRobin, upstreams);

    // 尝试选择上游，应该失败
    let result = balancer.select_upstream().await;
    assert!(result.is_err());
    assert!(matches!(result, Err(AppError::NoHealthyUpstreamAvailable)));
}
