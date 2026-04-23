//! Wrapper job planning types.

use serde::{Deserialize, Serialize};

use crate::{NamedPublicInputs, ProofSystemDescriptor};

/// Planned wrapper job built from a parsed inner-proof artifact set.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct WrapperJob {
  /// Logical identifier for the job.
  pub identifier: String,
  /// Inner proof system being wrapped.
  pub source: ProofSystemDescriptor,
  /// Outer proof system target.
  pub target: ProofSystemDescriptor,
  /// Number of verifier public inputs carried by the inner proof.
  pub public_input_count: usize,
  /// Optional semantic names for the ordered public inputs.
  pub named_public_inputs: Option<NamedPublicInputs>,
  /// Notes about planning assumptions and current-stage limits.
  pub notes: Vec<String>,
}

impl WrapperJob {
  /// Builds a wrapper job from normalized planning pieces.
  #[must_use]
  pub fn new(
    identifier: impl Into<String>,
    source: ProofSystemDescriptor,
    target: ProofSystemDescriptor,
    public_input_count: usize,
    named_public_inputs: Option<NamedPublicInputs>,
    notes: Vec<String>,
  ) -> Self {
    Self {
      identifier: identifier.into(),
      source,
      target,
      public_input_count,
      named_public_inputs,
      notes,
    }
  }
}

#[cfg(test)]
mod tests {
  use crate::{NamedPublicInput, NamedPublicInputs, ProofSystemDescriptor, ProofSystemKind, WrapperJob};

  #[test]
  fn wrapper_job_keeps_named_public_inputs() {
    let job = WrapperJob::new(
      "job-1",
      ProofSystemDescriptor {
        kind: ProofSystemKind::Groth16Bn254,
        source: "snarkjs".to_owned(),
      },
      ProofSystemDescriptor {
        kind: ProofSystemKind::Groth16Bls12_381,
        source: "planned-outer".to_owned(),
      },
      2,
      Some(NamedPublicInputs::new(vec![
        NamedPublicInput::new("x", "1"),
        NamedPublicInput::new("y", "2"),
      ])),
      vec!["note".to_owned()],
    );

    assert_eq!(job.public_input_count, 2);
    assert_eq!(
      job.named_public_inputs.expect("job should keep names").field_order(),
      vec!["x", "y"]
    );
  }
}
