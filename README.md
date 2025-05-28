English | [中文](./README_CN.md)

<div align="center">
    <img src="./images/logo.png" alt="logo" width="650">
</div>

**LLMProxy: An enterprise-grade load balancing system with intelligent scheduling capabilities for unified management of Large Language Models (public/private cloud, vLLM, Ollama, etc.), enabling seamless integration across multi-cloud and hybrid cloud environments with minimal client-side modifications.**

<p align="center">
  <a href="#introduction">Introduction</a>
  |
  <a href="#core-features">Core Features</a>
  |
  <a href="#architecture">Architecture</a>
  |
  <a href="#prometheus-metrics">Prometheus Metrics</a>
  |
  <a href="#api-endpoints">API Endpoints</a>
  |
  <a href="#use-cases">Use Cases</a>
  |
  <a href="#response-time-aware-load-balancing-algorithm">Load Balancing Algorithm</a>
  |
  <a href="#circuit-breaker-mechanism">Circuit Breaker</a>
  |
  <a href="#configuration">Configuration</a>
  |
  <a href="#deployment-guide">Deployment</a>
</p>

## Introduction

**LLMProxy** is an enterprise-grade, high-availability proxy service specifically engineered for Large Language Model APIs. It intercepts client requests, intelligently routes them to upstream LLM API servers based on configurable strategies, and returns responses to clients. This solution effectively addresses quality and reliability challenges in LLM API interactions, providing granular traffic management that significantly enhances stability, performance, and efficiency.

### Why Choose LLMProxy?

LLMProxy effectively solves key challenges in enterprise-level LLM API deployments:

-   **High Availability** - Eliminates single points of failure through intelligent request distribution across multiple LLM providers
-   **Advanced Load Balancing** - Implements sophisticated load distribution strategies to optimize resource utilization and maintain consistent performance under varying load conditions
-   **Robust Fault Tolerance** - Employs circuit breaker patterns to detect failing upstream services in real-time, preventing cascade failures and automatically reconnecting when services recover
-   **Seamless Horizontal Scalability** - Easily accommodates growing request volumes by adding upstream services without disrupting existing clients

## Core Features

-   🔄 **Flexible Request Forwarding**

    -   Configure multiple independent forwarding services via `http_server.forwards`
    -   Define and manage each forwarding service with precise naming and configuration
    -   Customize dedicated listening addresses and ports per service
    -   Map services to specific upstream groups using explicit routing rules

-   🌐 **Comprehensive Upstream Management**

    -   Orchestrate multiple backend LLM API services through the `upstreams` configuration
    -   Support enterprise-grade authentication including bearer token, basic authentication, and unauthenticated access
    -   Precisely control HTTP headers with granular operations:
        -   `insert`: Add headers when they don't exist
        -   `remove`: Delete specified headers
        -   `replace`: Replace existing headers or add if missing

-   ⚡ **Sophisticated Load Balancing**

    -   Organize upstream services into logical groups with `upstream_groups`
    -   Choose from multiple advanced load balancing strategies:
        -   **Round Robin (RR)**: Evenly distribute requests across servers
        -   **Weighted Round Robin (WRR)**: Prioritize servers based on capacity and performance
        -   **Random**: Non-deterministic selection for enhanced security
        -   **Response Time Aware**: Dynamically route to faster, less loaded servers
    -   Configure precise weights for each upstream in weighted strategies

-   🔁 **Granular Traffic Control**

    -   Implement precise rate limiting per forwarding service
    -   Configure requests-per-second thresholds based on service capacity
    -   Set burst capacity parameters for handling traffic spikes
    -   Apply IP-based rate limiting to prevent abuse and ensure fair resource allocation

-   🔌 **Enterprise-grade Connection Management**

    -   **Client connections**: Configure precise timeout parameters
    -   **Upstream connections**:
        -   Customize User-Agent identification
        -   Fine-tune TCP keepalive settings
        -   Adjust connection, request, and idle timeouts
        -   Configure intelligent retry mechanisms with backoff capabilities
        -   Enable HTTP/HTTPS proxy support for enhanced security

