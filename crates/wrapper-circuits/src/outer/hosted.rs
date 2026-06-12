use std::cmp::max;

use midnight_circuits::{
  field::{
    NativeChip, NativeConfig,
    native::{NB_ARITH_COLS, NB_ARITH_FIXED_COLS},
  },
  hash::poseidon::{
    NB_POSEIDON_ADVICE_COLS, NB_POSEIDON_FIXED_COLS, PoseidonChip, PoseidonConfig,
  },
  instructions::AssertionInstructions,
  types::ComposableChip,
};
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner},
  plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Fixed},
};

use crate::bn254::{Bls12HostField, NativeField};
use crate::{
  OuterVerificationKeyCommitmentValue, assign_and_commit_verification_key_on_bls12_host,
};

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
  poseidon_native: NativeConfig,
  poseidon_hash: PoseidonConfig<Bls12HostField>,
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
    let nb_advice_cols = max(NB_POSEIDON_ADVICE_COLS, NB_ARITH_COLS);
    let nb_fixed_cols = max(NB_POSEIDON_FIXED_COLS, NB_ARITH_FIXED_COLS);
    let advice_cols = (0..nb_advice_cols).map(|_| meta.advice_column()).collect::<Vec<Column<Advice>>>();
    let fixed_cols = (0..nb_fixed_cols).map(|_| meta.fixed_column()).collect::<Vec<Column<Fixed>>>();
    let poseidon_native = NativeChip::configure(
      meta,
      &(
        advice_cols[..NB_ARITH_COLS].try_into().expect("native advice width should match"),
        fixed_cols[..NB_ARITH_FIXED_COLS].try_into().expect("native fixed width should match"),
        instance_columns,
      ),
    );
    let poseidon_hash = PoseidonChip::configure(
      meta,
      &(
        advice_cols[..NB_POSEIDON_ADVICE_COLS]
          .try_into()
          .expect("poseidon advice width should match"),
        fixed_cols[..NB_POSEIDON_FIXED_COLS]
          .try_into()
          .expect("poseidon fixed width should match"),
      ),
    );
    OuterWrapperHostConfigBls12 {
      host,
      semantics: Bn254InnerVerifierConfig::configure(meta, &instance_columns),
      poseidon_native,
      poseidon_hash,
    }
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl Layouter<Bls12HostField>,
  ) -> Result<(), Error> {
    self.assert_ready_for_synthesis().map_err(|error| Error::Synthesis(error.to_string()))?;

    let native_chip = NativeChip::new(&config.poseidon_native, &());
    let poseidon_chip = PoseidonChip::new(&config.poseidon_hash, &native_chip);
    if let OuterVerificationKeyCommitmentValue::Bls12(expected_commitment) =
      self.semantic.input.outer_statement.vk_commitment.value
    {
      let field_chip = crate::bn254::Bn254FieldChip::new(config.semantics.field_config());
      let computed_commitment = assign_and_commit_verification_key_on_bls12_host(
        &field_chip,
        &native_chip,
        &poseidon_chip,
        &mut layouter,
        &self.semantic.input.inner_verification_key,
      )?;
      native_chip.assert_equal_to_fixed(&mut layouter, &computed_commitment, expected_commitment)?;
    }

    config.semantics.synthesize(&mut layouter, &self.semantic.input)?;
    let public_inputs = lift_outer_inputs_to_host::<Bls12HostField>(
      &self.semantic.input.outer_statement.public_inputs,
    );
    config.host.expose_outer_statement(&mut layouter, &public_inputs)?;
    native_chip.load(&mut layouter)
  }
}

/// Backward-compatible alias for the current BN254-hosted proving wrapper.
pub type HostedOuterWrapperCircuit = HostedOuterWrapperCircuitBn254;
/// Backward-compatible alias for the current BN254-hosted wrapper config.
pub type OuterWrapperHostConfig = OuterWrapperHostConfigBn254;
