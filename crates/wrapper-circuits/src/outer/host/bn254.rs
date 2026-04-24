use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, Value},
  plonk::{Advice, Column, ConstraintSystem, Error, Instance},
};

use crate::NativeField;

use super::{OuterHostFlavor, OuterHostLane};

/// Current Halo2/Midnight outer host lane over BN254.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MidnightBn254HostLane;

impl OuterHostLane for MidnightBn254HostLane {
  type Field = NativeField;

  fn flavor() -> OuterHostFlavor {
    OuterHostFlavor::MidnightBn254
  }

  fn protocol() -> &'static str {
    Self::flavor().protocol()
  }

  fn curve() -> &'static str {
    Self::flavor().curve()
  }

  fn pcs() -> &'static str {
    Self::flavor().pcs()
  }

  fn transcript() -> &'static str {
    Self::flavor().transcript()
  }

  fn supports_current_canonical_circuit() -> bool {
    true
  }
}

/// Host-specific public-input exposure config for the current BN254-hosted
/// outer circuit lane.
#[derive(Clone, Debug)]
pub struct MidnightBn254HostConfig {
  outer_statement_advice: Column<Advice>,
  outer_statement_instance: Column<Instance>,
}

impl MidnightBn254HostConfig {
  /// Configures host-lane-specific columns used by the canonical outer circuit.
  #[must_use]
  pub fn configure(meta: &mut ConstraintSystem<NativeField>) -> (Self, [Column<Instance>; 2]) {
    let outer_statement_instance = meta.instance_column();
    let shared_instance = meta.instance_column();
    meta.enable_equality(outer_statement_instance);
    meta.enable_equality(shared_instance);

    let outer_statement_advice = meta.advice_column();
    meta.enable_equality(outer_statement_advice);

    (
      Self { outer_statement_advice, outer_statement_instance },
      [outer_statement_instance, shared_instance],
    )
  }

  /// Exposes one ordered outer statement on the host lane's public-input columns.
  ///
  /// # Errors
  ///
  /// Returns any assignment or instance-constraint failure raised by the host
  /// proving system.
  pub fn expose_outer_statement(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    public_inputs: &[NativeField],
  ) -> Result<(), Error> {
    let cells = layouter.assign_region(
      || "expose outer statement public inputs",
      |mut region| {
        let mut cells = Vec::with_capacity(public_inputs.len());
        for (row, value) in public_inputs.iter().enumerate() {
          let cell = region.assign_advice(
            || format!("outer statement value {row}"),
            self.outer_statement_advice,
            row,
            || Value::known(*value),
          )?;
          cells.push(cell.cell());
        }

        Ok(cells)
      },
    )?;

    for (row, cell) in cells.into_iter().enumerate() {
      layouter.constrain_instance(cell, self.outer_statement_instance, row)?;
    }

    Ok(())
  }
}
