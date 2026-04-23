//! Narrow Groth16 BN254 verifier support built on the landed pairing core.
//!
//! This module intentionally stays small:
//! - proof / verification-key material is already parsed into affine coordinates
//! - public inputs are consumed only as verifier scalars for IC accumulation
//! - verification reduces directly to one pairing-product check
//! - broader backend orchestration, generalized serialization, and public API
//!   frameworks remain out of scope

use ff::Field;
use halo2curves::group::Group;
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};
use midnight_curves::{CurveAffine, bn256::G1Affine};
use thiserror::Error;

use crate::bn254::{
  AssignedBool, AssignedG1, AssignedG1Point, AssignedG2Affine, Bn254BoolChip, Bn254BoolConfig,
  Bn254FieldChip, Bn254FieldConfig, Bn254G1Chip, Bn254G1Config, ForeignCurve, ForeignField,
  NativeField, PreparedConstantG2Miller, pairing_check_with_prepared_terms,
};

pub mod fixtures;
pub mod profiling;
#[cfg(any(test, feature = "test"))]
pub mod reference;

type G2AffineCoordinates = ((ForeignField, ForeignField), (ForeignField, ForeignField));

pub(crate) fn groth16_g1_affine_coordinates(
  point: Groth16Bn254G1Point,
) -> (ForeignField, ForeignField) {
  match point {
    Groth16Bn254G1Point::Identity => {
      panic!("profiling scenarios require non-identity affine G1 points")
    }
    Groth16Bn254G1Point::Affine { x, y } => (x, y),
  }
}

pub(crate) fn groth16_g1_to_midnight_curve(point: Groth16Bn254G1Point) -> ForeignCurve {
  match point {
    Groth16Bn254G1Point::Identity => ForeignCurve::identity(),
    Groth16Bn254G1Point::Affine { x, y } => {
      let affine = Option::<G1Affine>::from(G1Affine::from_xy(x, y))
        .expect("Groth16 G1 point should be valid");
      affine.into()
    }
  }
}

pub(crate) fn groth16_negate_g1(point: Groth16Bn254G1Point) -> Groth16Bn254G1Point {
  match point {
    Groth16Bn254G1Point::Identity => Groth16Bn254G1Point::Identity,
    Groth16Bn254G1Point::Affine { x, y } => Groth16Bn254G1Point::Affine { x, y: -y },
  }
}

pub(crate) fn groth16_public_input_accumulator_constant(
  vk: &Groth16Bn254VerifyingKey,
  public_inputs: &[NativeField],
) -> Groth16Bn254G1Point {
  let mut accumulator = groth16_g1_to_midnight_curve(vk.ic[0]);

  for (scalar, ic_point) in public_inputs.iter().zip(vk.ic.iter().skip(1)) {
    accumulator += groth16_g1_to_midnight_curve(*ic_point) * *scalar;
  }

  let affine = G1Affine::from(accumulator);
  let coordinates = Option::<midnight_curves::Coordinates<G1Affine>>::from(affine.coordinates())
    .expect(
      "Groth16 public-input accumulator should remain non-identity for the canonical fixture",
    );
  Groth16Bn254G1Point::affine(*coordinates.x(), *coordinates.y())
}

/// Narrow BN254 G1 point encoding for the Week 5 Groth16 verifier slice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Groth16Bn254G1Point {
  /// The projective identity point.
  Identity,
  /// A non-identity affine point.
  Affine {
    /// Affine x-coordinate.
    x: ForeignField,
    /// Affine y-coordinate.
    y: ForeignField,
  },
}

impl Groth16Bn254G1Point {
  /// Builds a non-identity affine G1 point.
  #[must_use]
  pub fn affine(x: ForeignField, y: ForeignField) -> Self {
    Self::Affine { x, y }
  }
}

/// Narrow BN254 Groth16 proof material already normalized into affine coordinates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Groth16Bn254Proof {
  /// Groth16 proof element `A` in BN254 G1 affine coordinates.
  pub a: Groth16Bn254G1Point,
  /// Groth16 proof element `B` in BN254 G2 affine coordinates.
  pub b: G2AffineCoordinates,
  /// Groth16 proof element `C` in BN254 G1 affine coordinates.
  pub c: Groth16Bn254G1Point,
}

