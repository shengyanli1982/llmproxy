English | [中文](./README_CN.md)

<div align="center">
    <img src="./images/logo.png" alt="logo" width="650">
</div>

**LLMProxy: Enterprise-grade intelligent proxy and load balancer designed specifically for large language models. Unify management and orchestration of various LLM services (public cloud APIs, privately deployed vLLM/Ollama, etc.), enabling efficient, stable, and scalable LLM application access in multi-cloud/hybrid cloud architectures while minimizing client code modifications.**

<p align="center">
  <a href="#introduction">Introduction</a>
  |
  <a href="#quick-start">Quick Start</a>
  |
  <a href="#core-features">Core Features</a>
  |
  <a href="#use-cases">Use Cases</a>
  |
  <a href="#configuration-guide">Configuration Guide</a>
  |
  <a href="#advanced-deployment">Advanced Deployment</a>
  |
  <a href="#understanding-llmproxy">Understanding LLMProxy</a>
  |
  <a href="#api-endpoints">API Endpoints</a>
  |
  <a href="#prometheus-metrics">Prometheus Metrics</a>
  |
  <a href="#license">License</a>
</p>

## Introduction

**LLMProxy** is an enterprise-grade high-availability intelligent proxy and load balancing solution designed specifically for large language model (LLM) APIs. It serves as a unified entry point, receiving client requests and efficiently distributing them to various upstream LLM services (such as OpenAI, Anthropic, or privately deployed vLLM, Ollama, etc.) through flexible routing strategies and LLM-optimized load balancing algorithms, then safely returning responses. LLMProxy aims to solve the challenges of performance, cost, availability, and management complexity faced when directly calling LLM APIs, significantly improving the stability, response speed, and resource utilization efficiency of LLM applications through fine-grained traffic scheduling, connection management, and fault tolerance mechanisms.

### Why Choose LLMProxy?

LLMProxy effectively addresses key challenges in enterprise-level LLM API deployments:

-   **Unified LLM Access & High Availability**: Aggregates multiple LLM services (different cloud providers, private models like vLLM/Ollama), eliminating single points of failure and ensuring business continuity through intelligent routing and failover.
-   **LLM-Optimized Load Balancing**: Built-in strategies (round-robin, weighted, response-time aware) specifically optimized for LLM long connections and streaming responses, dynamically allocating requests to the best service nodes, balancing cost and performance.
-   **Powerful Fault Tolerance & Resilience**: Integrated circuit breaker pattern automatically isolates failing upstream services to prevent cascading failures; supports request retries to improve success rates of LLM calls in complex network environments.
-   **Easy Scaling & Cost Control**: Add or reduce upstream LLM services as needed, seamlessly expanding processing capacity; optimize LLM call expenses by prioritizing low-cost or high-performance resources through load balancing strategies.
-   **Simplified Integration & Management**: Provides a unified API entry point, shielding backend LLM service differences and simplifying client integration; centrally manage routing, authentication, and security policies through configuration files.
-   **Enhanced Observability**: Provides detailed Prometheus metrics for real-time monitoring of LLM service calls, proxy performance, and fault diagnosis.

## Quick Start

### 1. Running the Application Directly

This is the quickest way to experience LLMProxy without complex environment setup.

**Step 1: Download the Pre-compiled Binary**

Visit the project's [GitHub Releases](https://github.com/shengyanli1982/llmproxy/releases) page and download the latest pre-compiled binary package for your operating system (Windows, Linux, macOS), such as `llmproxyd-Linux-x64-<version>.zip` or `llmproxyd-Windows-x64-<version>.zip`.

After downloading, extract the file to get an executable named `llmproxyd-<os>-<arch>` (Linux/macOS) or `llmproxyd-windows-x64.exe` (Windows).

**Step 2: Create a Configuration File**

In the same directory as the executable, create a file named `config.yaml`. LLMProxy is designed to proxy large language models, and here's a minimal configuration example that forwards requests from local port `3000` to the OpenAI API. Note that you'll need to replace `YOUR_OPENAI_API_KEY_HERE` with your actual OpenAI API key for it to work:

```yaml
http_server:
    forwards:
        - name: "llm_openai_service"      # Forward service name
          port: 3000                      # Port LLMProxy listens on
          address: "0.0.0.0"              # Listen on all network interfaces
          upstream_group: "openai_main_group" # Link to the upstream group defined below
    admin:
        port: 9000                      # Admin port for monitoring
        address: "127.0.0.1"            # Recommended local-only access for security

upstreams:
    - name: "openai_chat_api"           # Upstream service name, e.g., OpenAI
      url: "https://api.openai.com/v1"  # Base URL for OpenAI API
      auth:
          type: "bearer"                # Authentication type is Bearer Token
          token: "YOUR_OPENAI_API_KEY_HERE" # !!IMPORTANT!! Replace with your actual OpenAI API key
                                      # If you don't have an OpenAI key, you can choose another LLM service or use a mock service for testing

upstream_groups:
    - name: "openai_main_group"         # Upstream group name
      upstreams:
          - name: "openai_chat_api"       # Reference to the openai_chat_api upstream defined above
    # [Optional] Configure longer timeout for LLM requests
    http_client:
      timeout:
        request: 300 # LLM requests typically need more time, 300 seconds or more is recommended
```

This configuration defines a forwarding service listening on port `3000` that routes requests to an upstream group named `openai_main_group`. The upstream group is configured with the OpenAI API as its backend service and sets appropriate request timeout values. For more detailed and advanced configuration options, refer to the `config.default.yaml` file in the project's root directory.

**Step 3: Run LLMProxy**

Open a terminal or command prompt, navigate to the directory containing `llmproxyd-<os>-<arch>` (or `llmproxyd-windows-x64.exe`) and `config.yaml`, then execute the following commands:

-   For Linux/macOS:
    ```bash
    mv llmproxyd-<os>-<arch> llmproxyd
    chmod +x llmproxyd # You may need to add execute permissions the first time
    ./llmproxyd --config config.yaml
    ```
-   For Windows:
    ```bash
    .\llmproxyd-windows-x64.exe --config config.yaml
    ```

If all goes well, you'll see LLMProxy start up and begin listening on the configured ports.

**Step 4: Test the Proxy Service**

Open another terminal and use `curl` or a similar tool to send an LLM API request to the forwarding port configured in LLMProxy. For example, if your `config.yaml` has the `llm_openai_service` service listening on port `3000` and the upstream is the OpenAI API, you can try sending a chat request (make sure your request body follows the OpenAI API format and replace the API Key in the request body as needed):

```bash
curl http://localhost:3000/v1/chat/completions \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer YOUR_CLIENT_SIDE_OPENAI_API_KEY" \ # Note: This key is typically provided by the client application and forwarded by LLMProxy. LLMProxy itself also configures a server-side key in the upstreams section.
  -d '{
    "model": "gpt-3.5-turbo",
    "messages": [{"role": "user", "content": "Hello, LLMProxy! Please introduce yourself."}],
    "stream": false
  }'
```

