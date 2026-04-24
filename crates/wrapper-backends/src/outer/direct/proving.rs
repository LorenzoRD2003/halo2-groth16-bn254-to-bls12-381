use blake2b_simd::State as Blake2bState;
use midnight_curves::bn256::Bn256;
use midnight_proofs::{
  plonk::{create_proof, k_from_circuit, keygen_pk, keygen_vk_with_k, prepare},
  poly::{
    commitment::{Guard, PolynomialCommitmentScheme},
    kzg::KZGCommitmentScheme,
  },
  transcript::{CircuitTranscript, Transcript},
  utils::SerdeFormat,
};
use rand_core::OsRng;
use wrapper_circuits::{HostedOuterWrapperCircuit, OuterHostField, OuterWrapperCircuit};
use wrapper_core::{
  ProducedOuterProofArtifactBundle, ProducedOuterVerificationKeyJson, WrapperExecutionPackage,
};

use super::MidnightDirectOuterBackend;
use crate::outer::{
  OuterProofBackend, OuterProofBackendError,
  helpers::{hex_decode, hex_encode, outer_instance_columns},
};

impl MidnightDirectOuterBackend {
  pub(super) fn produce_setup_verification_key(
    self,
    package: &WrapperExecutionPackage,
    circuit: &OuterWrapperCircuit,
  ) -> Result<ProducedOuterVerificationKeyJson, OuterProofBackendError> {
    let hosted = circuit.hosted();
    let k = k_from_circuit(&hosted);
    let params = KZGCommitmentScheme::<Bn256>::gen_params(k);
    let vk = keygen_vk_with_k::<OuterHostField, KZGCommitmentScheme<Bn256>, _>(&params, &hosted, k)
      .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("midnight keygen_vk failed: {error}"),
      })?;

    self.serialize_setup_verification_key(package, k, &params, &vk)
  }

  fn serialize_setup_verification_key(
    self,
    package: &WrapperExecutionPackage,
    k: u32,
    params: &<KZGCommitmentScheme<Bn256> as PolynomialCommitmentScheme<OuterHostField>>::Parameters,
    vk: &midnight_proofs::plonk::VerifyingKey<OuterHostField, KZGCommitmentScheme<Bn256>>,
  ) -> Result<ProducedOuterVerificationKeyJson, OuterProofBackendError> {
    let mut verifier_params_bytes = Vec::new();
    params.verifier_params().write(&mut verifier_params_bytes, SerdeFormat::Processed).map_err(
      |error| OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("failed to serialize verifier params: {error}"),
      },
    )?;

    let verification_key = self.metadata().verification_key_serialization().materialize(
      k,
      package.statement.public_inputs.entries.len(),
      hex_encode(&vk.to_bytes(SerdeFormat::Processed)),
      hex_encode(&verifier_params_bytes),
    );

    self.validate_setup_verification_key(package, &verification_key)?;
    Ok(verification_key)
  }

  pub(super) fn produce_proof_bundle(
    self,
    package: &WrapperExecutionPackage,
    circuit: &OuterWrapperCircuit,
  ) -> Result<ProducedOuterProofArtifactBundle, OuterProofBackendError> {
    let hosted = circuit.hosted();
    let k = k_from_circuit(&hosted);
    let params = KZGCommitmentScheme::<Bn256>::gen_params(k);
    let vk = keygen_vk_with_k::<OuterHostField, KZGCommitmentScheme<Bn256>, _>(&params, &hosted, k)
      .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("midnight keygen_vk failed during proving: {error}"),
      })?;
    let pk = keygen_pk::<OuterHostField, KZGCommitmentScheme<Bn256>, _>(vk.clone(), &hosted)
      .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("midnight keygen_pk failed: {error}"),
      })?;

    let instance_columns = outer_instance_columns(circuit);
    let instances = [&instance_columns[..]];
    let mut transcript = CircuitTranscript::<Blake2bState>::init();

    create_proof::<OuterHostField, KZGCommitmentScheme<Bn256>, _, _>(
      &params,
      &pk,
      std::slice::from_ref(&hosted),
      0,
      &instances,
      OsRng,
      &mut transcript,
    )
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("midnight create_proof failed: {error}"),
    })?;

    let proof =
      self.metadata().proof_serialization().materialize(hex_encode(&transcript.finalize()));
    self.validate_produced_proof(package, &proof)?;

    let verification_key = self.serialize_setup_verification_key(package, k, &params, &vk)?;
    self.assemble_produced_bundle(package, proof, verification_key)
  }

  pub(super) fn verify_produced_bundle(
    self,
    package: &WrapperExecutionPackage,
    produced: &ProducedOuterProofArtifactBundle,
    circuit: &OuterWrapperCircuit,
  ) -> Result<bool, OuterProofBackendError> {
    self.validate_produced_proof(package, &produced.proof)?;
    self.validate_setup_verification_key(package, &produced.verification_key)?;

    let verifier_params_bytes = hex_decode(&produced.verification_key.verifier_params)?;
    let verification_key_bytes = hex_decode(&produced.verification_key.verification_key)?;
    let proof_bytes = hex_decode(&produced.proof.proof)?;

    let verifier_params = midnight_proofs::poly::kzg::params::ParamsVerifierKZG::<Bn256>::read(
      &mut verifier_params_bytes.as_slice(),
      SerdeFormat::Processed,
    )
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("failed to deserialize verifier params: {error}"),
    })?;
    let verification_key = midnight_proofs::plonk::VerifyingKey::<
      OuterHostField,
      KZGCommitmentScheme<Bn256>,
    >::from_bytes::<HostedOuterWrapperCircuit>(
      &verification_key_bytes, SerdeFormat::Processed, ()
    )
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("failed to deserialize verification key: {error}"),
    })?;

    let instance_columns = outer_instance_columns(circuit);
    let instances = [&instance_columns[..]];
    let committed_instances: [&[<KZGCommitmentScheme<Bn256> as PolynomialCommitmentScheme<
      OuterHostField,
    >>::Commitment]; 1] = [&[]];
    let mut transcript = CircuitTranscript::<Blake2bState>::init_from_bytes(&proof_bytes);
    let guard = prepare::<OuterHostField, KZGCommitmentScheme<Bn256>, _>(
      &verification_key,
      &committed_instances,
      &instances,
      &mut transcript,
    )
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("failed to prepare proof verification: {error}"),
    })?;
    guard.verify(&verifier_params).map_err(|error| {
      OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("proof verification failed: {error:?}"),
      }
    })?;
    transcript.assert_empty().map_err(|error| {
      OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("proof transcript has trailing bytes: {error}"),
      }
    })?;

    Ok(true)
  }
}
