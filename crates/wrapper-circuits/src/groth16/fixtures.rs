//! Canonical Week 5 Groth16 fixture support shared across test crates.
//!
//! The fixture is split by responsibility:
//! - `raw` exposes the committed snarkjs artifacts
//! - `typed` exposes the parsed narrow BN254 values used by tests

use ff::PrimeField;

use super::{Groth16Bn254G1Point, Groth16Bn254Proof, Groth16Bn254VerifyingKey};
use crate::bn254::{ForeignField, NativeField};

fn fq(value: &str) -> ForeignField {
  ForeignField::from_str_vartime(value).expect("fixture Fq element should parse")
}

/// Raw committed snarkjs artifacts for the canonical Week 5 fixture.
pub mod raw {
  /// Returns the committed raw snarkjs proof JSON for the canonical Week 5 fixture.
  #[must_use]
  #[cfg_attr(test, allow(dead_code))]
  pub fn proof_json() -> &'static [u8] {
    include_bytes!("../../../wrapper-tests/fixtures/groth16/circom_multiplier2/proof.json")
  }

  /// Returns the committed raw snarkjs public-input JSON for the canonical Week 5 fixture.
  #[must_use]
  #[cfg_attr(test, allow(dead_code))]
  pub fn public_inputs_json() -> &'static [u8] {
    include_bytes!("../../../wrapper-tests/fixtures/groth16/circom_multiplier2/public.json")
  }

  /// Returns the committed raw snarkjs verification-key JSON for the canonical Week 5 fixture.
  #[must_use]
  #[cfg_attr(test, allow(dead_code))]
  pub fn verification_key_json() -> &'static [u8] {
    include_bytes!(
      "../../../wrapper-tests/fixtures/groth16/circom_multiplier2/verification_key.json"
    )
  }
}

/// Typed narrow BN254 values for the canonical Week 5 fixture.
pub mod typed {
  use super::{Groth16Bn254G1Point, Groth16Bn254Proof, Groth16Bn254VerifyingKey, NativeField, fq};

  /// Returns the typed narrow Groth16 BN254 proof for the canonical Week 5 fixture.
  #[must_use]
  pub fn proof() -> Groth16Bn254Proof {
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
    }
  }

  /// Returns the typed narrow Groth16 BN254 verification key for the canonical Week 5 fixture.
  #[must_use]
  pub fn verifying_key() -> Groth16Bn254VerifyingKey {
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

  /// Returns the typed public-input vector for the canonical Week 5 fixture.
  #[must_use]
  pub fn public_inputs() -> Vec<NativeField> {
    vec![NativeField::from(33_u64)]
  }
}
