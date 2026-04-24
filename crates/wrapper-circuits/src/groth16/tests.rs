use ark_bn254::{Fq12 as ArkFq12, G1Affine as ArkG1Affine};
use ark_ec::AffineRepr;
use ark_ff::Field as ArkField;
use midnight_proofs::dev::MockProver;

use crate::test_support::ark_to_midnight_g1;

use super::{
  Groth16Bn254VerifierCircuit, Groth16IcAccumulatorCircuit, NativeField,
  fixtures::typed::{
    proof as fixture_proof, public_inputs as fixture_public_inputs, verifying_key as fixture_vk,
  },
  reference::{host_pairing_product, host_public_input_accumulator},
};

fn assert_satisfied<C: midnight_proofs::plonk::Circuit<NativeField>>(k: u32, circuit: &C) {
  let prover = MockProver::run(k, circuit, vec![vec![], vec![]]).expect("MockProver should build");
  assert_eq!(prover.verify(), Ok(()));
}

#[test]
fn groth16_ic_accumulator_matches_arkworks_reference() {
  let vk = fixture_vk();
  let public_inputs = fixture_public_inputs();
  let expected = ark_to_midnight_g1(host_public_input_accumulator(&vk, &public_inputs));

  assert_satisfied(14, &Groth16IcAccumulatorCircuit::new(vk, public_inputs, expected));
}

#[test]
fn groth16_ic_accumulator_rejects_public_input_length_mismatch() {
  let circuit = Groth16IcAccumulatorCircuit::new(
    fixture_vk(),
    Vec::new(),
    ark_to_midnight_g1(ArkG1Affine::generator()),
  );

  let result = MockProver::run(14, &circuit, vec![vec![], vec![]]);
  assert!(result.is_err(), "length mismatch should fail during synthesis");
}

#[test]
fn groth16_pairing_product_encoding_matches_arkworks_verifier_relation() {
  let product = host_pairing_product(&fixture_vk(), &fixture_proof(), &fixture_public_inputs());

  assert_eq!(product, ArkFq12::ONE);
}

// These full-circuit MockProver checks are kept as explicit slow integration
// tests because the Week 5 pairing-backed verifier circuit is still too heavy
// for the default local lane. Always-run end-to-end acceptance/rejection lives
// in `wrapper-tests`, while these remain the highest-fidelity circuit stress
// checks for deliberate slow-lane execution.
#[test]
#[ignore = "slow pairing-core"]
fn slow_mockprover_valid_real_fixture_is_accepted_end_to_end() {
  assert_satisfied(
    22,
    &Groth16Bn254VerifierCircuit::new(fixture_vk(), fixture_proof(), fixture_public_inputs(), true),
  );
}

#[test]
#[ignore = "slow pairing-core"]
fn slow_mockprover_invalid_public_input_mutation_is_rejected_end_to_end() {
  let mut public_inputs = fixture_public_inputs();
  public_inputs[0] = NativeField::from(34_u64);

  assert_satisfied(
    22,
    &Groth16Bn254VerifierCircuit::new(fixture_vk(), fixture_proof(), public_inputs, false),
  );
}
