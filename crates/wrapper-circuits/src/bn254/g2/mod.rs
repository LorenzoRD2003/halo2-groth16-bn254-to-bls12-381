use ff::PrimeField;

use super::host::{Fp12Constant, Fp12Value};
use super::{
  AssignedFp, AssignedFp2, AssignedFp6, AssignedFp12, Bn254FieldChip, Bn254FieldConfig,
  ForeignField, NativeField,
  host::{
    Fp2Constant, Fp2Value, G2AffineConstant, G2LineCoeffsConstant, G2MillerPointConstant,
    G2ProjectiveConstant, bn254_final_exponentiation_constant, fp2_neg_constant, fp12_mul_constant,
    fp12_one_constant, fp12_square_constant, g1_generator_constant,
    g2_affine_from_miller_point_constant, g2_affine_from_projective_constant,
    g2_curve_coeff_b_constant, g2_line_evaluation_constant, g2_miller_double_with_line_constant,
    g2_miller_mixed_add_with_line_constant, g2_miller_point_from_affine_constant,
    g2_projective_add_constant, g2_projective_double_constant, g2_projective_from_affine_constant,
    g2_projective_identity_constant,
  },
};

mod affine;
mod jacobian;
mod miller;

pub use affine::{AssignedG2Affine, G2NegCircuit, G2OnCurveCircuit};
pub use jacobian::{
  AssignedG2Projective, G2ProjectiveAddCircuit, G2ProjectiveDoubleCircuit,
  G2ProjectiveFromAffineCircuit, G2ProjectiveIdentityCircuit, G2ProjectiveNegCircuit,
};
pub use miller::{
  AssignedG1Point, AssignedG2LineCoeffs, AssignedG2MillerPoint, AssignedMillerAccumulator,
  Bn254MillerAddend, Bn254MillerSchedule, Bn254MillerScheduleStep, FinalExponentiationCircuit,
  FinalExponentiationEasyPartCircuit, FinalExponentiationHardPartCircuit, G2DoubleWithLineCircuit,
  G2MixedAddWithLineCircuit, MillerAccumulatorMulByLineCircuit,
  MillerAccumulatorMulByLineSparseCircuit, MillerAccumulatorSquareCircuit, MillerLoopCircuit,
  MillerStep, MillerStepConstant, PairingCheckCircuit, PairingFinalExponentiationCircuit,
  PreparedConstantG2Miller, PreparedG2Miller, bn254_ate_loop_count, final_exponentiation,
  miller_loop, multi_miller_loop, pairing_check,
  pairing_check_with_prepared_terms_against_fixed_target_on_host,
};

type G2AffineValue = (Fp2Value, Fp2Value);
type G2LineCoeffsValue = (Fp2Value, Fp2Value, Fp2Value);

/// Returns the BN254 G2 twist coefficient `b = 3 / (u + 9)` in `Fq2(c0, c1)`.
///
/// # Panics
///
/// Panics if the hard-coded arkworks BN254 G2 twist coefficient fails to parse.
#[must_use]
pub fn g2_curve_coeff_b() -> (ForeignField, ForeignField) {
  g2_curve_coeff_b_constant()
}

fn g2_generator() -> G2AffineConstant {
  (
    (
      ForeignField::from_str_vartime(
        "10857046999023057135944570762232829481370756359578518086990519993285655852781",
      )
      .expect("hard-coded BN254 G2 generator x.c0 should parse"),
      ForeignField::from_str_vartime(
        "11559732032986387107991004021392285783925812861821192530917403151452391805634",
      )
      .expect("hard-coded BN254 G2 generator x.c1 should parse"),
    ),
    (
      ForeignField::from_str_vartime(
        "8495653923123431417604973247489272438418190587263600148770280649306958101930",
      )
      .expect("hard-coded BN254 G2 generator y.c0 should parse"),
      ForeignField::from_str_vartime(
        "4082367875863433681332203403145435568316851327593401208105741076214120093531",
      )
      .expect("hard-coded BN254 G2 generator y.c1 should parse"),
    ),
  )
}
