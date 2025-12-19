use std::io::{Read, Write, Result};
use std::clone::Clone;
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;
use native_tls::{TlsConnector, TlsStream};

enum HttpStream {
    Plain(TcpStream),
    Tls(TlsStream<TcpStream>),
}

impl Read for HttpStream {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize> {
        match self {
            HttpStream::Plain(stream) => stream.read(buf),
            HttpStream::Tls(stream) => stream.read(buf),
        }
    }
}

impl Write for HttpStream {
    fn write(&mut self, buf: &[u8]) -> Result<usize> {
        match self {
            HttpStream::Plain(stream) => stream.write(buf),
            HttpStream::Tls(stream) => stream.write(buf),
        }
    }

    fn flush(&mut self) -> Result<()> {
        match self {
            HttpStream::Plain(stream) => stream.flush(),
            HttpStream::Tls(stream) => stream.flush(),
        }
    }
}

pub struct HttpClient {
    timeout: Duration,
    tls_connector: TlsConnector,
}

impl HttpClient {
    pub fn new() -> Self {
        HttpClient {
            timeout: Duration::from_secs(30),
            tls_connector: TlsConnector::new().unwrap(),
        }
    }

    pub fn set_timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = timeout;
        self
    }

    pub fn connect<A: ToSocketAddrs>(&self, addr: A) -> Result<TcpStream> {
        let stream = TcpStream::connect(addr)?;
        stream.set_read_timeout(Some(self.timeout))?;
        stream.set_write_timeout(Some(self.timeout))?;
        Ok(stream)
    }

    pub fn connect_https<A: ToSocketAddrs>(&self, addr: A, domain: &str) -> Result<HttpStream> {
        let tcp_stream = self.connect(addr)?;
        let tls_stream = self.tls_connector.connect(domain, tcp_stream)?;
        Ok(HttpStream::Tls(tls_stream))
    }

    pub fn connect_http<A: ToSocketAddrs>(&self, addr: A) -> Result<HttpStream> {
        let tcp_stream = self.connect(addr)?;
        Ok(HttpStream::Plain(tcp_stream))
    }

    pub fn send_request(&self, stream: &mut HttpStream, request: &str) -> Result<()> {
        stream.write_all(request.as_bytes())?;
        Ok(())
    }

    pub fn receive_response(&self, stream: &mut HttpStream) -> Result<String> {
        let mut buffer = Vec::new();
        let mut chunk = [0; 4096];
        
        while let Ok(n) = stream.read(&mut chunk) {
            if n == 0 {
                break;
            }
            buffer.extend_from_slice(&chunk[..n]);
        }
        
        String::from_utf8(buffer).map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    pub fn receive_response_chunked(&self, stream: &mut HttpStream) -> Result<String> {
        let mut response = String::new();
        let mut buffer = Vec::new();
        let mut chunk = [0; 4096];
        
        // First read until we get all headers
        let mut in_body = false;
        let mut header_part = String::new();
        
        while let Ok(n) = stream.read(&mut chunk) {
            if n == 0 {
                break;
            }
            
            buffer.extend_from_slice(&chunk[..n]);
            
            if !in_body {
                if let Ok(text) = String::from_utf8_lossy(&buffer) {
                    if let Some((headers, rest)) = text.split_once("\r\n\r\n") {
                        header_part = headers.to_string();
                        response.push_str(headers);
                        response.push_str("\r\n\r\n");
                        
                        // Process the rest as chunked body
                        let rest: &str = rest;
                        let mut body_buffer: Vec<u8> = Vec::from(rest.as_bytes());
                        
                        // Parse chunks
                        loop {
                            // Find chunk size line
                            if let Some((size_line, rest)) = String::from_utf8_lossy(&body_buffer).split_once("\r\n") {
                                // Parse chunk size (in hex)
                                let chunk_size_str = size_line.split(';').next().unwrap().trim();
                                let chunk_size = u64::from_str_radix(chunk_size_str, 16)
                                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Invalid chunk size: {}", e)))?;
                                
                                if chunk_size == 0 {
                                    // End of chunks
                                    response.push_str("0\r\n\r\n");
                                    return Ok(response);
                                }
                                
                                // Calculate total bytes needed for this chunk
                                let total_chunk_bytes = chunk_size + 2; // +2 for trailing CRLF
                                let rest_bytes = rest.as_bytes();
                                
                                if rest_bytes.len() >= total_chunk_bytes as usize {
                                    // We have the complete chunk
                                    let chunk_data = &rest_bytes[..chunk_size as usize];
                                    response.push_str(&String::from_utf8_lossy(chunk_data));
                                    
                                    // Remove processed bytes from body_buffer
                                    let processed_bytes = size_line.len() + 2 + total_chunk_bytes as usize;
                                    body_buffer.drain(..processed_bytes);
                                } else {
                                    // Need more data
                                    break;
                                }
                            } else {
                                // Need more data to complete chunk size line
                                break;
                            }
                        }
                        
                        in_body = true;
                        buffer = body_buffer;
                    }
                }
            } else {
                // Continue reading chunked body
                let mut body_text = String::from_utf8_lossy(&buffer);
                
                // Process chunks
                loop {
                    if let Some((size_line, rest)) = body_text.split_once("\r\n") {
                        let chunk_size_str = size_line.split(';').next().unwrap().trim();
                        let chunk_size = u64::from_str_radix(chunk_size_str, 16)
                            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Invalid chunk size: {}", e)))?;
                        
                        if chunk_size == 0 {
                            response.push_str("0\r\n\r\n");
                            return Ok(response);
                        }
                        
                        let total_chunk_bytes = chunk_size + 2;
                        let rest_bytes = rest.as_bytes();
                        
                        if rest_bytes.len() >= total_chunk_bytes as usize {
                            let chunk_data = &rest_bytes[..chunk_size as usize];
                            response.push_str(&String::from_utf8_lossy(chunk_data));
                            
                            let processed_bytes = size_line.len() + 2 + total_chunk_bytes as usize;
                            body_text = body_text[processed_bytes..].to_string().into();
                        } else {
                            // Need more data
                            buffer = body_text.as_bytes().to_vec();
                            break;
                        }
                    } else {
                        buffer = body_text.as_bytes().to_vec();
                        break;
                    }
                }
            }
        }
        
        Ok(response)
    }
}

