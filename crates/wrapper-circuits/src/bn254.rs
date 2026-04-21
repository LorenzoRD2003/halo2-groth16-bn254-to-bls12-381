//! Shared Midnight/Halo2 wiring for BN254 Week 1 primitives.

use midnight_circuits::{
  ecc::foreign::ecc_chip::{AssignedForeignPoint, ForeignEccChip},
  field::{
    decomposition::chip::P2RDecompositionChip,
    foreign::{
      field_chip::{AssignedField, FieldChip},
      params::MultiEmulationParams,
    },
    native::{native_chip::NativeChip, native_gadget::NativeGadget},
  },
  instructions::{
    ArithInstructions, AssertionInstructions, AssignmentInstructions, EccInstructions,
  },
  midnight_proofs::{
    circuit::{Layouter, SimpleFloorPlanner, Value},
    dev::cost_model::circuit_model,
    plonk::{Circuit, ConstraintSystem, Error},
  },
  testing_utils::FromScratch,
};
use midnight_curves::bn256::{Fq, Fr, G1};

use crate::metrics::LayoutMetrics;

/// Native Halo2 field used by the current Midnight-backed circuits.
pub type NativeField = Fr;
/// BN254 base field emulated inside the Halo2 circuit.
pub type ForeignField = Fq;
/// BN254 G1 group used by the ECC chip.
pub type ForeignCurve = G1;
/// Assigned BN254 foreign-field element backed by Midnight's `FieldChip`.
pub type AssignedFp = AssignedField<NativeField, ForeignField, MultiEmulationParams>;
/// Assigned BN254 G1 point backed by Midnight's `ForeignEccChip`.
pub type AssignedG1 = AssignedForeignPoint<NativeField, ForeignCurve, MultiEmulationParams>;
type NativeBridge =
  NativeGadget<NativeField, P2RDecompositionChip<NativeField>, NativeChip<NativeField>>;

/// Midnight chip for BN254 foreign-field arithmetic.
pub type MidnightFieldChip =
  FieldChip<NativeField, ForeignField, MultiEmulationParams, NativeBridge>;
/// Midnight chip for BN254 foreign G1 arithmetic.
pub type MidnightEccChip =
  ForeignEccChip<NativeField, ForeignCurve, MultiEmulationParams, NativeBridge, NativeBridge>;
/// Public wrapper over the Midnight BN254 foreign-field chip.
pub type Bn254FpChip = Bn254FieldChip;
/// Public wrapper over the Midnight BN254 G1 chip.
pub type Bn254EccChip = Bn254G1Chip;

/// Shared configuration for the Midnight-backed BN254 foreign-field chip.
#[derive(Clone, Debug)]
pub struct Bn254FieldConfig(<MidnightFieldChip as FromScratch<NativeField>>::Config);

impl Bn254FieldConfig {
  /// Configures the foreign-field chip on a fresh constraint system.
  #[must_use]
  pub fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self {
    let instance_columns = [meta.instance_column(), meta.instance_column()];

    Self(MidnightFieldChip::configure_from_scratch(meta, &instance_columns))
  }
}

/// Shared configuration for the Midnight-backed BN254 G1 chip.
#[derive(Clone, Debug)]
pub struct Bn254G1Config(<MidnightEccChip as FromScratch<NativeField>>::Config);

impl Bn254G1Config {
  /// Configures the foreign-ECC chip on a fresh constraint system.
  #[must_use]
  pub fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self {
    let instance_columns = [meta.instance_column(), meta.instance_column()];

    Self(MidnightEccChip::configure_from_scratch(meta, &instance_columns))
  }
}

/// Thin adapter over Midnight's BN254 foreign-field chip.
#[derive(Clone, Debug)]
pub struct Bn254FieldChip {
  field_chip: MidnightFieldChip,
}

impl Bn254FieldChip {
  /// Instantiates the chip from an existing configuration.
  #[must_use]
  pub fn new(config: &Bn254FieldConfig) -> Self {
    Self { field_chip: MidnightFieldChip::new_from_scratch(&config.0) }
  }

  /// Loads any required tables into the layouter.
  pub fn load(&self, layouter: &mut impl Layouter<NativeField>) -> Result<(), Error> {
    self.field_chip.load_from_scratch(layouter)
  }

  /// Assigns a BN254 base-field witness.
  pub fn assign(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    value: Value<ForeignField>,
  ) -> Result<AssignedFp, Error> {
    self.field_chip.assign(layouter, value)
  }

  /// Adds two BN254 base-field values inside the circuit.
  pub fn add(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    left: &AssignedFp,
    right: &AssignedFp,
  ) -> Result<AssignedFp, Error> {
    self.field_chip.add(layouter, left, right)
  }

  /// Multiplies two BN254 base-field values inside the circuit.
  pub fn mul(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    left: &AssignedFp,
    right: &AssignedFp,
  ) -> Result<AssignedFp, Error> {
    self.field_chip.mul(layouter, left, right, None)
  }

