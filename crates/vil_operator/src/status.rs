#![allow(dead_code)]
// Status helpers.

use crate::crd::VilServerStatus;

pub fn status_running(replicas: i32) -> VilServerStatus {
    VilServerStatus {
        phase: "Running".into(),
        replicas,
        ready_replicas: replicas,
        message: "All replicas ready".into(),
    }
}

pub fn status_pending(message: &str) -> VilServerStatus {
    VilServerStatus {
        phase: "Pending".into(),
        replicas: 0,
        ready_replicas: 0,
        message: message.into(),
    }
}

pub fn status_error(message: &str) -> VilServerStatus {
    VilServerStatus {
        phase: "Error".into(),
        replicas: 0,
        ready_replicas: 0,
        message: message.into(),
    }
}
