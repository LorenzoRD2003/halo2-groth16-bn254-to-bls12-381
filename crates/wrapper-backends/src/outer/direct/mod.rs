mod adaptation;
mod artifacts;
mod planning;
mod proving;

use wrapper_circuits::{InnerVerifierFlavor, OuterArtifactSerializationFlavor, OuterHostFlavor};
use wrapper_core::{
  ExpectedWrapperArtifacts, ProducedOuterProofArtifactBundle, ProducedOuterVerificationKeyJson,
  WrapperExecutionPackage,
};

use super::{
  OuterCircuitInputArtifacts, OuterProofBackend, OuterProofBackendError, OuterProofBackendMetadata,
  helpers::{ensure_supported_target, expected_output_for_backend},
};

/// Concrete direct backend for the current BN254-hosted outer lane.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MidnightDirectOuterBackendBn254Host;

/// Direct backend for the BLS12-hosted outer lane.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MidnightDirectOuterBackendBls12Host;

/// Backward-compatible alias for the current direct BN254-hosted lane.
pub use MidnightDirectOuterBackendBn254Host as MidnightDirectOuterBackend;

const MIDNIGHT_DIRECT_BN254_BACKEND_METADATA: OuterProofBackendMetadata =
  OuterProofBackendMetadata {
    backend_id: "midnight-direct-halo2-outer-backend-bn254-host",
    inner_verifier: InnerVerifierFlavor::Groth16Bn254,
    outer_host: OuterHostFlavor::MidnightBn254,
    serialization: OuterArtifactSerializationFlavor::SerdeJsonHexEncodedProcessed,
    stack: "direct halo2/midnight outer lane over the canonical outer wrapper circuit (bn254 host)",
    protocol: OuterHostFlavor::MidnightBn254.protocol(),
    curve: OuterHostFlavor::MidnightBn254.curve(),
    pcs: OuterHostFlavor::MidnightBn254.pcs(),
    transcript: OuterHostFlavor::MidnightBn254.transcript(),
    supports_setup: true,
    supports_prove: true,
    supports_verify: true,
    setup_assumptions: &[
      "the outer circuit is authored in halo2/midnight and remains the canonical outer circuit surface",
      "the direct backend uses midnight_proofs keygen over a KZG commitment scheme",
      "trusted setup output must be serialized once and then reused across proofs for the same circuit size",
      "the wrapper statement mirrors the ordered inner verifier public inputs exactly",
    ],
    serialization_conventions: &[
      "proof.json stores a serialized proof payload plus protocol, curve, backend, transcript, and encoding labels",
      "wrapper-public.json is a JSON decimal-string array in wrapper statement order",
      "wrapper-verification-key.json stores the serialized plonk verification key plus serialized KZG verifier params",
    ],
    compatibility_notes: &[
      "this is the current reference implementation for the direct outer lane",
      "setup, proof generation, and verification are real on the current BN254-hosted lane",
      "artifact shapes remain aligned with the direct wrapper-core output model",
    ],
  };

const MIDNIGHT_DIRECT_BLS12_BACKEND_METADATA: OuterProofBackendMetadata =
  OuterProofBackendMetadata {
    backend_id: "midnight-direct-halo2-outer-backend-bls12-host",
    inner_verifier: InnerVerifierFlavor::Groth16Bn254,
    outer_host: OuterHostFlavor::MidnightBls12_381,
    serialization: OuterArtifactSerializationFlavor::SerdeJsonHexEncodedProcessed,
    stack: "direct halo2/midnight outer lane over the canonical outer wrapper circuit (bls12-381 host)",
    protocol: OuterHostFlavor::MidnightBls12_381.protocol(),
    curve: OuterHostFlavor::MidnightBls12_381.curve(),
    pcs: OuterHostFlavor::MidnightBls12_381.pcs(),
    transcript: OuterHostFlavor::MidnightBls12_381.transcript(),
    supports_setup: true,
    supports_prove: true,
    supports_verify: true,
    setup_assumptions: &[
      "the BLS12-hosted lane keeps the same canonical outer semantic circuit",
      "the direct backend uses midnight_proofs keygen over a KZG commitment scheme on the BLS12-381 host",
      "trusted setup output must be serialized once and then reused across proofs for the same circuit size",
      "the wrapper statement mirrors the ordered inner verifier public inputs exactly",
    ],
    serialization_conventions: &[
      "proof and verification-key artifacts keep the same serde-json-hex contract family",
      "public inputs stay as decimal-string JSON arrays in wrapper statement order",
    ],
    compatibility_notes: &[
      "this is an additive sibling lane to the current BN254-hosted direct backend",
      "setup, proof generation, and verification are real on the BLS12-381-hosted lane",
    ],
  };