/// Narrow BN254 Groth16 verification key material.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Groth16Bn254VerifyingKey {
  /// Groth16 verification-key element `alpha_g1`.
  pub alpha_g1: Groth16Bn254G1Point,
  /// Groth16 verification-key element `beta_g2`.
  pub beta_g2: G2AffineCoordinates,
  /// Groth16 verification-key element `gamma_g2`.
  pub gamma_g2: G2AffineCoordinates,
  /// Groth16 verification-key element `delta_g2`.
  pub delta_g2: G2AffineCoordinates,
  /// Verifier IC points, where `ic[0]` is the constant term.
  pub ic: Vec<Groth16Bn254G1Point>,
}

/// Errors for the narrow Groth16 BN254 verifier slice.
#[derive(Debug, Error)]
pub enum Groth16VerifierError {
  /// The public-input vector does not match the IC table.
  #[error("public input length mismatch: expected {expected} inputs from IC table, got {actual}")]
  PublicInputLengthMismatch {
    /// The number of public inputs implied by `vk.ic`.
    expected: usize,
    /// The number of public inputs supplied with the proof.
    actual: usize,
  },
  /// The verification key is malformed for this narrow slice.
  #[error("verification key must contain at least the constant IC point")]
  EmptyIcTable,
  /// Underlying Halo2 / Midnight synthesis error.
  #[error(transparent)]
  Circuit(#[from] Error),
}

fn assign_g1_affine(
  chip: &Bn254G1Chip,
  layouter: &mut impl Layouter<NativeField>,
  point: Groth16Bn254G1Point,
) -> Result<AssignedG1, Error> {
  match point {
    Groth16Bn254G1Point::Identity => chip.assign(layouter, Value::known(ForeignCurve::identity())),
    Groth16Bn254G1Point::Affine { x, y } => {
      let x = chip.assign_coordinate(layouter, Value::known(x))?;
      let y = chip.assign_coordinate(layouter, Value::known(y))?;
      chip.point_from_coordinates(layouter, &x, &y)
    }
  }
}

fn assign_g2_affine(
  chip: &Bn254FieldChip,
  layouter: &mut impl Layouter<NativeField>,
  coords: G2AffineCoordinates,
) -> Result<AssignedG2Affine, Error> {
  AssignedG2Affine::assign(
    chip,
    layouter,
    (Value::known((coords.0).0), Value::known((coords.0).1)),
    (Value::known((coords.1).0), Value::known((coords.1).1)),
  )
}

fn assigned_g1_to_pairing_point(chip: &Bn254G1Chip, point: &AssignedG1) -> AssignedG1Point {
  AssignedG1Point::new(chip.x_coordinate(point), chip.y_coordinate(point))
}

fn assert_non_identity(
  g1_chip: &Bn254G1Chip,
  bool_chip: &Bn254BoolChip,
  layouter: &mut impl Layouter<NativeField>,
  point: &AssignedG1,
) -> Result<(), Groth16VerifierError> {
  let is_identity = g1_chip.is_identity(layouter, point)?;
  bool_chip.assert_equal_to_fixed(layouter, &is_identity, false)?;
  Ok(())
}

fn validate_public_input_shape(
  vk: &Groth16Bn254VerifyingKey,
  public_inputs: &[NativeField],
) -> Result<(), Groth16VerifierError> {
  let Some(expected) = vk.ic.len().checked_sub(1) else {
    return Err(Groth16VerifierError::EmptyIcTable);
  };

  if public_inputs.len() != expected {
    return Err(Groth16VerifierError::PublicInputLengthMismatch {
      expected,
      actual: public_inputs.len(),
    });
  }

  Ok(())
}

/// Computes the verifier-side IC accumulator
/// `vk_x = IC_0 + sum_i public_input_i * IC_i`.
///
/// This uses a narrow verifier-only fixed-scalar multiplication path over the
/// existing Midnight G1 chip because the repository still does not expose a
/// broader public G1 scalar-multiplication API.
pub fn groth16_accumulate_ic(
  g1_chip: &Bn254G1Chip,
  layouter: &mut impl Layouter<NativeField>,
  vk: &Groth16Bn254VerifyingKey,
  public_inputs: &[NativeField],
) -> Result<AssignedG1, Groth16VerifierError> {
  validate_public_input_shape(vk, public_inputs)?;

  let mut accumulator = assign_g1_affine(g1_chip, layouter, vk.ic[0])?;

  for (scalar, ic_point) in public_inputs.iter().zip(vk.ic.iter().skip(1)) {
    if scalar.is_zero_vartime() || matches!(ic_point, Groth16Bn254G1Point::Identity) {
      continue;
    }

    let scaled_constant = groth16_g1_to_midnight_curve(*ic_point) * *scalar;
    let scaled = g1_chip.assign(layouter, Value::known(scaled_constant))?;
    accumulator = g1_chip.add(layouter, &accumulator, &scaled)?;
  }

  Ok(accumulator)
}

#[cfg(test)]
fn groth16_accumulate_ic_legacy(
  g1_chip: &Bn254G1Chip,
  layouter: &mut impl Layouter<NativeField>,
  vk: &Groth16Bn254VerifyingKey,
  public_inputs: &[NativeField],
) -> Result<AssignedG1, Groth16VerifierError> {
  validate_public_input_shape(vk, public_inputs)?;

  let mut accumulator = assign_g1_affine(g1_chip, layouter, vk.ic[0])?;

  for (scalar, ic_point) in public_inputs.iter().zip(vk.ic.iter().skip(1)) {
    let assigned_ic = assign_g1_affine(g1_chip, layouter, *ic_point)?;
    let scaled = g1_chip.mul_by_scalar_constant(layouter, *scalar, &assigned_ic)?;
    accumulator = g1_chip.add(layouter, &accumulator, &scaled)?;
  }

  Ok(accumulator)
}

/// Verifies one narrow BN254 Groth16 proof with the landed pairing core.
///
/// The standard Groth16 verifier relation is
/// `e(A, B) = e(alpha, beta) * e(vk_x, gamma) * e(C, delta)`.
///
/// We move every right-hand-side term to the left by negating the G1 inputs,
/// which is valid because pairings are bilinear in G1:
/// `e(-P, Q) = e(P, Q)^(-1)`.
///
/// The product-check form consumed by `pairing_check(...)` is therefore:
/// `e(A, B) * e(-alpha, beta) * e(-vk_x, gamma) * e(-C, delta) = 1`.
pub fn groth16_verify(
  field_chip: &Bn254FieldChip,
  bool_chip: &Bn254BoolChip,
  g1_chip: &Bn254G1Chip,
  layouter: &mut impl Layouter<NativeField>,
  vk: &Groth16Bn254VerifyingKey,
  proof: &Groth16Bn254Proof,
  public_inputs: &[NativeField],
) -> Result<AssignedBool, Groth16VerifierError> {
  validate_public_input_shape(vk, public_inputs)?;

  let proof_a = assign_g1_affine(g1_chip, layouter, proof.a)?;
  let proof_c = assign_g1_affine(g1_chip, layouter, proof.c)?;
  let alpha_g1 = assign_g1_affine(g1_chip, layouter, vk.alpha_g1)?;
  let vk_x = groth16_accumulate_ic(g1_chip, layouter, vk, public_inputs)?;

  assert_non_identity(g1_chip, bool_chip, layouter, &proof_a)?;
  assert_non_identity(g1_chip, bool_chip, layouter, &proof_c)?;
  assert_non_identity(g1_chip, bool_chip, layouter, &alpha_g1)?;
  assert_non_identity(g1_chip, bool_chip, layouter, &vk_x)?;

  let neg_alpha = g1_chip.negate(layouter, &alpha_g1)?;
  let neg_vk_x = g1_chip.negate(layouter, &vk_x)?;
  let neg_c = g1_chip.negate(layouter, &proof_c)?;

  let proof_b = assign_g2_affine(field_chip, layouter, proof.b)?;
  let prepared_beta_g2 = PreparedConstantG2Miller::from_affine_constant(vk.beta_g2);
  let prepared_gamma_g2 = PreparedConstantG2Miller::from_affine_constant(vk.gamma_g2);
  let prepared_delta_g2 = PreparedConstantG2Miller::from_affine_constant(vk.delta_g2);

  let proof_a_pair = assigned_g1_to_pairing_point(g1_chip, &proof_a);
  let neg_alpha_pair = assigned_g1_to_pairing_point(g1_chip, &neg_alpha);
  let neg_vk_x_pair = assigned_g1_to_pairing_point(g1_chip, &neg_vk_x);
  let neg_c_pair = assigned_g1_to_pairing_point(g1_chip, &neg_c);

  let variable_terms = [(&proof_a_pair, &proof_b)];
  let prepared_terms = [
    (&neg_alpha_pair, &prepared_beta_g2),
    (&neg_vk_x_pair, &prepared_gamma_g2),
    (&neg_c_pair, &prepared_delta_g2),
  ];

  Ok(pairing_check_with_prepared_terms(
    field_chip,
    bool_chip,
    layouter,
    &variable_terms,
    &prepared_terms,
  )?)
}

#[derive(Clone, Debug)]
pub struct Groth16VerifierConfig {
  field: Bn254FieldConfig,
  bools: Bn254BoolConfig,
  g1: Bn254G1Config,
}

/// Small circuit that exercises the narrow BN254 Groth16 verifier slice.
#[derive(Clone, Debug)]
pub struct Groth16Bn254VerifierCircuit {
  proof: Groth16Bn254Proof,
  vk: Groth16Bn254VerifyingKey,
  public_inputs: Vec<NativeField>,
  expected: bool,
}

impl Groth16Bn254VerifierCircuit {
  /// Builds a Groth16 verifier circuit with a known expected result.
  #[must_use]
  pub fn new(
    vk: Groth16Bn254VerifyingKey,
    proof: Groth16Bn254Proof,
    public_inputs: Vec<NativeField>,
    expected: bool,
  ) -> Self {
    Self { proof, vk, public_inputs, expected }
  }
}

impl Circuit<NativeField> for Groth16Bn254VerifierCircuit {
  type Config = Groth16VerifierConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      proof: self.proof.clone(),
      vk: self.vk.clone(),
      public_inputs: vec![NativeField::ZERO; self.public_inputs.len()],
      expected: self.expected,
    }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    let instance_columns = [meta.instance_column(), meta.instance_column()];
    Groth16VerifierConfig {
      field: Bn254FieldConfig::configure_with_instances(meta, &instance_columns),
      bools: Bn254BoolConfig::configure_with_instances(meta, &instance_columns),
      g1: Bn254G1Config::configure_with_instances(meta, &instance_columns),
    }
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    let field_chip = Bn254FieldChip::new(&config.field);
    let bool_chip = Bn254BoolChip::new(&config.bools);
    let g1_chip = Bn254G1Chip::new(&config.g1);

