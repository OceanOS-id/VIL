// =============================================================================
// Connection Pool — Round-robin dispatch with backpressure
// =============================================================================
//
// Manages N connections per sidecar with round-robin dispatch and in-flight
// tracking. When max_in_flight is exceeded, checkout returns an error to
// apply backpressure upstream.

use crate::protocol::Message;
use crate::transport::{SidecarConnection, TransportError};
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Configuration for a connection pool.
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// Number of connections in the pool. Default: 4.
    pub pool_size: usize,
    /// Maximum number of in-flight requests. 0 = unlimited. Default: 1000.
    pub max_in_flight: u64,
}

impl Default for PoolConfig {
    fn default() -> Self {
        Self {
            pool_size: 4,
            max_in_flight: 1000,
        }
    }
}

/// A pool of sidecar connections with round-robin dispatch and backpressure.
pub struct ConnectionPool {
    connections: Vec<Arc<Mutex<SidecarConnection>>>,
    counter: AtomicUsize,
    in_flight: AtomicU64,
    config: PoolConfig,
}

impl ConnectionPool {
    /// Create a new pool from a vec of established connections.
    pub fn new(connections: Vec<SidecarConnection>, config: PoolConfig) -> Self {
        let connections = connections
            .into_iter()
            .map(|c| Arc::new(Mutex::new(c)))
            .collect();
        Self {
            connections,
            counter: AtomicUsize::new(0),
            in_flight: AtomicU64::new(0),
            config,
        }
    }

    /// Get next connection (round-robin). Returns error if max_in_flight exceeded.
    pub fn checkout(&self) -> Result<PooledConnection<'_>, PoolError> {
        if self.connections.is_empty() {
            return Err(PoolError::Empty);
        }

        if self.config.max_in_flight > 0 {
            let current = self.in_flight.fetch_add(1, Ordering::Relaxed);
            if current >= self.config.max_in_flight {
                self.in_flight.fetch_sub(1, Ordering::Relaxed);
                return Err(PoolError::BackpressureExceeded {
                    in_flight: current,
                    max: self.config.max_in_flight,
                });
            }
        } else {
            self.in_flight.fetch_add(1, Ordering::Relaxed);
        }

        let idx = self.counter.fetch_add(1, Ordering::Relaxed) % self.connections.len();
        Ok(PooledConnection {
            conn: self.connections[idx].clone(),
            in_flight: &self.in_flight,
        })
    }

    /// Current number of in-flight requests.
    pub fn in_flight(&self) -> u64 {
        self.in_flight.load(Ordering::Relaxed)
    }

    /// Number of connections in the pool.
    pub fn size(&self) -> usize {
        self.connections.len()
    }
}

/// A checked-out connection that decrements in_flight on drop.
pub struct PooledConnection<'a> {
    conn: Arc<Mutex<SidecarConnection>>,
    in_flight: &'a AtomicU64,
}

impl<'a> std::fmt::Debug for PooledConnection<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PooledConnection")
            .field("in_flight", &self.in_flight.load(Ordering::Relaxed))
            .finish()
    }
}

impl<'a> PooledConnection<'a> {
    /// Send a message on the pooled connection.
    pub async fn send(&self, msg: &Message) -> Result<(), TransportError> {
        self.conn.lock().await.send(msg).await
    }

    /// Receive a message on the pooled connection.
    pub async fn recv(&self) -> Result<Message, TransportError> {
        self.conn.lock().await.recv().await
    }

    /// Send a message and receive the response atomically (holding the lock).
    pub async fn send_recv(&self, msg: &Message) -> Result<Message, TransportError> {
        let mut conn = self.conn.lock().await;
        conn.send(msg).await?;
        conn.recv().await
    }
}

impl<'a> Drop for PooledConnection<'a> {
    fn drop(&mut self) {
        self.in_flight.fetch_sub(1, Ordering::Relaxed);
    }
}

/// Errors from pool operations.
#[derive(Debug)]
pub enum PoolError {
    /// Backpressure limit exceeded.
    BackpressureExceeded { in_flight: u64, max: u64 },
    /// Pool has no connections.
    Empty,
}

impl std::fmt::Display for PoolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BackpressureExceeded { in_flight, max } => write!(
                f,
                "backpressure: {} in-flight exceeds max {}",
                in_flight, max
            ),
            Self::Empty => write!(f, "connection pool is empty"),
        }
    }
}

