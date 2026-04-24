use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, Value},
  plonk::{Advice, Column, ConstraintSystem, Error, Instance},
};

use crate::bn254::Bls12HostField;

use super::{OuterHostFlavor, OuterHostLane};

/// Halo2/Midnight outer host lane over BLS12-381.
///
/// This is an additive sibling to the BN254-hosted lane. The inner verifier
/// semantics remain BN254, while the outer Halo2 proof is hosted on BLS12-381.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MidnightBls12_381HostLane;

impl OuterHostLane for MidnightBls12_381HostLane {
  type Field = Bls12HostField;

  fn flavor() -> OuterHostFlavor {
    OuterHostFlavor::MidnightBls12_381
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

/// Host-specific public-input exposure config for the BLS12-381-hosted outer lane.
#[derive(Clone, Debug)]
pub struct MidnightBls12_381HostConfigShell {
  outer_statement_advice: Column<Advice>,
  outer_statement_instance: Column<Instance>,
}

impl MidnightBls12_381HostConfigShell {
  /// Configures host-lane-specific columns used by the canonical outer circuit.
  #[must_use]
  pub fn configure(meta: &mut ConstraintSystem<Bls12HostField>) -> (Self, [Column<Instance>; 2]) {
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
  pub fn expose_outer_statement(
    &self,
    layouter: &mut impl Layouter<Bls12HostField>,
    public_inputs: &[Bls12HostField],
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

  /// Returns the host flavor represented by this config shell.
  #[must_use]
  pub const fn flavor(self) -> OuterHostFlavor {
    OuterHostFlavor::MidnightBls12_381
  }
}
