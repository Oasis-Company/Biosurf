use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::hash::Hash;
use std::fmt::Debug;

use tokio::sync::{Semaphore, Mutex as TokioMutex};
use tokio::time::{sleep, timeout};

use crate::http_client::{HttpClient, HttpStream};
use crate::dns::DnsResolver;

const DEFAULT_MAX_CONNECTIONS: usize = 100;
const DEFAULT_IDLE_TIMEOUT: Duration = Duration::from_secs(300);
const DEFAULT_CONNECTION_TIMEOUT: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ConnectionKey {
    pub scheme: String,
    pub host: String,
    pub port: u16,
}

struct ConnectionPoolEntry {
    stream: HttpStream,
    created_at: Instant,
    last_used: Instant,
    in_use: bool,
}

struct ConnectionPoolInner {
    connections: HashMap<ConnectionKey, Vec<ConnectionPoolEntry>>,
    idle_timeout: Duration,
    max_connections: usize,
    total_connections: usize,
}

pub struct ConnectionPool {
    inner: Arc<TokioMutex<ConnectionPoolInner>>,
    semaphore: Arc<Semaphore>,
    http_client: HttpClient,
    dns_resolver: Arc<Mutex<DnsResolver>>,
    connection_timeout: Duration,
    max_connections: usize,
}

impl ConnectionPool {
    pub fn new(http_client: HttpClient, dns_resolver: DnsResolver) -> Self {
        Self::with_config(http_client, dns_resolver, DEFAULT_MAX_CONNECTIONS, DEFAULT_IDLE_TIMEOUT, DEFAULT_CONNECTION_TIMEOUT)
    }
    
    pub fn with_config(http_client: HttpClient, dns_resolver: DnsResolver, max_connections: usize, idle_timeout: Duration, connection_timeout: Duration) -> Self {
        ConnectionPool {
            inner: Arc::new(TokioMutex::new(ConnectionPoolInner {
                connections: HashMap::new(),
                idle_timeout,
                max_connections,
                total_connections: 0,
            })),
            semaphore: Arc::new(Semaphore::new(max_connections)),
            http_client,
            dns_resolver: Arc::new(Mutex::new(dns_resolver)),
            connection_timeout,
        }
    }
    
    pub async fn get_connection(&self, scheme: &str, host: &str, port: u16) -> tokio::io::Result<ConnectionGuard<'_>> {
        let key = ConnectionKey {
            scheme: scheme.to_string(),
            host: host.to_string(),
            port,
        };
        
        // Acquire semaphore to ensure we don't exceed max connections
        let permit = self.semaphore.acquire().await.unwrap();
        
        // Try to find an idle connection
        let mut found_idle = false;
        let mut stream_ref: Option<&mut HttpStream> = None;
        let mut key_clone = key.clone();
        
        { 
            let mut inner = self.inner.lock().await;
            let idle_timeout = inner.idle_timeout;
            
            if let Some(entries) = inner.connections.get_mut(&key) {
                // Find an idle entry
                for entry in entries.iter_mut() {
                    if !entry.in_use && entry.last_used.elapsed() < idle_timeout {
                        entry.in_use = true;
                        entry.last_used = Instant::now();
                        stream_ref = Some(&mut entry.stream);
                        found_idle = true;
                        break;
                    }
                }
            }
        }
        
        if found_idle {
            return Ok(ConnectionGuard {
                pool: self,
                key: key_clone,
                stream_ref: stream_ref.unwrap(),
                permit: Some(permit),
            });
        }
        
        // No idle connection, create a new one
        let host_clone = host.to_string();
        let host_clone_for_tls = host_clone.clone();
        let scheme_clone = scheme.to_string();
        let dns_resolver = self.dns_resolver.clone();
        let http_client = self.http_client.clone();
        let connection_timeout = self.connection_timeout;
        
        // Resolve DNS in a blocking context
        let ip = tokio::task::spawn_blocking(move || {
            let mut resolver = dns_resolver.lock().unwrap();
            resolver.resolve_ip(&host_clone)
        }).await??;
        
        // Create connection with timeout
        let stream = timeout(connection_timeout, async move {
            match scheme_clone.as_str() {
                "http" => http_client.connect_http((ip, port)),
                "https" => http_client.connect_https((ip, port), &host_clone_for_tls),
                _ => Err(std::io::Error::new(std::io::ErrorKind::InvalidInput, format!("Unsupported scheme: {}", scheme_clone))),
            }
        }).await??;
        
        // Add the new connection to the pool
        { 
            let mut inner = self.inner.lock().await;
            inner.total_connections += 1;
            
            let entry = ConnectionPoolEntry {
                stream,
                created_at: Instant::now(),
                last_used: Instant::now(),
                in_use: true,
            };
            
            let entries = inner.connections.entry(key.clone()).or_insert_with(Vec::new);
            entries.push(entry);
        }
        
