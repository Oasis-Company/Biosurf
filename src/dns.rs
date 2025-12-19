use std::net::{UdpSocket, ToSocketAddrs, SocketAddr}; 
use std::io::{Result, Error, ErrorKind}; 
use std::collections::HashMap; 
use std::time::{Duration, SystemTime}; 
use std::net::IpAddr; 

const DNS_PORT: u16 = 53; 
const DNS_TIMEOUT: Duration = Duration::from_secs(5); 

#[derive(Debug, Clone, PartialEq)] 
pub enum DnsRecordType { 
    A,     // IPv4 address 
    AAAA,  // IPv6 address 
    CNAME, // Canonical name 
    NS,    // Name server 
    MX,    // Mail exchange 
} 

impl DnsRecordType { 
    fn to_u16(&self) -> u16 { 
        match self { 
            DnsRecordType::A => 1, 
            DnsRecordType::AAAA => 28, 
            DnsRecordType::CNAME => 5, 
            DnsRecordType::NS => 2, 
            DnsRecordType::MX => 15, 
        } 
    } 
    
    fn from_u16(value: u16) -> Option<Self> { 
        match value { 
            1 => Some(DnsRecordType::A), 
            28 => Some(DnsRecordType::AAAA), 
            5 => Some(DnsRecordType::CNAME), 
            2 => Some(DnsRecordType::NS), 
            15 => Some(DnsRecordType::MX), 
            _ => None, 
        } 
    } 
} 

#[derive(Debug, Clone)] 
pub struct DnsRecord { 
    pub name: String, 
    pub record_type: DnsRecordType, 
    pub ttl: u32, 
    pub data: DnsRecordData, 
} 

#[derive(Debug, Clone)] 
pub enum DnsRecordData { 
    A(IpAddr), 
    AAAA(IpAddr), 
    CNAME(String), 
    NS(String), 
    MX { preference: u16, exchange: String }, 
} 

#[derive(Debug)] 
pub struct DnsResponse { 
    pub id: u16, 
    pub qr: bool, 
    pub opcode: u8, 
    pub aa: bool, 
    pub tc: bool, 
    pub rd: bool, 
    pub ra: bool, 
    pub rcode: u8, 
    pub questions: Vec<DnsQuestion>, 
    pub answers: Vec<DnsRecord>, 
    pub authority: Vec<DnsRecord>, 
    pub additional: Vec<DnsRecord>, 
} 

#[derive(Debug, Clone)] 
pub struct DnsQuestion { 
    pub name: String, 
    pub record_type: DnsRecordType, 
    pub class: u16, 
} 

struct DnsCacheEntry { 
    records: Vec<DnsRecord>, 
    expires_at: SystemTime, 
} 

pub struct DnsResolver { 
    socket: UdpSocket, 
    cache: HashMap<String, DnsCacheEntry>, 
    dns_server: SocketAddr, 
} 

impl DnsResolver { 
    pub fn new(dns_server: &str) -> Result<Self> { 
        let socket = UdpSocket::bind("0.0.0.0:0")?; 
        socket.set_read_timeout(Some(DNS_TIMEOUT))?; 
        
        let dns_addr: SocketAddr = format!("{}:{}", dns_server, DNS_PORT) 
            .to_socket_addrs()? 
            .next() 
            .ok_or(Error::new(ErrorKind::InvalidInput, "Invalid DNS server address"))?; 
        
        Ok(DnsResolver { 
            socket, 
            cache: HashMap::new(), 
            dns_server: dns_addr, 
        }) 
    } 
    
