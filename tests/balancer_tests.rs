// tests/balancer.rs

// 将 tests/balancer_tests 目录下的模块引入
mod balancer_tests {
    #[cfg(test)]
    mod common;
    #[cfg(test)]
    mod failover;
    #[cfg(test)]
    mod integration;
    #[cfg(test)]
    mod random;
    #[cfg(test)]
    mod response_aware;
    #[cfg(test)]
    mod round_robin;
}
