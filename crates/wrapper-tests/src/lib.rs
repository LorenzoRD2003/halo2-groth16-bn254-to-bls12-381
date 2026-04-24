//! Test harness crate for workspace-level fixtures and integration helpers.
#![allow(clippy::multiple_crate_versions)]

use wrapper_backends::{
  ArtifactSetLoader, Groth16Bn254ArtifactBundle, MidnightDirectOuterBackendBls12Host,
  MidnightDirectOuterBackendBn254Host, OuterCircuitInputArtifacts,
  SnarkjsGroth16Bn254ArtifactSetLoader, parse_snarkjs_groth16_bn254_bundle_with_names,
};
use wrapper_circuits::{
  HostedOuterWrapperCircuitBls12, HostedOuterWrapperCircuitBn254, groth16_fixture_raw,
};
use wrapper_core as _;

#[cfg(test)]
use criterion as _;
#[cfg(test)]
use midnight_proofs as _;

/// Ordered public-input names for the committed Semaphore fixture.
pub const SEMAPHORE_PUBLIC_INPUT_NAMES: [&str; 4] =
  ["merkle_root", "nullifier", "message_hash", "scope_hash"];

/// Committed Groth16 fixtures that can feed the canonical outer wrapper lane.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OuterBenchFixture {
  /// The small `circom` / `snarkjs` `multiplier2` fixture.
  CircomMultiplier2,
  /// The real Semaphore fixture committed for migration / end-to-end coverage.
  SemaphoreDepth10,
}

impl OuterBenchFixture {
  /// Stable artifact identifier used when parsing the fixture bundle.
  #[must_use]
  pub fn artifact_id(self) -> &'static str {
    match self {
      Self::CircomMultiplier2 => "circom-multiplier2",
      Self::SemaphoreDepth10 => "semaphore-depth-10",
    }
  }

  /// Stable slug for benchmark and profiling labels.
  #[must_use]
  pub fn slug(self) -> &'static str {
    match self {
      Self::CircomMultiplier2 => "circom_multiplier2",
      Self::SemaphoreDepth10 => "semaphore",
    }
  }
}

/// Returns the example config bundled for integration tests.
#[must_use]
pub fn example_config() -> &'static str {
  include_str!("../fixtures/example-config.toml")
}

/// Loads one committed Groth16 fixture bundle.
#[must_use]
pub fn load_outer_bench_fixture_bundle(fixture: OuterBenchFixture) -> Groth16Bn254ArtifactBundle {
  match fixture {
    OuterBenchFixture::CircomMultiplier2 => {
      let loader = SnarkjsGroth16Bn254ArtifactSetLoader;
      loader
        .load_artifact_set(
          fixture.artifact_id(),
          groth16_fixture_raw::proof_json(),
          groth16_fixture_raw::public_inputs_json(),
          groth16_fixture_raw::verification_key_json(),
        )
        .expect("canonical multiplier2 fixture bundle should parse")
    }
    OuterBenchFixture::SemaphoreDepth10 => parse_snarkjs_groth16_bn254_bundle_with_names(
      fixture.artifact_id(),
      include_bytes!("../fixtures/groth16/semaphore/proof.json"),
      include_bytes!("../fixtures/groth16/semaphore/public.json"),
      include_bytes!("../fixtures/groth16/semaphore/verification_key.json"),
      &SEMAPHORE_PUBLIC_INPUT_NAMES,
    )
    .expect("named Semaphore fixture bundle should parse"),
  }
}

/// Builds the canonical BN254-hosted outer circuit for one committed fixture.
#[must_use]
pub fn build_outer_bench_circuit_bn254(
  fixture: OuterBenchFixture,
) -> HostedOuterWrapperCircuitBn254 {
  let bundle = load_outer_bench_fixture_bundle(fixture);
  let package = bundle.build_halo2_outer_execution_package();
  let backend = MidnightDirectOuterBackendBn254Host;
  let circuit = backend
    .build_outer_circuit(
      &package,
      OuterCircuitInputArtifacts::new(
        Some(match fixture {
          OuterBenchFixture::CircomMultiplier2 => groth16_fixture_raw::proof_json(),
          OuterBenchFixture::SemaphoreDepth10 => {
            include_bytes!("../fixtures/groth16/semaphore/proof.json")
          }
        }),
        Some(match fixture {
          OuterBenchFixture::CircomMultiplier2 => groth16_fixture_raw::verification_key_json(),
          OuterBenchFixture::SemaphoreDepth10 => {
            include_bytes!("../fixtures/groth16/semaphore/verification_key.json")
          }
        }),
      ),
    )
    .expect("BN254-hosted outer benchmark circuit should build");
  circuit.into_hosted_bn254()
}