    pub fn query(&mut self, domain: &str, record_type: DnsRecordType) -> Result<Vec<DnsRecord>> { 
        // Check cache first 
        let cache_key = format!("{}:{:?}", domain, record_type); 
        if let Some(entry) = self.cache.get(&cache_key) { 
            if SystemTime::now() < entry.expires_at { 
                return Ok(entry.records.clone()); 
            } 
        } 
        
        // Create DNS query
        let query = self.create_query(domain, record_type.clone())?;
        
        // Send query 
        self.socket.send_to(&query, self.dns_server)?; 
        
        // Receive response 
        let mut buffer = [0; 512]; 
        let (size, _) = self.socket.recv_from(&mut buffer)?; 
        
        // Parse response 
        let response = self.parse_response(&buffer[..size])?; 
        
        // Check response status 
        if response.rcode != 0 { 
            return Err(Error::new(ErrorKind::Other, format!("DNS query failed with rcode: {}", response.rcode))); 
        } 
        
        // Filter records of requested type 
        let records: Vec<DnsRecord> = response.answers 
            .into_iter() 
            .filter(|record| record.record_type == record_type) 
            .collect(); 
        
        if records.is_empty() { 
            return Err(Error::new(ErrorKind::NotFound, "No records found")); 
        } 
        
        // Cache the results 
        let min_ttl = records.iter().map(|r| r.ttl).min().unwrap_or(300); 
        let expires_at = SystemTime::now() + Duration::from_secs(min_ttl.into()); 
        
        self.cache.insert(cache_key, DnsCacheEntry { 
            records: records.clone(), 
            expires_at, 
        }); 
        
        Ok(records) 
    } 
    
    pub fn resolve_ip(&mut self, domain: &str) -> Result<IpAddr> { 
        // Try A record (IPv4) first 
        if let Ok(records) = self.query(domain, DnsRecordType::A) { 
            if let DnsRecordData::A(ip) = &records[0].data { 
                return Ok(*ip); 
            } 
        } 
        
        // Try AAAA record (IPv6) if IPv4 failed 
        if let Ok(records) = self.query(domain, DnsRecordType::AAAA) { 
            if let DnsRecordData::AAAA(ip) = &records[0].data { 
                return Ok(*ip); 
            } 
        } 
        
        Err(Error::new(ErrorKind::NotFound, "Could not resolve IP address")) 
    } 
    
    fn create_query(&self, domain: &str, record_type: DnsRecordType) -> Result<Vec<u8>> { 
        let mut query = Vec::new(); 
        
        // Transaction ID (random) 
        let tid = rand::random::<u16>(); 
        query.extend_from_slice(&tid.to_be_bytes()); 
        
        // Flags: Standard query, recursion desired
        let flags: u16 = 0x0100; // 0000 0001 0000 0000
        query.extend_from_slice(&flags.to_be_bytes()); 
        
        // Questions count 
        let qdcount = 1u16; 
        query.extend_from_slice(&qdcount.to_be_bytes()); 
        
        // Answer records count (0 for query) 
        let ancount = 0u16; 
        query.extend_from_slice(&ancount.to_be_bytes()); 
        
        // Authority records count (0 for query) 
        let nscount = 0u16; 
        query.extend_from_slice(&nscount.to_be_bytes()); 
        
        // Additional records count (0 for query) 
        let arcount = 0u16; 
        query.extend_from_slice(&arcount.to_be_bytes()); 
        
        // Query name (encoded as labels) 
        for label in domain.split('.') { 
            let len = label.len() as u8; 
            query.push(len); 
            query.extend_from_slice(label.as_bytes()); 
        } 
        query.push(0); // End of name 
        
        // Query type 
        query.extend_from_slice(&record_type.to_u16().to_be_bytes()); 
        
        // Query class (IN for Internet) 
        let class = 1u16; 
        query.extend_from_slice(&class.to_be_bytes()); 
        
        Ok(query) 
    } 
    
