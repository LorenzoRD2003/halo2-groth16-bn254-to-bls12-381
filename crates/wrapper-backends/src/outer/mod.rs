//! Outer proof backend contracts, backend metadata, and input adapters.

mod compatibility;
mod direct;
mod errors;
mod helpers;
mod planned;
mod traits;
mod types;

pub use compatibility::{
  PlannedHalo2OuterBackendBn254Host, current_reference_outer_backend,
  current_reference_outer_backend_metadata, current_reference_outer_host,
};
pub use direct::{MidnightDirectOuterBackendBls12Host, MidnightDirectOuterBackendBn254Host};
pub use errors::OuterProofBackendError;
pub use planned::PlannedHalo2OuterBackend;
pub use traits::{CanonicalOuterCircuitProofBackend, OuterProofBackend};
pub use types::{
  CanonicalOuterCircuitProofArtifacts, CanonicalOuterCircuitSetupArtifacts,
  DirectOuterCircuitInput, DirectOuterProofPlan, DirectOuterSetupPlan, DirectOuterStatementInput,
  OuterBackendCapabilities, OuterCircuitInputArtifacts, OuterProofBackendMetadata,
  OuterProofSerialization, OuterVerificationKeySerialization, ProducedOuterProvingKeyJson,
  ProducedOuterSetupArtifactBundle,
};

#[cfg(test)]
mod tests;