/// Builds the canonical BLS12-381-hosted outer circuit for one committed fixture.
#[must_use]
pub fn build_outer_bench_circuit_bls12(
  fixture: OuterBenchFixture,
) -> HostedOuterWrapperCircuitBls12 {
  let bundle = load_outer_bench_fixture_bundle(fixture);
  let package = bundle.build_halo2_outer_execution_package();
  let backend = MidnightDirectOuterBackendBls12Host;
  let circuit = backend
    .build_outer_circuit(
      &package,
      OuterCircuitInputArtifacts::new(
        Some(match fixture {
          OuterBenchFixture::CircomMultiplier2 => groth16_fixture_raw::proof_json(),
          OuterBenchFixture::SemaphoreDepth10 => {
            include_bytes!("../fixtures/groth16/semaphore/proof.json")
          }
        }),
        Some(match fixture {
          OuterBenchFixture::CircomMultiplier2 => groth16_fixture_raw::verification_key_json(),
          OuterBenchFixture::SemaphoreDepth10 => {
            include_bytes!("../fixtures/groth16/semaphore/verification_key.json")
          }
        }),
      ),
    )
    .expect("BLS12-hosted outer benchmark circuit should build");
  circuit.into_hosted_bls12()
}

#[cfg(test)]
mod tests {
  use super::SEMAPHORE_PUBLIC_INPUT_NAMES;
  use wrapper_backends::{
    ArtifactSetLoader, BackendRegistry, Groth16Bn254ArtifactBundle, MidnightDirectOuterBackend,
    OuterCircuitInputArtifacts, OuterProofBackend, PlannedHalo2OuterBackend,
    SnarkjsGroth16Bn254ArtifactSetLoader, parse_snarkjs_groth16_bn254_bundle_with_names,
  };
  use wrapper_circuits::{
    CircuitBuildStatus, CircuitPlanningView, NativeField, groth16_fixture_raw,
    groth16_fixture_typed, host_verify,
  };
  use wrapper_core::{
    NamedPublicInput, NamedPublicInputs, OuterStatementSemantics, ProjectConfig, ProofSystemKind,
  };

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

  fn load_groth16_fixture() -> Groth16Bn254ArtifactBundle {
    let loader = SnarkjsGroth16Bn254ArtifactSetLoader;

    loader
      .load_artifact_set(
        "circom-multiplier2",
        groth16_fixture_raw::proof_json(),
        groth16_fixture_raw::public_inputs_json(),
        groth16_fixture_raw::verification_key_json(),
      )
      .expect("fixture bundle should parse")
  }

  fn load_semaphore_fixture() -> Groth16Bn254ArtifactBundle {
    let loader = SnarkjsGroth16Bn254ArtifactSetLoader;

    loader
      .load_artifact_set(
        "semaphore-depth-10",
        include_bytes!("../fixtures/groth16/semaphore/proof.json"),
        include_bytes!("../fixtures/groth16/semaphore/public.json"),
        include_bytes!("../fixtures/groth16/semaphore/verification_key.json"),
      )
      .expect("Semaphore fixture bundle should parse")
  }

  fn load_named_semaphore_public_inputs() -> NamedPublicInputs {
    parse_snarkjs_groth16_bn254_bundle_with_names(
      "semaphore-depth-10",
      include_bytes!("../fixtures/groth16/semaphore/proof.json"),
      include_bytes!("../fixtures/groth16/semaphore/public.json"),
      include_bytes!("../fixtures/groth16/semaphore/verification_key.json"),
      &SEMAPHORE_PUBLIC_INPUT_NAMES,
    )
    .expect("named Semaphore bundle should parse")
    .named_public_inputs
    .expect("named Semaphore bundle should expose named public inputs")
  }

