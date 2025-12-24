# Biosurf - Machine-Oriented Browser Documentation

## Overview

Biosurf is a machine-oriented browser/proxy system designed for automated web data acquisition and processing. Unlike traditional browsers, Biosurf is specifically optimized for APIs, crawlers, and AI agents rather than human users, providing efficient, deterministic, and machine-friendly web interaction capabilities.

## Features

### 1. Machine-HTTP Protocol Extensions

- **Priority Marking**: Supports three priority levels (Throughput, Latency, Accuracy) to optimize requests based on your needs
- **Field-Level Cache Directives**: Granular cache control for specific data fields with TTL and stale-while-revalidate support
- **Deterministic Execution**: Ensures reproducible request behavior across different environments

### 2. Custom DNS Resolver

- **UDP-Based Queries**: Efficient DNS resolution with support for A/AAAA/CNAME/NS/MX records
- **TTL-Based Caching**: Reduces redundant DNS queries by caching results
- **Thread-Safe Design**: Safe for concurrent use in multi-threaded applications

### 3. Connection Pool Manager

- **Async I/O with Tokio**: Efficient connection reuse using Tokio's async runtime
- **Semaphore-Controlled Limits**: Prevents connection overload with configurable maximum connections
- **Automatic Cleanup**: Periodic removal of idle connections to optimize resource usage
- **Health Checks**: Ensures connections are valid before reuse

### 4. Deterministic Execution Environment

- **Controlled Randomness**: Seed-based random number generation for reproducible results
- **Synchronized Timestamps**: Standardized timestamping across requests
- **Deterministic JS Interface**: Framework for deterministic JavaScript execution

### 5. DOM Snapshot and Diffing

- **Binary Serialization**: Efficient DOM snapshot storage with minimal overhead
- **Incremental Changes**: Detects and transmits only changes between DOM states
- **Structural Comparison**: Intelligent diffing algorithm that understands DOM structure

### 6. Session Management

- **Millions of Sessions**: Efficiently manages large numbers of concurrent sessions
- **State Compression**: Reduces memory usage with configurable compression levels
- **Resource Pooling**: Shares resources across sessions to optimize performance
- **Fast Recovery**: Session restoration from snapshots for quick resumption

## Getting Started

### Prerequisites

- Rust 1.56+ with Cargo
- Git (for cloning the repository)

### Installation

1. Clone the repository:
```bash
git clone https://github.com/Oasis-Company/Biosurf.git
cd Biosurf/biosurf
```

2. Build the project:
```bash
cargo build --release
```

3. Run the example:
```bash
cargo run
```

## Basic Usage

### Creating an HTTP Client

```rust
use biosurf::http_client::{HttpClient, HttpRequest, MachineHttpPriority};

// Create a new HTTP client
let client = HttpClient::new();

// Create a Machine-HTTP request with priority
let mut request = HttpRequest::new("GET", "/api/data");
request.set_machine_priority(MachineHttpPriority::Latency);
request.add_field_cache_directive("$.items[*].price", 3600, Some(300));
request.enable_deterministic_mode();

// Build the request
let request_str = request.build("example.com");
println!("{}", request_str);
```

### Using the DNS Resolver

```rust
use biosurf::dns::{DnsResolver, DnsRecordType};

// Create a DNS resolver
let mut resolver = DnsResolver::new("8.8.8.8").unwrap();

// Resolve a domain to IP addresses
let records = resolver.query("example.com", DnsRecordType::A).unwrap();
for record in records {
    println!("Record: {:?}", record);
}

// Resolve to specific IP address
let ip = resolver.resolve_ip("example.com").unwrap();
println!("IP: {:?}", ip);
```

### Using the Connection Pool

```rust
use biosurf::{http_client::HttpClient, dns::DnsResolver, connection_pool::ConnectionPool};

// Create HTTP client and DNS resolver
let client = HttpClient::new();
let resolver = DnsResolver::new("8.8.8.8").unwrap();

// Create connection pool
let pool = ConnectionPool::new(client, resolver);

// Get a connection
let mut guard = pool.get_connection("http", "example.com", 80).await.unwrap();

// Use the connection
let stream = guard.get_mut().unwrap();
// ... send requests and process responses ...

// Connection is automatically returned to the pool when guard goes out of scope
```

### Working with Sessions

```rust
use biosurf::session_manager::{SessionManager, SessionId};

// Create session manager for 1 million sessions
let session_manager = SessionManager::new(1_000_000, 60);

// Create a new session
let session = session_manager.create_session().unwrap();
println!("Session ID: {}", session.id.as_str());

// Add data to the session
{
    let mut state = session.get_mut_state();
    state.current_url = Some("https://example.com".to_string());
    state.headers.insert("User-Agent".to_string(), "Machine-HTTP/1.0".to_string());
}

// Compress session state to save memory
session.compress();

// Get session by ID
let retrieved_session = session_manager.get_session(&session.id).unwrap();
```

### DOM Snapshots