    let result = groth16_verify(
      &field_chip,
      &bool_chip,
      &g1_chip,
      &mut layouter,
      &self.vk,
      &self.proof,
      &self.public_inputs,
    )
    .map_err(|error| match error {
      Groth16VerifierError::Circuit(inner) => inner,
      _ => Error::Synthesis(error.to_string()),
    })?;

    bool_chip.assert_equal_to_fixed(&mut layouter, &result, self.expected)?;
    field_chip.load(&mut layouter)?;
    bool_chip.load(&mut layouter)?;
    g1_chip.load(&mut layouter)
  }
}

#[derive(Clone, Debug)]
pub struct Groth16IcAccumulatorConfig {
  g1: Bn254G1Config,
}

/// Small circuit that locks the IC accumulator against a known expected point.
#[derive(Clone, Debug)]
pub struct Groth16IcAccumulatorCircuit {
  vk: Groth16Bn254VerifyingKey,
  public_inputs: Vec<NativeField>,
  expected: ForeignCurve,
}

impl Groth16IcAccumulatorCircuit {
  /// Builds an IC-accumulator circuit from a verifying key, public inputs, and a host-side expected point.
  #[must_use]
  pub fn new(
    vk: Groth16Bn254VerifyingKey,
    public_inputs: Vec<NativeField>,
    expected: ForeignCurve,
  ) -> Self {
    Self { vk, public_inputs, expected }
  }
}

impl Circuit<NativeField> for Groth16IcAccumulatorCircuit {
  type Config = Groth16IcAccumulatorConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      vk: self.vk.clone(),
      public_inputs: vec![NativeField::ZERO; self.public_inputs.len()],
      expected: self.expected,
    }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Groth16IcAccumulatorConfig { g1: Bn254G1Config::configure(meta) }
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    let g1_chip = Bn254G1Chip::new(&config.g1);
    let accumulator = groth16_accumulate_ic(&g1_chip, &mut layouter, &self.vk, &self.public_inputs)
      .map_err(|error| match error {
        Groth16VerifierError::Circuit(inner) => inner,
        _ => Error::Synthesis(error.to_string()),
      })?;

    g1_chip.assert_equal_to_fixed(&mut layouter, &accumulator, self.expected)?;
    g1_chip.load(&mut layouter)
  }
}

#[cfg(test)]
mod tests;