-   🛡️ **Advanced Fault Tolerance**

    -   Implement automatic detection and isolation of failing upstream services
    -   Quickly identify problematic services to minimize latency impacts
    -   Automatically test and reconnect to recovered services
    -   Configure precise failure detection thresholds per upstream service
    -   Prevent cascade failures through effective failure isolation
    -   Automatically redirect traffic to healthy services when failures occur

-   📊 **Comprehensive Monitoring**

    -   Access management interface via `http_server.admin`
    -   Monitor system health with dedicated health check endpoints
    -   Analyze performance with detailed Prometheus metrics

## Architecture

LLMProxy implements a modular, microservice-oriented architecture consisting of these core components:

-   **Forwarding Server**: Highly efficient HTTP listeners that receive and process client requests, handling initial request validation and routing logic
-   **Upstream Manager**: Sophisticated service that coordinates communication with LLM API providers, managing authentication, load balancing, and connection pooling
-   **Load Balancer**: Intelligent request distribution engine that routes traffic according to configured strategies (round-robin, weighted round-robin, random, or response-time aware), continuously adapting to upstream performance metrics
-   **Circuit Breaker**: Fault tolerance mechanism that monitors upstream health, detects failing services, isolates problematic endpoints to prevent cascading failures, and automatically recovers connections when services stabilize
-   **Metrics Collector**: Comprehensive monitoring system that captures and exposes detailed performance and operational metrics, supporting real-time system visibility and proactive issue resolution

![architecture](./images/architecture.png)

## Prometheus Metrics

LLMProxy provides comprehensive Prometheus metrics through the `/metrics` endpoint, enabling real-time monitoring of system performance, health status, and operational indicators.

### Upstream Metrics

-   **llmproxy_upstream_requests_total** (counter) - Total forwarded requests segmented by group and upstream
-   **llmproxy_upstream_duration_seconds** (histogram) - Request latency distribution segmented by group and upstream
-   **llmproxy_upstream_errors_total** (counter) - Error counts segmented by error type, group, and upstream

### HTTP Request Metrics

-   **llmproxy_http_requests_total** (counter) - Total inbound HTTP requests segmented by forwarding service, method, and path
-   **llmproxy_http_request_duration_seconds** (histogram) - Request latency distribution segmented by forwarding service, method, and path
-   **llmproxy_http_request_errors_total** (counter) - Error counts segmented by forwarding service, error type, and status code

### Rate Limit Metrics

-   **llmproxy_ratelimit_total** (counter) - Requests rejected due to rate limiting, segmented by forwarding service

### Circuit Breaker Metrics

-   **llmproxy_circuitbreaker_state_changes_total** (counter) - Circuit breaker state transitions segmented by group, upstream, URL, and state
-   **llmproxy_circuitbreaker_calls_total** (counter) - Circuit breaker processed calls segmented by group, upstream, URL, and result
-   **llmproxy_circuitbreaker_opened_total** (counter) - Circuit breaker open events segmented by group, upstream, and URL
-   **llmproxy_circuitbreaker_closed_total** (counter) - Circuit breaker close events segmented by group, upstream, and URL
-   **llmproxy_circuitbreaker_half_opened_total** (counter) - Circuit breaker half-open transitions segmented by group, upstream, and URL

## API Endpoints

LLMProxy exposes the following HTTP API endpoints:

### Forwarding Endpoints

-   **Configurable HTTP Endpoints**
    -   _Description_: Each forwarding service listens for incoming requests at its configured address and port
    -   _Protocol_: HTTP/HTTPS
    -   _Purpose_: Client applications direct requests to these endpoints, which the system intelligently routes to corresponding upstream LLM providers

### Management Endpoints

-   **GET /health**

    -   _Description_: Health check endpoint for monitoring systems and alerting infrastructure
    -   _Returns_: 200 OK status when the system is operating normally

-   **GET /metrics**
    -   _Description_: Prometheus metrics endpoint providing comprehensive performance and operational telemetry
    -   _Content Type_: text/plain

## Use Cases

LLMProxy is strategically designed for the following enterprise applications:

-   **Enterprise AI Integration**: Centrally manage LLM provider access, implement robust security policies, and optimize cost efficiency
-   **AI Application Development**: Dramatically simplify integration with multiple LLM providers while improving system reliability and fault tolerance
-   **Multi-cloud Strategy**: Deploy as a sidecar or standalone service to provide a unified LLM API access layer across diverse cloud environments and on-premises infrastructure

