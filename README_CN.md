[English](./README.md) | 中文

<div align="center">
    <img src="./images/logo.png" alt="logo" width="650">
</div>

**LLMProxy: 专为大语言模型打造的企业级智能代理与负载均衡器。统一管理、调度各类 LLM 服务（公有云 API、私有化部署的 vLLM/Ollama 等），实现多云/混合云架构下的高效、稳定、可扩展的 LLM 应用访问，并最大限度减少客户端代码改动。**

<p align="center">
  <a href="#简介">简介</a>
  |
  <a href="#快速开始">快速开始</a>
  |
  <a href="#核心功能">核心功能</a>
  |
  <a href="#应用场景">应用场景</a>
  |
  <a href="#配置指南">配置指南</a>
  |
  <a href="#部署进阶">部署进阶</a>
  |
  <a href="#深入理解-llmproxy">深入理解 LLMProxy</a>
  |
  <a href="#api-端点">API 端点</a>
  |
  <a href="#prometheus-指标">Prometheus 指标</a>
  |
  <a href="#许可证">许可证</a>
</p>

## 简介

**LLMProxy** 是一款专为大语言模型 (LLM) API 设计的企业级高可用智能代理与负载均衡解决方案。它作为统一入口，接收客户端请求，通过灵活的路由策略与针对 LLM 优化的负载均衡算法，将请求高效分发至各类上游 LLM 服务（如 OpenAI, Anthropic, 或私有化部署的 vLLM, Ollama 等），并将响应安全返回。LLMProxy 旨在解决直接调用 LLM API 时面临的性能、成本、可用性及管理复杂性等挑战，通过精细化的流量调度、连接管理和容错机制，显著提升 LLM 应用的稳定性、响应速度和资源利用效率。

### 为什么选择 LLMProxy？

LLMProxy 有效解决了企业级 LLM API 部署中的关键挑战：

-   **统一 LLM 访问与高可用**：聚合多个 LLM 服务（不同云厂商、私有模型如 vLLM/Ollama），通过智能路由与故障转移，消除单点故障，确保业务连续性。
-   **LLM 优化型负载均衡**：内置多种策略（如轮询、加权、响应时间感知），特别针对 LLM 长连接、流式响应等特性优化，动态分配请求至最佳服务节点，平衡成本与性能。
-   **强大的容错与弹性**：集成断路器模式，自动隔离故障上游，防止雪崩效应；支持请求重试，提升复杂网络环境下 LLM 调用的成功率。
-   **轻松扩展与成本控制**：按需增减上游 LLM 服务，无缝扩展处理能力；通过负载均衡策略优先使用低成本或高性能资源，优化 LLM 调用开销。
-   **简化集成与管理**：提供统一 API 入口，屏蔽后端 LLM 服务差异，简化客户端集成；通过配置文件集中管理路由、认证、安全策略。
-   **增强的可观测性**：提供详细的 Prometheus 指标，助力实时监控 LLM 服务调用情况、代理性能及故障诊断。

## 快速开始

### 1. 直接运行应用程序

这是体验 LLMProxy 最快的方式，无需复杂的环境配置。

**步骤 1：下载预编译的二进制文件**

