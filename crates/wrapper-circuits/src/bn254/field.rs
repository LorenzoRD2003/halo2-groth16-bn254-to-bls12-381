use midnight_circuits::midnight_proofs::{
  circuit::{SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};

use super::{Bn254FieldChip, Bn254FieldConfig, ForeignField, NativeField};

/// Small circuit that exercises a single BN254 foreign-field addition.
#[derive(Clone, Debug)]
pub struct FpAddCircuit {
  left: Value<ForeignField>,
  right: Value<ForeignField>,
  expected: ForeignField,
}

impl FpAddCircuit {
  /// Builds a new addition circuit with a known expected output.
  #[must_use]
  pub fn new(left: ForeignField, right: ForeignField) -> Self {
    Self { left: Value::known(left), right: Value::known(right), expected: left + right }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    Self::new(ForeignField::from(7), ForeignField::from(11))
  }
}

impl Default for FpAddCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for FpAddCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self { left: Value::unknown(), right: Value::unknown(), expected: self.expected }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl midnight_proofs::circuit::Layouter<NativeField>,
  ) -> Result<(), Error> {
    let chip = Bn254FieldChip::new(&config);

    let left = chip.assign(&mut layouter, self.left)?;
    let right = chip.assign(&mut layouter, self.right)?;
    let output = chip.add(&mut layouter, &left, &right)?;
    chip.assert_equal_to_fixed(&mut layouter, &output, self.expected)?;
    chip.load(&mut layouter)
  }
}

/// Small circuit that exercises a single BN254 foreign-field multiplication.
#[derive(Clone, Debug)]
pub struct FpMulCircuit {
  left: Value<ForeignField>,
  right: Value<ForeignField>,
  expected: ForeignField,
}

impl FpMulCircuit {
  /// Builds a new multiplication circuit with a known expected output.
  #[must_use]
  pub fn new(left: ForeignField, right: ForeignField) -> Self {
    Self { left: Value::known(left), right: Value::known(right), expected: left * right }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    Self::new(ForeignField::from(13), ForeignField::from(17))
  }
}

impl Default for FpMulCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for FpMulCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self { left: Value::unknown(), right: Value::unknown(), expected: self.expected }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl midnight_proofs::circuit::Layouter<NativeField>,
  ) -> Result<(), Error> {
    let chip = Bn254FieldChip::new(&config);

    let left = chip.assign(&mut layouter, self.left)?;
    let right = chip.assign(&mut layouter, self.right)?;
    let output = chip.mul(&mut layouter, &left, &right)?;
    chip.assert_equal_to_fixed(&mut layouter, &output, self.expected)?;
    chip.load(&mut layouter)
  }
}
