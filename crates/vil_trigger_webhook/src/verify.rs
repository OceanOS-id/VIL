// =============================================================================
// vil_trigger_webhook::verify — HMAC-SHA256 signature verification
// =============================================================================
//
// Verifies the `X-Hub-Signature-256` header (or custom equivalent) against
// the raw request body using HMAC-SHA256.
//
// No println!, tracing, or log crate — COMPLIANCE.md §8.
// =============================================================================

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

/// Verify an HMAC-SHA256 signature against `body`.
///
/// `signature_hex` should be the hex-encoded digest, optionally prefixed with
/// `"sha256="` (GitHub / Stripe style).
///
/// Returns `true` if the signature matches, `false` otherwise.
///
/// # Constant-time comparison
/// Uses `hmac::Mac::verify_slice` which is constant-time to prevent
/// timing side-channel attacks.
pub fn verify_hmac(secret: &[u8], body: &[u8], signature_hex: &str) -> bool {
    // Strip optional "sha256=" prefix.
    let hex = signature_hex
        .strip_prefix("sha256=")
        .unwrap_or(signature_hex);

    // Decode hex to raw bytes.
    let Ok(sig_bytes) = hex::decode(hex) else {
        return false;
    };

    let Ok(mut mac) = HmacSha256::new_from_slice(secret) else {
        return false;
    };

    mac.update(body);
    mac.verify_slice(&sig_bytes).is_ok()
}

// ---------------------------------------------------------------------------
// hex decode helper — avoids pulling in the hex crate separately.
// ---------------------------------------------------------------------------
mod hex {
    pub fn decode(s: &str) -> Result<Vec<u8>, ()> {
        if s.len() % 2 != 0 {
            return Err(());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| ()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_signature_accepted() {
        use hmac::{Hmac, Mac};
        use sha2::Sha256;
        type H = Hmac<Sha256>;

        let secret = b"test-secret";
        let body   = b"hello world";

        let mut mac = H::new_from_slice(secret).unwrap();
        mac.update(body);
        let result  = mac.finalize().into_bytes();
        let hex_sig = result.iter().map(|b| format!("{:02x}", b)).collect::<String>();
        let prefixed = format!("sha256={}", hex_sig);

        assert!(verify_hmac(secret, body, &prefixed));
    }

    #[test]
    fn invalid_signature_rejected() {
        assert!(!verify_hmac(b"secret", b"body", "sha256=deadbeef"));
    }
}
