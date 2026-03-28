// =============================================================================
// vil_ws::room — RoomManager
// =============================================================================
//
// Room/channel management for the WebSocket server.
// Tracks which client IDs are subscribed to which rooms.
// Thread-safe via DashMap for concurrent access from multiple connection tasks.
// =============================================================================

use dashmap::DashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc;

use crate::error::WsFault;
use vil_log::dict::register_str;

/// Per-connection sender for outbound WebSocket messages.
pub type ClientSender = mpsc::UnboundedSender<Vec<u8>>;

/// Unique identifier for a WebSocket connection.
pub type ClientId = u64;

/// Room/channel manager.
///
/// Maps room names (as FxHash u32) to the set of client IDs subscribed.
/// Maps client IDs to their outbound `mpsc::UnboundedSender<Vec<u8>>`.
#[derive(Clone)]
pub struct RoomManager {
    /// room_hash → set of ClientIds
    rooms: Arc<DashMap<u32, Vec<ClientId>>>,
    /// client_id → outbound sender
    clients: Arc<DashMap<ClientId, ClientSender>>,
    /// Monotonic ID counter for new connections.
    next_id: Arc<AtomicU64>,
}

impl RoomManager {
    /// Create a new empty `RoomManager`.
    pub fn new() -> Self {
        Self {
            rooms: Arc::new(DashMap::new()),
            clients: Arc::new(DashMap::new()),
            next_id: Arc::new(AtomicU64::new(1)),
        }
    }

    /// Register a new client connection and return its unique ID.
    pub fn register_client(&self, sender: ClientSender) -> ClientId {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        self.clients.insert(id, sender);
        id
    }

    /// Remove a client from all rooms and the client registry.
    pub fn remove_client(&self, client_id: ClientId) {
        self.clients.remove(&client_id);
        for mut room in self.rooms.iter_mut() {
            room.value_mut().retain(|&id| id != client_id);
        }
    }

    /// Subscribe a client to a named room.
    pub fn join_room(&self, client_id: ClientId, room: &str) {
        let room_hash = register_str(room);
        self.rooms
            .entry(room_hash)
            .or_insert_with(Vec::new)
            .push(client_id);
    }

    /// Unsubscribe a client from a named room.
    pub fn leave_room(&self, client_id: ClientId, room: &str) {
        let room_hash = register_str(room);
        if let Some(mut members) = self.rooms.get_mut(&room_hash) {
            members.retain(|&id| id != client_id);
        }
    }

    /// Broadcast a message to all clients in a named room.
    ///
    /// Returns Ok(sent_count) on success. Failed sends are silently dropped
    /// (disconnected client) and the faulty count is returned in the Err.
    pub fn broadcast_room(&self, room: &str, msg: &[u8]) -> Result<u32, WsFault> {
        let room_hash = register_str(room);
        let members = self
            .rooms
            .get(&room_hash)
            .ok_or(WsFault::RoomNotFound { room_hash })?;

        let mut sent = 0u32;
        let mut failed = 0u32;
        for &client_id in members.iter() {
            if let Some(sender) = self.clients.get(&client_id) {
                if sender.send(msg.to_vec()).is_ok() {
                    sent += 1;
                } else {
                    failed += 1;
                }
            }
        }

        if failed > 0 {
            Err(WsFault::BroadcastPartialFail {
                room_hash,
                failed_count: failed,
            })
        } else {
            Ok(sent)
        }
    }

    /// Send a message to a single client by ID.
    pub fn send_to(&self, client_id: ClientId, msg: &[u8]) -> Result<(), WsFault> {
        let sender = self.clients.get(&client_id).ok_or(WsFault::RoomNotFound {
            room_hash: client_id as u32,
        })?;
        sender.send(msg.to_vec()).map_err(|_| WsFault::SendFailed {
            topic_hash: client_id as u32,
            reason_code: 1,
        })
    }

    /// Broadcast a message to ALL connected clients.
    pub fn broadcast_all(&self, msg: &[u8]) -> u32 {
        let mut sent = 0u32;
        for client in self.clients.iter() {
            if client.value().send(msg.to_vec()).is_ok() {
                sent += 1;
            }
        }
        sent
    }

    /// Return the current number of connected clients.
    pub fn connection_count(&self) -> usize {
        self.clients.len()
    }

    /// Return the current number of rooms.
    pub fn room_count(&self) -> usize {
        self.rooms.len()
    }
}

impl Default for RoomManager {
    fn default() -> Self {
        Self::new()
    }
}
