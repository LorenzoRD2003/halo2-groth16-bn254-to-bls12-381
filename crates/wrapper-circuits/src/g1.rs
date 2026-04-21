//! Circuit-backed BN254 G1 primitives built on Midnight chips.

use midnight_proofs::{
  circuit::{SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};

use crate::{
  bn254::{
    AssignedG1Value, Bn254G1Chip, Bn254G1Config, ForeignCurve, ForeignField, NativeField,
    measure_layout,
  },
  metrics::LayoutMetrics,
};

/// Assigned BN254 G1 point backed by Midnight's `ForeignEccChip`.
pub type AssignedG1 = AssignedG1Value;

/// Public wrapper over the Midnight BN254 G1 chip.
pub type Bn254EccChip = Bn254G1Chip;

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

/// Real layout metrics for the current BN254 G1 addition circuit.
#[must_use]
pub fn g1_add_layout_metrics() -> LayoutMetrics {
  measure_layout(&G1AddCircuit::sample())
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

  fn prover_result<CircuitT: Circuit<NativeField>>(circuit: &CircuitT) -> bool {
    let k = measure_layout(circuit).k;
    let prover = MockProver::run(k, circuit, vec![vec![], vec![]]).expect("mock prover should run");
    prover.verify().is_ok()
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
  fn layout_metrics_are_real_and_nonzero() {
    let metrics = g1_add_layout_metrics();

    assert!(metrics.rows > 0);
    assert!(metrics.lookups > 0 || metrics.permutations > 0);
  }
}