pub struct HttpRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Option<String>,
}

impl HttpRequest {
    pub fn new(method: &str, path: &str) -> Self {
        HttpRequest {
            method: method.to_string(),
            path: path.to_string(),
            headers: Vec::new(),
            body: None,
        }
    }

    pub fn add_header(&mut self, name: &str, value: &str) -> &mut Self {
        self.headers.push((name.to_string(), value.to_string()));
        self
    }

    pub fn set_body(&mut self, body: &str) -> &mut Self {
        self.body = Some(body.to_string());
        self
    }

    pub fn build(&self, host: &str) -> String {
        let mut request = format!("{} {} HTTP/1.1\r\n", self.method, self.path);
        
        request.push_str(&format!("Host: {}\r\n", host));
        request.push_str("Connection: keep-alive\r\n");
        
        if let Some(body) = &self.body {
            request.push_str(&format!("Content-Length: {}\r\n", body.len()));
        }
        
        for (name, value) in &self.headers {
            request.push_str(&format!("{}: {}\r\n", name, value));
        }
        
        request.push_str("\r\n");
        
        if let Some(body) = &self.body {
            request.push_str(body);
        }
        
        request
    }
}

pub struct HttpResponse {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

impl HttpResponse {
    pub fn parse(response: &str) -> Result<Self> {
        let mut lines = response.lines();
        
        let status_line = lines.next().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Empty response"))?;
        let mut parts = status_line.split_whitespace();
        
        let _http_version = parts.next().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid status line"))?;
        let status = parts.next().ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidData, "Missing status code"))?
            .parse::<u16>().map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "Invalid status code"))?;
        let status_text = parts.collect::<Vec<&str>>().join(" ");
        
        let mut headers = Vec::new();
        let mut body = String::new();
        let mut in_body = false;
        let mut transfer_encoding_chunked = false;
        
        for line in lines {
            if in_body {
                body.push_str(line);
                body.push_str("\r\n");
                continue;
            }
            
            if line.is_empty() {
                in_body = true;
                continue;
            }
            
            if let Some((name, value)) = line.split_once(':') {
                let name = name.trim().to_lowercase();
                let value = value.trim().to_string();
                headers.push((name.clone(), value.clone()));
                
                if name == "transfer-encoding" && value.to_lowercase() == "chunked" {
                    transfer_encoding_chunked = true;
                }
            }
        }
        
        // If chunked encoding, decode the body
        let decoded_body = if transfer_encoding_chunked {
            Self::decode_chunked_body(&body)?
        } else {
            body.trim_end().to_string()
        };
        
        Ok(HttpResponse {
            status,
            status_text,
            headers,
            body: decoded_body,
        })
    }
    
    fn decode_chunked_body(chunked_body: &str) -> Result<String> {
        let mut decoded = String::new();
        let mut remaining = chunked_body;
        
        loop {
            if let Some((size_line, rest)) = remaining.split_once("\r\n") {
                // Parse chunk size (in hex)
                let chunk_size_str = size_line.split(';').next().unwrap().trim();
                let chunk_size = u64::from_str_radix(chunk_size_str, 16)
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, format!("Invalid chunk size: {}", e)))?;
                
                if chunk_size == 0 {
                    // End of chunks
                    break;
                }
                
                // Calculate total bytes needed for this chunk
                let total_chunk_bytes = chunk_size as usize + 2; // +2 for trailing CRLF
                
                if rest.len() >= total_chunk_bytes {
                    // We have the complete chunk
                    let chunk_data = &rest[..chunk_size as usize];
                    decoded.push_str(chunk_data);
                    
                    // Move to next chunk
                    remaining = &rest[total_chunk_bytes..];
                } else {
                    return Err(std::io::Error::new(std::io::ErrorKind::UnexpectedEof, "Incomplete chunk"));
                }
            } else {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "Malformed chunked body"));
            }
        }
        
        Ok(decoded)
    }
}
