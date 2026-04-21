use ark_bn254::{
  Fq as ArkFq, Fq2 as ArkFq2, Fq6 as ArkFq6, Fq6Config as ArkFq6Config, Fq12 as ArkFq12,
  Fq12Config as ArkFq12Config, G1Affine as ArkG1Affine, G1Projective as ArkG1Projective,
  G2Affine as ArkG2Affine, G2Projective as ArkG2Projective, g2,
};
use ark_ec::{AffineRepr, CurveGroup, models::short_weierstrass::SWCurveConfig};
use ark_ff::{BigInteger, Fp6Config, Fp12Config, PrimeField, UniformRand};
use ff::{Field, PrimeField as HaloPrimeField};
use halo2curves::group::Group;
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};
use midnight_curves::{CurveAffine, bn256::G1Affine};
use midnight_proofs::dev::MockProver;
use rand::SeedableRng;
use rand_chacha::ChaCha20Rng;

use super::metrics::measure_layout;
use super::*;

type Fp2AssignedValue = (Value<ForeignField>, Value<ForeignField>);
type Fp6ConstantValue =
  ((ForeignField, ForeignField), (ForeignField, ForeignField), (ForeignField, ForeignField));
type Fp12ConstantValue = (Fp6ConstantValue, Fp6ConstantValue);
type G2AssignedValue = (Fp2AssignedValue, Fp2AssignedValue);
type G2ConstantValue = ((ForeignField, ForeignField), (ForeignField, ForeignField));

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

fn ark_to_midnight_fq2(value: ArkFq2) -> (ForeignField, ForeignField) {
  (ark_to_midnight_fq(value.c0), ark_to_midnight_fq(value.c1))
}

fn ark_to_midnight_fq6(value: ArkFq6) -> Fp6ConstantValue {
  (ark_to_midnight_fq2(value.c0), ark_to_midnight_fq2(value.c1), ark_to_midnight_fq2(value.c2))
}

fn ark_to_midnight_fq12(value: &ArkFq12) -> Fp12ConstantValue {
  (ark_to_midnight_fq6(value.c0), ark_to_midnight_fq6(value.c1))
}

fn ark_zero_fq6() -> ArkFq6 {
  ArkFq6::new(
    ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
    ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
    ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
  )
}

fn ark_one_fq6() -> ArkFq6 {
  ArkFq6::new(
    ArkFq2::new(ArkFq::from(1_u64), ArkFq::from(0_u64)),
    ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
    ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
  )
}

fn ark_to_assigned_g2_coords(
  point: ArkG2Affine,
) -> ((ForeignField, ForeignField), (ForeignField, ForeignField)) {
  assert!(!point.is_zero(), "this narrow G2 affine slice does not support infinity");
  (ark_to_midnight_fq2(point.x), ark_to_midnight_fq2(point.y))
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

#[derive(Clone, Debug)]
struct G2EqualityCircuit {
  left: G2AssignedValue,
  right: G2AssignedValue,
}

impl G2EqualityCircuit {
  fn new(left: G2ConstantValue, right: G2ConstantValue) -> Self {
    Self {
      left: (
        (Value::known(left.0.0), Value::known(left.0.1)),
        (Value::known(left.1.0), Value::known(left.1.1)),
      ),
      right: (
        (Value::known(right.0.0), Value::known(right.0.1)),
        (Value::known(right.1.0), Value::known(right.1.1)),
      ),
    }
  }
}

impl Circuit<NativeField> for G2EqualityCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    Self {
      left: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
      right: ((Value::unknown(), Value::unknown()), (Value::unknown(), Value::unknown())),
    }
  }

  fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self::Config {
    Bn254FieldConfig::configure(meta)
  }

  fn synthesize(
    &self,
    config: Self::Config,
    mut layouter: impl Layouter<NativeField>,
  ) -> Result<(), Error> {
    let chip = Bn254FieldChip::new(&config);
    let left = AssignedG2Affine::assign(&chip, &mut layouter, self.left.0, self.left.1)?;
    let right = AssignedG2Affine::assign(&chip, &mut layouter, self.right.0, self.right.1)?;
    left.assert_on_curve(&chip, &mut layouter)?;
    right.assert_on_curve(&chip, &mut layouter)?;
    left.assert_equal(&chip, &mut layouter, &right)?;
    chip.load(&mut layouter)
  }
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
fn fp2_zero_plus_x_is_x() {
  let x = ark_to_midnight_fq2(ArkFq2::new(ArkFq::from(5_u64), ArkFq::from(8_u64)));
  let zero = ark_to_midnight_fq2(ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)));

  assert_satisfied(&Fp2AddCircuit::new(zero, x));
}

