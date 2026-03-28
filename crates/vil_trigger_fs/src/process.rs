// =============================================================================
// vil_trigger_fs::process — create_fs_trigger()
// =============================================================================
//
// Convenience constructor: wires up an mpsc channel and returns a
// `(FsTrigger, Receiver<TriggerEvent>)` pair ready for use inside a VIL
// ServiceProcess.
// =============================================================================

use tokio::sync::mpsc;

use vil_log::dict::register_str;
use vil_trigger_core::TriggerEvent;

use crate::config::FsConfig;
use crate::source::FsTrigger;

/// Create a `FsTrigger` together with its event receiver channel.
///
/// Call `TriggerSource::start()` on the returned trigger to begin watching.
/// The `mpsc::Receiver<TriggerEvent>` should be handed to the downstream
/// pipeline stage that consumes events on the Trigger Lane.
pub fn create_fs_trigger(config: FsConfig) -> (FsTrigger, mpsc::Receiver<TriggerEvent>) {
    register_str(config.watch_path);
    register_str("fs");

    let (tx, rx) = mpsc::channel::<TriggerEvent>(config.channel_capacity);
    let trigger = FsTrigger::new(config, tx);
    (trigger, rx)
}
