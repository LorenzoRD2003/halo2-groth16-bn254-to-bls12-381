//! Narrow Circom / snarkjs JSON parsing for the Week 5 Groth16 BN254 slice.

use ff::PrimeField;
use serde::Deserialize;
use thiserror::Error;
use wrapper_core::{NamedPublicInput, NamedPublicInputs};
use wrapper_circuits::{
  ForeignField, Groth16Bn254G1Point, Groth16Bn254Proof, Groth16Bn254VerifyingKey, NativeField,
};

type G2AffineCoordinates = ((ForeignField, ForeignField), (ForeignField, ForeignField));

/// Errors raised while parsing narrow snarkjs Groth16 BN254 artifacts.
#[derive(Debug, Error)]
pub enum SnarkjsGroth16ParseError {
  /// The JSON payload is not valid for the expected artifact kind.
  #[error("failed to parse snarkjs {artifact} JSON: {source}")]
  Json {
    /// Which snarkjs artifact was being decoded.
    artifact: &'static str,
    #[source]
    /// The underlying serde JSON decoding error.
    source: serde_json::Error,
  },
  /// The artifact protocol is not the supported Groth16 shape.
  #[error("expected snarkjs protocol 'groth16', got '{0}'")]
  UnsupportedProtocol(String),
  /// The artifact curve is not the narrow BN254 / bn128 target.
  #[error("expected snarkjs curve 'bn128' or 'bn254', got '{0}'")]
  UnsupportedCurve(String),
  /// A field element failed to parse as a decimal BN254 value.
  #[error("invalid {field_kind} decimal field element '{value}'")]
  InvalidFieldElement {
    /// Which BN254 field was expected at this position.
    field_kind: &'static str,
    /// The original decimal string that failed to parse.
    value: String,
  },
  /// A point shape or affine encoding is unsupported for this narrow slice.
  #[error("invalid {point_kind} encoding: {reason}")]
  InvalidPointEncoding {
    /// Which proof or VK point was being parsed.
    point_kind: &'static str,
    /// Human-readable explanation of the rejected shape.
    reason: String,
  },
  /// The VK public-input metadata is inconsistent with the IC table.
  #[error(
    "verification key nPublic mismatch: expected IC length {expected_ic_len}, got {actual_ic_len}"
  )]
  InvalidIcLength {
    /// The IC length implied by `nPublic`.
    expected_ic_len: usize,
    /// The IC length actually present in the JSON artifact.
    actual_ic_len: usize,
  },
  /// A structured statement parser expected a fixed public-input count.
  #[error("expected {context} to expose {expected} public inputs, got {actual}")]
  InvalidPublicInputLength {
    /// Which higher-level statement shape rejected the vector.
    context: &'static str,
    /// Expected number of public inputs.
    expected: usize,
    /// Actual number of parsed public inputs.
    actual: usize,
  },
}

#[derive(Deserialize)]
struct SnarkjsProofJson {
  protocol: String,
  curve: String,
  pi_a: Vec<String>,
  pi_b: Vec<Vec<String>>,
  pi_c: Vec<String>,
}

#[derive(Deserialize)]
struct SnarkjsVerificationKeyJson {
  protocol: String,
  curve: String,
  #[serde(rename = "nPublic")]
  n_public: usize,
  #[serde(rename = "vk_alpha_1")]
  vk_alpha_1: Vec<String>,
  #[serde(rename = "vk_beta_2")]
  vk_beta_2: Vec<Vec<String>>,
  #[serde(rename = "vk_gamma_2")]
  vk_gamma_2: Vec<Vec<String>>,
  #[serde(rename = "vk_delta_2")]
  vk_delta_2: Vec<Vec<String>>,
  #[serde(rename = "IC")]
  ic: Vec<Vec<String>>,
}

fn ensure_groth16_bn254(protocol: &str, curve: &str) -> Result<(), SnarkjsGroth16ParseError> {
  if protocol != "groth16" {
    return Err(SnarkjsGroth16ParseError::UnsupportedProtocol(protocol.to_owned()));
  }

  if curve != "bn128" && curve != "bn254" {
    return Err(SnarkjsGroth16ParseError::UnsupportedCurve(curve.to_owned()));
  }

  Ok(())
}

fn parse_foreign_field(value: &str) -> Result<ForeignField, SnarkjsGroth16ParseError> {
  ForeignField::from_str_vartime(value).ok_or_else(|| {
    SnarkjsGroth16ParseError::InvalidFieldElement { field_kind: "Fq", value: value.to_owned() }
  })
}