#[test]
fn fp2_one_times_x_is_x() {
  let x = ark_to_midnight_fq2(ArkFq2::new(ArkFq::from(9_u64), ArkFq::from(4_u64)));
  let one = ark_to_midnight_fq2(ArkFq2::new(ArkFq::from(1_u64), ArkFq::from(0_u64)));

  assert_satisfied(&Fp2MulCircuit::new(one, x));
}

#[test]
fn fp2_x_plus_neg_x_is_zero() {
  let x = ArkFq2::new(ArkFq::from(12_u64), ArkFq::from(19_u64));

  assert_satisfied(&Fp2AddCircuit::new(ark_to_midnight_fq2(x), ark_to_midnight_fq2(-x)));
}

#[test]
fn fp2_randomized_additions_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([41_u8; 32]);

  for _ in 0..12 {
    let left = ArkFq2::rand(&mut rng);
    let right = ArkFq2::rand(&mut rng);

    assert_satisfied(&Fp2AddCircuit::new(ark_to_midnight_fq2(left), ark_to_midnight_fq2(right)));
  }
}

#[test]
fn fp2_randomized_multiplications_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([42_u8; 32]);

  for _ in 0..12 {
    let left = ArkFq2::rand(&mut rng);
    let right = ArkFq2::rand(&mut rng);

    assert_satisfied(&Fp2MulCircuit::new(ark_to_midnight_fq2(left), ark_to_midnight_fq2(right)));
  }
}

#[test]
fn fp2_randomized_squares_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([43_u8; 32]);

  for _ in 0..12 {
    let value = ArkFq2::rand(&mut rng);

    assert_satisfied(&Fp2SquareCircuit::new(ark_to_midnight_fq2(value)));
  }
}

#[test]
fn fp2_edge_cases_match_arkworks() {
  let vectors = [
    ArkFq2::new(ArkFq::from(7_u64), ArkFq::from(0_u64)),
    ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(9_u64)),
    ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
    ArkFq2::new(ArkFq::from(1_u64), ArkFq::from(1_u64)),
    ArkFq2::new(-ArkFq::from(1_u64), ArkFq::from(3_u64)),
  ];

  assert_satisfied(&Fp2AddCircuit::new(
    ark_to_midnight_fq2(vectors[0]),
    ark_to_midnight_fq2(vectors[1]),
  ));
  assert_satisfied(&Fp2MulCircuit::new(
    ark_to_midnight_fq2(vectors[0]),
    ark_to_midnight_fq2(vectors[1]),
  ));
  assert_satisfied(&Fp2SquareCircuit::new(ark_to_midnight_fq2(vectors[2])));
  assert_satisfied(&Fp2SquareCircuit::new(ark_to_midnight_fq2(vectors[3])));
  assert_satisfied(&Fp2AddCircuit::new(
    ark_to_midnight_fq2(vectors[4]),
    ark_to_midnight_fq2(ArkFq2::new(ArkFq::from(1_u64), ArkFq::from(0_u64))),
  ));
}

