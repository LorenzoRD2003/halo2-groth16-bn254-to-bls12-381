//! Backend integration surfaces for proof and verification-key artifacts.

pub mod groth16;
pub mod loader;
pub mod registry;
pub mod snarkjs;

pub use groth16::{
  Groth16Bn254ArtifactBundle, SnarkjsGroth16Bn254ArtifactSetLoader,
  parse_snarkjs_groth16_bn254_bundle,
  parse_snarkjs_groth16_bn254_bundle_with_names,
};
pub use loader::{ArtifactLoader, ArtifactLoaderError, ArtifactSetLoader, LoaderSummary};
pub use registry::{BackendDescriptor, BackendRegistry};
pub use snarkjs::{
  SnarkjsGroth16ParseError, parse_groth16_bn254_proof, parse_groth16_bn254_public_inputs,
  parse_groth16_bn254_public_inputs_with_names, parse_groth16_bn254_verifying_key,
};
