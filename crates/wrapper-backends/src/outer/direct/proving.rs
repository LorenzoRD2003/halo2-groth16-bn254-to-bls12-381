use blake2b_simd::State as Blake2bState;
use std::io;
use midnight_curves::{Bls12, bn256::Bn256};
use midnight_proofs::{
  plonk::{create_proof, k_from_circuit, keygen_pk, keygen_vk_with_k, prepare, ProvingKey},
  poly::{
    commitment::{Guard, PolynomialCommitmentScheme},
    kzg::KZGCommitmentScheme,
  },
  transcript::{CircuitTranscript, Transcript},
  utils::SerdeFormat,
};
use rand_core::OsRng;
use wrapper_circuits::{
  Bls12HostField, HostedOuterWrapperCircuit, HostedOuterWrapperCircuitBls12, OuterHostField,
  OuterWrapperCircuit,
};
use wrapper_core::{
  ProducedOuterProofArtifactBundle, ProducedOuterVerificationKeyJson, WrapperExecutionPackage,
};

use super::{MidnightDirectOuterBackend, MidnightDirectOuterBackendBls12Host};
use crate::outer::{
  OuterProofBackend, OuterProofBackendError, ProducedOuterProvingKeyJson,
  ProducedOuterSetupArtifactBundle,
  helpers::{hex_decode, hex_encode, outer_instance_columns, outer_instance_columns_for_host},
};

impl MidnightDirectOuterBackend {
  /// Produces reusable setup artifacts and streams the proving key to one caller-owned writer.
  pub fn write_setup_bundle<W: io::Write>(
    self,
    package: &WrapperExecutionPackage,
    circuit: &OuterWrapperCircuit,
    proving_key_writer: &mut W,
    proving_key_artifact: impl Into<String>,
  ) -> Result<ProducedOuterSetupArtifactBundle, OuterProofBackendError> {
    let hosted = circuit.hosted();
    let k = k_from_circuit(&hosted);
    let params = KZGCommitmentScheme::<Bn256>::gen_params(k);
    let vk = keygen_vk_with_k::<OuterHostField, KZGCommitmentScheme<Bn256>, _>(&params, &hosted, k)
      .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("midnight keygen_vk failed: {error}"),
      })?;
    let pk = keygen_pk::<OuterHostField, KZGCommitmentScheme<Bn256>, _>(vk.clone(), &hosted)
      .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("midnight keygen_pk failed: {error}"),
      })?;
    pk.write(proving_key_writer, SerdeFormat::Processed).map_err(|error| {
      OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("failed to serialize proving key: {error}"),
      }
    })?;

    let verification_key = self.serialize_setup_verification_key(package, k, &params, &vk)?;
    let proving_key = ProducedOuterProvingKeyJson {
      protocol: self.metadata().protocol.to_owned(),
      curve: self.metadata().curve.to_owned(),
      backend: self.metadata().backend_id.to_owned(),
      pcs: self.metadata().pcs.to_owned(),
      encoding: self.metadata().serialization.payload_encoding().to_owned(),
      circuit_k: k,
      public_input_count: package.statement.public_inputs.entries.len(),
      proving_key_artifact: proving_key_artifact.into(),
    };

    Ok(ProducedOuterSetupArtifactBundle {
      backend: self.metadata().backend_id.to_owned(),
      outer_host: self.metadata().outer_host.id().to_owned(),
      verification_key,
      proving_key,
      notes: vec![
        format!("selected outer backend stack: {}", self.metadata().stack),
        "setup bundle contains reusable proving key plus verification materials".to_owned(),
      ],
    })
  }

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

  /// Produces a real proof bundle by reusing previously persisted setup artifacts.
  pub fn produce_proof_bundle_from_setup_reader<R: io::Read>(
    self,
    package: &WrapperExecutionPackage,
    circuit: &OuterWrapperCircuit,
    setup: &ProducedOuterSetupArtifactBundle,
    proving_key_reader: &mut R,
  ) -> Result<ProducedOuterProofArtifactBundle, OuterProofBackendError> {
    self.validate_setup_verification_key(package, &setup.verification_key)?;

    if setup.backend != self.metadata().backend_id {
      return Err(OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!(
          "setup bundle backend mismatch: expected {}, got {}",
          self.metadata().backend_id,
          setup.backend
        ),
      });
    }

    if setup.proving_key.public_input_count != package.statement.public_inputs.entries.len() {
      return Err(OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!(
          "setup bundle public-input count mismatch: expected {}, got {}",
          package.statement.public_inputs.entries.len(),
          setup.proving_key.public_input_count
        ),
      });
    }

    let hosted = circuit.hosted();
    let k = k_from_circuit(&hosted);
    if setup.proving_key.circuit_k != k {
      return Err(OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!(
          "setup bundle circuit_k mismatch: expected {}, got {}",
          k,
          setup.proving_key.circuit_k
        ),
      });
    }

    let params = KZGCommitmentScheme::<Bn256>::gen_params(k);
    let pk = ProvingKey::<OuterHostField, KZGCommitmentScheme<Bn256>>::read::<
      _,
      HostedOuterWrapperCircuit,
    >(proving_key_reader, SerdeFormat::Processed, ())
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("failed to deserialize proving key: {error}"),
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
    self.assemble_produced_bundle(package, proof, setup.verification_key.clone())
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