#[test]
fn fp2_layout_metrics_are_real_and_nonzero() {
  let add_metrics = fp2_add_layout_metrics();
  let mul_metrics = fp2_mul_layout_metrics();
  let square_metrics = fp2_square_layout_metrics();

  assert!(add_metrics.rows > 0);
  assert!(mul_metrics.rows > 0);
  assert!(square_metrics.rows > 0);
  assert!(mul_metrics.column_queries > 0);
  assert!(square_metrics.column_queries > 0);
}

#[test]
fn fp6_nonresidue_matches_arkworks() {
  assert_eq!(fp6_nonresidue(), ark_to_midnight_fq2(ArkFq6Config::NONRESIDUE));
}

#[test]
fn fp12_nonresidue_matches_arkworks() {
  assert_eq!(fp12_nonresidue(), ark_to_midnight_fq6(ArkFq12Config::NONRESIDUE));
}

#[test]
fn fp6_zero_plus_x_is_x() {
  let x = ArkFq6::new(
    ArkFq2::new(ArkFq::from(5_u64), ArkFq::from(8_u64)),
    ArkFq2::new(ArkFq::from(13_u64), ArkFq::from(21_u64)),
    ArkFq2::new(ArkFq::from(34_u64), ArkFq::from(55_u64)),
  );
  let zero = ArkFq6::new(
    ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
    ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
    ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
  );

  assert_satisfied(&Fp6AddCircuit::new(ark_to_midnight_fq6(zero), ark_to_midnight_fq6(x)));
}

#[test]
fn fp6_one_times_x_is_x() {
  let x = ArkFq6::new(
    ArkFq2::new(ArkFq::from(9_u64), ArkFq::from(4_u64)),
    ArkFq2::new(ArkFq::from(7_u64), ArkFq::from(3_u64)),
    ArkFq2::new(ArkFq::from(11_u64), ArkFq::from(6_u64)),
  );
  let one = ArkFq6::new(
    ArkFq2::new(ArkFq::from(1_u64), ArkFq::from(0_u64)),
    ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
    ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
  );

  assert_satisfied(&Fp6MulCircuit::new(ark_to_midnight_fq6(one), ark_to_midnight_fq6(x)));
}

#[test]
fn fp6_x_plus_neg_x_is_zero() {
  let x = ArkFq6::new(
    ArkFq2::new(ArkFq::from(12_u64), ArkFq::from(19_u64)),
    ArkFq2::new(ArkFq::from(2_u64), ArkFq::from(7_u64)),
    ArkFq2::new(ArkFq::from(5_u64), ArkFq::from(14_u64)),
  );

  assert_satisfied(&Fp6AddCircuit::new(ark_to_midnight_fq6(x), ark_to_midnight_fq6(-x)));
}

#[test]
fn fp6_randomized_additions_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([61_u8; 32]);

  for _ in 0..10 {
    let left = ArkFq6::rand(&mut rng);
    let right = ArkFq6::rand(&mut rng);

    assert_satisfied(&Fp6AddCircuit::new(ark_to_midnight_fq6(left), ark_to_midnight_fq6(right)));
  }
}

#[test]
fn fp6_randomized_multiplications_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([62_u8; 32]);

  for _ in 0..10 {
    let left = ArkFq6::rand(&mut rng);
    let right = ArkFq6::rand(&mut rng);

    assert_satisfied(&Fp6MulCircuit::new(ark_to_midnight_fq6(left), ark_to_midnight_fq6(right)));
  }
}

#[test]
fn fp6_randomized_squares_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([63_u8; 32]);

  for _ in 0..10 {
    let value = ArkFq6::rand(&mut rng);

    assert_satisfied(&Fp6SquareCircuit::new(ark_to_midnight_fq6(value)));
  }
}