## Response Time Aware Load Balancing Algorithm

LLMProxy's response time aware load balancing algorithm is an advanced intelligent scheduling strategy engineered specifically for high-latency, compute-intensive services like Large Language Models. Unlike traditional algorithms, this approach continuously monitors real-time performance metrics of upstream services and dynamically routes traffic to optimal service nodes.

### How It Works

1. **Real-time Performance Monitoring**: The system continuously collects and analyzes key performance indicators for each upstream node:

    - **Average Response Time**: Historical response times processed using exponential moving average (EMA) smoothing for stability
    - **Current Load**: Real-time count of concurrent in-flight requests being processed
    - **Success Rate**: Statistical percentage of successfully completed requests

2. **Comprehensive Performance Scoring**: A holistic performance score is calculated for each node using the following formula, where lower scores indicate better performance:

    $$\text{Score} = \text{ResponseTime} \times (\text{ProcessingRequests} + 1) \times \frac{1}{\text{SuccessRate}}$$

    Where:

    - $\text{ResponseTime}$ represents the node's average response time in milliseconds
    - $\text{ProcessingRequests}$ represents the number of currently in-flight requests
    - $\text{SuccessRate}$ represents the node's request success rate (0.0-1.0)

![score](./images/response_aware_parameter_impact_en.png)

3. **Intelligent Node Selection**:

    - Starting from the current polling position, the algorithm evaluates all healthy upstream nodes
    - It calculates the comprehensive performance score for each available node
    - The node with the lowest score (indicating optimal performance) is selected for the current request
    - The system increments the processing request count for the selected node

4. **Continuous Adaptive Optimization**:
    - After request completion, the system records the actual response time
    - The node's average response time is updated using a configurable smoothing factor (default: 0.15)
    - The processing request count is decremented
    - Success rate statistics are updated based on the request outcome

### Advantages

-   **Dynamic Adaptation**: Automatically responds to performance fluctuations without manual intervention
-   **Multi-dimensional Analysis**: Considers both latency and concurrent load, preventing any single node from becoming overloaded
-   **Statistical Stability**: Employs exponential moving averages to smooth transient fluctuations for consistent traffic distribution
-   **High-Performance Design**: Utilizes lock-free architecture and atomic operations for efficient operation in high-concurrency environments
-   **Integrated Fault Management**: Seamlessly works with circuit breaker mechanisms to bypass unhealthy nodes

### Optimal Use Cases

This algorithm delivers exceptional value in these scenarios:

-   **LLM API Proxying**: Managing requests with latencies ranging from hundreds of milliseconds to tens of seconds
-   **Heterogeneous Infrastructure**: Environments with varying server capabilities, capacities, or network conditions
-   **Dynamic Workloads**: Applications with significant load variations throughout operational cycles
-   **Mission-Critical Systems**: Enterprise deployments with stringent SLA requirements for response time and availability

### Configuration Example

```yaml
upstream_groups:
    - name: "llm_services"
      upstreams:
          # Note: Response time aware strategy does not utilize weight values
          - name: "openai_service"
          - name: "anthropic_service"
      balance:
          strategy: "response_aware" # Enable response time aware load balancing
```

## Circuit Breaker Mechanism

LLMProxy integrates a sophisticated Circuit Breaker pattern to enhance system resilience and operational stability. This mechanism automatically detects failing upstream services, rapidly "trips" connections to avoid resource depletion and request backlogs, and intelligently reconnects when services recover.

### How It Works

The circuit breaker implements a three-state lifecycle model:

1. **Closed State (Normal Operation)**:

    - Default operational state where all requests are forwarded to upstream services
    - Continuously monitors request success/failure patterns against configurable thresholds
    - Transitions to Open state when the failure rate exceeds the defined threshold (e.g., 50%)

2. **Open State (Protection Mode)**:

    - Circuit breaker activates, immediately rejecting requests without attempting to forward to the failing upstream
    - Implements fast-fail responses, eliminating wait times and conserving system resources
    - Automatically transitions to Half-Open state after a configurable cooldown period (e.g., 30 seconds)

