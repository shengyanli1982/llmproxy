[English](./README.md) | 中文

<div align="center">
    <img src="./images/logo.png" alt="logo" width="650">
</div>

**一个具有智能调度功能的高级负载均衡系统，统一了各种大语言模型（公共/私有云、vLLM、Ollama），实现了无缝的多云和混合云集成，且仅需最少的客户端代码修改。**

## 简介

**LLMProxy** 是一款企业级、容错性强的代理服务，专为大语言模型 API 设计。它拦截客户端请求，使用可配置的路由策略将请求转发到上游 LLM API 服务器，并将响应返回给客户端。该解决方案解决了 LLM API 访问中的质量和可靠性挑战，提供精细的流量管理，并显著提高了 LLM API 交互的稳定性、性能和效率。

### 为什么选择 LLMProxy？

LLMProxy 解决了企业 LLM API 部署中的关键挑战：

-   **高可用性** - 通过跨多个 LLM 提供商的智能请求分发，消除单点故障
-   **负载均衡** - 实施复杂的负载分配策略，优化资源利用并在不同负载下维持性能
-   **容错能力** - 采用断路器模式检测故障上游服务，防止级联故障，并在服务恢复时自动恢复
-   **水平扩展性** - 通过添加更多上游服务轻松应对不断增长的请求量，而不会影响现有客户端

## 核心功能

-   🔄 **高级请求转发**

    -   通过`http_server.forwards`配置多个独立的转发服务
    -   精确控制每个转发服务的单独命名和配置
    -   为每个转发服务定义特定的监听地址和端口配置
    -   使用明确的路由规则将每个转发服务映射到指定的上游组

-   🌐 **全面的上游管理**

    -   通过`upstreams`定义和编排多个后端 LLM API 服务
    -   独立命名、配置和管理每个上游服务
    -   支持企业级身份验证机制：
        -   Bearer 令牌认证
        -   基本认证
        -   无认证（默认）
    -   精确的 HTTP 头部操作：
        -   `insert`：在不存在时添加头部
        -   `remove`：删除指定头部
        -   `replace`：替换现有头部或在不存在时添加

-   ⚡ **复杂的负载均衡**

    -   使用`upstream_groups`将上游服务组织成逻辑功能组
    -   支持多种高级负载均衡策略：
        -   **轮询（RR）** - 在服务器之间公平分配
        -   **加权轮询（WRR）** - 根据容量和性能优先考虑服务器
        -   **随机** - 非确定性选择，增强安全性和隐私性
        -   **响应时间感知** - 动态优先选择响应更快、负载更低的服务器
    -   在加权轮询策略中为每个上游配置权重（其他策略不使用权重值）

-   🔁 **精细的流量控制**

    -   为每个转发服务实施具有精确阈值的速率限制
    -   配置适合服务容量的每秒请求限制
    -   定义突发容量，有效处理流量峰值
    -   部署基于 IP 的速率限制，防止滥用并确保公平资源分配

-   🔌 **企业级连接性**

    -   **入站连接管理：** 为客户端连接配置精确的超时时间
    -   **出站连接和请求优化：**
        -   可自定义的 User-Agent 标识
        -   TCP 保活配置，确保连接稳定
        -   可配置的连接、请求和空闲超时
        -   具有可配置尝试次数和退避延迟的智能重试机制
        -   可选的 HTTP/HTTPS 代理支持，增强安全性和合规性

-   🛡️ **高级容错能力**

    -   **断路器模式：** 自动检测并隔离故障上游服务
    -   **快速故障检测：** 快速识别问题服务，最小化请求延迟
    -   **自动恢复：** 定期测试故障服务并在可用时恢复
    -   **可配置阈值：** 针对每个上游服务微调故障检测灵敏度（故障率和冷却时间）
    -   **故障隔离：** 通过限制问题在受影响的上游服务内，防止级联故障
    -   **智能故障转移：** 当一个上游服务熔断时，自动将流量转发到同一组内的健康服务

