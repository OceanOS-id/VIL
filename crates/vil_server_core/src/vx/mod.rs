//! VX — Process-Oriented Server Architecture (Tri-Lane)
//!
//! Returns vil-server to its original Tri-Lane architecture where every
//! service is a VIL Process communicating via SHM descriptor queues.
//!
//! # Architecture
//!
//! ```text
//!   HTTP Request
//!       |
//!   HttpIngress (raw bytes -> SHM)
//!       |
//!   RequestDescriptor (Trigger Lane)
//!       |
//!   ServiceProcess (endpoint handler)
//!       |
//!   ResponseDescriptor (Data Lane)
//!       |
//!   HTTP Response
//! ```
//!
//! # Phase 1 (current)
//!
//! VilApp builds an Axum router from public ServiceProcess definitions
//! and delegates to the existing VilServer for HTTP boundary. Tri-Lane
//! routing is wired but the HTTP ingress still uses Axum extractors.
//!
//! # Phase 2 (future)
//!
//! Raw bytes land in SHM at the TCP layer. The endpoint Process reads a
//! RequestDescriptor from the Trigger Lane and parses the body itself.

pub mod app;
pub mod cleanup;
pub mod ctx;
pub mod descriptor;
pub mod egress;
pub mod endpoint;
pub mod ingress;
pub mod kernel;
pub mod service;
pub mod tri_lane;