  fn assert_canonical_fixture_public_inputs(public_inputs: &[NativeField]) {
    assert_eq!(public_inputs, groth16_fixture_typed::public_inputs());
  }

  #[test]
  fn groth16_real_snarkjs_fixture_is_accepted_end_to_end() {
    let bundle = load_groth16_fixture();
    assert_canonical_fixture_public_inputs(&bundle.public_inputs);

    assert!(host_verify(&bundle.verification_key, &bundle.proof, &bundle.public_inputs,));
  }

  #[test]
  fn groth16_mutated_public_input_is_rejected_end_to_end() {
    let bundle = load_groth16_fixture();
    let mut public_inputs = bundle.public_inputs.clone();
    public_inputs[0] = NativeField::from(34_u64);

    assert!(!host_verify(&bundle.verification_key, &bundle.proof, &public_inputs,));
  }

  #[test]
  fn semaphore_snarkjs_fixture_is_accepted_end_to_end() {
    let bundle = load_semaphore_fixture();

    assert_eq!(bundle.public_inputs.len(), 4);
    assert_eq!(bundle.verification_key.ic.len(), 5);
    assert!(host_verify(&bundle.verification_key, &bundle.proof, &bundle.public_inputs,));
  }

  #[test]
  fn semaphore_mutated_public_input_is_rejected_end_to_end() {
    let bundle = load_semaphore_fixture();
    let mut public_inputs = bundle.public_inputs.clone();
    public_inputs[1] += NativeField::from(1_u64);

    assert!(!host_verify(&bundle.verification_key, &bundle.proof, &public_inputs,));
  }