#[test]
fn fp6_edge_cases_match_arkworks() {
  let vectors = [
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(7_u64), ArkFq::from(0_u64)),
      ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
      ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
    ),
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
      ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(9_u64)),
      ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
    ),
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
      ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(0_u64)),
      ArkFq2::new(ArkFq::from(4_u64), ArkFq::from(6_u64)),
    ),
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(1_u64), ArkFq::from(1_u64)),
      ArkFq2::new(ArkFq::from(2_u64), ArkFq::from(3_u64)),
      ArkFq2::new(ArkFq::from(5_u64), ArkFq::from(8_u64)),
    ),
  ];

  assert_satisfied(&Fp6AddCircuit::new(
    ark_to_midnight_fq6(vectors[0]),
    ark_to_midnight_fq6(vectors[1]),
  ));
  assert_satisfied(&Fp6MulCircuit::new(
    ark_to_midnight_fq6(vectors[0]),
    ark_to_midnight_fq6(vectors[3]),
  ));
  assert_satisfied(&Fp6MulCircuit::new(
    ark_to_midnight_fq6(vectors[1]),
    ark_to_midnight_fq6(vectors[3]),
  ));
  assert_satisfied(&Fp6MulCircuit::new(
    ark_to_midnight_fq6(vectors[2]),
    ark_to_midnight_fq6(vectors[3]),
  ));
  assert_satisfied(&Fp6SquareCircuit::new(ark_to_midnight_fq6(vectors[2])));
  assert_satisfied(&Fp6SquareCircuit::new(ark_to_midnight_fq6(vectors[3])));
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
fn fp6_layout_metrics_are_real_and_nonzero() {
  let add_metrics = fp6_add_layout_metrics();
  let mul_metrics = fp6_mul_layout_metrics();
  let square_metrics = fp6_square_layout_metrics();

  assert!(add_metrics.rows > 0);
  assert!(mul_metrics.rows > 0);
  assert!(square_metrics.rows > 0);
  assert!(add_metrics.column_queries > 0);
  assert!(mul_metrics.column_queries > 0);
  assert!(square_metrics.column_queries > 0);
}

#[test]
fn fp12_zero_plus_x_is_x() {
  let x = ArkFq12::new(
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(5_u64), ArkFq::from(8_u64)),
      ArkFq2::new(ArkFq::from(13_u64), ArkFq::from(21_u64)),
      ArkFq2::new(ArkFq::from(34_u64), ArkFq::from(55_u64)),
    ),
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(89_u64), ArkFq::from(144_u64)),
      ArkFq2::new(ArkFq::from(233_u64), ArkFq::from(377_u64)),
      ArkFq2::new(ArkFq::from(610_u64), ArkFq::from(987_u64)),
    ),
  );
  let zero = ArkFq12::new(ark_zero_fq6(), ark_zero_fq6());

  assert_satisfied(&Fp12AddCircuit::new(ark_to_midnight_fq12(&zero), ark_to_midnight_fq12(&x)));
}

#[test]
fn fp12_one_times_x_is_x() {
  let x = ArkFq12::new(
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(9_u64), ArkFq::from(4_u64)),
      ArkFq2::new(ArkFq::from(7_u64), ArkFq::from(3_u64)),
      ArkFq2::new(ArkFq::from(11_u64), ArkFq::from(6_u64)),
    ),
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(10_u64), ArkFq::from(12_u64)),
      ArkFq2::new(ArkFq::from(14_u64), ArkFq::from(16_u64)),
      ArkFq2::new(ArkFq::from(18_u64), ArkFq::from(20_u64)),
    ),
  );
  let one = ArkFq12::new(ark_one_fq6(), ark_zero_fq6());

  assert_satisfied(&Fp12MulCircuit::new(ark_to_midnight_fq12(&one), ark_to_midnight_fq12(&x)));
}