fn parse_native_field(value: &str) -> Result<NativeField, SnarkjsGroth16ParseError> {
  NativeField::from_str_vartime(value).ok_or_else(|| {
    SnarkjsGroth16ParseError::InvalidFieldElement { field_kind: "Fr", value: value.to_owned() }
  })
}

fn parse_g1_point(
  point_kind: &'static str,
  coords: &[String],
) -> Result<Groth16Bn254G1Point, SnarkjsGroth16ParseError> {
  if coords.len() != 3 {
    return Err(SnarkjsGroth16ParseError::InvalidPointEncoding {
      point_kind,
      reason: format!("expected projective [x, y, z], got length {}", coords.len()),
    });
  }

  match coords[2].as_str() {
    "1" => Ok(Groth16Bn254G1Point::affine(
      parse_foreign_field(&coords[0])?,
      parse_foreign_field(&coords[1])?,
    )),
    "0" if coords[0] == "0" && coords[1] == "1" => Ok(Groth16Bn254G1Point::Identity),
    other => Err(SnarkjsGroth16ParseError::InvalidPointEncoding {
      point_kind,
      reason: format!(
        "expected affine z = 1 or the snarkjs G1 identity [0, 1, 0], got z = {}",
        other
      ),
    }),
  }
}

fn parse_affine_g2(
  point_kind: &'static str,
  coords: &[Vec<String>],
) -> Result<G2AffineCoordinates, SnarkjsGroth16ParseError> {
  if coords.len() != 3 {
    return Err(SnarkjsGroth16ParseError::InvalidPointEncoding {
      point_kind,
      reason: format!("expected [[x.c0, x.c1], [y.c0, y.c1], [1, 0]], got length {}", coords.len()),
    });
  }

  for (index, component) in coords.iter().take(2).enumerate() {
    if component.len() != 2 {
      return Err(SnarkjsGroth16ParseError::InvalidPointEncoding {
        point_kind,
        reason: format!(
          "expected Fq2 component {} to have length 2, got {}",
          index,
          component.len()
        ),
      });
    }
  }

  if coords[2].len() != 2 || coords[2][0] != "1" || coords[2][1] != "0" {
    return Err(SnarkjsGroth16ParseError::InvalidPointEncoding {
      point_kind,
      reason: "expected affine Fq2 z = [1, 0]".to_owned(),
    });
  }

  Ok((
    (parse_foreign_field(&coords[0][0])?, parse_foreign_field(&coords[0][1])?),
    (parse_foreign_field(&coords[1][0])?, parse_foreign_field(&coords[1][1])?),
  ))
}

/// Parses a snarkjs Groth16 proof plus public-input array into the narrow verifier proof type.
pub fn parse_groth16_bn254_proof(
  proof_json: &[u8],
) -> Result<Groth16Bn254Proof, SnarkjsGroth16ParseError> {
  let proof: SnarkjsProofJson = serde_json::from_slice(proof_json)
    .map_err(|source| SnarkjsGroth16ParseError::Json { artifact: "proof", source })?;

  ensure_groth16_bn254(&proof.protocol, &proof.curve)?;

  Ok(Groth16Bn254Proof {
    a: parse_g1_point("proof.pi_a", &proof.pi_a)?,
    b: parse_affine_g2("proof.pi_b", &proof.pi_b)?,
    c: parse_g1_point("proof.pi_c", &proof.pi_c)?,
  })
}

/// Parses a snarkjs public-input JSON array into the narrow verifier statement values.
pub fn parse_groth16_bn254_public_inputs(
  public_json: &[u8],
) -> Result<Vec<NativeField>, SnarkjsGroth16ParseError> {
  let public_inputs: Vec<String> = serde_json::from_slice(public_json)
    .map_err(|source| SnarkjsGroth16ParseError::Json { artifact: "public-input", source })?;

  public_inputs.iter().map(|value| parse_native_field(value)).collect::<Result<Vec<_>, _>>()
}

/// Parses a snarkjs Groth16 public-input array into a caller-named shape.
pub fn parse_groth16_bn254_public_inputs_with_names(
  public_json: &[u8],
  field_names: &[&str],
) -> Result<NamedPublicInputs, SnarkjsGroth16ParseError> {
  let public_inputs: Vec<String> = serde_json::from_slice(public_json)
    .map_err(|source| SnarkjsGroth16ParseError::Json { artifact: "public-input", source })?;

  if public_inputs.len() != field_names.len() {
    return Err(SnarkjsGroth16ParseError::InvalidPublicInputLength {
      context: "named Groth16 public-input vector",
      expected: field_names.len(),
      actual: public_inputs.len(),
    });
  }

  for value in &public_inputs {
    parse_native_field(value)?;
  }

  Ok(NamedPublicInputs::new(
    field_names
      .iter()
      .zip(public_inputs)
      .map(|(name, value)| NamedPublicInput::new(*name, value))
      .collect(),
  ))
}

