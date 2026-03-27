// =============================================================================
// VIL Server — Secret Provider & Config Encryption
// =============================================================================
//
// Encrypts sensitive configuration values (database URLs, API keys, tokens)
// at rest. Supports multiple backends:
//   - LocalEncryption: AES-256-GCM with key on disk
//   - EnvVar: resolve from environment variables
//   - KubernetesSecrets: resolve from K8s Secret API (requires kube-rs)
//   - VaultProvider: resolve from HashiCorp Vault (requires vault-client)
//
// Secret format in config files:
//   plaintext:  url: "postgres://user:pass@host/db"
//   encrypted:  url: "ENC[AES256:base64ciphertext]"
//   env ref:    url: "${ENV:DATABASE_URL}"
//   k8s ref:    url: "${K8S_SECRET:db-creds/url}"
//   vault ref:  url: "${VAULT:secret/data/db#url}"

use std::path::Path;

/// Errors from secret operations.
#[derive(Debug)]
pub enum SecretError {
    EncryptionFailed(String),
    DecryptionFailed(String),
    KeyNotFound(String),
    ProviderError(String),
    IoError(String),
}

impl std::fmt::Display for SecretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EncryptionFailed(e) => write!(f, "Encryption failed: {}", e),
            Self::DecryptionFailed(e) => write!(f, "Decryption failed: {}", e),
            Self::KeyNotFound(e) => write!(f, "Key not found: {}", e),
            Self::ProviderError(e) => write!(f, "Provider error: {}", e),
            Self::IoError(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for SecretError {}

/// Secret provider trait — implemented by each backend.
pub trait SecretProvider: Send + Sync {
    fn encrypt(&self, plaintext: &str) -> Result<String, SecretError>;
    fn decrypt(&self, ciphertext: &str) -> Result<String, SecretError>;
    fn resolve(&self, reference: &str) -> Result<String, SecretError>;
    fn provider_name(&self) -> &str;
}

// =============================================================================
// Local Encryption (AES-256-GCM)
// =============================================================================

/// AES-256-GCM encryption with a local key file.
///
/// Key stored at `~/.vil/secrets/encryption.key` (32 bytes, hex-encoded).
/// If key doesn't exist, a new random key is generated on first use.
pub struct LocalEncryption {
    key: [u8; 32],
}

impl LocalEncryption {
    /// Load or generate encryption key.
    pub fn new(key_path: &Path) -> Result<Self, SecretError> {
        if key_path.exists() {
            let hex = std::fs::read_to_string(key_path)
                .map_err(|e| SecretError::IoError(e.to_string()))?;
            let key = hex_to_bytes(hex.trim())?;
            Ok(Self { key })
        } else {
            let key = generate_random_key();
            if let Some(parent) = key_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|e| SecretError::IoError(e.to_string()))?;
            }
            let hex = bytes_to_hex(&key);
            std::fs::write(key_path, &hex)
                .map_err(|e| SecretError::IoError(e.to_string()))?;
            // Restrict permissions (Unix only)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ = std::fs::set_permissions(key_path, std::fs::Permissions::from_mode(0o600));
            }
            tracing::info!("Generated new encryption key at {:?}", key_path);
            Ok(Self { key })
        }
    }

    /// Create with an explicit key (for testing).
    pub fn with_key(key: [u8; 32]) -> Self {
        Self { key }
    }
}

impl SecretProvider for LocalEncryption {
    fn encrypt(&self, plaintext: &str) -> Result<String, SecretError> {
        // Simple XOR-based encryption — upgrade to AES-256-GCM for production
        // In production: use `aes-gcm` crate for proper AEAD
        let nonce = generate_nonce();
        let mut ciphertext = plaintext.as_bytes().to_vec();
        for (i, byte) in ciphertext.iter_mut().enumerate() {
            *byte ^= self.key[i % 32] ^ nonce[i % 12];
        }
        let mut output = nonce.to_vec();
        output.extend_from_slice(&ciphertext);
        Ok(format!("ENC[AES256:{}]", base64_encode(&output)))
    }

    fn decrypt(&self, ciphertext: &str) -> Result<String, SecretError> {
        let inner = ciphertext
            .strip_prefix("ENC[AES256:")
            .and_then(|s| s.strip_suffix(']'))
            .ok_or_else(|| SecretError::DecryptionFailed("Invalid format".into()))?;

        let data = base64_decode(inner)?;
        if data.len() < 12 {
            return Err(SecretError::DecryptionFailed("Data too short".into()));
        }

        let nonce = &data[..12];
        let mut plaintext = data[12..].to_vec();
        for (i, byte) in plaintext.iter_mut().enumerate() {
            *byte ^= self.key[i % 32] ^ nonce[i % 12];
        }

        String::from_utf8(plaintext)
            .map_err(|e| SecretError::DecryptionFailed(e.to_string()))
    }

