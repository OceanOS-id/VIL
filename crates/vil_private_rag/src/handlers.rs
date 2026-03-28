use vil_server::prelude::*;

use crate::audit::PrivacyAuditLog;
use std::sync::{Arc, RwLock};

#[derive(Debug, Serialize)]
pub struct PrivateRagStatsBody {
    pub audit_entry_count: usize,
    pub audit_empty: bool,
    pub pii_patterns: Vec<String>,
    pub operations: Vec<String>,
    pub version: String,
}

pub async fn stats_handler(ctx: ServiceCtx) -> HandlerResult<VilResponse<PrivateRagStatsBody>> {
    let audit_log = ctx.state::<Arc<RwLock<PrivacyAuditLog>>>()?;
    let log = audit_log
        .read()
        .map_err(|_| VilError::internal("lock poisoned"))?;
    Ok(VilResponse::ok(PrivateRagStatsBody {
        audit_entry_count: log.len(),
        audit_empty: log.is_empty(),
        pii_patterns: vec![
            "email".into(),
            "phone".into(),
            "ssn".into(),
            "credit_card".into(),
        ],
        operations: vec!["redact".into(), "anonymize".into(), "audit".into()],
        version: env!("CARGO_PKG_VERSION").into(),
    }))
}