    fn parse_response(&self, data: &[u8]) -> Result<DnsResponse> { 
        if data.len() < 12 { 
            return Err(Error::new(ErrorKind::InvalidData, "DNS response too short")); 
        } 
        
        let mut offset = 0; 
        
        // Transaction ID 
        let id = u16::from_be_bytes([data[0], data[1]]); 
        offset += 2; 
        
        // Flags 
        let flags = u16::from_be_bytes([data[2], data[3]]); 
        let qr = (flags & 0x8000) != 0; 
        let opcode = ((flags & 0x7800) >> 11) as u8; 
        let aa = (flags & 0x0400) != 0; 
        let tc = (flags & 0x0200) != 0; 
        let rd = (flags & 0x0100) != 0; 
        let ra = (flags & 0x0080) != 0; 
        let rcode = (flags & 0x000F) as u8; 
        offset += 2; 
        
        // Counts 
        let qdcount = u16::from_be_bytes([data[4], data[5]]); 
        let ancount = u16::from_be_bytes([data[6], data[7]]); 
        let nscount = u16::from_be_bytes([data[8], data[9]]); 
        let arcount = u16::from_be_bytes([data[10], data[11]]); 
        offset += 8; 
        
        // Parse questions 
        let mut questions = Vec::new(); 
        for _ in 0..qdcount { 
            let (name, new_offset) = self.parse_dns_name(data, offset)?; 
            offset = new_offset; 
            
            let record_type = u16::from_be_bytes([data[offset], data[offset + 1]]); 
            let class = u16::from_be_bytes([data[offset + 2], data[offset + 3]]); 
            offset += 4; 
            
            questions.push(DnsQuestion { 
                name, 
                record_type: DnsRecordType::from_u16(record_type) 
                    .ok_or(Error::new(ErrorKind::InvalidData, "Unknown record type"))?, 
                class, 
            }); 
        } 
        
        // Parse records 
        let (answers, new_offset) = self.parse_records(data, offset, ancount)?; 
        offset = new_offset; 
        
        let (authority, new_offset) = self.parse_records(data, offset, nscount)?; 
        offset = new_offset; 
        
        let (additional, _) = self.parse_records(data, offset, arcount)?; 
        
        Ok(DnsResponse { 
            id, 
            qr, 
            opcode, 
            aa, 
            tc, 
            rd, 
            ra, 
            rcode, 
            questions, 
            answers, 
            authority, 
            additional, 
        }) 
    } 
    
