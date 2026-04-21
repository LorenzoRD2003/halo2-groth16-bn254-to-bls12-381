//! Backend integration placeholders for proof and verification-key artifacts.

pub mod loader;
pub mod registry;

pub use loader::{ArtifactLoader, ArtifactLoaderError, LoaderSummary};
pub use registry::{BackendDescriptor, BackendRegistry};