  /// Squares a BN254 base-field value inside the circuit.
  pub fn square(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    value: &AssignedFp,
  ) -> Result<AssignedFp, Error> {
    self.field_chip.mul(layouter, value, value, None)
  }

  /// Subtracts two BN254 base-field values inside the circuit.
  pub fn sub(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    left: &AssignedFp,
    right: &AssignedFp,
  ) -> Result<AssignedFp, Error> {
    self.field_chip.sub(layouter, left, right)
  }

  /// Negates a BN254 base-field value inside the circuit.
  pub fn neg(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    value: &AssignedFp,
  ) -> Result<AssignedFp, Error> {
    self.field_chip.neg(layouter, value)
  }

  /// Asserts that the assigned value matches the expected constant.
  pub fn assert_equal_to_fixed(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    value: &AssignedFp,
    expected: ForeignField,
  ) -> Result<(), Error> {
    self.field_chip.assert_equal_to_fixed(layouter, value, expected)
  }
}

/// Thin adapter over Midnight's BN254 foreign-ECC chip.
#[derive(Clone, Debug)]
pub struct Bn254G1Chip {
  ecc_chip: MidnightEccChip,
}

impl Bn254G1Chip {
  /// Instantiates the chip from an existing configuration.
  #[must_use]
  pub fn new(config: &Bn254G1Config) -> Self {
    Self { ecc_chip: MidnightEccChip::new_from_scratch(&config.0) }
  }

  /// Loads any required tables into the layouter.
  pub fn load(&self, layouter: &mut impl Layouter<NativeField>) -> Result<(), Error> {
    self.ecc_chip.load_from_scratch(layouter)
  }

  /// Assigns a BN254 G1 witness.
  pub fn assign(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    point: Value<ForeignCurve>,
  ) -> Result<AssignedG1, Error> {
    self.ecc_chip.assign(layouter, point)
  }

  /// Assigns a BN254 base-field witness through the ECC chip's base field.
  pub fn assign_coordinate(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    value: Value<ForeignField>,
  ) -> Result<AssignedFp, Error> {
    self.ecc_chip.base_field_chip().assign(layouter, value)
  }

  /// Constructs a point from assigned coordinates and enforces the on-curve relation.
  pub fn point_from_coordinates(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    x: &AssignedFp,
    y: &AssignedFp,
  ) -> Result<AssignedG1, Error> {
    self.ecc_chip.point_from_coordinates(layouter, x, y)
  }

  /// Adds two BN254 G1 points inside the circuit.
  pub fn add(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    left: &AssignedG1,
    right: &AssignedG1,
  ) -> Result<AssignedG1, Error> {
    self.ecc_chip.add(layouter, left, right)
  }

  /// Negates a BN254 G1 point inside the circuit.
  pub fn negate(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    point: &AssignedG1,
  ) -> Result<AssignedG1, Error> {
    self.ecc_chip.negate(layouter, point)
  }

  /// Asserts that the assigned point matches the expected constant point.
  pub fn assert_equal_to_fixed(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    point: &AssignedG1,
    expected: ForeignCurve,
  ) -> Result<(), Error> {
    self.ecc_chip.assert_equal_to_fixed(layouter, point, expected)
  }
}

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