3. **Half-Open State (Recovery Assessment)**:
    - Allows a limited number of test requests to evaluate if the upstream service has recovered
    - Upon successful test requests, transitions back to Closed state, resuming normal operations
    - If test requests continue to fail, reverts to Open state, maintaining protection mechanisms

### Intelligent Failover

The circuit breaker integrates seamlessly with the load balancer to provide sophisticated failover capabilities:

-   When an upstream service trips, traffic is automatically redistributed to healthy services within the same group
-   Error responses are returned to clients only when all upstream services in a group become unavailable
-   Circuit breaker state information is transparently incorporated into load balancing decisions, ensuring optimal routing

### Key Benefits

-   **Rapid Failure Detection**: Identifies unavailable services immediately, eliminating timeout-related delays
-   **Resource Conservation**: Prevents requests to known failing services from consuming critical system resources
-   **Failure Isolation**: Contains service failures, preventing them from cascading throughout the system
-   **Automatic Recovery**: Detects service restoration and seamlessly reintegrates recovered services
-   **Configurable Behavior**: Offers granular configuration options for each upstream service
-   **Comprehensive Monitoring**: Provides detailed metrics on circuit breaker states and transitions

### Configuration Example

```yaml
upstreams:
    - name: "openai_service"
      url: "https://api.openai.com/v1"
      breaker:
          threshold: 0.5 # Failure rate threshold triggering circuit breaking (50%)
          cooldown: 30 # Recovery assessment delay in seconds (1-3600, default: 30)

    - name: "anthropic_service"
      url: "https://api.anthropic.com"
      breaker:
          threshold: 0.3 # Lower threshold for critical services (30%)
          cooldown: 60 # Extended cooldown for services with longer recovery patterns
```

## Configuration

LLMProxy utilizes structured YAML configuration files that provide comprehensive and flexible customization options. Below is a detailed explanation of key configuration sections:

### Configuration Options in Detail

#### HTTP Server Configuration Options

| Configuration Item                            | Type    | Default   | Description                                                                       |
| --------------------------------------------- | ------- | --------- | --------------------------------------------------------------------------------- |
| `http_server.forwards[].name`                 | String  | -         | **[Required]** Unique identifier for the forwarding service                       |
| `http_server.forwards[].port`                 | Integer | 3000      | **[Required]** Listening port for the forwarding service                          |
| `http_server.forwards[].address`              | String  | "0.0.0.0" | Network interface binding address for the forwarding service                      |
| `http_server.forwards[].upstream_group`       | String  | -         | **[Required]** Name of the upstream group associated with this forwarding service |
| `http_server.forwards[].ratelimit.enabled`    | Boolean | false     | Enable/disable rate limiting functionality                                        |
| `http_server.forwards[].ratelimit.per_second` | Integer | 100       | Maximum requests per second allowed per IP address                                |
| `http_server.forwards[].ratelimit.burst`      | Integer | 200       | Burst capacity allowed per IP address (buffer size)                               |
| `http_server.forwards[].timeout.connect`      | Integer | 10        | Client connection timeout to LLMProxy in seconds                                  |
| `http_server.admin.port`                      | Integer | 9000      | Listening port for the administrative interface                                   |
| `http_server.admin.address`                   | String  | "0.0.0.0" | Network interface binding address for the administrative interface                |
| `http_server.admin.timeout.connect`           | Integer | 10        | Connection timeout to the administrative interface in seconds                     |

#### Upstream Service Configuration Options

| Configuration Item              | Type    | Default | Description                                                                           |
| ------------------------------- | ------- | ------- | ------------------------------------------------------------------------------------- |
| `upstreams[].name`              | String  | -       | **[Required]** Unique identifier for the upstream service                             |
| `upstreams[].url`               | String  | -       | **[Required]** Base URL of the upstream service endpoint                              |
| `upstreams[].auth.type`         | String  | "none"  | Authentication method: `bearer`, `basic`, or `none`                                   |
| `upstreams[].auth.token`        | String  | -       | API key or access token when `type` is `bearer`                                       |
| `upstreams[].auth.username`     | String  | -       | Username credential when `type` is `basic`                                            |
| `upstreams[].auth.password`     | String  | -       | Password credential when `type` is `basic`                                            |
| `upstreams[].headers[].op`      | String  | -       | HTTP header operation type: `insert`, `replace`, or `remove`                          |
| `upstreams[].headers[].key`     | String  | -       | Target HTTP header name                                                               |
| `upstreams[].headers[].value`   | String  | -       | Header value for `insert` or `replace` operations                                     |
| `upstreams[].breaker.threshold` | Float   | 0.5     | Circuit breaker activation threshold as failure rate ratio (0.01-1.0)                 |
| `upstreams[].breaker.cooldown`  | Integer | 30      | Circuit breaker recovery assessment period in seconds before entering half-open state |

