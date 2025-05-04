use std::env;

use anyhow::{Context, Result};

/// Read a required environment variable.
///
/// Fail with useful context if the variable is not set in the environment.
pub fn required_env(key: &str) -> Result<String> {
    env::var(key).with_context(|| format!("Failed to read {key} from environment"))
}
