//! Backend integration surfaces for proof and verification-key artifacts.
#![allow(clippy::bool_assert_comparison)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::multiple_crate_versions)]
#![allow(clippy::needless_pass_by_value)]
#![allow(clippy::uninlined_format_args)]

pub mod groth16;
pub mod loader;
pub mod outer;
pub mod registry;
pub mod snarkjs;

pub use groth16::{
  Groth16Bn254ArtifactBundle, SnarkjsGroth16Bn254ArtifactSetLoader,
  parse_snarkjs_groth16_bn254_bundle, parse_snarkjs_groth16_bn254_bundle_with_names,
};
pub use loader::{ArtifactLoader, ArtifactLoaderError, ArtifactSetLoader, LoaderSummary};
pub use outer::{
  CanonicalOuterCircuitProofArtifacts, CanonicalOuterCircuitProofBackend,
  CanonicalOuterCircuitSetupArtifacts, DirectOuterCircuitInput, DirectOuterProofPlan,
  DirectOuterSetupPlan, DirectOuterStatementInput, MidnightDirectOuterBackend,
  MidnightDirectOuterBackendBls12Host, MidnightDirectOuterBackendBn254Host,
  OuterBackendCapabilities, OuterCircuitInputArtifacts, OuterProofBackend, OuterProofBackendError,
  OuterProofBackendMetadata, OuterProofSerialization, OuterVerificationKeySerialization,
  PlannedHalo2OuterBackend, PlannedHalo2OuterBackendBn254Host, current_reference_outer_backend,
  current_reference_outer_backend_metadata, current_reference_outer_host,
};
pub use registry::{BackendDescriptor, BackendRegistry};
pub use snarkjs::{
  SnarkjsGroth16ParseError, parse_groth16_bn254_proof, parse_groth16_bn254_public_inputs,
  parse_groth16_bn254_public_inputs_with_names, parse_groth16_bn254_verifying_key,
};
