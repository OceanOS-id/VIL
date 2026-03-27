// =============================================================================
// VIL Server Auth — IP Allowlist/Blocklist Middleware
// =============================================================================
//
// Filter requests based on client IP address.
// Supports both allowlist (whitelist) and blocklist (blacklist) modes.
//
// Modes:
//   Allowlist: only listed IPs can access (deny by default)
//   Blocklist: listed IPs are denied (allow by default)
//
// Supports CIDR notation: 10.0.0.0/8, 192.168.1.0/24

use std::net::IpAddr;
use std::sync::Arc;

/// IP filter mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IpFilterMode {
    /// Only listed IPs are allowed (deny all others)
    Allowlist,
    /// Listed IPs are denied (allow all others)
    Blocklist,
}

/// IP address filter.
#[derive(Clone)]
pub struct IpFilter {
    mode: IpFilterMode,
    /// List of IP addresses (exact match)
    ips: Arc<Vec<IpAddr>>,
    /// List of CIDR ranges as (network, prefix_len)
    cidrs: Arc<Vec<(IpAddr, u8)>>,
}

impl IpFilter {
    /// Create a new allowlist filter (deny by default).
    pub fn allowlist() -> Self {
        Self {
            mode: IpFilterMode::Allowlist,
            ips: Arc::new(Vec::new()),
            cidrs: Arc::new(Vec::new()),
        }
    }

    /// Create a new blocklist filter (allow by default).
    pub fn blocklist() -> Self {
        Self {
            mode: IpFilterMode::Blocklist,
            ips: Arc::new(Vec::new()),
            cidrs: Arc::new(Vec::new()),
        }
    }

    /// Add an IP address to the filter list.
    pub fn add_ip(mut self, ip: IpAddr) -> Self {
        Arc::make_mut(&mut self.ips).push(ip);
        self
    }

    /// Add a CIDR range to the filter list.
    /// Example: "192.168.1.0/24"
    pub fn add_cidr(mut self, cidr: &str) -> Self {
        if let Some((ip_str, prefix_str)) = cidr.split_once('/') {
            if let (Ok(ip), Ok(prefix)) = (ip_str.parse::<IpAddr>(), prefix_str.parse::<u8>()) {
                Arc::make_mut(&mut self.cidrs).push((ip, prefix));
            }
        }
        self
    }

    /// Check if an IP is allowed through the filter.
    pub fn is_allowed(&self, ip: &IpAddr) -> bool {
        let matched = self.matches(ip);
        match self.mode {
            IpFilterMode::Allowlist => matched,   // Must be in list
            IpFilterMode::Blocklist => !matched,   // Must NOT be in list
        }
    }

    /// Check if an IP matches any rule in the filter.
    fn matches(&self, ip: &IpAddr) -> bool {
        // Exact match
        if self.ips.contains(ip) {
            return true;
        }

        // CIDR match
        for (network, prefix_len) in self.cidrs.iter() {
            if cidr_match(ip, network, *prefix_len) {
                return true;
            }
        }

        false
    }

    pub fn mode(&self) -> IpFilterMode {
        self.mode
    }
}

/// Check if an IP matches a CIDR range.
fn cidr_match(ip: &IpAddr, network: &IpAddr, prefix_len: u8) -> bool {
    match (ip, network) {
        (IpAddr::V4(ip), IpAddr::V4(net)) => {
            let ip_bits = u32::from(*ip);
            let net_bits = u32::from(*net);
            let mask = if prefix_len >= 32 { u32::MAX } else { u32::MAX << (32 - prefix_len) };
            (ip_bits & mask) == (net_bits & mask)
        }
        (IpAddr::V6(ip), IpAddr::V6(net)) => {
            let ip_bits = u128::from(*ip);
            let net_bits = u128::from(*net);
            let mask = if prefix_len >= 128 { u128::MAX } else { u128::MAX << (128 - prefix_len) };
            (ip_bits & mask) == (net_bits & mask)
        }
        _ => false, // IPv4/IPv6 mismatch
    }
}