-   📊 **全面的监控和管理**

    -   通过`http_server.admin`提供独立的管理界面
    -   用于运营监控和告警的健康检查端点
    -   用于详细性能分析的 Prometheus 指标

## 架构

LLMProxy 实现了模块化、面向微服务的架构，包含以下关键组件：

-   **转发服务器**：接收和处理客户端请求的 HTTP 监听器
-   **上游管理器**：编排与 LLM API 服务器的通信，包括负载均衡和身份验证
-   **负载均衡器**：根据配置的策略（轮询、加权轮询、随机或响应时间感知）在可用上游之间智能分配请求，动态适应上游性能和负载情况
-   **断路器**：监控上游健康状况，检测持续失败的服务，自动熔断并隔离问题服务，防止级联故障，并在服务恢复后自动重新启用
-   **指标收集器**：收集并公开详细的性能和运营指标

## Prometheus 指标

LLMProxy 通过`/metrics`端点提供全面的 Prometheus 指标，用于监控性能、健康状况和运营状态。

### 上游指标

-   **llmproxy_upstream_requests_total**（计数器）- 转发到上游服务的聚合请求数，按组和上游标记
-   **llmproxy_upstream_duration_seconds**（直方图）- 上游服务的请求延迟分布，按组和上游标记
-   **llmproxy_upstream_errors_total**（计数器）- 上游请求处理期间遇到的聚合错误数，按错误类型、组和上游标记

### HTTP 请求指标

-   **llmproxy_http_requests_total**（计数器）- 代理接收的聚合传入 HTTP 请求数，按转发、方法和路径标记
-   **llmproxy_http_request_duration_seconds**（直方图）- 传入 HTTP 请求的请求延迟分布，按转发、方法和路径标记
-   **llmproxy_http_request_errors_total**（计数器）- HTTP 请求处理期间遇到的聚合错误数，按转发、错误和状态标记

### 速率限制指标

-   **llmproxy_ratelimit_total**（计数器）- 由于速率限制策略而被拒绝的聚合请求数，按转发标记

### 断路器指标

-   **llmproxy_circuitbreaker_state_changes_total**（计数器）- 断路器状态转换的聚合数，按组、上游、url 和状态标记
-   **llmproxy_circuitbreaker_calls_total**（计数器）- 通过断路器处理的聚合调用数，按组、上游、url 和结果标记
-   **llmproxy_circuitbreaker_opened_total**（计数器）- 断路器转换为打开状态的聚合数，按组、上游和 url 标记
-   **llmproxy_circuitbreaker_closed_total**（计数器）- 断路器转换为关闭状态的聚合数，按组、上游和 url 标记
-   **llmproxy_circuitbreaker_half_opened_total**（计数器）- 断路器转换为半开状态的聚合数，按组、上游和 url 标记

## API 端点

LLMProxy 公开以下 HTTP API 端点：

### 转发端点

-   **可配置的 HTTP 端点**
    -   _描述_：每个转发服务在其配置的地址和端口上监听
    -   _协议_：HTTP/HTTPS
    -   _用途_：客户端应用程序将请求定向到这些端点，然后路由到适当的上游 LLM API

### 管理端点

-   **GET /health**

    -   _描述_：用于监控和告警系统的健康检查端点
    -   _返回_：服务运行正常时返回 200 OK

-   **GET /metrics**
    -   _描述_：公开全面性能和运营统计数据的 Prometheus 指标端点
    -   _内容类型_：text/plain

## 使用场景

LLMProxy 针对以下企业场景进行了优化：

-   **企业 AI 集成**：集中 LLM API 访问，实施强大的安全策略，并实现复杂的成本优化策略
-   **AI 应用程序开发**：简化与多个 LLM 提供商的集成，显著提高可靠性
-   **云环境**：作为 sidecar 或独立服务部署，在多云和混合基础设施中提供统一的 LLM API 访问

## 响应时间感知的负载均衡算法

LLMProxy 的响应时间感知负载均衡算法是专为大语言模型这类高延迟、计算密集型服务设计的智能调度策略。与传统的轮询或随机策略不同，该算法能够动态感知上游服务的实际性能表现，自动将请求分配给当前最佳的服务节点。