impl OuterProofBackend for MidnightDirectOuterBackendBn254Host {
  fn metadata(&self) -> &'static OuterProofBackendMetadata {
    &MIDNIGHT_DIRECT_BN254_BACKEND_METADATA
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

    let mut planned = expected_output_for_backend(package, self.metadata());
    planned.notes.push(format!("selected outer backend stack: {}", self.metadata().stack));
    planned.notes.push(
      "outer statement contract is frozen to mirror ordered inner verifier public inputs"
        .to_owned(),
    );
    planned.notes.push(
      "selected real backend mode is direct halo2/midnight proving over the canonical outer circuit"
        .to_owned(),
    );
    planned.notes.push(
      "setup uses midnight_proofs keygen over the canonical outer circuit with KZG verifier parameters serialized alongside the VK"
        .to_owned(),
    );
    planned
      .notes
      .extend(self.metadata().serialization_conventions.iter().map(|note| (*note).to_owned()));
    planned
      .bundle_template
      .notes
      .push("selected backend is the direct halo2/midnight BN254-hosted lane".to_owned());

    Ok(planned)
  }

  fn setup(
    &self,
    package: &WrapperExecutionPackage,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<ProducedOuterVerificationKeyJson, OuterProofBackendError> {
    let circuit = self.build_outer_circuit(package, artifacts)?;
    self.produce_setup_verification_key(package, &circuit)
  }

  fn prove(
    &self,
    package: &WrapperExecutionPackage,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<ProducedOuterProofArtifactBundle, OuterProofBackendError> {
    let circuit = self.build_outer_circuit(package, artifacts)?;
    self.produce_proof_bundle(package, &circuit)
  }

  fn verify(
    &self,
    package: &WrapperExecutionPackage,
    produced: &ProducedOuterProofArtifactBundle,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<bool, OuterProofBackendError> {
    let circuit = self.build_outer_circuit(package, artifacts)?;
    self.verify_produced_bundle(package, produced, &circuit)
  }
}

impl OuterProofBackend for MidnightDirectOuterBackendBls12Host {
  fn metadata(&self) -> &'static OuterProofBackendMetadata {
    &MIDNIGHT_DIRECT_BLS12_BACKEND_METADATA
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

    let mut planned = expected_output_for_backend(package, self.metadata());
    planned.notes.push(format!("selected outer backend stack: {}", self.metadata().stack));
    planned
      .notes
      .push("selected backend is the direct halo2/midnight BLS12-hosted proving lane".to_owned());
    planned.notes.push(
      "setup uses midnight_proofs keygen over the canonical outer circuit with BLS12-381 KZG verifier parameters serialized alongside the VK"
        .to_owned(),
    );
    planned
      .notes
      .extend(self.metadata().serialization_conventions.iter().map(|note| (*note).to_owned()));
    planned
      .bundle_template
      .notes
      .push("selected backend is the direct halo2/midnight BLS12-hosted lane".to_owned());
    Ok(planned)
  }

  fn setup(
    &self,
    package: &WrapperExecutionPackage,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<ProducedOuterVerificationKeyJson, OuterProofBackendError> {
    let circuit = self.build_outer_circuit(package, artifacts)?;
    self.produce_setup_verification_key(package, &circuit)
  }

  fn prove(
    &self,
    package: &WrapperExecutionPackage,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<ProducedOuterProofArtifactBundle, OuterProofBackendError> {
    let circuit = self.build_outer_circuit(package, artifacts)?;
    self.produce_proof_bundle(package, &circuit)
  }

  fn verify(
    &self,
    package: &WrapperExecutionPackage,
    produced: &ProducedOuterProofArtifactBundle,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<bool, OuterProofBackendError> {
    let circuit = self.build_outer_circuit(package, artifacts)?;
    self.verify_produced_bundle(package, produced, &circuit)
  }
}