impl MidnightDirectOuterBackendBls12Host {
  /// Produces reusable setup artifacts and streams the proving key to one caller-owned writer.
  pub fn write_setup_bundle<W: io::Write>(
    self,
    package: &WrapperExecutionPackage,
    circuit: &OuterWrapperCircuit,
    proving_key_writer: &mut W,
    proving_key_artifact: impl Into<String>,
  ) -> Result<ProducedOuterSetupArtifactBundle, OuterProofBackendError> {
    let hosted = circuit.hosted_bls12();
    let k = k_from_circuit(&hosted);
    let params = KZGCommitmentScheme::<Bls12>::gen_params(k);
    let vk = keygen_vk_with_k::<Bls12HostField, KZGCommitmentScheme<Bls12>, _>(&params, &hosted, k)
      .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("midnight BLS12 keygen_vk failed: {error}"),
      })?;
    let pk = keygen_pk::<Bls12HostField, KZGCommitmentScheme<Bls12>, _>(vk.clone(), &hosted)
      .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("midnight BLS12 keygen_pk failed: {error}"),
      })?;
    pk.write(proving_key_writer, SerdeFormat::Processed).map_err(|error| {
      OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("failed to serialize BLS12 proving key: {error}"),
      }
    })?;

    let verification_key = self.serialize_setup_verification_key(package, k, &params, &vk)?;
    let proving_key = ProducedOuterProvingKeyJson {
      protocol: self.metadata().protocol.to_owned(),
      curve: self.metadata().curve.to_owned(),
      backend: self.metadata().backend_id.to_owned(),
      pcs: self.metadata().pcs.to_owned(),
      encoding: self.metadata().serialization.payload_encoding().to_owned(),
      circuit_k: k,
      public_input_count: package.statement.public_inputs.entries.len(),
      proving_key_artifact: proving_key_artifact.into(),
    };

    Ok(ProducedOuterSetupArtifactBundle {
      backend: self.metadata().backend_id.to_owned(),
      outer_host: self.metadata().outer_host.id().to_owned(),
      verification_key,
      proving_key,
      notes: vec![
        format!("selected outer backend stack: {}", self.metadata().stack),
        "setup bundle contains reusable proving key plus verification materials".to_owned(),
      ],
    })
  }

  pub(super) fn produce_setup_verification_key(
    self,
    package: &WrapperExecutionPackage,
    circuit: &OuterWrapperCircuit,
  ) -> Result<ProducedOuterVerificationKeyJson, OuterProofBackendError> {
    let hosted = circuit.hosted_bls12();
    let k = k_from_circuit(&hosted);
    let params = KZGCommitmentScheme::<Bls12>::gen_params(k);
    let vk = keygen_vk_with_k::<Bls12HostField, KZGCommitmentScheme<Bls12>, _>(&params, &hosted, k)
      .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("midnight BLS12 keygen_vk failed: {error}"),
      })?;

    self.serialize_setup_verification_key(package, k, &params, &vk)
  }

  /// Produces a real proof bundle by reusing previously persisted setup artifacts.
  pub fn produce_proof_bundle_from_setup_reader<R: io::Read>(
    self,
    package: &WrapperExecutionPackage,
    circuit: &OuterWrapperCircuit,
    setup: &ProducedOuterSetupArtifactBundle,
    proving_key_reader: &mut R,
  ) -> Result<ProducedOuterProofArtifactBundle, OuterProofBackendError> {
    self.validate_setup_verification_key(package, &setup.verification_key)?;

    if setup.backend != self.metadata().backend_id {
      return Err(OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!(
          "setup bundle backend mismatch: expected {}, got {}",
          self.metadata().backend_id,
          setup.backend
        ),
      });
    }

    if setup.proving_key.public_input_count != package.statement.public_inputs.entries.len() {
      return Err(OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!(
          "setup bundle public-input count mismatch: expected {}, got {}",
          package.statement.public_inputs.entries.len(),
          setup.proving_key.public_input_count
        ),
      });
    }

    let hosted = circuit.hosted_bls12();
    let k = k_from_circuit(&hosted);
    if setup.proving_key.circuit_k != k {
      return Err(OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!(
          "setup bundle circuit_k mismatch: expected {}, got {}",
          k,
          setup.proving_key.circuit_k
        ),
      });
    }

    let params = KZGCommitmentScheme::<Bls12>::gen_params(k);
    let pk = ProvingKey::<Bls12HostField, KZGCommitmentScheme<Bls12>>::read::<
      _,
      HostedOuterWrapperCircuitBls12,
    >(proving_key_reader, SerdeFormat::Processed, ())
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("failed to deserialize BLS12 proving key: {error}"),
    })?;

    let instance_columns = outer_instance_columns_for_host::<Bls12HostField>(circuit);
    let instance_column_refs = [instance_columns[0].as_slice(), instance_columns[1].as_slice()];
    let instances = [&instance_column_refs[..]];
    let mut transcript = CircuitTranscript::<Blake2bState>::init();

    create_proof::<Bls12HostField, KZGCommitmentScheme<Bls12>, _, _>(
      &params,
      &pk,
      std::slice::from_ref(&hosted),
      0,
      &instances,
      OsRng,
      &mut transcript,
    )
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("midnight BLS12 create_proof failed: {error}"),
    })?;

    let proof =
      self.metadata().proof_serialization().materialize(hex_encode(&transcript.finalize()));
    self.validate_produced_proof(package, &proof)?;
    self.assemble_produced_bundle(package, proof, setup.verification_key.clone())
  }

  fn serialize_setup_verification_key(
    self,
    package: &WrapperExecutionPackage,
    k: u32,
    params: &<KZGCommitmentScheme<Bls12> as PolynomialCommitmentScheme<Bls12HostField>>::Parameters,
    vk: &midnight_proofs::plonk::VerifyingKey<Bls12HostField, KZGCommitmentScheme<Bls12>>,
  ) -> Result<ProducedOuterVerificationKeyJson, OuterProofBackendError> {
    let mut verifier_params_bytes = Vec::new();
    params.verifier_params().write(&mut verifier_params_bytes, SerdeFormat::Processed).map_err(
      |error| OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("failed to serialize BLS12 verifier params: {error}"),
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
    let hosted = circuit.hosted_bls12();
    let k = k_from_circuit(&hosted);
    let params = KZGCommitmentScheme::<Bls12>::gen_params(k);
    let vk = keygen_vk_with_k::<Bls12HostField, KZGCommitmentScheme<Bls12>, _>(&params, &hosted, k)
      .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("midnight BLS12 keygen_vk failed during proving: {error}"),
      })?;
    let pk = keygen_pk::<Bls12HostField, KZGCommitmentScheme<Bls12>, _>(vk.clone(), &hosted)
      .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("midnight BLS12 keygen_pk failed: {error}"),
      })?;

    let instance_columns = outer_instance_columns_for_host::<Bls12HostField>(circuit);
    let instance_column_refs = [instance_columns[0].as_slice(), instance_columns[1].as_slice()];
    let instances = [&instance_column_refs[..]];
    let mut transcript = CircuitTranscript::<Blake2bState>::init();

    create_proof::<Bls12HostField, KZGCommitmentScheme<Bls12>, _, _>(
      &params,
      &pk,
      std::slice::from_ref(&hosted),
      0,
      &instances,
      OsRng,
      &mut transcript,
    )
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("midnight BLS12 create_proof failed: {error}"),
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

    let verifier_params = midnight_proofs::poly::kzg::params::ParamsVerifierKZG::<Bls12>::read(
      &mut verifier_params_bytes.as_slice(),
      SerdeFormat::Processed,
    )
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("failed to deserialize BLS12 verifier params: {error}"),
    })?;
    let verification_key = midnight_proofs::plonk::VerifyingKey::<
      Bls12HostField,
      KZGCommitmentScheme<Bls12>,
    >::from_bytes::<HostedOuterWrapperCircuitBls12>(
      &verification_key_bytes,
      SerdeFormat::Processed,
      (),
    )
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("failed to deserialize BLS12 verification key: {error}"),
    })?;

    let instance_columns = outer_instance_columns_for_host::<Bls12HostField>(circuit);
    let instance_column_refs = [instance_columns[0].as_slice(), instance_columns[1].as_slice()];
    let instances = [&instance_column_refs[..]];
    let committed_instances: [&[<KZGCommitmentScheme<Bls12> as PolynomialCommitmentScheme<
      Bls12HostField,
    >>::Commitment]; 1] = [&[]];
    let mut transcript = CircuitTranscript::<Blake2bState>::init_from_bytes(&proof_bytes);
    let guard = prepare::<Bls12HostField, KZGCommitmentScheme<Bls12>, _>(
      &verification_key,
      &committed_instances,
      &instances,
      &mut transcript,
    )
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("failed to prepare BLS12 proof verification: {error}"),
    })?;
    guard.verify(&verifier_params).map_err(|error| {
      OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("BLS12 proof verification failed: {error:?}"),
      }
    })?;
    transcript.assert_empty().map_err(|error| {
      OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("BLS12 proof transcript has trailing bytes: {error}"),
      }
    })?;

    Ok(true)
  }
}
