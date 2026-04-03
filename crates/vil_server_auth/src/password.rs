// =============================================================================
// VIL Password — Argon2id password hashing
// =============================================================================

use vil_server_core::VilError;

/// Password hashing and verification using Argon2id (OWASP recommended).
pub struct VilPassword;

impl VilPassword {
    /// Hash a plaintext password. Returns Argon2id PHC string.
    ///
    /// # Example
    /// ```ignore
    /// let hash = VilPassword::hash("mypassword")?;
    /// // hash = "$argon2id$v=19$m=19456,t=2,p=1$..."
    /// ```
    pub fn hash(password: &str) -> Result<String, VilError> {
        use argon2::{Argon2, PasswordHasher};
        use password_hash::SaltString;

        let salt = SaltString::generate(&mut rand::thread_rng());
        Argon2::default()
            .hash_password(password.as_bytes(), &salt)
            .map(|h| h.to_string())
            .map_err(|e| VilError::internal(format!("password hash failed: {e}")))
    }

    /// Verify a plaintext password against a stored hash.
    ///
    /// # Example
    /// ```ignore
    /// let valid = VilPassword::verify("mypassword", &stored_hash)?;
    /// ```
    pub fn verify(password: &str, hash: &str) -> Result<bool, VilError> {
        use argon2::{Argon2, PasswordHash, PasswordVerifier};

        let parsed = PasswordHash::new(hash)
            .map_err(|e| VilError::internal(format!("invalid password hash: {e}")))?;
        Ok(Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok())
    }
}