        // Get the new connection from the pool
        let mut inner = self.inner.lock().await;
        let entries = inner.connections.get_mut(&key).unwrap();
        let stream_ref = &mut entries.last_mut().unwrap().stream;
        
        Ok(ConnectionGuard {
            pool: self,
            key,
            stream_ref,
            permit: Some(permit),
        })
    } 
    
    pub async fn cleanup(&self) { 
        let mut inner = self.inner.lock().await; 
        let now = Instant::now(); 
        let mut keys_to_remove = Vec::new(); 
        let idle_timeout = inner.idle_timeout; 
        let mut total_removed = 0; 
        
        // First pass: count removed connections and mark empty lists 
        for (key, entries) in &mut inner.connections { 
            let original_len = entries.len(); 
            
            // Remove idle connections that exceed the timeout 
            entries.retain(|entry| entry.in_use || (now - entry.last_used) < idle_timeout); 
            
            // Count the number of connections removed 
            let removed = original_len - entries.len(); 
            total_removed += removed; 
            
            // If no connections left, mark for removal 
            if entries.is_empty() { 
                keys_to_remove.push(key.clone()); 
            } 
        } 
        
        // Update total connections 
        inner.total_connections = inner.total_connections.saturating_sub(total_removed); 
        
        // Remove empty connection lists 
        for key in keys_to_remove { 
            inner.connections.remove(&key); 
        } 
    } 
    
    pub async fn run_cleanup_task(self: Arc<Self>, interval: Duration) { 
        loop { 
            sleep(interval).await; 
            self.cleanup().await; 
        } 
    } 
    
    pub async fn get_stats(&self) -> PoolStats { 
        let inner = self.inner.lock().await; 
        let mut total_idle = 0; 
        let mut total_in_use = 0; 
        
        for entries in inner.connections.values() { 
            for entry in entries { 
                if entry.in_use { 
                    total_in_use += 1; 
                } else { 
                    total_idle += 1; 
                } 
            } 
        } 
        
        PoolStats { 
            total_connections: inner.total_connections, 
            total_idle, 
            total_in_use, 
            max_connections: inner.max_connections, 
            idle_timeout: inner.idle_timeout, 
            connection_count: inner.connections.len(), 
        } 
    } 
    
    pub async fn close_all_connections(&self) { 
        let mut inner = self.inner.lock().await; 
        inner.connections.clear(); 
        inner.total_connections = 0; 
    } 
} 

#[derive(Debug, Clone)] 
pub struct PoolStats { 
    pub total_connections: usize, 
    pub total_idle: usize, 
    pub total_in_use: usize, 
    pub max_connections: usize, 
    pub idle_timeout: Duration, 
    pub connection_count: usize, 
} 

pub struct ConnectionGuard<'a> {
    pub pool: &'a ConnectionPool,
    pub key: ConnectionKey,
    pub stream_ref: &'a mut HttpStream,
    pub permit: Option<tokio::sync::SemaphorePermit<'a>>,
}

impl<'a> ConnectionGuard<'a> {
    pub fn get_mut(&mut self) -> Option<&mut HttpStream> {
        Some(self.stream_ref)
    }
    
    pub fn is_valid(&self) -> bool {
        true
    }
}

impl<'a> Drop for ConnectionGuard<'a> {
    fn drop(&mut self) {
        // Release the connection back to the pool
        let pool = self.pool.inner.clone();
        let key = self.key.clone();
        
        tokio::spawn(async move {
            let mut inner = pool.lock().await;
            if let Some(entries) = inner.connections.get_mut(&key) {
                for entry in entries.iter_mut() {
                    if entry.in_use {
                        entry.in_use = false;
                        entry.last_used = Instant::now();
                        break;
                    }
                }
            }
        });
        
        // Release the semaphore permit
        drop(self.permit.take());
    }
} 

#[cfg(test)] 
mod tests { 
    use super::*; 
    use crate::http_client::HttpClient; 
    use crate::dns::DnsResolver; 
    
    #[tokio::test] 
    async fn test_connection_pool() { 
        // This is a basic test structure 
        // In a real environment, you would need a test server to connect to 
        let http_client = HttpClient::new(); 
        let dns_resolver = DnsResolver::new("8.8.8.8").unwrap(); 
        let pool = Arc::new(ConnectionPool::new(http_client, dns_resolver)); 
        
        // Start cleanup task 
        let pool_clone = pool.clone(); 
        tokio::spawn(async move { 
            pool_clone.run_cleanup_task(Duration::from_secs(60)).await; 
        }); 
        
        // Get stats 
        let stats = pool.get_stats().await; 
        assert_eq!(stats.total_connections, 0); 
        assert_eq!(stats.total_idle, 0); 
        assert_eq!(stats.total_in_use, 0); 
        
        // Close all connections 
        pool.close_all_connections().await; 
        
        let stats = pool.get_stats().await; 
        assert_eq!(stats.total_connections, 0); 
    } 
} 
