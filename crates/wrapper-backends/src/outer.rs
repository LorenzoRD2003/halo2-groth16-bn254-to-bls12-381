//! Outer Groth16 backend contracts, backend metadata, and input adapters.

use ff::{Field, PrimeField};
use thiserror::Error;
use wrapper_circuits::{
  CircuitBuildStatus, Groth16Bn254Proof, Groth16Bn254VerifyingKey, NativeField,
  OuterStatementInput, OuterStatementSemantics, OuterWrapperCircuit, OuterWrapperCircuitInput,
  R1csCircuit, build_outer_wrapper_canonical_r1cs, build_outer_wrapper_circuit,
};
use wrapper_core::{
  ExpectedWrapperArtifacts, OuterStatementContractError, ProducedOuterGroth16ArtifactBundle,
  ProducedOuterGroth16ProofJson, ProducedOuterGroth16VerificationKeyJson, ProofSystemKind,
  WrapperExecutionPackage,
};

use crate::snarkjs::{
  SnarkjsGroth16ParseError, parse_groth16_bn254_proof, parse_groth16_bn254_verifying_key,
};

/// Static metadata describing one selected outer Groth16 backend stack.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OuterGroth16BackendMetadata {
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
pub struct ArkworksGroth16OuterStatementInput {
  /// Ordered semantic public-input names.
  pub field_names: Vec<String>,
  /// Ordered field values for the outer public statement.
  pub public_inputs: Vec<NativeField>,
}

/// Exact witness/config input shape expected by the chosen arkworks outer backend lane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArkworksGroth16OuterCircuitInput {
  /// Logical identifier of the inner artifact set.
  pub source_artifact_id: String,
  /// Parsed inner Groth16 BN254 proof.
  pub inner_proof: Groth16Bn254Proof,
  /// Parsed inner Groth16 BN254 verification key.
  pub inner_verification_key: Groth16Bn254VerifyingKey,
  /// Ordered inner verifier public inputs, normalized to field elements.
  pub inner_verifier_public_inputs: Vec<NativeField>,
  /// Outer public statement normalized for the selected backend lane.
  pub outer_statement: ArkworksGroth16OuterStatementInput,
}

impl ArkworksGroth16OuterCircuitInput {
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

/// Setup-time plan for the selected arkworks outer backend lane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArkworksGroth16SetupPlan {
  /// Logical verification-key artifact identifier.
  pub verification_key_artifact: String,
  /// Expected outer public-input count.
  pub expected_n_public: usize,
  /// Expected IC table length.
  pub expected_ic_len: usize,
  /// Setup notes for the selected backend lane.
  pub notes: Vec<String>,
}

/// Proving-time plan for the selected arkworks outer backend lane.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ArkworksGroth16ProofPlan {
  /// Logical proof artifact identifier.
  pub proof_artifact: String,
  /// Logical public-input artifact identifier.
  pub public_inputs_artifact: String,
  /// Ordered public inputs that the produced proof must expose.
  pub public_inputs: Vec<String>,
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
  pub expected_n_public: usize,
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
/// This sits below `OuterGroth16Backend` and above any concrete prover /
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
  ) -> Result<CanonicalOuterCircuitSetupArtifacts, OuterGroth16BackendError>;

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
  ) -> Result<CanonicalOuterCircuitProofArtifacts, OuterGroth16BackendError>;
}

/// Errors raised while producing outer Groth16 artifacts.
#[derive(Debug, Error)]
pub enum OuterGroth16BackendError {
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
pub trait OuterGroth16Backend {
  /// Returns static metadata for the selected backend stack.
  fn metadata(&self) -> &'static OuterGroth16BackendMetadata;

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
  ) -> Result<ExpectedWrapperArtifacts, OuterGroth16BackendError>;