This request will be received by LLMProxy and forwarded to the OpenAI upstream service (`https://api.openai.com/v1`) defined in your `config.yaml`, according to the configuration (including authentication, load balancing, etc.). You should see a JSON response from the OpenAI API. If the upstream service supports streaming responses and you enable streaming in your request (e.g., `"stream": true`), LLMProxy will also correctly handle the streaming data.

You've now successfully run LLMProxy! Next, you can explore more advanced configurations and features.

### 2. Deployment with Docker (Recommended)

Using Docker Compose is one of the most convenient ways to deploy LLMProxy. Complete Docker Compose configuration examples are provided in the project's `examples/config` directory.

1. **Prepare the Configuration File**:

    Place your custom `config.yaml` file in the same directory as the `docker-compose.yaml` file.

2. **Start the Service**:

    ```bash
    docker-compose up -d
    ```

3. **View Running Logs**:

    ```bash
    docker-compose logs -f llmproxy # llmproxy is the service name defined in the compose file
    ```

4. **Stop the Service**:

    ```bash
    docker-compose down
    ```

Docker Compose Configuration Example (refer to `examples/config/docker-compose.yaml` in the project for the latest version):

```yaml
version: "3.8" # Using a newer compose version is recommended

services:
    llmproxy:
        image: shengyanli1982/llmproxy:latest # For production, using a specific tag version is recommended
        container_name: llmproxy_service
        restart: unless-stopped
        ports:
            # Map ports according to the forwards defined in your config.yaml
            - "3000:3000" # Example: mapping the forwarding service listening on port 3000 in the config file
            # - "3001:3001"
            # Admin interface port mapping
            - "127.0.0.1:9000:9000" # Recommended to map the admin port only to the local loopback address
        volumes:
            - ./config.yaml:/app/config.yaml:ro # Mount your configuration file into the container
            # If you need to persist logs, you can mount a log directory
            # - ./logs:/app/logs
        command: ["--config", "/app/config.yaml"]
        environment:
            - TZ=America/New_York # Set container timezone
            # You can override some configurations with environment variables, for example:
            # - LLMPROXY_UPSTREAMS__0__AUTH__TOKEN=your_env_openai_key
        networks:
            - llmproxy_net

networks:
    llmproxy_net:
        driver: bridge
```

## Core Features

-   🔄 **Intelligent LLM Routing & Request Handling**

    -   Configure independent forwarding services for different business scenarios or LLM models through `http_server.forwards`, enabling fine-grained management.
    -   Customize dedicated listening addresses and ports for each forwarding service.
    -   Flexibly route requests to specified upstream LLM service groups based on paths, headers, or other request characteristics.

-   🌐 **Unified Upstream LLM Service Management**

    -   Centrally define and manage various upstream LLM services (public cloud APIs, private model services like vLLM/Ollama, etc.) through `upstreams`.
    -   Independently name, configure URLs, and manage health for each upstream LLM service.
    -   Built-in authentication proxy mechanisms (Bearer Token, API Key Header Injection, Basic Auth) to securely connect with different types of LLM services.
    -   Flexible HTTP header operations (add, delete, modify) to adapt to special requirements of different LLM APIs or inject tracking information.

-   ⚡ **LLM-Optimized Load Balancing**

    -   Use `upstream_groups` to organize functionally similar or mutually backup LLM services into upstream groups for unified scheduling and high availability.
    -   Provide multiple load balancing strategies optimized for LLMs:
        -   **Round Robin** - Distribute requests evenly among upstream LLM services.
        -   **Weighted Round Robin** - Distribute requests to different LLM services according to preset weights (e.g., service processing capacity, cost considerations).
        -   **Random** - Randomly select an available LLM service.
        -   **Response Aware** - Especially suitable for LLM services, monitoring node performance in real-time (response latency, concurrent load, success rate) and dynamically directing requests to the currently optimal node, maximizing throughput and user experience.
        -   **Failover** - Try upstream services in the order they are listed. If the current upstream is unavailable, automatically switch to the next one, providing sequential backup capability.
    -   Set weights for each upstream LLM service in the weighted round-robin strategy.

-   🔁 **Flexible Traffic Control & QoS Assurance**

    -   Configure rate limits based on IP or other identifiers (requests/second, concurrent request peaks) for each forwarding service.
    -   Protect backend LLM services from malicious attacks or traffic surges, ensuring service quality (QoS) for core business.

-   🔌 **LLM-Optimized Connection Management**

    -   **Inbound Connection Management:** Configure precise connection timeouts for client connections.
    -   **Outbound Connection & Request Optimization (for upstream LLM services):**
        -   Custom User-Agent for upstream service identification and statistics.
        -   TCP Keepalive to maintain long-lasting connections with upstream LLM services, reducing handshake latency, especially beneficial for streaming responses.
        -   Fine-grained timeout control (connection timeout, request timeout, idle timeout) to accommodate diverse response time characteristics of LLM services (from seconds to minutes).
        -   Configure intelligent retry strategies for transient errors that may occur with LLM APIs (configurable attempt counts, initial backoff intervals, and exponential backoff).
        -   Support connecting to upstream LLM services through outbound HTTP/HTTPS proxies to meet enterprise network security and compliance requirements.
        -   **Native Streaming Support & Timeout Control**: Natively handles LLM streaming responses (Server-Sent Events) via the `http_client.stream` setting. When enabled (default), it disables the request timeout, which is crucial for long-lived streaming connections that would otherwise be prematurely terminated. When disabled, it enforces a fixed request timeout, making it ideal for non-streaming API calls.

-   🛡️ **Robust Fault Tolerance & Failover**

    -   **Intelligent Circuit Breaker:** Automatically monitor the health status of upstream LLM services (based on error rates) and quickly isolate failing nodes when thresholds are reached.
    -   **Configurable Circuit Breaking Policies:** Customize circuit breaking thresholds (e.g., failure rate) and cooldown times (waiting time to enter half-open state after breaking) for each upstream LLM service.
    -   **Automatic Recovery & Probing:** Periodically attempt to send probe requests to failed nodes after circuit breaking, automatically reintegrating them into the load balancing pool once service is restored.
    -   **Seamless Failover:** When an upstream LLM service in a group fails or trips the circuit breaker, automatically and smoothly switch traffic to other healthy nodes in the group, transparent to clients, ensuring business continuity.

-   📊 **Observability & Management Interface**
    -   Provide independent management interface and API endpoints through `http_server.admin`.
    -   Offer `/health` health check endpoint for integration with various monitoring and automated operations systems.
    -   Expose rich Prometheus metrics through the `/metrics` endpoint, providing comprehensive insights into LLM proxy performance, traffic, errors, latency, upstream LLM service health status, and circuit breaker states.

## Use Cases

LLMProxy is designed for enterprise-level application scenarios that require efficient, reliable, and scalable access to and management of large language model APIs:

-   **Enterprise AI Application Gateway**:

    -   Provides a unified LLM API access entry point for multiple applications or teams within an enterprise.
    -   Centrally implements access authentication and API key configuration for large language models.

