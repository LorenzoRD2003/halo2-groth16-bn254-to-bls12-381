//! Project configuration models.

use serde::{Deserialize, Serialize};

use crate::{error::ConfigError, phase::ProjectPhase};

/// High-level wrapper flavor options for future experiments.
#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WrapperFlavor {
  /// A generic research scaffold without backend specialization.
  #[default]
  ResearchScaffold,
  /// Reserved for future Cardano-adjacent exploration.
  CardanoExploration,
  /// Reserved for future Semaphore-like migration experiments.
  SemaphoreMigrationStudy,
}

/// Filesystem paths used by the workspace.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectPaths {
  /// Directory for input artifacts.
  pub artifacts_dir: String,
  /// Directory for generated reports.
  pub reports_dir: String,
}

impl Default for ProjectPaths {
  fn default() -> Self {
    Self { artifacts_dir: "artifacts".to_owned(), reports_dir: "reports".to_owned() }
  }
}

/// Top-level wrapper tuning parameters that are safe to model before cryptographic code exists.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WrapperParameters {
  /// A human-readable label for the current experiment profile.
  pub profile_name: String,
  /// Whether developer diagnostics should print additional placeholder detail.
  pub diagnostics_verbose: bool,
}

impl Default for WrapperParameters {
  fn default() -> Self {
    Self { profile_name: "week1-foundation".to_owned(), diagnostics_verbose: true }
  }
}

/// Repository configuration parsed from TOML.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, Default)]
pub struct ProjectConfig {
  /// Current implementation phase.
  pub phase: ProjectPhase,
  /// Broad experiment flavor.
  pub flavor: WrapperFlavor,
  /// Filesystem layout.
  pub paths: ProjectPaths,
  /// Non-cryptographic tuning knobs.
  pub parameters: WrapperParameters,
}

impl ProjectConfig {
  /// Parses a project config from TOML text.
  ///
  /// # Errors
  ///
  /// Returns an error if the TOML is invalid or if validation fails for the
  /// current project rules.
  pub fn from_toml_str(input: &str) -> Result<Self, ConfigError> {
    let config = toml::from_str::<Self>(input)?;
    config.validate()?;
    Ok(config)
  }

  /// Validates basic configuration invariants for the current scaffold.
  ///
  /// # Errors
  ///
  /// Returns an error if a required field is empty or otherwise invalid for the
  /// current stage-1 configuration model.
  pub fn validate(&self) -> Result<(), ConfigError> {
    if self.parameters.profile_name.trim().is_empty() {
      return Err(ConfigError::InvalidField {
        field: "parameters.profile_name",
        reason: "profile name must not be empty".to_owned(),
      });
    }

    if self.paths.artifacts_dir.trim().is_empty() {
      return Err(ConfigError::InvalidField {
        field: "paths.artifacts_dir",
        reason: "artifacts directory must not be empty".to_owned(),
      });
    }

    if self.paths.reports_dir.trim().is_empty() {
      return Err(ConfigError::InvalidField {
        field: "paths.reports_dir",
        reason: "reports directory must not be empty".to_owned(),
      });
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests {
  use super::ProjectConfig;

  #[test]
  fn parses_valid_toml() {
    let input = r#"
 phase = "stage1"
flavor = "research-scaffold"

[paths]
artifacts_dir = "fixtures"
reports_dir = "reports"

[parameters]
profile_name = "local-dev"
diagnostics_verbose = true
"#;

    let config = ProjectConfig::from_toml_str(input).expect("config should parse");

    assert_eq!(config.parameters.profile_name, "local-dev");
  }

  #[test]
  fn rejects_empty_profile_name() {
    let input = r#"
 phase = "stage1"
flavor = "research-scaffold"

[paths]
artifacts_dir = "fixtures"
reports_dir = "reports"

[parameters]
profile_name = " "
diagnostics_verbose = true
"#;

    let error = ProjectConfig::from_toml_str(input).expect_err("config should fail");

    assert!(error.to_string().contains("profile name"));
  }
}