### 工作原理

1. **实时性能监控**：系统持续记录每个上游节点的关键指标：

    - **平均响应时间**：使用指数移动平均（EMA）平滑处理的历史响应时间
    - **当前负载**：正在处理但尚未完成的请求数量
    - **成功率**：请求成功完成的比例

2. **综合评分机制**：根据以下公式计算每个节点的综合得分，得分越低表示节点性能越好：

    $$\text{Score} = \text{ResponseTime} \times (\text{ProcessingRequests} + 1) \times \frac{1}{\text{SuccessRate}}$$

    其中：

    - $\text{ResponseTime}$ 是节点的平均响应时间（毫秒）
    - $\text{ProcessingRequests}$ 是节点当前处理中的请求数
    - $\text{SuccessRate}$ 是节点的请求成功率（0-1 之间的值）

![score](./images/response_aware_parameter_impact_cn.png)

1. **智能选择流程**：

    - 从当前轮询位置开始，遍历所有健康的上游节点
    - 计算每个节点的综合得分
    - 选择得分最低（性能最佳）的节点处理请求
    - 增加所选节点的处理中请求计数

2. **自适应调整**：
    - 请求完成后，记录实际响应时间
    - 使用平滑因子（默认 0.15）更新节点的平均响应时间
    - 减少处理中请求计数
    - 更新节点成功率

### 优势特点

-   **动态适应性**：自动适应上游服务性能变化，无需人工干预
-   **负载均衡**：同时考虑响应时间和当前负载，避免任何单点过载
-   **平滑过渡**：使用指数移动平均技术平滑短期波动，提供稳定的负载分配
-   **高并发支持**：采用无锁设计和原子操作，在高并发场景下保持高效运行
-   **自动故障转移**：结合熔断器机制，自动跳过不健康的节点

### 适用场景

该算法特别适合以下场景：

-   **大语言模型 API 代理**：处理延迟从几百毫秒到几十秒不等的 LLM 请求
-   **异构服务环境**：上游服务器的硬件配置、负载或网络条件存在差异
-   **需求波动场景**：服务负载随时间变化明显的应用
-   **高可用性要求**：对服务质量和响应时间有严格要求的企业级应用

### 配置示例

```yaml
upstream_groups:
    - name: "llm_services"
      upstreams:
          # 注意：响应时间感知策略不使用权重值
          - name: "openai_service"
          - name: "anthropic_service"
      balance:
          strategy: "response_aware" # 启用响应时间感知负载均衡
```

## 熔断器机制

LLMProxy 集成了强大的熔断器（Circuit Breaker）模式，用于增强系统的弹性和稳定性。熔断器能够自动检测故障的上游服务，快速"断开"连接以避免资源浪费和请求堆积，并在服务恢复后自动重新启用。

### 工作原理

熔断器遵循三种状态的生命周期：

1. **关闭状态（Closed）**：

    - 正常状态，所有请求都会被转发到上游服务
    - 持续监控请求的成功/失败情况
    - 当失败率超过配置的阈值（如 50%）时，转换为开启状态

2. **开启状态（Open）**：

    - 熔断激活，快速拒绝所有请求，不再转发到故障上游
    - 返回快速失败响应，避免请求等待和资源消耗
    - 在配置的冷却时间（如 30 秒）后，转换为半开状态

3. **半开状态（Half-Open）**：
    - 允许有限数量的"探测"请求通过，测试上游服务是否恢复
    - 如果这些请求成功，认为服务已恢复，转换回关闭状态
    - 如果这些请求仍然失败，回到开启状态，继续隔离故障服务

### 智能故障转移

熔断器与负载均衡器紧密集成，提供智能故障转移能力：

-   当一个上游服务被熔断后，负载均衡器会自动将流量转发到同一组内的其他健康服务
-   只有当组内所有上游服务都不可用时，才会向客户端返回错误
-   熔断器状态对负载均衡决策透明，确保请求总是路由到健康的上游

### 优势特点

