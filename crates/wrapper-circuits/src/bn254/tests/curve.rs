use super::*;

fn assert_mixed_add_with_line_matches_expected(
  doubled_state: ArkG2MillerPoint,
  addend: ArkG2Affine,
  expected_point: ArkG2Affine,
  line: ArkG2LineCoeffs,
) {
  assert_satisfied(&G2MixedAddWithLineCircuit::new(
    ark_to_miller_point_constant(doubled_state),
    ark_to_assigned_g2_coords(addend),
    ark_to_assigned_g2_coords(expected_point),
    ark_to_line_coeffs_constant(line),
  ));
}

#[test]
fn g1_addition_matches_arkworks() {
  let mut rng = StdRng::from_seed([31_u8; 32]);

  for _ in 0..8 {
    let left = ArkG1Projective::rand(&mut rng).into_affine();
    let right = ArkG1Projective::rand(&mut rng).into_affine();

    let circuit = G1AddCircuit::new(ark_to_midnight_g1(left), ark_to_midnight_g1(right));
    assert!(prover_result(&circuit));
  }
}

#[test]
fn g1_doubling_works_via_addition() {
  let mut rng = StdRng::from_seed([32_u8; 32]);

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
  let mut rng = StdRng::from_seed([51_u8; 32]);

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
  let mut rng = StdRng::from_seed([52_u8; 32]);

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
  let mut rng = StdRng::from_seed([53_u8; 32]);

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
  let mut rng = StdRng::from_seed([54_u8; 32]);

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
  let mut rng = StdRng::from_seed([55_u8; 32]);

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
  let mut rng = StdRng::from_seed([56_u8; 32]);

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
fn g2_double_with_line_matches_arkworks_reference() {
  let mut rng = StdRng::from_seed([57_u8; 32]);

  for _ in 0..8 {
    let point = ArkG2Projective::rand(&mut rng).into_affine();
    if point.is_zero() {
      continue;
    }

    let (next_point, line) = ark_double_with_line(ark_miller_point_from_affine(point));
    let expected_point = ark_miller_point_to_affine(next_point);

    assert_satisfied(&G2DoubleWithLineCircuit::new(
      ark_to_assigned_g2_coords(point),
      ark_to_assigned_g2_coords(expected_point),
      ark_to_line_coeffs_constant(line),
    ));
  }
}

#[test]
fn g2_double_with_line_matches_fixed_generator_fixture() {
  let (g2_point, _, next_state, line, _) = ark_generator_double_line_fixture();
  let expected_point = ark_miller_point_to_affine(next_state);

  assert_satisfied(&G2DoubleWithLineCircuit::new(
    ark_to_assigned_g2_coords(g2_point),
    ark_to_assigned_g2_coords(expected_point),
    ark_to_line_coeffs_constant(line),
  ));
}

#[test]
fn g2_mixed_add_with_line_matches_arkworks_reference() {
  let mut rng = StdRng::from_seed([58_u8; 32]);

  for _ in 0..8 {
    let (_seed_point, addend, doubled_state) = random_supported_mixed_add_fixture(&mut rng);

    let (next_point, line) = ark_mixed_add_with_line(doubled_state, addend);
    let expected_point = ark_miller_point_to_affine(next_point);

    assert_mixed_add_with_line_matches_expected(doubled_state, addend, expected_point, line);
  }
}

#[test]
fn g2_mixed_add_with_line_matches_fixed_generator_fixture() {
  let (g2_point, _, doubled_state, _, add_line, _) = ark_generator_double_add_fixture();
  let (next_state, _) = ark_mixed_add_with_line(doubled_state, g2_point);
  let expected_point = ark_miller_point_to_affine(next_state);

  assert_mixed_add_with_line_matches_expected(doubled_state, g2_point, expected_point, add_line);
}