/// Parses a snarkjs Groth16 verification key into the narrow verifier VK type.
pub fn parse_groth16_bn254_verifying_key(
  vk_json: &[u8],
) -> Result<Groth16Bn254VerifyingKey, SnarkjsGroth16ParseError> {
  let vk: SnarkjsVerificationKeyJson = serde_json::from_slice(vk_json)
    .map_err(|source| SnarkjsGroth16ParseError::Json { artifact: "verification-key", source })?;

  ensure_groth16_bn254(&vk.protocol, &vk.curve)?;

  if vk.ic.len() != vk.n_public + 1 {
    return Err(SnarkjsGroth16ParseError::InvalidIcLength {
      expected_ic_len: vk.n_public + 1,
      actual_ic_len: vk.ic.len(),
    });
  }

  Ok(Groth16Bn254VerifyingKey {
    alpha_g1: parse_g1_point("vk_alpha_1", &vk.vk_alpha_1)?,
    beta_g2: parse_affine_g2("vk_beta_2", &vk.vk_beta_2)?,
    gamma_g2: parse_affine_g2("vk_gamma_2", &vk.vk_gamma_2)?,
    delta_g2: parse_affine_g2("vk_delta_2", &vk.vk_delta_2)?,
    ic: vk.ic.iter().map(|point| parse_g1_point("IC", point)).collect::<Result<Vec<_>, _>>()?,
  })
}

#[cfg(test)]
mod tests {
  use super::{
    SnarkjsGroth16ParseError, parse_groth16_bn254_proof, parse_groth16_bn254_public_inputs,
    parse_groth16_bn254_public_inputs_with_names, parse_groth16_bn254_verifying_key,
  };
  use ff::{Field, PrimeField};
  use wrapper_core::{NamedPublicInput, NamedPublicInputs};
  use wrapper_circuits::{
    ForeignField, Groth16Bn254G1Point, groth16_fixture_raw, groth16_fixture_typed,
  };