  /// Runs setup for the outer backend and emits a real verification key once supported.
  ///
  /// # Errors
  ///
  /// Returns an error if setup is not implemented or the package is invalid.
  fn setup(
    &self,
    package: &WrapperExecutionPackage,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<ProducedOuterGroth16VerificationKeyJson, OuterGroth16BackendError>;

  /// Produces a real outer Groth16 artifact bundle once supported.
  ///
  /// # Errors
  ///
  /// Returns an error if proving is not implemented or the package is invalid.
  fn prove(
    &self,
    package: &WrapperExecutionPackage,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<ProducedOuterGroth16ArtifactBundle, OuterGroth16BackendError>;

  /// Verifies a produced outer Groth16 artifact bundle against the package statement.
  ///
  /// # Errors
  ///
  /// Returns an error if verification is not implemented or the inputs are invalid.
  fn verify(
    &self,
    package: &WrapperExecutionPackage,
    produced: &ProducedOuterGroth16ArtifactBundle,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<bool, OuterGroth16BackendError>;
}

/// Placeholder backend for the planned Groth16 BLS12-381 outer proof system.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlannedGroth16Bls12381Backend;

/// Selected concrete backend for the outer Groth16 BLS12-381 lane.
///
/// This backend chooses the arkworks Groth16 stack as the implementation
/// target while steps 5-8 are still pending.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ArkworksGroth16Bls12381Backend;

const PLANNED_BACKEND_METADATA: OuterGroth16BackendMetadata = OuterGroth16BackendMetadata {
  backend_id: "planned-groth16-bls12-381-backend",
  stack: "planning-only placeholder",
  protocol: "groth16",
  curve: "bls12-381",
  setup_assumptions: &[
    "no proving stack is bound yet",
    "prepare() only materializes the planned artifact contract",
  ],
  serialization_conventions: &[
    "public inputs stay as decimal-string JSON arrays",
    "verification key skeleton follows snarkjs-like field names such as nPublic and IC",
  ],
  compatibility_notes: &[
    "proof payload remains absent by construction",
    "use only for planning/materialization, not for setup/prove/verify",
  ],
};

const ARKWORKS_BACKEND_METADATA: OuterGroth16BackendMetadata = OuterGroth16BackendMetadata {
  backend_id: "arkworks-groth16-bls12-381-backend",
  stack: "canonical R1CS -> arkworks Groth16 outer lane targeting Groth16 BLS12-381 artifacts",
  protocol: "groth16",
  curve: "bls12-381",
  setup_assumptions: &[
    "the outer circuit is authored in halo2/midnight and remains the canonical outer circuit surface",
    "the first real backend proves the deterministic canonical R1CS lowering, not the halo2/midnight circuit directly",
    "the concrete backend will use one Groth16 CRS per canonical R1CS identity",
    "trusted setup output must be serialized once and then reused across proofs for that canonical R1CS identity",
    "the wrapper statement mirrors the ordered inner verifier public inputs exactly",
  ],
  serialization_conventions: &[
    "proof.json uses snarkjs-like keys pi_a, pi_b, pi_c plus protocol and curve labels",
    "wrapper-public.json is a JSON decimal-string array in wrapper statement order",
    "wrapper-verification-key.json uses nPublic and IC and keeps projective decimal-string point encodings",
  ],
  compatibility_notes: &[
    "the current backend targets canonical R1CS produced by the deterministic lowering path",
    "the halo2/midnight outer circuit is not proved directly in this phase",
    "artifact shapes remain aligned with the current wrapper-core expected output model",
    "backend use must be rejected until the outer circuit has a canonical R1CS lowering",
  ],
};

fn ensure_supported_target(
  package: &WrapperExecutionPackage,
) -> Result<(), OuterGroth16BackendError> {
  if package.job.target.kind != ProofSystemKind::Groth16Bls12_381 {
    return Err(OuterGroth16BackendError::UnsupportedTarget {
      expected: "groth16-bls12-381",
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
) -> Result<NativeField, OuterGroth16BackendError> {
  if let Some(hex) = value.strip_prefix("0x") {
    let mut accumulator = NativeField::ZERO;
    let radix = NativeField::from(16_u64);

    for ch in hex.chars() {
      let digit =
        ch.to_digit(16).ok_or_else(|| OuterGroth16BackendError::InvalidPublicInputValue {
          context,
          field_name: field_name.to_owned(),
          value: value.to_owned(),
        })?;
      accumulator = accumulator * radix + NativeField::from(u64::from(digit));
    }

    return Ok(accumulator);
  }

  NativeField::from_str_vartime(value).ok_or_else(|| {
    OuterGroth16BackendError::InvalidPublicInputValue {
      context,
      field_name: field_name.to_owned(),
      value: value.to_owned(),
    }
  })
}

impl OuterGroth16Backend for PlannedGroth16Bls12381Backend {
  fn metadata(&self) -> &'static OuterGroth16BackendMetadata {
    &PLANNED_BACKEND_METADATA
  }

  fn backend_id(&self) -> &'static str {
    self.metadata().backend_id
  }

  fn prepare(
    &self,
    package: &WrapperExecutionPackage,
  ) -> Result<ExpectedWrapperArtifacts, OuterGroth16BackendError> {
    ensure_supported_target(package)?;
    package.validate_outer_statement_contract()?;
    Ok(package.expected_output())
  }

  fn setup(
    &self,
    package: &WrapperExecutionPackage,
    _artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<ProducedOuterGroth16VerificationKeyJson, OuterGroth16BackendError> {
    let _ = self.prepare(package)?;
    Err(OuterGroth16BackendError::UnsupportedOperation {
      backend: OuterGroth16Backend::backend_id(self),
      operation: "setup",
    })
  }

  fn prove(
    &self,
    package: &WrapperExecutionPackage,
    _artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<ProducedOuterGroth16ArtifactBundle, OuterGroth16BackendError> {
    let _ = self.prepare(package)?;
    Err(OuterGroth16BackendError::UnsupportedOperation {
      backend: OuterGroth16Backend::backend_id(self),
      operation: "prove",
    })
  }

  fn verify(
    &self,
    package: &WrapperExecutionPackage,
    _produced: &ProducedOuterGroth16ArtifactBundle,
    _artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<bool, OuterGroth16BackendError> {
    let _ = self.prepare(package)?;
    Err(OuterGroth16BackendError::UnsupportedOperation {
      backend: OuterGroth16Backend::backend_id(self),
      operation: "verify",
    })
  }
}

impl OuterGroth16Backend for ArkworksGroth16Bls12381Backend {
  fn metadata(&self) -> &'static OuterGroth16BackendMetadata {
    &ARKWORKS_BACKEND_METADATA
  }

  fn backend_id(&self) -> &'static str {
    self.metadata().backend_id
  }

  fn prepare(
    &self,
    package: &WrapperExecutionPackage,
  ) -> Result<ExpectedWrapperArtifacts, OuterGroth16BackendError> {
    ensure_supported_target(package)?;
    package.validate_outer_statement_contract()?;

    let mut planned = package.expected_output();
    planned.notes.push(format!("selected outer backend stack: {}", self.metadata().stack));
    planned.notes.push(
      "outer statement contract is frozen to mirror ordered inner verifier public inputs"
        .to_owned(),
    );
    planned.notes.push(
      "selected real backend mode is canonical R1CS -> arkworks Groth16; the halo2/midnight circuit is not proved directly"
        .to_owned(),
    );
    planned.notes.push(
      "canonical circuit identity is not attached yet: outer circuit -> canonical R1CS lowering is still pending"
        .to_owned(),
    );
    planned
      .notes
      .extend(self.metadata().serialization_conventions.iter().map(|note| (*note).to_owned()));
    planned.bundle_template.notes.push(
      "selected backend is the arkworks Groth16 BLS12-381 lane over canonical R1CS; outer circuit lowering remains pending"
        .to_owned(),
    );

    Ok(planned)
  }

  fn setup(
    &self,
    package: &WrapperExecutionPackage,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<ProducedOuterGroth16VerificationKeyJson, OuterGroth16BackendError> {
    let _ = self.build_outer_canonical_r1cs(package, artifacts)?;
    Err(OuterGroth16BackendError::UnsupportedOperation {
      backend: OuterGroth16Backend::backend_id(self),
      operation: "setup after canonical outer R1CS lowering",
    })
  }

  fn prove(
    &self,
    package: &WrapperExecutionPackage,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<ProducedOuterGroth16ArtifactBundle, OuterGroth16BackendError> {
    let _ = self.build_outer_canonical_r1cs(package, artifacts)?;
    Err(OuterGroth16BackendError::UnsupportedOperation {
      backend: OuterGroth16Backend::backend_id(self),
      operation: "prove after canonical outer R1CS lowering",
    })
  }

  fn verify(
    &self,
    package: &WrapperExecutionPackage,
    _produced: &ProducedOuterGroth16ArtifactBundle,
    artifacts: OuterCircuitInputArtifacts<'_>,
  ) -> Result<bool, OuterGroth16BackendError> {
    let _ = self.build_outer_canonical_r1cs(package, artifacts)?;
    Err(OuterGroth16BackendError::UnsupportedOperation {
      backend: OuterGroth16Backend::backend_id(self),
      operation: "verify after canonical outer R1CS lowering",
    })
  }
}

impl ArkworksGroth16Bls12381Backend {
  /// Builds the setup plan for the selected arkworks outer backend lane.
  ///
  /// # Errors
  ///
  /// Returns an error if the package does not satisfy the selected backend's
  /// target proof system or frozen outer-statement contract.
  pub fn plan_setup(
    &self,
    package: &WrapperExecutionPackage,
  ) -> Result<ArkworksGroth16SetupPlan, OuterGroth16BackendError> {
    let planned = self.prepare(package)?;

    Ok(ArkworksGroth16SetupPlan {
      verification_key_artifact: planned.verification_key_artifact,
      expected_n_public: package.statement.public_inputs.entries.len(),
      expected_ic_len: package.statement.public_inputs.entries.len() + 1,
      notes: vec![
        format!("selected outer backend stack: {}", self.metadata().stack),
        format!(
          "expected setup verification-key protocol/curve: {}/{}",
          planned.verification_key_shape.protocol, planned.verification_key_shape.curve
        ),
        format!(
          "expected setup verification-key shape rule: {}",
          planned.verification_key_shape.ic_shape_rule
        ),
      ],
    })
  }

  /// Builds the proving plan for the selected arkworks outer backend lane.
  ///
  /// # Errors
  ///
  /// Returns an error if the package does not satisfy the selected backend's
  /// target proof system or frozen outer-statement contract.
  pub fn plan_proof(
    &self,
    package: &WrapperExecutionPackage,
  ) -> Result<ArkworksGroth16ProofPlan, OuterGroth16BackendError> {
    let planned = self.prepare(package)?;

    Ok(ArkworksGroth16ProofPlan {
      proof_artifact: planned.proof_artifact,
      public_inputs_artifact: planned.public_inputs_artifact,
      public_inputs: package
        .statement
        .public_inputs
        .entries
        .iter()
        .map(|entry| entry.value.clone())
        .collect(),
      notes: vec![
        format!("selected outer backend stack: {}", self.metadata().stack),
        format!(
          "expected produced proof protocol/curve: {}/{}",
          planned.proof_shape.protocol, planned.proof_shape.curve
        ),
        "produced proof must keep snarkjs-like top-level keys pi_a, pi_b, pi_c, protocol, curve"
          .to_owned(),
      ],
    })
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
    verification_key: &ProducedOuterGroth16VerificationKeyJson,
  ) -> Result<(), OuterGroth16BackendError> {
    let planned = self.prepare(package)?;
    let expected_shape = planned.verification_key_shape;

    if verification_key.protocol != expected_shape.protocol {
      return Err(OuterGroth16BackendError::VerificationKeyProtocolMismatch {
        expected: expected_shape.protocol,
        actual: verification_key.protocol.clone(),
      });
    }

    if verification_key.curve != expected_shape.curve {
      return Err(OuterGroth16BackendError::VerificationKeyCurveMismatch {
        expected: expected_shape.curve,
        actual: verification_key.curve.clone(),
      });
    }

    if verification_key.n_public != package.statement.public_inputs.entries.len() {
      return Err(OuterGroth16BackendError::VerificationKeyPublicInputCountMismatch {
        expected: package.statement.public_inputs.entries.len(),
        actual: verification_key.n_public,
      });
    }

    if verification_key.ic.len() != package.statement.public_inputs.entries.len() + 1 {
      return Err(OuterGroth16BackendError::VerificationKeyShapeMismatch {
        expected: package.statement.public_inputs.entries.len() + 1,
        actual: verification_key.ic.len(),
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
      return Err(OuterGroth16BackendError::VerificationKeyTopLevelKeysMismatch {
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
    proof: &ProducedOuterGroth16ProofJson,
  ) -> Result<(), OuterGroth16BackendError> {
    let planned = self.prepare(package)?;
    let expected_shape = planned.proof_shape;

    if proof.protocol != expected_shape.protocol {
      return Err(OuterGroth16BackendError::ProofProtocolMismatch {
        expected: expected_shape.protocol,
        actual: proof.protocol.clone(),
      });
    }

    if proof.curve != expected_shape.curve {
      return Err(OuterGroth16BackendError::ProofCurveMismatch {
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
      return Err(OuterGroth16BackendError::ProofTopLevelKeysMismatch {
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
    proof: ProducedOuterGroth16ProofJson,
    verification_key: ProducedOuterGroth16VerificationKeyJson,
  ) -> Result<ProducedOuterGroth16ArtifactBundle, OuterGroth16BackendError> {
    let planned = self.prepare(package)?;
    self.validate_produced_proof(package, &proof)?;
    self.validate_setup_verification_key(package, &verification_key)?;

    Ok(ProducedOuterGroth16ArtifactBundle::new(
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
  ) -> Result<ArkworksGroth16OuterCircuitInput, OuterGroth16BackendError> {
    let _ = self.prepare(package)?;

    let proof_json =
      artifacts.proof_json.ok_or_else(|| OuterGroth16BackendError::MissingInnerProofPayload {
        source_artifact_id: package.witness.source_artifact_id.clone(),
      })?;
    let verification_key_json = artifacts.verification_key_json.ok_or_else(|| {
      OuterGroth16BackendError::MissingInnerVerificationKeyPayload {
        source_artifact_id: package.witness.source_artifact_id.clone(),
      }
    })?;

    let inner_proof = parse_groth16_bn254_proof(proof_json).map_err(|source| {
      OuterGroth16BackendError::MalformedInnerProof {
        source_artifact_id: package.witness.source_artifact_id.clone(),
        source,
      }
    })?;
    let inner_verification_key =
      parse_groth16_bn254_verifying_key(verification_key_json).map_err(|source| {
        OuterGroth16BackendError::MalformedInnerVerificationKey {
          source_artifact_id: package.witness.source_artifact_id.clone(),
          source,
        }
      })?;

    if inner_verification_key.ic.len() != package.witness.verification_key_ic_count {
      return Err(OuterGroth16BackendError::VerificationKeyIcCountMismatch {
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

    let outer_statement = ArkworksGroth16OuterStatementInput {
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
      return Err(OuterGroth16BackendError::UnsupportedStatementLayout {
        reason: "current arkworks outer lane only supports an outer statement that mirrors inner verifier public-input values exactly"
          .to_owned(),
      });
    }

    Ok(ArkworksGroth16OuterCircuitInput {
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
  ) -> Result<OuterWrapperCircuit, OuterGroth16BackendError> {
    let adapted = self.adapt_input(package, artifacts)?;
    let circuit = build_outer_wrapper_circuit(adapted.to_circuit_input());
    circuit.assert_ready_for_synthesis().map_err(|error| {
      OuterGroth16BackendError::OuterCircuitInputInvalid { reason: error.to_string() }
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
  ) -> Result<CanonicalOuterCircuitSetupArtifacts, OuterGroth16BackendError> {
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
  ) -> Result<CanonicalOuterCircuitProofArtifacts, OuterGroth16BackendError> {
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
  ) -> Result<R1csCircuit, OuterGroth16BackendError> {
    let adapted = self.adapt_input(package, artifacts)?;
    build_outer_wrapper_canonical_r1cs(&adapted.to_circuit_input()).map_err(|_| {
      OuterGroth16BackendError::MissingOuterCanonicalR1csLowering {
        backend: OuterGroth16Backend::backend_id(self),
        circuit_stack: "halo2/midnight outer wrapper circuit",
      }
    })
  }
}

impl CanonicalOuterCircuitProofBackend for ArkworksGroth16Bls12381Backend {
  fn backend_id(&self) -> &'static str {
    OuterGroth16Backend::backend_id(self)
  }

  fn plan_canonical_setup(
    &self,
    circuit: &OuterWrapperCircuit,
    verification_key_artifact: &str,
  ) -> Result<CanonicalOuterCircuitSetupArtifacts, OuterGroth16BackendError> {
    circuit.assert_ready_for_synthesis().map_err(|error| {
      OuterGroth16BackendError::OuterCircuitInputInvalid { reason: error.to_string() }
    })?;

    Ok(CanonicalOuterCircuitSetupArtifacts {
      build_status: match circuit.build_status() {
        CircuitBuildStatus::VerifierIntegrated => "verifier-integrated",
      },
      verification_key_artifact: verification_key_artifact.to_owned(),
      expected_n_public: circuit.input.outer_statement.public_inputs.len(),
      notes: vec![
        "canonical outer circuit is ready for synthesis".to_owned(),
        "real direct setup/prover wiring is still missing".to_owned(),
      ],
    })
  }

  fn plan_canonical_proof(
    &self,
    circuit: &OuterWrapperCircuit,
    proof_artifact: &str,
    public_inputs_artifact: &str,
    public_inputs: &[String],
  ) -> Result<CanonicalOuterCircuitProofArtifacts, OuterGroth16BackendError> {
    circuit.assert_ready_for_synthesis().map_err(|error| {
      OuterGroth16BackendError::OuterCircuitInputInvalid { reason: error.to_string() }
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
        "real direct proof/prover wiring is still missing".to_owned(),
      ],
    })
  }
}

#[cfg(test)]
mod tests {
  use serde_json::Value;
  use wrapper_circuits::groth16_fixture_raw;
  use wrapper_core::{
    NamedPublicInput, NamedPublicInputs, ProducedGroth16G1PointJson, ProducedGroth16G2PointJson,
    ProducedOuterGroth16ProofJson, ProducedOuterGroth16VerificationKeyJson, ProofSystemDescriptor,
    ProofSystemKind, WrapperExecutionPackage, WrapperJob, WrapperStatement, WrapperWitnessInput,
  };

  use crate::parse_snarkjs_groth16_bn254_bundle;

  use super::{
    ArkworksGroth16Bls12381Backend, OuterCircuitInputArtifacts, OuterGroth16Backend,
    OuterGroth16BackendError, PlannedGroth16Bls12381Backend,
  };

  fn sample_package(target_kind: ProofSystemKind) -> WrapperExecutionPackage {
    let named = NamedPublicInputs::new(vec![
      NamedPublicInput::new("a", "1"),
      NamedPublicInput::new("b", "2"),
    ]);

    WrapperExecutionPackage::new(
      WrapperJob::new(
        "job-1",
        ProofSystemDescriptor { kind: ProofSystemKind::Groth16Bn254, source: "loader".to_owned() },
        ProofSystemDescriptor { kind: target_kind, source: "planner".to_owned() },
        2,
        Some(named.clone()),
        vec![],
      ),
      WrapperStatement::new(named.clone()),
      WrapperWitnessInput::new(
        "artifact-1",
        ProofSystemDescriptor { kind: ProofSystemKind::Groth16Bn254, source: "loader".to_owned() },
        named,
        3,
        true,
        true,
        vec![],
      ),
    )
  }

  fn real_fixture_package() -> WrapperExecutionPackage {
    parse_snarkjs_groth16_bn254_bundle(
      "circom-multiplier2",
      groth16_fixture_raw::proof_json(),
      groth16_fixture_raw::public_inputs_json(),
      groth16_fixture_raw::verification_key_json(),
    )
    .expect("fixture bundle should parse")
    .build_bls12_381_execution_package()
  }

  #[test]
  fn planned_bls12_381_backend_prepares_placeholder_bundle() {
    let backend = PlannedGroth16Bls12381Backend;
    let planned = backend
      .prepare(&sample_package(ProofSystemKind::Groth16Bls12_381))
      .expect("planned backend should accept BLS12-381 target");
    let bundle = planned.bundle_template;

    assert_eq!(backend.backend_id(), "planned-groth16-bls12-381-backend");
    assert_eq!(bundle.proof_system.kind, ProofSystemKind::Groth16Bls12_381);
    assert_eq!(bundle.proof_artifact, "job-1-wrapper-proof.json");
    assert!(bundle.proof.is_none());
    assert_eq!(bundle.public_inputs, vec!["1", "2"]);
    assert_eq!(bundle.public_inputs_artifact, "job-1-wrapper-public.json");
    assert_eq!(bundle.verification_key_artifact, "job-1-wrapper-verification-key.json");
    let verification_key =
      bundle.verification_key.as_ref().expect("planned backend should materialize a VK skeleton");
    assert_eq!(verification_key.protocol, "groth16");
    assert_eq!(verification_key.curve, "bls12-381");
    assert_eq!(verification_key.n_public, 2);
    assert_eq!(verification_key.ic.len(), 3);
  }

  #[test]
  fn planned_bls12_381_backend_rejects_wrong_target() {
    let backend = PlannedGroth16Bls12381Backend;
    let error = backend
      .prepare(&sample_package(ProofSystemKind::Groth16Bn254))
      .expect_err("backend should reject non-BLS12-381 target");

    assert!(matches!(
      error,
      OuterGroth16BackendError::UnsupportedTarget {
        expected: "groth16-bls12-381",
        actual: "groth16-bn254"
      }
    ));
  }

  #[test]
  fn planned_bls12_381_backend_marks_real_phases_as_unimplemented() {
    let backend = PlannedGroth16Bls12381Backend;
    let package = sample_package(ProofSystemKind::Groth16Bls12_381);

    assert!(matches!(
      backend.setup(&package, OuterCircuitInputArtifacts::default()),
      Err(OuterGroth16BackendError::UnsupportedOperation {
        backend: "planned-groth16-bls12-381-backend",
        operation: "setup",
      })
    ));
    assert!(matches!(
      backend.prove(&package, OuterCircuitInputArtifacts::default()),
      Err(OuterGroth16BackendError::UnsupportedOperation {
        backend: "planned-groth16-bls12-381-backend",
        operation: "prove",
      })
    ));
  }

  #[test]
  fn arkworks_backend_exposes_selected_stack_metadata() {
    let backend = ArkworksGroth16Bls12381Backend;
    let metadata = backend.metadata();

    assert_eq!(metadata.backend_id, "arkworks-groth16-bls12-381-backend");
    assert_eq!(
      metadata.stack,
      "canonical R1CS -> arkworks Groth16 outer lane targeting Groth16 BLS12-381 artifacts"
    );
    assert_eq!(metadata.protocol, "groth16");
    assert_eq!(metadata.curve, "bls12-381");
    assert!(metadata.serialization_conventions.iter().any(|entry| entry.contains("pi_a")));
  }

  #[test]
  fn arkworks_backend_prepares_expected_output_with_stack_note() {
    let backend = ArkworksGroth16Bls12381Backend;
    let planned = backend
      .prepare(&sample_package(ProofSystemKind::Groth16Bls12_381))
      .expect("arkworks backend should prepare a valid package");

    assert!(planned
      .notes
      .iter()
      .any(|note| note.contains("selected outer backend stack: canonical R1CS -> arkworks Groth16 outer lane targeting Groth16 BLS12-381 artifacts")));
    assert!(planned.notes.iter().any(|note| {
      note.contains("selected real backend mode is canonical R1CS -> arkworks Groth16")
    }));
    assert!(
      planned
        .bundle_template
        .notes
        .iter()
        .any(|note| note.contains("arkworks Groth16 BLS12-381 lane over canonical R1CS"))
    );
  }

  #[test]
  fn arkworks_backend_plans_setup_verification_key_output() {
    let backend = ArkworksGroth16Bls12381Backend;
    let plan = backend
      .plan_setup(&sample_package(ProofSystemKind::Groth16Bls12_381))
      .expect("setup planning should succeed for a valid package");

    assert_eq!(plan.verification_key_artifact, "job-1-wrapper-verification-key.json");
    assert_eq!(plan.expected_n_public, 2);
    assert_eq!(plan.expected_ic_len, 3);
  }

  #[test]
  fn arkworks_backend_plans_proof_output() {
    let backend = ArkworksGroth16Bls12381Backend;
    let plan = backend
      .plan_proof(&sample_package(ProofSystemKind::Groth16Bls12_381))
      .expect("proof planning should succeed for a valid package");

    assert_eq!(plan.proof_artifact, "job-1-wrapper-proof.json");
    assert_eq!(plan.public_inputs_artifact, "job-1-wrapper-public.json");
    assert_eq!(plan.public_inputs, vec!["1".to_owned(), "2".to_owned()]);
  }

  fn sample_produced_vk(n_public: usize) -> ProducedOuterGroth16VerificationKeyJson {
    ProducedOuterGroth16VerificationKeyJson {
      protocol: "groth16".to_owned(),
      curve: "bls12-381".to_owned(),
      n_public,
      vk_alpha_1: wrapper_core::ProducedGroth16G1PointJson {
        x: "1".to_owned(),
        y: "2".to_owned(),
        z: "1".to_owned(),
      },
      vk_beta_2: wrapper_core::ProducedGroth16G2PointJson {
        x: ["1".to_owned(), "0".to_owned()],
        y: ["2".to_owned(), "0".to_owned()],
        z: ["1".to_owned(), "0".to_owned()],
      },
      vk_gamma_2: wrapper_core::ProducedGroth16G2PointJson {
        x: ["3".to_owned(), "0".to_owned()],
        y: ["4".to_owned(), "0".to_owned()],
        z: ["1".to_owned(), "0".to_owned()],
      },
      vk_delta_2: wrapper_core::ProducedGroth16G2PointJson {
        x: ["5".to_owned(), "0".to_owned()],
        y: ["6".to_owned(), "0".to_owned()],
        z: ["1".to_owned(), "0".to_owned()],
      },
      ic: (0..=n_public)
        .map(|index| wrapper_core::ProducedGroth16G1PointJson {
          x: (index + 1).to_string(),
          y: (index + 2).to_string(),
          z: "1".to_owned(),
        })
        .collect(),
    }
  }

  fn sample_produced_proof() -> ProducedOuterGroth16ProofJson {
    ProducedOuterGroth16ProofJson {
      protocol: "groth16".to_owned(),
      curve: "bls12-381".to_owned(),
      pi_a: ProducedGroth16G1PointJson { x: "1".to_owned(), y: "2".to_owned(), z: "1".to_owned() },
      pi_b: ProducedGroth16G2PointJson {
        x: ["1".to_owned(), "0".to_owned()],
        y: ["2".to_owned(), "0".to_owned()],
        z: ["1".to_owned(), "0".to_owned()],
      },
      pi_c: ProducedGroth16G1PointJson { x: "3".to_owned(), y: "4".to_owned(), z: "1".to_owned() },
    }
  }

  #[test]
  fn arkworks_backend_accepts_setup_verification_key_matching_expected_shape() {
    let backend = ArkworksGroth16Bls12381Backend;
    let package = sample_package(ProofSystemKind::Groth16Bls12_381);

    backend
      .validate_setup_verification_key(&package, &sample_produced_vk(2))
      .expect("setup VK matching the expected shape should validate");
  }

  #[test]
  fn arkworks_backend_accepts_produced_proof_matching_expected_shape() {
    let backend = ArkworksGroth16Bls12381Backend;
    let package = sample_package(ProofSystemKind::Groth16Bls12_381);

    backend
      .validate_produced_proof(&package, &sample_produced_proof())
      .expect("produced proof matching the expected shape should validate");
  }

  #[test]
  fn arkworks_backend_rejects_produced_proof_with_wrong_curve() {
    let backend = ArkworksGroth16Bls12381Backend;
    let package = sample_package(ProofSystemKind::Groth16Bls12_381);
    let mut proof = sample_produced_proof();
    proof.curve = "bn254".to_owned();

    assert!(matches!(
      backend.validate_produced_proof(&package, &proof),
      Err(OuterGroth16BackendError::ProofCurveMismatch { .. })
    ));
  }

  #[test]
  fn arkworks_backend_can_assemble_validated_produced_bundle() {
    let backend = ArkworksGroth16Bls12381Backend;
    let package = sample_package(ProofSystemKind::Groth16Bls12_381);
    let bundle = backend
      .assemble_produced_bundle(&package, sample_produced_proof(), sample_produced_vk(2))
      .expect("validated proof and VK should assemble into a produced bundle");

    assert_eq!(bundle.proof_artifact, "job-1-wrapper-proof.json");
    assert_eq!(bundle.public_inputs_artifact, "job-1-wrapper-public.json");
    assert_eq!(bundle.verification_key_artifact, "job-1-wrapper-verification-key.json");
    assert_eq!(bundle.public_inputs, vec!["1", "2"]);
  }

  #[test]
  fn arkworks_backend_reports_missing_outer_canonical_r1cs_lowering_for_prove() {
    let backend = ArkworksGroth16Bls12381Backend;
    let package = real_fixture_package();

    assert!(matches!(
      backend.prove(
        &package,
        OuterCircuitInputArtifacts::new(
          Some(groth16_fixture_raw::proof_json()),
          Some(groth16_fixture_raw::verification_key_json()),
        ),
      ),
      Err(OuterGroth16BackendError::MissingOuterCanonicalR1csLowering {
        backend: "arkworks-groth16-bls12-381-backend",
        circuit_stack: "halo2/midnight outer wrapper circuit",
      })
    ));
  }

  #[test]
  fn arkworks_backend_reports_missing_outer_canonical_r1cs_lowering_for_setup() {
    let backend = ArkworksGroth16Bls12381Backend;
    let package = real_fixture_package();

    assert!(matches!(
      backend.setup(
        &package,
        OuterCircuitInputArtifacts::new(
          Some(groth16_fixture_raw::proof_json()),
          Some(groth16_fixture_raw::verification_key_json()),
        ),
      ),
      Err(OuterGroth16BackendError::MissingOuterCanonicalR1csLowering {
        backend: "arkworks-groth16-bls12-381-backend",
        circuit_stack: "halo2/midnight outer wrapper circuit",
      })
    ));
  }

  #[test]
  fn arkworks_backend_reports_missing_outer_canonical_r1cs_lowering_for_verify() {
    let backend = ArkworksGroth16Bls12381Backend;
    let package = real_fixture_package();
    let produced = backend
      .assemble_produced_bundle(&package, sample_produced_proof(), sample_produced_vk(1))
      .expect("shape-valid produced bundle should assemble");

    assert!(matches!(
      backend.verify(
        &package,
        &produced,
        OuterCircuitInputArtifacts::new(
          Some(groth16_fixture_raw::proof_json()),
          Some(groth16_fixture_raw::verification_key_json()),
        ),
      ),
      Err(OuterGroth16BackendError::MissingOuterCanonicalR1csLowering {
        backend: "arkworks-groth16-bls12-381-backend",
        circuit_stack: "halo2/midnight outer wrapper circuit",
      })
    ));
  }

  #[test]
  fn arkworks_backend_rejects_setup_verification_key_with_wrong_protocol() {
    let backend = ArkworksGroth16Bls12381Backend;
    let package = sample_package(ProofSystemKind::Groth16Bls12_381);
    let mut vk = sample_produced_vk(2);
    vk.protocol = "not-groth16".to_owned();

    assert!(matches!(
      backend.validate_setup_verification_key(&package, &vk),
      Err(OuterGroth16BackendError::VerificationKeyProtocolMismatch { .. })
    ));
  }

  #[test]
  fn arkworks_backend_rejects_setup_verification_key_with_wrong_n_public() {
    let backend = ArkworksGroth16Bls12381Backend;
    let package = sample_package(ProofSystemKind::Groth16Bls12_381);

    assert!(matches!(
      backend.validate_setup_verification_key(&package, &sample_produced_vk(1)),
      Err(OuterGroth16BackendError::VerificationKeyPublicInputCountMismatch {
        expected: 2,
        actual: 1,
      })
    ));
  }

  #[test]
  fn arkworks_backend_rejects_setup_verification_key_with_wrong_ic_length() {
    let backend = ArkworksGroth16Bls12381Backend;
    let package = sample_package(ProofSystemKind::Groth16Bls12_381);
    let mut vk = sample_produced_vk(2);
    let _ = vk.ic.pop();

    assert!(matches!(
      backend.validate_setup_verification_key(&package, &vk),
      Err(OuterGroth16BackendError::VerificationKeyShapeMismatch { expected: 3, actual: 2 })
    ));
  }

  #[test]
  fn arkworks_backend_adapts_real_fixture_inputs() {
    let backend = ArkworksGroth16Bls12381Backend;
    let package = real_fixture_package();
    let adapted = backend
      .adapt_input(
        &package,
        OuterCircuitInputArtifacts::new(
          Some(groth16_fixture_raw::proof_json()),
          Some(groth16_fixture_raw::verification_key_json()),
        ),
      )
      .expect("arkworks adapter should normalize the real fixture package");

    assert_eq!(adapted.source_artifact_id, "circom-multiplier2");
    assert_eq!(adapted.inner_verifier_public_inputs, adapted.outer_statement.public_inputs);
    assert_eq!(adapted.outer_statement.field_names, vec!["public_input_0".to_owned()]);
    assert_eq!(adapted.inner_verification_key.ic.len(), 2);
  }

  #[test]
  fn arkworks_adapted_input_can_map_to_circuit_owned_input() {
    let backend = ArkworksGroth16Bls12381Backend;
    let package = real_fixture_package();
    let adapted = backend
      .adapt_input(
        &package,
        OuterCircuitInputArtifacts::new(
          Some(groth16_fixture_raw::proof_json()),
          Some(groth16_fixture_raw::verification_key_json()),
        ),
      )
      .expect("arkworks adapter should normalize the real fixture package");
    let circuit_input = adapted.to_circuit_input();

    assert_eq!(circuit_input.inner_public_inputs, adapted.inner_verifier_public_inputs);
    assert_eq!(circuit_input.outer_statement.public_inputs, adapted.outer_statement.public_inputs);
  }

  #[test]
  fn arkworks_backend_can_build_ready_outer_circuit() {
    let backend = ArkworksGroth16Bls12381Backend;
    let package = real_fixture_package();
    let circuit = backend
      .build_outer_circuit(
        &package,
        OuterCircuitInputArtifacts::new(
          Some(groth16_fixture_raw::proof_json()),
          Some(groth16_fixture_raw::verification_key_json()),
        ),
      )
      .expect("backend should build a ready outer circuit from the real fixture");

    circuit
      .assert_ready_for_synthesis()
      .expect("outer circuit built by backend should be ready for synthesis");
  }

  #[test]
  fn arkworks_backend_can_plan_direct_outer_circuit_setup_surface() {
    let backend = ArkworksGroth16Bls12381Backend;
    let package = real_fixture_package();
    let plan = backend
      .plan_direct_outer_circuit_setup(
        &package,
        OuterCircuitInputArtifacts::new(
          Some(groth16_fixture_raw::proof_json()),
          Some(groth16_fixture_raw::verification_key_json()),
        ),
      )
      .expect("direct outer-circuit setup surface should plan for a ready circuit");

    assert_eq!(plan.build_status, "verifier-integrated");
    assert_eq!(plan.verification_key_artifact, "circom-multiplier2-wrapper-verification-key.json");
    assert_eq!(plan.expected_n_public, 1);
  }

  #[test]
  fn arkworks_backend_can_plan_direct_outer_circuit_proof_surface() {
    let backend = ArkworksGroth16Bls12381Backend;
    let package = real_fixture_package();
    let plan = backend
      .plan_direct_outer_circuit_proof(
        &package,
        OuterCircuitInputArtifacts::new(
          Some(groth16_fixture_raw::proof_json()),
          Some(groth16_fixture_raw::verification_key_json()),
        ),
      )
      .expect("direct outer-circuit proving surface should plan for a ready circuit");

    assert_eq!(plan.build_status, "verifier-integrated");
    assert_eq!(plan.proof_artifact, "circom-multiplier2-wrapper-proof.json");
    assert_eq!(plan.public_inputs_artifact, "circom-multiplier2-wrapper-public.json");
    assert_eq!(
      plan.public_inputs,
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
  fn arkworks_adapter_rejects_missing_inner_proof_payload() {
    let backend = ArkworksGroth16Bls12381Backend;
    let package = real_fixture_package();

    assert!(matches!(
      backend.adapt_input(
        &package,
        OuterCircuitInputArtifacts::new(None, Some(groth16_fixture_raw::verification_key_json())),
      ),
      Err(OuterGroth16BackendError::MissingInnerProofPayload { source_artifact_id })
        if source_artifact_id == "circom-multiplier2"
    ));
  }

  #[test]
  fn arkworks_adapter_rejects_malformed_inner_verification_key() {
    let backend = ArkworksGroth16Bls12381Backend;
    let package = real_fixture_package();

    assert!(matches!(
      backend.adapt_input(
        &package,
        OuterCircuitInputArtifacts::new(Some(groth16_fixture_raw::proof_json()), Some(br"{}")),
      ),
      Err(OuterGroth16BackendError::MalformedInnerVerificationKey { .. })
    ));
  }

  #[test]
  fn arkworks_adapter_rejects_verification_key_arity_mismatch_against_package() {
    let backend = ArkworksGroth16Bls12381Backend;
    let package = real_fixture_package();
    let mut vk_json: Value = serde_json::from_slice(groth16_fixture_raw::verification_key_json())
      .expect("fixture VK JSON should parse as a serde_json::Value");
    let ic = vk_json["IC"].as_array_mut().expect("fixture VK should carry an IC array");
    ic.push(ic[0].clone());
    vk_json["nPublic"] = Value::from(2_u64);
    let malformed_vk = serde_json::to_vec(&vk_json).expect("modified VK should serialize");

    assert!(matches!(
      backend.adapt_input(
        &package,
        OuterCircuitInputArtifacts::new(
          Some(groth16_fixture_raw::proof_json()),
          Some(malformed_vk.as_slice()),
        ),
      ),
      Err(OuterGroth16BackendError::VerificationKeyIcCountMismatch { expected: 2, actual: 3 })
    ));
  }

  #[test]
  fn arkworks_adapter_rejects_non_mirrored_outer_statement_values() {
    let backend = ArkworksGroth16Bls12381Backend;
    let mut package = real_fixture_package();
    package.statement.public_inputs.entries[0].value = "999".to_owned();

    assert!(matches!(
      backend.adapt_input(
        &package,
        OuterCircuitInputArtifacts::new(
          Some(groth16_fixture_raw::proof_json()),
          Some(groth16_fixture_raw::verification_key_json()),
        ),
      ),
      Err(OuterGroth16BackendError::UnsupportedStatementLayout { .. })
    ));
  }
}