-   **Multi-tenancy and Service Isolation**:

    -   Achieve a multi-tenant architecture within a single LLMProxy instance by configuring independent `forwards` and `upstream_groups` for different teams, applications, or customers (tenants).
    -   Assign a unique access endpoint (port) to each tenant and apply separate routing rules, API keys, rate limits, and load balancing strategies.
    -   This is particularly useful for SaaS platforms that need to provide customized LLM services to different customers, or for isolating resources and billing for different departments within an enterprise.

-   **High-Availability, High-Concurrency LLM Services**:

    -   Build high-traffic AI products for end users (such as intelligent customer service, content generation tools, AI assistants).
    -   Ensure uninterrupted service through load balancing and failover across instances from multiple LLM providers (such as OpenAI, Anthropic, Azure OpenAI, Google Gemini) or self-built models (vLLM, Ollama).
    -   Utilize advanced strategies like response-time awareness to dynamically allocate traffic to the best-performing nodes, enhancing user experience.

-   **LLM Application Development & Testing Acceleration**:

    -   Simplify the complexity of developer integration with multiple LLM APIs, decoupling application code from specific LLM services.
    -   Easily switch between different LLM models or providers for effect evaluation and cost comparison.
    -   Simulate different upstream responses (such as latency, errors) for test environments, or isolate test traffic.

-   **Multi-Cloud/Hybrid Cloud LLM Strategy Implementation**:

    -   Provide a unified LLM API access layer in complex cloud environments (such as AWS, Azure, GCP, and on-premises data centers in hybrid deployment).
    -   Route requests to specific geographic locations or specific types of LLM services based on data sovereignty, compliance requirements, or cost factors.
    -   Deploy as an independent service in container orchestration platforms like Kubernetes to provide LLM access capabilities for microservices.

-   **API Version & Compatibility Management**:
    -   When backend LLM APIs upgrade or undergo incompatible changes, LLMProxy can serve as an adaptation layer, maintaining compatibility with older clients through header operations or lightweight transformations (possibly supported in future versions).

By applying LLMProxy in these scenarios, enterprises can significantly enhance the reliability, performance, and manageability of their LLM applications while reducing integration and operational complexity.

## Configuration Guide

LLMProxy uses structured YAML files for configuration, offering flexible and powerful configuration options. Below is a detailed explanation of key configuration sections:

### Configuration Options Explained

#### HTTP Server Configuration Options

| Configuration Item                            | Type    | Default   | Description                                                                       |
| --------------------------------------------- | ------- | --------- | --------------------------------------------------------------------------------- |
| `http_server.forwards[].name`                 | String  | -         | **[Required]** Unique identifier name for the forwarding service                  |
| `http_server.forwards[].port`                 | Integer | 3000      | **[Required]** Listening port for the forwarding service                          |
| `http_server.forwards[].address`              | String  | "0.0.0.0" | Binding network address for the forwarding service                                |
| `http_server.forwards[].upstream_group`       | String  | -         | **[Required]** Name of the upstream group associated with this forwarding service |
| `http_server.forwards[].ratelimit`            | Object  | null      | **[Optional]** Rate limiting configuration. If omitted, rate limiting is disabled |
| `http_server.forwards[].ratelimit.per_second` | Integer | 100       | Maximum number of requests allowed per second per IP (range: 1-10000)             |
| `http_server.forwards[].ratelimit.burst`      | Integer | 200       | Number of burst requests allowed per IP (buffer size) (range: 1-20000)            |
| `http_server.forwards[].timeout`              | Object  | null      | **[Optional]** Timeout configuration. If omitted, default values are used         |
| `http_server.forwards[].timeout.connect`      | Integer | 10        | Timeout for client connections to LLMProxy (seconds)                              |
| `http_server.admin.port`                      | Integer | 9000      | Optional listening port for the admin service                                     |
| `http_server.admin.address`                   | String  | "0.0.0.0" | Binding network address for the admin service                                     |
| `http_server.admin.timeout`                   | Object  | null      | **[Optional]** Timeout configuration. If omitted, default values are used         |
| `http_server.admin.timeout.connect`           | Integer | 10        | Timeout for connections to the admin interface (seconds)                          |

#### Upstream Service Configuration Options (Upstream LLM Services)

| Configuration Item              | Type    | Default | Description                                                                                                                    |
| ------------------------------- | ------- | ------- | ------------------------------------------------------------------------------------------------------------------------------ |
| `upstreams[].name`              | String  | -       | **[Required]** Unique identifier name for the upstream LLM service                                                             |
| `upstreams[].url`               | String  | -       | **[Required]** Full URL for the upstream LLM service (e.g., `https://api.openai.com/v1/chat/completions`)                      |
| `upstreams[].auth.type`         | String  | "none"  | Authentication type: `bearer`, `basic`, or `none`                                                                              |
| `upstreams[].auth.token`        | String  | -       | API key or token when `type` is `bearer`                                                                                       |
| `upstreams[].auth.username`     | String  | -       | Username when `type` is `basic`                                                                                                |
| `upstreams[].auth.password`     | String  | -       | Password when `type` is `basic`                                                                                                |
| `upstreams[].headers[].op`      | String  | -       | HTTP header operation type: `insert` (add if not exists), `replace` (replace or add), `remove`                                 |
| `upstreams[].headers[].key`     | String  | -       | Name of the HTTP header to operate on                                                                                          |
| `upstreams[].headers[].value`   | String  | -       | Header value for `insert` or `replace` operations                                                                              |
| `upstreams[].breaker.threshold` | Float   | 0.5     | Circuit breaker trigger threshold, representing failure rate (0.01-1.0), e.g., 0.5 means 50% failures trigger circuit breaking |
| `upstreams[].breaker.cooldown`  | Integer | 30      | Circuit breaker cooldown time (seconds), i.e., how long after breaking to try half-open state (1-3600)                         |

#### Upstream Group Configuration Options (Upstream LLM Groups)

> [!NOTE]
>
> The parameter `upstreams[].url` should be configured with the full URL of the upstream service, e.g., `https://api.openai.com/v1/chat/completions`, not `https://api.openai.com/v1` or `https://api.openai.com`.