    fn parse_records(&self, data: &[u8], offset: usize, count: u16) -> Result<(Vec<DnsRecord>, usize)> { 
        let mut records = Vec::new(); 
        let mut current_offset = offset; 
        
        for _ in 0..count { 
            let (name, new_offset) = self.parse_dns_name(data, current_offset)?; 
            current_offset = new_offset; 
            
            let record_type = u16::from_be_bytes([data[current_offset], data[current_offset + 1]]); 
            let _class = u16::from_be_bytes([data[current_offset + 2], data[current_offset + 3]]); 
            let ttl = u32::from_be_bytes([data[current_offset + 4], data[current_offset + 5], data[current_offset + 6], data[current_offset + 7]]); 
            let rdlength = u16::from_be_bytes([data[current_offset + 8], data[current_offset + 9]]); 
            current_offset += 10; 
            
            let record = match DnsRecordType::from_u16(record_type) { 
                Some(DnsRecordType::A) => { 
                    if rdlength != 4 { 
                        return Err(Error::new(ErrorKind::InvalidData, "Invalid A record length")); 
                    } 
                    let ip = format!("{}.{}.{}.{}", 
                        data[current_offset], 
                        data[current_offset + 1], 
                        data[current_offset + 2], 
                        data[current_offset + 3]) 
                        .parse::<IpAddr>().map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?; 
                    DnsRecord { 
                        name: name.clone(), 
                        record_type: DnsRecordType::A, 
                        ttl, 
                        data: DnsRecordData::A(ip), 
                    } 
                } 
                Some(DnsRecordType::AAAA) => { 
                    if rdlength != 16 { 
                        return Err(Error::new(ErrorKind::InvalidData, "Invalid AAAA record length")); 
                    } 
                    let mut ipv6_parts = Vec::new(); 
                    for i in 0..8 { 
                        let part = u16::from_be_bytes([data[current_offset + i * 2], data[current_offset + i * 2 + 1]]); 
                        ipv6_parts.push(format!("{:x}", part)); 
                    } 
                    let ip = ipv6_parts.join(":").parse::<IpAddr>().map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?; 
                    DnsRecord { 
                        name: name.clone(), 
                        record_type: DnsRecordType::AAAA, 
                        ttl, 
                        data: DnsRecordData::AAAA(ip), 
                    } 
                } 
                Some(DnsRecordType::CNAME) => { 
                    let (cname, _cname_offset) = self.parse_dns_name(data, current_offset)?;
                    DnsRecord { 
                        name: name.clone(), 
                        record_type: DnsRecordType::CNAME, 
                        ttl, 
                        data: DnsRecordData::CNAME(cname), 
                    } 
                } 
                Some(DnsRecordType::NS) => { 
                    let (ns_name, _ns_offset) = self.parse_dns_name(data, current_offset)?;
                    DnsRecord { 
                        name: name.clone(), 
                        record_type: DnsRecordType::NS, 
                        ttl, 
                        data: DnsRecordData::NS(ns_name), 
                    } 
                } 
                Some(DnsRecordType::MX) => { 
                    let preference = u16::from_be_bytes([data[current_offset], data[current_offset + 1]]); 
                    let (exchange, _exchange_offset) = self.parse_dns_name(data, current_offset + 2)?;
                    DnsRecord { 
                        name: name.clone(), 
                        record_type: DnsRecordType::MX, 
                        ttl, 
                        data: DnsRecordData::MX { preference, exchange }, 
                    } 
                } 
                _ => { 
                    // Skip unknown record types 
                    DnsRecord { 
                        name: name.clone(), 
                        record_type: DnsRecordType::from_u16(record_type) 
                            .ok_or(Error::new(ErrorKind::InvalidData, "Unknown record type"))?, 
                        ttl, 
                        data: DnsRecordData::A("0.0.0.0".parse::<IpAddr>().unwrap()), 
                    } 
                } 
            }; 
            
            records.push(record); 
            current_offset += rdlength as usize; 
        } 
        
        Ok((records, current_offset)) 
    } 
    
    fn parse_dns_name(&self, data: &[u8], offset: usize) -> Result<(String, usize)> { 
        let mut name = String::new(); 
        let mut current_offset = offset; 
        
        loop { 
            let len = data[current_offset] as usize; 
            
            // Check for compression (pointer) 
            if (len & 0xC0) == 0xC0 { 
                current_offset += 2; // Move past the pointer 
                
                let pointer_offset = ((len & 0x3F) as u16) << 8 | data[current_offset - 1] as u16; 
                let (pointer_name, _) = self.parse_dns_name(data, pointer_offset as usize)?; 
                name.push_str(&pointer_name); 
                break; 
            } 
            
            // End of name 
            if len == 0 { 
                current_offset += 1; 
                break; 
            } 
            
            // Normal label 
            current_offset += 1; 
            if current_offset + len > data.len() { 
                return Err(Error::new(ErrorKind::InvalidData, "Invalid DNS name label")); 
            } 
            
            let label = String::from_utf8_lossy(&data[current_offset..current_offset + len]); 
            if !name.is_empty() { 
                name.push('.'); 
            } 
            name.push_str(&label); 
            current_offset += len; 
        } 
        
        Ok((name, current_offset)) 
    } 
    
    pub fn clear_cache(&mut self) { 
        self.cache.clear(); 
    } 
    
    pub fn set_dns_server(&mut self, dns_server: &str) -> Result<()> { 
        let dns_addr: SocketAddr = format!("{}:{}", dns_server, DNS_PORT) 
            .to_socket_addrs()? 
            .next() 
            .ok_or(Error::new(ErrorKind::InvalidInput, "Invalid DNS server address"))?; 
        
        self.dns_server = dns_addr; 
        Ok(()) 
    } 
} 
