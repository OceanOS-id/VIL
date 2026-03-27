#![allow(dead_code)]
// =============================================================================
// VIL Operator — Controller (Reconciliation Loop)
// =============================================================================

use crate::crd::VilServer;

/// Reconciliation result.
pub enum ReconcileAction {
    Create,
    Update,
    NoChange,
    Delete,
}

/// Determine what action to take for a VilServer CR.
pub fn determine_action(server: &VilServer) -> ReconcileAction {
    if server.metadata.deletion_timestamp.is_some() {
        return ReconcileAction::Delete;
    }

    match &server.status {
        Some(status) if status.phase == "Running" => {
            // Check if spec changed (replicas, image, etc.)
            if status.replicas != server.spec.replicas {
                ReconcileAction::Update
            } else {
                ReconcileAction::NoChange
            }
        }
        _ => ReconcileAction::Create,
    }
}