-   **快速失败**：立即识别不可用的服务，避免长时间等待
-   **资源保护**：防止对已知故障服务的请求消耗宝贵资源
-   **级联故障防护**：阻止单个服务故障影响整个系统
-   **自愈能力**：自动检测服务恢复并重新启用
-   **细粒度控制**：每个上游服务独立的熔断器配置
-   **可观测性**：通过 Prometheus 指标全面监控熔断器状态和行为

### 配置示例

```yaml
upstreams:
    - name: "openai_service"
      url: "https://api.openai.com/v1"
      breaker:
          threshold: 0.5 # 触发熔断的失败率阈值（50%）
          cooldown: 30 # 熔断后进入半开状态的冷却时间（秒）（1-3600，默认：30）

    - name: "anthropic_service"
      url: "https://api.anthropic.com"
      breaker:
          threshold: 0.3 # 对重要服务可设置更低的阈值（30%）
          cooldown: 60 # 对恢复较慢的服务可设置更长的冷却时间
```

## 配置

LLMProxy 使用结构化 YAML 文件进行配置。以下是关键配置部分的详细说明：

### 配置选项详解

#### HTTP 服务器配置选项

| 配置项                                        | 类型   | 默认值    | 说明                                        |
| --------------------------------------------- | ------ | --------- | ------------------------------------------- |
| `http_server.forwards[].name`                 | 字符串 | -         | **[必填]** 转发服务的名称，在配置中必须唯一 |
| `http_server.forwards[].port`                 | 整数   | 3000      | **[必填]** 转发服务监听的端口号             |
| `http_server.forwards[].address`              | 字符串 | "0.0.0.0" | 转发服务监听的网络地址                      |
| `http_server.forwards[].upstream_group`       | 字符串 | -         | **[必填]** 此转发服务关联的上游组名称       |
| `http_server.forwards[].ratelimit.enabled`    | 布尔值 | false     | 是否启用速率限制                            |
| `http_server.forwards[].ratelimit.per_second` | 整数   | 100       | 每秒允许来自单个 IP 的最大请求数            |
| `http_server.forwards[].ratelimit.burst`      | 整数   | 200       | 允许来自单个 IP 的突发请求数                |
| `http_server.forwards[].timeout.connect`      | 整数   | 10        | 客户端连接到 LLMProxy 的超时时间（秒）      |
| `http_server.admin.port`                      | 整数   | 9000      | 管理服务监听的端口号                        |
| `http_server.admin.address`                   | 字符串 | "0.0.0.0" | 管理服务监听的网络地址                      |
| `http_server.admin.timeout.connect`           | 整数   | 10        | 连接到管理接口的超时时间（秒）              |

#### 上游服务配置选项

| 配置项                          | 类型   | 默认值 | 说明                                               |
| ------------------------------- | ------ | ------ | -------------------------------------------------- |
| `upstreams[].name`              | 字符串 | -      | **[必填]** 上游服务的唯一名称                      |
| `upstreams[].url`               | 字符串 | -      | **[必填]** 上游服务的完整基础 URL                  |
| `upstreams[].auth.type`         | 字符串 | "none" | 认证类型：`bearer`、`basic` 或 `none`              |
| `upstreams[].auth.token`        | 字符串 | -      | 当 `type` 为 `bearer` 时的 API 密钥/令牌           |
| `upstreams[].auth.username`     | 字符串 | -      | 当 `type` 为 `basic` 时的用户名                    |
| `upstreams[].auth.password`     | 字符串 | -      | 当 `type` 为 `basic` 时的密码                      |
| `upstreams[].headers[].op`      | 字符串 | -      | HTTP 头部操作类型：`insert`、`replace` 或 `remove` |
| `upstreams[].headers[].key`     | 字符串 | -      | 要操作的 HTTP 头部名称                             |
| `upstreams[].headers[].value`   | 字符串 | -      | 对于 `insert` 或 `replace` 操作的头部值            |
| `upstreams[].breaker.threshold` | 浮点数 | 0.5    | 熔断器触发所需的失败率（0.01-1.0）                 |
| `upstreams[].breaker.cooldown`  | 整数   | 30     | 熔断器冷却时间（秒），即熔断后多久尝试进入半开状态 |

