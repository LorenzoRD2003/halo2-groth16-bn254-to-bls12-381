use ark_bn254::{
  Fq12 as ArkFq12, G1Affine as ArkG1Affine, G1Projective as ArkG1Projective,
  G2Affine as ArkG2Affine,
};
use ark_ec::{AffineRepr, CurveGroup, PrimeGroup};
use ark_ff::Field as _;
use ark_std::rand::{SeedableRng, rngs::StdRng};
use midnight_circuits::midnight_proofs::{
  circuit::{Layouter, SimpleFloorPlanner, Value},
  plonk::{Circuit, ConstraintSystem, Error},
};

use super::support::{
  ArkMillerStep, ark_bn254_final_exponentiation, ark_bn254_miller_loop_accumulate,
  ark_bn254_multi_miller_loop_product, ark_bn254_pairing, ark_bn254_pairing_check,
  ark_bn254_pairing_product, ark_bn254_prepared_miller_steps, ark_pairing_terms_to_constants,
  ark_to_assigned_g2_coords, ark_to_midnight_fq, ark_to_midnight_fq12, assert_satisfied,
  random_nonzero_g1_affine, random_nonzero_g2_affine,
};
use super::*;

#[derive(Clone, Debug)]
struct MultiMillerLoopCircuit {
  terms: Vec<PairingTermConstantValue>,
  expected: Fp12ConstantValue,
}

impl MultiMillerLoopCircuit {
  fn new(terms: &[(ArkG1Affine, ArkG2Affine)], expected: &ArkFq12) -> Self {
    Self { terms: ark_pairing_terms_to_constants(terms), expected: ark_to_midnight_fq12(expected) }
  }
}

impl Circuit<NativeField> for MultiMillerLoopCircuit {
  type Config = Bn254FieldConfig;
  type FloorPlanner = SimpleFloorPlanner;
  type Params = ();

  fn without_witnesses(&self) -> Self {
    self.clone()
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
    let mut assigned_terms = Vec::with_capacity(self.terms.len());

    for (g1, g2) in &self.terms {
      let assigned_g1 =
        AssignedG1Point::assign(&chip, &mut layouter, Value::known(g1.0), Value::known(g1.1))?;
      let assigned_g2 = AssignedG2Affine::assign(
        &chip,
        &mut layouter,
        (Value::known((g2.0).0), Value::known((g2.0).1)),
        (Value::known((g2.1).0), Value::known((g2.1).1)),
      )?;
      assigned_terms.push((assigned_g1, assigned_g2));
    }

    let borrowed_terms: Vec<_> = assigned_terms.iter().map(|term| (&term.0, &term.1)).collect();
    let actual = multi_miller_loop(&chip, &mut layouter, &borrowed_terms)?;
    let expected = assign_fixed_fp12(&chip, &mut layouter, self.expected)?;
    actual.assert_equal(&chip, &mut layouter, &expected)?;
    chip.load(&mut layouter)
  }
}

#[test]
fn bn254_miller_schedule_matches_expected_optimal_ate_shape() {
  let expected_loop_digits = [
    1, 0, 1, 0, 0, 0, -1, 0, -1, 0, 0, 0, -1, 0, 1, 0, -1, 0, 0, -1, 0, 0, 0, 0, 0, 1, 0, 0, -1, 0,
    1, 0, 0, -1, 0, 0, 0, 0, -1, 0, 1, 0, 0, 0, -1, 0, -1, 0, 0, 1, 0, 0, 0, -1, 0, 0, -1, 0, 1, 0,
    1, 0, 0, 0,
  ];
  let mut expected = Vec::new();
  for digit in expected_loop_digits {
    expected.push(Bn254MillerScheduleStep::Double);
    match digit {
      1 => expected.push(Bn254MillerScheduleStep::Add(Bn254MillerAddend::Base)),
      -1 => expected.push(Bn254MillerScheduleStep::Add(Bn254MillerAddend::NegBase)),
      0 => {}
      _ => unreachable!("test digits are ternary"),
    }
  }
  expected.push(Bn254MillerScheduleStep::Add(Bn254MillerAddend::FrobeniusQ1));
  expected.push(Bn254MillerScheduleStep::Add(Bn254MillerAddend::FrobeniusQ2NegY));

  let schedule = Bn254MillerSchedule::bn254();

  assert_eq!(bn254_ate_loop_count().len(), 65);
  assert_eq!(schedule.steps, expected);
  assert_eq!(
    schedule.steps.iter().filter(|step| matches!(step, Bn254MillerScheduleStep::Double)).count(),
    64,
  );
}