```rust
use biosurf::dom::{DomNode, DomSnapshot};

// Create a DOM tree
let mut root = DomNode::new_element("html");
let mut body = DomNode::new_element("body");
body.add_child(DomNode::new_text("Hello, Machine-HTTP!"));
root.add_child(body);

// Create a snapshot
let snapshot = DomSnapshot::new(root);
println!("Snapshot: {} nodes, {} bytes", snapshot.node_count, snapshot.size_in_bytes);

// Serialize snapshot to binary format
let mut buffer = Vec::new();
snapshot.serialize(&mut buffer).unwrap();

// Deserialize snapshot
let deserialized = DomSnapshot::deserialize(&mut &buffer[..]).unwrap();
```

## API Reference

### HttpClient

```rust
// Create a new client
fn new() -> Self

// Set connection timeout
fn set_timeout(&mut self, timeout: Duration) -> &mut Self

// Connect to HTTP server
fn connect_http<A: ToSocketAddrs>(&self, addr: A) -> Result<HttpStream>

// Connect to HTTPS server
fn connect_https<A: ToSocketAddrs>(&self, addr: A, domain: &str) -> Result<HttpStream>

// Send request
fn send_request(&self, stream: &mut HttpStream, request: &str) -> Result<()>

// Receive response
fn receive_response(&self, stream: &mut HttpStream) -> Result<String>

// Receive chunked response
fn receive_response_chunked(&self, stream: &mut HttpStream) -> Result<String>
```

### HttpRequest

```rust
// Create new request
fn new(method: &str, path: &str) -> Self

// Add header
fn add_header(&mut self, name: &str, value: &str) -> &mut Self

// Set request body
fn set_body(&mut self, body: &str) -> &mut Self

// Set Machine-HTTP priority
fn set_machine_priority(&mut self, priority: MachineHttpPriority) -> &mut Self

// Add field-level cache directive
fn add_field_cache_directive(&mut self, field_path: &str, ttl: u32, stale_while_revalidate: Option<u32>) -> &mut Self

// Enable deterministic mode
fn enable_deterministic_mode(&mut self) -> &mut Self

// Build request string
fn build(&self, host: &str) -> String
```

### DnsResolver

```rust
// Create new resolver
fn new(server: &str) -> Result<Self>

// Query specific record type
fn query(&mut self, domain: &str, record_type: DnsRecordType) -> Result<Vec<DnsRecord>>

// Resolve to IP address
fn resolve_ip(&mut self, domain: &str) -> Result<IpAddr>
```

### ConnectionPool

```rust
// Create new pool
fn new(http_client: HttpClient, dns_resolver: DnsResolver) -> Self

// Create with custom config
fn with_config(http_client: HttpClient, dns_resolver: DnsResolver, max_connections: usize, idle_timeout: Duration, connection_timeout: Duration) -> Self

// Get connection from pool
async fn get_connection(&self, scheme: &str, host: &str, port: u16) -> tokio::io::Result<ConnectionGuard>

// Cleanup idle connections
async fn cleanup(&self)

// Close all connections
async fn close_all_connections(&self)

// Get pool stats
async fn stats(&self) -> PoolStats
```

### SessionManager

```rust
// Create new session manager
fn new(max_sessions: usize, cleanup_interval: u64) -> Self

// Start periodic cleanup task
fn start_cleanup_task(self: Arc<Self>)

// Create new session
fn create_session(&self) -> Result<Arc<Session>, String>

// Get existing session
fn get_session(&self, session_id: &SessionId) -> Option<Arc<Session>>

// Remove session
fn remove_session(&self, session_id: &SessionId) -> bool

// Create session from snapshot
fn create_session_from_snapshot(&self, snapshot: SessionState) -> Result<Arc<Session>, String>
```

## Architecture

Biosurf follows a modular architecture with clear separation between components:

```
┌───────────────────────────────────────────────────────────────────┐
│                          Application Layer                         │
├───────────────┬───────────────┬───────────────┬─────────────────┤
│   HTTP Client │ DNS Resolver   │ Session Manager│ Connection Pool │
├───────────────┴───────────────┴───────────────┴─────────────────┤
│                          Core Services                            │
├───────────────┬───────────────┬───────────────┬─────────────────┤
│ Machine-HTTP  │ Deterministic │ DOM Processing │ Resource Pooling│
│ Extensions    │ Execution     │                │                 │
├───────────────┴───────────────┴───────────────┴─────────────────┤
│                         Transport Layer                          │
├───────────────┬───────────────┬───────────────┬─────────────────┤
│ TCP Sockets   │ TLS Encryption│ Async I/O      │ DNS Protocol    │
└───────────────┴───────────────┴───────────────┴─────────────────┘
```

## Performance Considerations

- **Connection Pooling**: Always use the connection pool for multiple requests to the same host to avoid TCP handshake overhead
- **Session Compression**: Enable compression for idle sessions to reduce memory usage
- **Priority Selection**: Choose appropriate priorities based on your use case:
  - **Throughput**: For high-volume data fetching
  - **Latency**: For time-sensitive requests
  - **Accuracy**: For critical data that must be up-to-date
- **Cache Directives**: Use field-level caching for frequently accessed data to reduce network requests

## Best Practices

