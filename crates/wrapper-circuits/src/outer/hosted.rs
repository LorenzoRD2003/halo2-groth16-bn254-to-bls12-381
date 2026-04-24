use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner},
  plonk::{Circuit, ConstraintSystem, Error},
};

use super::{Bn254InnerVerifierConfig, OuterHostConfig, OuterHostField, OuterWrapperCircuit};

/// Configuration for the host-lane-specific proving wrapper.
#[derive(Clone, Debug)]
pub struct OuterWrapperHostConfig {
  host: OuterHostConfig,
  semantics: Bn254InnerVerifierConfig,
}

/// Host-lane-specific proving wrapper for the canonical outer semantic circuit.
#[derive(Clone, Debug)]
pub struct HostedOuterWrapperCircuit {
  semantic: OuterWrapperCircuit,
}

impl HostedOuterWrapperCircuit {
  /// Builds a host-lane-specific proving wrapper around the canonical semantic circuit.
  #[must_use]
  pub fn new(semantic: OuterWrapperCircuit) -> Self {
    Self { semantic }
  }

  /// Returns the wrapped semantic circuit.
  #[must_use]
  pub fn semantic(&self) -> &OuterWrapperCircuit {
    &self.semantic
  }

  /// Returns the current build status reported by the canonical semantic circuit.
  #[must_use]
  pub fn build_status(&self) -> super::CircuitBuildStatus {
    self.semantic.build_status()
  }

  /// Validates that the wrapped semantic circuit is ready for synthesis.
  ///
  /// # Errors
  ///
  /// Returns any semantic validation error raised by the canonical circuit.
  pub fn assert_ready_for_synthesis(&self) -> Result<(), wrapper_core::WrapperError> {
    self.semantic.assert_ready_for_synthesis()
  }
}

impl Circuit<OuterHostField> for HostedOuterWrapperCircuit {
  type Config = OuterWrapperHostConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self { semantic: self.semantic.without_witnesses() }
  }

  fn configure(meta: &mut ConstraintSystem<OuterHostField>) -> Self::Config {
    let (host, instance_columns) = OuterHostConfig::configure(meta);
    OuterWrapperHostConfig {
      host,
      semantics: Bn254InnerVerifierConfig::configure(meta, &instance_columns),
    }
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl Layouter<OuterHostField>,
  ) -> Result<(), Error> {
    self.assert_ready_for_synthesis().map_err(|error| Error::Synthesis(error.to_string()))?;

    config.semantics.synthesize(&mut layouter, &self.semantic.input)?;
    config
      .host
      .expose_outer_statement(&mut layouter, &self.semantic.input.outer_statement.public_inputs)
  }
}
