//! Error types shared across the workspace.

use thiserror::Error;

/// Errors surfaced by configuration parsing and validation.
#[derive(Debug, Error)]
pub enum ConfigError {
  /// TOML parsing failed.
  #[error("failed to parse config TOML: {0}")]
  Toml(#[from] toml::de::Error),
  /// A field contained an invalid value.
  #[error("invalid config field `{field}`: {reason}")]
  InvalidField {
    /// Field path.
    field: &'static str,
    /// Reason the value is rejected.
    reason: String,
  },
}

/// Domain-level errors for placeholder interfaces.
#[derive(Debug, Error)]
pub enum WrapperError {
  /// The requested feature is intentionally unavailable in the current stage.
  #[error("feature not implemented in current phase: {0}")]
  NotImplemented(&'static str),
  /// Configuration was invalid.
  #[error(transparent)]
  Config(#[from] ConfigError),
}