#### 上游组配置选项

| 配置项                                          | 类型   | 默认值         | 说明                                                                            |
| ----------------------------------------------- | ------ | -------------- | ------------------------------------------------------------------------------- |
| `upstream_groups[].name`                        | 字符串 | -              | **[必填]** 上游组的唯一名称                                                     |
| `upstream_groups[].upstreams[].name`            | 字符串 | -              | **[必填]** 引用的上游服务名称，必须在 `upstreams` 部分定义                      |
| `upstream_groups[].upstreams[].weight`          | 整数   | 1              | 仅在 `balance.strategy` 为 `weighted_roundrobin` 时有效的权重值                 |
| `upstream_groups[].balance.strategy`            | 字符串 | "roundrobin"   | 负载均衡策略：`roundrobin`、`weighted_roundrobin`、`random` 或 `response_aware` |
| `upstream_groups[].http_client.agent`           | 字符串 | "LLMProxy/1.0" | 发送到上游的 User-Agent 头部值                                                  |
| `upstream_groups[].http_client.keepalive`       | 整数   | 60             | TCP Keepalive 时间（秒），0-600，0 表示禁用                                     |
| `upstream_groups[].http_client.stream`          | 布尔值 | true           | 是否启用流式传输模式，对于 LLM API 的流式响应非常重要                           |
| `upstream_groups[].http_client.timeout.connect` | 整数   | 10             | 连接到上游服务的超时时间（秒）                                                  |
| `upstream_groups[].http_client.timeout.request` | 整数   | 300            | 从发送请求到接收到上游完整响应的超时时间（秒）                                  |
| `upstream_groups[].http_client.timeout.idle`    | 整数   | 60             | 与上游服务的连接在无活动后被视为空闲并关闭的超时时间（秒）                      |
| `upstream_groups[].http_client.retry.enabled`   | 布尔值 | false          | 是否启用向上游的请求重试                                                        |
| `upstream_groups[].http_client.retry.attempts`  | 整数   | 3              | 最大重试次数（不包括首次尝试）                                                  |
| `upstream_groups[].http_client.retry.initial`   | 整数   | 500            | 首次重试前的初始等待间隔（毫秒）                                                |
| `upstream_groups[].http_client.proxy.enabled`   | 布尔值 | false          | 是否启用出站代理                                                                |
| `upstream_groups[].http_client.proxy.url`       | 字符串 | -              | 代理服务器 URL                                                                  |

### HTTP 服务器配置

```yaml
http_server:
    # 转发服务配置
    forwards:
        - name: "to_mixgroup" # [必需] 转发服务的名称
          port: 3000 # [必需] 监听的端口
          address: "0.0.0.0" # [可选] 绑定的网络地址（默认："0.0.0.0"）
          upstream_group: "mixgroup" # [必需] 此转发的目标上游组
          ratelimit:
              enabled: true # 是否启用速率限制（默认：false）
              per_second: 100 # 单个IP每秒最大请求数
              burst: 200 # 单个IP的突发容量（必须 >= per_second）
          timeout:
              connect: 10 # 客户端连接超时（秒）

    # 管理界面配置
    admin:
        port: 9000 # [必需] 管理界面端口
        address: "0.0.0.0" # [可选] 绑定的网络地址（默认："0.0.0.0"）
        timeout:
            connect: 10 # 连接超时（秒）
```

### 上游配置

