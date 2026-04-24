//! Outer proof backend contracts, backend metadata, and input adapters.

use blake2b_simd::State as Blake2bState;
use ff::{Field, PrimeField};
use midnight_curves::bn256::Bn256;
use midnight_proofs::{
  plonk::{create_proof, k_from_circuit, keygen_pk, keygen_vk_with_k, prepare},
  poly::{
    commitment::{Guard, PolynomialCommitmentScheme},
    kzg::KZGCommitmentScheme,
  },
  transcript::{CircuitTranscript, Transcript},
  utils::SerdeFormat,
};
use rand_core::OsRng;
use thiserror::Error;
use wrapper_circuits::{
  CircuitBuildStatus, Groth16Bn254Proof, Groth16Bn254VerifyingKey, NativeField,
  OuterStatementInput, OuterStatementSemantics, OuterWrapperCircuit, OuterWrapperCircuitInput,
  R1csCircuit, build_outer_wrapper_canonical_r1cs, build_outer_wrapper_circuit,
};
use wrapper_core::{
  ExpectedWrapperArtifacts, OuterStatementContractError, ProducedOuterProofArtifactBundle,
  ProducedOuterProofJson, ProducedOuterVerificationKeyJson, ProofSystemKind,
  WrapperExecutionPackage,
};

use crate::snarkjs::{
  SnarkjsGroth16ParseError, parse_groth16_bn254_proof, parse_groth16_bn254_verifying_key,
};

/// Static metadata describing one selected outer Groth16 backend stack.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OuterProofBackendMetadata {
  /// Stable backend identifier.
  pub backend_id: &'static str,
  /// Human-readable stack family.
  pub stack: &'static str,
  /// Protocol label expected from the backend.
  pub protocol: &'static str,
  /// Curve label expected from the backend.
  pub curve: &'static str,
  /// Setup assumptions for this backend choice.
  pub setup_assumptions: &'static [&'static str],
  /// Serialization conventions expected from this backend choice.
  pub serialization_conventions: &'static [&'static str],
  /// Compatibility notes for future implementors.
  pub compatibility_notes: &'static [&'static str],
}

/// Raw inner-artifact payloads required to adapt a wrapper package to one outer backend.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct OuterCircuitInputArtifacts<'a> {
  /// Raw inner `proof.json` payload.
  pub proof_json: Option<&'a [u8]>,
  /// Raw inner `verification_key.json` payload.
  pub verification_key_json: Option<&'a [u8]>,
}

impl<'a> OuterCircuitInputArtifacts<'a> {
  /// Builds a raw inner-artifact bundle for backend adaptation.
  #[must_use]
  pub fn new(proof_json: Option<&'a [u8]>, verification_key_json: Option<&'a [u8]>) -> Self {
    Self { proof_json, verification_key_json }
  }
}

/// Outer public statement normalized to the exact field layout expected by the arkworks lane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectOuterStatementInput {
  /// Ordered semantic public-input names.
  pub field_names: Vec<String>,
  /// Ordered field values for the outer public statement.
  pub public_inputs: Vec<NativeField>,
}

/// Exact witness/config input shape expected by the chosen arkworks outer backend lane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectOuterCircuitInput {
  /// Logical identifier of the inner artifact set.
  pub source_artifact_id: String,
  /// Parsed inner Groth16 BN254 proof.
  pub inner_proof: Groth16Bn254Proof,
  /// Parsed inner Groth16 BN254 verification key.
  pub inner_verification_key: Groth16Bn254VerifyingKey,
  /// Ordered inner verifier public inputs, normalized to field elements.
  pub inner_verifier_public_inputs: Vec<NativeField>,
  /// Outer public statement normalized for the selected backend lane.
  pub outer_statement: DirectOuterStatementInput,
}

impl DirectOuterCircuitInput {
  /// Converts the backend-normalized input into the canonical circuit-owned input.
  #[must_use]
  pub fn to_circuit_input(&self) -> OuterWrapperCircuitInput {
    OuterWrapperCircuitInput::new(
      self.inner_proof.clone(),
      self.inner_verification_key.clone(),
      self.inner_verifier_public_inputs.clone(),
      OuterStatementInput::new(
        OuterStatementSemantics::MirrorInnerPublicInputs,
        self.outer_statement.field_names.clone(),
        self.outer_statement.public_inputs.clone(),
      ),
    )
  }
}

/// Setup-time plan for the selected direct outer backend lane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectOuterSetupPlan {
  /// Logical verification-key artifact identifier.
  pub verification_key_artifact: String,
  /// Expected outer public-input count.
  pub expected_public_input_count: usize,
  /// Expected polynomial-commitment scheme.
  pub expected_pcs: String,
  /// Setup notes for the selected backend lane.
  pub notes: Vec<String>,
}

/// Proving-time plan for the selected direct outer backend lane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DirectOuterProofPlan {
  /// Logical proof artifact identifier.
  pub proof_artifact: String,
  /// Logical public-input artifact identifier.
  pub public_inputs_artifact: String,
  /// Ordered public inputs that the produced proof must expose.
  pub public_inputs: Vec<String>,
  /// Expected transcript family used by the direct backend.
  pub expected_transcript: String,
  /// Proving notes for the selected backend lane.
  pub notes: Vec<String>,
}

/// Setup material emitted by a backend that proves the canonical outer Halo2/Midnight circuit directly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalOuterCircuitSetupArtifacts {
  /// Canonical outer circuit build status used during setup.
  pub build_status: &'static str,
  /// Expected verification-key artifact identifier.
  pub verification_key_artifact: String,
  /// Ordered outer public-input count.
  pub expected_public_input_count: usize,
  /// Setup notes for the direct outer-circuit lane.
  pub notes: Vec<String>,
}