#### Upstream Group Configuration Options

> [!NOTE]
>
> The parameter `upstreams[].url` must be configured with the complete base URL of the upstream service, for example: `https://api.openai.com/v1`, not `https://api.openai.com` or `https://api.openai.com/v1/chat/completions`.

| Configuration Item                              | Type    | Default        | Description                                                                                      |
| ----------------------------------------------- | ------- | -------------- | ------------------------------------------------------------------------------------------------ |
| `upstream_groups[].name`                        | String  | -              | **[Required]** Unique identifier for the upstream group                                          |
| `upstream_groups[].upstreams[].name`            | String  | -              | **[Required]** Referenced upstream service name (must match an entry in the `upstreams` section) |
| `upstream_groups[].upstreams[].weight`          | Integer | 1              | Service weight factor (only applies when `balance.strategy` is set to `weighted_roundrobin`)     |
| `upstream_groups[].balance.strategy`            | String  | "roundrobin"   | Load balancing algorithm: `roundrobin`, `weighted_roundrobin`, `random`, or `response_aware`     |
| `upstream_groups[].http_client.agent`           | String  | "LLMProxy/1.0" | User-Agent header value sent to upstream services                                                |
| `upstream_groups[].http_client.keepalive`       | Integer | 60             | TCP Keepalive duration in seconds (range: 0-600, 0 disables keepalive)                           |
| `upstream_groups[].http_client.stream`          | Boolean | true           | Enable/disable streaming mode (critical for LLM API streaming responses)                         |
| `upstream_groups[].http_client.timeout.connect` | Integer | 10             | Connection establishment timeout to upstream services in seconds                                 |
| `upstream_groups[].http_client.timeout.request` | Integer | 300            | Total request timeout in seconds (maximum time from request initiation to complete response)     |
| `upstream_groups[].http_client.timeout.idle`    | Integer | 60             | Idle connection timeout in seconds (time after which inactive connections are closed)            |
| `upstream_groups[].http_client.retry.enabled`   | Boolean | false          | Enable/disable automatic request retry functionality                                             |
| `upstream_groups[].http_client.retry.attempts`  | Integer | 3              | Maximum retry attempts (excluding the initial request)                                           |
| `upstream_groups[].http_client.retry.initial`   | Integer | 500            | Initial backoff delay before first retry attempt in milliseconds                                 |
| `upstream_groups[].http_client.proxy.enabled`   | Boolean | false          | Enable/disable outbound proxy usage                                                              |
| `upstream_groups[].http_client.proxy.url`       | String  | -              | Proxy server URL with optional authentication                                                    |

### HTTP Server Configuration

```yaml
http_server:
    # Forwarding service configuration
    forwards:
        - name: "to_mixgroup" # [Required] Unique service identifier
          port: 3000 # [Required] Listening port
          address: "0.0.0.0" # [Optional] Binding address (default: "0.0.0.0")
          upstream_group: "mixgroup" # [Required] Target upstream group
          ratelimit:
              enabled: true # Enable rate limiting (default: false)
              per_second: 100 # Requests/second limit per IP
              burst: 200 # Burst capacity per IP (must be >= per_second)
          timeout:
              connect: 10 # Client connection timeout in seconds

    # Admin interface configuration
    admin:
        port: 9000 # [Required] Admin interface port
        address: "0.0.0.0" # [Optional] Binding address (default: "0.0.0.0")
        timeout:
            connect: 10 # Connection timeout in seconds
```

### Upstream Service Configuration