  #[test]
  fn semaphore_fixture_public_inputs_can_be_named_at_fixture_layer() {
    let named = load_named_semaphore_public_inputs();

    assert_eq!(named.field_order(), SEMAPHORE_PUBLIC_INPUT_NAMES);
    assert_eq!(
      named,
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
  fn semaphore_execution_package_can_materialize_placeholder_outer_bundle() {
    let bundle = load_semaphore_fixture();
    let package = bundle.build_halo2_outer_execution_package();
    let backend = PlannedHalo2OuterBackend;
    let planned_output =
      backend.prepare(&package).expect("planned outer backend should accept the Semaphore package");
    let outer_bundle = planned_output.bundle_template;

    assert_eq!(outer_bundle.proof_system.kind, ProofSystemKind::Halo2Outer);
    assert_eq!(outer_bundle.proof_artifact, "semaphore-depth-10-wrapper-proof.json");
    assert_eq!(outer_bundle.public_inputs.len(), 4);
    assert_eq!(outer_bundle.public_inputs_artifact, "semaphore-depth-10-wrapper-public.json");
    assert_eq!(
      outer_bundle.verification_key_artifact,
      "semaphore-depth-10-wrapper-verification-key.json"
    );
    assert_eq!(outer_bundle.canonical_circuit_identity, None);
    assert!(outer_bundle.proof.is_none());
    let verification_key = outer_bundle
      .verification_key
      .as_ref()
      .expect("placeholder outer backend should materialize a VK skeleton");
    assert_eq!(verification_key.protocol, "halo2-plonkish");
    assert_eq!(verification_key.curve, "bn254");
    assert_eq!(verification_key.public_input_count, 4);
  }

  #[test]
  fn semaphore_execution_package_can_prepare_selected_midnight_outer_lane() {
    let bundle = load_semaphore_fixture();
    let package = bundle.build_halo2_outer_execution_package();
    let backend = MidnightDirectOuterBackend;
    let planned_output = backend
      .prepare(&package)
      .expect("selected midnight backend should accept the Semaphore package");

    assert!(planned_output
      .notes
      .iter()
      .any(|note| note.contains("selected outer backend stack: direct halo2/midnight outer lane over the canonical outer wrapper circuit")));
    assert!(planned_output.notes.iter().any(|note| note.contains(
      "outer statement contract is frozen to mirror ordered inner verifier public inputs"
    )));
    assert!(planned_output.notes.iter().any(|note| note.contains("midnight_proofs keygen")));
    assert_eq!(backend.metadata().curve, "bn254");
  }

  #[test]
  fn semaphore_execution_package_exposes_mirrored_outer_statement_contract() {
    let bundle = load_semaphore_fixture();
    let package = bundle.build_halo2_outer_execution_package();
    let contract = package
      .validate_outer_statement_contract()
      .expect("Semaphore package should satisfy the frozen outer-statement contract");

    assert_eq!(contract.semantics, OuterStatementSemantics::MirrorInnerVerifierPublicInputs);
    assert_eq!(contract.expected_outer_public_input_count, 4);
    assert_eq!(contract.expected_inner_public_input_count, 4);
    assert_eq!(contract.expected_verification_key_ic_count, 5);
  }

  #[test]
  fn canonical_fixture_package_can_adapt_into_direct_outer_input() {
    let bundle = load_groth16_fixture();
    let package = bundle.build_halo2_outer_execution_package();
    let backend = MidnightDirectOuterBackend;
    let adapted = backend
      .adapt_input(
        &package,
        OuterCircuitInputArtifacts::new(
          Some(groth16_fixture_raw::proof_json()),
          Some(groth16_fixture_raw::verification_key_json()),
        ),
      )
      .expect("midnight backend should adapt the canonical fixture package");

    assert_eq!(adapted.source_artifact_id, "circom-multiplier2");
    assert_eq!(adapted.inner_verifier_public_inputs, adapted.outer_statement.public_inputs);
    assert_eq!(adapted.outer_statement.field_names, vec!["public_input_0".to_owned()]);
  }

  #[test]
  fn canonical_fixture_package_can_build_outer_circuit_through_backend() {
    let bundle = load_groth16_fixture();
    let package = bundle.build_halo2_outer_execution_package();
    let backend = MidnightDirectOuterBackend;
    let circuit = backend
      .build_outer_circuit(
        &package,
        OuterCircuitInputArtifacts::new(
          Some(groth16_fixture_raw::proof_json()),
          Some(groth16_fixture_raw::verification_key_json()),
        ),
      )
      .expect("backend should build a ready outer circuit for the canonical fixture");

    assert_eq!(circuit.build_status(), CircuitBuildStatus::VerifierIntegrated);
    circuit
      .assert_ready_for_synthesis()
      .expect("backend-built outer circuit should be ready for synthesis");
  }

  #[test]
  fn semaphore_fixture_package_can_build_outer_circuit_through_backend() {
    let bundle = load_semaphore_fixture();
    let package = bundle.build_halo2_outer_execution_package();
    let backend = MidnightDirectOuterBackend;
    let circuit = backend
      .build_outer_circuit(
        &package,
        OuterCircuitInputArtifacts::new(
          Some(include_bytes!("../fixtures/groth16/semaphore/proof.json")),
          Some(include_bytes!("../fixtures/groth16/semaphore/verification_key.json")),
        ),
      )
      .expect("backend should build a ready outer circuit for the Semaphore fixture");

    assert_eq!(circuit.build_status(), CircuitBuildStatus::VerifierIntegrated);
  }

  #[test]
  fn semaphore_named_inputs_are_preserved_by_outer_input_adapter() {
    let bundle = parse_snarkjs_groth16_bn254_bundle_with_names(
      "semaphore-depth-10",
      include_bytes!("../fixtures/groth16/semaphore/proof.json"),
      include_bytes!("../fixtures/groth16/semaphore/public.json"),
      include_bytes!("../fixtures/groth16/semaphore/verification_key.json"),
      &SEMAPHORE_PUBLIC_INPUT_NAMES,
    )
    .expect("named Semaphore bundle should parse");
    let package = bundle.build_halo2_outer_execution_package();
    let backend = MidnightDirectOuterBackend;
    let adapted = backend
      .adapt_input(
        &package,
        OuterCircuitInputArtifacts::new(
          Some(include_bytes!("../fixtures/groth16/semaphore/proof.json")),
          Some(include_bytes!("../fixtures/groth16/semaphore/verification_key.json")),
        ),
      )
      .expect("midnight backend should adapt the Semaphore package");

    assert_eq!(adapted.outer_statement.field_names, SEMAPHORE_PUBLIC_INPUT_NAMES);
    assert_eq!(adapted.outer_statement.public_inputs, adapted.inner_verifier_public_inputs);
  }

  #[test]
  #[ignore = "slow outer proving"]
  fn canonical_fixture_package_can_produce_real_outer_proof_bundle() {
    let bundle = load_groth16_fixture();
    let package = bundle.build_halo2_outer_execution_package();
    let backend = MidnightDirectOuterBackend;
    let produced = backend
      .prove(
        &package,
        OuterCircuitInputArtifacts::new(
          Some(groth16_fixture_raw::proof_json()),
          Some(groth16_fixture_raw::verification_key_json()),
        ),
      )
      .expect("backend should produce a real direct outer proof bundle");

    assert_eq!(produced.proof.protocol, "halo2-plonkish");
    assert_eq!(produced.proof.curve, "bn254");
    assert_eq!(produced.verification_key.pcs, "kzg");
    assert_eq!(
      produced.public_inputs,
      package
        .statement
        .public_inputs
        .entries
        .iter()
        .map(|entry| entry.value.clone())
        .collect::<Vec<_>>()
    );
  }

  #[test]
  #[ignore = "slow outer proving"]
  fn canonical_fixture_package_can_verify_real_outer_proof_bundle() {
    let bundle = load_groth16_fixture();
    let package = bundle.build_halo2_outer_execution_package();
    let backend = MidnightDirectOuterBackend;
    let produced = backend
      .prove(
        &package,
        OuterCircuitInputArtifacts::new(
          Some(groth16_fixture_raw::proof_json()),
          Some(groth16_fixture_raw::verification_key_json()),
        ),
      )
      .expect("backend should produce a real direct outer proof bundle");

    assert!(
      backend
        .verify(
          &package,
          &produced,
          OuterCircuitInputArtifacts::new(
            Some(groth16_fixture_raw::proof_json()),
            Some(groth16_fixture_raw::verification_key_json()),
          ),
        )
        .expect("backend should verify the produced direct outer proof bundle")
    );
  }

  #[test]
  #[ignore = "slow outer proving"]
  fn semaphore_fixture_runs_real_end_to_end_outer_flow() {
    let bundle = load_semaphore_fixture();
    let package = bundle.build_halo2_outer_execution_package();
    let backend = MidnightDirectOuterBackend;
    let artifacts = OuterCircuitInputArtifacts::new(
      Some(include_bytes!("../fixtures/groth16/semaphore/proof.json")),
      Some(include_bytes!("../fixtures/groth16/semaphore/verification_key.json")),
    );

    let verification_key = backend
      .setup(&package, artifacts)
      .expect("setup should produce a real VK artifact for the Semaphore fixture");
    let produced = backend
      .prove(&package, artifacts)
      .expect("prove should produce a real direct outer proof bundle for the Semaphore fixture");

    assert_eq!(verification_key.protocol, "halo2-plonkish");
    assert_eq!(verification_key.curve, "bn254");
    assert_eq!(verification_key.public_input_count, 4);
    assert_eq!(produced.proof_artifact, "semaphore-depth-10-wrapper-proof.json");
    assert_eq!(produced.public_inputs_artifact, "semaphore-depth-10-wrapper-public.json");
    assert_eq!(
      produced.verification_key_artifact,
      "semaphore-depth-10-wrapper-verification-key.json"
    );
    assert_eq!(
      produced.public_inputs,
      package
        .statement
        .public_inputs
        .entries
        .iter()
        .map(|entry| entry.value.clone())
        .collect::<Vec<_>>()
    );
    assert_eq!(produced.verification_key.public_input_count, 4);
    assert!(
      backend
        .verify(&package, &produced, artifacts)
        .expect("verify should accept the produced Semaphore outer proof bundle")
    );
  }
}