  fn semaphore_proof_json() -> &'static [u8] {
    include_bytes!("../../wrapper-tests/fixtures/groth16/semaphore/proof.json")
  }

  fn semaphore_public_inputs_json() -> &'static [u8] {
    include_bytes!("../../wrapper-tests/fixtures/groth16/semaphore/public.json")
  }

  fn semaphore_verification_key_json() -> &'static [u8] {
    include_bytes!("../../wrapper-tests/fixtures/groth16/semaphore/verification_key.json")
  }

  #[test]
  fn parses_real_snarkjs_fixture_structure() {
    let proof = parse_groth16_bn254_proof(groth16_fixture_raw::proof_json())
      .expect("fixture proof should parse");
    let public_inputs =
      parse_groth16_bn254_public_inputs(groth16_fixture_raw::public_inputs_json())
        .expect("fixture public inputs should parse");
    let vk = parse_groth16_bn254_verifying_key(groth16_fixture_raw::verification_key_json())
      .expect("fixture vk should parse");

    assert_eq!(vk.ic.len(), 2);
    assert_eq!(vk.ic[0], Groth16Bn254G1Point::Identity);
    assert_eq!(
      vk.alpha_g1,
      Groth16Bn254G1Point::affine(ForeignField::ONE, ForeignField::from(2_u64))
    );
    assert_eq!(public_inputs.len(), 1);
    assert_eq!(proof.a, groth16_fixture_typed::proof().a);
  }

  #[test]
  fn parsed_real_snarkjs_fixture_matches_canonical_typed_fixture() {
    let proof = parse_groth16_bn254_proof(groth16_fixture_raw::proof_json())
      .expect("fixture proof should parse");
    let public_inputs =
      parse_groth16_bn254_public_inputs(groth16_fixture_raw::public_inputs_json())
        .expect("fixture public inputs should parse");
    let vk = parse_groth16_bn254_verifying_key(groth16_fixture_raw::verification_key_json())
      .expect("fixture vk should parse");

    assert_eq!(proof, groth16_fixture_typed::proof());
    assert_eq!(public_inputs, groth16_fixture_typed::public_inputs());
    assert_eq!(vk, groth16_fixture_typed::verifying_key());
  }

  #[test]
  fn parses_semaphore_snarkjs_fixture_without_special_cases() {
    let proof =
      parse_groth16_bn254_proof(semaphore_proof_json()).expect("Semaphore proof should parse");
    let public_inputs = parse_groth16_bn254_public_inputs(semaphore_public_inputs_json())
      .expect("Semaphore public inputs should parse");
    let vk = parse_groth16_bn254_verifying_key(semaphore_verification_key_json())
      .expect("Semaphore VK should parse");

    assert_eq!(public_inputs.len(), 4);
    assert_eq!(vk.ic.len(), 5);
    assert_eq!(
      proof.a,
      Groth16Bn254G1Point::affine(
        ForeignField::from_str_vartime(
          "2448901300518098096993075752654536134313649038239216706400667219963346227679"
        )
        .expect("Semaphore proof x-coordinate should parse"),
        ForeignField::from_str_vartime(
          "11383357624181217239434984412545229801919536849542936327488167664579097021171"
        )
        .expect("Semaphore proof y-coordinate should parse"),
      )
    );
    assert_eq!(
      public_inputs[0],
      ff::PrimeField::from_str_vartime(
        "4990292586352433503726012711155167179034286198473030768981544541070532815155"
      )
      .expect("Semaphore root public input should parse"),
    );
  }

  #[test]
  fn parses_public_inputs_into_caller_named_shape() {
    let public_inputs = parse_groth16_bn254_public_inputs_with_names(
      semaphore_public_inputs_json(),
      &["merkle_root", "nullifier", "message_hash", "scope_hash"],
    )
    .expect("named public inputs should parse into structured form");

    assert_eq!(
      public_inputs,
      NamedPublicInputs::new(vec![
        NamedPublicInput::new(
          "merkle_root",
          "4990292586352433503726012711155167179034286198473030768981544541070532815155",
        ),
        NamedPublicInput::new(
          "nullifier",
          "17540473064543782218297133630279824063352907908315494138425986188962403570231",
        ),
        NamedPublicInput::new(
          "message_hash",
          "8665846418922331996225934941481656421248110469944536651334918563951783029",
        ),
        NamedPublicInput::new(
          "scope_hash",
          "170164770795872309789133717676167925425155944778337387941930839678899666300",
        ),
      ])
    );
  }

  #[test]
  fn rejects_malformed_proof_point_shape() {
    let malformed = br#"{
      "protocol":"groth16",
      "curve":"bn128",
      "pi_a":["1","2"],
      "pi_b":[["1","2"],["3","4"],["1","0"]],
      "pi_c":["1","2","1"]
    }"#;

    let error = parse_groth16_bn254_proof(malformed).expect_err("malformed proof should fail");

    assert!(matches!(
      error,
      SnarkjsGroth16ParseError::InvalidPointEncoding { point_kind: "proof.pi_a", .. }
    ));
  }

  #[test]
  fn rejects_vk_with_inconsistent_ic_length() {
    let malformed = br#"{
      "protocol":"groth16",
      "curve":"bn128",
      "nPublic":2,
      "vk_alpha_1":["1","2","1"],
      "vk_beta_2":[["1","2"],["3","4"],["1","0"]],
      "vk_gamma_2":[["1","2"],["3","4"],["1","0"]],
      "vk_delta_2":[["1","2"],["3","4"],["1","0"]],
      "IC":[["0","1","1"],["1","2","1"]]
    }"#;

    let error =
      parse_groth16_bn254_verifying_key(malformed).expect_err("bad IC length should fail");

    assert!(matches!(error, SnarkjsGroth16ParseError::InvalidIcLength { .. }));
  }

  #[test]
  fn rejects_malformed_public_inputs() {
    let error = parse_groth16_bn254_public_inputs(br#"{"not":"an array"}"#)
      .expect_err("malformed public input json should fail");

    assert!(matches!(error, SnarkjsGroth16ParseError::Json { artifact: "public-input", .. }));
  }

  #[test]
  fn rejects_named_public_inputs_with_wrong_arity() {
    let error = parse_groth16_bn254_public_inputs_with_names(br#"["1","2","3"]"#, &["a", "b"])
      .expect_err("mismatched named public-input vector should fail");

    assert!(matches!(
      error,
      SnarkjsGroth16ParseError::InvalidPublicInputLength {
        context: "named Groth16 public-input vector",
        expected: 2,
        actual: 3
      }
    ));
  }
}