/// Proving material emitted by a backend that proves the canonical outer Halo2/Midnight circuit directly.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CanonicalOuterCircuitProofArtifacts {
  /// Canonical outer circuit build status used during proving.
  pub build_status: &'static str,
  /// Expected proof artifact identifier.
  pub proof_artifact: String,
  /// Expected public-input artifact identifier.
  pub public_inputs_artifact: String,
  /// Ordered outer public inputs as decimal strings.
  pub public_inputs: Vec<String>,
  /// Proving notes for the direct outer-circuit lane.
  pub notes: Vec<String>,
}

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

/// Errors raised while producing outer Groth16 artifacts.
#[derive(Debug, Error)]
pub enum OuterProofBackendError {
  /// The package targets a different outer proof system than the backend supports.
  #[error("outer backend expected target proof system '{expected}', got '{actual}'")]
  UnsupportedTarget {
    /// Expected proof-system identifier.
    expected: &'static str,
    /// Actual proof-system identifier from the package.
    actual: &'static str,
  },
  /// The package violates the frozen outer-statement contract.
  #[error(transparent)]
  InvalidPackage(#[from] OuterStatementContractError),
  /// The backend expected an inner proof payload but none was supplied.
  #[error("missing inner proof payload for source artifact '{source_artifact_id}'")]
  MissingInnerProofPayload {
    /// Source artifact identifier from the package witness metadata.
    source_artifact_id: String,
  },
  /// The backend expected an inner verification-key payload but none was supplied.
  #[error("missing inner verification-key payload for source artifact '{source_artifact_id}'")]
  MissingInnerVerificationKeyPayload {
    /// Source artifact identifier from the package witness metadata.
    source_artifact_id: String,
  },
  /// The supplied inner proof payload could not be parsed as the expected Groth16 BN254 shape.
  #[error(
    "failed to parse inner proof payload for source artifact '{source_artifact_id}': {source}"
  )]
  MalformedInnerProof {
    /// Source artifact identifier from the package witness metadata.
    source_artifact_id: String,
    #[source]
    /// Underlying snarkjs parsing error.
    source: SnarkjsGroth16ParseError,
  },
  /// The supplied inner verification key could not be parsed as the expected Groth16 BN254 shape.
  #[error(
    "failed to parse inner verification-key payload for source artifact '{source_artifact_id}': {source}"
  )]
  MalformedInnerVerificationKey {
    /// Source artifact identifier from the package witness metadata.
    source_artifact_id: String,
    #[source]
    /// Underlying snarkjs parsing error.
    source: SnarkjsGroth16ParseError,
  },
  /// One public-input value inside the package statement or witness could not be parsed as a field element.
  #[error("invalid {context} public-input value for field '{field_name}': '{value}'")]
  InvalidPublicInputValue {
    /// Whether the invalid value came from the outer statement or inner witness view.
    context: &'static str,
    /// Semantic field name from the package.
    field_name: String,
    /// Original decimal-string value that failed to parse.
    value: String,
  },
  /// The parsed inner verification key no longer matches the arity expected by the package.
  #[error(
    "inner verification-key IC arity mismatch after parsing: expected {expected}, got {actual}"
  )]
  VerificationKeyIcCountMismatch {
    /// Expected IC count from the package.
    expected: usize,
    /// Actual IC count parsed from the supplied verification key payload.
    actual: usize,
  },
  /// The emitted setup verification key does not match the expected protocol label.
  #[error("outer setup verification-key protocol mismatch: expected '{expected}', got '{actual}'")]
  VerificationKeyProtocolMismatch {
    /// Expected protocol label from the planning contract.
    expected: String,
    /// Actual protocol label emitted by setup.
    actual: String,
  },
  /// The emitted setup verification key does not match the expected curve label.
  #[error("outer setup verification-key curve mismatch: expected '{expected}', got '{actual}'")]
  VerificationKeyCurveMismatch {
    /// Expected curve label from the planning contract.
    expected: String,
    /// Actual curve label emitted by setup.
    actual: String,
  },
  /// The emitted setup verification key does not match the expected public-input arity.
  #[error("outer setup verification-key nPublic mismatch: expected {expected}, got {actual}")]
  VerificationKeyPublicInputCountMismatch {
    /// Expected `nPublic` from the planning contract.
    expected: usize,
    /// Actual `nPublic` emitted by setup.
    actual: usize,
  },
  /// The emitted setup verification key does not match the expected IC table length.
  #[error("outer setup verification-key IC length mismatch: expected {expected}, got {actual}")]
  VerificationKeyShapeMismatch {
    /// Expected IC table length from the planning contract.
    expected: usize,
    /// Actual IC table length emitted by setup.
    actual: usize,
  },
  /// The emitted setup verification key does not preserve the planned snarkjs-like top-level keys.
  #[error(
    "outer setup verification-key top-level key mismatch: expected {expected:?}, got {actual:?}"
  )]
  VerificationKeyTopLevelKeysMismatch {
    /// Expected top-level key set from the planning contract.
    expected: Vec<String>,
    /// Actual top-level key set emitted by setup.
    actual: Vec<String>,
  },
  /// The emitted proof does not match the expected protocol label.
  #[error("outer produced proof protocol mismatch: expected '{expected}', got '{actual}'")]
  ProofProtocolMismatch {
    /// Expected protocol label from the planning contract.
    expected: String,
    /// Actual protocol label emitted by proving.
    actual: String,
  },
  /// The emitted proof does not match the expected curve label.
  #[error("outer produced proof curve mismatch: expected '{expected}', got '{actual}'")]
  ProofCurveMismatch {
    /// Expected curve label from the planning contract.
    expected: String,
    /// Actual curve label emitted by proving.
    actual: String,
  },
  /// The emitted proof does not preserve the planned snarkjs-like top-level keys.
  #[error("outer produced proof top-level key mismatch: expected {expected:?}, got {actual:?}")]
  ProofTopLevelKeysMismatch {
    /// Expected top-level key set from the planning contract.
    expected: Vec<String>,
    /// Actual top-level key set emitted by proving.
    actual: Vec<String>,
  },
  /// Real outer proof generation is blocked because no concrete outer prover exists yet.
  #[error(
    "outer proof generation is not yet possible: missing {prover_kind} prover/serializer for {circuit_stack}"
  )]
  MissingOuterProofBackend {
    /// Circuit stack currently used by the outer circuit.
    circuit_stack: &'static str,
    /// Missing prover/serializer capability.
    prover_kind: &'static str,
  },
  /// Real direct proving of the canonical outer Halo2/Midnight circuit is not wired yet.
  #[error(
    "outer backend '{backend}' has no direct prover/serializer for the canonical outer circuit '{circuit_stack}' yet"
  )]
  MissingDirectOuterCircuitBackend {
    /// Backend identifier.
    backend: &'static str,
    /// Canonical outer circuit stack expected by the direct backend surface.
    circuit_stack: &'static str,
  },
  /// The selected backend requires a canonical outer R1CS lowering that does not exist yet.
  #[error(
    "outer backend '{backend}' requires canonical R1CS lowering for {circuit_stack}, but none is implemented yet"
  )]
  MissingOuterCanonicalR1csLowering {
    /// Backend identifier.
    backend: &'static str,
    /// Circuit stack whose canonical R1CS lowering is still missing.
    circuit_stack: &'static str,
  },
  /// The adapted outer circuit input is invalid for synthesis.
  #[error("outer circuit input is not ready for synthesis: {reason}")]
  OuterCircuitInputInvalid {
    /// Human-readable explanation of the rejected circuit input.
    reason: String,
  },
  /// The chosen backend lane only supports the current frozen mirror-statement layout.
  #[error("unsupported outer statement layout: {reason}")]
  UnsupportedStatementLayout {
    /// Human-readable explanation of the rejected layout.
    reason: String,
  },
  /// The backend does not implement the requested execution phase yet.
  #[error(
    "outer backend '{backend}' does not implement '{operation}' in the current repository phase"
  )]
  UnsupportedOperation {
    /// Backend identifier.
    backend: &'static str,
    /// Requested operation.
    operation: &'static str,
  },
}