#[test]
fn bn254_prepared_schedule_matches_arkworks_prepared_coeffs() {
  let g2_point = ArkG2Affine::generator();
  let expected = ark_bn254_prepared_miller_steps(g2_point);
  let schedule = Bn254MillerSchedule::bn254();

  assert_eq!(expected.len(), schedule.steps.len());
  assert!(matches!(expected.first(), Some(ArkMillerStep::Double(_))));
  assert!(matches!(expected.last(), Some(ArkMillerStep::Add(_))));
}

#[test]
#[ignore = "slow pairing-core"]
fn miller_loop_real_bn254_schedule_matches_arkworks_reference_on_generator() {
  let g1_point = ArkG1Affine::generator();
  let g2_point = ArkG2Affine::generator();
  let expected = ark_bn254_miller_loop_accumulate(g2_point, g1_point);

  assert_satisfied(&MillerLoopCircuit::new(
    (ark_to_midnight_fq(g1_point.x), ark_to_midnight_fq(g1_point.y)),
    ark_to_assigned_g2_coords(g2_point),
    &ark_to_midnight_fq12(&expected),
  ));
}

#[test]
#[ignore = "slow pairing-core"]
fn miller_loop_real_bn254_schedule_matches_arkworks_reference() {
  let mut rng = StdRng::from_seed([66_u8; 32]);
  let g1_point = random_nonzero_g1_affine(&mut rng);
  let g2_point = random_nonzero_g2_affine(&mut rng);
  let expected = ark_bn254_miller_loop_accumulate(g2_point, g1_point);

  assert_satisfied(&MillerLoopCircuit::new(
    (ark_to_midnight_fq(g1_point.x), ark_to_midnight_fq(g1_point.y)),
    ark_to_assigned_g2_coords(g2_point),
    &ark_to_midnight_fq12(&expected),
  ));
}

#[test]
#[ignore = "slow pairing-core"]
fn final_exponentiation_matches_arkworks_on_generator_miller_output() {
  let g1_point = ArkG1Affine::generator();
  let g2_point = ArkG2Affine::generator();
  let miller_output = ark_bn254_miller_loop_accumulate(g2_point, g1_point);
  let expected = ark_bn254_final_exponentiation(miller_output);

  assert_satisfied(&FinalExponentiationCircuit::new(
    &ark_to_midnight_fq12(&miller_output),
    &ark_to_midnight_fq12(&expected),
  ));
}

#[test]
#[ignore = "slow pairing-core"]
fn final_exponentiation_easy_part_sample_matches_host_decomposition() {
  assert_satisfied(&FinalExponentiationEasyPartCircuit::sample());
}

#[test]
#[ignore = "slow pairing-core"]
fn final_exponentiation_hard_part_sample_matches_host_decomposition() {
  assert_satisfied(&FinalExponentiationHardPartCircuit::sample());
}

#[test]
#[ignore = "slow pairing-core"]
fn final_exponentiation_matches_arkworks_on_deterministic_random_miller_outputs() {
  let mut rng = StdRng::from_seed([67_u8; 32]);

  for _ in 0..3 {
    let g1_point = random_nonzero_g1_affine(&mut rng);
    let g2_point = random_nonzero_g2_affine(&mut rng);
    let miller_output = ark_bn254_miller_loop_accumulate(g2_point, g1_point);
    let expected = ark_bn254_final_exponentiation(miller_output);

    assert_satisfied(&FinalExponentiationCircuit::new(
      &ark_to_midnight_fq12(&miller_output),
      &ark_to_midnight_fq12(&expected),
    ));
  }
}

#[test]
#[ignore = "slow pairing-core"]
fn miller_loop_then_final_exponentiation_matches_arkworks_pairing_on_generator() {
  let g1_point = ArkG1Affine::generator();
  let g2_point = ArkG2Affine::generator();
  let expected = ark_bn254_pairing(g1_point, g2_point);

  assert_satisfied(&PairingFinalExponentiationCircuit::new(
    (ark_to_midnight_fq(g1_point.x), ark_to_midnight_fq(g1_point.y)),
    ark_to_assigned_g2_coords(g2_point),
    &ark_to_midnight_fq12(&expected),
  ));
}

