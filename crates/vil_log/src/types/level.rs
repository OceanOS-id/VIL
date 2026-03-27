// =============================================================================
// vil_log::types::level — LogLevel enum
// =============================================================================

use std::fmt;

/// Severity level for a log event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum LogLevel {
    Trace    = 0,
    Debug    = 1,
    Info     = 2,
    Warn     = 3,
    Error    = 4,
    Fatal    = 5,
}

impl LogLevel {
    /// ANSI color code for terminal display.
    pub fn ansi_color(&self) -> &'static str {
        match self {
            LogLevel::Trace => "\x1b[37m",    // white
            LogLevel::Debug => "\x1b[36m",    // cyan
            LogLevel::Info  => "\x1b[32m",    // green
            LogLevel::Warn  => "\x1b[33m",    // yellow
            LogLevel::Error => "\x1b[31m",    // red
            LogLevel::Fatal => "\x1b[35;1m",  // bold magenta
        }
    }
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info  => "INFO",
            LogLevel::Warn  => "WARN",
            LogLevel::Error => "ERROR",
            LogLevel::Fatal => "FATAL",
        };
        f.write_str(s)
    }
}

impl From<u8> for LogLevel {
    fn from(v: u8) -> Self {
        match v {
            0 => LogLevel::Trace,
            1 => LogLevel::Debug,
            2 => LogLevel::Info,
            3 => LogLevel::Warn,
            4 => LogLevel::Error,
            5 => LogLevel::Fatal,
            _ => LogLevel::Info,
        }
    }
}