/// Backend contract for producing outer Groth16 artifacts.
pub trait OuterProofBackend {
  /// Returns static metadata for the selected backend stack.
  fn metadata(&self) -> &'static OuterProofBackendMetadata;

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

/// Placeholder backend for the planned direct Halo2/Midnight outer proof system.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlannedHalo2OuterBackend;

/// Selected concrete backend for the direct Halo2/Midnight outer lane.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MidnightDirectOuterBackend;

const PLANNED_BACKEND_METADATA: OuterProofBackendMetadata = OuterProofBackendMetadata {
  backend_id: "planned-halo2-outer-backend",
  stack: "planning-only placeholder",
  protocol: "halo2-plonkish",
  curve: "bn254",
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

const MIDNIGHT_DIRECT_BACKEND_METADATA: OuterProofBackendMetadata = OuterProofBackendMetadata {
  backend_id: "midnight-direct-halo2-outer-backend",
  stack: "direct halo2/midnight outer lane over the canonical outer wrapper circuit",
  protocol: "halo2-plonkish",
  curve: "bn254",
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
    "proof generation and verification still need dedicated stage-5 and stage-6 wiring",
    "setup is already real and runs directly over the canonical outer circuit",
    "artifact shapes remain aligned with the direct wrapper-core output model",
  ],
};

fn ensure_supported_target(
  package: &WrapperExecutionPackage,
) -> Result<(), OuterProofBackendError> {
  if package.job.target.kind != ProofSystemKind::Halo2Outer {
    return Err(OuterProofBackendError::UnsupportedTarget {
      expected: "halo2-outer",
      actual: match package.job.target.kind {
        ProofSystemKind::Groth16Bn254 => "groth16-bn254",
        ProofSystemKind::Groth16Bls12_381 => "groth16-bls12-381",
        ProofSystemKind::Halo2Outer => "halo2-outer",
      },
    });
  }

  Ok(())
}

fn parse_native_input_value(
  context: &'static str,
  field_name: &str,
  value: &str,
) -> Result<NativeField, OuterProofBackendError> {
  if let Some(hex) = value.strip_prefix("0x") {
    let mut accumulator = NativeField::ZERO;
    let radix = NativeField::from(16_u64);

    for ch in hex.chars() {
      let digit =
        ch.to_digit(16).ok_or_else(|| OuterProofBackendError::InvalidPublicInputValue {
          context,
          field_name: field_name.to_owned(),
          value: value.to_owned(),
        })?;
      accumulator = accumulator * radix + NativeField::from(u64::from(digit));
    }

    return Ok(accumulator);
  }

  NativeField::from_str_vartime(value).ok_or_else(|| {
    OuterProofBackendError::InvalidPublicInputValue {
      context,
      field_name: field_name.to_owned(),
      value: value.to_owned(),
    }
  })
}

fn hex_encode(bytes: &[u8]) -> String {
  let mut encoded = String::with_capacity(bytes.len() * 2);
  for byte in bytes {
    use std::fmt::Write as _;
    let _ = write!(&mut encoded, "{byte:02x}");
  }
  encoded
}

fn hex_decode(value: &str) -> Result<Vec<u8>, OuterProofBackendError> {
  if value.len() % 2 != 0 {
    return Err(OuterProofBackendError::OuterCircuitInputInvalid {
      reason: "hex payload has odd length".to_owned(),
    });
  }

  let mut bytes = Vec::with_capacity(value.len() / 2);
  for index in (0..value.len()).step_by(2) {
    let byte = u8::from_str_radix(&value[index..index + 2], 16).map_err(|error| {
      OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("invalid hex payload at byte {}: {error}", index / 2),
      }
    })?;
    bytes.push(byte);
  }