#[test]
#[ignore = "slow pairing-core"]
fn miller_loop_then_final_exponentiation_matches_arkworks_pairing() {
  let mut rng = StdRng::from_seed([68_u8; 32]);

  for _ in 0..2 {
    let g1_point = random_nonzero_g1_affine(&mut rng);
    let g2_point = random_nonzero_g2_affine(&mut rng);
    let expected = ark_bn254_pairing(g1_point, g2_point);

    assert_satisfied(&PairingFinalExponentiationCircuit::new(
      (ark_to_midnight_fq(g1_point.x), ark_to_midnight_fq(g1_point.y)),
      ark_to_assigned_g2_coords(g2_point),
      &ark_to_midnight_fq12(&expected),
    ));
  }
}


#[test]
fn multi_miller_loop_matches_product_of_individual_miller_outputs() {
  let g1 = ArkG1Affine::generator();
  let g2 = ArkG2Affine::generator();
  let two_g1 = (ArkG1Projective::generator() + ArkG1Projective::generator()).into_affine();
  let terms = [(g1, g2), (two_g1, g2)];
  let expected =
    ark_bn254_miller_loop_accumulate(g2, g1) * ark_bn254_miller_loop_accumulate(g2, two_g1);
  let actual = ark_bn254_multi_miller_loop_product(&terms);

  assert_eq!(actual, expected);
}

#[test]
#[ignore = "slow pairing-core"]
fn multi_miller_loop_two_term_circuit_matches_arkworks_reference() {
  let g1 = ArkG1Affine::generator();
  let g2 = ArkG2Affine::generator();
  let two_g1 = (ArkG1Projective::generator() + ArkG1Projective::generator()).into_affine();
  let terms = [(g1, g2), (two_g1, g2)];
  let expected = ark_bn254_multi_miller_loop_product(&terms);

  assert_satisfied(&MultiMillerLoopCircuit::new(&terms, &expected));
}

#[test]
#[ignore = "slow pairing-core"]
fn multi_miller_loop_three_term_circuit_matches_arkworks_reference() {
  let mut rng = StdRng::from_seed([71_u8; 32]);
  let q = random_nonzero_g2_affine(&mut rng);
  let p1 = random_nonzero_g1_affine(&mut rng);
  let p2 = random_nonzero_g1_affine(&mut rng);
  let p3 = (-(p1.into_group() + p2.into_group())).into_affine();
  let terms = [(p1, q), (p2, q), (p3, q)];
  let expected = ark_bn254_multi_miller_loop_product(&terms);

  assert_satisfied(&MultiMillerLoopCircuit::new(&terms, &expected));
}

#[test]
#[ignore = "slow pairing-core"]
fn pairing_check_one_term_matches_arkworks_negative_case() {
  let terms = [(ArkG1Affine::generator(), ArkG2Affine::generator())];
  assert!(!ark_bn254_pairing_check(&terms));

  let terms = ark_pairing_terms_to_constants(&terms);
  assert_satisfied(&PairingCheckCircuit::new(&terms, false));
}

#[test]
#[ignore = "slow pairing-core"]
fn pairing_check_two_term_inverse_cancellation_matches_arkworks() {
  let g2 = ArkG2Affine::generator();
  let g1 = ArkG1Affine::generator();
  let neg_g1 = (-ArkG1Projective::generator()).into_affine();
  let terms = [(g1, g2), (neg_g1, g2)];
  assert!(ark_bn254_pairing_check(&terms));
  assert_eq!(ark_bn254_pairing_product(&terms), ArkFq12::ONE);

  let terms = ark_pairing_terms_to_constants(&terms);
  assert_satisfied(&PairingCheckCircuit::new(&terms, true));
}