```yaml
upstreams:
    - name: "openai_primary" # [必需] 此上游的唯一名称
      url: "https://api.openai.com/v1" # [必需] 上游API的基础URL
      auth:
          type: "bearer" # 认证类型："bearer"、"basic"或"none"（默认）
          token: "YOUR_API_KEY" # [bearer认证必需] API密钥/令牌
          # username: "user"             # [basic认证必需] 用户名
          # password: "pass"             # [basic认证必需] 密码
      headers:
          - op: "insert" # 操作："insert"、"replace"或"remove"
            key: "X-Custom-Header" # 要操作的头部名称
            value: "MyProxyValue" # 头部值（用于"insert"或"replace"操作）
      breaker: # [可选] 断路器配置
          threshold: 0.5 # 触发断路器的故障率阈值（0.01-1.0，默认：0.5）
          cooldown: 30 # 进入半开状态前的冷却期（秒）（1-3600，默认：30）

    - name: "anthropic_primary"
      url: "https://api.anthropic.com"
      auth:
          type: "bearer"
          token: "YOUR_API_KEY"
      headers:
          - op: "insert"
            key: "x-api-key" # 某些API可能需要在头部中提供密钥
            value: "YOUR_API_KEY"
```

### 上游组配置

```yaml
upstream_groups:
    - name: "mixgroup" # [必需] 此组的唯一名称
      upstreams: # [必需] 上游引用列表
          - name: "openai_primary" # 必须匹配上面定义的上游名称
            weight: 8 # [可选] 仅在加权轮询(weighted_roundrobin)策略中有效，其他策略忽略此值
          - name: "anthropic_primary"
            weight: 2 # 仅在加权轮询策略中有效
      balance:
          strategy:
              "weighted_roundrobin" # 负载均衡策略："roundrobin"（默认）、
              # "weighted_roundrobin"、"random"或"response_aware"
      http_client:
          agent: "LLMProxy/1.0" # [可选] User-Agent头部（默认："LLMProxy/1.0"）
          keepalive: 60 # [可选] TCP保活（秒）（0-600，0=禁用）
          stream: true # [可选] 启用流模式（默认：true）
          timeout:
              connect: 10 # 连接超时（秒）（默认：10）
              request: 300 # 请求超时（秒）（默认：300）
              idle: 60 # 空闲连接超时（秒）（默认：60）
          retry:
              enabled: true # 是否启用重试（默认：false）
              attempts: 3 # 最大重试次数
              initial: 500 # 初始重试延迟（毫秒）
          proxy:
              enabled: false # 是否使用出站代理（默认：false）
              url: "http://user:pass@proxy:8080" # 代理服务器URL
```

### 配置最佳实践

1. **安全考虑**：

    - 对于生产环境，实施包含敏感 API 密钥的配置文件的安全存储
    - 利用环境变量或专用的密钥管理服务进行凭据管理
    - 通过绑定到 localhost（`127.0.0.1`）并实施适当的身份验证来限制管理界面访问

2. **性能优化**：

    - 根据特定 LLM 提供商的响应特性微调超时配置
    - 启用`stream: true`以高效流式传输 LLM 响应
    - 实施适当的速率限制，保护代理基础设施和上游服务

3. **可靠性增强**：
    - 为幂等请求启用智能重试机制
    - 实施加权轮询负载均衡，优先考虑更可靠或更高容量的提供商
    - 在每个组中配置多个上游，实现冗余和故障转移能力
    - 部署具有适当阈值的断路器，快速隔离故障服务
    - 根据上游恢复模式设置合理的断路器冷却期
    - 监控断路器指标，识别反复出现的上游稳定性问题
    - 对于高延迟场景（如大语言模型），使用响应时间感知的负载均衡策略
    - 利用响应时间感知负载均衡自动将流量引导到性能更好的上游服务

有关详细说明所有可用选项的综合配置参考，请参阅 LLMProxy 附带的`config.default.yaml`文件。

## 部署

LLMProxy 支持多种部署方式，包括 Docker、Kubernetes 和传统的系统服务。以下是各种部署方法的详细说明：

### Docker 部署

使用 Docker Compose 是部署 LLMProxy 最简单的方式之一。在项目的 `examples/config` 目录中提供了一个完整的 Docker Compose 配置文件。

1. **准备配置文件**：

    将您的 `config.yaml` 文件放在与 `docker-compose.yaml` 相同的目录中。

2. **启动服务**：

    ```bash
    docker-compose up -d
    ```

3. **查看日志**：

    ```bash
    docker-compose logs -f
    ```

