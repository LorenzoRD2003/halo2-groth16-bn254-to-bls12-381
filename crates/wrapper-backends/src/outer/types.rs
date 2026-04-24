use wrapper_circuits::{
  Groth16Bn254Proof, Groth16Bn254VerifyingKey, InnerVerifierFlavor,
  OuterArtifactSerializationFlavor, OuterHostField, OuterHostFlavor, OuterStatementInput,
  OuterStatementSemantics, OuterWrapperCircuitInput,
};
use wrapper_core::{ProducedOuterProofJson, ProducedOuterVerificationKeyJson};

/// Stable capability summary for one selected outer backend lane.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OuterBackendCapabilities {
  /// Proof-system protocol used by the host lane.
  pub protocol: &'static str,
  /// Host curve used by the outer proof system.
  pub host_curve: &'static str,
  /// Polynomial-commitment scheme used by the host lane.
  pub pcs: &'static str,
  /// Transcript used by the host lane.
  pub transcript: &'static str,
  /// Artifact serialization family emitted by this backend.
  pub serialization: OuterArtifactSerializationFlavor,
  /// Whether the backend supports setup in the current repository phase.
  pub supports_setup: bool,
  /// Whether the backend supports proving in the current repository phase.
  pub supports_prove: bool,
  /// Whether the backend supports verification in the current repository phase.
  pub supports_verify: bool,
}

/// Typed serializer profile for one produced outer proof payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OuterProofSerialization {
  /// Proof-system protocol label.
  pub protocol: &'static str,
  /// Commitment-curve label.
  pub curve: &'static str,
  /// Stable backend identifier.
  pub backend: &'static str,
  /// Transcript label.
  pub transcript: &'static str,
  /// Payload encoding label.
  pub encoding: &'static str,
}

impl OuterProofSerialization {
  /// Builds one produced proof JSON payload from an already encoded proof body.
  #[must_use]
  pub fn materialize(self, proof: String) -> ProducedOuterProofJson {
    ProducedOuterProofJson {
      protocol: self.protocol.to_owned(),
      curve: self.curve.to_owned(),
      backend: self.backend.to_owned(),
      transcript: self.transcript.to_owned(),
      encoding: self.encoding.to_owned(),
      proof,
    }
  }
}

/// Typed serializer profile for one produced outer verification-key payload.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OuterVerificationKeySerialization {
  /// Proof-system protocol label.
  pub protocol: &'static str,
  /// Commitment-curve label.
  pub curve: &'static str,
  /// Stable backend identifier.
  pub backend: &'static str,
  /// Polynomial-commitment scheme label.
  pub pcs: &'static str,
  /// Payload encoding label.
  pub encoding: &'static str,
}

impl OuterVerificationKeySerialization {
  /// Builds one produced verification-key JSON payload from already encoded VK
  /// and verifier-param bodies.
  #[must_use]
  pub fn materialize(
    self,
    circuit_k: u32,
    public_input_count: usize,
    verification_key: String,
    verifier_params: String,
  ) -> ProducedOuterVerificationKeyJson {
    ProducedOuterVerificationKeyJson {
      protocol: self.protocol.to_owned(),
      curve: self.curve.to_owned(),
      backend: self.backend.to_owned(),
      pcs: self.pcs.to_owned(),
      encoding: self.encoding.to_owned(),
      circuit_k,
      public_input_count,
      verification_key,
      verifier_params,
    }
  }
}

/// Static metadata describing one selected outer Groth16 backend stack.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OuterProofBackendMetadata {
  /// Stable backend identifier.
  pub backend_id: &'static str,
  /// Inner verifier semantics consumed by the canonical outer circuit.
  pub inner_verifier: InnerVerifierFlavor,
  /// Outer host lane selected by this backend.
  pub outer_host: OuterHostFlavor,
  /// Outer artifact serialization contract emitted by this backend.
  pub serialization: OuterArtifactSerializationFlavor,
  /// Human-readable stack family.
  pub stack: &'static str,
  /// Protocol label expected from the backend.
  pub protocol: &'static str,
  /// Curve label expected from the backend.
  pub curve: &'static str,
  /// Polynomial-commitment scheme used by the backend host lane.
  pub pcs: &'static str,
  /// Transcript family used by the backend host lane.
  pub transcript: &'static str,
  /// Whether the backend supports setup in the current repository phase.
  pub supports_setup: bool,
  /// Whether the backend supports proving in the current repository phase.
  pub supports_prove: bool,
  /// Whether the backend supports verification in the current repository phase.
  pub supports_verify: bool,
  /// Setup assumptions for this backend choice.
  pub setup_assumptions: &'static [&'static str],
  /// Serialization conventions expected from this backend choice.
  pub serialization_conventions: &'static [&'static str],
  /// Compatibility notes for future implementors.
  pub compatibility_notes: &'static [&'static str],
}

impl OuterProofBackendMetadata {
  /// Returns the stable capability summary for this backend.
  #[must_use]
  pub const fn capabilities(&self) -> OuterBackendCapabilities {
    OuterBackendCapabilities {
      protocol: self.protocol,
      host_curve: self.curve,
      pcs: self.pcs,
      transcript: self.transcript,
      serialization: self.serialization,
      supports_setup: self.supports_setup,
      supports_prove: self.supports_prove,
      supports_verify: self.supports_verify,
    }
  }

  /// Returns the typed proof-serialization helper for this backend.
  #[must_use]
  pub const fn proof_serialization(&self) -> OuterProofSerialization {
    OuterProofSerialization {
      protocol: self.protocol,
      curve: self.curve,
      backend: self.backend_id,
      transcript: self.transcript,
      encoding: self.serialization.payload_encoding(),
    }
  }

  /// Returns the typed verification-key serialization helper for this backend.
  #[must_use]
  pub const fn verification_key_serialization(&self) -> OuterVerificationKeySerialization {
    OuterVerificationKeySerialization {
      protocol: self.protocol,
      curve: self.curve,
      backend: self.backend_id,
      pcs: self.pcs,
      encoding: self.serialization.payload_encoding(),
    }
  }
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
  pub public_inputs: Vec<OuterHostField>,
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
  pub inner_verifier_public_inputs: Vec<OuterHostField>,
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