#[test]
fn fp12_x_plus_neg_x_is_zero() {
  let x = ArkFq12::new(
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(12_u64), ArkFq::from(19_u64)),
      ArkFq2::new(ArkFq::from(2_u64), ArkFq::from(7_u64)),
      ArkFq2::new(ArkFq::from(5_u64), ArkFq::from(14_u64)),
    ),
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(22_u64), ArkFq::from(29_u64)),
      ArkFq2::new(ArkFq::from(31_u64), ArkFq::from(37_u64)),
      ArkFq2::new(ArkFq::from(41_u64), ArkFq::from(43_u64)),
    ),
  );

  let neg_x = -x;
  assert_satisfied(&Fp12AddCircuit::new(ark_to_midnight_fq12(&x), ark_to_midnight_fq12(&neg_x)));
}

#[test]
fn fp12_randomized_additions_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([71_u8; 32]);

  for _ in 0..10 {
    let left = ArkFq12::rand(&mut rng);
    let right = ArkFq12::rand(&mut rng);

    assert_satisfied(&Fp12AddCircuit::new(
      ark_to_midnight_fq12(&left),
      ark_to_midnight_fq12(&right),
    ));
  }
}

#[test]
fn fp12_randomized_multiplications_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([72_u8; 32]);

  for _ in 0..10 {
    let left = ArkFq12::rand(&mut rng);
    let right = ArkFq12::rand(&mut rng);

    assert_satisfied(&Fp12MulCircuit::new(
      ark_to_midnight_fq12(&left),
      ark_to_midnight_fq12(&right),
    ));
  }
}

#[test]
fn fp12_randomized_squares_match_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([73_u8; 32]);

  for _ in 0..10 {
    let value = ArkFq12::rand(&mut rng);

    assert_satisfied(&Fp12SquareCircuit::new(ark_to_midnight_fq12(&value)));
  }
}

#[test]
fn fp12_structured_cases_match_arkworks() {
  let c0_only = ArkFq12::new(
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(7_u64), ArkFq::from(0_u64)),
      ArkFq2::new(ArkFq::from(0_u64), ArkFq::from(9_u64)),
      ArkFq2::new(ArkFq::from(4_u64), ArkFq::from(6_u64)),
    ),
    ark_zero_fq6(),
  );
  let c1_only = ArkFq12::new(
    ark_zero_fq6(),
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(1_u64), ArkFq::from(1_u64)),
      ArkFq2::new(ArkFq::from(2_u64), ArkFq::from(3_u64)),
      ArkFq2::new(ArkFq::from(5_u64), ArkFq::from(8_u64)),
    ),
  );
  let mixed_small = ArkFq12::new(
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(1_u64), ArkFq::from(2_u64)),
      ArkFq2::new(ArkFq::from(3_u64), ArkFq::from(5_u64)),
      ArkFq2::new(ArkFq::from(8_u64), ArkFq::from(13_u64)),
    ),
    ArkFq6::new(
      ArkFq2::new(ArkFq::from(21_u64), ArkFq::from(34_u64)),
      ArkFq2::new(ArkFq::from(55_u64), ArkFq::from(89_u64)),
      ArkFq2::new(ArkFq::from(144_u64), ArkFq::from(233_u64)),
    ),
  );

  assert_satisfied(&Fp12AddCircuit::new(
    ark_to_midnight_fq12(&c0_only),
    ark_to_midnight_fq12(&c1_only),
  ));
  assert_satisfied(&Fp12MulCircuit::new(
    ark_to_midnight_fq12(&c0_only),
    ark_to_midnight_fq12(&mixed_small),
  ));
  assert_satisfied(&Fp12MulCircuit::new(
    ark_to_midnight_fq12(&c1_only),
    ark_to_midnight_fq12(&mixed_small),
  ));
  assert_satisfied(&Fp12SquareCircuit::new(ark_to_midnight_fq12(&c0_only)));
  assert_satisfied(&Fp12SquareCircuit::new(ark_to_midnight_fq12(&c1_only)));
  assert_satisfied(&Fp12SquareCircuit::new(ark_to_midnight_fq12(&mixed_small)));
}