4. **停止服务**：

    ```bash
    docker-compose down
    ```

Docker Compose 配置示例：

```yaml
version: "3"

services:
    llmproxy:
        image: shengyanli1982/llmproxy:latest
        container_name: llmproxy
        restart: unless-stopped
        ports:
            # 转发服务端口
            - "3000:3000" # to_mixgroup
            - "3001:3001" # openai_group
            # 管理界面端口
            - "9000:9000" # admin
        volumes:
            - ./config.yaml:/app/config.yaml:ro
        command: ["--config", "/app/config.yaml"]
        environment:
            - TZ=Asia/Shanghai
        networks:
            - llmproxy-network

networks:
    llmproxy-network:
        driver: bridge
```

### Kubernetes 部署

对于 Kubernetes 环境，我们提供了完整的部署配置文件，位于 `examples/config/kubernetes` 目录中。

1. **创建命名空间和配置**：

    ```bash
    # 设置 API 密钥环境变量
    export OPENAI_API_KEY="your_openai_api_key"
    export ANTHROPIC_API_KEY="your_anthropic_api_key"

    # 进入 kubernetes 配置目录
    cd examples/config/kubernetes

    # 使用部署脚本
    kubectl apply -f namespace.yaml
    kubectl apply -f configmap.yaml
    kubectl apply -f service.yaml
    kubectl apply -f deployment.yaml
    ```

2. **验证部署**：

    ```bash
    kubectl get pods -n llmproxy
    kubectl get services -n llmproxy
    ```

3. **访问服务**：

    如果您在集群内部访问服务，可以使用服务名称：

    ```
    http://llmproxy.llmproxy.svc.cluster.local:3000
    ```

    如果需要从集群外部访问，可以设置 Ingress 或使用端口转发：

    ```bash
    kubectl port-forward svc/llmproxy -n llmproxy 3000:3000 3001:3001 9000:9000
    ```

### Linux 系统服务部署

对于传统的 Linux 服务器部署，我们提供了 systemd 服务文件。

1. **下载并安装二进制文件**：

    ```bash
    # 下载最新版本
    curl -L -o llmproxyd https://github.com/shengyanli1982/llmproxy/releases/latest/download/llmproxyd-Linux-x64-<version>.zip

    # 解压 zip
    unzip -x llmproxyd-Linux-x64-<version>.zip

    # 添加执行权限
    chmod +x llmproxyd

    # 移动到系统目录
    sudo mkdir -p /opt/llmproxy
    sudo mv llmproxyd /opt/llmproxy/
    ```

2. **创建配置文件**：

    ```bash
    sudo mkdir -p /opt/llmproxy
    sudo nano /opt/llmproxy/config.yaml
    # 将您的配置粘贴到编辑器中
    ```

3. **创建系统用户**：

    ```bash
    sudo useradd -r -s /bin/false llmproxy
    sudo chown -R llmproxy:llmproxy /opt/llmproxy
    ```

4. **安装 systemd 服务文件**：

    ```bash
    sudo cp examples/config/llmproxy.service /etc/systemd/system/
    sudo systemctl daemon-reload
    ```

5. **启动和启用服务**：

    ```bash
    sudo systemctl start llmproxy
    sudo systemctl enable llmproxy
    ```

6. **检查服务状态**：

    ```bash
    sudo systemctl status llmproxy
    ```

### 安全建议

无论您选择哪种部署方式，都应考虑以下安全最佳实践：

1. **API 密钥保护**：

    - 避免在配置文件中硬编码 API 密钥
    - 使用环境变量、密钥管理服务或 Kubernetes Secrets

2. **网络安全**：

    - 限制管理接口（端口 9000）的访问，仅对内部网络开放
    - 考虑使用反向代理（如 Nginx）添加额外的身份验证层

3. **最小权限原则**：

    - 使用专用的非特权用户运行服务
    - 限制服务对文件系统的访问权限

4. **监控和日志**：
    - 配置日志聚合和监控
    - 设置 Prometheus 告警以检测异常行为

## 许可证

[MIT 许可证](LICENSE)
