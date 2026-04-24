use wrapper_core::{
  ExpectedWrapperArtifacts, ProducedOuterProofArtifactBundle, ProducedOuterVerificationKeyJson,
  WrapperExecutionPackage,
};

use super::{
  OuterCircuitInputArtifacts, OuterProofBackend, OuterProofBackendError, OuterProofBackendMetadata,
  helpers::ensure_supported_target,
};
use wrapper_circuits::{InnerVerifierFlavor, OuterArtifactSerializationFlavor, OuterHostFlavor};

/// Placeholder backend for the planned direct Halo2/Midnight outer proof system.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlannedHalo2OuterBackend;

const PLANNED_BACKEND_METADATA: OuterProofBackendMetadata = OuterProofBackendMetadata {
  backend_id: "planned-halo2-outer-backend",
  inner_verifier: InnerVerifierFlavor::Groth16Bn254,
  outer_host: OuterHostFlavor::MidnightBn254,
  serialization: OuterArtifactSerializationFlavor::SerdeJsonHexEncodedProcessed,
  stack: "planning-only placeholder",
  protocol: OuterHostFlavor::MidnightBn254.protocol(),
  curve: OuterHostFlavor::MidnightBn254.curve(),
  pcs: OuterHostFlavor::MidnightBn254.pcs(),
  transcript: OuterHostFlavor::MidnightBn254.transcript(),
  supports_setup: false,
  supports_prove: false,
  supports_verify: false,
  setup_assumptions: &[
    "no proving stack is bound yet",
    "prepare() only materializes the planned artifact contract",
  ],
  serialization_conventions: &[
    "public inputs stay as decimal-string JSON arrays",
    "proof and verification-key payloads stay serde-friendly JSON objects",
  ],
  compatibility_notes: &[
    "proof payload remains absent by construction",
    "use only for planning/materialization, not for setup/prove/verify",
  ],
};

impl OuterProofBackend for PlannedHalo2OuterBackend {
  fn metadata(&self) -> &'static OuterProofBackendMetadata {
    &PLANNED_BACKEND_METADATA
  }

  fn backend_id(&self) -> &'static str {
    self.metadata().backend_id
  }

  fn prepare(
    &self,
    package: &WrapperExecutionPackage,
  ) -> Result<ExpectedWrapperArtifacts, OuterProofBackendError> {
    ensure_supported_target(package)?;
    package.validate_outer_statement_contract()?;
    Ok(package.expected_output())
  }

  fn setup(
    &self,
    package: &WrapperExecutionPackage,
    _artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<ProducedOuterVerificationKeyJson, OuterProofBackendError> {
    let _ = self.prepare(package)?;
    Err(OuterProofBackendError::UnsupportedOperation {
      backend: OuterProofBackend::backend_id(self),
      operation: "setup",
    })
  }

  fn prove(
    &self,
    package: &WrapperExecutionPackage,
    _artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<ProducedOuterProofArtifactBundle, OuterProofBackendError> {
    let _ = self.prepare(package)?;
    Err(OuterProofBackendError::UnsupportedOperation {
      backend: OuterProofBackend::backend_id(self),
      operation: "prove",
    })
  }

  fn verify(
    &self,
    package: &WrapperExecutionPackage,
    _produced: &ProducedOuterProofArtifactBundle,
    _artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<bool, OuterProofBackendError> {
    let _ = self.prepare(package)?;
    Err(OuterProofBackendError::UnsupportedOperation {
      backend: OuterProofBackend::backend_id(self),
      operation: "verify",
    })
  }
}