  Ok(bytes)
}

fn outer_instance_columns(circuit: &OuterWrapperCircuit) -> [&[NativeField]; 2] {
  [circuit.input.outer_statement.public_inputs.as_slice(), &[]]
}

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

impl OuterProofBackend for MidnightDirectOuterBackend {
  fn metadata(&self) -> &'static OuterProofBackendMetadata {
    &MIDNIGHT_DIRECT_BACKEND_METADATA
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

    let mut planned = package.expected_output();
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
    planned.bundle_template.notes.push(
      "selected backend is the direct halo2/midnight lane; proof production remains pending after setup"
        .to_owned(),
    );

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

impl MidnightDirectOuterBackend {
  /// Builds the setup plan for the selected direct outer backend lane.
  ///
  /// # Errors
  ///
  /// Returns an error if the package does not satisfy the selected backend's
  /// target proof system or frozen outer-statement contract.
  pub fn plan_setup(
    &self,
    package: &WrapperExecutionPackage,
  ) -> Result<DirectOuterSetupPlan, OuterProofBackendError> {
    let planned = self.prepare(package)?;

    Ok(DirectOuterSetupPlan {
      verification_key_artifact: planned.verification_key_artifact,
      expected_public_input_count: package.statement.public_inputs.entries.len(),
      expected_pcs: planned.verification_key_shape.pcs,
      notes: vec![
        format!("selected outer backend stack: {}", self.metadata().stack),
        format!(
          "expected setup verification-key protocol/curve: {}/{}",
          planned.verification_key_shape.protocol, planned.verification_key_shape.curve
        ),
        format!(
          "expected setup verification-key payload keys: {:?}",
          planned.verification_key_shape.top_level_keys
        ),
      ],
    })
  }

  /// Builds the proving plan for the selected direct outer backend lane.
  ///
  /// # Errors
  ///
  /// Returns an error if the package does not satisfy the selected backend's
  /// target proof system or frozen outer-statement contract.
  pub fn plan_proof(
    &self,
    package: &WrapperExecutionPackage,
  ) -> Result<DirectOuterProofPlan, OuterProofBackendError> {
    let planned = self.prepare(package)?;

    Ok(DirectOuterProofPlan {
      proof_artifact: planned.proof_artifact,
      public_inputs_artifact: planned.public_inputs_artifact,
      public_inputs: package
        .statement
        .public_inputs
        .entries
        .iter()
        .map(|entry| entry.value.clone())
        .collect(),
      expected_transcript: planned.proof_shape.transcript,
      notes: vec![
        format!("selected outer backend stack: {}", self.metadata().stack),
        format!(
          "expected produced proof protocol/curve: {}/{}",
          planned.proof_shape.protocol, planned.proof_shape.curve
        ),
        format!("produced proof must keep top-level keys {:?}", planned.proof_shape.top_level_keys),
      ],
    })
  }

  fn produce_setup_verification_key(
    self,
    package: &WrapperExecutionPackage,
    circuit: &OuterWrapperCircuit,
  ) -> Result<ProducedOuterVerificationKeyJson, OuterProofBackendError> {
    let planned = self.prepare(package)?;
    let k = k_from_circuit(circuit);
    let params = KZGCommitmentScheme::<Bn256>::gen_params(k);
    let vk = keygen_vk_with_k::<NativeField, KZGCommitmentScheme<Bn256>, _>(&params, circuit, k)
      .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("midnight keygen_vk failed: {error}"),
      })?;

