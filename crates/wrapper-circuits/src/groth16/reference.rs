//! Host-side Groth16 BN254 reference helpers shared by verifier tests.
//!
//! This module intentionally stays on the host/reference side. It mirrors the
//! narrow verifier equation with arkworks so tests can cross-check the circuit
//! path without re-implementing the algebra in multiple places.

use ark_bn254::{
  Bn254 as ArkBn254, Fq as ArkFq, Fq2 as ArkFq2, Fq12 as ArkFq12, Fr as ArkFr,
  G1Affine as ArkG1Affine, G2Affine as ArkG2Affine,
};
use ark_ec::{AffineRepr, CurveGroup, pairing::Pairing};
use ark_ff::{Field as ArkField, PrimeField as ArkPrimeField};

use super::{Groth16Bn254G1Point, Groth16Bn254Proof, Groth16Bn254VerifyingKey};
use crate::bn254::{ForeignField, NativeField};
use crate::test_support::{
  midnight_to_ark_fq as shared_midnight_to_ark_fq, midnight_to_ark_fr as shared_midnight_to_ark_fr,
};

/// Converts a Midnight BN254 base-field element into arkworks `Fq`.
#[must_use]
pub(crate) fn midnight_to_ark_fq(value: ForeignField) -> ArkFq {
  shared_midnight_to_ark_fq(value)
}

/// Converts a Midnight native field element into arkworks `Fr`.
#[must_use]
pub(crate) fn midnight_to_ark_fr(value: NativeField) -> ArkFr {
  shared_midnight_to_ark_fr(value)
}

/// Converts the narrow Groth16 G1 encoding into arkworks affine form.
#[must_use]
pub(crate) fn groth16_g1_to_ark(point: Groth16Bn254G1Point) -> ArkG1Affine {
  match point {
    Groth16Bn254G1Point::Identity => ArkG1Affine::identity(),
    Groth16Bn254G1Point::Affine { x, y } => {
      ArkG1Affine::new_unchecked(midnight_to_ark_fq(x), midnight_to_ark_fq(y))
    }
  }
}

/// Converts the narrow Groth16 G2 encoding into arkworks affine form.
#[must_use]
pub(crate) fn groth16_g2_to_ark(
  point: ((ForeignField, ForeignField), (ForeignField, ForeignField)),
) -> ArkG2Affine {
  ArkG2Affine::new_unchecked(
    ArkFq2::new(midnight_to_ark_fq((point.0).0), midnight_to_ark_fq((point.0).1)),
    ArkFq2::new(midnight_to_ark_fq((point.1).0), midnight_to_ark_fq((point.1).1)),
  )
}

/// Computes the verifier-side host accumulator `vk_x`.
#[must_use]
pub(crate) fn host_public_input_accumulator(
  vk: &Groth16Bn254VerifyingKey,
  public_inputs: &[NativeField],
) -> ArkG1Affine {
  let mut accumulator = groth16_g1_to_ark(vk.ic[0]).into_group();

  for (scalar, ic_point) in public_inputs.iter().zip(vk.ic.iter().skip(1)) {
    accumulator +=
      groth16_g1_to_ark(*ic_point).mul_bigint(midnight_to_ark_fr(*scalar).into_bigint());
  }

  accumulator.into_affine()
}

/// Builds the host-side Groth16 verifier pairing product.
#[must_use]
pub(crate) fn host_pairing_product(
  vk: &Groth16Bn254VerifyingKey,
  proof: &Groth16Bn254Proof,
  public_inputs: &[NativeField],
) -> ArkFq12 {
  let vk_x = host_public_input_accumulator(vk, public_inputs);
  let terms = [
    (groth16_g1_to_ark(proof.a), groth16_g2_to_ark(proof.b)),
    ((-groth16_g1_to_ark(vk.alpha_g1).into_group()).into_affine(), groth16_g2_to_ark(vk.beta_g2)),
    ((-vk_x.into_group()).into_affine(), groth16_g2_to_ark(vk.gamma_g2)),
    ((-groth16_g1_to_ark(proof.c).into_group()).into_affine(), groth16_g2_to_ark(vk.delta_g2)),
  ];

  terms.into_iter().fold(ArkFq12::ONE, |acc, (g1, g2)| acc * ArkBn254::pairing(g1, g2).0)
}

/// Verifies the narrow Groth16 relation on the host side using arkworks.
#[must_use]
pub fn host_verify(
  vk: &Groth16Bn254VerifyingKey,
  proof: &Groth16Bn254Proof,
  public_inputs: &[NativeField],
) -> bool {
  if public_inputs.len() + 1 != vk.ic.len() {
    return false;
  }

  host_pairing_product(vk, proof, public_inputs) == ArkFq12::ONE
}
