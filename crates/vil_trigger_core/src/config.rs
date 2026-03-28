// =============================================================================
// vil_trigger_core::config — TriggerConfig, TriggerKind
// =============================================================================
//
// Setup-time configuration (External layout profile — heap types allowed here).
// Not used on the hot event-emission path.
// =============================================================================

/// The category of event source represented by a trigger.
///
/// Stored as a plain `u8` tag; resolved to a human-readable label via
/// `vil_log::dict` during drain output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TriggerKind {
    /// Cron / scheduled interval trigger.
    Cron = 0,
    /// Filesystem / directory watcher trigger.
    Fs = 1,
    /// Database change-data-capture trigger.
    Cdc = 2,
    /// HTTP webhook receiver trigger.
    Webhook = 3,
    /// Email (IMAP) trigger.
    Email = 4,
    /// IoT device (MQTT/CoAP) trigger.
    Iot = 5,
    /// EVM blockchain log subscription trigger.
    Evm = 6,
    /// Custom / user-defined trigger kind.
    Custom = 255,
}

/// Setup-time configuration for a trigger source.
///
/// **Layout profile: External** — may contain `&'static str` references used
/// only at initialisation, never on the hot event path.
#[derive(Debug, Clone, Copy)]
pub struct TriggerConfig {
    /// Numeric identity for this trigger instance.
    /// Must be unique within the process.
    pub id: u64,

    /// Semantic kind of the trigger.
    pub kind: TriggerKind,

    /// Whether the trigger starts in an active state.
    pub enabled: bool,

    /// Human-readable label for log dict registration.
    pub label: &'static str,
}

impl TriggerConfig {
    /// Construct a new enabled trigger config.
    #[inline]
    pub fn new(id: u64, kind: TriggerKind, label: &'static str) -> Self {
        Self {
            id,
            kind,
            enabled: true,
            label,
        }
    }
}
