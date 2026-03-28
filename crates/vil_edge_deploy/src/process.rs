// =============================================================================
// vil_edge_deploy — process.rs
// =============================================================================
//
// Provides a convenience `create` function that builds an `EdgeConfig` from
// a profile + target combination and registers relevant strings in vil_log's
// dict for structured log hash lookups.
//
// Usage:
// ```ignore
// use vil_edge_deploy::process::create;
// use vil_edge_deploy::{EdgeProfile, EdgeTarget};
//
// let config = create(EdgeTarget::Aarch64Linux, EdgeProfile::Standard)?;
// let args = config.target.cargo_build_args();
// ```
// =============================================================================

use vil_log::dict::register_str;

use crate::{config::EdgeConfig, error::EdgeFault, profile::EdgeProfile, targets::EdgeTarget};

/// Build an `EdgeConfig` from a target and profile, registering all relevant
/// strings in `vil_log::dict` for structured log hash lookups.
pub fn create(target: EdgeTarget, profile: EdgeProfile) -> Result<EdgeConfig, EdgeFault> {
    // Register component identity strings in vil_log dict.
    register_str("edge_deploy.create");
    register_str(target.rustc_target_triple());
    register_str(&profile.to_string());

    let config = EdgeConfig::from_profile(target, profile);

    if !config.validate() {
        return Err(EdgeFault::InvalidProfile);
    }

    // Register config shape hints.
    register_str("edge_deploy.config.ready");
    register_str(&config.scheduler_mode.to_string());

    Ok(config)
}

/// Load an `EdgeConfig` from a YAML file and register its strings in vil_log dict.
pub fn create_from_file(path: &std::path::Path) -> Result<EdgeConfig, EdgeFault> {
    register_str("edge_deploy.create_from_file");

    let config = EdgeConfig::from_file(path)?;

    if !config.validate() {
        return Err(EdgeFault::InvalidProfile);
    }

    register_str(config.target.rustc_target_triple());
    register_str(&config.profile.to_string());
    register_str("edge_deploy.config.loaded");

    Ok(config)
}
