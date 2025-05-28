[English](./README.md) | 中文

<div align="center">
    <img src="./images/logo.png" alt="logo" width="650">
</div>

**一款具有智能调度功能的企业级负载均衡系统，可统一调度各类大语言模型（公有云/私有云、vLLM、Ollama 等），实现多云和混合云的无缝集成，并且仅需对客户端代码进行最小化修改。**

<p align="center">
  <a href="#简介">简介</a>
  |
  <a href="#核心功能">核心功能</a>
  |
  <a href="#架构">架构</a>
  |
  <a href="#prometheus-指标">Prometheus 指标</a>
  |
  <a href="#api-端点">API 端点</a>
  |
  <a href="#应用场景">应用场景</a>
  |
  <a href="#响应时间感知的负载均衡算法">负载均衡算法</a>
  |
  <a href="#熔断器机制">熔断器机制</a>
  |
  <a href="#配置">配置</a>
  |
  <a href="#部署指南">部署指南</a>
</p>

## 简介

**LLMProxy** 是一款专为大语言模型 API 设计的企业级高可用代理服务。它能拦截客户端请求，通过可配置的路由策略将请求智能转发至上游 LLM API 服务器，并将响应返回给客户端。该解决方案有效解决了 LLM API 调用中的质量和可靠性挑战，提供精细化的流量管理，显著提升了大语言模型 API 交互的稳定性、性能和效率。

### 为什么选择 LLMProxy？

LLMProxy 有效解决了企业级 LLM API 部署中的关键挑战：

-   **高可用性** - 通过跨多个 LLM 提供商的智能请求分发机制，彻底消除单点故障风险
-   **负载均衡** - 实现复杂的负载分配策略，优化资源利用率，在各种负载条件下保持稳定性能
-   **容错能力** - 采用断路器模式实时检测故障上游服务，有效防止级联故障，并在服务恢复后自动重连
-   **水平扩展性** - 只需添加更多上游服务即可轻松应对不断增长的请求量，且不影响现有客户端

## 核心功能

-   🔄 **高级请求转发**

    -   通过`http_server.forwards`配置多个独立转发服务
    -   对每个转发服务实现精确命名和独立配置
    -   为每个转发服务定制专属监听地址和端口
    -   使用明确的路由规则将转发服务映射至指定上游服务组

-   🌐 **全面的上游管理**

    -   通过`upstreams`定义和灵活编排多个后端 LLM API 服务
    -   对每个上游服务进行独立命名、配置与管理
    -   支持企业级身份验证机制：
        -   Bearer 令牌认证
        -   基本认证
        -   无认证（默认）
    -   精确的 HTTP 头部操作：
        -   `insert`：头部不存在时添加
        -   `remove`：删除指定头部
        -   `replace`：替换现有头部或在不存在时添加

-   ⚡ **复杂的负载均衡**

    -   使用`upstream_groups`将上游服务组织为逻辑功能组
    -   支持多种高级负载均衡策略：
        -   **轮询（RR）** - 实现服务器间的公平分配
        -   **加权轮询（WRR）** - 根据服务器容量和性能进行优先级调度
        -   **随机** - 非确定性选择，增强安全性和隐私保护
        -   **响应时间感知** - 动态优先选择响应更快、负载更低的服务器
    -   在加权轮询策略中可为每个上游设置权重（其他策略不使用权重值）

-   🔁 **精细的流量控制**

    -   为每个转发服务配置精确阈值的速率限制
    -   根据服务容量定制每秒请求限制
    -   配置突发容量，高效处理流量峰值
    -   实施基于 IP 的速率限制，防止滥用并确保资源公平分配

-   🔌 **企业级连接管理**

    -   **入站连接管理：** 为客户端连接配置精确超时参数
    -   **出站连接和请求优化：**
        -   可自定义 User-Agent 标识
        -   TCP 保活配置，确保连接稳定性
        -   可调整的连接、请求和空闲超时
        -   智能重试机制，支持可配置的尝试次数和退避延迟
        -   可选的 HTTP/HTTPS 代理支持，增强安全性和合规性

