//! Domain-level statement helpers for wrapper experiments.

use serde::{Deserialize, Serialize};

/// One named public-input value.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NamedPublicInput {
  /// Semantic field name supplied by the caller.
  pub name: String,
  /// Decimal field element encoded as text.
  pub value: String,
}

impl NamedPublicInput {
  /// Builds one named public-input value.
  #[must_use]
  pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
    Self { name: name.into(), value: value.into() }
  }
}

/// Structured view over an ordered Groth16 public-input vector.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct NamedPublicInputs {
  /// Ordered named fields.
  pub entries: Vec<NamedPublicInput>,
}

impl NamedPublicInputs {
  /// Builds a named public-input list from `(name, value)` pairs.
  #[must_use]
  pub fn new(entries: Vec<NamedPublicInput>) -> Self {
    Self { entries }
  }

  /// Returns the ordered field names.
  #[must_use]
  pub fn field_order(&self) -> Vec<&str> {
    self.entries.iter().map(|entry| entry.name.as_str()).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::{NamedPublicInput, NamedPublicInputs};

  #[test]
  fn named_public_inputs_preserve_order() {
    let inputs = NamedPublicInputs::new(vec![
      NamedPublicInput::new("first", "1"),
      NamedPublicInput::new("second", "2"),
    ]);

    assert_eq!(inputs.field_order(), vec!["first", "second"]);
  }
}
