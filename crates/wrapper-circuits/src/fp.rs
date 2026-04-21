//! Circuit-backed BN254 foreign-field primitives built on Midnight chips.

use midnight_proofs::{
  circuit::{SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};

use crate::{
  bn254::{
    AssignedFpValue, Bn254FieldChip, Bn254FieldConfig, ForeignField, NativeField, measure_layout,
  },
  metrics::LayoutMetrics,
};

/// Assigned BN254 foreign-field element backed by Midnight's `FieldChip`.
pub type AssignedFp = AssignedFpValue;

/// Public wrapper over the Midnight BN254 foreign-field chip.
pub type Bn254FpChip = Bn254FieldChip;

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

#[cfg(test)]
mod tests {
  use ark_bn254::Fq as ArkFq;
  use ark_ff::{BigInteger, PrimeField, UniformRand};
  use ff::PrimeField as HaloPrimeField;
  use midnight_proofs::dev::MockProver;
  use rand::SeedableRng;
  use rand_chacha::ChaCha20Rng;

  use super::*;

  fn ark_to_midnight(value: ArkFq) -> ForeignField {
    let bytes = value.into_bigint().to_bytes_le();
    let mut repr = <ForeignField as HaloPrimeField>::Repr::default();
    let repr_bytes = repr.as_mut();
    let copy_len = bytes.len().min(repr_bytes.len());
    repr_bytes[..copy_len].copy_from_slice(&bytes[..copy_len]);

    ForeignField::from_repr_vartime(repr)
      .expect("arkworks bn254 fq value should fit midnight bn254 fq")
  }

  fn assert_satisfied<CircuitT: Circuit<NativeField>>(circuit: &CircuitT) {
    let k = measure_layout(circuit).k;
    let prover = MockProver::run(k, circuit, vec![vec![], vec![]]).expect("mock prover should run");
    assert_eq!(prover.verify(), Ok(()));
  }

  #[test]
  fn field_edge_cases_match_arkworks() {
    let zero = ArkFq::from(0_u64);
    let one = ArkFq::from(1_u64);
    let modulus_minus_one = -ArkFq::from(1_u64);

    assert_satisfied(&FpAddCircuit::new(ark_to_midnight(zero), ark_to_midnight(one)));
    assert_satisfied(&FpMulCircuit::new(ark_to_midnight(one), ark_to_midnight(modulus_minus_one)));
  }

  #[test]
  fn randomized_additions_match_arkworks() {
    let mut rng = ChaCha20Rng::from_seed([21_u8; 32]);

    for _ in 0..12 {
      let left = ArkFq::rand(&mut rng);
      let right = ArkFq::rand(&mut rng);

      assert_satisfied(&FpAddCircuit::new(ark_to_midnight(left), ark_to_midnight(right)));
    }
  }

  #[test]
  fn randomized_multiplications_match_arkworks() {
    let mut rng = ChaCha20Rng::from_seed([22_u8; 32]);

    for _ in 0..12 {
      let left = ArkFq::rand(&mut rng);
      let right = ArkFq::rand(&mut rng);

      assert_satisfied(&FpMulCircuit::new(ark_to_midnight(left), ark_to_midnight(right)));
    }
  }

  #[test]
  fn layout_metrics_are_real_and_nonzero() {
    let add_metrics = fp_add_layout_metrics();
    let mul_metrics = fp_mul_layout_metrics();

    assert!(add_metrics.rows > 0);
    assert!(mul_metrics.rows > 0);
    assert!(mul_metrics.column_queries > 0);
  }
}