-   🛡️ **高级容错机制**

    -   **断路器模式：** 自动检测并隔离故障上游服务
    -   **快速故障检测：** 迅速识别问题服务，最小化请求延迟
    -   **自动恢复：** 定期测试故障服务并在恢复时自动重连
    -   **可配置阈值：** 针对每个上游服务精细调整故障检测灵敏度（故障率和冷却时间）
    -   **故障隔离：** 将问题限制在受影响的上游服务内，有效防止级联故障
    -   **智能故障转移：** 当上游服务熔断时，自动将流量转发至同组内的健康服务

-   📊 **全面的监控与管理**

    -   通过`http_server.admin`提供独立管理界面
    -   为运维监控和告警系统提供健康检查端点
    -   提供详细的 Prometheus 指标，支持深入性能分析

## 架构

LLMProxy 采用模块化、面向微服务的架构设计，包含以下核心组件：

-   **转发服务器**：接收并处理客户端请求的 HTTP 监听器，负责请求的初步处理和路由
-   **上游管理器**：协调与 LLM API 服务器的通信，管理负载均衡和身份验证流程
-   **负载均衡器**：根据配置策略（轮询、加权轮询、随机或响应时间感知）在可用上游服务间智能分配请求，实时适应上游性能和负载变化
-   **断路器**：持续监控上游健康状况，检测持续失败的服务，自动熔断并隔离问题服务，防止故障扩散，并在服务恢复后自动重新启用
-   **指标收集器**：实时收集并暴露详细的性能和运营指标，支持系统监控和问题诊断

![architecture](./images/architecture.png)

## Prometheus 指标

LLMProxy 通过`/metrics`端点提供全面的 Prometheus 指标，用于实时监控系统性能、健康状况和运营状态。

### 上游指标

-   **llmproxy_upstream_requests_total**（计数器）- 按组和上游分类统计的转发请求总数
-   **llmproxy_upstream_duration_seconds**（直方图）- 按组和上游分类的请求延迟分布
-   **llmproxy_upstream_errors_total**（计数器）- 按错误类型、组和上游分类的错误总数

### HTTP 请求指标

-   **llmproxy_http_requests_total**（计数器）- 按转发服务、方法和路径分类的入站 HTTP 请求总数
-   **llmproxy_http_request_duration_seconds**（直方图）- 按转发服务、方法和路径分类的请求延迟分布
-   **llmproxy_http_request_errors_total**（计数器）- 按转发服务、错误类型和状态码分类的错误总数

### 速率限制指标

-   **llmproxy_ratelimit_total**（计数器）- 按转发服务分类的因速率限制而被拒绝的请求总数

### 断路器指标

-   **llmproxy_circuitbreaker_state_changes_total**（计数器）- 按组、上游、URL 和状态分类的断路器状态转换总数
-   **llmproxy_circuitbreaker_calls_total**（计数器）- 按组、上游、URL 和结果分类的断路器处理调用总数
-   **llmproxy_circuitbreaker_opened_total**（计数器）- 按组、上游和 URL 分类的断路器开启次数总计
-   **llmproxy_circuitbreaker_closed_total**（计数器）- 按组、上游和 URL 分类的断路器关闭次数总计
-   **llmproxy_circuitbreaker_half_opened_total**（计数器）- 按组、上游和 URL 分类的断路器半开状态转换次数总计

## API 端点

LLMProxy 对外暴露以下 HTTP API 端点：

### 转发端点

-   **可配置的 HTTP 端点**
    -   _描述_：每个转发服务在其配置的地址和端口上监听请求
    -   _协议_：HTTP/HTTPS
    -   _用途_：客户端应用程序将请求发送至这些端点，系统随后将请求路由至相应的上游 LLM API

### 管理端点

-   **GET /health**

    -   _描述_：供监控和告警系统使用的健康检查端点
    -   _返回_：系统运行正常时返回 200 OK 状态码

