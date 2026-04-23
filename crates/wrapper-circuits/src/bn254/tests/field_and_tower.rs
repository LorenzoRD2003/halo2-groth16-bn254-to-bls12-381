use super::*;

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
fn fp12_cyclotomic_square_host_matches_generic_square_on_random_cyclotomic_elements() {
  let mut rng = ChaCha20Rng::from_seed([74_u8; 32]);

  for _ in 0..6 {
    let g1 = random_nonzero_g1_affine(&mut rng);
    let g2 = random_nonzero_g2_affine(&mut rng);
    let miller_output = ark_bn254_miller_loop_accumulate(g2, g1);
    let cyclotomic = super::super::host::bn254_final_exponentiation_easy_part_constant(
      &ark_to_midnight_fq12(&miller_output),
    );

    assert_eq!(
      super::super::host::fp12_cyclotomic_square_constant(&cyclotomic),
      super::super::host::fp12_square_constant(&cyclotomic),
    );
  }
}

#[test]
fn fp12_cyclotomic_square_circuit_matches_host_on_random_cyclotomic_elements() {
  let mut rng = ChaCha20Rng::from_seed([75_u8; 32]);

  for _ in 0..6 {
    let g1 = random_nonzero_g1_affine(&mut rng);
    let g2 = random_nonzero_g2_affine(&mut rng);
    let miller_output = ark_bn254_miller_loop_accumulate(g2, g1);
    let cyclotomic = super::super::host::bn254_final_exponentiation_easy_part_constant(
      &ark_to_midnight_fq12(&miller_output),
    );

    assert_satisfied(&Fp12CyclotomicSquareCircuit::new(cyclotomic));
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
  let cyclotomic_square_metrics = fp12_cyclotomic_square_layout_metrics();

  assert!(add_metrics.rows > 0);
  assert!(mul_metrics.rows > 0);
  assert!(square_metrics.rows > 0);
  assert!(cyclotomic_square_metrics.rows > 0);
  assert!(add_metrics.column_queries > 0);
  assert!(mul_metrics.column_queries > 0);
  assert!(square_metrics.column_queries > 0);
  assert!(cyclotomic_square_metrics.column_queries > 0);
  assert!(cyclotomic_square_metrics.rows < square_metrics.rows);

  println!(
    "cyclotomic_square: {} rows vs generic_square: {} rows",
    cyclotomic_square_metrics.rows, square_metrics.rows
  );
}
