use ff::{Field, PrimeField};
use wrapper_circuits::{OuterHostField, OuterWrapperCircuit, lift_outer_inputs_to_host};
use wrapper_core::{ExpectedWrapperArtifacts, ProofSystemKind, WrapperExecutionPackage};

use super::{OuterProofBackendError, OuterProofBackendMetadata};

pub(super) fn ensure_supported_target(
  package: &WrapperExecutionPackage,
) -> Result<(), OuterProofBackendError> {
  if package.job.target.kind != ProofSystemKind::Halo2Outer {
    return Err(OuterProofBackendError::UnsupportedTarget {
      expected: "halo2-outer",
      actual: match package.job.target.kind {
        ProofSystemKind::Groth16Bn254 => "groth16-bn254",
        ProofSystemKind::Groth16Bls12_381 => "groth16-bls12-381",
        ProofSystemKind::Halo2Outer => "halo2-outer",
      },
    });
  }

  Ok(())
}

pub(super) fn parse_native_input_value(
  context: &'static str,
  field_name: &str,
  value: &str,
) -> Result<OuterHostField, OuterProofBackendError> {
  if let Some(hex) = value.strip_prefix("0x") {
    let mut accumulator = OuterHostField::ZERO;
    let radix = OuterHostField::from(16_u64);

    for ch in hex.chars() {
      let digit =
        ch.to_digit(16).ok_or_else(|| OuterProofBackendError::InvalidPublicInputValue {
          context,
          field_name: field_name.to_owned(),
          value: value.to_owned(),
        })?;
      accumulator = accumulator * radix + OuterHostField::from(u64::from(digit));
    }

    return Ok(accumulator);
  }

  OuterHostField::from_str_vartime(value).ok_or_else(|| {
    OuterProofBackendError::InvalidPublicInputValue {
      context,
      field_name: field_name.to_owned(),
      value: value.to_owned(),
    }
  })
}

pub(super) fn hex_encode(bytes: &[u8]) -> String {
  let mut encoded = String::with_capacity(bytes.len() * 2);
  for byte in bytes {
    use std::fmt::Write as _;
    let _ = write!(&mut encoded, "{byte:02x}");
  }
  encoded
}

pub(super) fn hex_decode(value: &str) -> Result<Vec<u8>, OuterProofBackendError> {
  if value.len() % 2 != 0 {
    return Err(OuterProofBackendError::OuterCircuitInputInvalid {
      reason: "hex payload has odd length".to_owned(),
    });
  }

  let mut bytes = Vec::with_capacity(value.len() / 2);
  for index in (0..value.len()).step_by(2) {
    let byte = u8::from_str_radix(&value[index..index + 2], 16).map_err(|error| {
      OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("invalid hex payload at byte {}: {error}", index / 2),
      }
    })?;
    bytes.push(byte);
  }

  Ok(bytes)
}

pub(super) fn outer_instance_columns(circuit: &OuterWrapperCircuit) -> [&[OuterHostField]; 2] {
  [circuit.input.outer_statement.public_inputs.as_slice(), &[]]
}

pub(super) fn outer_instance_columns_for_host<FHost: PrimeField>(
  circuit: &OuterWrapperCircuit,
) -> [Vec<FHost>; 2] {
  [lift_outer_inputs_to_host(&circuit.input.outer_statement.public_inputs), Vec::new()]
}

pub(super) fn expected_output_for_backend(
  package: &WrapperExecutionPackage,
  metadata: &OuterProofBackendMetadata,
) -> ExpectedWrapperArtifacts {
  let mut expected = package.expected_output();

  metadata.backend_id.clone_into(&mut expected.proof_system.source);
  metadata.protocol.clone_into(&mut expected.proof_shape.protocol);
  metadata.curve.clone_into(&mut expected.proof_shape.curve);
  metadata.backend_id.clone_into(&mut expected.proof_shape.backend);
  metadata.transcript.clone_into(&mut expected.proof_shape.transcript);
  metadata.serialization.payload_encoding().clone_into(&mut expected.proof_shape.payload_encoding);

  metadata.protocol.clone_into(&mut expected.verification_key_shape.protocol);
  metadata.curve.clone_into(&mut expected.verification_key_shape.curve);
  metadata.backend_id.clone_into(&mut expected.verification_key_shape.backend);
  metadata.pcs.clone_into(&mut expected.verification_key_shape.pcs);
  metadata
    .serialization
    .payload_encoding()
    .clone_into(&mut expected.verification_key_shape.payload_encoding);

  metadata.backend_id.clone_into(&mut expected.bundle_template.proof_system.source);
  if let Some(proof) = expected.bundle_template.proof.as_mut() {
    metadata.protocol.clone_into(&mut proof.protocol);
    metadata.curve.clone_into(&mut proof.curve);
    metadata.backend_id.clone_into(&mut proof.backend);
    metadata.transcript.clone_into(&mut proof.transcript);
    metadata.serialization.payload_encoding().clone_into(&mut proof.encoding);
  }
  if let Some(vk) = expected.bundle_template.verification_key.as_mut() {
    metadata.protocol.clone_into(&mut vk.protocol);
    metadata.curve.clone_into(&mut vk.curve);
    metadata.backend_id.clone_into(&mut vk.backend);
    metadata.pcs.clone_into(&mut vk.pcs);
    metadata.serialization.payload_encoding().clone_into(&mut vk.encoding);
    vk.public_input_count = package.statement.public_inputs.entries.len();
  }

  expected
}
