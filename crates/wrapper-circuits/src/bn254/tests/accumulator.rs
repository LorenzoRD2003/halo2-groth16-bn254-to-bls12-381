use super::*;

#[test]
fn g2_line_coeff_evaluation_matches_sparse_fp12_embedding() {
  let (_, g1_point, _, line, expected) = ark_generator_double_line_fixture();

  assert_satisfied(&MillerAccumulatorMulByLineSparseCircuit::new(
    ark_to_line_coeffs_constant(line),
    ark_to_midnight_fq(g1_point.x),
    ark_to_midnight_fq(g1_point.y),
    &ark_to_midnight_fq12(&expected),
  ));
}

#[test]
fn miller_accumulator_one_is_fp12_identity() {
  assert_satisfied(&MillerAccumulatorOneCircuit);
}

#[test]
fn miller_accumulator_square_matches_arkworks_reference() {
  let mut rng = ChaCha20Rng::from_seed([59_u8; 32]);

  for _ in 0..8 {
    let value = ArkFq12::rand(&mut rng);
    let expected = value.square();
    assert_satisfied(&MillerAccumulatorSquareCircuit::new(
      &ark_to_midnight_fq12(&value),
      &ark_to_midnight_fq12(&expected),
    ));
  }
}

#[test]
fn miller_accumulator_square_matches_fixed_generator_line_fixture() {
  let (_, _, _, _, value) = ark_generator_double_line_fixture();
  let expected = value.square();

  assert_satisfied(&MillerAccumulatorSquareCircuit::new(
    &ark_to_midnight_fq12(&value),
    &ark_to_midnight_fq12(&expected),
  ));
}

#[test]
fn miller_accumulator_mul_by_evaluated_line_matches_arkworks_reference() {
  let mut rng = ChaCha20Rng::from_seed([60_u8; 32]);

  for _ in 0..8 {
    let initial = ArkFq12::rand(&mut rng);
    let line_value = ArkFq12::rand(&mut rng);
    let expected = initial * line_value;

    assert_satisfied(&MillerAccumulatorMulByEvaluatedLineCircuit::new(
      &initial,
      &line_value,
      &expected,
    ));
  }
}

#[test]
fn miller_accumulator_mul_by_line_matches_arkworks_reference() {
  let mut rng = ChaCha20Rng::from_seed([61_u8; 32]);

  for _ in 0..8 {
    let g2_point = ArkG2Projective::rand(&mut rng).into_affine();
    let g1_point = ArkG1Projective::rand(&mut rng).into_affine();
    if g2_point.is_zero() || g1_point.is_zero() {
      continue;
    }

    let (_, line) = ark_double_with_line(ark_miller_point_from_affine(g2_point));
    let expected = ark_line_evaluation(line, g1_point);

    assert_satisfied(&MillerAccumulatorMulByLineSparseCircuit::new(
      ark_to_line_coeffs_constant(line),
      ark_to_midnight_fq(g1_point.x),
      ark_to_midnight_fq(g1_point.y),
      &ark_to_midnight_fq12(&expected),
    ));
  }
}

#[test]
fn miller_accumulator_mul_by_line_baseline_and_sparse_match_fixed_fixture() {
  let (_, g1_point, _, line, expected) = ark_generator_double_line_fixture();
  let line = ark_to_line_coeffs_constant(line);
  let g1_x = ark_to_midnight_fq(g1_point.x);
  let g1_y = ark_to_midnight_fq(g1_point.y);
  let expected = ark_to_midnight_fq12(&expected);

  assert_satisfied(&MillerAccumulatorMulByLineCircuit::new(line, g1_x, g1_y, &expected));
  assert_satisfied(&MillerAccumulatorMulByLineSparseCircuit::new(line, g1_x, g1_y, &expected));
}

#[test]
fn miller_accumulator_mul_by_line_baseline_and_sparse_match_randomized_fixtures() {
  let mut rng = ChaCha20Rng::from_seed([65_u8; 32]);

  for _ in 0..4 {
    let g2_point = ArkG2Projective::rand(&mut rng).into_affine();
    let g1_point = ArkG1Projective::rand(&mut rng).into_affine();
    if g2_point.is_zero() || g1_point.is_zero() {
      continue;
    }

    let (_, line) = ark_double_with_line(ark_miller_point_from_affine(g2_point));
    let expected = ark_line_evaluation(line, g1_point);
    let line = ark_to_line_coeffs_constant(line);
    let g1_x = ark_to_midnight_fq(g1_point.x);
    let g1_y = ark_to_midnight_fq(g1_point.y);
    let expected = ark_to_midnight_fq12(&expected);

    assert_satisfied(&MillerAccumulatorMulByLineCircuit::new(line, g1_x, g1_y, &expected));
    assert_satisfied(&MillerAccumulatorMulByLineSparseCircuit::new(line, g1_x, g1_y, &expected));
  }
}