#[test]
#[ignore = "slow pairing-core"]
fn pairing_check_with_prepared_constant_terms_matches_arkworks() {
  let g2 = ArkG2Affine::generator();
  let g1 = ArkG1Affine::generator();
  let neg_g1 = (-ArkG1Projective::generator()).into_affine();
  let terms = [(g1, g2), (neg_g1, g2)];
  assert!(ark_bn254_pairing_check(&terms));

  let variable_terms =
    vec![((ark_to_midnight_fq(g1.x), ark_to_midnight_fq(g1.y)), ark_to_assigned_g2_coords(g2))];
  let prepared_terms = vec![(
    (ark_to_midnight_fq(neg_g1.x), ark_to_midnight_fq(neg_g1.y)),
    PreparedConstantG2Miller::from_affine_constant(ark_to_assigned_g2_coords(g2)),
  )];

  assert_satisfied(&PairingCheckCircuit::new_with_prepared_constant_terms(
    &variable_terms,
    &prepared_terms,
    true,
  ));
}

#[test]
#[ignore = "slow pairing-core"]
fn pairing_check_four_term_prepared_vk_style_matches_arkworks() {
  let g2 = ArkG2Affine::generator();
  let g1 = ArkG1Affine::generator();
  let neg_g1 = (-ArkG1Projective::generator()).into_affine();
  let terms = [(g1, g2), (neg_g1, g2), (g1, g2), (neg_g1, g2)];
  assert!(ark_bn254_pairing_check(&terms));

  let variable_terms =
    vec![((ark_to_midnight_fq(g1.x), ark_to_midnight_fq(g1.y)), ark_to_assigned_g2_coords(g2))];
  let prepared_terms = vec![
    (
      (ark_to_midnight_fq(neg_g1.x), ark_to_midnight_fq(neg_g1.y)),
      PreparedConstantG2Miller::from_affine_constant(ark_to_assigned_g2_coords(g2)),
    ),
    (
      (ark_to_midnight_fq(g1.x), ark_to_midnight_fq(g1.y)),
      PreparedConstantG2Miller::from_affine_constant(ark_to_assigned_g2_coords(g2)),
    ),
    (
      (ark_to_midnight_fq(neg_g1.x), ark_to_midnight_fq(neg_g1.y)),
      PreparedConstantG2Miller::from_affine_constant(ark_to_assigned_g2_coords(g2)),
    ),
  ];

  assert_satisfied(&PairingCheckCircuit::new_with_prepared_constant_terms(
    &variable_terms,
    &prepared_terms,
    true,
  ));
}

#[test]
#[ignore = "slow pairing-core"]
fn pairing_check_two_term_negative_matches_arkworks() {
  let g2 = ArkG2Affine::generator();
  let g1 = ArkG1Affine::generator();
  let two_g1 = (ArkG1Projective::generator() + ArkG1Projective::generator()).into_affine();
  let terms = [(g1, g2), (two_g1, g2)];
  assert!(!ark_bn254_pairing_check(&terms));

  let terms = ark_pairing_terms_to_constants(&terms);
  assert_satisfied(&PairingCheckCircuit::new(&terms, false));
}

#[test]
#[ignore = "slow pairing-core"]
fn pairing_check_three_term_cancellation_matches_arkworks() {
  let mut rng = StdRng::from_seed([69_u8; 32]);
  let q = random_nonzero_g2_affine(&mut rng);
  let p1 = random_nonzero_g1_affine(&mut rng);
  let p2 = random_nonzero_g1_affine(&mut rng);
  let p3 = (-(p1.into_group() + p2.into_group())).into_affine();
  let terms = [(p1, q), (p2, q), (p3, q)];
  assert!(ark_bn254_pairing_check(&terms));
  assert_eq!(ark_bn254_pairing_product(&terms), ArkFq12::ONE);

  let terms = ark_pairing_terms_to_constants(&terms);
  assert_satisfied(&PairingCheckCircuit::new(&terms, true));
}

#[test]
#[ignore = "slow pairing-core"]
fn pairing_check_three_term_negative_matches_arkworks() {
  let mut rng = StdRng::from_seed([70_u8; 32]);
  let q1 = random_nonzero_g2_affine(&mut rng);
  let q2 = random_nonzero_g2_affine(&mut rng);
  let p1 = random_nonzero_g1_affine(&mut rng);
  let p2 = random_nonzero_g1_affine(&mut rng);
  let p3 = random_nonzero_g1_affine(&mut rng);
  let terms = [(p1, q1), (p2, q1), (p3, q2)];
  assert!(!ark_bn254_pairing_check(&terms));

  let terms = ark_pairing_terms_to_constants(&terms);
  assert_satisfied(&PairingCheckCircuit::new(&terms, false));
}
