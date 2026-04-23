use ark_bn254::{
  Bn254 as ArkBn254, Fq as ArkFq, Fq2 as ArkFq2, Fq12 as ArkFq12, Fr as ArkFr,
  G1Affine as ArkG1Affine, G2Affine as ArkG2Affine,
};
use ark_ec::{AffineRepr, CurveGroup, pairing::Pairing};
use ark_ff::{Field as ArkField, PrimeField as ArkPrimeField};
use ff::PrimeField;
use midnight_curves::{CurveAffine, bn256::G1Affine};
use midnight_proofs::dev::MockProver;

use super::{
  Groth16Bn254G1Point, Groth16Bn254Proof, Groth16Bn254VerifierCircuit, Groth16Bn254VerifyingKey,
  Groth16IcAccumulatorCircuit, NativeField,
};
use crate::bn254::{ForeignCurve, ForeignField};

fn fq(value: &str) -> ForeignField {
  ForeignField::from_str_vartime(value).expect("fixture Fq element should parse")
}

fn fixture_vk() -> Groth16Bn254VerifyingKey {
  Groth16Bn254VerifyingKey {
    alpha_g1: Groth16Bn254G1Point::affine(fq("1"), fq("2")),
    beta_g2: (
      (
        fq("10857046999023057135944570762232829481370756359578518086990519993285655852781"),
        fq("11559732032986387107991004021392285783925812861821192530917403151452391805634"),
      ),
      (
        fq("8495653923123431417604973247489272438418190587263600148770280649306958101930"),
        fq("4082367875863433681332203403145435568316851327593401208105741076214120093531"),
      ),
    ),
    gamma_g2: (
      (
        fq("10857046999023057135944570762232829481370756359578518086990519993285655852781"),
        fq("11559732032986387107991004021392285783925812861821192530917403151452391805634"),
      ),
      (
        fq("8495653923123431417604973247489272438418190587263600148770280649306958101930"),
        fq("4082367875863433681332203403145435568316851327593401208105741076214120093531"),
      ),
    ),
    delta_g2: (
      (
        fq("10857046999023057135944570762232829481370756359578518086990519993285655852781"),
        fq("11559732032986387107991004021392285783925812861821192530917403151452391805634"),
      ),
      (
        fq("8495653923123431417604973247489272438418190587263600148770280649306958101930"),
        fq("4082367875863433681332203403145435568316851327593401208105741076214120093531"),
      ),
    ),
    ic: vec![
      Groth16Bn254G1Point::Identity,
      Groth16Bn254G1Point::affine(
        fq("1"),
        fq("21888242871839275222246405745257275088696311157297823662689037894645226208581"),
      ),
    ],
  }
}

fn fixture_proof() -> Groth16Bn254Proof {
  Groth16Bn254Proof {
    a: Groth16Bn254G1Point::affine(
      fq("1653059996313124324802471924921847871597694627520170958366082551667472867283"),
      fq("18696001991600901277024406088643158760693146181579651313528816019951170530131"),
    ),
    b: (
      (
        fq("10714359129198285705645341409989611527657170054061686203433452882792238857325"),
        fq("6963679642087473904956049809448511192920744881717080384977918961494368309799"),
      ),
      (
        fq("3108817312769526827106087159116729193745334557568354752209648560138552302731"),
        fq("1010747483848541377997082257808774665986863873371098715157683517578688720372"),
      ),
    ),
    c: Groth16Bn254G1Point::affine(
      fq("1230302483956234588333563686576036121802908159539686403289381495048101984285"),
      fq("12912843532200292943612900902015350563811804195931381861776211428416366913459"),
    ),
    public_inputs: vec![NativeField::from(33_u64)],
  }
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

fn ark_to_midnight_g1(point: ArkG1Affine) -> ForeignCurve {
  let affine = Option::<G1Affine>::from(G1Affine::from_xy(
    fq(&point.x.into_bigint().to_string()),
    fq(&point.y.into_bigint().to_string()),
  ))
  .expect("ark G1 point should map to a valid Midnight G1 point");

  affine.into()
}

fn ark_vk_x(vk: &Groth16Bn254VerifyingKey, proof: &Groth16Bn254Proof) -> ArkG1Affine {
  let mut accumulator = midnight_g1_to_ark(vk.ic[0]).into_group();

  for (scalar, ic_point) in proof.public_inputs.iter().zip(vk.ic.iter().skip(1)) {
    accumulator +=
      midnight_g1_to_ark(*ic_point).mul_bigint(midnight_to_ark_fr(*scalar).into_bigint());
  }

  accumulator.into_affine()
}

fn groth16_product_terms(
  vk: &Groth16Bn254VerifyingKey,
  proof: &Groth16Bn254Proof,
) -> [(ArkG1Affine, ArkG2Affine); 4] {
  let vk_x = ark_vk_x(vk, proof);

  [
    (midnight_g1_to_ark(proof.a), midnight_g2_to_ark(proof.b)),
    ((-midnight_g1_to_ark(vk.alpha_g1).into_group()).into_affine(), midnight_g2_to_ark(vk.beta_g2)),
    ((-vk_x.into_group()).into_affine(), midnight_g2_to_ark(vk.gamma_g2)),
    ((-midnight_g1_to_ark(proof.c).into_group()).into_affine(), midnight_g2_to_ark(vk.delta_g2)),
  ]
}

fn assert_satisfied<C: midnight_proofs::plonk::Circuit<NativeField>>(k: u32, circuit: &C) {
  let prover = MockProver::run(k, circuit, vec![vec![], vec![]]).expect("MockProver should build");
  assert_eq!(prover.verify(), Ok(()));
}

#[test]
fn groth16_ic_accumulator_matches_arkworks_reference() {
  let vk = fixture_vk();
  let proof = fixture_proof();
  let expected = ark_to_midnight_g1(ark_vk_x(&vk, &proof));

  assert_satisfied(
    14,
    &Groth16IcAccumulatorCircuit::new(vk, proof.public_inputs.clone(), expected),
  );
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
  let vk = fixture_vk();
  let proof = fixture_proof();
  let lhs = ArkBn254::pairing(midnight_g1_to_ark(proof.a), midnight_g2_to_ark(proof.b)).0;
  let rhs = ArkBn254::pairing(midnight_g1_to_ark(vk.alpha_g1), midnight_g2_to_ark(vk.beta_g2)).0
    * ArkBn254::pairing(ark_vk_x(&vk, &proof), midnight_g2_to_ark(vk.gamma_g2)).0
    * ArkBn254::pairing(midnight_g1_to_ark(proof.c), midnight_g2_to_ark(vk.delta_g2)).0;
  let product = groth16_product_terms(&vk, &proof)
    .into_iter()
    .fold(ArkFq12::ONE, |acc, (g1, g2)| acc * ArkBn254::pairing(g1, g2).0);

  assert_eq!(lhs, rhs);
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
  assert_satisfied(22, &Groth16Bn254VerifierCircuit::new(fixture_vk(), fixture_proof(), true));
}

#[test]
#[ignore = "slow pairing-core"]
fn slow_mockprover_invalid_public_input_mutation_is_rejected_end_to_end() {
  let mut proof = fixture_proof();
  proof.public_inputs[0] = NativeField::from(34_u64);

  assert_satisfied(22, &Groth16Bn254VerifierCircuit::new(fixture_vk(), proof, false));
}
