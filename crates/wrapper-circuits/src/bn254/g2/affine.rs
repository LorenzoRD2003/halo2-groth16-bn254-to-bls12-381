use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};

use super::{
  AssignedFp2, Bn254FieldChip, Bn254FieldConfig, Fp2Constant, Fp2Value, G2AffineConstant,
  G2AffineValue, NativeField, fp2_neg_constant, g2_curve_coeff_b, g2_generator,
};

/// Assigned BN254 G2 affine point represented over the Fp2 twist coordinates.
///
/// This narrow slice supports only non-infinity affine points.
#[derive(Clone, Debug)]
pub struct AssignedG2Affine {
  /// X coordinate in Fp2.
  pub x: AssignedFp2,
  /// Y coordinate in Fp2.
  pub y: AssignedFp2,
}

impl AssignedG2Affine {
  /// Builds an assigned G2 affine point from assigned Fp2 coordinates.
  #[must_use]
  pub fn new(x: AssignedFp2, y: AssignedFp2) -> Self {
    Self { x, y }
  }

  /// Assigns a non-infinity G2 affine point from Fp2 coordinates.
  ///
  /// # Errors
  ///
  /// Returns an error if assigning any of the underlying Fp2 coordinates fails.
  pub fn assign(
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    x: Fp2Value,
    y: Fp2Value,
  ) -> Result<Self, Error> {
    Ok(Self::new(
      AssignedFp2::assign(chip, layouter, x.0, x.1)?,
      AssignedFp2::assign(chip, layouter, y.0, y.1)?,
    ))
  }

  /// Negates a G2 affine point inside the circuit.
  ///
  /// # Errors
  ///
  /// Returns an error if negating the underlying Fp2 y-coordinate fails.
  pub fn neg(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<Self, Error> {
    Ok(Self::new(self.x.clone(), self.y.neg(chip, layouter)?))
  }

  /// Asserts coordinate-wise equality against another assigned G2 affine point.
  ///
  /// # Errors
  ///
  /// Returns an error if either Fp2 coordinate equality constraint cannot be enforced.
  pub fn assert_equal(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
    rhs: &Self,
  ) -> Result<(), Error> {
    self.x.assert_equal(chip, layouter, &rhs.x)?;
    self.y.assert_equal(chip, layouter, &rhs.y)
  }

  /// Asserts that the assigned coordinates satisfy the BN254 G2 twist equation.
  ///
  /// # Errors
  ///
  /// Returns an error if any Fp2 assignment or equality constraint involved in the twist
  /// equation check fails.
  pub fn assert_on_curve(
    &self,
    chip: &Bn254FieldChip,
    layouter: &mut impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    let y_square = self.y.square(chip, layouter)?;
    let x_square = self.x.square(chip, layouter)?;
    let x_cube = x_square.mul(chip, layouter, &self.x)?;
    let coeff_b = {
      let coeff_b = g2_curve_coeff_b();
      AssignedFp2::assign(chip, layouter, Value::known(coeff_b.0), Value::known(coeff_b.1))?
    };
    let rhs = x_cube.add(chip, layouter, &coeff_b)?;

    y_square.assert_equal(chip, layouter, &rhs)
  }
}

/// Small circuit that asserts that a pair of Fp2 coordinates lies on BN254 G2.
#[derive(Clone, Debug)]
pub struct G2OnCurveCircuit {
  x: Fp2Value,
  y: Fp2Value,
}

impl G2OnCurveCircuit {
  /// Builds a new G2 on-curve circuit from affine Fp2 coordinates.
  #[must_use]
  pub fn new(x: Fp2Constant, y: Fp2Constant) -> Self {
    Self { x: (Value::known(x.0), Value::known(x.1)), y: (Value::known(y.0), Value::known(y.1)) }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    let generator = g2_generator();
    Self::new(generator.0, generator.1)
  }
}

impl Default for G2OnCurveCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for G2OnCurveCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self { x: (Value::unknown(), Value::unknown()), y: (Value::unknown(), Value::unknown()) }
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
    let point = AssignedG2Affine::assign(&chip, &mut layouter, self.x, self.y)?;
    point.assert_on_curve(&chip, &mut layouter)?;
    chip.load(&mut layouter)
  }
}

/// Small circuit that exercises G2 affine negation and checks the result.
#[derive(Clone, Debug)]
pub struct G2NegCircuit {
  point: G2AffineValue,
  expected: G2AffineValue,
}

impl G2NegCircuit {
  /// Builds a new G2 negation circuit with a known expected output.
  #[must_use]
  pub fn new(point: G2AffineConstant, expected: G2AffineConstant) -> Self {
    Self {
      point: (
        (Value::known(point.0.0), Value::known(point.0.1)),
        (Value::known(point.1.0), Value::known(point.1.1)),
      ),
      expected: (
        (Value::known(expected.0.0), Value::known(expected.0.1)),
        (Value::known(expected.1.0), Value::known(expected.1.1)),
      ),
    }
  }

  /// Returns a deterministic sample circuit suitable for metrics and benches.
  #[must_use]
  pub fn sample() -> Self {
    let point = g2_generator();
    Self::new(point, (point.0, fp2_neg_constant(point.1)))
  }
}

impl Default for G2NegCircuit {
  fn default() -> Self {
    Self::sample()
  }
}

impl Circuit<NativeField> for G2NegCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      point: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
      expected: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
    }
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
    let point = AssignedG2Affine::assign(&chip, &mut layouter, self.point.0, self.point.1)?;
    point.assert_on_curve(&chip, &mut layouter)?;
    let output = point.neg(&chip, &mut layouter)?;
    output.assert_on_curve(&chip, &mut layouter)?;
    let expected =
      AssignedG2Affine::assign(&chip, &mut layouter, self.expected.0, self.expected.1)?;
    output.assert_equal(&chip, &mut layouter, &expected)?;
    chip.load(&mut layouter)
  }
}
