use blake2b_simd::State as Blake2bState;
use midnight_curves::{Bls12, bn256::Bn256};
use midnight_proofs::{
  plonk::{
    BaseProvingKey, create_proof, create_proof_from_base, create_proof_trace_from_base,
    finalise_proof_from_base_trace, k_from_circuit, keygen_pk, keygen_pk_base, keygen_vk_with_k,
    prepare, traces::PersistedProverTrace,
  },
  poly::{
    commitment::{Guard, PolynomialCommitmentScheme},
    kzg::KZGCommitmentScheme,
  },
  transcript::{CircuitTranscript, Transcript},
  utils::SerdeFormat,
};
use rand_core::OsRng;
use std::{
  fs::OpenOptions,
  io,
  io::{BufRead, BufReader, Write as _},
  time::{Instant, SystemTime, UNIX_EPOCH},
};
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

const DIRECT_LOG_FILE_ENV: &str = "WRAPPER_DIRECT_LOG_FILE";

fn now_millis() -> u128 {
  SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis()
}

fn parse_status_kb_field(line: &str, label: &str) -> Option<u64> {
  let rest = line.strip_prefix(label)?.trim();
  let numeric = rest.split_whitespace().next()?;
  numeric.parse::<u64>().ok()
}

fn memory_snapshot() -> (Option<u64>, Option<u64>, Option<u64>) {
  let Ok(file) = std::fs::File::open("/proc/self/status") else {
    return (None, None, None);
  };
  let reader = BufReader::new(file);
  let mut rss_kb = None;
  let mut hwm_kb = None;
  let mut vmsize_kb = None;
  for line in reader.lines().map_while(Result::ok) {
    if let Some(value) = parse_status_kb_field(&line, "VmRSS:") {
      rss_kb = Some(value);
    } else if let Some(value) = parse_status_kb_field(&line, "VmHWM:") {
      hwm_kb = Some(value);
    } else if let Some(value) = parse_status_kb_field(&line, "VmSize:") {
      vmsize_kb = Some(value);
    }
  }
  (rss_kb, hwm_kb, vmsize_kb)
}

fn append_backend_log(message: &str) {
  if let Ok(path) = std::env::var(DIRECT_LOG_FILE_ENV) {
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
      let _ = writeln!(file, "{message}");
    }
  }
}

fn append_backend_event(phase: &str, step: &str, event: &str, extra: &str) {
  let (rss_kb, hwm_kb, vmsize_kb) = memory_snapshot();
  let run_id = std::env::var("WRAPPER_DIRECT_LOG_RUN_ID")
    .unwrap_or_else(|_| format!("pid{}", std::process::id()));
  let command = std::env::var("WRAPPER_DIRECT_LOG_COMMAND").unwrap_or_default();
  let identifier = std::env::var("WRAPPER_DIRECT_LOG_IDENTIFIER").unwrap_or_default();
  let backend = std::env::var("WRAPPER_DIRECT_LOG_BACKEND").unwrap_or_default();
  let host = std::env::var("WRAPPER_DIRECT_LOG_HOST").unwrap_or_default();
  let mut line = format!(
    "ts_ms={} level=INFO run_id={} pid={} phase={} step={} event={}",
    now_millis(),
    run_id,
    std::process::id(),
    phase,
    step,
    event
  );
  if !command.is_empty() {
    line.push_str(&format!(" command={command}"));
  }
  if !identifier.is_empty() {
    line.push_str(&format!(" identifier={identifier}"));
  }
  if !backend.is_empty() {
    line.push_str(&format!(" backend={backend}"));
  }
  if !host.is_empty() {
    line.push_str(&format!(" host={host}"));
  }
  if let Some(rss_kb) = rss_kb {
    line.push_str(&format!(" rss_kb={rss_kb}"));
  }
  if let Some(hwm_kb) = hwm_kb {
    line.push_str(&format!(" hwm_kb={hwm_kb}"));
  }
  if let Some(vmsize_kb) = vmsize_kb {
    line.push_str(&format!(" vmsize_kb={vmsize_kb}"));
  }
  if !extra.is_empty() {
    line.push(' ');
    line.push_str(extra);
  }
  append_backend_log(&line);
}

fn append_backend_elapsed(phase: &str, step: &str, started_at: Instant, extra: &str) {
  let mut merged_extra = format!("elapsed_ms={}", started_at.elapsed().as_millis());
  if !extra.is_empty() {
    merged_extra.push(' ');
    merged_extra.push_str(extra);
  }
  append_backend_event(phase, step, "end", &merged_extra);
}

