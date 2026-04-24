use wrapper_circuits::{
  CircuitBuildStatus, OuterWrapperCircuit, R1csCircuit, build_outer_wrapper_canonical_r1cs,
};
use wrapper_core::WrapperExecutionPackage;

use crate::outer::{
  CanonicalOuterCircuitProofArtifacts, CanonicalOuterCircuitProofBackend,
  CanonicalOuterCircuitSetupArtifacts, DirectOuterProofPlan, DirectOuterSetupPlan,
  OuterCircuitInputArtifacts, OuterProofBackend, OuterProofBackendError,
};

use super::MidnightDirectOuterBackend;

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
      expected_pcs: self.capabilities().pcs.to_owned(),
      notes: vec![
        format!("selected outer backend stack: {}", self.metadata().stack),
        format!(
          "expected setup verification-key protocol/curve: {}/{}",
          self.capabilities().protocol,
          self.capabilities().host_curve
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
      expected_transcript: self.capabilities().transcript.to_owned(),
      notes: vec![
        format!("selected outer backend stack: {}", self.metadata().stack),
        format!(
          "expected produced proof protocol/curve: {}/{}",
          self.capabilities().protocol,
          self.capabilities().host_curve
        ),
        format!("produced proof must keep top-level keys {:?}", planned.proof_shape.top_level_keys),
      ],
    })
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