-   **GET /metrics**
    -   _描述_：提供全面性能和运营统计数据的 Prometheus 指标端点
    -   _内容类型_：text/plain

## 应用场景

LLMProxy 专为以下企业级应用场景优化设计：

-   **企业 AI 集成**：集中管理 LLM API 访问，实施强大的安全策略，优化成本控制
-   **AI 应用开发**：大幅简化与多个 LLM 提供商的集成流程，显著提升系统可靠性
-   **多云环境部署**：可作为 sidecar 或独立服务部署，在多云和混合基础设施中提供统一的 LLM API 访问层

## 响应时间感知的负载均衡算法

LLMProxy 的响应时间感知负载均衡算法是专为大语言模型这类高延迟、计算密集型服务设计的智能调度策略。与传统的轮询或随机策略不同，该算法能够实时感知上游服务的性能表现，自动将请求分配给当前最优的服务节点。

### 工作原理

1. **实时性能监控**：系统持续采集并记录每个上游节点的关键性能指标：

    - **平均响应时间**：采用指数移动平均（EMA）平滑处理的历史响应时间
    - **当前负载**：正在处理但尚未完成的并发请求数量
    - **成功率**：请求成功完成的百分比

2. **综合评分机制**：根据以下公式计算每个节点的综合性能得分，得分越低表示节点性能越优：

    $$\text{Score} = \text{ResponseTime} \times (\text{ProcessingRequests} + 1) \times \frac{1}{\text{SuccessRate}}$$

    其中：

    - $\text{ResponseTime}$ 是节点的平均响应时间（毫秒）
    - $\text{ProcessingRequests}$ 是节点当前处理中的并发请求数
    - $\text{SuccessRate}$ 是节点的请求成功率（0-1 之间的值）

![score](./images/response_aware_parameter_impact_cn.png)

3. **智能选择流程**：

    - 从当前轮询位置开始，遍历所有处于健康状态的上游节点
    - 分别计算每个节点的综合性能得分
    - 选择得分最低（即性能最佳）的节点处理当前请求
    - 增加所选节点的处理中请求计数

4. **自适应调整**：
    - 请求完成后，记录实际响应时间
    - 使用平滑因子（默认值 0.15）更新节点的平均响应时间
    - 减少处理中请求计数
    - 更新节点的成功率统计

### 优势特点

-   **动态适应性**：自动适应上游服务性能波动，无需人工干预
-   **多维度负载均衡**：同时考虑响应时间和当前负载，避免任何单点过载
-   **平滑过渡**：采用指数移动平均技术平滑短期波动，提供稳定的负载分配
-   **高并发支持**：采用无锁设计和原子操作，在高并发场景下保持高效运行
-   **自动故障规避**：结合熔断器机制，自动跳过不健康的节点

### 适用场景

该算法特别适合以下应用场景：

-   **大语言模型 API 代理**：处理延迟从数百毫秒到数十秒不等的 LLM 请求
-   **异构服务环境**：上游服务器在硬件配置、负载能力或网络条件上存在差异
-   **需求波动明显**：服务负载随时间变化显著的应用场景
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

LLMProxy 集成了强大的熔断器（Circuit Breaker）模式，用于增强系统的弹性和稳定性。熔断器能够自动检测故障上游服务，快速"断开"连接以避免资源浪费和请求堆积，并在服务恢复后自动重新接入。

### 工作原理

熔断器遵循三种状态的生命周期模型：

1. **关闭状态（Closed）**：

    - 系统正常运行状态，所有请求均会被转发到上游服务
    - 持续监控请求的成功/失败情况
    - 当失败率超过配置的阈值（如 50%）时，状态转换为开启

2. **开启状态（Open）**：

    - 熔断器激活，系统快速拒绝所有请求，不再转发到故障上游
    - 返回快速失败响应，避免请求等待和资源消耗
    - 在配置的冷却时间（如 30 秒）后，状态转换为半开