请前往项目的 [GitHub Releases](https://github.com/shengyanli1982/llmproxy/releases) 页面，下载适用于你操作系统（如 Windows, Linux, macOS）的最新预编译二进制文件压缩包 (例如 `llmproxyd-Linux-x64-<version>.zip` 或 `llmproxyd-Windows-x64-<version>.zip`)。

下载后，请解压缩文件，你会得到一个名为 `llmproxyd-<os>-<arch>` (Linux/macOS) 或 `llmproxyd-windows-x64.exe` (Windows) 的可执行文件。

**步骤 2：创建配置文件**

在与可执行文件相同的目录下，创建一个名为 `config.yaml` 的配置文件。LLMProxy 专为代理大语言模型设计，以下是一个最小化的配置示例，用于将本地 `3000` 端口的请求转发到 OpenAI API。请注意，你需要将 `YOUR_OPENAI_API_KEY_HERE` 替换为你的真实 OpenAI API 密钥才能使其正常工作：

```yaml
http_server:
    forwards:
        - name: "llm_openai_service"      # 转发服务名称
          port: 3000                      # LLMProxy 监听此端口
          address: "0.0.0.0"              # 监听所有网络接口
          default_group: "openai_main_group" # 关联到下面的上游组
    admin:
        port: 9000                      # 管理端口，用于监控等
        address: "127.0.0.1"            # 建议仅本地访问，确保安全

upstreams:
    - name: "openai_chat_api"           # 上游服务名称，例如 OpenAI
      url: "https://api.openai.com/v1"  # OpenAI API 的基础 URL
      auth:
          type: "bearer"                # 认证类型为 Bearer Token
          token: "YOUR_OPENAI_API_KEY_HERE" # !!重要!! 请务必替换为你的真实 OpenAI API 密钥
                                      # 如果没有 OpenAI 密钥，你可以选择其他 LLM 服务或使用模拟服务进行测试

upstream_groups:
    - name: "openai_main_group"         # 上游组名称
      upstreams:
          - name: "openai_chat_api"       # 引用上面定义的 openai_chat_api 上游
    # [可选] 为 LLM 请求配置更长的超时时间
    http_client:
      timeout:
        request: 300 # LLM 请求通常需要较长时间，建议设置 300 秒或以上
```

此配置定义了一个监听 `3000` 端口的转发服务，它会将请求路由到名为 `openai_main_group` 的上游组。该上游组配置了 OpenAI API 作为其后端服务，并设置了合适的请求超时时间。关于更详细和高级的配置选项，请参考项目根目录下的 `config.default.yaml` 文件。

**步骤 3：运行 LLMProxy**

打开终端或命令提示符，切换到包含 `llmproxyd-<os>-<arch>` (或 `llmproxyd-windows-x64.exe`) 和 `config.yaml` 的目录，然后执行以下命令：

-   对于 Linux/macOS:
    ```bash
    mv llmproxyd-<os>-<arch> llmproxyd
    chmod +x llmproxyd # 首次运行时可能需要添加执行权限
    ./llmproxyd --config config.yaml
    ```
-   对于 Windows:
    ```bash
    .\llmproxyd-windows-x64.exe --config config.yaml
    ```

如果一切顺利，你会看到 LLMProxy 启动并开始监听配置的端口。

**步骤 4：测试代理服务**

打开另一个终端，使用 `curl` 或类似的工具向 LLMProxy 配置的转发端口发送一个 LLM API 请求。例如，如果你的 `config.yaml` 中 `llm_openai_service` 服务监听的是 `3000` 端口，并且上游是 OpenAI API，你可以尝试发送一个聊天请求 (请确保你的请求体符合 OpenAI API 格式，并根据需要替换请求体中的 API Key)：

```bash
curl http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_CLIENT_SIDE_OPENAI_API_KEY" \ # 注意：此处的Key通常由客户端应用提供并由LLMProxy转发。LLMProxy自身在upstreams中也配置了服务端的Key。
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{"role": "user", "content": "你好，LLMProxy！请介绍一下你自己。"}],
    "stream": false
  }'
```

此请求将被 LLMProxy 接收，并根据配置（包括认证、负载均衡等）转发到你在 `config.yaml` 中定义的 OpenAI 上游服务 (`https://api.openai.com/v1`)。你应该能看到来自 OpenAI API 的 JSON 响应。如果上游服务支持流式响应并且你在请求中启用了流式传输（例如 `"stream": true`），LLMProxy 也会正确处理流式数据。

现在你已经成功运行了 LLMProxy！接下来，你可以探索更高级的配置和功能。

### 2. 使用 Docker 部署 (推荐)

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
        image: shengyanli1982/llmproxy:latest # 建议指定更具体的版本号
        container_name: llmproxy
        restart: unless-stopped
        ports:
            # 转发服务端口映射 (根据你的 config.yaml 进行调整)
            - "3000:3000" # 例如：llm_openai_service
            # - "3001:3001" # 其他转发服务
            # 管理界面端口映射
            - "9000:9000" # admin
        volumes:
            - ./config.yaml:/app/config.yaml:ro # 挂载你的配置文件
        command: ["--config", "/app/config.yaml"]
        environment:
            - TZ=Asia/Shanghai # 根据需要设置时区
        networks:
            - llmproxy-network

networks:
    llmproxy-network:
        driver: bridge
```

## 核心功能

-   🔄 **智能 LLM 路由与请求处理**

    -   通过`http_server.forwards`为不同业务场景或 LLM 模型配置独立的转发服务，实现精细化管理。
    -   对每个转发服务定制专属监听地址和端口。
    -   基于路径、头部或其他请求特征，将请求灵活路由至指定的上游 LLM 服务组。

-   🌐 **统一上游 LLM 服务管理**

    -   通过`upstreams`集中定义和管理各类上游 LLM 服务（公有云 API、私有化模型服务如 vLLM/Ollama 等）。
    -   对每个上游 LLM 服务进行独立命名、URL 配置与健康管理。
    -   内置多种认证代理机制（Bearer Token, API Key Header Injection, Basic Auth），安全对接不同类型的 LLM 服务。
    -   灵活的 HTTP 头部操作（增、删、改），适配不同 LLM API 的特殊要求或注入追踪信息。

-   ⚡ **LLM 优化型负载均衡**

    -   使用`upstream_groups`将功能相似或可互为备份的 LLM 服务组织为上游组，实现统一调度与高可用。
    -   提供多种专为 LLM 优化的负载均衡策略：
        -   **轮询（Round Robin）** - 在各上游 LLM 服务间平均分配请求。
        -   **加权轮询（Weighted Round Robin）** - 根据预设权重（如服务处理能力、成本考量）分配请求至不同 LLM 服务。
        -   **随机（Random）** - 随机选择一个可用的 LLM 服务。
        -   **响应时间感知（Response Aware）** - 尤其适用于 LLM 服务，实时监测各节点性能（响应延迟、并发负载、成功率），动态将请求导向当前最优节点，最大化吞吐量与用户体验。
        -   **故障转移（Failover）** - 按照上游列表的顺序尝试，如果当前的上游不可用，则自动切换到下一个，提供顺序备份能力。
    -   在加权轮询策略中可为每个上游 LLM 服务设置权重。
    -   **动态负载均衡器更新** - 通过 API 调用在运行时动态更新任何负载均衡器的上游列表，允许无需服务中断或重启即可无缝添加、移除或修改上游服务。

-   🔁 **灵活的流量控制与 QoS 保障**

    -   为每个转发服务配置基于 IP 或其他标识符的速率限制（请求数/秒、并发请求峰值）。
    -   保护后端 LLM 服务免受恶意攻击或流量冲击，保障核心业务的服务质量 (QoS)。

-   🔌 **针对 LLM 优化的连接管理**

    -   **入站连接管理：** 为客户端连接配置精确的连接超时。
    -   **出站连接与请求优化 (针对上游 LLM 服务)：**
        -   自定义 User-Agent，便于上游服务识别与统计。
        -   TCP Keepalive，维持与上游 LLM 服务的长效连接，降低握手延迟，尤其利好流式响应。
        -   精细的超时控制（连接超时、请求超时、空闲超时），适配 LLM 服务多样化的响应时间特性（从秒级到分钟级）。
        -   针对 LLM API 可能出现的瞬时错误，配置智能重试策略（可配置尝试次数、初始退避间隔及指数退避）。
        -   支持通过出站 HTTP/HTTPS 代理连接上游 LLM 服务，满足企业网络安全与合规要求。
        -   **原生流式支持与超时控制**: 通过 `http_client.stream` 设置原生处理 LLM 流式响应 (Server-Sent Events)。当启用时（默认），会禁用请求超时，这对于长连接的流式响应至关重要，可避免其被过早中断。禁用时，则会强制执行固定的请求超时，适用于非流式 API 调用。

-   🛡️ **健壮的容错与故障转移**

    -   **智能断路器：** 自动监测上游 LLM 服务的健康状态（基于错误率），达到阈值时快速隔离故障节点。
    -   **可配置的熔断策略：** 为每个上游 LLM 服务自定义熔断阈值（如失败率）和冷却时间（熔断后进入半开状态的等待时间）。
    -   **自动恢复与探测：** 熔断后定期尝试发送探测请求至故障节点，一旦服务恢复则自动将其重新纳入负载均衡池。
    -   **无缝故障转移：** 上游组内某个 LLM 服务故障或熔断时，自动将流量平滑切换至组内其他健康节点，对客户端透明，保障业务连续性。

-   📊 **可观测性与管理接口**
    -   通过`http_server.admin`提供独立的管理界面和 API 端点。
    -   提供 `/health` 健康检查端点，便于集成到各类监控和自动化运维体系。
    -   通过 `/metrics` 端点暴露丰富的 Prometheus 指标，全面洞察 LLM 代理性能、流量、错误、延迟、上游 LLM 服务健康状况及断路器状态等。

## 应用场景

LLMProxy 专为需要高效、可靠、可扩展地访问和管理大语言模型 API 的各类企业级应用场景而设计：

-   **企业级 AI 应用网关**：

    -   为企业内部多个应用或团队提供统一的 LLM API 访问入口。
    -   集中实施对大模型 API 的访问认证、API 密钥设置。

-   **多租户与服务隔离**：

    -   通过为不同团队、应用或客户（租户）配置独立的`forwards`和`upstream_groups`，在单个 LLMProxy 实例中实现多租户架构。
    -   为每个租户分配唯一的访问端点（端口），并应用独立的路由规则、API 密钥、速率限制和负载均衡策略。
    -   这对于需要为不同客户提供定制化 LLM 服务的 SaaS 平台，或在企业内部为不同部门隔离资源和计费的场景尤为适用。

-   **高可用、高并发 LLM 服务**：

    -   构建面向最终用户的高流量 AI 产品（如智能客服、内容生成工具、AI 助手）。
    -   通过跨多个 LLM 提供商（如 OpenAI, Anthropic, Azure OpenAI, Google Gemini）或自建模型（vLLM, Ollama）的实例进行负载均衡和故障转移，确保服务不中断。
    -   利用响应时间感知等高级策略，动态分配流量至性能最佳的节点，提升用户体验。

-   **LLM 应用开发与测试加速**：

    -   简化开发人员与多种 LLM API 的集成复杂度，使应用代码与具体 LLM 服务解耦。
    -   在不同 LLM 模型或提供商之间轻松切换，方便进行效果评估和成本比较。
    -   为测试环境模拟不同的上游响应（如延迟、错误），或隔离测试流量。

-   **多云/混合云 LLM 策略实施**：

    -   在复杂的云环境中（如 AWS, Azure, GCP 及本地数据中心混合部署）提供统一的 LLM API 访问层。
    -   根据数据主权、合规性要求或成本因素，将请求路由到特定地理位置或特定类型的 LLM 服务。
    -   作为独立服务部署在 Kubernetes 等容器编排平台中，为微服务提供 LLM 访问能力。

-   **API 版本与兼容性管理**：
    -   当后端 LLM API 升级或发生不兼容变更时，LLMProxy 可作为适配层，通过头部操作或轻量级转换（未来可能支持）保持对旧版客户端的兼容。

通过在这些场景中应用 LLMProxy，企业可以显著提升其 LLM 应用的可靠性、性能和可管理性，同时降低集成和运营的复杂性。

## 配置指南

LLMProxy 采用结构化 YAML 文件进行配置，提供灵活且强大的配置选项。以下是关键配置部分的详细说明：

### 配置选项详解

#### HTTP 服务器配置选项

| 配置项                                          | 类型   | 默认值    | 说明                                                               |
| ----------------------------------------------- | ------ | --------- | ------------------------------------------------------------------ |
| `http_server.forwards[].name`                   | 字符串 | -         | **[必填]** 转发服务的唯一标识名称                                  |
| `http_server.forwards[].port`                   | 整数   | 3000      | **[必填]** 转发服务的监听端口                                      |
| `http_server.forwards[].address`                | 字符串 | "0.0.0.0" | 转发服务的绑定网络地址                                             |
| `http_server.forwards[].default_group`          | 字符串 | -         | **[必填]** 当没有路由规则匹配时使用的默认上游组名称                |
| `http_server.forwards[].routing`                | 数组   | null      | **[可选]** 高级路由规则配置。如果省略，则不启用路由规则            |
| `http_server.forwards[].routing[].path`         | 字符串 | -         | **[必填]** 此路由规则的路径模式                                    |
| `http_server.forwards[].routing[].target_group` | 字符串 | -         | **[必填]** 此路由对应的上游组名称，必须在`upstream_groups`部分定义 |
| `http_server.forwards[].ratelimit`              | 对象   | null      | **[可选]** 速率限制配置。如果省略，则不启用速率限制                |
| `http_server.forwards[].ratelimit.per_second`   | 整数   | 100       | 单个 IP 每秒允许的最大请求数（取值范围：1-10000）                  |
| `http_server.forwards[].ratelimit.burst`        | 整数   | 200       | 单个 IP 允许的突发请求数（缓冲区大小）（取值范围：1-20000）        |
| `http_server.forwards[].timeout`                | 对象   | null      | **[可选]** 连接超时配置。如果省略，将使用默认值                    |
| `http_server.forwards[].timeout.connect`        | 整数   | 10        | 客户端连接到 LLMProxy 的超时时间（秒）                             |
| `http_server.admin.port`                        | 整数   | 9000      | 可选的管理服务监听端口                                             |
| `http_server.admin.address`                     | 字符串 | "0.0.0.0" | 管理服务的绑定网络地址                                             |
| `http_server.admin.timeout`                     | 对象   | null      | **[可选]** 连接超时配置。如果省略，将使用默认值                    |
| `http_server.admin.timeout.connect`             | 整数   | 10        | 连接到管理接口的超时时间（秒）                                     |

#### 上游服务配置选项 (Upstream LLM Services)

| 配置项                          | 类型   | 默认值 | 说明                                                                                   |
| ------------------------------- | ------ | ------ | -------------------------------------------------------------------------------------- |
| `upstreams[].name`              | 字符串 | -      | **[必填]** 上游 LLM 服务的唯一标识名称                                                 |
| `upstreams[].url`               | 字符串 | -      | **[必填]** 上游 LLM 服务的完整 URL (例如 `https://api.openai.com/v1/chat/completions`) |
| `upstreams[].auth.type`         | 字符串 | "none" | 认证类型：`bearer`、`basic`或`none`                                                    |
| `upstreams[].auth.token`        | 字符串 | -      | 当`type`为`bearer`时的 API 密钥或令牌                                                  |
| `upstreams[].auth.username`     | 字符串 | -      | 当`type`为`basic`时的用户名                                                            |
| `upstreams[].auth.password`     | 字符串 | -      | 当`type`为`basic`时的密码                                                              |
| `upstreams[].headers[].op`      | 字符串 | -      | HTTP 头部操作类型：`insert` (不存在则添加)、`replace` (替换或添加)、`remove`           |
| `upstreams[].headers[].key`     | 字符串 | -      | 要操作的 HTTP 头部名称                                                                 |
| `upstreams[].headers[].value`   | 字符串 | -      | 用于`insert`或`replace`操作的头部值                                                    |
| `upstreams[].breaker.threshold` | 浮点数 | 0.5    | 熔断器触发阈值，表示失败率（0.01-1.0），如 0.5 代表 50% 失败则熔断                     |
| `upstreams[].breaker.cooldown`  | 整数   | 30     | 熔断器冷却时间（秒），即熔断后多久尝试进入半开状态 (1-3600)                            |

#### 上游组配置选项 (Upstream LLM Groups)

> [!NOTE]
>
> 参数 `upstreams[].url` 需要配置上游服务的完整 URL，例如：`https://api.openai.com/v1/chat/completions`， 而不是 `https://api.openai.com/v1` 或者 `https://api.openai.com`。

| 配置项                                          | 类型   | 默认值         | 说明                                                                                                                                                                       |
| ----------------------------------------------- | ------ | -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `upstream_groups[].name`                        | 字符串 | -              | **[必填]** 上游组的唯一标识名称                                                                                                                                            |
| `upstream_groups[].upstreams[].name`            | 字符串 | -              | **[必填]** 引用的上游 LLM 服务名称，必须在`upstreams`部分已定义                                                                                                            |
| `upstream_groups[].upstreams[].weight`          | 整数   | 1              | 仅在`balance.strategy`为`weighted_roundrobin`时有效的权重值，用于按比例分配请求（取值范围：1-65535）                                                                       |
| `upstream_groups[].balance.strategy`            | 字符串 | "roundrobin"   | 负载均衡策略：`roundrobin`、`weighted_roundrobin`、`random`、`response_aware`或`failover`                                                                                  |
| `upstream_groups[].http_client.agent`           | 字符串 | "LLMProxy/1.0" | 发送到上游 LLM 服务的 User-Agent 头部值                                                                                                                                    |
| `upstream_groups[].http_client.keepalive`       | 整数   | 30             | TCP Keepalive 时间（秒），取值范围 5-600，不允许为 0。有助于保持与上游 LLM 服务的连接活跃，减少延迟                                                                        |
| `upstream_groups[].http_client.stream`          | 布尔值 | true           | 控制请求超时行为。若为 `true` (默认值)，则禁用请求超时，这对于 LLM 流式响应 (Server-Sent Events) **至关重要**。若为 `false`，则 `timeout.request` 生效，适用于非流式调用。 |
| `upstream_groups[].http_client.timeout`         | 对象   | null           | **[可选]** 连接和请求超时配置。如果省略，将使用默认值                                                                                                                      |
| `upstream_groups[].http_client.timeout.connect` | 整数   | 10             | 连接到上游 LLM 服务的超时时间（秒）（取值范围：1-120）                                                                                                                     |
| `upstream_groups[].http_client.timeout.request` | 整数   | 300            | 非流式请求的请求超时时间（秒）。仅在 `http_client.stream` 为 `false` 时生效。定义了等待上游完整响应的最长时间。（取值范围：1-1200）                                        |
| `upstream_groups[].http_client.timeout.idle`    | 整数   | 60             | 与上游 LLM 服务的连接在无活动后被视为空闲并关闭的超时时间（秒）（取值范围：5-1800）                                                                                        |
| `upstream_groups[].http_client.retry`           | 对象   | null           | **[可选]** 请求重试配置。如果省略，则不启用重试功能                                                                                                                        |
| `upstream_groups[].http_client.retry.attempts`  | 整数   | 3              | 最大重试次数（不包括首次尝试）（取值范围：1-100）                                                                                                                          |
| `upstream_groups[].http_client.retry.initial`   | 整数   | 500            | 首次重试前的初始等待时间（毫秒），后续重试间隔可能采用指数退避策略（取值范围：100-10000）                                                                                  |
| `upstream_groups[].http_client.proxy`           | 对象   | null           | **[可选]** 出站代理配置。如果省略，则不使用代理                                                                                                                            |
| `upstream_groups[].http_client.proxy.url`       | 字符串 | -              | 出站代理服务器 URL (例如 `http://user:pass@proxy.example.com:8080`)                                                                                                        |

### HTTP 服务器配置

```yaml
http_server:
    # 转发服务配置 (处理客户端入站请求)
    forwards:
        - name: "to_openai_llm_group" # [必需] 转发服务的唯一名称
          port: 3000 # [必需] 此服务监听的端口
          address: "0.0.0.0" # [可选] 绑定的网络地址（默认："0.0.0.0"，监听所有接口）
          default_group: "openai_llm_group" # [必需] 未匹配任何路由规则时的默认上游组
          # [可选] 高级路径路由规则
          routing:
              # 静态路径规则 - 优先级最高的匹配
              - path: "/api/specific-endpoint"
                target_group: "specialized_llm_group"
              # 命名参数规则
              - path: "/api/users/:id/history"
                target_group: "user_history_group"
              # 正则表达式规则 - 仅匹配数字ID
              - path: "/api/items/{id:[0-9]+}"
                target_group: "item_api_group"
          ratelimit: # [可选] IP 速率限制
              enabled: true # 是否启用速率限制（默认：false）
              per_second: 100 # 单个IP每秒最大请求数
              burst: 200 # 单个IP的突发请求容量
          timeout: # [可选] 客户端连接超时
              connect: 10 # 客户端连接到 LLMProxy 的超时时间（秒）

    # 管理界面配置
    admin:
        port: 9000 # [可选] 管理界面端口 (用于 /metrics, /health)
        address: "127.0.0.1" # [可选] 绑定的网络地址（默认："0.0.0.0", 建议生产环境设为 "127.0.0.1"）
        timeout:
            connect: 10 # 连接到管理接口的超时时间（秒）
```

### 高级路径路由

LLMProxy 支持在每个转发服务中配置复杂的基于路径的路由，允许根据请求路径将请求转发到不同的上游组。这一功能使得为 LLM 服务创建复杂的路由拓扑和服务网格成为可能。

#### 路由规则配置

路由规则在每个转发服务配置的 `routing` 数组下定义。当请求到达时，LLMProxy 按顺序评估所有配置的路由规则，并选择第一个匹配的规则。如果没有规则匹配，请求将被发送到 `default_group`。

```yaml
forwards:
    - name: "advanced_routing_service"
      port: 3000
      address: "0.0.0.0"
      default_group: "fallback_group" # 当没有路由规则匹配时使用
      routing:
          - path: "/path/to/match"
            target_group: "target_upstream_group"
          # 其他规则...
```

#### 路径匹配模式

LLMProxy 支持多种路径匹配模式，从简单的静态路径到复杂的基于正则表达式的模式：

1. **静态路径**：精确路径匹配，具有最高优先级。

    ```yaml
    - path: "/api/users/admin"
      target_group: "admin_api_group"
    ```

2. **命名参数**：使用 `:param_name` 语法匹配可变路径段。

    ```yaml
    - path: "/api/users/:id"
      target_group: "user_api_group" # 匹配 "/api/users/123", "/api/users/abc" 等
    ```

3. **正则约束参数**：使用 `{param:regex}` 语法匹配具有正则约束的路径段。

    ```yaml
    - path: "/api/items/{id:[0-9]+}"
      target_group: "item_api_group" # 匹配 "/api/items/42"，但不匹配 "/api/items/abc"
    ```

4. **复杂正则模式**：使用更复杂的正则表达式匹配特定格式。

    ```yaml
    - path: "/api/products/{code:[A-Z][A-Z][A-Z][0-9][0-9][0-9]}"
      target_group: "product_api_group" # 匹配 "/api/products/ABC123"
    ```

5. **通配符**：使用 `*` 匹配单个路径段或尾部内容。

    ```yaml
    - path: "/api/*/docs"
      target_group: "api_docs_group" # 匹配 "/api/v1/docs", "/api/v2/docs"

    - path: "/files/*"
      target_group: "file_server_group" # 匹配任何以 "/files/" 开头的路径
    ```

6. **混合模式**：结合各种模式满足复杂路由需求。
    ```yaml
    - path: "/api/:version/users/{id:[0-9]+}/profile"
      target_group: "user_profile_group" # 匹配 "/api/v2/users/42/profile"
    ```

#### 路由匹配优先级

当多条规则可能匹配同一路径时，`LLMProxy` 遵循以下优先级规则：

1. 静态路径始终具有最高优先级
2. 含有更多静态段的参数化路径优先级更高
3. 有正则约束的参数优先级高于无约束参数
4. 更长的路径模式优先级高于更短的路径模式
5. 按配置中定义的顺序评估规则

#### 高级路由的应用场景

-   **特定模型路由**：将不同模型的请求导向专门的上游组。

    ```yaml
    - path: "/v1/chat/completions" # 默认聊天补全
      target_group: "standard_models_group"
    - path: "/v1/embeddings"
      target_group: "embedding_models_group"
    ```

-   **基于版本的路由**：支持 API 版本管理或 A/B 测试。

    ```yaml
    - path: "/api/v1/*"
      target_group: "api_v1_group"
    - path: "/api/v2/*"
      target_group: "api_v2_group"
    ```

-   **基于资源的路由**：将不同类型的资源路由到专门的服务。
    ```yaml
    - path: "/api/images/*"
      target_group: "image_generation_group"
    - path: "/api/text/*"
      target_group: "text_generation_group"
    ```

### 上游服务配置 (定义后端 LLM API 服务)

```yaml
upstreams:
    - name: "openai_gpt4_primary" # [必需] 上游 LLM 服务的唯一标识名称
      url: "https://api.openai.com/v1" # [必需] 上游 LLM API 的基础URL
      auth: # [可选] 认证配置
          type: "bearer" # 认证类型："bearer"、"basic"或"none"（默认）
          token: "YOUR_OPENAI_API_KEY" # [bearer认证必需] API密钥/令牌
          # username: "user"         # [basic认证必需] 用户名
          # password: "pass"         # [basic认证必需] 密码
      headers: # [可选] HTTP 头部操作 (在请求转发到此上游前修改)
          - op: "insert" # 操作类型："insert"、"replace"或"remove"
            key: "X-Custom-Proxy-Header" # 要操作的HTTP头部名称
            value: "LLMProxy-OpenAI-GPT4" # 头部值（用于"insert"或"replace"操作）
      breaker: # [可选] 此上游的断路器配置
          threshold: 0.5 # 触发断路器的故障率阈值（0.01-1.0，默认：0.5）
          cooldown: 30 # 进入半开状态前的冷却期（秒）（1-3600，默认：30）

    - name: "anthropic_claude_haiku"
      url: "https://api.anthropic.com" # 示例：Anthropic API
      auth:
          type: "bearer" # Anthropic 通常也使用 Bearer Token
          token: "YOUR_ANTHROPIC_API_KEY"
      headers: # Anthropic 可能需要在header中指定 anthropic-version
          - op: "insert"
            key: "anthropic-version"
            value: "2023-06-01"
          - op: "insert" # anthropic-beta: messages-2023-12-15, max-tokens-3-5-sonnet-2024-07-15 等
            key: "anthropic-beta"
            value: "max-tokens-3-5-sonnet-2024-07-15"
      breaker:
          threshold: 0.4
          cooldown: 45
```

### 上游组配置 (组织上游并定义负载均衡等行为)

```yaml
upstream_groups:
    - name: "openai_llm_group" # [必需] 上游组的唯一标识名称
      upstreams: # [必需] 此组包含的上游 LLM 服务列表 (至少一个)
          - name: "openai_gpt4_primary" # 引用在 `upstreams` 部分定义的 `name`
            weight: 8 # [可选] 权重，仅在 `balance.strategy` 为 "weighted_roundrobin" 时有效 (默认: 1)
          # - name: "another_openai_backup_service"
          #   weight: 2
      balance:
          strategy:
              "weighted_roundrobin" # 负载均衡策略：
              # "roundrobin"（默认轮询）、
              # "weighted_roundrobin"（加权轮询）、
              # "random"（随机）、
              # "response_aware"（响应时间感知，推荐用于LLM）、
              # "failover"（故障转移，按上游列表顺序尝试）
      http_client: # [可选] 定义 LLMProxy 如何与此组中的上游 LLM 服务通信
          agent: "LLMProxy/1.0 (OpenAI-Group)" # [可选] 自定义 User-Agent 头部
          keepalive: 90 # [可选] TCP保活时间（秒）（0-600，0=禁用，默认：60）
          stream: true # [可选] 启用流式传输模式 (对LLM流式响应重要, 默认：true）
          timeout:
              connect: 15 # 连接到上游 LLM 服务的超时时间（秒）（默认：10）
              request: 360 # 请求超时（秒）（默认：300, 对于耗时长的LLM可能需要更大）
              idle: 90 # 空闲连接超时（秒）（默认：60）
          retry: # [可选] 请求重试配置
              attempts: 3 # 最大重试次数
              initial: 1000 # 首次重试前的初始等待时间（毫秒）
          proxy: # [可选] 出站代理配置
              url: "http://user:pass@your-proxy-server.com:8080" # 代理服务器URL
```

### 配置最佳实践

1. **安全建议**：

    - 严格限制管理接口（`admin`服务）的访问。将其绑定到本地回环地址（`address: "127.0.0.1"`），并考虑使用防火墙规则或反向代理（如 Nginx）为其添加额外的认证和访问控制。

2. **性能与成本优化**：

    - 针对不同 LLM 服务提供商的 API 特性（如响应时间、并发限制、计费模型），精细调整各项超时配置（`timeout.connect`, `timeout.request`, `timeout.idle`）和重试策略（`retry`）。
    - 对于支持流式响应的 LLM 服务，务必确保 `http_client.stream: true`（此为默认值），以最低延迟接收和转发数据。
    - 合理配置速率限制（`ratelimit`），既要保护后端 LLM 服务免遭过载，也要满足业务高峰期的需求。
    - 利用 `weighted_roundrobin` 负载均衡策略，结合不同 LLM 服务的成本和性能，将更多流量导向性价比高的服务。
    - 对于延迟敏感型应用，优先考虑使用 `response_aware` 负载均衡策略，它能动态选择当前表现最佳的上游服务。

3. **可靠性与弹性设计**:
    - 为每个上游 LLM 服务（`upstreams`）配置合理的断路器参数（`breaker.threshold`, `breaker.cooldown`）。阈值设置需权衡故障检测的灵敏度和服务自身的波动性。
    - 在每个上游组（`upstream_groups`）中配置多个上游 LLM 服务实例（可以是同一提供商的不同区域节点，或不同提供商的可替代服务），以实现冗余和自动故障转移。
    - 仅对幂等或可安全重试的 LLM API 调用启用请求重试（通过配置 `http_client.retry` 对象）。注意某些 LLM 操作（如生成内容）可能不是幂等的。
    - 定期监控 Prometheus 指标中关于断路器状态、上游错误率、请求延迟等数据，据此优化配置和排查潜在问题。

有关所有可用配置选项的详细说明，请参阅 LLMProxy 项目附带的`config.default.yaml`文件作为完整参考。

### 示例: 多租户配置

LLMProxy 通过将不同的`forwards`（监听不同端口）映射到不同的`upstream_groups`，可以轻松实现多租户或服务隔离。每个`upstream_group`可以拥有自己独立的上游 LLM 服务、负载均衡策略和客户端行为配置。这使得单个 LLMProxy 实例能够为多个独立的客户端或应用提供服务，同时保持配置和流量的隔离。

以下示例展示了如何为两个租户（`tenant-a` 和 `tenant-b`）配置独立的代理服务：

-   `tenant-a` 在端口 `3001` 上访问，拥有专属的 OpenAI API 密钥和速率限制策略。
-   `tenant-b` 在端口 `3002` 上访问，使用不同的 API 密钥，并配置了故障转移（Failover）策略，同时有更严格的速率限制。

```yaml
http_server:
    forwards:
        - name: "tenant-a-service"
          port: 3001
          address: "0.0.0.0"
          default_group: "tenant-a-group"
          ratelimit:
              enabled: true
              per_second: 50 # 租户A的速率限制
              burst: 100
        - name: "tenant-b-service"
          port: 3002
          address: "0.0.0.0"
          default_group: "tenant-b-group"
          ratelimit:
              enabled: true
              per_second: 20 # 租户B的速率限制
              burst: 40

upstreams:
    - name: "openai_primary_for_a"
      url: "https://api.openai.com/v1"
      auth:
          type: "bearer"
          token: "TENANT_A_OPENAI_API_KEY" # 租户A的密钥
    - name: "openai_primary_for_b"
      url: "https://api.openai.com/v1"
      auth:
          type: "bearer"
          token: "TENANT_B_OPENAI_API_KEY" # 租户B的密钥
    - name: "openai_backup_for_b"
      url: "https://api.openai.com/v1"
      auth:
          type: "bearer"
          token: "TENANT_B_BACKUP_API_KEY" # 租户B的备用密钥

upstream_groups:
    # 租户A的配置组
    - name: "tenant-a-group"
      upstreams:
          - name: "openai_primary_for_a"
      balance:
          strategy: "roundrobin" # 简单轮询
      http_client:
          timeout:
              request: 300

    # 租户B的配置组
    - name: "tenant-b-group"
      upstreams:
          - name: "openai_primary_for_b" # 主要服务
          - name: "openai_backup_for_b" # 备用服务
      balance:
          strategy: "failover" # 故障时自动切换到备用
      http_client:
          timeout:
              request: 360
```

## 部署进阶

LLMProxy 支持多种灵活的部署方式，包括 Kubernetes 集群部署和传统 Linux 系统服务部署。以下是各部署方法的详细说明：

### Kubernetes 部署

对于 Kubernetes 环境，我们在`examples/config/kubernetes`目录中提供了推荐的部署配置文件（Deployment, Service, ConfigMap 等）。

1. **准备配置文件和密钥**：

    - 将你的`config.yaml`内容放入`examples/config/kubernetes/configmap.yaml`中，或通过其他方式创建 ConfigMap。
    - **强烈建议**：将 API 密钥等敏感信息存储在 Kubernetes Secrets 中，并通过环境变量或卷挂载的方式注入到 Pod 中，然后在 `config.yaml` 中引用这些环境变量。

2. **应用部署清单**：

    ```bash
    # (可选) 创建命名空间
    kubectl apply -f examples/config/kubernetes/namespace.yaml

    # 创建 ConfigMap (包含 LLMProxy 配置)
    kubectl apply -f examples/config/kubernetes/configmap.yaml -n llmproxy

    # (可选) 创建 Secrets (用于存放 API 密钥)
    # kubectl create secret generic llm-api-keys -n llmproxy \
    #   --from-literal=OPENAI_API_KEY='your_openai_key' \
    #   --from-literal=ANTHROPIC_API_KEY='your_anthropic_key'

    # 创建 Deployment
    kubectl apply -f examples/config/kubernetes/deployment.yaml -n llmproxy

    # 创建 Service (暴露 LLMProxy 服务)
    kubectl apply -f examples/config/kubernetes/service.yaml -n llmproxy
    ```

3. **验证部署状态**：

    ```bash
    kubectl get pods -n llmproxy -l app=llmproxy
    kubectl get services -n llmproxy llmproxy
    kubectl logs -n llmproxy -l app=llmproxy -f # 查看日志
    ```

4. **访问服务**：

    - **集群内部访问** (通过 Service 名称)：
      `http://llmproxy.llmproxy.svc.cluster.local:<port>` (port 根据 Service 定义)
    - **集群外部访问**：
      通常通过配置 Ingress 资源实现，或者使用 `LoadBalancer`类型的 Service (如果你的 K8s 环境支持)。进行端口转发测试：
        ```bash
        kubectl port-forward svc/llmproxy -n llmproxy 3000:3000 9000:9000 # 将服务端口转发到本地
        ```
        然后即可通过 `http://localhost:3000` 访问转发服务，`http://localhost:9000` 访问管理接口。

### Linux 系统服务部署 (使用 systemd)

对于传统的 Linux 服务器环境，可以使用 systemd 管理 LLMProxy 服务。

1. **下载并安装二进制文件**：
   前往 [GitHub Releases](https://github.com/shengyanli1982/llmproxy/releases) 下载最新的 `llmproxyd-Linux-x64-<version>.zip`。

    ```bash
    # 示例版本，请替换为最新版
    VERSION="0.1.0" # 假设这是你下载的版本
    curl -L -o llmproxyd-Linux-x64.zip https://github.com/shengyanli1982/llmproxy/releases/download/v${VERSION}/llmproxyd-Linux-x64-${VERSION}.zip
    unzip llmproxyd-Linux-x64.zip
    sudo mv llmproxyd-Linux-x64 /usr/local/bin/llmproxyd
    sudo chmod +x /usr/local/bin/llmproxyd
    ```

2. **创建配置文件目录和配置文件**：

    ```bash
    sudo mkdir -p /etc/llmproxy
    sudo nano /etc/llmproxy/config.yaml
    # 在编辑器中粘贴你的配置内容，并确保 API 密钥等敏感信息得到妥善处理
    # (例如，后续版本可能支持从环境变量读取)
    ```

3. **创建专用系统用户 (推荐)**：

    ```bash
    sudo useradd --system --no-create-home --shell /usr/sbin/nologin llmproxyuser
    sudo chown -R llmproxyuser:llmproxyuser /etc/llmproxy
    # 如果有日志目录，也应授权
    ```

4. **创建 systemd 服务文件**：
   将 `examples/config/llmproxy.service` 文件复制到 `/etc/systemd/system/llmproxy.service`。

    ```bash
    sudo cp examples/config/llmproxy.service /etc/systemd/system/llmproxy.service
    ```

    根据需要编辑 `/etc/systemd/system/llmproxy.service`，特别是 `User`、`Group` (如果创建了专用用户) 和 `ExecStart` 中的配置文件路径 (`--config /etc/llmproxy/config.yaml`)。

5. **重载 systemd，启动并设置开机自启**：

    ```bash
    sudo systemctl daemon-reload
    sudo systemctl start llmproxy
    sudo systemctl enable llmproxy
    ```

6. **检查服务状态和日志**：
    ```bash
    sudo systemctl status llmproxy
    sudo journalctl -u llmproxy -f # 查看实时日志
    ```

### 安全最佳实践 (所有部署方式通用)

1.  **API 密钥与凭证管理**：

    -   **绝对避免**在配置文件中明文硬编码 API 密钥。
    -   **容器化/Kubernetes**：优先使用 Secrets 管理凭证，并通过环境变量或文件挂载注入到 LLMProxy 容器中。
    -   **系统服务**：限制配置文件的读取权限，考虑使用外部配置源或环境变量（待 LLMProxy 支持）。
    -   定期轮换所有 API 密钥。

2.  **网络安全**：

    -   **管理接口安全**：将管理端口 (`admin.port`, 默认为 9000) 绑定到本地回环地址 (`127.0.0.1`) 或内部可信网络，切勿暴露到公网。考虑在其前端部署反向代理并添加认证。
    -   **TLS/SSL 加密**：对于面向公网的转发服务，建议在 LLMProxy 前端部署反向代理（如 Nginx, Traefik, Caddy 或云服务商的负载均衡器）来处理 TLS 终止和证书管理。LLMProxy 本身专注于代理逻辑。
    -   使用防火墙限制不必要的端口访问。

3.  **最小权限原则**：

    -   **容器**：以非 root 用户运行 LLMProxy 进程 (后续版本镜像会支持)。确保容器文件系统尽可能只读。
    -   **系统服务**：使用专用的、低权限的系统用户运行 LLMProxy 服务。

4.  **日志与监控**：
    -   配置日志收集方案，将 LLMProxy 的日志（包括访问日志和错误日志）聚合到集中的日志管理系统（如 ELK Stack, Grafana Loki, Splunk）。
    -   持续监控通过 `/metrics` 端点暴露的 Prometheus 指标，设置关键指标的告警（如错误率、延迟、断路器状态）。

## 深入理解 LLMProxy

### 架构

LLMProxy 采用模块化、高性能的异步架构设计，核心组件包括：

-   **HTTP 监听器 (Forward Servers)**：基于异步 HTTP 服务器，负责监听客户端请求，每个 `forwards` 配置项对应一个独立的监听实例。
-   **请求处理器与路由器**：解析入站请求，根据配置的路由规则将其导向指定的上游组。
-   **上游组管理器 (Upstream Groups)**：管理一组逻辑上相关的上游 LLM 服务，内含负载均衡器和 HTTP 客户端池。
-   **上游服务实例 (Upstreams)**：代表一个具体的后端 LLM API 端点，包含其 URL、认证信息、断路器配置等。
-   **负载均衡器 (Load Balancer)**：为每个上游组内嵌，根据所选策略（轮询、加权、随机、响应时间感知）在健康的可用上游服务间智能分配请求。
-   **HTTP 客户端 (HTTP Client)**：负责与上游 LLM 服务建立连接并发送请求，支持连接池、超时控制、重试、流式传输等。
-   **断路器 (Circuit Breaker)**：为每个上游服务实例配备，持续监控其健康状况，在检测到持续失败时自动熔断，防止故障扩散，并在服务恢复后自动重试。
-   **指标收集器 (Metrics Collector)**：基于 Prometheus 客户端，实时收集并暴露详细的性能和运营指标。
-   **配置管理器 (Config Manager)**：负责加载和解析 `config.yaml` 文件，并验证配置的有效性。

![architecture](./images/architecture.png)
_图：LLMProxy 核心架构示意图 (简化版)_

### Linux 上的暖重启

为提升服务可用性，LLMProxy 在 `Linux` 系统上为其转发和管理服务均启用了 `SO_REUSEPORT` 套接字选项。该特性允许多个 LLMProxy 实例监听同一端口，从而实现无缝的零停机重启与升级。当新进程启动时，它能立即在共享端口上开始接收新连接，而旧进程则在完成所有进行中的请求后优雅地关闭(**任然会存在非常少量的连接中断，但可以忽略不计**)。此机制可防止部署过程中的连接中断，并显著简化高可用性环境的配置。请注意，此功能为 `Linux` 平台独有，在 `Windows` 或 `macOS` 等其他操作系统上不受支持。

### 响应时间感知的负载均衡算法

LLMProxy 的响应时间感知（`response_aware`）负载均衡算法是专为大语言模型这类响应时间波动较大、计算密集型服务设计的智能调度策略。与传统的轮询或随机策略不同，该算法专为 LLM 这类响应时间波动较大的服务设计，通过实时分析上游节点的综合性能表现（结合平均响应时间、当前并发负载、请求成功率），动态地将新请求分配给当前最优的服务节点。

#### 工作原理

1.  **实时性能采样与平滑**：系统持续采集并记录每个上游 LLM 服务节点的关键性能指标：

    -   **平均响应时间**：采用指数加权移动平均算法计算，平滑短期波动，更好地反映近期性能趋势。
    -   **处理中的请求数**：节点当前正在处理但尚未完成的并发请求数量。
    -   **请求成功率**：近期请求成功完成的百分比。

2.  **动态健康度与综合评分**：结合断路器状态，仅考虑健康（未熔断）的节点。对于健康节点，根据以下类似公式计算其综合性能得分，得分越低表示节点性能越优：

    $$\text{Score} = \text{ResponseTime} \times (\text{ProcessingRequests} + 1) \times \frac{1}{\text{SuccessRate}}$$

    其中：

    -   $\text{ResponseTime}$ 是节点的平均响应时间（毫秒）
    -   $\text{ProcessingRequests}$ 是节点当前处理中的并发请求数
    -   $\text{SuccessRate}$ 是节点的请求成功率（0-1 之间的值）

![score](./images/response_aware_parameter_impact_cn.png)
_图：响应时间感知算法中各参数对选择概率的影响（示意图）_

3.  **智能节点选择**：

    -   当新请求到达时，负载均衡器遍历当前上游组内所有健康（未熔断）的节点。
    -   计算每个健康节点的实时性能得分。
    -   选择得分最低（即综合表现最佳）的节点来处理当前请求。
    -   所选节点的处理中请求计数会相应增加。

4.  **持续自适应调整**：
    -   请求完成后，记录其实际响应时间、是否成功等信息。
    -   这些信息会用于更新节点的平均响应时间、成功率等统计数据。
    -   处理中请求计数相应减少。
    -   这个持续的反馈循环使得算法能够动态适应上游 LLM 服务性能的实时变化。

#### 优势特点

-   **动态适应性**：自动适应上游 LLM 服务性能的实时波动和突发负载，无需人工干预。
-   **LLM 优化**：特别适合处理 LLM 请求的高延迟和延迟不确定性（从毫秒级到数分钟级别均可适用）。
-   **多维度考量**：综合考虑延迟、并发和成功率，避免将流量集中到单一慢节点或过载节点。
-   **平滑分配**：指数加权移动平均等平滑技术避免因瞬时抖动导致决策剧烈摇摆，提供更稳定的负载分配。
-   **高并发性能**：算法设计注重效率，确保在高并发场景下自身开销极小。
-   **故障规避**：与断路器机制紧密集成，自动排除故障或熔断的节点。

#### 适用场景

该算法特别适合以下应用场景：

-   **大语言模型 API 代理**：有效处理 LLM 请求的高延迟和延迟不确定性（从毫秒级到数分钟级别均可适用）。
-   **异构上游服务**：当上游组内包含不同性能特征、不同成本的 LLM 服务时（例如混合使用 GPT-4 和 GPT-3.5，或不同厂商的模型）。
-   **服务质量敏感型应用**：对 LLM 响应延迟有较高要求的场景。

#### 配置示例

```yaml
upstream_groups:
    - name: "mixed_llm_services"
      upstreams:
          # 响应时间感知策略不直接使用 'weight' 参数，它会动态评估
          - name: "fast_but_expensive_llm"
          - name: "slower_but_cheaper_llm"
          - name: "another_llm_provider_model"
      balance:
          strategy: "response_aware" # 启用响应时间感知负载均衡
      http_client:
          # ... (建议为LLM请求配置合适的超时和重试)
          timeout:
              request: 300 # 根据最慢模型的预期调整
```

### 熔断器机制 (Circuit Breaker)

LLMProxy 为每个上游服务实例集成了强大的熔断器模式，旨在提高整个系统的弹性和稳定性，防止局部故障向上游或客户端蔓延。

#### 工作原理

熔断器模拟了电路中保险丝的行为，遵循三种状态的生命周期模型：

1.  **关闭状态（Closed）**：

    -   初始和正常运行状态。所有指向该上游服务的请求都被允许通过。
    -   LLMProxy 持续监控发往此上游的请求的成功与失败情况（通常基于 HTTP 状态码或连接错误）。
    -   如果在定义的统计窗口内，失败率达到了配置的阈值（`breaker.threshold`），熔断器转换到"开启"状态。

2.  **开启状态（Open）**：

    -   熔断器激活（"跳闸"）。此时，LLMProxy 会**立即拒绝**所有新的指向该故障上游服务的请求，而不会实际尝试连接。
    -   这样做可以避免不必要的等待超时，快速失败，并减轻故障上游的压力，给其恢复时间。
    -   如果此上游服务属于某个上游组，负载均衡器会将流量导向组内其他健康的成员（如果存在）。
    -   在"开启"状态持续一段预设的冷却时间（`breaker.cooldown`）后，熔断器转换到"半开"状态。

3.  **半开状态（Half-Open）**：
    -   熔断器允许一小部分（通常是单个）"探测"请求通过，尝试连接到之前故障的上游服务。
    -   **如果探测请求成功**：系统认为该上游服务可能已经恢复。熔断器重置并转换回"关闭"状态，恢复正常流量。
    -   **如果探测请求失败**：系统认为该上游服务仍然不稳定。熔断器重新回到"开启"状态，并开始新一轮的冷却计时。

#### 与负载均衡的协同

-   当一个上游服务的熔断器处于"开启"或"半开"（探测失败时）状态，负载均衡器会将其视为不可用节点，不会将新的用户请求分配给它。
-   智能故障转移：如果一个上游组内有多个上游服务，其中一个或多个被熔断，负载均衡器会自动将流量分配到剩余的健康（"关闭"状态）的服务上。
-   仅当组内所有上游服务都不可用（例如全部熔断）时，发往该组的请求才会失败。

#### 优势特点

-   **快速失败与资源保护**：迅速识别并隔离故障上游，避免客户端请求长时间等待或因上游无响应而耗尽代理资源。
-   **防止级联故障**：通过隔离问题服务，防止其故障压力传导到系统的其他部分或导致客户端重试风暴。
-   **自动恢复检测**：无需人工干预，自动探测上游服务是否恢复，并在恢复后重新纳入服务。
-   **精细化配置**：可以为每个上游服务实例（`upstreams[].breaker`）独立配置熔断阈值和冷却时间，以适应不同服务的特性。
-   **提升系统整体弹性**：使系统在面对部分后端 LLM 服务不稳定或故障时，仍能保持部分功能可用或优雅降级。
-   **可观测性**：通过 Prometheus 指标监控各上游熔断器的状态变化和行为，便于运维。

#### 配置示例

```yaml
upstreams:
    - name: "openai_service_main"
      url: "https://api.openai.com/v1"
      # ... auth config ...
      breaker:
          threshold: 0.5 # 当对此上游的请求在统计窗口内失败率达到50%时，熔断器开启
          cooldown: 30 # 熔断后，等待30秒进入半开状态尝试恢复

    - name: "critical_custom_llm_service"
      url: "http://my.custom.llm:8080"
      breaker:
          threshold: 0.3 # 对于更关键或波动较小的服务，可设置更低的失败率阈值
          cooldown: 60 # 如果服务恢复较慢，可适当延长冷却时间
```

## API 端点

LLMProxy 对外暴露以下主要类型的 HTTP API 端点：

### 转发端点 (Forwarding Endpoints)

-   **路径与端口**：由 `http_server.forwards[]` 配置中的 `port`, `address` 以及客户端请求的原始路径共同决定。
    -   _示例_：若配置了 `port: 3000`, `address: "0.0.0.0"`，则客户端向 `http://<llmproxy_host>:3000/v1/chat/completions` 发起的请求会被此转发服务处理。
-   _描述_：这些是 LLMProxy 的主要工作端点。客户端应用程序（如你的 AI 应用后端）将标准的 LLM API 请求（例如 OpenAI, Anthropic 格式）发送至这些端点。
-   _协议_：HTTP (LLMProxy 本身目前不直接处理 HTTPS 终止；建议在前面使用反向代理如 Nginx 处理 TLS)。
-   _用途_：LLMProxy 接收这些请求，根据配置的 `upstream_group` 进行负载均衡、认证注入、头部修改、断路等处理，然后将请求转发至选定的上游 LLM API 服务，并将上游的响应（包括流式响应）返回给客户端。

### 管理端点 (Admin Endpoints)

这些端点由 `http_server.admin` 配置块定义，默认监听在 `0.0.0.0:9000` (建议生产环境修改为 `127.0.0.1:9000`)。

#### 标准端点

-   **GET /health**

    -   _描述_：提供基础的健康检查。主要用于自动化系统（如 Kubernetes Liveness/Readiness Probes, 负载均衡器健康检查）判断 LLMProxy 服务进程是否正在运行且能够响应请求。
    -   _返回_：
        -   `200 OK`：表示服务基本正常。
        -   (未来可能加入更详细的健康状态，如配置加载是否成功等)
    -   _内容类型_：`text/plain` 或 `application/json`

-   **GET /metrics**

    -   _描述_：暴露 Prometheus 格式的监控指标。供 Prometheus 服务器抓取，用于监控 LLMProxy 的性能、流量、错误、上游状态等。
    -   _返回_：文本格式的 Prometheus 指标数据。
    -   _内容类型_：`text/plain; version=0.0.4; charset=utf-8`

#### 配置管理 API

![openapi_ui](./images/openapi-ui.png)
_图：OpenAPI UI 示意图_

为了增强运维过程中的可观测性与灵活性，管理服务提供了一套全面的配置管理 API。这套 RESTful API 允许您随时检查和修改 LLMProxy 在内存中运行的实时配置，无需重启服务，这对于配置审计、问题排查、动态重配置以及与自动化运维流程集成而言至关重要。

该 API 统一以 `/api/v1` 作为路径前缀进行版本管理。为确保安全，您可以通过设置 `LLMPROXY_ADMIN_AUTH_TOKEN` 环境变量来启用 `Bearer Token` 认证，从而保护这些 `API 端点` 的访问。

**API 端点**

API 提供了一组结构化的端点，用于获取和修改所有关键配置实体的详细信息：

-   **转发服务 (Forwards)**：
    -   `GET /api/v1/forwards`: 获取所有已配置的转发服务列表。
    -   `GET /api/v1/forwards/{name}`: 根据名称获取特定转发服务的详细信息。
-   **上游组 (Upstream Groups)**：
    -   `GET /api/v1/upstream-groups`: 列出所有已配置的上游组。
    -   `GET /api/v1/upstream-groups/{name}`: 获取特定上游组的详细信息。
    -   `PATCH /api/v1/upstream-groups/{name}`: 更新特定上游组的上游服务列表。此操作会原子性地用新提供的列表替换整个上游服务列表。
-   **上游服务 (Upstreams)**：
    -   `GET /api/v1/upstreams`: 列出所有已配置的上游服务。
    -   `GET /api/v1/upstreams/{name}`: 获取特定上游服务的详细信息。
    -   `POST /api/v1/upstreams`: 创建新的上游服务。
    -   `PUT /api/v1/upstreams/{name}`: 更新已存在的上游服务。
    -   `DELETE /api/v1/upstreams/{name}`: 删除上游服务（具有依赖保护机制，防止删除仍被上游组引用的服务）。

**动态配置**

动态配置 API 实现了 LLMProxy 配置的热重载，无需重启服务：

-   **上游服务管理**：随时创建、更新或删除上游服务。
-   **上游组管理**：通过修改其上游服务列表来重新配置上游组。
-   **依赖保护**：内置安全机制防止破坏性变更，例如删除当前正在使用的上游服务。
-   **配置一致性**：所有修改都会保持 LLMProxy 配置的完整性。

**交互式 OpenAPI UI**

为了尽可能地方便您探索和使用配置管理 API，LLMProxy 内置了 OpenAPI UI。这个交互式界面为所有端点提供了详尽的文档，并允许您直接在浏览器中执行 API 调用。

-   **访问方式**: OpenAPI UI 可通过管理端口的 `/api/v1/docs` 路径访问。出于安全考虑，它**仅在** LLMProxy 以调试模式（例如，使用 `-d` 或 `--debug` 命令行标志）启动时才会启用。

## Prometheus 指标

LLMProxy 通过管理端点的 `/metrics` 路径暴露全面的 Prometheus 指标，用于实时监控系统性能、请求处理、上游服务健康状况和内部组件状态。这些指标对于运维、故障排查和容量规划至关重要。

以下是关键指标类别和示例：

### HTTP 服务器与请求指标 (针对客户端到 LLMProxy 的入站请求)

-   `llmproxy_http_requests_total` (计数器)
    -   描述：接收到的 HTTP 请求总数。
    -   标签：`forward` (转发服务名称), `method` (HTTP 方法), `path` (请求路径)。
-   `llmproxy_http_request_duration_seconds` (直方图)
    -   描述：处理 HTTP 请求的延迟分布。
    -   标签：`forward`, `method`, `path`。
-   `llmproxy_http_request_errors_total` (计数器)
    -   描述：处理 HTTP 请求时发生的错误总数。
    -   标签：`forward`, `error`, `status`。
-   `llmproxy_ratelimit_total` (计数器)
    -   描述：因速率限制而被拒绝的请求总数。
    -   标签：`forward`。

### 上游客户端指标 (针对 LLMProxy 到后端 LLM 服务的出站请求)

-   `llmproxy_upstream_requests_total` (计数器)
    -   描述：向上游 LLM 服务发送的请求总数。
    -   标签：`group` (上游组名称), `upstream` (上游服务名)。
-   `llmproxy_upstream_duration_seconds` (直方图)
    -   描述：向上游 LLM 服务发送请求并获得响应的延迟分布。
    -   标签：`group`, `upstream`。
-   `llmproxy_upstream_errors_total` (计数器)
    -   描述：与上游 LLM 服务通信时发生的错误总数。
    -   标签：`error` (错误类型), `group`, `upstream`。

### 断路器指标

-   `llmproxy_circuitbreaker_state_changes_total` (计数器)
    -   描述：断路器状态转换的总次数。
    -   标签：`group`, `upstream`, `url`, `from` (原状态), `to` (新状态)。
-   `llmproxy_circuitbreaker_calls_total` (计数器)
    -   描述：通过断路器处理的调用总数（包括成功、失败、被拒绝的）。
    -   标签：`group`, `upstream`, `url`, `result` (结果类型)。

这些指标可以通过 Prometheus 抓取后，使用 Grafana 等工具进行可视化和告警配置，从而实现对 LLMProxy 服务及其代理的 LLM API 调用的全面监控。

## 许可证

[MIT 许可证](LICENSE)
