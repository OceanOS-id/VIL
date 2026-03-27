// =============================================================================
// vil_trigger_email::config — EmailConfig
// =============================================================================
//
// Configuration for the IMAP IDLE email trigger.
// =============================================================================

/// Configuration for the VIL email IMAP trigger.
///
/// # Example YAML
/// ```yaml
/// email:
///   imap_host: "imap.example.com"
///   port: 993
///   username: "user@example.com"
///   password: "secret"
///   folder: "INBOX"
/// ```
#[derive(Debug, Clone)]
pub struct EmailConfig {
    /// IMAP server hostname.
    pub imap_host: String,
    /// IMAP server port (typically 993 for TLS).
    pub port: u16,
    /// IMAP account username (usually the email address).
    pub username: String,
    /// IMAP account password.
    pub password: String,
    /// Mailbox folder to watch (default: `"INBOX"`).
    pub folder: String,
}

impl EmailConfig {
    /// Construct a new `EmailConfig` with the given credentials.
    pub fn new(
        imap_host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
        folder: impl Into<String>,
    ) -> Self {
        Self {
            imap_host: imap_host.into(),
            port,
            username: username.into(),
            password: password.into(),
            folder: folder.into(),
        }
    }

    /// Return `"host:port"` connection string.
    pub fn socket_addr(&self) -> String {
        format!("{}:{}", self.imap_host, self.port)
    }
}
