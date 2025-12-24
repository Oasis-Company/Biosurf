mod http_client;
mod dns;
mod connection_pool;
mod deterministic;
mod dom;
mod session_manager;

#[tokio::main]
async fn main() {
    println!("Biosurf - Machine Browser");
    
    // Example 1: Machine-HTTP Client with priority headers
    println!("\n1. Testing Machine-HTTP Client:");
    let http_client = http_client::HttpClient::new();
    
    // Create a request with Machine-HTTP extensions
    let mut request = http_client::HttpRequest::new("GET", "/api/data");
    request.set_machine_priority(http_client::MachineHttpPriority::Latency);
    request.add_field_cache_directive("$.data.items[*].price", 3600, Some(300));
    request.enable_deterministic_mode();
    
    // Build the request for example.com
    let request_str = request.build("example.com");
    println!("Machine-HTTP Request Headers:");
    for line in request_str.lines() {
        if line.starts_with("X-Machine-") {
            println!("  {}", line);
        }
    }
    
    // Example 2: Deterministic Execution
    println!("\n2. Testing Deterministic Execution:");
    let mut rng = deterministic::DeterministicRng::new(12345);
    println!("Deterministic RNG sequence:");
    for i in 0..5 {
        println!("  Random {}: {}", i, rng.next_u64());
    }
    
    // Reset and get the same sequence
    rng.reset();
    println!("Reset RNG sequence (should be same as above):");
    for i in 0..5 {
        println!("  Random {}: {}", i, rng.next_u64());
    }
    
    // Example 3: Session Manager
    println!("\n3. Testing Session Manager:");
    let session_manager = session_manager::SessionManager::new(1000000, 60);
    
    // Create a session
    let session = session_manager.create_session().unwrap();
    println!("Created session with ID: {}", session.id.as_str());
    
    // Add some data to the session
    { 
        let mut state = session.get_mut_state();
        state.current_url = Some("https://example.com".to_string());
        state.headers.insert("User-Agent".to_string(), "Machine-HTTP/1.0".to_string());
    }
    
    // Compress the session
    session.compress();
    println!("Compressed session state");
    
    // Get session size
    let size = session.get_state().estimated_size();
    println!("Session size: {} bytes", size);
    
    // Example 4: DOM Snapshot
    println!("\n4. Testing DOM Snapshot:");
    let mut root_node = dom::DomNode::new_element("html");
    let mut body_node = dom::DomNode::new_element("body");
    body_node.add_child(dom::DomNode::new_text("Hello, Machine-HTTP!"));
    root_node.add_child(body_node);
    
    let snapshot = dom::DomSnapshot::new(root_node);
    println!("Created DOM snapshot with {} nodes, size: {} bytes", 
             snapshot.node_count, snapshot.size_in_bytes);
    
    // Example 5: DNS Resolver and Connection Pool
    println!("\n5. Testing DNS Resolver and Connection Pool:");
    match dns::DnsResolver::new("8.8.8.8") {
        Ok(mut dns_resolver) => {
            println!("DNS Resolver created successfully");
            
            // Example: Resolve a domain
            match dns_resolver.query("example.com", dns::DnsRecordType::A) {
                Ok(records) => {
                    println!("DNS query results for example.com:");
                    for record in records {
                        println!("  Record: {:?}", record);
                    }
                }
                Err(e) => println!("DNS query failed: {:?}", e),
            }
            
            // Example: Create connection pool
            let _connection_pool = connection_pool::ConnectionPool::new(http_client, dns_resolver);
            println!("Connection Pool created successfully");
            
        },
        Err(e) => println!("Failed to create DNS resolver: {:?}", e),
    }
    
    println!("\nBiosurf components initialized successfully!");
}