#[test]
fn fp12_layout_metrics_are_real_and_nonzero() {
  let add_metrics = fp12_add_layout_metrics();
  let mul_metrics = fp12_mul_layout_metrics();
  let square_metrics = fp12_square_layout_metrics();

  assert!(add_metrics.rows > 0);
  assert!(mul_metrics.rows > 0);
  assert!(square_metrics.rows > 0);
  assert!(add_metrics.column_queries > 0);
  assert!(mul_metrics.column_queries > 0);
  assert!(square_metrics.column_queries > 0);
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

#[test]
fn g2_curve_coeff_b_matches_arkworks() {
  assert_eq!(g2_curve_coeff_b(), ark_to_midnight_fq2(g2::Config::COEFF_B));
}

#[test]
fn g2_generator_is_on_curve() {
  let generator = ark_to_assigned_g2_coords(ArkG2Affine::generator());

  assert_satisfied(&G2OnCurveCircuit::new(generator.0, generator.1));
}

#[test]
fn random_valid_g2_points_pass_on_curve_checks() {
  let mut rng = ChaCha20Rng::from_seed([51_u8; 32]);

  for _ in 0..8 {
    let point = ArkG2Projective::rand(&mut rng).into_affine();
    if point.is_zero() {
      continue;
    }

    let point = ark_to_assigned_g2_coords(point);
    assert_satisfied(&G2OnCurveCircuit::new(point.0, point.1));
  }
}

#[test]
fn modified_g2_x_coordinates_are_rejected() {
  let point = ArkG2Affine::generator();
  let bad_x = ArkFq2::new(point.x.c0 + ArkFq::from(1_u64), point.x.c1);

  assert!(!prover_result(&G2OnCurveCircuit::new(
    ark_to_midnight_fq2(bad_x),
    ark_to_midnight_fq2(point.y),
  )));
}

#[test]
fn perturbed_g2_y_coordinates_are_rejected() {
  let point = ArkG2Affine::generator();
  let bad_y = ArkFq2::new(point.y.c0, point.y.c1 + ArkFq::from(1_u64));

  assert!(!prover_result(&G2OnCurveCircuit::new(
    ark_to_midnight_fq2(point.x),
    ark_to_midnight_fq2(bad_y),
  )));
}

#[test]
fn g2_negation_preserves_on_curve_validity() {
  let mut rng = ChaCha20Rng::from_seed([52_u8; 32]);

  for _ in 0..6 {
    let point = ArkG2Projective::rand(&mut rng).into_affine();
    if point.is_zero() {
      continue;
    }

    let negated = -point;
    assert_satisfied(&G2NegCircuit::new(
      ark_to_assigned_g2_coords(point),
      ark_to_assigned_g2_coords(negated),
    ));
  }
}

#[test]
fn g2_projective_identity_encoding_is_available() {
  assert_satisfied(&G2ProjectiveIdentityCircuit);
}

#[test]
fn g2_projective_from_affine_matches_the_same_affine_point() {
  let mut rng = ChaCha20Rng::from_seed([53_u8; 32]);

  for _ in 0..6 {
    let point = ArkG2Projective::rand(&mut rng).into_affine();
    if point.is_zero() {
      continue;
    }

    assert_satisfied(&G2ProjectiveFromAffineCircuit::new(ark_to_assigned_g2_coords(point)));
  }
}

#[test]
fn g2_projective_negation_matches_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([54_u8; 32]);

  for _ in 0..6 {
    let point = ArkG2Projective::rand(&mut rng).into_affine();
    if point.is_zero() {
      continue;
    }

    let negated = -point;
    assert_satisfied(&G2ProjectiveNegCircuit::new(
      ark_to_assigned_g2_coords(point),
      ark_to_assigned_g2_coords(negated),
    ));
  }
}