#[test]
fn mixed_add_with_line_then_accumulate_matches_arkworks_reference() {
  let mut rng = ChaCha20Rng::from_seed([63_u8; 32]);

  for _ in 0..8 {
    let seed_point = ArkG2Projective::rand(&mut rng).into_affine();
    let addend = ArkG2Projective::rand(&mut rng).into_affine();
    let g1_point = ArkG1Projective::rand(&mut rng).into_affine();

    if seed_point.is_zero() || addend.is_zero() || g1_point.is_zero() {
      continue;
    }

    let doubled_state = ark_double_with_line(ark_miller_point_from_affine(seed_point)).0;
    let current_affine = ark_miller_point_to_affine(doubled_state);
    if addend == current_affine || addend == -current_affine {
      continue;
    }

    let (_, line) = ark_mixed_add_with_line(doubled_state, addend);
    let expected = ark_line_evaluation(line, g1_point);

    assert_satisfied(&MillerAccumulatorMulByLineSparseCircuit::new(
      ark_to_line_coeffs_constant(line),
      ark_to_midnight_fq(g1_point.x),
      ark_to_midnight_fq(g1_point.y),
      &ark_to_midnight_fq12(&expected),
    ));
  }
}

#[test]
fn miller_accumulator_sparse_and_generic_mul_by_line_paths_match_same_reference() {
  let g2_point = ArkG2Affine::generator();
  let g1_point = ArkG1Affine::generator();
  let (_, line) = ark_double_with_line(ark_miller_point_from_affine(g2_point));
  let expected = ark_line_evaluation(line, g1_point);
  let expected = ark_to_midnight_fq12(&expected);
  let line = ark_to_line_coeffs_constant(line);
  let g1_x = ark_to_midnight_fq(g1_point.x);
  let g1_y = ark_to_midnight_fq(g1_point.y);

  assert_satisfied(&MillerAccumulatorMulByLineCircuit::new(line, g1_x, g1_y, &expected));
  assert_satisfied(&MillerAccumulatorMulByLineSparseCircuit::new(line, g1_x, g1_y, &expected));
}

#[test]
fn g2_mixed_add_with_line_same_point_is_not_supported_in_this_slice() {
  let point = ArkG2Affine::generator();
  let current_state = ark_miller_point_from_affine(point);
  let honest_double = (point.into_group() + point).into_affine();
  let (_, honest_line) = ark_double_with_line(current_state);

  assert!(!prover_result(&G2MixedAddWithLineCircuit::new(
    ark_to_miller_point_constant(current_state),
    ark_to_assigned_g2_coords(point),
    ark_to_assigned_g2_coords(honest_double),
    ark_to_line_coeffs_constant(honest_line),
  )));
}

#[test]
fn g2_mixed_add_with_line_inverse_point_is_not_supported_in_this_slice() {
  let point = ArkG2Affine::generator();
  let current_state = ark_miller_point_from_affine(point);
  let unsupported_addend = -point;
  let (_, honest_line) = ark_double_with_line(current_state);

  assert!(!prover_result(&G2MixedAddWithLineCircuit::new(
    ark_to_miller_point_constant(current_state),
    ark_to_assigned_g2_coords(unsupported_addend),
    ark_to_assigned_g2_coords(point),
    ark_to_line_coeffs_constant(honest_line),
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
  let double_with_line_metrics = g2_double_with_line_layout_metrics();
  let mixed_add_with_line_metrics = g2_mixed_add_with_line_layout_metrics();
  let accumulator_square_metrics = miller_accumulator_square_layout_metrics();
  let accumulator_mul_by_line_metrics = miller_accumulator_mul_by_line_layout_metrics();
  let accumulator_mul_by_line_sparse_metrics =
    miller_accumulator_mul_by_line_sparse_layout_metrics();
  let miller_loop_metrics = miller_loop_layout_metrics();
  let final_exponentiation_metrics = final_exponentiation_layout_metrics();
  let pairing_check_metrics = pairing_check_layout_metrics();

  assert!(on_curve_metrics.rows > 0);
  assert!(neg_metrics.rows > 0);
  assert!(from_affine_metrics.rows > 0);
  assert!(double_metrics.rows > 0);
  assert!(add_metrics.rows > 0);
  assert!(double_with_line_metrics.rows > 0);
  assert!(mixed_add_with_line_metrics.rows > 0);
  assert!(accumulator_square_metrics.rows > 0);
  assert!(accumulator_mul_by_line_metrics.rows > 0);
  assert!(accumulator_mul_by_line_sparse_metrics.rows > 0);
  assert!(miller_loop_metrics.rows > 0);
  assert!(final_exponentiation_metrics.rows > 0);
  assert!(pairing_check_metrics.rows > 0);
  assert!(on_curve_metrics.column_queries > 0);
  assert!(neg_metrics.column_queries > 0);
  assert!(from_affine_metrics.column_queries > 0);
  assert!(double_metrics.column_queries > 0);
  assert!(add_metrics.column_queries > 0);
  assert!(double_with_line_metrics.column_queries > 0);
  assert!(mixed_add_with_line_metrics.column_queries > 0);
  assert!(accumulator_square_metrics.column_queries > 0);
  assert!(accumulator_mul_by_line_metrics.column_queries > 0);
  assert!(accumulator_mul_by_line_sparse_metrics.column_queries > 0);
  assert!(miller_loop_metrics.column_queries > 0);
  assert!(final_exponentiation_metrics.column_queries > 0);
  assert!(pairing_check_metrics.column_queries > 0);
  assert!(accumulator_mul_by_line_sparse_metrics.rows < accumulator_mul_by_line_metrics.rows);
}
