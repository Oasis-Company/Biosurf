# biosurf

A machine-oriented browser/proxy system for automated web data acquisition and processing, designed specifically for APIs, crawlers, and AI agents rather than human users.

## Features

- **Custom HTTP Client**: Minimal HTTP/1.1 client implementation from scratch with TCP socket connection, manual HTTP request construction, response parsing (including chunked encoding), and HTTPS support via native-tls.
- **DNS Resolver**: Custom DNS resolution with UDP queries, response parsing, caching, and support for A/AAAA/CNAME/NS/MX records.
- **Connection Pool Manager**: Efficient TCP connection reuse with async I/O (Tokio), connection lifecycle management, health checks, and leak detection.

## Getting Started

### Prerequisites

- Rust programming language (1.56+)
- Cargo package manager

### Installation

```bash
git clone <repository-url>
cd biosurf
cargo build --release
```

### Usage

```bash
cargo run
```

## Architecture

- **http_client**: Implements HTTP/HTTPS client functionality with support for chunked encoding.
- **dns**: Handles DNS resolution with caching and UDP-based queries.
- **connection_pool**: Manages TCP connection reuse with async I/O and semaphore control.

## Authors

- ceaserzhao (zbbsdsb)

## License

This project is licensed under the GNU General Public License - see the [LICENSE](LICENSE) file for details.