impl MidnightDirectOuterBackend {
  /// Produces reusable setup artifacts and streams the proving key sidecar to one caller-owned writer.
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
    let pk = keygen_pk_base::<OuterHostField, KZGCommitmentScheme<Bn256>, _>(vk.clone(), &hosted)
      .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("midnight keygen_pk_base failed: {error}"),
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
        "setup bundle persists verification materials plus a proving-key sidecar".to_owned(),
        "prove reuses the persisted proving-key sidecar and avoids rerunning keygen_pk".to_owned(),
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
          k, setup.proving_key.circuit_k
        ),
      });
    }

    let params = KZGCommitmentScheme::<Bn256>::gen_params(k);
    let base_pk = BaseProvingKey::<OuterHostField, KZGCommitmentScheme<Bn256>>::read::<
      _,
      HostedOuterWrapperCircuit,
    >(proving_key_reader, SerdeFormat::Processed, ())
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("failed to deserialize proving key: {error}"),
    })?;

    let instance_columns = outer_instance_columns(circuit);
    let instances = [&instance_columns[..]];
    let mut transcript = CircuitTranscript::<Blake2bState>::init();

    create_proof_from_base::<OuterHostField, KZGCommitmentScheme<Bn256>, _, _>(
      &params,
      base_pk,
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

  /// Produces the pre-`compute_h_poly` proving trace artifact from persisted
  /// setup materials.
  pub fn produce_proof_trace_from_setup_reader<R: io::Read, W: io::Write>(
    self,
    package: &WrapperExecutionPackage,
    circuit: &OuterWrapperCircuit,
    setup: &ProducedOuterSetupArtifactBundle,
    proving_key_reader: &mut R,
    trace_writer: &mut W,
  ) -> Result<(), OuterProofBackendError> {
    append_backend_log("prove-trace: validating setup verification key");
    self.validate_setup_verification_key(package, &setup.verification_key)?;

    let hosted = circuit.hosted();
    let k = k_from_circuit(&hosted);
    append_backend_log(&format!("prove-trace: using circuit_k={k}"));
    let params = KZGCommitmentScheme::<Bn256>::gen_params(k);
    append_backend_log("prove-trace: deserializing BaseProvingKey");
    let base_pk = BaseProvingKey::<OuterHostField, KZGCommitmentScheme<Bn256>>::read::<
      _,
      HostedOuterWrapperCircuit,
    >(proving_key_reader, SerdeFormat::Processed, ())
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("failed to deserialize proving key: {error}"),
    })?;

    let instance_columns = outer_instance_columns(circuit);
    let instances = [&instance_columns[..]];
    let mut transcript = CircuitTranscript::<Blake2bState>::init();
    append_backend_log("prove-trace: entering create_proof_trace_from_base");
    let trace = create_proof_trace_from_base::<OuterHostField, KZGCommitmentScheme<Bn256>, _, _>(
      &params,
      &base_pk,
      std::slice::from_ref(&hosted),
      0,
      &instances,
      OsRng,
      &mut transcript,
    )
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("midnight create_proof_trace_from_base failed: {error}"),
    })?;
    append_backend_log("prove-trace: create_proof_trace_from_base complete");

    append_backend_log("prove-trace: serializing persisted prover trace");
    trace.write(trace_writer, base_pk.get_vk()).map_err(|error| {
      OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("failed to serialize persisted prover trace: {error}"),
      }
    })?;
    append_backend_log("prove-trace: trace serialization complete");
    Ok(())
  }

  /// Finalizes a proof bundle from a previously persisted proving trace
  /// artifact.
  pub fn produce_proof_bundle_from_trace_reader<R: io::Read, TR: io::Read>(
    self,
    package: &WrapperExecutionPackage,
    circuit: &OuterWrapperCircuit,
    setup: &ProducedOuterSetupArtifactBundle,
    proving_key_reader: &mut R,
    trace_reader: &mut TR,
  ) -> Result<ProducedOuterProofArtifactBundle, OuterProofBackendError> {
    append_backend_event("backend_finalize", "validate_setup_vk", "start", "");
    self.validate_setup_verification_key(package, &setup.verification_key)?;
    append_backend_event("backend_finalize", "validate_setup_vk", "end", "");

    let hosted = circuit.hosted();
    let k = k_from_circuit(&hosted);
    append_backend_event("backend_finalize", "context", "point", &format!("circuit_k={k}"));
    let params = KZGCommitmentScheme::<Bn256>::gen_params(k);
    let deserialize_base_pk_started_at = Instant::now();
    append_backend_event(
      "backend_finalize",
      "deserialize_base_pk",
      "start",
      &format!("circuit_k={k}"),
    );
    let base_pk = BaseProvingKey::<OuterHostField, KZGCommitmentScheme<Bn256>>::read::<
      _,
      HostedOuterWrapperCircuit,
    >(proving_key_reader, SerdeFormat::Processed, ())
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("failed to deserialize proving key: {error}"),
    })?;
    append_backend_elapsed(
      "backend_finalize",
      "deserialize_base_pk",
      deserialize_base_pk_started_at,
      &format!("circuit_k={k}"),
    );

    let deserialize_prepared_trace_started_at = Instant::now();
    append_backend_event(
      "backend_finalize",
      "deserialize_prepared_trace",
      "start",
      &format!("circuit_k={k}"),
    );
    let prepared_trace = PersistedProverTrace::<OuterHostField>::read_prepared(trace_reader)
      .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("failed to deserialize persisted prover trace: {error}"),
      })?;
    append_backend_elapsed(
      "backend_finalize",
      "deserialize_prepared_trace",
      deserialize_prepared_trace_started_at,
      &format!("circuit_k={k}"),
    );

    let init_transcript_started_at = Instant::now();
    append_backend_event("backend_finalize", "init_transcript", "start", &format!("circuit_k={k}"));
    let mut transcript = prepared_trace.init_transcript::<CircuitTranscript<Blake2bState>>();
    append_backend_elapsed(
      "backend_finalize",
      "init_transcript",
      init_transcript_started_at,
      &format!("circuit_k={k}"),
    );

    let finalize_from_trace_started_at = Instant::now();
    append_backend_event(
      "backend_finalize",
      "finalise_proof_from_base_trace",
      "start",
      &format!("circuit_k={k}"),
    );
    finalise_proof_from_base_trace::<OuterHostField, KZGCommitmentScheme<Bn256>, _, _>(
      &params,
      base_pk,
      0,
      prepared_trace,
      trace_reader,
      &mut transcript,
    )
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("midnight finalise_proof_from_base_trace failed: {error}"),
    })?;
    append_backend_elapsed(
      "backend_finalize",
      "finalise_proof_from_base_trace",
      finalize_from_trace_started_at,
      &format!("circuit_k={k}"),
    );

    let serialize_validate_proof_started_at = Instant::now();
    append_backend_event(
      "backend_finalize",
      "serialize_and_validate_proof",
      "start",
      &format!("circuit_k={k}"),
    );
    let proof =
      self.metadata().proof_serialization().materialize(hex_encode(&transcript.finalize()));
    self.validate_produced_proof(package, &proof)?;
    append_backend_elapsed(
      "backend_finalize",
      "serialize_and_validate_proof",
      serialize_validate_proof_started_at,
      &format!("circuit_k={k}"),
    );
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
  /// Produces reusable setup artifacts and streams the proving key sidecar to one caller-owned writer.
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
    let pk = keygen_pk_base::<Bls12HostField, KZGCommitmentScheme<Bls12>, _>(vk.clone(), &hosted)
      .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("midnight BLS12 keygen_pk_base failed: {error}"),
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
        "setup bundle persists verification materials plus a proving-key sidecar".to_owned(),
        "prove reuses the persisted proving-key sidecar and avoids rerunning keygen_pk".to_owned(),
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
          k, setup.proving_key.circuit_k
        ),
      });
    }

    let params = KZGCommitmentScheme::<Bls12>::gen_params(k);
    let base_pk = BaseProvingKey::<Bls12HostField, KZGCommitmentScheme<Bls12>>::read::<
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

    create_proof_from_base::<Bls12HostField, KZGCommitmentScheme<Bls12>, _, _>(
      &params,
      base_pk,
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

  /// Produces the pre-`compute_h_poly` proving-trace artifact for the
  /// BLS12-hosted direct outer lane from persisted setup materials.
  pub fn produce_proof_trace_from_setup_reader<R: io::Read, W: io::Write>(
    self,
    package: &WrapperExecutionPackage,
    circuit: &OuterWrapperCircuit,
    setup: &ProducedOuterSetupArtifactBundle,
    proving_key_reader: &mut R,
    trace_writer: &mut W,
  ) -> Result<(), OuterProofBackendError> {
    append_backend_log("prove-trace: validating setup verification key");
    self.validate_setup_verification_key(package, &setup.verification_key)?;

    let hosted = circuit.hosted_bls12();
    let k = k_from_circuit(&hosted);
    append_backend_log(&format!("prove-trace: using circuit_k={k}"));
    let params = KZGCommitmentScheme::<Bls12>::gen_params(k);
    append_backend_log("prove-trace: deserializing BaseProvingKey");
    let base_pk = BaseProvingKey::<Bls12HostField, KZGCommitmentScheme<Bls12>>::read::<
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
    append_backend_log("prove-trace: entering create_proof_trace_from_base");
    let trace = create_proof_trace_from_base::<Bls12HostField, KZGCommitmentScheme<Bls12>, _, _>(
      &params,
      &base_pk,
      std::slice::from_ref(&hosted),
      0,
      &instances,
      OsRng,
      &mut transcript,
    )
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("midnight BLS12 create_proof_trace_from_base failed: {error}"),
    })?;
    append_backend_log("prove-trace: create_proof_trace_from_base complete");

    append_backend_log("prove-trace: serializing persisted prover trace");
    trace.write(trace_writer, base_pk.get_vk()).map_err(|error| {
      OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("failed to serialize BLS12 persisted prover trace: {error}"),
      }
    })?;
    append_backend_log("prove-trace: trace serialization complete");
    Ok(())
  }

  /// Finalizes one BLS12-hosted outer proof bundle from a previously persisted
  /// proving-trace artifact plus the setup proving-key sidecar.
  pub fn produce_proof_bundle_from_trace_reader<R: io::Read, TR: io::Read>(
    self,
    package: &WrapperExecutionPackage,
    circuit: &OuterWrapperCircuit,
    setup: &ProducedOuterSetupArtifactBundle,
    proving_key_reader: &mut R,
    trace_reader: &mut TR,
  ) -> Result<ProducedOuterProofArtifactBundle, OuterProofBackendError> {
    append_backend_event("backend_finalize", "validate_setup_vk", "start", "");
    self.validate_setup_verification_key(package, &setup.verification_key)?;
    append_backend_event("backend_finalize", "validate_setup_vk", "end", "");

    let hosted = circuit.hosted_bls12();
    let k = k_from_circuit(&hosted);
    append_backend_event("backend_finalize", "context", "point", &format!("circuit_k={k}"));
    let params = KZGCommitmentScheme::<Bls12>::gen_params(k);
    let deserialize_base_pk_started_at = Instant::now();
    append_backend_event(
      "backend_finalize",
      "deserialize_base_pk",
      "start",
      &format!("circuit_k={k}"),
    );
    let base_pk = BaseProvingKey::<Bls12HostField, KZGCommitmentScheme<Bls12>>::read::<
      _,
      HostedOuterWrapperCircuitBls12,
    >(proving_key_reader, SerdeFormat::Processed, ())
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("failed to deserialize BLS12 proving key: {error}"),
    })?;
    append_backend_elapsed(
      "backend_finalize",
      "deserialize_base_pk",
      deserialize_base_pk_started_at,
      &format!("circuit_k={k}"),
    );

    let deserialize_prepared_trace_started_at = Instant::now();
    append_backend_event(
      "backend_finalize",
      "deserialize_prepared_trace",
      "start",
      &format!("circuit_k={k}"),
    );
    let prepared_trace = PersistedProverTrace::<Bls12HostField>::read_prepared(trace_reader)
      .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
        reason: format!("failed to deserialize BLS12 persisted prover trace: {error}"),
      })?;
    append_backend_elapsed(
      "backend_finalize",
      "deserialize_prepared_trace",
      deserialize_prepared_trace_started_at,
      &format!("circuit_k={k}"),
    );

    let init_transcript_started_at = Instant::now();
    append_backend_event("backend_finalize", "init_transcript", "start", &format!("circuit_k={k}"));
    let mut transcript = prepared_trace.init_transcript::<CircuitTranscript<Blake2bState>>();
    append_backend_elapsed(
      "backend_finalize",
      "init_transcript",
      init_transcript_started_at,
      &format!("circuit_k={k}"),
    );

    let finalize_from_trace_started_at = Instant::now();
    append_backend_event(
      "backend_finalize",
      "finalise_proof_from_base_trace",
      "start",
      &format!("circuit_k={k}"),
    );
    finalise_proof_from_base_trace::<Bls12HostField, KZGCommitmentScheme<Bls12>, _, _>(
      &params,
      base_pk,
      0,
      prepared_trace,
      trace_reader,
      &mut transcript,
    )
    .map_err(|error| OuterProofBackendError::OuterCircuitInputInvalid {
      reason: format!("midnight BLS12 finalise_proof_from_base_trace failed: {error}"),
    })?;
    append_backend_elapsed(
      "backend_finalize",
      "finalise_proof_from_base_trace",
      finalize_from_trace_started_at,
      &format!("circuit_k={k}"),
    );

    let serialize_validate_proof_started_at = Instant::now();
    append_backend_event(
      "backend_finalize",
      "serialize_and_validate_proof",
      "start",
      &format!("circuit_k={k}"),
    );
    let proof =
      self.metadata().proof_serialization().materialize(hex_encode(&transcript.finalize()));
    self.validate_produced_proof(package, &proof)?;
    append_backend_elapsed(
      "backend_finalize",
      "serialize_and_validate_proof",
      serialize_validate_proof_started_at,
      &format!("circuit_k={k}"),
    );
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
