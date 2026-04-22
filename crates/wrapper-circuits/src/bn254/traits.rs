use ff::Field;
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, Value},
  plonk::Error,
};

use super::{AssignedFp, Bn254FieldChip, Bn254FieldConfig, ForeignField, NativeField};

/// Shared arithmetic surface for assigned BN254 field-like values used in the primitive tower.
pub trait AssignedFieldExt: Clone {
  /// Fixed constant representation used by `assert_equal_to_fixed`.
  type Fixed: Clone;

  /// Assigns the additive identity.
  fn zero(chip: &Bn254FieldChip, layouter: &mut impl Layouter<NativeField>) -> Result<Self, Error>;

  /// Assigns the multiplicative identity.
  fn one(chip: &Bn254FieldChip, layouter: &mut impl Layouter<NativeField>) -> Result<Self, Error>;

  /// Adds two values inside the circuit.
  fn add(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<Self, Error>;

  /// Subtracts two values inside the circuit.
  fn sub(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<Self, Error>;

  /// Negates a value inside the circuit.
  fn neg(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error>;

  /// Asserts equality against another assigned value.
  fn assert_equal(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<(), Error>;

  /// Asserts equality against a fixed constant.
  fn assert_equal_to_fixed(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    expected: Self::Fixed,
  ) -> Result<(), Error>;
}

/// Shared witness-assignment surface for small unary/binary sanity circuits.
pub(crate) trait AssignedCircuitValue: AssignedFieldExt {
  /// Witness shape used by the circuit wrapper.
  type Witness: Clone;

  /// Assigns the value from its circuit witness shape.
  fn assign_witness(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    witness: Self::Witness,
  ) -> Result<Self, Error>;

  /// Produces a witness with the same shape but unknown values.
  fn unknown_witness(witness: &Self::Witness) -> Self::Witness;
}

impl AssignedFieldExt for AssignedFp {
  type Fixed = ForeignField;

  fn zero(chip: &Bn254FieldChip, layouter: &mut impl Layouter<NativeField>) -> Result<Self, Error> {
    chip.assign(layouter, Value::known(ForeignField::ZERO))
  }

  fn one(chip: &Bn254FieldChip, layouter: &mut impl Layouter<NativeField>) -> Result<Self, Error> {
    chip.assign(layouter, Value::known(ForeignField::ONE))
  }

  fn add(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    chip.add(layouter, self, rhs)
  }

  fn sub(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<Self, Error> {
    chip.sub(layouter, self, rhs)
  }

  fn neg(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    chip.neg(layouter, self)
  }

  fn assert_equal(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<(), Error> {
    chip.assert_equal(layouter, self, rhs)
  }

  fn assert_equal_to_fixed(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    expected: Self::Fixed,
  ) -> Result<(), Error> {
    chip.assert_equal_to_fixed(layouter, self, expected)
  }
}

/// Shared synthesize body for binary operation sanity circuits.
pub(crate) fn synthesize_binary_value_circuit<T, L, Op>(
  config: &Bn254FieldConfig,
  mut layouter: L,
  left_witness: T::Witness,
  right_witness: T::Witness,
  expected: T::Fixed,
  op: Op,
) -> Result<(), Error>
where
  T: AssignedCircuitValue,
  L: Layouter<NativeField>,
  Op: FnOnce(&T, &Bn254FieldChip, &mut L, &T) -> Result<T, Error>,
{
  let chip = Bn254FieldChip::new(config);
  let left = T::assign_witness(&chip, &mut layouter, left_witness)?;
  let right = T::assign_witness(&chip, &mut layouter, right_witness)?;
  let output = op(&left, &chip, &mut layouter, &right)?;
  output.assert_equal_to_fixed(&chip, &mut layouter, expected)?;
  chip.load(&mut layouter)
}

/// Shared synthesize body for unary operation sanity circuits.
pub(crate) fn synthesize_unary_value_circuit<T, L, Op>(
  config: &Bn254FieldConfig,
  mut layouter: L,
  value_witness: T::Witness,
  expected: T::Fixed,
  op: Op,
) -> Result<(), Error>
where
  T: AssignedCircuitValue,
  L: Layouter<NativeField>,
  Op: FnOnce(&T, &Bn254FieldChip, &mut L) -> Result<T, Error>,
{
  let chip = Bn254FieldChip::new(config);
  let value = T::assign_witness(&chip, &mut layouter, value_witness)?;
  let output = op(&value, &chip, &mut layouter)?;
  output.assert_equal_to_fixed(&chip, &mut layouter, expected)?;
  chip.load(&mut layouter)
}
