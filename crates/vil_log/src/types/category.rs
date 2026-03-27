// =============================================================================
// vil_log::types::category — LogCategory enum
// =============================================================================

use std::fmt;

/// Semantic category of a log event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum LogCategory {
    Access   = 0,
    App      = 1,
    System   = 2,
    Security = 3,
    Ai       = 4,
    Db       = 5,
    Mq       = 6,
}

impl fmt::Display for LogCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            LogCategory::Access   => "ACCESS",
            LogCategory::App      => "APP",
            LogCategory::System   => "SYSTEM",
            LogCategory::Security => "SECURITY",
            LogCategory::Ai       => "AI",
            LogCategory::Db       => "DB",
            LogCategory::Mq       => "MQ",
        };
        f.write_str(s)
    }
}

impl From<u8> for LogCategory {
    fn from(v: u8) -> Self {
        match v {
            0 => LogCategory::Access,
            1 => LogCategory::App,
            2 => LogCategory::System,
            3 => LogCategory::Security,
            4 => LogCategory::Ai,
            5 => LogCategory::Db,
            6 => LogCategory::Mq,
            _ => LogCategory::App,
        }
    }
}