| Configuration Item                              | Type    | Default        | Description                                                                                                                                                                                                                                        |
| ----------------------------------------------- | ------- | -------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `upstream_groups[].name`                        | String  | -              | **[Required]** Unique identifier name for the upstream group                                                                                                                                                                                       |
| `upstream_groups[].upstreams[].name`            | String  | -              | **[Required]** Referenced upstream LLM service name, must be defined in the `upstreams` section                                                                                                                                                    |
| `upstream_groups[].upstreams[].weight`          | Integer | 1              | Weight value effective only when `balance.strategy` is `weighted_roundrobin`, used for proportional request allocation (range: 1-65535)                                                                                                            |
| `upstream_groups[].balance.strategy`            | String  | "roundrobin"   | Load balancing strategy: `roundrobin`, `weighted_roundrobin`, `random`, `response_aware` or `failover`                                                                                                                                             |
| `upstream_groups[].http_client.agent`           | String  | "LLMProxy/1.0" | User-Agent header value sent to upstream LLM services                                                                                                                                                                                              |
| `upstream_groups[].http_client.keepalive`       | Integer | 30             | TCP Keepalive time (seconds), range 5-600, 0 is not allowed. Helps keep connections with upstream LLM services active, reducing latency                                                                                                            |
| `upstream_groups[].http_client.stream`          | Boolean | true           | Controls the request timeout behavior. If `true` (default), the request timeout is disabled, which is **essential** for LLM streaming responses (Server-Sent Events). If `false`, `timeout.request` is enforced, suitable for non-streaming calls. |
| `upstream_groups[].http_client.timeout`         | Object  | null           | **[Optional]** Timeout configuration. If omitted, default values are used                                                                                                                                                                          |
| `upstream_groups[].http_client.timeout.connect` | Integer | 10             | Timeout for connecting to upstream LLM services (seconds) (range: 1-120)                                                                                                                                                                           |
| `upstream_groups[].http_client.timeout.request` | Integer | 300            | Request timeout (seconds) for non-streaming requests. Only effective when `http_client.stream` is `false`. Defines the maximum waiting time for a complete upstream response. (range: 1-1200)                                                      |
| `upstream_groups[].http_client.timeout.idle`    | Integer | 60             | Timeout (seconds) after which a connection with an upstream LLM service is considered idle and closed if no activity (range: 5-1800)                                                                                                               |
| `upstream_groups[].http_client.retry`           | Object  | null           | **[Optional]** Request retry configuration. If omitted, retry functionality is disabled                                                                                                                                                            |
| `upstream_groups[].http_client.retry.attempts`  | Integer | 3              | Maximum number of retry attempts (excluding the first attempt) (range: 1-100)                                                                                                                                                                      |
| `upstream_groups[].http_client.retry.initial`   | Integer | 500            | Initial waiting time (milliseconds) before the first retry, subsequent retry intervals may use exponential backoff (range: 100-10000)                                                                                                              |
| `upstream_groups[].http_client.proxy`           | Object  | null           | **[Optional]** Outbound proxy configuration. If omitted, no proxy will be used                                                                                                                                                                     |
| `upstream_groups[].http_client.proxy.url`       | String  | -              | Outbound proxy server URL (e.g., `http://user:pass@proxy.example.com:8080`)                                                                                                                                                                        |

### HTTP Server Configuration

```yaml
http_server:
    # Forward service configuration (handling client inbound requests)
    forwards:
        - name: "to_openai_llm_group" # [Required] Unique name for the forwarding service
          port: 3000 # [Required] Port this service listens on
          address: "0.0.0.0" # [Optional] Binding network address (default: "0.0.0.0", listen on all interfaces)
          upstream_group: "openai_llm_group" # [Required] Target upstream group for this forward
          ratelimit: # [Optional] IP rate limiting
              enabled: true # Whether to enable rate limiting (default: false)
              per_second: 100 # Maximum requests per second per IP
              burst: 200 # Burst request capacity per IP
          timeout: # [Optional] Client connection timeout
              connect: 10 # Timeout for client connections to LLMProxy (seconds)

    # Admin interface configuration
    admin:
        port: 9000 # [Optional] Admin interface port (for /metrics, /health)
        address: "127.0.0.1" # [Optional] Binding network address (default: "0.0.0.0", recommended "127.0.0.1" for production)
        timeout:
            connect: 10 # Timeout for connections to the admin interface (seconds)
```

### Upstream Service Configuration (Defining Backend LLM API Services)

```yaml
upstreams:
    - name: "openai_gpt4_primary" # [Required] Unique identifier name for the upstream LLM service
      url: "https://api.openai.com/v1" # [Required] Base URL for the upstream LLM API
      auth: # [Optional] Authentication configuration
          type: "bearer" # Authentication type: "bearer", "basic", or "none" (default)
          token: "YOUR_OPENAI_API_KEY" # [Required for bearer auth] API key/token
          # username: "user"         # [Required for basic auth] Username
          # password: "pass"         # [Required for basic auth] Password
      headers: # [Optional] HTTP header operations (modify before forwarding request to this upstream)
          - op: "insert" # Operation type: "insert", "replace", or "remove"
            key: "X-Custom-Proxy-Header" # Name of the HTTP header to operate on
            value: "LLMProxy-OpenAI-GPT4" # Header value (for "insert" or "replace" operations)
      breaker: # [Optional] Circuit breaker configuration for this upstream
          threshold: 0.5 # Failure rate threshold to trigger the circuit breaker (0.01-1.0, default: 0.5)
          cooldown: 30 # Cooling period before entering half-open state (seconds) (1-3600, default: 30)

    - name: "anthropic_claude_haiku"
      url: "https://api.anthropic.com" # Example: Anthropic API
      auth:
          type: "bearer" # Anthropic also typically uses Bearer Token
          token: "YOUR_ANTHROPIC_API_KEY"
      headers: # Anthropic may require anthropic-version in header
          - op: "insert"
            key: "anthropic-version"
            value: "2023-06-01"
          - op: "insert" # anthropic-beta: messages-2023-12-15, max-tokens-3-5-sonnet-2024-07-15, etc.
            key: "anthropic-beta"
            value: "max-tokens-3-5-sonnet-2024-07-15"
      breaker:
          threshold: 0.4
          cooldown: 45
```

### Upstream Group Configuration (Organizing Upstreams and Defining Load Balancing Behavior)

```yaml
upstream_groups:
    - name: "openai_llm_group" # [Required] Unique identifier name for the upstream group
      upstreams: # [Required] List of upstream LLM services in this group (at least one)
          - name: "openai_gpt4_primary" # Reference to the `name` defined in the `upstreams` section
            weight: 8 # [Optional] Weight, only effective when `balance.strategy` is "weighted_roundrobin" (default: 1)
          # - name: "another_openai_backup_service"
          #   weight: 2
      balance:
          strategy:
              "weighted_roundrobin" # Load balancing strategy:
              # "roundrobin" (default round-robin),
              # "weighted_roundrobin" (weighted round-robin),
              # "random" (random),
              # "response_aware" (response time aware, recommended for LLM),
              # "failover" (failover strategy, tries upstreams in order)
      http_client: # [Optional] Define how LLMProxy communicates with upstream LLM services in this group
          agent: "LLMProxy/1.0 (OpenAI-Group)" # [Optional] Custom User-Agent header
          keepalive: 90 # [Optional] TCP keepalive time (seconds) (0-600, 0=disabled, default: 60)
          stream: true # [Optional] Enable streaming mode (important for LLM streaming responses, default: true)
          timeout:
              connect: 15 # Timeout for connecting to upstream LLM services (seconds) (default: 10)
              request: 360 # Request timeout (seconds) (default: 300, may need higher for time-consuming LLMs)
              idle: 90 # Idle connection timeout (seconds) (default: 60)
          retry: # [Optional] Request retry configuration
              attempts: 3 # Maximum retry attempts
              initial: 1000 # Initial waiting time before first retry (milliseconds)
          proxy: # [Optional] Outbound proxy configuration
              url: "http://user:pass@your-proxy-server.com:8080" # Proxy server URL
```

### Configuration Best Practices

