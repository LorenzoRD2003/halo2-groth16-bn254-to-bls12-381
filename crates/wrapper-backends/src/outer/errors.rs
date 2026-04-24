use thiserror::Error;
use wrapper_core::OuterStatementContractError;

use crate::snarkjs::SnarkjsGroth16ParseError;

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
