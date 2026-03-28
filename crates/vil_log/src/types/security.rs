// =============================================================================
// vil_log::types::security — SecurityPayload
// =============================================================================
//
// Security event payload: auth, authz, audit, anomaly detection.
// =============================================================================

/// Security event payload. Fits in 192 bytes.
#[derive(Debug, Clone, Copy, zerocopy::FromBytes, zerocopy::Immutable, zerocopy::KnownLayout)]
#[repr(C)]
pub struct SecurityPayload {
    /// FxHash of the actor/user identity.
    pub actor_hash: u32,
    /// FxHash of the resource being accessed.
    pub resource_hash: u32,
    /// FxHash of the action attempted.
    pub action_hash: u32,
    /// Client IPv4 address (packed u32, big-endian).
    pub client_ip: u32,
    /// Event type: 0=auth 1=authz 2=audit 3=anomaly 4=intrusion 5=policy
    pub event_type: u8,
    /// Outcome: 0=allow 1=deny 2=challenge 3=error
    pub outcome: u8,
    /// Risk score 0–255.
    pub risk_score: u8,
    /// MFA factor used: 0=none 1=totp 2=sms 3=hw_key
    pub mfa_factor: u8,
    /// Session ID (first 8 bytes of token).
    pub session_id: u64,
    /// Number of failed attempts (within window).
    pub failed_attempts: u16,
    /// Geo-region code (ISO 3166 numeric, 0 = unknown).
    pub geo_region: u16,
    /// Padding.
    pub _pad: [u8; 4],
    /// Inline security context metadata (msgpack).
    pub meta_bytes: [u8; 152],
}

impl Default for SecurityPayload {
    fn default() -> Self {
        unsafe { std::mem::zeroed() }
    }
}

const _: () = {
    assert!(
        std::mem::size_of::<SecurityPayload>() <= 192,
        "SecurityPayload must fit within 192 bytes"
    );
};
