// =============================================================================
// vil_validate::traits — Core Validation Types
// =============================================================================
// Defines the contract for all validation passes.
// Each pass inspects a specific part of the Semantic IR and
// reports a list of diagnostics (errors/warnings).
// =============================================================================

use std::fmt;

use vil_ir::core::WorkflowIR;

/// Severity level of a validation finding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

/// Represents an issue or suggestion found by a validation pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    /// Unique identifier for the error type, grep-friendly (e.g. "E01-LAYOUT_VIOLATION")
    pub code: String,
    /// Informative error message for the developer.
    pub message: String,
    /// Location of the problematic IR node (process/port/message name).
    pub context: String,
}

impl Diagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>, context: impl Into<String>) -> Self {
        Self {
            severity: Severity::Error,
            code: code.into(),
            message: message.into(),
            context: context.into(),
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>, context: impl Into<String>) -> Self {
        Self {
            severity: Severity::Warning,
            code: code.into(),
            message: message.into(),
            context: context.into(),
        }
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = match self.severity {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
        };
        write!(
            f,
            "{}[{}]: {} (at {})",
            prefix, self.code, self.message, self.context
        )
    }
}

/// Overall report from a pass execution.
#[derive(Debug, Default)]
pub struct ValidationReport {
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&mut self, diag: Diagnostic) {
        self.diagnostics.push(diag);
    }

    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(|d| d.severity == Severity::Error)
    }

    /// Merge another report into this one.
    pub fn merge(&mut self, mut other: ValidationReport) {
        self.diagnostics.append(&mut other.diagnostics);
    }
}

/// Contract for a validation pass.
pub trait ValidationPass {
    /// Short name identifying this pass (e.g. "LayoutLegalityPass").
    fn name(&self) -> &'static str;

    /// Run validation against the entire workflow IR.
    fn run(&self, ir: &WorkflowIR) -> ValidationReport;
}