3. **半开状态（Half-Open）**：
    - 系统允许有限数量的"探测"请求通过，测试上游服务是否已恢复
    - 如果这些探测请求成功完成，判定服务已恢复，状态转换回关闭
    - 如果探测请求仍然失败，回到开启状态，继续隔离故障服务

### 智能故障转移

熔断器与负载均衡器紧密集成，提供智能故障转移能力：

-   当一个上游服务被熔断后，负载均衡器自动将流量重新分配到同一组内的其他健康服务
-   仅当组内所有上游服务都不可用时，系统才会向客户端返回错误响应
-   熔断器状态对负载均衡决策保持透明，确保请求始终路由到健康的上游服务

### 优势特点

-   **快速失败检测**：立即识别不可用服务，避免长时间等待超时
-   **资源保护机制**：防止对已知故障服务的请求继续消耗宝贵系统资源
-   **级联故障防护**：有效阻止单个服务故障扩散影响整个系统
-   **自愈能力**：自动检测服务恢复并重新启用连接
-   **细粒度控制**：为每个上游服务提供独立的熔断器配置选项
-   **全面可观测性**：通过 Prometheus 指标全方位监控熔断器状态和行为

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
          threshold: 0.3 # 对关键服务可设置更低的阈值（30%）
          cooldown: 60 # 对恢复较慢的服务可设置更长的冷却时间
```

## 配置

LLMProxy 采用结构化 YAML 文件进行配置，提供灵活且强大的配置选项。以下是关键配置部分的详细说明：

### 配置选项详解

#### HTTP 服务器配置选项

| 配置项                                        | 类型   | 默认值    | 说明                                   |
| --------------------------------------------- | ------ | --------- | -------------------------------------- |
| `http_server.forwards[].name`                 | 字符串 | -         | **[必填]** 转发服务的唯一标识名称      |
| `http_server.forwards[].port`                 | 整数   | 3000      | **[必填]** 转发服务的监听端口          |
| `http_server.forwards[].address`              | 字符串 | "0.0.0.0" | 转发服务的绑定网络地址                 |
| `http_server.forwards[].upstream_group`       | 字符串 | -         | **[必填]** 此转发服务关联的上游组名称  |
| `http_server.forwards[].ratelimit.enabled`    | 布尔值 | false     | 是否启用速率限制功能                   |
| `http_server.forwards[].ratelimit.per_second` | 整数   | 100       | 单个 IP 每秒允许的最大请求数           |
| `http_server.forwards[].ratelimit.burst`      | 整数   | 200       | 单个 IP 允许的突发请求数（缓冲区大小） |
| `http_server.forwards[].timeout.connect`      | 整数   | 10        | 客户端连接到 LLMProxy 的超时时间（秒） |
| `http_server.admin.port`                      | 整数   | 9000      | 管理服务的监听端口                     |
| `http_server.admin.address`                   | 字符串 | "0.0.0.0" | 管理服务的绑定网络地址                 |
| `http_server.admin.timeout.connect`           | 整数   | 10        | 连接到管理接口的超时时间（秒）         |

#### 上游服务配置选项

| 配置项                          | 类型   | 默认值 | 说明                                               |
| ------------------------------- | ------ | ------ | -------------------------------------------------- |
| `upstreams[].name`              | 字符串 | -      | **[必填]** 上游服务的唯一标识名称                  |
| `upstreams[].url`               | 字符串 | -      | **[必填]** 上游服务的基础 URL                      |
| `upstreams[].auth.type`         | 字符串 | "none" | 认证类型：`bearer`、`basic`或`none`                |
| `upstreams[].auth.token`        | 字符串 | -      | 当`type`为`bearer`时的 API 密钥或令牌              |
| `upstreams[].auth.username`     | 字符串 | -      | 当`type`为`basic`时的用户名                        |
| `upstreams[].auth.password`     | 字符串 | -      | 当`type`为`basic`时的密码                          |
| `upstreams[].headers[].op`      | 字符串 | -      | HTTP 头部操作类型：`insert`、`replace`或`remove`   |
| `upstreams[].headers[].key`     | 字符串 | -      | 要操作的 HTTP 头部名称                             |
| `upstreams[].headers[].value`   | 字符串 | -      | 用于`insert`或`replace`操作的头部值                |
| `upstreams[].breaker.threshold` | 浮点数 | 0.5    | 熔断器触发阈值，表示失败率（0.01-1.0）             |
| `upstreams[].breaker.cooldown`  | 整数   | 30     | 熔断器冷却时间（秒），即熔断后多久尝试进入半开状态 |

#### 上游组配置选项

> [!NOTE]
>
> 参数 `upstreams[].url` 需要配置上游服务的基础 URL，例如：`https://api.openai.com/v1`，而不是`https://api.openai.com` 或者 `https://api.openai.com/v1/chat/completions`。

