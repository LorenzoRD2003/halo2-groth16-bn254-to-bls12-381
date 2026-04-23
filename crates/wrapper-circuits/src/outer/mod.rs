//! Outer wrapper circuit built on top of the landed narrow BN254 verifier.

mod builder;
mod input;
mod statement;

use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance},
};
use wrapper_core::{LayoutDescriptor, ProjectConfig, WrapperError};

use crate::{
  Bn254BoolChip, Bn254BoolConfig, NativeField, groth16_verify,
  bn254::{Bn254FieldChip, Bn254FieldConfig, Bn254G1Chip, Bn254G1Config},
};

pub use builder::build_outer_wrapper_circuit;
pub use input::OuterWrapperCircuitInput;
pub use statement::{OuterStatementInput, OuterStatementSemantics};

/// Build status for the outer circuit shell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CircuitBuildStatus {
  /// The outer wrapper circuit is synthesized from the landed narrow verifier.
  VerifierIntegrated,
}

/// Configuration for the outer wrapper circuit.
#[derive(Clone, Debug)]
pub struct OuterWrapperConfig {
  field: Bn254FieldConfig,
  bools: Bn254BoolConfig,
  g1: Bn254G1Config,
  outer_statement_advice: Column<Advice>,
  outer_statement_instance: Column<Instance>,
}

/// Outer wrapper circuit definition backed by the narrow Groth16 BN254 verifier.
#[derive(Clone, Debug)]
pub struct OuterWrapperCircuit {
  /// Config used to describe the intended circuit.
  pub config: ProjectConfig,
  /// Canonical outer-circuit input.
  pub input: OuterWrapperCircuitInput,
}

impl OuterWrapperCircuit {
  /// Creates a new outer wrapper circuit from explicit input plus config.
  #[must_use]
  pub fn new(input: OuterWrapperCircuitInput, config: ProjectConfig) -> Self {
    Self { config, input }
  }

  /// Creates a new outer wrapper circuit using the default project config.
  #[must_use]
  pub fn from_input(input: OuterWrapperCircuitInput) -> Self {
    Self::new(input, ProjectConfig::default())
  }

  /// Returns the current build status.
  #[must_use]
  pub fn build_status(&self) -> CircuitBuildStatus {
    CircuitBuildStatus::VerifierIntegrated
  }

  /// Returns the scaffold layout for reporting purposes.
  #[must_use]
  pub fn layout_descriptor(&self) -> LayoutDescriptor {
    LayoutDescriptor::scaffold()
  }

  /// Validates that the outer circuit input is ready for real synthesis.
  ///
  /// # Errors
  ///
  /// Returns an error if the outer statement no longer mirrors the inner
  /// verifier public inputs or the inner verification key arity is inconsistent.
  pub fn assert_ready_for_synthesis(&self) -> Result<(), WrapperError> {
    self.input.validate()
  }

  fn expose_outer_statement(
    &self,
    config: &OuterWrapperConfig,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    let cells = layouter.assign_region(
      || "expose outer statement public inputs",
      |mut region| {
        let mut cells = Vec::with_capacity(self.input.outer_statement.public_inputs.len());
        for (row, value) in self.input.outer_statement.public_inputs.iter().enumerate() {
          let cell = region.assign_advice(
            || format!("outer statement value {row}"),
            config.outer_statement_advice,
            row,
            || Value::known(*value),
          )?;
          cells.push(cell.cell());
        }

        Ok(cells)
      },
    )?;

    for (row, cell) in cells.into_iter().enumerate() {
      layouter.constrain_instance(cell, config.outer_statement_instance, row)?;
    }

    Ok(())
  }
}

impl Circuit<NativeField> for OuterWrapperCircuit {
  type Config = OuterWrapperConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      config: self.config.clone(),
      input: self.input.without_witnesses(),
    }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    let outer_statement_instance = meta.instance_column();
    let shared_instance = meta.instance_column();
    meta.enable_equality(outer_statement_instance);
    meta.enable_equality(shared_instance);

    let instance_columns = [outer_statement_instance, shared_instance];
    let outer_statement_advice = meta.advice_column();
    meta.enable_equality(outer_statement_advice);

    OuterWrapperConfig {
      field: Bn254FieldConfig::configure_with_instances(meta, &instance_columns),
      bools: Bn254BoolConfig::configure_with_instances(meta, &instance_columns),
      g1: Bn254G1Config::configure_with_instances(meta, &instance_columns),
      outer_statement_advice,
      outer_statement_instance,
    }
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    self
      .assert_ready_for_synthesis()
      .map_err(|error| Error::Synthesis(error.to_string()))?;

    let field_chip = Bn254FieldChip::new(&config.field);
    let bool_chip = Bn254BoolChip::new(&config.bools);
    let g1_chip = Bn254G1Chip::new(&config.g1);

    let result = groth16_verify(
      &field_chip,
      &bool_chip,
      &g1_chip,
      &mut layouter,
      &self.input.inner_verification_key,
      &self.input.inner_proof,
      &self.input.inner_public_inputs,
    )
    .map_err(|error| match error {
      crate::Groth16VerifierError::Circuit(inner) => inner,
      _ => Error::Synthesis(error.to_string()),
    })?;

    bool_chip.assert_equal_to_fixed(&mut layouter, &result, true)?;
    self.expose_outer_statement(&config, &mut layouter)?;
    field_chip.load(&mut layouter)?;
    bool_chip.load(&mut layouter)?;
    g1_chip.load(&mut layouter)
  }
}

#[cfg(test)]
mod tests;