```yaml
upstreams:
    - name: "openai_primary" # [Required] Unique service identifier
      url: "https://api.openai.com/v1" # [Required] Base URL of the API endpoint
      auth:
          type: "bearer" # Authentication type: "bearer", "basic", or "none" (default)
          token: "YOUR_API_KEY" # [Required for bearer auth] API key/token
          # username: "user"           # [Required for basic auth] Username
          # password: "pass"           # [Required for basic auth] Password
      headers:
          - op: "insert" # Operation: "insert", "replace", or "remove"
            key: "X-Custom-Header" # Target HTTP header name
            value: "MyProxyValue" # Header value (for "insert"/"replace" operations)
      breaker: # [Optional] Circuit breaker configuration
          threshold: 0.5 # Failure threshold (0.01-1.0, default: 0.5)
          cooldown: 30 # Recovery period in seconds (1-3600, default: 30)

    - name: "anthropic_primary"
      url: "https://api.anthropic.com"
      auth:
          type: "bearer"
          token: "YOUR_API_KEY"
      headers:
          - op: "insert"
            key: "x-api-key" # Provider-specific header requirements
            value: "YOUR_API_KEY"
```

### Upstream Group Configuration

```yaml
upstream_groups:
    - name: "mixgroup" # [Required] Unique group identifier
      upstreams: # [Required] List of upstream service references
          - name: "openai_primary" # Must match a defined upstream service name
            weight: 8 # [Optional] Used only in weighted_roundrobin strategy
          - name: "anthropic_primary"
            weight: 2 # Used only in weighted_roundrobin strategy
      balance:
          strategy:
              "weighted_roundrobin" # Load balancing algorithm:
              # "roundrobin" (default), "weighted_roundrobin",
              # "random", or "response_aware"
      http_client:
          agent: "LLMProxy/1.0" # [Optional] Custom User-Agent (default: "LLMProxy/1.0")
          keepalive: 60 # [Optional] TCP keepalive in seconds (0-600, 0=disabled)
          stream: true # [Optional] Enable streaming support (default: true)
          timeout:
              connect: 10 # Connection timeout in seconds (default: 10)
              request: 300 # Request timeout in seconds (default: 300)
              idle: 60 # Idle connection timeout in seconds (default: 60)
          retry:
              enabled: true # Enable request retries (default: false)
              attempts: 3 # Maximum retry attempts
              initial: 500 # Initial retry delay in milliseconds
          proxy:
              enabled: false # Use outbound proxy (default: false)
              url: "http://user:pass@proxy:8080" # Proxy server URL with optional auth
```

### Configuration Best Practices

1. **Security Hardening**:

    - In production environments, implement secure storage solutions for sensitive API credentials
    - Leverage environment variables or dedicated key management services (Vault, AWS Secrets Manager, etc.) for credential handling
    - Restrict admin interface access by binding to localhost (`127.0.0.1`) and implementing appropriate authentication

2. **Performance Optimization**:

    - Fine-tune timeout parameters based on the specific characteristics of each LLM provider
    - For streaming-capable LLM services, ensure `stream: true` is enabled to optimize response handling
    - Configure appropriate rate limits that balance system protection with service capacity

3. **Reliability Engineering**:
    - Enable intelligent retry mechanisms for idempotent operations to maximize success rates
    - Implement weighted load balancing to prioritize more reliable or higher-capacity providers
    - Configure redundant service providers in each upstream group for automatic failover capability
    - Set appropriate circuit breaker thresholds tailored to each upstream service's stability profile
    - Configure recovery periods based on observed service restoration patterns
    - Implement proactive monitoring of circuit breaker metrics to identify recurring stability issues
    - Utilize response time aware load balancing for high-latency applications like LLM interactions
    - Leverage the response time aware algorithm to automatically optimize traffic distribution based on real-time performance metrics

For comprehensive documentation of all available configuration options, refer to the `config.default.yaml` file included with the LLMProxy project.

## Deployment Guide

LLMProxy supports multiple deployment methodologies, including containerized Docker deployments, Kubernetes orchestration, and traditional Linux system services. Detailed instructions for each approach are provided below:

### Docker Deployment

Docker Compose provides a streamlined approach to deploying LLMProxy. Complete configuration examples are available in the project's `examples/config` directory.

1. **Prepare Configuration**:

    Place your customized `config.yaml` file in the same directory as your `docker-compose.yaml` file.