#[test]
fn g2_projective_doubling_matches_arkworks() {
  let mut rng = ChaCha20Rng::from_seed([55_u8; 32]);

  for _ in 0..8 {
    let point = ArkG2Projective::rand(&mut rng).into_affine();
    if point.is_zero() {
      continue;
    }

    let doubled = (point.into_group() + point).into_affine();
    assert_satisfied(&G2ProjectiveDoubleCircuit::new(
      ark_to_assigned_g2_coords(point),
      ark_to_assigned_g2_coords(doubled),
    ));
  }
}

#[test]
fn g2_projective_addition_matches_arkworks_for_distinct_points() {
  let mut rng = ChaCha20Rng::from_seed([56_u8; 32]);

  for _ in 0..8 {
    let left = ArkG2Projective::rand(&mut rng).into_affine();
    let mut right = ArkG2Projective::rand(&mut rng).into_affine();

    if left.is_zero() || right.is_zero() {
      continue;
    }

    while right == left || right == -left {
      right = ArkG2Projective::rand(&mut rng).into_affine();
    }

    let expected = (left.into_group() + right).into_affine();
    assert_satisfied(&G2ProjectiveAddCircuit::new(
      ark_to_assigned_g2_coords(left),
      ark_to_assigned_g2_coords(right),
      ark_to_assigned_g2_coords(expected),
    ));
  }
}

#[test]
fn g2_projective_doubling_matches_generator_edge_case() {
  let generator = ArkG2Affine::generator();
  let expected = (generator.into_group() + generator).into_affine();

  assert_satisfied(&G2ProjectiveDoubleCircuit::new(
    ark_to_assigned_g2_coords(generator),
    ark_to_assigned_g2_coords(expected),
  ));
}

#[test]
fn g2_projective_addition_matches_generator_plus_double_generator() {
  let generator = ArkG2Affine::generator();
  let double_generator = (generator.into_group() + generator).into_affine();
  let expected = (generator.into_group() + double_generator).into_affine();

  assert_satisfied(&G2ProjectiveAddCircuit::new(
    ark_to_assigned_g2_coords(generator),
    ark_to_assigned_g2_coords(double_generator),
    ark_to_assigned_g2_coords(expected),
  ));
}

#[test]
fn g2_projective_addition_of_inverses_is_not_supported_in_this_slice() {
  let point = ArkG2Affine::generator();
  let negated = -point;

  assert!(!prover_result(&G2ProjectiveAddCircuit::new(
    ark_to_assigned_g2_coords(point),
    ark_to_assigned_g2_coords(negated),
    ark_to_assigned_g2_coords(point),
  )));
}

#[test]
fn g2_assert_equal_accepts_identical_points() {
  let point = ark_to_assigned_g2_coords(ArkG2Affine::generator());

  assert_satisfied(&G2EqualityCircuit::new(point, point));
}

#[test]
fn g2_assert_equal_rejects_distinct_points() {
  let point = ArkG2Affine::generator();
  let negated = -point;

  assert!(!prover_result(&G2EqualityCircuit::new(
    ark_to_assigned_g2_coords(point),
    ark_to_assigned_g2_coords(negated),
  )));
}

#[test]
fn g2_layout_metrics_are_real_and_nonzero() {
  let on_curve_metrics = g2_on_curve_layout_metrics();
  let neg_metrics = g2_neg_layout_metrics();
  let from_affine_metrics = g2_proj_from_affine_layout_metrics();
  let double_metrics = g2_proj_double_layout_metrics();
  let add_metrics = g2_proj_add_layout_metrics();

  assert!(on_curve_metrics.rows > 0);
  assert!(neg_metrics.rows > 0);
  assert!(from_affine_metrics.rows > 0);
  assert!(double_metrics.rows > 0);
  assert!(add_metrics.rows > 0);
  assert!(on_curve_metrics.column_queries > 0);
  assert!(neg_metrics.column_queries > 0);
  assert!(from_affine_metrics.column_queries > 0);
  assert!(double_metrics.column_queries > 0);
  assert!(add_metrics.column_queries > 0);
}