1. **Security Recommendations**:

    - Strictly limit access to the admin interface (`admin` service). Bind it to the local loopback address (`address: "127.0.0.1"`), and consider using firewall rules or a reverse proxy (like Nginx) to add additional authentication and access control.

2. **Performance & Cost Optimization**:

    - Fine-tune timeout configurations (`timeout.connect`, `timeout.request`, `timeout.idle`) and retry strategies (`retry`) based on the characteristics of different LLM service providers' APIs (such as response time, concurrency limits, billing models).
    - For LLM services that support streaming responses, ensure `http_client.stream: true` (this is the default value) to receive and forward data with minimal latency.
    - Configure rate limits (`ratelimit`) reasonably to both protect backend LLM services from overload and meet business needs during peak periods.
    - Leverage the `weighted_roundrobin` load balancing strategy, combined with the cost and performance of different LLM services, to direct more traffic to services with better price-performance ratios.
    - For latency-sensitive applications, prioritize using the `response_aware` load balancing strategy, which can dynamically select the best-performing upstream service at any given time.

3. **Reliability & Resilience Design**:
    - Configure reasonable circuit breaker parameters (`breaker.threshold`, `breaker.cooldown`) for each upstream LLM service (`upstreams`). Threshold settings need to balance the sensitivity of fault detection and the inherent volatility of the service.
    - Configure multiple upstream LLM service instances in each upstream group (`upstream_groups`) to achieve redundancy and automatic failover. These can be different regional nodes of the same provider or alternative services from different providers.
    - Enable request retries (by configuring the `http_client.retry` object) only for idempotent or safely retryable LLM API calls. Note that some LLM operations (such as content generation) may not be idempotent.
    - Regularly monitor Prometheus metrics data about circuit breaker status, upstream error rates, request latency, etc., and use this information to optimize configurations and troubleshoot potential issues.

For detailed explanations of all available configuration options, please refer to the `config.default.yaml` file included with the LLMProxy project as a complete reference.

### Example: Multi-tenancy Configuration

LLMProxy can easily achieve multi-tenancy or service isolation by mapping different `forwards` (listening on different ports) to different `upstream_groups`. Each `upstream_group` can have its own independent upstream LLM services, load balancing strategies, and client behavior configurations. This allows a single LLMProxy instance to serve multiple independent clients or applications while maintaining configuration and traffic isolation.

The following example shows how to configure independent proxy services for two tenants (`tenant-a` and `tenant-b`):

-   `tenant-a` accesses the service on port `3001` with its own dedicated OpenAI API key and rate limiting policy.
-   `tenant-b` accesses the service on port `3002`, uses a different API key, is configured with a Failover strategy, and has stricter rate limits.

```yaml
http_server:
    forwards:
        - name: "tenant-a-service"
          port: 3001
          address: "0.0.0.0"
          upstream_group: "tenant-a-group"
          ratelimit:
              enabled: true
              per_second: 50 # Rate limit for Tenant A
              burst: 100
        - name: "tenant-b-service"
          port: 3002
          address: "0.0.0.0"
          upstream_group: "tenant-b-group"
          ratelimit:
              enabled: true
              per_second: 20 # Rate limit for Tenant B
              burst: 40

upstreams:
    - name: "openai_primary_for_a"
      url: "https://api.openai.com/v1"
      auth:
          type: "bearer"
          token: "TENANT_A_OPENAI_API_KEY" # API key for Tenant A
    - name: "openai_primary_for_b"
      url: "https://api.openai.com/v1"
      auth:
          type: "bearer"
          token: "TENANT_B_OPENAI_API_KEY" # API key for Tenant B
    - name: "openai_backup_for_b"
      url: "https://api.openai.com/v1"
      auth:
          type: "bearer"
          token: "TENANT_B_BACKUP_API_KEY" # Backup API key for Tenant B

upstream_groups:
    # Configuration group for Tenant A
    - name: "tenant-a-group"
      upstreams:
          - name: "openai_primary_for_a"
      balance:
          strategy: "roundrobin" # Simple round-robin
      http_client:
          timeout:
              request: 300

    # Configuration group for Tenant B
    - name: "tenant-b-group"
      upstreams:
          - name: "openai_primary_for_b" # Primary service
          - name: "openai_backup_for_b" # Backup service
      balance:
          strategy: "failover" # Automatically switch to backup on failure
      http_client:
          timeout:
              request: 360
```

## Advanced Deployment

LLMProxy supports various flexible deployment methods, including Kubernetes cluster deployment and traditional Linux system service deployment. Here are detailed instructions for each deployment method:

### Kubernetes Deployment

For Kubernetes environments, we provide recommended deployment configuration files (Deployment, Service, ConfigMap, etc.) in the `examples/config/kubernetes` directory.

1. **Prepare Configuration Files and Secrets**:

    - Put your `config.yaml` content into `examples/config/kubernetes/configmap.yaml`, or create a ConfigMap through other means.
    - **Strongly recommended**: Store sensitive information like API keys in Kubernetes Secrets, and inject them into the Pod via environment variables or volume mounts, then reference these environment variables in `config.yaml`.

2. **Apply Deployment Manifests**:

    ```bash
    # (Optional) Create namespace
    kubectl apply -f examples/config/kubernetes/namespace.yaml

    # Create ConfigMap (containing LLMProxy configuration)
    kubectl apply -f examples/config/kubernetes/configmap.yaml -n llmproxy

    # (Optional) Create Secrets (for storing API keys)
    # kubectl create secret generic llm-api-keys -n llmproxy \
    #   --from-literal=OPENAI_API_KEY='your_openai_key' \
    #   --from-literal=ANTHROPIC_API_KEY='your_anthropic_key'

    # Create Deployment
    kubectl apply -f examples/config/kubernetes/deployment.yaml -n llmproxy

    # Create Service (exposing LLMProxy service)
    kubectl apply -f examples/config/kubernetes/service.yaml -n llmproxy
    ```

3. **Verify Deployment Status**:

    ```bash
    kubectl get pods -n llmproxy -l app=llmproxy
    kubectl get services -n llmproxy llmproxy
    kubectl logs -n llmproxy -l app=llmproxy -f # View logs
    ```

4. **Access the Service**:

    - **Internal Cluster Access** (via Service name):
      `http://llmproxy.llmproxy.svc.cluster.local:<port>` (port according to Service definition)
    - **External Cluster Access**:
      Typically achieved by configuring an Ingress resource, or using a `LoadBalancer` type Service (if your K8s environment supports it). For port forwarding testing:
        ```bash
        kubectl port-forward svc/llmproxy -n llmproxy 3000:3000 9000:9000 # Forward service ports to local
        ```
        Then you can access the forwarding service via `http://localhost:3000` and the admin interface via `http://localhost:9000`.

### Linux System Service Deployment (using systemd)

For traditional Linux server environments, you can use systemd to manage the LLMProxy service.

