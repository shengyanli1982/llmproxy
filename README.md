<div align="center">
    <img src="./images/logo.png" alt="logo" width="650">
</div>

**A sophisticated load balancing system with intelligent scheduling that unifies diverse LLMs (Public/Private Cloud, vLLM, Ollama), enabling frictionless multi-cloud and hybrid-cloud integration with minimal client-side code modifications.**

## Introduction

**LLMProxy** is an enterprise-grade, fault-tolerant proxy service engineered specifically for large language model APIs. It intercepts client requests, routes them to upstream LLM API servers using configurable routing strategies, and delivers responses back to clients. This solution addresses quality and reliability challenges in LLM API access, provides granular traffic management, and significantly enhances the stability, performance, and efficiency of LLM API interactions.

### Why LLMProxy?

LLMProxy addresses critical challenges in enterprise LLM API deployments:

-   **High Availability** - Eliminates single points of failure through intelligent request distribution across multiple LLM providers
-   **Load Balancing** - Implements sophisticated load distribution strategies to optimize resource utilization and maintain performance under varying loads
-   **Horizontal Scalability** - Easily scales to handle growing request volumes by adding more upstream services without disrupting existing clients

## Key Features

-   🔄 **Advanced Request Forwarding**

    -   Configure multiple independent forwarding services through `http_server.forwards`
    -   Individually name and configure each forward service with precise control
    -   Define specific listening address and port configurations for each forward service
    -   Map each forward service to designated upstream groups with explicit routing rules

-   🌐 **Comprehensive Upstream Management**

    -   Define and orchestrate multiple backend LLM API services through `upstreams`
    -   Independently name, configure, and manage each upstream service
    -   Support for enterprise-grade authentication mechanisms:
        -   Bearer Token Authentication
        -   Basic Authentication
        -   No Authentication (default)
    -   Precise HTTP header manipulation:
        -   `insert`: Add headers when not present
        -   `remove`: Eliminate specified headers
        -   `replace`: Substitute existing headers or add when absent

-   ⚡ **Sophisticated Load Balancing**

    -   Organize upstreams into logical, functional groups with `upstream_groups`
    -   Support for multiple advanced load balancing strategies:
        -   **Round Robin (RR)** - Equitable distribution among servers
        -   **Weighted Round Robin (WRR)** - Prioritize servers by capacity and performance
        -   **Random** - Non-deterministic selection for enhanced security and privacy
    -   Configurable weighting for each upstream in weighted distribution strategies

-   🔁 **Granular Traffic Control**

    -   Implement rate limiting for each forward service with precise thresholds
    -   Configure requests-per-second limits tailored to service capacity
    -   Define burst allowances for effectively handling traffic spikes
    -   Deploy IP-based rate limiting to prevent abuse and ensure fair resource allocation

-   🔌 **Enterprise-Grade Connectivity**

    -   **Inbound Connection Management:** Configure precise timeouts for client connections
    -   **Outbound Connection & Request Optimization:**
        -   Customizable User-Agent identification
        -   TCP Keepalive configuration for connection stability
        -   Configurable connect, request, and idle timeouts
        -   Intelligent retry mechanisms with configurable attempts and backoff delays
        -   Optional HTTP/HTTPS proxy support for enhanced security and compliance

-   📊 **Comprehensive Monitoring & Management**

    -   Independent administrative interface through `http_server.admin`
    -   Health check endpoints for operational monitoring and alerting
    -   Prometheus metrics for detailed performance analytics

## Architecture

LLMProxy implements a modular, microservices-oriented architecture with the following key components:

-   **Forward Server**: HTTP listeners that receive and process client requests
-   **Upstream Manager**: Orchestrates communication with LLM API servers, including load balancing and authentication
-   **Load Balancer**: Intelligently distributes requests among available upstreams based on configured strategies
-   **Metrics Collector**: Gathers and exposes detailed performance and operational metrics

## Prometheus Metrics

LLMProxy provides comprehensive Prometheus metrics for monitoring performance, health, and operational status through the `/metrics` endpoint.

### Upstream Metrics

-   **llmproxy_upstream_requests_total** (counter) - Aggregate requests forwarded to upstream services, labeled by group and upstream
-   **llmproxy_upstream_duration_seconds** (histogram) - Request latency distribution for upstream services, labeled by group and upstream
-   **llmproxy_upstream_errors_total** (counter) - Aggregate errors encountered during upstream request processing, labeled by error type, group, and upstream

### HTTP Request Metrics

-   **llmproxy_http_requests_total** (counter) - Aggregate incoming HTTP requests received by the proxy, labeled by forward, method, and path
-   **llmproxy_http_request_duration_seconds** (histogram) - Request latency distribution for incoming HTTP requests, labeled by forward, method, and path
-   **llmproxy_http_request_errors_total** (counter) - Aggregate errors encountered during HTTP request processing, labeled by forward, error, and status

### Rate Limiting Metrics

-   **llmproxy_ratelimit_total** (counter) - Aggregate requests rejected due to rate limiting policies, labeled by forward

## API Endpoints

LLMProxy exposes the following HTTP API endpoints:

### Forward Endpoints

