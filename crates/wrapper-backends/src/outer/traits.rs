use wrapper_circuits::OuterWrapperCircuit;
use wrapper_core::{
  ExpectedWrapperArtifacts, ProducedOuterProofArtifactBundle, ProducedOuterVerificationKeyJson,
  WrapperExecutionPackage,
};

use super::{
  CanonicalOuterCircuitProofArtifacts, CanonicalOuterCircuitSetupArtifacts,
  OuterCircuitInputArtifacts, OuterProofBackendError, OuterProofBackendMetadata,
};

/// Internal surface for proving the canonical outer Halo2/Midnight circuit directly.
///
/// This sits below `OuterProofBackend` and above any concrete prover /
/// serializer integration. It exists so the real direct outer-circuit path can
/// be implemented without forcing the package-oriented backend contract to own
/// low-level proving details.
pub trait CanonicalOuterCircuitProofBackend {
  /// Stable backend identifier.
  fn backend_id(&self) -> &'static str;

  /// Plans setup over a canonical outer circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if the circuit is not ready for synthesis or if the
  /// backend cannot support direct setup.
  fn plan_canonical_setup(
    &self,
    circuit: &OuterWrapperCircuit,
    verification_key_artifact: &str,
  ) -> Result<CanonicalOuterCircuitSetupArtifacts, OuterProofBackendError>;

  /// Plans proving over a canonical outer circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if the circuit is not ready for synthesis or if the
  /// backend cannot support direct proving.
  fn plan_canonical_proof(
    &self,
    circuit: &OuterWrapperCircuit,
    proof_artifact: &str,
    public_inputs_artifact: &str,
    public_inputs: &[String],
  ) -> Result<CanonicalOuterCircuitProofArtifacts, OuterProofBackendError>;
}

/// Backend contract for producing outer Groth16 artifacts.
pub trait OuterProofBackend {
  /// Returns static metadata for the selected backend stack.
  fn metadata(&self) -> &'static OuterProofBackendMetadata;

  /// Returns stable capability metadata for the selected backend stack.
  #[must_use]
  fn capabilities(&self) -> super::OuterBackendCapabilities {
    self.metadata().capabilities()
  }

  /// Returns a short backend identifier.
  fn backend_id(&self) -> &'static str;

  /// Plans/materializes the outer artifact contract from a wrapper execution package.
  ///
  /// # Errors
  ///
  /// Returns an error if the package is incompatible with the backend target or
  /// violates the frozen outer-statement contract.
  fn prepare(
    &self,
    package: &WrapperExecutionPackage,
  ) -> Result<ExpectedWrapperArtifacts, OuterProofBackendError>;

  /// Runs setup for the outer backend and emits a real verification key once supported.
  ///
  /// # Errors
  ///
  /// Returns an error if setup is not implemented or the package is invalid.
  fn setup(
    &self,
    package: &WrapperExecutionPackage,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<ProducedOuterVerificationKeyJson, OuterProofBackendError>;

  /// Produces a real outer Groth16 artifact bundle once supported.
  ///
  /// # Errors
  ///
  /// Returns an error if proving is not implemented or the package is invalid.
  fn prove(
    &self,
    package: &WrapperExecutionPackage,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<ProducedOuterProofArtifactBundle, OuterProofBackendError>;

  /// Verifies a produced outer Groth16 artifact bundle against the package statement.
  ///
  /// # Errors
  ///
  /// Returns an error if verification is not implemented or the inputs are invalid.
  fn verify(
    &self,
    package: &WrapperExecutionPackage,
    produced: &ProducedOuterProofArtifactBundle,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<bool, OuterProofBackendError>;
}
