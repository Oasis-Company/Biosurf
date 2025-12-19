mod http_client;
mod dns;
mod connection_pool;

#[tokio::main]
async fn main() {
    println!("Biosurf - Machine Browser");
    
    // Example: Create HTTP client
    let http_client = http_client::HttpClient::new();
    
    // Example: Create DNS resolver
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
}