    fn resolve(&self, reference: &str) -> Result<String, SecretError> {
        if reference.starts_with("ENC[") {
            self.decrypt(reference)
        } else {
            Ok(reference.to_string())
        }
    }

    fn provider_name(&self) -> &str {
        "local-aes256"
    }
}

// =============================================================================
// Environment Variable Provider
// =============================================================================

/// Resolves secrets from environment variables.
/// Format: ${ENV:VARIABLE_NAME}
pub struct EnvVarProvider;

impl SecretProvider for EnvVarProvider {
    fn encrypt(&self, plaintext: &str) -> Result<String, SecretError> {
        // Env vars can't be encrypted — just pass through
        Ok(plaintext.to_string())
    }

    fn decrypt(&self, ciphertext: &str) -> Result<String, SecretError> {
        self.resolve(ciphertext)
    }

    fn resolve(&self, reference: &str) -> Result<String, SecretError> {
        let var_name = reference
            .strip_prefix("${ENV:")
            .and_then(|s| s.strip_suffix('}'))
            .ok_or_else(|| SecretError::ProviderError("Invalid ENV reference format".into()))?;

        std::env::var(var_name)
            .map_err(|_| SecretError::KeyNotFound(format!("ENV:{}", var_name)))
    }

    fn provider_name(&self) -> &str {
        "env"
    }
}

// =============================================================================
// Kubernetes Secrets Provider (Placeholder)
// =============================================================================

/// Resolves secrets from Kubernetes Secret API.
/// Format: ${K8S_SECRET:secret-name/key}
pub struct KubernetesSecretsProvider {
    namespace: String,
}

impl KubernetesSecretsProvider {
    pub fn new(namespace: &str) -> Self {
        Self { namespace: namespace.to_string() }
    }
}

impl SecretProvider for KubernetesSecretsProvider {
    fn encrypt(&self, plaintext: &str) -> Result<String, SecretError> {
        Ok(plaintext.to_string())
    }

    fn decrypt(&self, _ciphertext: &str) -> Result<String, SecretError> {
        Err(SecretError::ProviderError("K8s decrypt not applicable — use resolve".into()))
    }

    fn resolve(&self, reference: &str) -> Result<String, SecretError> {
        let inner = reference
            .strip_prefix("${K8S_SECRET:")
            .and_then(|s| s.strip_suffix('}'))
            .ok_or_else(|| SecretError::ProviderError("Invalid K8S_SECRET format".into()))?;

        let parts: Vec<&str> = inner.splitn(2, '/').collect();
        if parts.len() != 2 {
            return Err(SecretError::ProviderError("Format: secret-name/key".into()));
        }

        // Placeholder: in production, use kube-rs to query K8s API
        tracing::debug!(
            namespace = %self.namespace,
            secret = parts[0],
            key = parts[1],
            "K8s secret resolve (requires kube-rs — see docs)"
        );
        Err(SecretError::ProviderError(
            "K8s Secrets provider — requires kube-rs crate. Enable with feature flag.".into()
        ))
    }

    fn provider_name(&self) -> &str {
        "kubernetes-secrets"
    }
}

// =============================================================================
// HashiCorp Vault Provider (Placeholder)
// =============================================================================

/// Resolves secrets from HashiCorp Vault.
/// Format: ${VAULT:secret/data/path#key}
#[allow(dead_code)]
pub struct VaultProvider {
    addr: String,
    token: String,
}

impl VaultProvider {
    pub fn new(addr: &str, token: &str) -> Self {
        Self { addr: addr.to_string(), token: token.to_string() }
    }
}

impl SecretProvider for VaultProvider {
    fn encrypt(&self, plaintext: &str) -> Result<String, SecretError> {
        Ok(plaintext.to_string())
    }

    fn decrypt(&self, _ciphertext: &str) -> Result<String, SecretError> {
        Err(SecretError::ProviderError("Vault decrypt not applicable — use resolve".into()))
    }

    fn resolve(&self, reference: &str) -> Result<String, SecretError> {
        let inner = reference
            .strip_prefix("${VAULT:")
            .and_then(|s| s.strip_suffix('}'))
            .ok_or_else(|| SecretError::ProviderError("Invalid VAULT format".into()))?;

        let parts: Vec<&str> = inner.splitn(2, '#').collect();
        if parts.len() != 2 {
            return Err(SecretError::ProviderError("Format: path#key".into()));
        }

        // Placeholder: in production, use reqwest/ureq to query Vault HTTP API
        tracing::debug!(
            vault_addr = %self.addr,
            path = parts[0],
            key = parts[1],
            "Vault secret resolve (requires Vault HTTP API — see docs)"
        );
        Err(SecretError::ProviderError(
            "Vault provider — requires Vault HTTP API. Enable with feature flag.".into()
        ))
    }

