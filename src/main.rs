// Module declarations - declare Rust modules from separate files
// Each module corresponds to a different component of the Biosurf browser
mod http_client;        // Custom HTTP client with machine-specific extensions
mod dns;                // DNS resolution functionality
mod connection_pool;    // Connection pooling for network efficiency
mod deterministic;      // Deterministic execution utilities
mod dom;                // Document Object Model handling
mod session_manager;    // Browser session management

// Entry point of the application
// `#[tokio::main]` macro configures the Tokio runtime for async operations
#[tokio::main]
async fn main() {
    // Print application banner
    println!("Biosurf - Machine Browser");
    
    // =========================================================================
    // Example 1: Machine-HTTP Client with priority headers
    // Demonstrates custom HTTP headers for machine-to-machine communication
    // =========================================================================
    println!("\n1. Testing Machine-HTTP Client:");
    
    // Create a new HTTP client instance
    let http_client = http_client::HttpClient::new();
    
    // Create a Machine-HTTP request with custom extensions
    // This request includes machine-specific optimizations
    let mut request = http_client::HttpRequest::new("GET", "/api/data");
    
    // Set machine priority for latency-sensitive operations
    request.set_machine_priority(http_client::MachineHttpPriority::Latency);
    
    // Add field cache directive: cache price data for 1 hour, with 5 min stale tolerance
    // The selector "$.data.items[*].price" targets JSON path for specific data
    request.add_field_cache_directive("$.data.items[*].price", 3600, Some(300));
    
    // Enable deterministic mode for reproducible machine behavior
    request.enable_deterministic_mode();
    
    // Build the complete HTTP request string for example.com
    let request_str = request.build("example.com");
    
    // Display only the custom machine-specific headers
    println!("Machine-HTTP Request Headers:");
    for line in request_str.lines() {
        // Filter for custom headers (starting with X-Machine-)
        if line.starts_with("X-Machine-") {
            println!("  {}", line);
        }
    }
    
    // =========================================================================
    // Example 2: Deterministic Execution
    // Demonstrates reproducible random number generation for consistent behavior
    // =========================================================================
    println!("\n2. Testing Deterministic Execution:");
    
    // Create deterministic RNG with seed 12345
    // Same seed will always produce the same sequence
    let mut rng = deterministic::DeterministicRng::new(12345);
    
    // Generate and display a sequence of deterministic random numbers
    println!("Deterministic RNG sequence:");
    for i in 0..5 {
        println!("  Random {}: {}", i, rng.next_u64());
    }
    
    // Reset RNG to initial state
    // This should reproduce the exact same sequence
    rng.reset();
    
    // Verify deterministic behavior
    println!("Reset RNG sequence (should be same as above):");
    for i in 0..5 {
        println!("  Random {}: {}", i, rng.next_u64());
    }
    
    // =========================================================================
    // Example 3: Session Manager
    // Demonstrates browser session handling with compression
    // =========================================================================
    println!("\n3. Testing Session Manager:");
    
    // Create session manager with:
    // - Max session size: 1MB
    // - Session timeout: 60 minutes
    let session_manager = session_manager::SessionManager::new(1000000, 60);
    
    // Create a new browser session
    let session = session_manager.create_session().unwrap();
    println!("Created session with ID: {}", session.id.as_str());
    
    // Add session data within a scoped block
    // The scope ensures mutable borrow ends properly
    { 
        // Get mutable access to session state
        let mut state = session.get_mut_state();
        
        // Set current URL in session
        state.current_url = Some("https://example.com".to_string());
        
        // Add custom User-Agent header
        state.headers.insert("User-Agent".to_string(), "Machine-HTTP/1.0".to_string());
    }
    
    // Compress session state for efficient storage
    session.compress();
    println!("Compressed session state");
    
    // Calculate and display session size
    let size = session.get_state().estimated_size();
    println!("Session size: {} bytes", size);
    
    // =========================================================================
    // Example 4: DOM Snapshot
    // Demonstrates HTML document structure creation and analysis
    // =========================================================================
    println!("\n4. Testing DOM Snapshot:");
    
    // Create DOM tree structure
    // Build HTML document programmatically
    let mut root_node = dom::DomNode::new_element("html");
    let mut body_node = dom::DomNode::new_element("body");
    
    // Add text content to body
    body_node.add_child(dom::DomNode::new_text("Hello, Machine-HTTP!"));
    
    // Assemble document tree
    root_node.add_child(body_node);
    
    // Create snapshot of DOM state
    let snapshot = dom::DomSnapshot::new(root_node);
    
    // Display snapshot statistics
    println!("Created DOM snapshot with {} nodes, size: {} bytes", 
             snapshot.node_count, snapshot.size_in_bytes);
    
    // =========================================================================
    // Example 5: DNS Resolver and Connection Pool
    // Demonstrates network infrastructure components
    // =========================================================================
    println!("\n5. Testing DNS Resolver and Connection Pool:");
    
    // Create DNS resolver using Google's public DNS server
    match dns::DnsResolver::new("8.8.8.8") {
        Ok(mut dns_resolver) => {
            println!("DNS Resolver created successfully");
            
            // Perform DNS lookup for example.com (IPv4 records)
            match dns_resolver.query("example.com", dns::DnsRecordType::A) {
                Ok(records) => {
                    println!("DNS query results for example.com:");
                    for record in records {
                        // Display each DNS record found
                        println!("  Record: {:?}", record);
                    }
                }
                Err(e) => println!("DNS query failed: {:?}", e),
            }
            
            // Create connection pool combining HTTP client and DNS resolver
            // The pool manages reusable connections for efficiency
            let _connection_pool = connection_pool::ConnectionPool::new(http_client, dns_resolver);
            println!("Connection Pool created successfully");
            
        },
        Err(e) => println!("Failed to create DNS resolver: {:?}", e),
    }
    
    // =========================================================================
    // Completion message
    // =========================================================================
    println!("\nBiosurf components initialized successfully!");
}
