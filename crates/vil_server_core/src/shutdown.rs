// =============================================================================
// VIL Server Shutdown — Graceful shutdown handling
// =============================================================================

use vil_log::{system_log, types::SystemPayload};

/// Wait for a shutdown signal (SIGTERM or SIGINT/Ctrl+C).
/// Used with Axum's graceful_shutdown.
pub async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            system_log!(Info, SystemPayload { event_type: 5, signal_num: 2, ..Default::default() });
        },
        _ = terminate => {
            system_log!(Info, SystemPayload { event_type: 5, signal_num: 15, ..Default::default() });
        },
    }
}