    self.serialize_setup_verification_key(package, &planned, k, &params, &vk)
  }

  fn serialize_setup_verification_key(
    self,
    package: &WrapperExecutionPackage,
    planned: &ExpectedWrapperArtifacts,
    k: u32,
    params: &<KZGCommitmentScheme<Bn256> as PolynomialCommitmentScheme<NativeField>>::Parameters,
    vk: &midnight_proofs::plonk::VerifyingKey<NativeField, KZGCommitmentScheme<Bn256>>,
  ) -> Result<ProducedOuterVerificationKeyJson, OuterProofBackendError> {
    let verification_key = ProducedOuterVerificationKeyJson {
      protocol: planned.proof_shape.protocol.clone(),
      curve: planned.verification_key_shape.curve.clone(),
      backend: planned.verification_key_shape.backend.clone(),
      pcs: planned.verification_key_shape.pcs.clone(),
      encoding: planned.verification_key_shape.payload_encoding.clone(),
      circuit_k: k,
      public_input_count: package.statement.public_inputs.entries.len(),
      verification_key: hex_encode(&vk.to_bytes(SerdeFormat::Processed)),
      verifier_params: {
        let mut bytes = Vec::new();
        params.verifier_params().write(&mut bytes, SerdeFormat::Processed).map_err(|error| {
          OuterProofBackendError::OuterCircuitInputInvalid {
            reason: format!("failed to serialize verifier params: {error}"),
          }
        })?;
        hex_encode(&bytes)
      },
    };

    self.validate_setup_verification_key(package, &verification_key)?;
    Ok(verification_key)
  }

  fn produce_proof_bundle(
    self,
    package: &WrapperExecutionPackage,
    circuit: &OuterWrapperCircuit,
  ) -> Result<ProducedOuterProofArtifactBundle, OuterProofBackendError> {
    let planned = self.prepare(package)?;
    let k = k_from_circuit(circuit);
    let params = KZGCommitmentScheme::<Bn256>::gen_params(k);
    let vk = keygen_vk_with_k::<NativeField, KZGCommitmentScheme<Bn256>, _>(&params, circuit, k)
      .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("midnight keygen_vk failed during proving: {error}"),
      })?;
    let pk = keygen_pk::<NativeField, KZGCommitmentScheme<Bn256>, _>(vk.clone(), circuit).map_err(
      |error| OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("midnight keygen_pk failed: {error}"),
      },
    )?;

    let instance_columns = outer_instance_columns(circuit);
    let instances = [&instance_columns[..]];
    let mut transcript = CircuitTranscript::<Blake2bState>::init();

    create_proof::<NativeField, KZGCommitmentScheme<Bn256>, _, _>(
      &params,
      &pk,
      std::slice::from_ref(circuit),
      0,
      &instances,
      OsRng,
      &mut transcript,
    )
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("midnight create_proof failed: {error}"),
    })?;

    let proof = ProducedOuterProofJson {
      protocol: planned.proof_shape.protocol.clone(),
      curve: planned.proof_shape.curve.clone(),
      backend: planned.proof_shape.backend.clone(),
      transcript: planned.proof_shape.transcript.clone(),
      encoding: planned.proof_shape.payload_encoding.clone(),
      proof: hex_encode(&transcript.finalize()),
    };
    self.validate_produced_proof(package, &proof)?;

    let verification_key =
      self.serialize_setup_verification_key(package, &planned, k, &params, &vk)?;
    self.assemble_produced_bundle(package, proof, verification_key)
  }

  fn verify_produced_bundle(
    self,
    package: &WrapperExecutionPackage,
    produced: &ProducedOuterProofArtifactBundle,
    circuit: &OuterWrapperCircuit,
  ) -> Result<bool, OuterProofBackendError> {
    self.validate_produced_proof(package, &produced.proof)?;
    self.validate_setup_verification_key(package, &produced.verification_key)?;

    let verifier_params_bytes = hex_decode(&produced.verification_key.verifier_params)?;
    let verification_key_bytes = hex_decode(&produced.verification_key.verification_key)?;
    let proof_bytes = hex_decode(&produced.proof.proof)?;

    let verifier_params = midnight_proofs::poly::kzg::params::ParamsVerifierKZG::<Bn256>::read(
      &mut verifier_params_bytes.as_slice(),
      SerdeFormat::Processed,
    )
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("failed to deserialize verifier params: {error}"),
    })?;
    let verification_key = midnight_proofs::plonk::VerifyingKey::<
      NativeField,
      KZGCommitmentScheme<Bn256>,
    >::from_bytes::<OuterWrapperCircuit>(
      &verification_key_bytes, SerdeFormat::Processed, ()
    )
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("failed to deserialize verification key: {error}"),
    })?;

    let instance_columns = outer_instance_columns(circuit);
    let instances = [&instance_columns[..]];
    let committed_instances: [&[<KZGCommitmentScheme<Bn256> as PolynomialCommitmentScheme<
      NativeField,
    >>::Commitment]; 1] = [&[]];
    let mut transcript = CircuitTranscript::<Blake2bState>::init_from_bytes(&proof_bytes);
    let guard = prepare::<NativeField, KZGCommitmentScheme<Bn256>, _>(
      &verification_key,
      &committed_instances,
      &instances,
      &mut transcript,
    )
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("failed to prepare proof verification: {error}"),
    })?;
    guard.verify(&verifier_params).map_err(|error| {
      OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("proof verification failed: {error:?}"),
      }
    })?;
    transcript.assert_empty().map_err(|error| {
      OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("proof transcript has trailing bytes: {error}"),
      }
    })?;

    Ok(true)
  }

  /// Validates that one produced setup verification key matches the current
  /// wrapper-core planning contract.
  ///
  /// # Errors
  ///
  /// Returns an error if the verification key does not match the expected
  /// protocol, curve, arity, or top-level field layout.
  pub fn validate_setup_verification_key(
    &self,
    package: &WrapperExecutionPackage,
    verification_key: &ProducedOuterVerificationKeyJson,
  ) -> Result<(), OuterProofBackendError> {
    let planned = self.prepare(package)?;
    let expected_shape = planned.verification_key_shape;

    if verification_key.protocol != expected_shape.protocol {
      return Err(OuterProofBackendError::VerificationKeyProtocolMismatch {
        expected: expected_shape.protocol,
        actual: verification_key.protocol.clone(),
      });
    }

    if verification_key.curve != expected_shape.curve {
      return Err(OuterProofBackendError::VerificationKeyCurveMismatch {
        expected: expected_shape.curve,
        actual: verification_key.curve.clone(),
      });
    }

    if verification_key.public_input_count != package.statement.public_inputs.entries.len() {
      return Err(OuterProofBackendError::VerificationKeyPublicInputCountMismatch {
        expected: package.statement.public_inputs.entries.len(),
        actual: verification_key.public_input_count,
      });
    }

    let serialized = serde_json::to_value(verification_key).expect("produced VK should serialize");
    let mut actual_keys = serialized
      .as_object()
      .expect("produced VK should serialize as a JSON object")
      .keys()
      .cloned()
      .collect::<Vec<_>>();
    let mut expected_keys = expected_shape.top_level_keys;
    actual_keys.sort();
    expected_keys.sort();

    if actual_keys != expected_keys {
      return Err(OuterProofBackendError::VerificationKeyTopLevelKeysMismatch {
        expected: expected_keys,
        actual: actual_keys,
      });
    }

    Ok(())
  }

  /// Validates that one produced outer proof matches the current wrapper-core
  /// planning contract.
  ///
  /// # Errors
  ///
  /// Returns an error if the proof does not match the expected protocol, curve,
  /// or top-level field layout.
  pub fn validate_produced_proof(
    &self,
    package: &WrapperExecutionPackage,
    proof: &ProducedOuterProofJson,
  ) -> Result<(), OuterProofBackendError> {
    let planned = self.prepare(package)?;
    let expected_shape = planned.proof_shape;

    if proof.protocol != expected_shape.protocol {
      return Err(OuterProofBackendError::ProofProtocolMismatch {
        expected: expected_shape.protocol,
        actual: proof.protocol.clone(),
      });
    }

    if proof.curve != expected_shape.curve {
      return Err(OuterProofBackendError::ProofCurveMismatch {
        expected: expected_shape.curve,
        actual: proof.curve.clone(),
      });
    }

    let serialized = serde_json::to_value(proof).expect("produced proof should serialize");
    let mut actual_keys = serialized
      .as_object()
      .expect("produced proof should serialize as a JSON object")
      .keys()
      .cloned()
      .collect::<Vec<_>>();
    let mut expected_keys = expected_shape.top_level_keys;
    actual_keys.sort();
    expected_keys.sort();

    if actual_keys != expected_keys {
      return Err(OuterProofBackendError::ProofTopLevelKeysMismatch {
        expected: expected_keys,
        actual: actual_keys,
      });
    }

    Ok(())
  }

  /// Assembles a strict produced outer artifact bundle from validated proof and
  /// verification-key payloads.
  ///
  /// # Errors
  ///
  /// Returns an error if either the proof or verification key violates the
  /// current planning contract.
  pub fn assemble_produced_bundle(
    &self,
    package: &WrapperExecutionPackage,
    proof: ProducedOuterProofJson,
    verification_key: ProducedOuterVerificationKeyJson,
  ) -> Result<ProducedOuterProofArtifactBundle, OuterProofBackendError> {
    let planned = self.prepare(package)?;
    self.validate_produced_proof(package, &proof)?;
    self.validate_setup_verification_key(package, &verification_key)?;

    Ok(ProducedOuterProofArtifactBundle::new(
      planned.proof_system,
      planned.canonical_circuit_identity,
      planned.proof_artifact,
      proof,
      planned.public_inputs_artifact,
      package
        .statement
        .public_inputs
        .entries
        .iter()
        .map(|entry| entry.value.clone())
        .collect(),
      planned.verification_key_artifact,
      verification_key,
      vec![
        format!("selected outer backend stack: {}", self.metadata().stack),
        "produced bundle matches the current wrapper-core proof and verification-key shape contracts"
          .to_owned(),
      ],
    ))
  }

  /// Adapts a wrapper execution package plus raw inner artifacts into the exact
  /// normalized input shape expected by the selected arkworks outer lane.
  ///
  /// # Errors
  ///
  /// Returns an error if the package is invalid, required inner artifacts are
  /// missing or malformed, the parsed verification key disagrees with package
  /// arity metadata, or the outer statement no longer mirrors the inner
  /// verifier public-input values exactly.
  pub fn adapt_input(
    &self,
    package: &WrapperExecutionPackage,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<DirectOuterCircuitInput, OuterProofBackendError> {
    let _ = self.prepare(package)?;

    let proof_json =
      artifacts.proof_json.ok_or_else(|| OuterProofBackendError::MissingInnerProofPayload {
        source_artifact_id: package.witness.source_artifact_id.clone(),
      })?;
    let verification_key_json = artifacts.verification_key_json.ok_or_else(|| {
      OuterProofBackendError::MissingInnerVerificationKeyPayload {
        source_artifact_id: package.witness.source_artifact_id.clone(),
      }
    })?;

    let inner_proof = parse_groth16_bn254_proof(proof_json).map_err(|source| {
      OuterProofBackendError::MalformedInnerProof {
        source_artifact_id: package.witness.source_artifact_id.clone(),
        source,
      }
    })?;
    let inner_verification_key =
      parse_groth16_bn254_verifying_key(verification_key_json).map_err(|source| {
        OuterProofBackendError::MalformedInnerVerificationKey {
          source_artifact_id: package.witness.source_artifact_id.clone(),
          source,
        }
      })?;

    if inner_verification_key.ic.len() != package.witness.verification_key_ic_count {
      return Err(OuterProofBackendError::VerificationKeyIcCountMismatch {
        expected: package.witness.verification_key_ic_count,
        actual: inner_verification_key.ic.len(),
      });
    }

    let inner_verifier_public_inputs = package
      .witness
      .verifier_public_inputs
      .entries
      .iter()
      .map(|entry| parse_native_input_value("inner-witness", &entry.name, &entry.value))
      .collect::<Result<Vec<_>, _>>()?;

    let outer_statement = DirectOuterStatementInput {
      field_names: package
        .statement
        .public_inputs
        .entries
        .iter()
        .map(|entry| entry.name.clone())
        .collect(),
      public_inputs: package
        .statement
        .public_inputs
        .entries
        .iter()
        .map(|entry| parse_native_input_value("outer-statement", &entry.name, &entry.value))
        .collect::<Result<Vec<_>, _>>()?,
    };

    if outer_statement.public_inputs != inner_verifier_public_inputs {
      return Err(OuterProofBackendError::UnsupportedStatementLayout {
        reason: "current arkworks outer lane only supports an outer statement that mirrors inner verifier public-input values exactly"
          .to_owned(),
      });
    }

    Ok(DirectOuterCircuitInput {
      source_artifact_id: package.witness.source_artifact_id.clone(),
      inner_proof,
      inner_verification_key,
      inner_verifier_public_inputs,
      outer_statement,
    })
  }

  /// Builds the canonical outer wrapper circuit from package plus raw artifacts.
  ///
  /// # Errors
  ///
  /// Returns an error if adaptation fails or the circuit-owned input is not
  /// ready for synthesis under the frozen outer statement contract.
  pub fn build_outer_circuit(
    &self,
    package: &WrapperExecutionPackage,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<OuterWrapperCircuit, OuterProofBackendError> {
    let adapted = self.adapt_input(package, artifacts)?;
    let circuit = build_outer_wrapper_circuit(adapted.to_circuit_input());
    circuit.assert_ready_for_synthesis().map_err(|error| {
      OuterProofBackendError::OuterCircuitInputInvalid { reason: error.to_string() }
    })?;
    Ok(circuit)
  }

  /// Plans the direct canonical outer-circuit setup lane.
  ///
  /// # Errors
  ///
  /// Returns an error if the circuit is not ready for synthesis or if direct
  /// proving of the canonical outer circuit is not wired yet.
  pub fn plan_direct_outer_circuit_setup(
    &self,
    package: &WrapperExecutionPackage,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<CanonicalOuterCircuitSetupArtifacts, OuterProofBackendError> {
    let planned = self.prepare(package)?;
    let circuit = self.build_outer_circuit(package, artifacts)?;
    self.plan_canonical_setup(&circuit, &planned.verification_key_artifact)
  }

  /// Plans the direct canonical outer-circuit proving lane.
  ///
  /// # Errors
  ///
  /// Returns an error if the circuit is not ready for synthesis or if direct
  /// proving of the canonical outer circuit is not wired yet.
  pub fn plan_direct_outer_circuit_proof(
    &self,
    package: &WrapperExecutionPackage,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<CanonicalOuterCircuitProofArtifacts, OuterProofBackendError> {
    let planned = self.prepare(package)?;
    let circuit = self.build_outer_circuit(package, artifacts)?;
    self.plan_canonical_proof(
      &circuit,
      &planned.proof_artifact,
      &planned.public_inputs_artifact,
      &package
        .statement
        .public_inputs
        .entries
        .iter()
        .map(|entry| entry.value.clone())
        .collect::<Vec<_>>(),
    )
  }

  /// Builds the canonical outer R1CS once the outer Halo2/Midnight circuit has
  /// a deterministic lowering path.
  ///
  /// # Errors
  ///
  /// Returns an error if the adapted outer circuit is invalid or if canonical
  /// outer R1CS lowering has not been implemented yet.
  pub fn build_outer_canonical_r1cs(
    &self,
    package: &WrapperExecutionPackage,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<R1csCircuit, OuterProofBackendError> {
    let adapted = self.adapt_input(package, artifacts)?;
    build_outer_wrapper_canonical_r1cs(&adapted.to_circuit_input()).map_err(|_| {
      OuterProofBackendError::MissingOuterCanonicalR1csLowering {
        backend: OuterProofBackend::backend_id(self),
        circuit_stack: "halo2/midnight outer wrapper circuit",
      }
    })
  }
}

impl CanonicalOuterCircuitProofBackend for MidnightDirectOuterBackend {
  fn backend_id(&self) -> &'static str {
    OuterProofBackend::backend_id(self)
  }

  fn plan_canonical_setup(
    &self,
    circuit: &OuterWrapperCircuit,
    verification_key_artifact: &str,
  ) -> Result<CanonicalOuterCircuitSetupArtifacts, OuterProofBackendError> {
    circuit.assert_ready_for_synthesis().map_err(|error| {
      OuterProofBackendError::OuterCircuitInputInvalid { reason: error.to_string() }
    })?;

    Ok(CanonicalOuterCircuitSetupArtifacts {
      build_status: match circuit.build_status() {
        CircuitBuildStatus::VerifierIntegrated => "verifier-integrated",
      },
      verification_key_artifact: verification_key_artifact.to_owned(),
      expected_public_input_count: circuit.input.outer_statement.public_inputs.len(),
      notes: vec![
        "canonical outer circuit is ready for synthesis".to_owned(),
        "real direct setup wiring is available through midnight_proofs keygen".to_owned(),
      ],
    })
  }

  fn plan_canonical_proof(
    &self,
    circuit: &OuterWrapperCircuit,
    proof_artifact: &str,
    public_inputs_artifact: &str,
    public_inputs: &[String],
  ) -> Result<CanonicalOuterCircuitProofArtifacts, OuterProofBackendError> {
    circuit.assert_ready_for_synthesis().map_err(|error| {
      OuterProofBackendError::OuterCircuitInputInvalid { reason: error.to_string() }
    })?;

    Ok(CanonicalOuterCircuitProofArtifacts {
      build_status: match circuit.build_status() {
        CircuitBuildStatus::VerifierIntegrated => "verifier-integrated",
      },
      proof_artifact: proof_artifact.to_owned(),
      public_inputs_artifact: public_inputs_artifact.to_owned(),
      public_inputs: public_inputs.to_vec(),
      notes: vec![
        "canonical outer circuit is ready for synthesis".to_owned(),
        "real direct proof wiring remains pending after setup".to_owned(),
      ],
    })
  }
}

#[cfg(test)]
mod tests {
  use wrapper_circuits::groth16_fixture_raw;
  use wrapper_core::{ProducedOuterProofJson, ProofSystemKind};

  use crate::parse_snarkjs_groth16_bn254_bundle;

  use super::{
    MidnightDirectOuterBackend, OuterCircuitInputArtifacts, OuterProofBackend,
    OuterProofBackendError, PlannedHalo2OuterBackend,
  };

  fn real_fixture_package() -> wrapper_core::WrapperExecutionPackage {
    parse_snarkjs_groth16_bn254_bundle(
      "circom-multiplier2",
      groth16_fixture_raw::proof_json(),
      groth16_fixture_raw::public_inputs_json(),
      groth16_fixture_raw::verification_key_json(),
    )
    .expect("fixture bundle should parse")
    .build_halo2_outer_execution_package()
  }

  #[test]
  fn planned_backend_prepares_halo2_outer_placeholder_bundle() {
    let backend = PlannedHalo2OuterBackend;
    let planned = backend
      .prepare(&real_fixture_package())
      .expect("planned backend should accept halo2-outer target");

    assert_eq!(backend.backend_id(), "planned-halo2-outer-backend");
    assert_eq!(planned.bundle_template.proof_system.kind, ProofSystemKind::Halo2Outer);
    assert_eq!(
      planned
        .bundle_template
        .verification_key
        .as_ref()
        .expect("planned bundle should materialize a verification-key skeleton")
        .curve,
      "bn254"
    );
  }

  #[test]
  fn direct_backend_exposes_selected_stack_metadata() {
    let metadata = MidnightDirectOuterBackend.metadata();

    assert_eq!(metadata.backend_id, "midnight-direct-halo2-outer-backend");
    assert_eq!(metadata.protocol, "halo2-plonkish");
    assert_eq!(metadata.curve, "bn254");
  }

  #[test]
  #[ignore = "slow outer proving"]
  fn direct_backend_can_plan_setup_and_produce_real_vk_artifact() {
    let backend = MidnightDirectOuterBackend;
    let package = real_fixture_package();
    let plan =
      backend.plan_setup(&package).expect("setup planning should succeed for a valid package");
    let vk = backend
      .setup(
        &package,
        OuterCircuitInputArtifacts::new(
          Some(groth16_fixture_raw::proof_json()),
          Some(groth16_fixture_raw::verification_key_json()),
        ),
      )
      .expect("setup should produce a real VK artifact");

    assert_eq!(plan.expected_public_input_count, 1);
    assert_eq!(plan.expected_pcs, "kzg");
    assert_eq!(vk.protocol, "halo2-plonkish");
    assert_eq!(vk.curve, "bn254");
    assert_eq!(vk.public_input_count, 1);
    assert!(!vk.verification_key.is_empty());
    assert!(!vk.verifier_params.is_empty());
  }

  #[test]
  #[ignore = "slow outer proving"]
  fn direct_backend_can_produce_real_proof_bundle() {
    let backend = MidnightDirectOuterBackend;
    let package = real_fixture_package();
    let bundle = backend
      .prove(
        &package,
        OuterCircuitInputArtifacts::new(
          Some(groth16_fixture_raw::proof_json()),
          Some(groth16_fixture_raw::verification_key_json()),
        ),
      )
      .expect("prove should produce a real proof bundle");

    assert_eq!(bundle.proof.protocol, "halo2-plonkish");
    assert_eq!(bundle.proof.curve, "bn254");
    assert_eq!(bundle.proof.backend, "midnight-direct-halo2-outer-backend");
    assert_eq!(bundle.proof.transcript, "blake2b");
    assert!(!bundle.proof.proof.is_empty());
    assert_eq!(bundle.verification_key.public_input_count, 1);
    assert_eq!(
      bundle.public_inputs,
      package
        .statement
        .public_inputs
        .entries
        .iter()
        .map(|entry| entry.value.clone())
        .collect::<Vec<_>>()
    );
  }

  #[test]
  #[ignore = "slow outer proving"]
  fn direct_backend_can_verify_real_proof_bundle() {
    let backend = MidnightDirectOuterBackend;
    let package = real_fixture_package();
    let bundle = backend
      .prove(
        &package,
        OuterCircuitInputArtifacts::new(
          Some(groth16_fixture_raw::proof_json()),
          Some(groth16_fixture_raw::verification_key_json()),
        ),
      )
      .expect("prove should produce a real proof bundle");

    assert!(
      backend
        .verify(
          &package,
          &bundle,
          OuterCircuitInputArtifacts::new(
            Some(groth16_fixture_raw::proof_json()),
            Some(groth16_fixture_raw::verification_key_json()),
          ),
        )
        .expect("verify should accept the produced proof bundle")
    );
  }

  #[test]
  fn direct_backend_rejects_proof_with_wrong_curve() {
    let backend = MidnightDirectOuterBackend;
    let package = real_fixture_package();
    let proof = ProducedOuterProofJson {
      protocol: "halo2-plonkish".to_owned(),
      curve: "bls12-381".to_owned(),
      backend: "midnight-direct-halo2-outer-backend".to_owned(),
      transcript: "blake2b".to_owned(),
      encoding: "hex".to_owned(),
      proof: "beef".to_owned(),
    };

    assert!(matches!(
      backend.validate_produced_proof(&package, &proof),
      Err(OuterProofBackendError::ProofCurveMismatch { .. })
    ));
  }
}
