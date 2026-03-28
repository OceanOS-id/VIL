// =============================================================================
// VIL Plugin SDK Prelude — Single import for plugin authors
// =============================================================================
//
// Usage: use vil_plugin_sdk::prelude::*;

// Core trait
pub use crate::VilPlugin;

// Registration context
pub use crate::EndpointSpec;
pub use crate::PluginCapability;
pub use crate::PluginContext;
pub use crate::PluginDependency;
pub use crate::PluginHealth;
pub use crate::ResourceRegistry;

// Service building
pub use crate::ServiceProcess;
pub use crate::VxLane;

// Handler types
pub use crate::Method;
pub use crate::ServiceCtx;
pub use crate::ShmSlice;
pub use crate::VilError;
pub use crate::VilResponse;

// Routing
pub use crate::{delete, get, post, put};

// Serde (plugin config types need these)
pub use serde::{Deserialize, Serialize};

// Arc (plugins typically share state via Arc)
pub use std::sync::Arc;
