//! Backend integration surfaces for proof and verification-key artifacts.

pub mod loader;
pub mod registry;
pub mod snarkjs;

pub use loader::{ArtifactLoader, ArtifactLoaderError, LoaderSummary};
pub use registry::{BackendDescriptor, BackendRegistry};
pub use snarkjs::{
  SnarkjsGroth16ParseError, parse_groth16_bn254_proof, parse_groth16_bn254_public_inputs,
  parse_groth16_bn254_verifying_key,
};