1. **Download and Install the Binary**:
   Visit [GitHub Releases](https://github.com/shengyanli1982/llmproxy/releases) to download the latest `llmproxyd-Linux-x64-<version>.zip`.

    ```bash
    # Example version, please replace with the latest version
    VERSION="0.1.0" # Assuming this is the version you downloaded
    curl -L -o llmproxyd-Linux-x64.zip https://github.com/shengyanli1982/llmproxy/releases/download/v${VERSION}/llmproxyd-Linux-x64-${VERSION}.zip
    unzip llmproxyd-Linux-x64.zip
    sudo mv llmproxyd-Linux-x64 /usr/local/bin/llmproxyd
    sudo chmod +x /usr/local/bin/llmproxyd
    ```

2. **Create Configuration Directory and File**:

    ```bash
    sudo mkdir -p /etc/llmproxy
    sudo nano /etc/llmproxy/config.yaml
    # Paste your configuration content in the editor, and ensure sensitive information like API keys is properly handled
    # (e.g., future versions may support reading from environment variables)
    ```

3. **Create a Dedicated System User (Recommended)**:

    ```bash
    sudo useradd --system --no-create-home --shell /usr/sbin/nologin llmproxyuser
    sudo chown -R llmproxyuser:llmproxyuser /etc/llmproxy
    # If there's a log directory, it should also be authorized
    ```

4. **Create systemd Service File**:
   Copy the `examples/config/llmproxy.service` file to `/etc/systemd/system/llmproxy.service`.

    ```bash
    sudo cp examples/config/llmproxy.service /etc/systemd/system/llmproxy.service
    ```

    Edit `/etc/systemd/system/llmproxy.service` as needed, especially `User`, `Group` (if you created a dedicated user), and the configuration file path in `ExecStart` (`--config /etc/llmproxy/config.yaml`).

5. **Reload systemd, Start, and Enable Auto-start**:

    ```bash
    sudo systemctl daemon-reload
    sudo systemctl start llmproxy
    sudo systemctl enable llmproxy
    ```

6. **Check Service Status and Logs**:
    ```bash
    sudo systemctl status llmproxy
    sudo journalctl -u llmproxy -f # View real-time logs
    ```

### Security Best Practices (Common to All Deployment Methods)

1.  **API Key and Credential Management**:

    -   **Absolutely avoid** hardcoding API keys in plain text in configuration files.
    -   **Containerization/Kubernetes**: Prioritize using Secrets to manage credentials and inject them into LLMProxy containers via environment variables or file mounts.
    -   **System Service**: Restrict configuration file read permissions, consider using external configuration sources or environment variables (pending LLMProxy support).
    -   Regularly rotate all API keys.

2.  **Network Security**:

    -   **Admin Interface Security**: Bind the admin port (`admin.port`, default 9000) to the local loopback address (`127.0.0.1`) or an internal trusted network, never expose it to the public internet. Consider deploying a reverse proxy in front of it and adding authentication.
    -   **TLS/SSL Encryption**: For forwarding services facing the public internet, it's recommended to deploy a reverse proxy (such as Nginx, Traefik, Caddy, or cloud provider's load balancer) in front of LLMProxy to handle TLS termination and certificate management. LLMProxy itself focuses on proxy logic.
    -   Use firewalls to restrict unnecessary port access.

3.  **Principle of Least Privilege**:

    -   **Container**: Run the LLMProxy process as a non-root user (future version images will support this). Ensure the container filesystem is as read-only as possible.
    -   **System Service**: Use a dedicated, low-privilege system user to run the LLMProxy service.

4.  **Logging and Monitoring**:
    -   Configure a log collection solution to aggregate LLMProxy logs (including access logs and error logs) to a centralized log management system (such as ELK Stack, Grafana Loki, Splunk).
    -   Continuously monitor Prometheus metrics exposed through the `/metrics` endpoint, and set up alerts for critical metrics (such as error rates, latency, circuit breaker status).

## Understanding LLMProxy

### Architecture

LLMProxy adopts a modular, high-performance asynchronous architecture design. Core components include:

-   **HTTP Listeners (Forward Servers)**: Based on asynchronous HTTP servers, responsible for listening to client requests, with each `forwards` configuration item corresponding to an independent listening instance.
-   **Request Processor & Router**: Parses incoming requests and routes them to the specified upstream group according to configured routing rules.
-   **Upstream Group Manager**: Manages a group of logically related upstream LLM services, containing load balancers and HTTP client pools.
-   **Upstream Service Instances**: Represents a specific backend LLM API endpoint, including its URL, authentication information, circuit breaker configuration, etc.
-   **Load Balancer**: Embedded in each upstream group, intelligently distributes requests among healthy available upstream services according to the selected strategy (round-robin, weighted, random, response-time aware).
-   **HTTP Client**: Responsible for establishing connections with upstream LLM services and sending requests, supporting connection pooling, timeout control, retries, streaming, etc.
-   **Circuit Breaker**: Equipped for each upstream service instance, continuously monitors its health status, automatically breaks the circuit when persistent failures are detected to prevent failure propagation, and automatically retries when the service recovers.
-   **Metrics Collector**: Based on the Prometheus client, collects and exposes detailed performance and operational metrics in real-time.
-   **Configuration Manager**: Responsible for loading and parsing the `config.yaml` file and validating the configuration's validity.

![architecture](./images/architecture.png)
_Figure: LLMProxy core architecture diagram (simplified version)_

### Warm Restarts on Linux

To enhance service availability, LLMProxy leverages the `SO_REUSEPORT` socket option on `Linux` systems for both its forwarding and admin services. This feature allows multiple instances of LLMProxy to listen on the same port, enabling seamless, zero-downtime restarts and upgrades. When a new process starts, it can immediately begin accepting new connections on the shared port, while the old process completes any ongoing requests before gracefully shutting down(**There will be a very small amount of connection drops, but it can be ignored**). This mechanism prevents connection drops during deployments and significantly simplifies high-availability setups. Please note that this feature is specific to `Linux` and is not available on other operating systems like `Windows` or `macOS`.

### Response-Time Aware Load Balancing Algorithm

LLMProxy's response-time aware (`response_aware`) load balancing algorithm is an intelligent scheduling strategy designed specifically for large language models, which typically have high and variable response times and are computationally intensive. Unlike traditional round-robin or random strategies, this algorithm is specifically designed for services like LLMs with highly variable response times. It dynamically allocates new requests to the best service node by analyzing the comprehensive performance of upstream nodes in real-time (combining average response time, current concurrent load, and request success rate).

#### How It Works

1.  **Real-time Performance Sampling & Smoothing**: The system continuously collects and records key performance metrics for each upstream LLM service node:

    -   **Average Response Time**: Calculated using an exponentially weighted moving average algorithm, smoothing short-term fluctuations to better reflect recent performance trends.
    -   **In-flight Requests**: The number of concurrent requests currently being processed but not yet completed by the node.
    -   **Request Success Rate**: The percentage of requests successfully completed recently.