| 配置项                                          | 类型   | 默认值         | 说明                                                                          |
| ----------------------------------------------- | ------ | -------------- | ----------------------------------------------------------------------------- |
| `upstream_groups[].name`                        | 字符串 | -              | **[必填]** 上游组的唯一标识名称                                               |
| `upstream_groups[].upstreams[].name`            | 字符串 | -              | **[必填]** 引用的上游服务名称，必须在`upstreams`部分已定义                    |
| `upstream_groups[].upstreams[].weight`          | 整数   | 1              | 仅在`balance.strategy`为`weighted_roundrobin`时有效的权重值                   |
| `upstream_groups[].balance.strategy`            | 字符串 | "roundrobin"   | 负载均衡策略：`roundrobin`、`weighted_roundrobin`、`random`或`response_aware` |
| `upstream_groups[].http_client.agent`           | 字符串 | "LLMProxy/1.0" | 发送到上游的 User-Agent 头部值                                                |
| `upstream_groups[].http_client.keepalive`       | 整数   | 60             | TCP Keepalive 时间（秒），范围 0-600，0 表示禁用                              |
| `upstream_groups[].http_client.stream`          | 布尔值 | true           | 是否启用流式传输模式，对 LLM API 的流式响应至关重要                           |
| `upstream_groups[].http_client.timeout.connect` | 整数   | 10             | 连接到上游服务的超时时间（秒）                                                |
| `upstream_groups[].http_client.timeout.request` | 整数   | 300            | 请求超时时间（秒），从发送请求到接收完整响应的最大等待时间                    |
| `upstream_groups[].http_client.timeout.idle`    | 整数   | 60             | 空闲连接超时时间（秒），连接在无活动后被关闭的时间                            |
| `upstream_groups[].http_client.retry.enabled`   | 布尔值 | false          | 是否启用请求重试功能                                                          |
| `upstream_groups[].http_client.retry.attempts`  | 整数   | 3              | 最大重试次数（不包括首次尝试）                                                |
| `upstream_groups[].http_client.retry.initial`   | 整数   | 500            | 首次重试前的初始等待时间（毫秒）                                              |
| `upstream_groups[].http_client.proxy.enabled`   | 布尔值 | false          | 是否启用出站代理                                                              |
| `upstream_groups[].http_client.proxy.url`       | 字符串 | -              | 代理服务器 URL                                                                |

### HTTP 服务器配置

```yaml
http_server:
    # 转发服务配置
    forwards:
        - name: "to_mixgroup" # [必需] 转发服务的唯一名称
          port: 3000 # [必需] 监听端口
          address: "0.0.0.0" # [可选] 绑定的网络地址（默认："0.0.0.0"）
          upstream_group: "mixgroup" # [必需] 此转发对应的目标上游组
          ratelimit:
              enabled: true # 是否启用速率限制（默认：false）
              per_second: 100 # 单个IP每秒最大请求数
              burst: 200 # 单个IP的突发请求容量（必须 >= per_second）
          timeout:
              connect: 10 # 客户端连接超时时间（秒）

    # 管理界面配置
    admin:
        port: 9000 # [必需] 管理界面端口
        address: "0.0.0.0" # [可选] 绑定的网络地址（默认："0.0.0.0"）
        timeout:
            connect: 10 # 连接超时时间（秒）
```

