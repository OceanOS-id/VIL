// =============================================================================
// VIL Plugin SDK — Stable Community Plugin Interface
// =============================================================================
//
// This crate is the ONLY dependency community plugin authors need.
// It re-exports the stable plugin API surface from vil_server_core and adds
// ergonomic utilities for building, testing, and declaring plugins.
//
// Stability guarantee: public API in this crate follows semver.
// Internal vil_server_core changes will NOT break plugin authors.
//
// Quick start:
//   use vil_plugin_sdk::prelude::*;
//
//   pub struct MyPlugin;
//   impl VilPlugin for MyPlugin {
//       fn id(&self) -> &str { "my-plugin" }
//       fn version(&self) -> &str { "1.0.0" }
//       fn register(&self, ctx: &mut PluginContext) { ... }
//   }

pub mod builder;
pub mod manifest;
pub mod prelude;
pub mod testing;

// ── Stable re-exports from vil_server_core ──────────────────────────────

// Core trait + plugin system types
pub use vil_server_core::{
    PluginCapability, PluginContext, PluginDependency, PluginEndpointSpec as EndpointSpec,
    PluginError, PluginHealth, PluginInfo, PluginRegistry, ResourceRegistry, VilPlugin,
};

// Service building
pub use vil_server_core::ServiceProcess;
pub use vil_server_core::VxLane;

// Handler types
pub use vil_server_core::error::VilError;
pub use vil_server_core::response::VilResponse;
pub use vil_server_core::ServiceCtx;
pub use vil_server_core::ShmSlice;

// Axum routing (plugins need these for endpoint registration)
pub use vil_server_core::axum::http::Method;
pub use vil_server_core::axum::routing::{delete, get, post, put};

// Re-export serde for plugin config types
pub use serde;
pub use serde_json;

// ── NativeCode Handler SDK (.so plugin) ─────────────────────────────────
//
// For building .so plugins that VIL/VFlow can load at runtime via dlopen.
// Compatible with both VIL (vil_vwfd) and VFlow (vflow_server).
//
// Quick start:
//   use vil_plugin_sdk::vil_handler;
//   use serde_json::{Value, json};
//
//   vil_handler!("my_handler", |input| {
//       let name = input["name"].as_str().unwrap_or("world");
//       Ok(json!({"greeting": format!("Hello {}", name)}))
//   });
//
// Build: cargo build --release (crate-type = ["cdylib"])
// Deploy: cp target/release/libmy_handler.so /var/lib/vil/plugins/
//
// The macro generates 3 C ABI functions (same ABI as vflow_plugin_sdk):
//   - vflow_plugin_name() -> *const c_char
//   - vflow_plugin_execute(in_ptr, in_len, out_ptr, out_len) -> i32
//   - vflow_plugin_free(ptr, len)

/// Generate C ABI functions for a VIL/VFlow NativeCode handler plugin.
///
/// # Example
/// ```rust,ignore
/// vil_handler!("credit_score", |input| {
///     let income = input["income"].as_f64().unwrap_or(0.0);
///     let score = (income / 1000.0).min(850.0) as u32;
///     Ok(serde_json::json!({"score": score, "grade": if score > 700 { "A" } else { "B" }}))
/// });
/// ```
#[macro_export]
macro_rules! vil_handler {
    ($name:expr, $handler:expr) => {
        #[no_mangle]
        pub extern "C" fn vflow_plugin_name() -> *const std::ffi::c_char {
            concat!($name, "\0").as_ptr() as *const std::ffi::c_char
        }

        #[no_mangle]
        pub extern "C" fn vflow_plugin_execute(
            input_ptr: *const u8,
            input_len: usize,
            output_ptr: *mut *mut u8,
            output_len: *mut usize,
        ) -> i32 {
            let input_bytes = if input_ptr.is_null() || input_len == 0 {
                b"null".as_slice()
            } else {
                unsafe { std::slice::from_raw_parts(input_ptr, input_len) }
            };

            let input: $crate::serde_json::Value = match $crate::serde_json::from_slice(input_bytes) {
                Ok(v) => v,
                Err(e) => {
                    let msg = format!("input parse error: {}", e);
                    let boxed = msg.into_bytes().into_boxed_slice();
                    let len = boxed.len();
                    let ptr = Box::into_raw(boxed) as *mut u8;
                    unsafe { *output_ptr = ptr; *output_len = len; }
                    return 1;
                }
            };

            let handler: fn(&$crate::serde_json::Value) -> Result<$crate::serde_json::Value, String> = $handler;
            // Catch panics (e.g. tokio runtime not available) to avoid aborting the host process
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| handler(&input)));
            let result = match result {
                Ok(r) => r,
                Err(e) => {
                    let msg = if let Some(s) = e.downcast_ref::<&str>() {
                        format!("handler panic: {}", s)
                    } else if let Some(s) = e.downcast_ref::<String>() {
                        format!("handler panic: {}", s)
                    } else {
                        "handler panic: unknown".to_string()
                    };
                    Err(msg)
                }
            };
            match result {
                Ok(result) => {
                    let bytes = $crate::serde_json::to_vec(&result).unwrap_or_else(|_| b"null".to_vec());
                    let boxed = bytes.into_boxed_slice();
                    let len = boxed.len();
                    let ptr = Box::into_raw(boxed) as *mut u8;
                    unsafe { *output_ptr = ptr; *output_len = len; }
                    0
                }
                Err(e) => {
                    let boxed = e.into_bytes().into_boxed_slice();
                    let len = boxed.len();
                    let ptr = Box::into_raw(boxed) as *mut u8;
                    unsafe { *output_ptr = ptr; *output_len = len; }
                    1
                }
            }
        }

        #[no_mangle]
        pub extern "C" fn vflow_plugin_free(ptr: *mut u8, len: usize) {
            if !ptr.is_null() && len > 0 {
                unsafe { let _ = Box::from_raw(std::slice::from_raw_parts_mut(ptr, len)); }
            }
        }
    };
}