2.  **Dynamic Health & Comprehensive Scoring**: Combined with circuit breaker status, only healthy (non-broken) nodes are considered. For healthy nodes, a comprehensive performance score is calculated using a formula similar to the following, where a lower score indicates better node performance:

    $$\text{Score} = \text{ResponseTime} \times (\text{ProcessingRequests} + 1) \times \frac{1}{\text{SuccessRate}}$$

    Where:

    -   $\text{ResponseTime}$ is the node's average response time (milliseconds)
    -   $\text{ProcessingRequests}$ is the number of concurrent requests currently being processed by the node
    -   $\text{SuccessRate}$ is the node's request success rate (a value between 0-1)

![score](./images/response_aware_parameter_impact_en.png)
_Figure: Impact of various parameters on selection probability in the response-time aware algorithm (illustration)_

3.  **Intelligent Node Selection**:

    -   When a new request arrives, the load balancer traverses all healthy (non-broken) nodes in the current upstream group.
    -   Calculates the real-time performance score for each healthy node.
    -   Selects the node with the lowest score (i.e., best overall performance) to handle the current request.
    -   The in-flight request count for the selected node is incremented accordingly.

4.  **Continuous Adaptive Adjustment**:
    -   After a request is completed, its actual response time, success status, etc. are recorded.
    -   This information is used to update the node's average response time, success rate, and other statistics.
    -   The in-flight request count is decremented accordingly.
    -   This continuous feedback loop enables the algorithm to dynamically adapt to real-time changes in upstream LLM service performance.

#### Advantages

-   **Dynamic Adaptability**: Automatically adapts to real-time fluctuations and sudden load spikes in upstream LLM service performance without manual intervention.
-   **LLM Optimization**: Particularly suitable for handling the high latency and latency uncertainty of LLM requests (applicable from millisecond to minute-level responses).
-   **Multi-dimensional Consideration**: Comprehensively considers latency, concurrency, and success rate, avoiding concentrating traffic on a single slow node or overloaded node.
-   **Smooth Distribution**: Smoothing techniques like exponentially weighted moving averages avoid decision oscillations due to momentary jitters, providing more stable load distribution.
-   **High-concurrency Performance**: Algorithm design emphasizes efficiency, ensuring minimal overhead in high-concurrency scenarios.
-   **Fault Avoidance**: Tightly integrated with the circuit breaker mechanism, automatically excluding faulty or broken nodes.

#### Applicable Scenarios

This algorithm is particularly suitable for the following application scenarios:

-   **Large Language Model API Proxying**: Effectively handles the high latency and latency uncertainty of LLM requests (applicable from millisecond to minute-level responses).
-   **Heterogeneous Upstream Services**: When the upstream group contains LLM services with different performance characteristics and costs (e.g., mixing GPT-4 and GPT-3.5, or models from different vendors).
-   **Service Quality Sensitive Applications**: Scenarios with high requirements for LLM response latency.

#### Configuration Example

```yaml
upstream_groups:
    - name: "mixed_llm_services"
      upstreams:
          # The response-time aware strategy doesn't directly use the 'weight' parameter, it evaluates dynamically
          - name: "fast_but_expensive_llm"
          - name: "slower_but_cheaper_llm"
          - name: "another_llm_provider_model"
      balance:
          strategy: "response_aware" # Enable response-time aware load balancing
      http_client:
          # ... (recommended to configure appropriate timeout and retry for LLM requests)
          timeout:
              request: 300 # Adjust according to the expected time of the slowest model
```

### Circuit Breaker Mechanism

LLMProxy integrates a powerful circuit breaker pattern for each upstream service instance, aimed at enhancing the resilience and stability of the entire system and preventing local failures from propagating to upstream services or clients.

#### How It Works

The circuit breaker emulates the behavior of a fuse in an electrical circuit, following a three-state lifecycle model:

1.  **Closed State**:

    -   Initial and normal operating state. All requests directed to this upstream service are allowed through.
    -   LLMProxy continuously monitors the success and failure of requests sent to this upstream (typically based on HTTP status codes or connection errors).
    -   If the failure rate within the defined statistical window reaches the configured threshold (`breaker.threshold`), the circuit breaker transitions to the "Open" state.

2.  **Open State**:

    -   Circuit breaker is activated ("tripped"). At this point, LLMProxy will **immediately reject** all new requests directed to this failing upstream service without actually attempting to connect.
    -   This avoids unnecessary timeout waits, enables fast failure, and reduces pressure on the failing upstream, giving it time to recover.
    -   If this upstream service belongs to an upstream group, the load balancer will direct traffic to other healthy members in the group (if they exist).
    -   After the "Open" state persists for a preset cooling period (`breaker.cooldown`), the circuit breaker transitions to the "Half-Open" state.

3.  **Half-Open State**:
    -   The circuit breaker allows a small portion (typically a single) "probe" request through, attempting to connect to the previously failing upstream service.
    -   **If the probe request succeeds**: The system considers that the upstream service may have recovered. The circuit breaker resets and transitions back to the "Closed" state, resuming normal traffic.
    -   **If the probe request fails**: The system considers that the upstream service is still unstable. The circuit breaker returns to the "Open" state and starts a new cooling timer.

#### Coordination with Load Balancing

-   When an upstream service's circuit breaker is in the "Open" or "Half-Open" (after a failed probe) state, the load balancer treats it as an unavailable node and will not assign new user requests to it.
-   Intelligent failover: If an upstream group has multiple upstream services, and one or more are broken, the load balancer will automatically distribute traffic to the remaining healthy ("Closed" state) services.
-   Only when all upstream services in a group are unavailable (e.g., all broken) will requests to that group fail.

#### Advantages

-   **Fast Failure & Resource Protection**: Quickly identifies and isolates failing upstreams, preventing client requests from waiting for long periods or depleting proxy resources due to unresponsive upstreams.
-   **Preventing Cascading Failures**: By isolating problem services, prevents their failure pressure from propagating to other parts of the system or causing client retry storms.
-   **Automatic Recovery Detection**: No manual intervention needed; automatically probes whether upstream services have recovered and reintegrates them into service when they have.
-   **Fine-grained Configuration**: Can independently configure circuit breaking thresholds and cooldown times for each upstream service instance (`upstreams[].breaker`) to adapt to different service characteristics.
-   **Enhancing Overall System Resilience**: Enables the system to maintain partial functionality or gracefully degrade when faced with instability or failure in some backend LLM services.
-   **Observability**: Monitor circuit breaker status changes and behavior through Prometheus metrics for operational purposes.

#### Configuration Example

```yaml
upstreams:
    - name: "openai_service_main"
      url: "https://api.openai.com/v1"
      # ... auth config ...
      breaker:
          threshold: 0.5 # When the failure rate of requests to this upstream reaches 50% within the statistical window, the circuit breaker opens
          cooldown: 30 # After breaking, wait 30 seconds to enter the half-open state to attempt recovery

    - name: "critical_custom_llm_service"
      url: "http://my.custom.llm:8080"
      breaker:
          threshold: 0.3 # For more critical or less volatile services, a lower failure rate threshold can be set
          cooldown: 60 # If the service recovers more slowly, the cooling time can be appropriately extended
```

## API Endpoints

LLMProxy exposes the following main types of HTTP API endpoints:

### Forwarding Endpoints