-   **Configurable HTTP endpoints**
    -   _Description_: Each forward service listens on its configured address and port
    -   _Protocol_: HTTP/HTTPS
    -   _Usage_: Client applications direct requests to these endpoints, which are then routed to the appropriate upstream LLM API

### Monitoring and Health Endpoints

-   **GET /health**

    -   _Description_: Health check endpoint for monitoring and alerting systems
    -   _Returns_: 200 OK when service is operational

-   **GET /metrics**
    -   _Description_: Prometheus metrics endpoint exposing comprehensive performance and operational statistics
    -   _Content Type_: text/plain

## Use Cases

LLMProxy is optimized for the following enterprise scenarios:

-   **Enterprise AI Integration**: Centralize LLM API access, enforce robust security policies, and implement sophisticated cost optimization strategies
-   **AI Application Development**: Streamline integration with multiple LLM providers and significantly enhance reliability
-   **Cloud Environments**: Deploy as a sidecar or standalone service providing unified LLM API access across multi-cloud and hybrid infrastructures

## Configuration

LLMProxy is configured using a structured YAML file. Below is a detailed explanation of the key configuration sections:

### HTTP Server Configuration

```yaml
http_server:
    # Forward services configuration
    forwards:
        - name: "to_mixgroup" # [Required] Name of the forward service
          port: 3000 # [Required] Port to listen on
          address: "0.0.0.0" # [Optional] Network address to bind (default: "0.0.0.0")
          upstream_group: "mixgroup" # [Required] Target upstream group for this forward
          ratelimit:
              enabled: true # Whether to enable rate limiting (default: false)
              per_second: 100 # Maximum requests per second from a single IP
              burst: 200 # Burst allowance for a single IP (must be >= per_second)
          timeout:
              connect: 10 # Client connection timeout in seconds

    # Admin interface configuration
    admin:
        port: 9000 # [Required] Port for admin interface
        address: "0.0.0.0" # [Optional] Network address to bind (default: "0.0.0.0")
        timeout:
            connect: 10 # Connection timeout in seconds
```

### Upstream Configuration

```yaml
upstreams:
    - name: "openai_primary" # [Required] Unique name for this upstream
      url: "https://api.openai.com/v1" # [Required] Base URL for the upstream API
      auth:
          type: "bearer" # Authentication type: "bearer", "basic", or "none" (default)
          token: "YOUR_API_KEY" # [Required for bearer auth] API key/token
          # username: "user"             # [Required for basic auth] Username
          # password: "pass"             # [Required for basic auth] Password
      headers:
          - op: "insert" # Operation: "insert", "replace", or "remove"
            key: "X-Custom-Header" # Header name to operate on
            value: "MyProxyValue" # Header value (for "insert" or "replace" operations)

    - name: "anthropic_primary"
      url: "https://api.anthropic.com"
      auth:
          type: "bearer"
          token: "YOUR_API_KEY"
      headers:
          - op: "insert"
            key: "x-api-key" # Some APIs may require keys in headers
            value: "YOUR_API_KEY"
```

### Upstream Group Configuration

```yaml
upstream_groups:
    - name: "mixgroup" # [Required] Unique name for this group
      upstreams: # [Required] List of upstream references
          - name: "openai_primary" # Must match an upstream name defined above
            weight: 8 # [Optional] Weight for weighted strategies (default: 1)
          - name: "anthropic_primary"
            weight: 2
      balance:
          strategy:
              "weighted_roundrobin" # Load balancing strategy: "roundrobin" (default),
              # "weighted_roundrobin", or "random"
      http_client:
          agent: "LLMProxy/1.0" # [Optional] User-Agent header (default: "LLMProxy/1.0")
          keepalive: 60 # [Optional] TCP keepalive in seconds (0-600, 0=disabled)
          stream_mode: true # [Optional] Enable streaming mode (default: true)
          timeout:
              connect: 10 # Connection timeout in seconds (default: 10)
              request: 300 # Request timeout in seconds (default: 300)
              idle: 60 # Idle connection timeout in seconds (default: 60)
          retry:
              enabled: true # Whether to enable retries (default: false)
              attempts: 3 # Maximum retry attempts
              initial: 500 # Initial retry delay in milliseconds
          proxy:
              enabled: false # Whether to use an outbound proxy (default: false)
              url: "http://user:pass@proxy:8080" # Proxy server URL
```

### Configuration Best Practices

1. **Security Considerations**:

    - For production environments, implement secure storage for configuration files containing sensitive API keys
    - Leverage environment variables or dedicated secret management services for credential management
    - Restrict admin interface access by binding to localhost (`127.0.0.1`) and implementing appropriate authentication

2. **Performance Optimization**:

    - Fine-tune timeout configurations based on specific LLM provider response characteristics
    - Enable `stream_mode: true` for efficient streaming of LLM responses
    - Implement appropriate rate limits to protect both proxy infrastructure and upstream services

3. **Reliability Enhancement**:
    - Enable intelligent retry mechanisms for idempotent requests
    - Implement weighted load balancing to prioritize more reliable or higher-capacity providers
    - Configure multiple upstreams in each group for redundancy and failover capabilities

For a comprehensive configuration reference detailing all available options, refer to the `config.default.yaml` file included with LLMProxy.

## License

[MIT License](LICENSE)
