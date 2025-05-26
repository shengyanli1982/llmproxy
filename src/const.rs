// 应用常量定义

//
// 配置参数限制常量
//

// 应用关闭等待时间限制
pub mod shutdown_timeout {
    // 默认值
    pub const DEFAULT: u64 = 30;
    // 最小值
    pub const MIN: u64 = 1;
    // 最大值
    pub const MAX: u64 = 120;
}

// HTTP客户端配置限制
pub mod http_client_limits {
    // 默认连接超时（秒）
    pub const DEFAULT_CONNECT_TIMEOUT: u64 = 60;
    // 最小连接超时（秒）
    pub const MIN_CONNECT_TIMEOUT: u64 = 1;
    // 最大连接超时（秒）
    pub const MAX_CONNECT_TIMEOUT: u64 = 120;
    // 默认请求超时（秒）
    pub const DEFAULT_REQUEST_TIMEOUT: u64 = 120;
    // 最小请求超时（秒）
    pub const MIN_REQUEST_TIMEOUT: u64 = 1;
    // 最大请求超时（秒）
    pub const MAX_REQUEST_TIMEOUT: u64 = 1200;
    // 默认空闲超时（秒）
    pub const DEFAULT_IDLE_TIMEOUT: u64 = 60;
    // 最小空闲超时（秒）
    pub const MIN_IDLE_TIMEOUT: u64 = 5;
    // 最大空闲超时（秒）
    pub const MAX_IDLE_TIMEOUT: u64 = 1800;
    // 默认keepalive时间（秒）
    pub const DEFAULT_KEEPALIVE: u32 = 30;
    // 最小keepalive时间（秒）
    pub const MIN_KEEPALIVE: u32 = 5;
    // 最大keepalive时间（秒）
    pub const MAX_KEEPALIVE: u32 = 600;
}

// 重试配置限制
pub mod retry_limits {
    // 最小重试次数
    pub const MIN_ATTEMPTS: u32 = 1;
    // 最大重试次数
    pub const MAX_ATTEMPTS: u32 = 100;
    // 最小重试延迟（秒）
    pub const MIN_DELAY: u32 = 1;
    // 最大重试延迟（秒）
    pub const MAX_DELAY: u32 = 120;
    // 默认重试次数
    pub const DEFAULT_ATTEMPTS: u32 = 3;
    // 默认初始重试延迟（毫秒）
    pub const DEFAULT_INITIAL_MS: u32 = 500;
    // 最小初始重试延迟（毫秒）
    pub const MIN_INITIAL_MS: u32 = 100;
    // 最大初始重试延迟（毫秒）
    pub const MAX_INITIAL_MS: u32 = 10000;
}

// 权重配置限制
pub mod weight_limits {
    // 最小权重值
    pub const MIN_WEIGHT: u32 = 1;
    // 最大权重值
    pub const MAX_WEIGHT: u32 = 65535;
}

// 限流配置限制
pub mod rate_limit_limits {
    // 最小每秒请求数
    pub const MIN_PER_SECOND: u32 = 1;
    // 最大每秒请求数
    pub const MAX_PER_SECOND: u32 = 10000;
    // 最小突发请求数
    pub const MIN_BURST: u32 = 1;
    // 最大突发请求数
    pub const MAX_BURST: u32 = 20000;
    // 默认每秒请求数
    pub const DEFAULT_PER_SECOND: u32 = 100;
    // 默认突发请求数
    pub const DEFAULT_BURST: u32 = 200;
}

// HTTP 头部常量
pub mod http_headers {
    // 内容类型头部
    pub const CONTENT_TYPE: &str = "content-type";
    // 传输编码头部
    pub const TRANSFER_ENCODING: &str = "transfer-encoding";

    // 内容类型值
    pub mod content_types {
        // 事件流内容类型
        pub const EVENT_STREAM: &str = "text/event-stream";
    }

    // 传输编码值
    pub mod transfer_encodings {
        // 分块传输编码
        pub const CHUNKED: &str = "chunked";
    }
}

//
// 指标标签常量
//

// 错误类型标签
pub mod error_labels {
    // 上游错误
    pub const UPSTREAM_ERROR: &str = "upstream_error";
    // 选择错误
    pub const SELECT_ERROR: &str = "select_error";
    // 请求错误
    pub const REQUEST_ERROR: &str = "request_error";
    // 路由错误
    pub const ROUTE_ERROR: &str = "route_error";
    // 配置错误
    pub const CONFIG_ERROR: &str = "config_error";
    // 验证错误
    pub const VALIDATION_ERROR: &str = "validation_error";
}

// 上游标签
pub mod upstream_labels {
    // 未知上游
    pub const UNKNOWN: &str = "unknown";
    // 重试
    pub const RETRY: &str = "retry";
}

// 负载均衡策略标签
pub mod balance_strategy_labels {
    // 轮询
    pub const ROUND_ROBIN: &str = "roundrobin";
    // 加权轮询
    pub const WEIGHTED_ROUND_ROBIN: &str = "weighted_roundrobin";
    // 随机
    pub const RANDOM: &str = "random";
}
