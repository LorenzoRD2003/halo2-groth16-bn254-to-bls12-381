use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner},
  plonk::{Circuit, ConstraintSystem, Error},
};

use crate::bn254::{Bls12HostField, NativeField};

use super::{
  Bn254InnerVerifierConfig, MidnightBls12_381HostConfigShell, MidnightBn254HostConfig,
  OuterWrapperCircuit, host::lift_outer_inputs_to_host,
};

/// Configuration for the BN254-hosted proving wrapper.
#[derive(Clone, Debug)]
pub struct OuterWrapperHostConfigBn254 {
  host: MidnightBn254HostConfig,
  semantics: Bn254InnerVerifierConfig<NativeField>,
}

/// Configuration for the BLS12-381-hosted proving wrapper.
#[derive(Clone, Debug)]
pub struct OuterWrapperHostConfigBls12 {
  host: MidnightBls12_381HostConfigShell,
  semantics: Bn254InnerVerifierConfig<Bls12HostField>,
}

/// BN254-hosted proving wrapper for the canonical outer semantic circuit.
#[derive(Clone, Debug)]
pub struct HostedOuterWrapperCircuitBn254 {
  semantic: OuterWrapperCircuit,
}

impl HostedOuterWrapperCircuitBn254 {
  /// Wraps one semantic outer circuit for the current BN254-hosted proving lane.
  #[must_use]
  pub fn new(semantic: OuterWrapperCircuit) -> Self {
    Self { semantic }
  }

  /// Returns the host-independent semantic circuit.
  #[must_use]
  pub fn semantic(&self) -> &OuterWrapperCircuit {
    &self.semantic
  }

  /// Returns the semantic circuit build status.
  #[must_use]
  pub fn build_status(&self) -> super::CircuitBuildStatus {
    self.semantic.build_status()
  }

  /// Validates that this BN254-hosted wrapper can be synthesized.
  pub fn assert_ready_for_synthesis(&self) -> Result<(), wrapper_core::WrapperError> {
    self.semantic.assert_ready_for_synthesis()
  }
}

impl Circuit<NativeField> for HostedOuterWrapperCircuitBn254 {
  type Config = OuterWrapperHostConfigBn254;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self { semantic: self.semantic.without_witnesses() }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    let (host, instance_columns) = MidnightBn254HostConfig::configure(meta);
    OuterWrapperHostConfigBn254 {
      host,
      semantics: Bn254InnerVerifierConfig::configure(meta, &instance_columns),
    }
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    self.assert_ready_for_synthesis().map_err(|error| Error::Synthesis(error.to_string()))?;

    config.semantics.synthesize(&mut layouter, &self.semantic.input)?;
    config
      .host
      .expose_outer_statement(&mut layouter, &self.semantic.input.outer_statement.public_inputs)
  }
}

/// BLS12-381-hosted proving wrapper for the canonical outer semantic circuit.
#[derive(Clone, Debug)]
pub struct HostedOuterWrapperCircuitBls12 {
  semantic: OuterWrapperCircuit,
}

impl HostedOuterWrapperCircuitBls12 {
  /// Wraps one semantic outer circuit for the BLS12-381-hosted proving lane.
  #[must_use]
  pub fn new(semantic: OuterWrapperCircuit) -> Self {
    Self { semantic }
  }

  /// Returns the host-independent semantic circuit.
  #[must_use]
  pub fn semantic(&self) -> &OuterWrapperCircuit {
    &self.semantic
  }

  /// Returns the semantic circuit build status.
  #[must_use]
  pub fn build_status(&self) -> super::CircuitBuildStatus {
    self.semantic.build_status()
  }

  /// Validates that this BLS12-381-hosted wrapper can be synthesized.
  pub fn assert_ready_for_synthesis(&self) -> Result<(), wrapper_core::WrapperError> {
    self.semantic.assert_ready_for_synthesis()
  }
}

impl Circuit<Bls12HostField> for HostedOuterWrapperCircuitBls12 {
  type Config = OuterWrapperHostConfigBls12;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self { semantic: self.semantic.without_witnesses() }
  }

  fn configure(meta: &mut ConstraintSystem<Bls12HostField>) -> Self::Config {
    let (host, instance_columns) = MidnightBls12_381HostConfigShell::configure(meta);
    OuterWrapperHostConfigBls12 {
      host,
      semantics: Bn254InnerVerifierConfig::configure(meta, &instance_columns),
    }
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl Layouter<Bls12HostField>,
  ) -> Result<(), Error> {
    self.assert_ready_for_synthesis().map_err(|error| Error::Synthesis(error.to_string()))?;

    config.semantics.synthesize(&mut layouter, &self.semantic.input)?;
    let public_inputs = lift_outer_inputs_to_host::<Bls12HostField>(
      &self.semantic.input.outer_statement.public_inputs,
    );
    config.host.expose_outer_statement(&mut layouter, &public_inputs)
  }
}

/// Backward-compatible alias for the current BN254-hosted proving wrapper.
pub type HostedOuterWrapperCircuit = HostedOuterWrapperCircuitBn254;
/// Backward-compatible alias for the current BN254-hosted wrapper config.
pub type OuterWrapperHostConfig = OuterWrapperHostConfigBn254;
