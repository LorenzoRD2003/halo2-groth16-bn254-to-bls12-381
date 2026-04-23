//! Test harness crate for workspace-level fixtures and integration helpers.
#![allow(clippy::multiple_crate_versions)]

use wrapper_backends as _;
use wrapper_circuits as _;
use wrapper_core as _;

#[cfg(test)]
use criterion as _;
#[cfg(test)]
use midnight_proofs as _;

/// Returns the example config bundled for integration tests.
#[must_use]
pub fn example_config() -> &'static str {
  include_str!("../fixtures/example-config.toml")
}

#[cfg(test)]
mod tests {
  use ark_bn254::{
    Bn254 as ArkBn254, Fq as ArkFq, Fq12 as ArkFq12, Fq2 as ArkFq2, Fr as ArkFr,
    G1Affine as ArkG1Affine, G2Affine as ArkG2Affine,
  };
  use ark_ec::{AffineRepr, CurveGroup, pairing::Pairing};
  use ark_ff::{Field as ArkField, PrimeField as ArkPrimeField};
  use ff::PrimeField;
  use wrapper_backends::BackendRegistry;
  use wrapper_backends::{
    parse_groth16_bn254_proof_with_public_inputs, parse_groth16_bn254_verifying_key,
  };
  use wrapper_circuits::{
    CircuitPlanningView, ForeignField, Groth16Bn254G1Point, Groth16Bn254Proof,
    Groth16Bn254VerifyingKey, NativeField,
  };
  use wrapper_core::ProjectConfig;

  use super::example_config;

  #[test]
  fn example_config_parses() {
    let config = ProjectConfig::from_toml_str(example_config()).expect("config should parse");
    let layout = CircuitPlanningView::from_config(config).describe();

    assert_eq!(layout.name, "wrapper-scaffold");
  }

  #[test]
  fn backend_registry_contains_placeholders() {
    let registry = BackendRegistry::scaffold();

    assert_eq!(registry.entries().len(), 2);
  }

  fn load_groth16_fixture() -> (Groth16Bn254VerifyingKey, Groth16Bn254Proof) {
    let vk = parse_groth16_bn254_verifying_key(include_bytes!(
      "../fixtures/groth16/circom_multiplier2/verification_key.json"
    ))
    .expect("fixture vk should parse");
    let proof = parse_groth16_bn254_proof_with_public_inputs(
      include_bytes!("../fixtures/groth16/circom_multiplier2/proof.json"),
      include_bytes!("../fixtures/groth16/circom_multiplier2/public.json"),
    )
    .expect("fixture proof should parse");

    (vk, proof)
  }

  fn midnight_to_ark_fq(value: ForeignField) -> ArkFq {
    ArkFq::from_le_bytes_mod_order(value.to_repr().as_ref())
  }

  fn midnight_to_ark_fr(value: NativeField) -> ArkFr {
    ArkFr::from_le_bytes_mod_order(value.to_repr().as_ref())
  }

  fn midnight_g1_to_ark(point: Groth16Bn254G1Point) -> ArkG1Affine {
    match point {
      Groth16Bn254G1Point::Identity => ArkG1Affine::identity(),
      Groth16Bn254G1Point::Affine { x, y } => {
        ArkG1Affine::new_unchecked(midnight_to_ark_fq(x), midnight_to_ark_fq(y))
      }
    }
  }

  fn midnight_g2_to_ark(
    point: ((ForeignField, ForeignField), (ForeignField, ForeignField)),
  ) -> ArkG2Affine {
    ArkG2Affine::new_unchecked(
      ArkFq2::new(midnight_to_ark_fq((point.0).0), midnight_to_ark_fq((point.0).1)),
      ArkFq2::new(midnight_to_ark_fq((point.1).0), midnight_to_ark_fq((point.1).1)),
    )
  }

  fn host_vk_x(vk: &Groth16Bn254VerifyingKey, proof: &Groth16Bn254Proof) -> ArkG1Affine {
    let mut accumulator = midnight_g1_to_ark(vk.ic[0]).into_group();

    for (scalar, ic_point) in proof.public_inputs.iter().zip(vk.ic.iter().skip(1)) {
      accumulator +=
        midnight_g1_to_ark(*ic_point).mul_bigint(midnight_to_ark_fr(*scalar).into_bigint());
    }

    accumulator.into_affine()
  }

  fn host_groth16_product(vk: &Groth16Bn254VerifyingKey, proof: &Groth16Bn254Proof) -> ArkFq12 {
    let vk_x = host_vk_x(vk, proof);
    let terms = [
      (midnight_g1_to_ark(proof.a), midnight_g2_to_ark(proof.b)),
      (
        (-midnight_g1_to_ark(vk.alpha_g1).into_group()).into_affine(),
        midnight_g2_to_ark(vk.beta_g2),
      ),
      ((-vk_x.into_group()).into_affine(), midnight_g2_to_ark(vk.gamma_g2)),
      (
        (-midnight_g1_to_ark(proof.c).into_group()).into_affine(),
        midnight_g2_to_ark(vk.delta_g2),
      ),
    ];

    terms
      .into_iter()
      .fold(ArkFq12::ONE, |acc, (g1, g2)| acc * ArkBn254::pairing(g1, g2).0)
  }

  fn host_groth16_verify(vk: &Groth16Bn254VerifyingKey, proof: &Groth16Bn254Proof) -> bool {
    if proof.public_inputs.len() + 1 != vk.ic.len() {
      return false;
    }

    host_groth16_product(vk, proof) == ArkFq12::ONE
  }

  #[test]
  fn groth16_real_snarkjs_fixture_is_accepted_end_to_end() {
    let (vk, proof) = load_groth16_fixture();

    assert!(host_groth16_verify(&vk, &proof));
  }

  #[test]
  fn groth16_mutated_public_input_is_rejected_end_to_end() {
    let (vk, mut proof) = load_groth16_fixture();
    proof.public_inputs[0] = NativeField::from(34_u64);

    assert!(!host_groth16_verify(&vk, &proof));
  }
}