### 上游服务配置

```yaml
upstreams:
    - name: "openai_primary" # [必需] 上游服务的唯一标识名称
      url: "https://api.openai.com/v1" # [必需] 上游API的基础URL
      auth:
          type: "bearer" # 认证类型："bearer"、"basic"或"none"（默认）
          token: "YOUR_API_KEY" # [bearer认证必需] API密钥/令牌
          # username: "user"             # [basic认证必需] 用户名
          # password: "pass"             # [basic认证必需] 密码
      headers:
          - op: "insert" # 操作类型："insert"、"replace"或"remove"
            key: "X-Custom-Header" # 要操作的HTTP头部名称
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
            key: "x-api-key" # 某些API可能需要在头部中额外提供密钥
            value: "YOUR_API_KEY"
```

### 上游组配置

```yaml
upstream_groups:
    - name: "mixgroup" # [必需] 上游组的唯一标识名称
      upstreams: # [必需] 上游服务引用列表
          - name: "openai_primary" # 必须匹配已定义的上游服务名称
            weight: 8 # [可选] 仅在加权轮询(weighted_roundrobin)策略中有效
          - name: "anthropic_primary"
            weight: 2 # 仅在加权轮询策略中有效
      balance:
          strategy:
              "weighted_roundrobin" # 负载均衡策略：
              # "roundrobin"（默认）、"weighted_roundrobin"、
              # "random"或"response_aware"
      http_client:
          agent: "LLMProxy/1.0" # [可选] 自定义User-Agent头部（默认："LLMProxy/1.0"）
          keepalive: 60 # [可选] TCP保活时间（秒）（0-600，0=禁用）
          stream: true # [可选] 启用流式传输模式（默认：true）
          timeout:
              connect: 10 # 连接超时时间（秒）（默认：10）
              request: 300 # 请求超时时间（秒）（默认：300）
              idle: 60 # 空闲连接超时时间（秒）（默认：60）
          retry:
              enabled: true # 是否启用请求重试（默认：false）
              attempts: 3 # 最大重试次数
              initial: 500 # 初始重试延迟（毫秒）
          proxy:
              enabled: false # 是否使用出站代理（默认：false）
              url: "http://user:pass@proxy:8080" # 代理服务器URL
```

### 配置最佳实践

1. **安全建议**：

    - 在生产环境中，采用安全存储机制保护包含敏感 API 密钥的配置文件
    - 优先使用环境变量或专业密钥管理服务（如 Vault、AWS Secrets Manager 等）进行凭据管理
    - 通过将管理界面绑定到本地地址（`127.0.0.1`）并实施适当的身份验证机制，限制管理接口的访问

2. **性能优化**：

    - 根据不同 LLM 提供商的响应特性，精细调整各项超时配置
    - 对支持流式响应的 LLM 服务，确保启用`stream: true`配置，优化响应传输效率
    - 根据实际系统负载和上游服务容量，配置合理的速率限制，同时保护代理基础设施和上游服务

3. **可靠性提升**：
    - 为幂等性请求启用智能重试机制，提高请求成功率
    - 实施加权轮询负载均衡策略，优先考虑更可靠或处理能力更强的提供商
    - 在每个上游组中配置多个服务提供商，实现冗余和自动故障转移
    - 为每个上游服务配置适当阈值的断路器，快速识别并隔离故障服务
    - 根据不同上游服务的恢复特性，设置合理的断路器冷却期
    - 定期监控断路器指标，及时发现并解决频繁出现的上游稳定性问题
    - 在处理大语言模型等高延迟应用场景时，优先使用响应时间感知的负载均衡策略
    - 充分利用响应时间感知算法，自动将流量引导至性能最佳的上游服务

有关所有可用配置选项的详细说明，请参阅 LLMProxy 项目附带的`config.default.yaml`文件作为完整参考。

## 部署指南

LLMProxy 支持多种灵活的部署方式，包括 Docker 容器化部署、Kubernetes 集群部署和传统 Linux 系统服务部署。以下是各部署方法的详细说明：