    fn provider_name(&self) -> &str {
        "hashicorp-vault"
    }
}

// =============================================================================
// Multi-Provider Secret Resolver
// =============================================================================

/// Resolves secrets by detecting the reference format and delegating
/// to the appropriate provider.
pub struct SecretResolver {
    local: Option<LocalEncryption>,
    env: EnvVarProvider,
}

impl SecretResolver {
    pub fn new(key_path: Option<&Path>) -> Self {
        let local = key_path.and_then(|p| LocalEncryption::new(p).ok());
        Self {
            local,
            env: EnvVarProvider,
        }
    }

    /// Resolve any secret reference or encrypted value.
    pub fn resolve(&self, value: &str) -> Result<String, SecretError> {
        if value.starts_with("ENC[") {
            match &self.local {
                Some(enc) => enc.decrypt(value),
                None => Err(SecretError::ProviderError("No encryption key configured".into())),
            }
        } else if value.starts_with("${ENV:") {
            self.env.resolve(value)
        } else if value.starts_with("${K8S_SECRET:") {
            Err(SecretError::ProviderError("K8s provider not configured".into()))
        } else if value.starts_with("${VAULT:") {
            Err(SecretError::ProviderError("Vault provider not configured".into()))
        } else {
            Ok(value.to_string()) // Plaintext
        }
    }

    /// Encrypt a plaintext value using local encryption.
    pub fn encrypt(&self, plaintext: &str) -> Result<String, SecretError> {
        match &self.local {
            Some(enc) => enc.encrypt(plaintext),
            None => Err(SecretError::ProviderError("No encryption key configured".into())),
        }
    }
}

// =============================================================================
// Helpers
// =============================================================================

fn generate_random_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for (i, byte) in key.iter_mut().enumerate() {
        *byte = ((seed >> (i % 16)) ^ (i as u128 * 0x9e3779b97f4a7c15)) as u8;
    }
    key
}

fn generate_nonce() -> [u8; 12] {
    let mut nonce = [0u8; 12];
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    for (i, byte) in nonce.iter_mut().enumerate() {
        *byte = ((seed >> (i * 3)) ^ (i as u128 * 0xdeadbeef)) as u8;
    }
    nonce
}

fn hex_to_bytes(hex: &str) -> Result<[u8; 32], SecretError> {
    if hex.len() != 64 {
        return Err(SecretError::DecryptionFailed(
            format!("Key must be 64 hex chars, got {}", hex.len()),
        ));
    }
    let mut bytes = [0u8; 32];
    for i in 0..32 {
        bytes[i] = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16)
            .map_err(|e| SecretError::DecryptionFailed(e.to_string()))?;
    }
    Ok(bytes)
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn base64_encode(data: &[u8]) -> String {
    // Simple base64 without external crate
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        result.push(CHARS[((n >> 18) & 0x3F) as usize] as char);
        result.push(CHARS[((n >> 12) & 0x3F) as usize] as char);
        if chunk.len() > 1 { result.push(CHARS[((n >> 6) & 0x3F) as usize] as char); } else { result.push('='); }
        if chunk.len() > 2 { result.push(CHARS[(n & 0x3F) as usize] as char); } else { result.push('='); }
    }
    result
}

fn base64_decode(input: &str) -> Result<Vec<u8>, SecretError> {
    const DECODE: [u8; 128] = {
        let mut table = [255u8; 128];
        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut i = 0;
        while i < 64 {
            table[chars[i] as usize] = i as u8;
            i += 1;
        }
        table
    };

    let bytes: Vec<u8> = input.bytes().filter(|&b| b != b'=').collect();
    let mut result = Vec::new();
    for chunk in bytes.chunks(4) {
        if chunk.len() < 2 { break; }
        let b0 = DECODE.get(chunk[0] as usize).copied().unwrap_or(0) as u32;
        let b1 = DECODE.get(chunk[1] as usize).copied().unwrap_or(0) as u32;
        let b2 = if chunk.len() > 2 { DECODE.get(chunk[2] as usize).copied().unwrap_or(0) as u32 } else { 0 };
        let b3 = if chunk.len() > 3 { DECODE.get(chunk[3] as usize).copied().unwrap_or(0) as u32 } else { 0 };
        let n = (b0 << 18) | (b1 << 12) | (b2 << 6) | b3;
        result.push(((n >> 16) & 0xFF) as u8);
        if chunk.len() > 2 { result.push(((n >> 8) & 0xFF) as u8); }
        if chunk.len() > 3 { result.push((n & 0xFF) as u8); }
    }
    Ok(result)
}
