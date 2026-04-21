use midnight_circuits::midnight_proofs::{
  circuit::{SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};

use super::{Bn254G1Chip, Bn254G1Config, ForeignCurve, ForeignField, NativeField};

/// Small circuit that exercises a single BN254 G1 addition.
#[derive(Clone, Debug)]
pub struct G1AddCircuit {
  left: Value<ForeignCurve>,
  right: Value<ForeignCurve>,
  pub(crate) expected: ForeignCurve,
}

impl G1AddCircuit {
  /// Builds a new G1 addition circuit with a known expected output.
  #[must_use]
  pub fn new(left: ForeignCurve, right: ForeignCurve) -> Self {
    Self { left: Value::known(left), right: Value::known(right), expected: left + right }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    Self::new(ForeignCurve::generator(), ForeignCurve::generator())
  }
}

impl Default for G1AddCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for G1AddCircuit {
  type Config = Bn254G1Config;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self { left: Value::unknown(), right: Value::unknown(), expected: self.expected }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254G1Config::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl midnight_proofs::circuit::Layouter<NativeField>,
  ) -> Result<(), Error> {
    let chip = Bn254G1Chip::new(&config);

    let left = chip.assign(&mut layouter, self.left)?;
    let right = chip.assign(&mut layouter, self.right)?;
    let output = chip.add(&mut layouter, &left, &right)?;
    chip.assert_equal_to_fixed(&mut layouter, &output, self.expected)?;
    chip.load(&mut layouter)
  }
}

/// Small circuit that asserts that a pair of coordinates lies on BN254 G1.
#[derive(Clone, Debug)]
pub struct G1OnCurveCircuit {
  x: Value<ForeignField>,
  y: Value<ForeignField>,
}

impl G1OnCurveCircuit {
  /// Builds a new on-curve circuit from affine coordinates.
  #[must_use]
  pub fn new(x: ForeignField, y: ForeignField) -> Self {
    Self { x: Value::known(x), y: Value::known(y) }
  }
}

impl Circuit<NativeField> for G1OnCurveCircuit {
  type Config = Bn254G1Config;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self { x: Value::unknown(), y: Value::unknown() }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254G1Config::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl midnight_proofs::circuit::Layouter<NativeField>,
  ) -> Result<(), Error> {
    let chip = Bn254G1Chip::new(&config);

    let x = chip.assign_coordinate(&mut layouter, self.x)?;
    let y = chip.assign_coordinate(&mut layouter, self.y)?;
    let _ = chip.point_from_coordinates(&mut layouter, &x, &y)?;

    chip.load(&mut layouter)
  }
}