1. **Use Connection Pools**: Always use the connection pool for HTTP requests to optimize performance
2. **Set Appropriate Timeouts**: Configure timeouts based on your network environment and requirements
3. **Enable Determinism for Testing**: Use deterministic mode for reproducible test results
4. **Monitor Pool Stats**: Regularly check connection pool statistics to optimize settings
5. **Implement Proper Session Cleanup**: Ensure sessions are properly closed when no longer needed
6. **Use Field-Level Caching**: Apply cache directives to frequently accessed data fields

## Example Use Cases

### 1. High-Performance API Crawler

```rust
use biosurf::{http_client::{HttpClient, HttpRequest, MachineHttpPriority}, connection_pool::ConnectionPool, dns::DnsResolver};

// Create components
let client = HttpClient::new();
let resolver = DnsResolver::new("8.8.8.8").unwrap();
let pool = ConnectionPool::new(client, resolver);

// Create high-throughput request
let mut request = HttpRequest::new("GET", "/api/items");
request.set_machine_priority(MachineHttpPriority::Throughput);
request.add_header("Accept", "application/json");

// Process multiple pages efficiently
for page in 1..100 {
    let mut page_request = request.clone();
    page_request.add_header("X-Page", &page.to_string());
    
    // Get connection from pool
    let mut conn = pool.get_connection("https", "api.example.com", 443).await.unwrap();
    
    // Send request and process response
    let request_str = page_request.build("api.example.com");
    client.send_request(conn.get_mut().unwrap(), &request_str).unwrap();
    let response = client.receive_response(conn.get_mut().unwrap()).unwrap();
    
    // Process response data...
}
```

### 2. Deterministic AI Agent

```rust
use biosurf::{http_client::{HttpClient, HttpRequest}, deterministic::DeterministicRng};

// Create components with deterministic settings
let client = HttpClient::new();
let mut rng = DeterministicRng::new(12345); // Fixed seed for reproducibility

// Create deterministic request
let mut request = HttpRequest::new("POST", "/api/analyze");
request.enable_deterministic_mode();
request.add_header("Content-Type", "application/json");

// Generate reproducible random parameters
let param1 = rng.next_f64();
let param2 = rng.next_u64() % 100;

// Build request body with deterministic parameters
let body = format!(r#"{{"param1": {}, "param2": {}}}"#, param1, param2);
request.set_body(&body);

// Send request
let conn = client.connect_http("api.example.com:80").unwrap();
let request_str = request.build("api.example.com");
client.send_request(&mut conn, &request_str).unwrap();
let response = client.receive_response(&mut conn).unwrap();

// Process response with deterministic logic...
```

## Troubleshooting

### Common Issues

1. **DNS Resolution Failed**: 
   - Check DNS server configuration
   - Verify network connectivity
   - Ensure domain exists and is resolvable

2. **Connection Pool Exhausted**: 
   - Increase max_connections setting
   - Check for connection leaks in your code
   - Enable connection pool statistics to monitor usage

3. **TLS Handshake Failed**: 
   - Verify domain name matches certificate
   - Check TLS version compatibility
   - Ensure proper CA certificates are installed

4. **Session Memory Usage High**: 
   - Enable session compression
   - Reduce max_sessions setting
   - Implement proper session cleanup

### Debugging Tips

- **Enable Debug Logging**: Use Rust's logging crate to enable debug logs
- **Monitor Connection Pool**: Check pool statistics regularly
- **Test with Deterministic Mode**: Use deterministic execution for reproducible debugging
- **Validate DNS Configuration**: Test DNS resolution separately if having issues
- **Check Response Headers**: Verify Machine-HTTP headers are being sent correctly

## Contributing

We welcome contributions to Biosurf! Here's how you can help:

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests with `cargo test`
5. Check for linting issues with `cargo clippy`
6. Submit a pull request

Please follow our [Code of Conduct](CODE_OF_CONDUCT.md) and ensure all tests pass before submitting.

## License

Biosurf is licensed under the GNU General Public License v3.0. See the [LICENSE](LICENSE) file for details.

## Authors

- ceaserzhao (zbbsdsb)

## Version History

- **v0.1.0**: Initial release with core functionality
  - Machine-HTTP protocol extensions
  - Custom DNS resolver
  - Connection pool manager
  - Deterministic execution support
  - DOM snapshot and diffing
  - Session management

## Roadmap

- **v0.2.0**: Enhanced Machine-HTTP support
  - More granular cache control
  - Additional priority levels
  - Request batching support

- **v0.3.0**: Advanced DOM processing
  - CSS selector support for DOM snapshots
  - XPath query support
  - Advanced diffing algorithms

- **v0.4.0**: Distributed architecture
  - Cluster support for large-scale deployment
  - Distributed session management
  - Load balancing capabilities

## Contact

For questions, issues, or feature requests, please:

- Open an issue on GitHub
- Contact the author at [ceaserzhao@example.com](mailto:ceaserzhao@example.com)
- Join our development community on Discord

## Acknowledgments

- Tokio for async runtime support
- native-tls for TLS encryption
- rand for random number generation
- The Rust community for their valuable contributions and feedback

---

Biosurf - Making the web more machine-friendly
