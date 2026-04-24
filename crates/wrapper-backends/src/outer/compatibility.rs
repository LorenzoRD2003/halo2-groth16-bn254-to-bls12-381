//! Compatibility-reference surface for the current BN254-hosted outer lane.

use wrapper_circuits::OuterHostFlavor;

use super::{
  MidnightDirectOuterBackendBn254Host, OuterProofBackend, OuterProofBackendMetadata,
  PlannedHalo2OuterBackend,
};

/// Stable alias for the current planned BN254-hosted outer backend reference lane.
pub type PlannedHalo2OuterBackendBn254Host = PlannedHalo2OuterBackend;

/// Returns the current reference outer host lane for compatibility checks.
#[must_use]
pub const fn current_reference_outer_host() -> OuterHostFlavor {
  OuterHostFlavor::MidnightBn254
}

/// Returns the current direct BN254-hosted reference backend.
#[must_use]
pub const fn current_reference_outer_backend() -> MidnightDirectOuterBackendBn254Host {
  MidnightDirectOuterBackendBn254Host
}

/// Returns static metadata for the current direct BN254-hosted reference backend.
#[must_use]
pub fn current_reference_outer_backend_metadata() -> &'static OuterProofBackendMetadata {
  current_reference_outer_backend().metadata()
}
