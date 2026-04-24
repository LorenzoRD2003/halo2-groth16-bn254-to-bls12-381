//! Outer wrapper circuit built on top of the landed narrow BN254 verifier.

mod builder;
mod host;
mod hosted;
mod input;
mod r1cs;
mod semantics;
mod statement;

use wrapper_core::{LayoutDescriptor, ProjectConfig, WrapperError};

pub use builder::build_outer_wrapper_circuit;
pub use host::{
  InnerVerifierFlavor, MidnightBls12_381HostConfigShell, MidnightBls12_381HostLane,
  MidnightBn254HostConfig, MidnightBn254HostLane, OuterArtifactSerializationFlavor,
  OuterHostConfig, OuterHostField, OuterHostFlavor, OuterHostLane, OuterWrapperFlavorProfile,
};
pub use hosted::{HostedOuterWrapperCircuit, OuterWrapperHostConfig};
pub use input::OuterWrapperCircuitInput;
pub use r1cs::{
  OuterCanonicalR1csLoweringError, OuterCanonicalR1csLoweringReport, OuterCanonicalR1csSliceKind,
  OuterCanonicalR1csSliceReport, OuterCanonicalR1csSliceStatus, OuterGroth16IcAccumulatorSlice,
  OuterGroth16PairingProductCheckSlice, OuterStatementExposureR1cs,
  OuterVerifierResultAssertionSlice, build_outer_groth16_ic_accumulator_slice,
  build_outer_groth16_pairing_product_check_slice, build_outer_statement_exposure_r1cs,
  build_outer_verifier_result_assertion_slice, build_outer_wrapper_canonical_r1cs,
  inspect_outer_wrapper_canonical_r1cs,
};
pub use semantics::Bn254InnerVerifierConfig;
pub use statement::{OuterStatementInput, OuterStatementSemantics};

/// Build status for the outer circuit shell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CircuitBuildStatus {
  /// The outer wrapper circuit is synthesized from the landed narrow verifier.
  VerifierIntegrated,
}

/// Canonical outer wrapper semantic circuit backed by the narrow Groth16 BN254 verifier.
///
/// This type captures the semantic statement:
/// verify Groth16 BN254 and expose the outer statement.
///
/// Host-lane-specific proving details live in `HostedOuterWrapperCircuit`.
#[derive(Clone, Debug)]
pub struct OuterWrapperCircuit {
  /// Config used to describe the intended circuit.
  pub config: ProjectConfig,
  /// Explicit semantic/host flavor boundary for this circuit instance.
  pub flavors: OuterWrapperFlavorProfile,
  /// Canonical outer-circuit input.
  pub input: OuterWrapperCircuitInput,
}

impl OuterWrapperCircuit {
  /// Creates a new outer wrapper circuit from explicit input plus config.
  #[must_use]
  pub fn new(input: OuterWrapperCircuitInput, config: ProjectConfig) -> Self {
    Self::new_with_flavors(input, config, OuterWrapperFlavorProfile::current())
  }

  /// Creates a new outer wrapper circuit from explicit input, config, and
  /// flavor-boundary metadata.
  #[must_use]
  pub fn new_with_flavors(
    input: OuterWrapperCircuitInput,
    config: ProjectConfig,
    flavors: OuterWrapperFlavorProfile,
  ) -> Self {
    Self { config, flavors, input }
  }

  /// Creates a new outer wrapper circuit using the default project config.
  #[must_use]
  pub fn from_input(input: OuterWrapperCircuitInput) -> Self {
    Self::new(input, ProjectConfig::default())
  }

  /// Creates a new outer wrapper circuit using the default project config and
  /// one explicit outer host lane.
  #[must_use]
  pub fn from_input_for_host(input: OuterWrapperCircuitInput, outer_host: OuterHostFlavor) -> Self {
    Self::new_with_flavors(
      input,
      ProjectConfig::default(),
      OuterWrapperFlavorProfile::current().with_outer_host(outer_host),
    )
  }

  /// Returns the current build status.
  #[must_use]
  pub fn build_status(&self) -> CircuitBuildStatus {
    CircuitBuildStatus::VerifierIntegrated
  }

  /// Returns the scaffold layout for reporting purposes.
  #[must_use]
  pub fn layout_descriptor(&self) -> LayoutDescriptor {
    LayoutDescriptor::scaffold()
  }

  /// Validates that the outer circuit input is ready for real synthesis.
  ///
  /// # Errors
  ///
  /// Returns an error if the outer statement no longer mirrors the inner
  /// verifier public inputs or the inner verification key arity is inconsistent.
  pub fn assert_ready_for_synthesis(&self) -> Result<(), WrapperError> {
    if !self.flavors.outer_host.supports_current_canonical_circuit() {
      return Err(WrapperError::InvalidInput {
        context: "outer host flavor",
        reason: format!(
          "host lane '{}' is planned but not yet wired to the canonical outer circuit in the current repository phase",
          self.flavors.outer_host.id()
        ),
      });
    }

    self.input.validate()
  }

  /// Returns a host-lane-specific proving wrapper around this semantic circuit.
  #[must_use]
  pub fn hosted(&self) -> HostedOuterWrapperCircuit {
    HostedOuterWrapperCircuit::new(self.clone())
  }

  /// Converts this semantic circuit into a host-lane-specific proving wrapper.
  #[must_use]
  pub fn into_hosted(self) -> HostedOuterWrapperCircuit {
    HostedOuterWrapperCircuit::new(self)
  }

  /// Returns a witness-free semantic variant.
  #[must_use]
  pub fn without_witnesses(&self) -> Self {
    Self {
      config: self.config.clone(),
      flavors: self.flavors,
      input: self.input.without_witnesses(),
    }
  }
}

#[cfg(test)]
mod tests;