-   **Path & Port**: Determined by the `port`, `address` in the `http_server.forwards[]` configuration and the original path of the client request.
    -   _Example_: If configured with `port: 3000`, `address: "0.0.0.0"`, then client requests to `http://<llmproxy_host>:3000/v1/chat/completions` will be handled by this forwarding service.
-   _Description_: These are the main working endpoints of LLMProxy. Client applications (such as your AI application backend) send standard LLM API requests (e.g., OpenAI, Anthropic format) to these endpoints.
-   _Protocol_: HTTP (LLMProxy itself does not currently handle HTTPS termination directly; it's recommended to use a reverse proxy like Nginx in front to handle TLS).
-   _Usage_: LLMProxy receives these requests, processes them according to the configured `upstream_group` (load balancing, authentication injection, header modification, circuit breaking, etc.), then forwards the request to the selected upstream LLM API service, and returns the upstream's response (including streaming responses) to the client.

### Admin Endpoints

These endpoints are defined by the `http_server.admin` configuration block, by default listening on `0.0.0.0:9000` (recommended to change to `127.0.0.1:9000` in production environments).

#### Standard Endpoints

-   **GET /health**

    -   _Description_: Provides basic health checking. Mainly used for automated systems (such as Kubernetes Liveness/Readiness Probes, load balancer health checks) to determine if the LLMProxy service process is running and able to respond to requests.
    -   _Returns_:
        -   `200 OK`: Indicates the service is basically normal.
        -   (Future versions may add more detailed health status, such as whether configuration loading was successful, etc.)
    -   _Content Type_: `text/plain` or `application/json`

-   **GET /metrics**

    -   _Description_: Exposes Prometheus format monitoring metrics. For Prometheus servers to scrape, used to monitor LLMProxy's performance, traffic, errors, upstream status, etc.
    -   _Returns_: Text format Prometheus metrics data.
    -   _Content Type_: `text/plain; version=0.0.4; charset=utf-8`

#### Configuration Management API

![openapi_ui](./images/openapi-ui.png)
_Figure: OpenAPI UI Example_

For enhanced operational visibility and easier debugging, the admin service provides a comprehensive Configuration Management API. This RESTful API allows you to inspect and modify the live, in-memory configuration of LLMProxy at any time without service restart, which is invaluable for auditing, troubleshooting, dynamic reconfiguration, and integration with automated operational workflows.

The API is versioned under the `/api/v1` path prefix. For security, access to these `api endpoints` can be protected by setting the `LLMPROXY_ADMIN_AUTH_TOKEN` environment variable, which enforces `Bearer Token` authentication.

**API Endpoints**

The API offers a structured set of endpoints to retrieve and modify all key configuration entities:

-   **Forwards**:
    -   `GET /api/v1/forwards`: Retrieves a list of all configured forward services.
    -   `GET /api/v1/forwards/{name}`: Fetches the details of a specific forward service.
-   **Upstream Groups**:
    -   `GET /api/v1/upstream-groups`: Lists all configured upstream groups.
    -   `GET /api/v1/upstream-groups/{name}`: Fetches the details of a specific group.
    -   `PATCH /api/v1/upstream-groups/{name}`: Updates the upstreams list of a specific group. This operation atomically replaces the entire upstreams list with the new one provided.
-   **Upstreams**:
    -   `GET /api/v1/upstreams`: Lists all configured upstream services.
    -   `GET /api/v1/upstreams/{name}`: Fetches the details of a specific upstream.
    -   `POST /api/v1/upstreams`: Creates a new upstream service.
    -   `PUT /api/v1/upstreams/{name}`: Updates an existing upstream service.
    -   `DELETE /api/v1/upstreams/{name}`: Deletes an upstream service (with dependency protection to prevent deletion if the service is referenced by any upstream group).

**Dynamic Configuration**

The dynamic configuration API enables hot-reloading of LLMProxy's configuration without service restart:

-   **Upstream Service Management**: Create, update, or delete upstream services on-the-fly.
-   **Upstream Group Management**: Reconfigure upstream groups by modifying their upstreams list.
-   **Dependency Protection**: Built-in safeguards prevent breaking changes, such as deleting an upstream that's currently in use.
-   **Configuration Consistency**: All modifications maintain the integrity of LLMProxy's configuration.

**Interactive OpenAPI UI**

To make exploring and interacting with the Configuration Management API as easy as possible, LLMProxy includes a built-in OpenAPI UI. This interactive interface provides comprehensive documentation for all endpoints and allows you to execute API calls directly from your browser.

-   **Access**: The OpenAPI UI is available at the `/api/v1/docs` path on the admin port. For security reasons, it is **only enabled** when LLMProxy is launched in debug mode (e.g., by using the `-d` or `--debug` command-line flag).

## Prometheus Metrics

LLMProxy exposes comprehensive Prometheus metrics through the admin endpoint's `/metrics` path for real-time monitoring of system performance, request handling, upstream service health status, and internal component states. These metrics are crucial for operations, troubleshooting, and capacity planning.

Below are the key metric categories and examples:

### HTTP Server & Request Metrics (for inbound requests from clients to LLMProxy)

-   `llmproxy_http_requests_total` (Counter)
    -   Description: Total number of HTTP requests received.
    -   Labels: `forward` (forwarding service name), `method` (HTTP method), `path` (request path).
-   `llmproxy_http_request_duration_seconds` (Histogram)
    -   Description: Latency distribution of HTTP request processing.
    -   Labels: `forward`, `method`, `path`.
-   `llmproxy_http_request_errors_total` (Counter)
    -   Description: Total number of errors that occurred while processing HTTP requests.
    -   Labels: `forward`, `error`, `status`.
-   `llmproxy_ratelimit_total` (Counter)
    -   Description: Total number of requests rejected due to rate limiting.
    -   Labels: `forward`.

### Upstream Client Metrics (for outbound requests from LLMProxy to backend LLM services)

-   `llmproxy_upstream_requests_total` (Counter)
    -   Description: Total number of requests sent to upstream LLM services.
    -   Labels: `group` (upstream group name), `upstream` (upstream service name).
-   `llmproxy_upstream_duration_seconds` (Histogram)
    -   Description: Latency distribution of sending requests to upstream LLM services and receiving responses.
    -   Labels: `group`, `upstream`.
-   `llmproxy_upstream_errors_total` (Counter)
    -   Description: Total number of errors that occurred when communicating with upstream LLM services.
    -   Labels: `error` (error type), `group`, `upstream`.

### Circuit Breaker Metrics

-   `llmproxy_circuitbreaker_state_changes_total` (Counter)
    -   Description: Total number of circuit breaker state transitions.
    -   Labels: `group`, `upstream`, `url`, `from` (original state), `to` (new state).
-   `llmproxy_circuitbreaker_calls_total` (Counter)
    -   Description: Total number of calls processed through the circuit breaker (including successful, failed, rejected ones).
    -   Labels: `group`, `upstream`, `url`, `result` (result type).

These metrics can be scraped by Prometheus and then visualized and configured for alerting using tools like Grafana, enabling comprehensive monitoring of the LLMProxy service and the LLM API calls it proxies.

## License

[MIT License](LICENSE)