### Docker 部署

使用 Docker Compose 是部署 LLMProxy 最便捷的方式之一。项目的`examples/config`目录中提供了完整的 Docker Compose 配置示例。

1. **准备配置文件**：

    将自定义的`config.yaml`文件放置在与`docker-compose.yaml`相同的目录中。

2. **启动服务**：

    ```bash
    docker-compose up -d
    ```

3. **查看运行日志**：

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
            # 转发服务端口映射
            - "3000:3000" # to_mixgroup
            - "3001:3001" # openai_group
            # 管理界面端口映射
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

对于 Kubernetes 环境，我们在`examples/config/kubernetes`目录中提供了完整的部署配置文件。

1. **创建命名空间和相关资源**：

    ```bash
    # 设置API密钥环境变量（用于配置文件渲染）
    export OPENAI_API_KEY="your_openai_api_key"
    export ANTHROPIC_API_KEY="your_anthropic_api_key"

    # 进入kubernetes配置目录
    cd examples/config/kubernetes

    # 依次应用部署资源
    kubectl apply -f namespace.yaml
    kubectl apply -f configmap.yaml
    kubectl apply -f service.yaml
    kubectl apply -f deployment.yaml
    ```

2. **验证部署状态**：

    ```bash
    kubectl get pods -n llmproxy
    kubectl get services -n llmproxy
    ```

3. **访问服务**：

    集群内部访问（使用服务名称）：

    ```
    http://llmproxy.llmproxy.svc.cluster.local:3000
    ```

    从集群外部访问（可配置 Ingress 或使用端口转发）：

    ```bash
    kubectl port-forward svc/llmproxy -n llmproxy 3000:3000 3001:3001 9000:9000
    ```

### Linux 系统服务部署

对于传统的 Linux 服务器环境，我们提供了标准的 systemd 服务文件。

1. **下载并安装二进制文件**：

    ```bash
    # 下载最新版本
    curl -L -o llmproxyd.zip https://github.com/shengyanli1982/llmproxy/releases/latest/download/llmproxyd-Linux-x64-<version>.zip

    # 解压文件
    unzip llmproxyd.zip

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
    # 在编辑器中粘贴您的配置内容
    ```

3. **创建专用系统用户**：

    ```bash
    sudo useradd -r -s /bin/false llmproxy
    sudo chown -R llmproxy:llmproxy /opt/llmproxy
    ```

4. **安装 systemd 服务文件**：

    ```bash
    sudo cp examples/config/llmproxy.service /etc/systemd/system/
    sudo systemctl daemon-reload
    ```

5. **启动并设置自动启动**：

    ```bash
    sudo systemctl start llmproxy
    sudo systemctl enable llmproxy
    ```

6. **检查服务状态**：

    ```bash
    sudo systemctl status llmproxy
    ```

### 安全最佳实践

无论选择何种部署方式，都应考虑以下安全最佳实践：

1. **API 密钥保护**：

    - 避免在配置文件中直接硬编码 API 密钥
    - 优先使用环境变量、专业密钥管理服务或 Kubernetes Secrets
    - 定期轮换 API 密钥，降低潜在泄露风险

2. **网络安全加固**：

    - 严格限制管理接口（端口 9000）的访问范围，仅对可信内部网络开放
    - 考虑在前端部署反向代理（如 Nginx、Traefik），增加额外的身份验证和 TLS 加密层
    - 实施网络分段，确保不同安全级别的组件隔离部署

3. **最小权限原则**：

    - 使用专用的非特权系统用户运行服务
    - 严格限制服务对文件系统的访问权限
    - 采用容器安全最佳实践，如非 root 用户运行、只读文件系统等

4. **监控与日志管理**：
    - 配置集中式日志聚合和分析系统
    - 设置基于 Prometheus 指标的实时监控和告警机制
    - 实施异常检测，及时发现潜在的安全威胁或性能问题

## 许可证

[MIT 许可证](LICENSE)