2. **Launch Services**:

    ```bash
    docker-compose up -d
    ```

3. **Monitor Logs**:

    ```bash
    docker-compose logs -f
    ```

4. **Terminate Services**:

    ```bash
    docker-compose down
    ```

Docker Compose Configuration Example:

```yaml
version: "3"

services:
    llmproxy:
        image: shengyanli1982/llmproxy:latest
        container_name: llmproxy
        restart: unless-stopped
        ports:
            # Forwarding service port mappings
            - "3000:3000" # to_mixgroup
            - "3001:3001" # openai_group
            # Admin interface port mapping
            - "9000:9000" # admin
        volumes:
            - ./config.yaml:/app/config.yaml:ro
        command: ["--config", "/app/config.yaml"]
        environment:
            - TZ=UTC
        networks:
            - llmproxy-network

networks:
    llmproxy-network:
        driver: bridge
```

### Kubernetes Deployment

For Kubernetes environments, complete deployment manifests are provided in the `examples/config/kubernetes` directory.

1. **Create Resources**:

    ```bash
    # Set API key environment variables for configuration rendering
    export OPENAI_API_KEY="your_openai_api_key"
    export ANTHROPIC_API_KEY="your_anthropic_api_key"

    # Navigate to Kubernetes configuration directory
    cd examples/config/kubernetes

    # Apply resources in sequence
    kubectl apply -f namespace.yaml
    kubectl apply -f configmap.yaml
    kubectl apply -f service.yaml
    kubectl apply -f deployment.yaml
    ```

2. **Verify Deployment**:

    ```bash
    kubectl get pods -n llmproxy
    kubectl get services -n llmproxy
    ```

3. **Access Services**:

    Internal cluster access (using DNS service discovery):

    ```
    http://llmproxy.llmproxy.svc.cluster.local:3000
    ```

    External access (via Ingress or port forwarding):

    ```bash
    kubectl port-forward svc/llmproxy -n llmproxy 3000:3000 3001:3001 9000:9000
    ```

### Linux System Service

For traditional Linux environments, standard systemd service configurations are provided.

1. **Install Binary**:

    ```bash
    # Download latest release
    curl -L -o llmproxyd.zip https://github.com/shengyanli1982/llmproxy/releases/latest/download/llmproxyd-Linux-x64-<version>.zip

    # Extract package
    unzip llmproxyd.zip

    # Set executable permissions
    chmod +x llmproxyd

    # Install to system directory
    sudo mkdir -p /opt/llmproxy
    sudo mv llmproxyd /opt/llmproxy/
    ```

2. **Configure Service**:

    ```bash
    sudo mkdir -p /opt/llmproxy
    sudo nano /opt/llmproxy/config.yaml
    # Insert your configuration
    ```

3. **Create Service Account**:

    ```bash
    sudo useradd -r -s /bin/false llmproxy
    sudo chown -R llmproxy:llmproxy /opt/llmproxy
    ```

4. **Install Service Definition**:

    ```bash
    sudo cp examples/config/llmproxy.service /etc/systemd/system/
    sudo systemctl daemon-reload
    ```

5. **Enable and Start Service**:

    ```bash
    sudo systemctl start llmproxy
    sudo systemctl enable llmproxy
    ```

6. **Verify Operation**:

    ```bash
    sudo systemctl status llmproxy
    ```

### Security Best Practices

Regardless of deployment methodology, implement these security controls:

1. **Credential Protection**:

    - Never hardcode API keys directly in configuration files
    - Utilize environment variables, secrets management solutions, or Kubernetes Secrets
    - Implement regular credential rotation policies

2. **Network Security**:

    - Restrict admin interface access (port 9000) to trusted networks only
    - Consider implementing a reverse proxy (Nginx, Traefik) with TLS termination and authentication
    - Apply network segmentation to isolate components with different security requirements

3. **Least Privilege Principle**:

    - Run services with dedicated non-privileged accounts
    - Apply strict filesystem access controls
    - Implement container security best practices including non-root execution and read-only filesystems

4. **Operational Monitoring**:
    - Implement centralized logging with structured analysis capabilities
    - Configure alerting based on Prometheus metrics
    - Deploy anomaly detection to identify potential security incidents or performance degradation

## License

[MIT License](LICENSE)
