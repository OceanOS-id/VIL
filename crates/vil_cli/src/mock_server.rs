#![allow(dead_code)]

use anyhow::Result;
use std::net::SocketAddr;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

pub struct MockServer {
    port: u16,
    latency_ms: u64,
    error_rate: f32,
}

impl MockServer {
    pub fn new(port: u16) -> Self {
        Self {
            port,
            latency_ms: 50,
            error_rate: 0.0,
        }
    }

    pub fn latency_ms(mut self, ms: u64) -> Self {
        self.latency_ms = ms;
        self
    }

    pub fn error_rate(mut self, rate: f32) -> Self {
        self.error_rate = rate;
        self
    }

    pub async fn run(&self) -> Result<()> {
        let addr = SocketAddr::from(([127, 0, 0, 1], self.port));
        let listener = TcpListener::bind(addr).await?;

        println!("Mock inference server listening on http://{}", addr);

        loop {
            let (mut socket, _) = listener.accept().await?;
            let latency = self.latency_ms;
            let error_rate = self.error_rate;

            tokio::spawn(async move {
                let mut buf = [0u8; 1024];

                if let Ok(n) = socket.read(&mut buf).await {
                    if n > 0 {
                        tokio::time::sleep(tokio::time::Duration::from_millis(latency)).await;

                        if error_rate > 0.0 && rand::random::<f32>() < error_rate {
                            let response = "HTTP/1.1 500 Internal Server Error\r\n\r\n";
                            let _ = socket.write_all(response.as_bytes()).await;
                        } else {
                            let response = format!(
                                "HTTP/1.1 200 OK\r\n\
                                Content-Type: text/event-stream\r\n\
                                Cache-Control: no-cache\r\n\
                                Connection: keep-alive\r\n\
                                \r\n\
                                data: {{\"choices\":[{{\"delta\":{{\"content\":\"Hello from mock server!\"}}}}]}}\n\n"
                            );
                            let _ = socket.write_all(response.as_bytes()).await;
                        }
                    }
                }
            });
        }
    }
}

pub async fn start_mock_server(port: u16) -> Result<()> {
    let server = MockServer::new(port);
    server.run().await
}
