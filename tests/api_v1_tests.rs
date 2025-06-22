//! API v1 集成测试模块

// 将tests/api_v1_tests目录下的模块引入
mod api_v1_tests {
    // 导出helpers模块，使其可以被其他测试模块使用
    pub mod helpers;
    // 测试模块
    #[cfg(test)]
    mod forwards;
    #[cfg(test)]
    mod upstream_groups;
    #[cfg(test)]
    mod upstreams;
}