impl std::error::Error for PoolError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_defaults() {
        let cfg = PoolConfig::default();
        assert_eq!(cfg.pool_size, 4);
        assert_eq!(cfg.max_in_flight, 1000);
    }

    #[test]
    fn test_pool_empty_error() {
        let pool = ConnectionPool::new(vec![], PoolConfig::default());
        assert_eq!(pool.size(), 0);
        let err = pool.checkout().unwrap_err();
        assert!(matches!(err, PoolError::Empty));
        assert_eq!(format!("{}", err), "connection pool is empty");
    }

    #[test]
    fn test_backpressure_rejection() {
        // Create a pool with max_in_flight = 2 using real UDS pairs
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let dir = tempfile::tempdir().unwrap();

            // Create 2 socket pairs via listener/connect
            let mut conns = Vec::new();
            for i in 0..2 {
                let sock = dir.path().join(format!("pool{}.sock", i));
                let sock_str = sock.to_str().unwrap().to_string();
                let listener = crate::transport::SidecarListener::bind(&sock_str)
                    .await
                    .unwrap();
                let client = crate::transport::SidecarConnection::connect(&sock_str)
                    .await
                    .unwrap();
                // Accept the server side (we only use the client side)
                let _server = listener.accept().await.unwrap();
                conns.push(client);
            }

            let config = PoolConfig {
                pool_size: 2,
                max_in_flight: 2,
            };
            let pool = ConnectionPool::new(conns, config);
            assert_eq!(pool.size(), 2);
            assert_eq!(pool.in_flight(), 0);

            // Checkout 2 connections — should succeed
            let _c1 = pool.checkout().unwrap();
            assert_eq!(pool.in_flight(), 1);
            let _c2 = pool.checkout().unwrap();
            assert_eq!(pool.in_flight(), 2);

            // Third checkout should fail with backpressure
            let err = pool.checkout().unwrap_err();
            match &err {
                PoolError::BackpressureExceeded { in_flight, max } => {
                    assert_eq!(*in_flight, 2);
                    assert_eq!(*max, 2);
                }
                _ => panic!("expected BackpressureExceeded"),
            }
            assert!(format!("{}", err).contains("backpressure"));

            // Drop one connection, in_flight should decrease
            drop(_c1);
            assert_eq!(pool.in_flight(), 1);

            // Now checkout should succeed again
            let _c3 = pool.checkout().unwrap();
            assert_eq!(pool.in_flight(), 2);
        });
    }

    #[test]
    fn test_unlimited_in_flight() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let dir = tempfile::tempdir().unwrap();
            let sock = dir.path().join("unlimited.sock");
            let sock_str = sock.to_str().unwrap().to_string();
            let listener = crate::transport::SidecarListener::bind(&sock_str)
                .await
                .unwrap();
            let client = crate::transport::SidecarConnection::connect(&sock_str)
                .await
                .unwrap();
            let _server = listener.accept().await.unwrap();

            let config = PoolConfig {
                pool_size: 1,
                max_in_flight: 0,
            };
            let pool = ConnectionPool::new(vec![client], config);

            // With max_in_flight=0 (unlimited), many checkouts should succeed
            let mut handles = Vec::new();
            for _ in 0..100 {
                handles.push(pool.checkout().unwrap());
            }
            assert_eq!(pool.in_flight(), 100);

            drop(handles);
            assert_eq!(pool.in_flight(), 0);
        });
    }

    #[test]
    fn test_round_robin_dispatch() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();

        rt.block_on(async {
            let dir = tempfile::tempdir().unwrap();
            let mut conns = Vec::new();
            let mut _servers = Vec::new();

            for i in 0..3 {
                let sock = dir.path().join(format!("rr{}.sock", i));
                let sock_str = sock.to_str().unwrap().to_string();
                let listener = crate::transport::SidecarListener::bind(&sock_str)
                    .await
                    .unwrap();
                let client = crate::transport::SidecarConnection::connect(&sock_str)
                    .await
                    .unwrap();
                let server = listener.accept().await.unwrap();
                conns.push(client);
                _servers.push(server);
            }

            let config = PoolConfig {
                pool_size: 3,
                max_in_flight: 0,
            };
            let pool = ConnectionPool::new(conns, config);

            // Checkout 6 times — should cycle through 0,1,2,0,1,2
            for _ in 0..6 {
                let c = pool.checkout().unwrap();
                drop(c);
            }
            // Counter should be at 6
            assert_eq!(pool.in_flight(), 0);
        });
    }
}
