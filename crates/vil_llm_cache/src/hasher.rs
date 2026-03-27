/// FNV-1a 64-bit hash for exact-match lookups.
///
/// Fast, non-cryptographic hash suitable for cache keys.
pub fn fnv1a_hash(data: &[u8]) -> u64 {
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x00000100000001B3;

    let mut hash = FNV_OFFSET_BASIS;
    for &byte in data {
        hash ^= byte as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Hash a serialized messages string for use as an exact-match cache key.
pub fn hash_messages(messages: &str) -> u64 {
    fnv1a_hash(messages.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1a_deterministic() {
        let data = b"hello world";
        assert_eq!(fnv1a_hash(data), fnv1a_hash(data));
    }

    #[test]
    fn fnv1a_different_inputs() {
        assert_ne!(fnv1a_hash(b"hello"), fnv1a_hash(b"world"));
    }

    #[test]
    fn fnv1a_empty() {
        // Empty input should still produce a valid hash (the offset basis).
        let h = fnv1a_hash(b"");
        assert_ne!(h, 0);
    }

    #[test]
    fn hash_messages_works() {
        let h1 = hash_messages("What is Rust?");
        let h2 = hash_messages("What is Rust?");
        assert_eq!(h1, h2);
        assert_ne!(hash_messages("a"), hash_messages("b"));
    }
}