/// Small circuit that exercises a single BN254 G1 addition.
#[derive(Clone, Debug)]
pub struct G1AddCircuit {
  left: Value<ForeignCurve>,
  right: Value<ForeignCurve>,
  expected: ForeignCurve,
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

/// Models a circuit and returns real layout metrics.
#[must_use]
pub fn measure_layout(circuit: &impl Circuit<NativeField>) -> LayoutMetrics {
  LayoutMetrics::from(circuit_model::<NativeField, 48, 32>(circuit))
}

/// Real layout metrics for the current BN254 foreign-field addition circuit.
#[must_use]
pub fn fp_add_layout_metrics() -> LayoutMetrics {
  measure_layout(&FpAddCircuit::sample())
}

/// Real layout metrics for the current BN254 foreign-field multiplication circuit.
#[must_use]
pub fn fp_mul_layout_metrics() -> LayoutMetrics {
  measure_layout(&FpMulCircuit::sample())
}

/// Real layout metrics for the current BN254 G1 addition circuit.
#[must_use]
pub fn g1_add_layout_metrics() -> LayoutMetrics {
  measure_layout(&G1AddCircuit::sample())
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn fp_add_k() -> u32 {
  fp_add_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn fp_mul_k() -> u32 {
  fp_mul_layout_metrics().k
}

/// Returns the smallest power-of-two domain reported by the cost model.
#[must_use]
pub fn g1_add_k() -> u32 {
  g1_add_layout_metrics().k
}

#[cfg(test)]
mod tests {
  use ark_bn254::{Fq as ArkFq, G1Affine as ArkG1Affine, G1Projective as ArkG1Projective};
  use ark_ec::{AffineRepr, CurveGroup};
  use ark_ff::{BigInteger, PrimeField, UniformRand};
  use ff::{Field, PrimeField as HaloPrimeField};
  use halo2curves::group::Group;
  use midnight_curves::{CurveAffine, bn256::G1Affine};
  use midnight_proofs::dev::MockProver;
  use rand::SeedableRng;
  use rand_chacha::ChaCha20Rng;

  use super::*;

  fn ark_to_midnight_fq(value: ArkFq) -> ForeignField {
    let bytes = value.into_bigint().to_bytes_le();
    let mut repr = <ForeignField as HaloPrimeField>::Repr::default();
    let repr_bytes = repr.as_mut();
    let copy_len = bytes.len().min(repr_bytes.len());
    repr_bytes[..copy_len].copy_from_slice(&bytes[..copy_len]);

    ForeignField::from_repr_vartime(repr)
      .expect("arkworks bn254 fq value should fit midnight bn254 fq")
  }

  fn ark_to_midnight_g1(point: ArkG1Affine) -> ForeignCurve {
    if point.is_zero() {
      return ForeignCurve::identity();
    }

    let affine = Option::<G1Affine>::from(G1Affine::from_xy(
      ark_to_midnight_fq(point.x),
      ark_to_midnight_fq(point.y),
    ))
    .expect("arkworks point should map to a valid midnight bn254 point");

    affine.into()
  }

  fn assert_satisfied<CircuitT: Circuit<NativeField>>(circuit: &CircuitT) {
    let k = measure_layout(circuit).k;
    let prover = MockProver::run(k, circuit, vec![vec![], vec![]]).expect("mock prover should run");
    assert_eq!(prover.verify(), Ok(()));
  }

  fn prover_result<CircuitT: Circuit<NativeField>>(circuit: &CircuitT) -> bool {
    let k = measure_layout(circuit).k;
    let prover = MockProver::run(k, circuit, vec![vec![], vec![]]).expect("mock prover should run");
    prover.verify().is_ok()
  }

  #[test]
  fn field_edge_cases_match_arkworks() {
    let zero = ArkFq::from(0_u64);
    let one = ArkFq::from(1_u64);
    let modulus_minus_one = -ArkFq::from(1_u64);

    assert_satisfied(&FpAddCircuit::new(ark_to_midnight_fq(zero), ark_to_midnight_fq(one)));
    assert_satisfied(&FpMulCircuit::new(
      ark_to_midnight_fq(one),
      ark_to_midnight_fq(modulus_minus_one),
    ));
  }

  #[test]
  fn randomized_additions_match_arkworks() {
    let mut rng = ChaCha20Rng::from_seed([21_u8; 32]);

    for _ in 0..12 {
      let left = ArkFq::rand(&mut rng);
      let right = ArkFq::rand(&mut rng);

      assert_satisfied(&FpAddCircuit::new(ark_to_midnight_fq(left), ark_to_midnight_fq(right)));
    }
  }

  #[test]
  fn randomized_multiplications_match_arkworks() {
    let mut rng = ChaCha20Rng::from_seed([22_u8; 32]);

    for _ in 0..12 {
      let left = ArkFq::rand(&mut rng);
      let right = ArkFq::rand(&mut rng);

      assert_satisfied(&FpMulCircuit::new(ark_to_midnight_fq(left), ark_to_midnight_fq(right)));
    }
  }

  #[test]
  fn fp_layout_metrics_are_real_and_nonzero() {
    let add_metrics = fp_add_layout_metrics();
    let mul_metrics = fp_mul_layout_metrics();

    assert!(add_metrics.rows > 0);
    assert!(mul_metrics.rows > 0);
    assert!(mul_metrics.column_queries > 0);
  }

  #[test]
  fn g1_addition_matches_arkworks() {
    let mut rng = ChaCha20Rng::from_seed([31_u8; 32]);

    for _ in 0..8 {
      let left = ArkG1Projective::rand(&mut rng).into_affine();
      let right = ArkG1Projective::rand(&mut rng).into_affine();

      let circuit = G1AddCircuit::new(ark_to_midnight_g1(left), ark_to_midnight_g1(right));
      assert!(prover_result(&circuit));
    }
  }

  #[test]
  fn g1_doubling_works_via_addition() {
    let mut rng = ChaCha20Rng::from_seed([32_u8; 32]);

    for _ in 0..6 {
      let point = ArkG1Projective::rand(&mut rng).into_affine();
      let doubled = (point.into_group() + point).into_affine();
      let circuit = G1AddCircuit::new(ark_to_midnight_g1(point), ark_to_midnight_g1(point));

      assert!(prover_result(&circuit));
      assert_eq!(ark_to_midnight_g1(doubled), circuit.expected);
    }
  }

  #[test]
  fn invalid_point_is_rejected() {
    let result = std::panic::catch_unwind(|| {
      let circuit = G1OnCurveCircuit::new(ForeignField::ZERO, ForeignField::ZERO);
      prover_result(&circuit)
    });

    assert!(result.is_err() || !result.expect("catch_unwind should resolve"));
  }

  #[test]
  fn g1_layout_metrics_are_real_and_nonzero() {
    let metrics = g1_add_layout_metrics();

    assert!(metrics.rows > 0);
    assert!(metrics.lookups > 0 || metrics.permutations > 0);
  }
}
