use crate::NativeField;

use super::{OuterHostFlavor, OuterHostLane};

/// Placeholder Halo2/Midnight outer host lane over BLS12-381.
///
/// This is intentionally only a metadata/config shell in the current
/// repository phase. It exists so future work can land as an additive sibling
/// to the BN254-hosted lane instead of rewriting the current host module.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MidnightBls12_381HostLane;

impl OuterHostLane for MidnightBls12_381HostLane {
  type Field = NativeField;

  fn flavor() -> OuterHostFlavor {
    OuterHostFlavor::MidnightBls12_381
  }

  fn protocol() -> &'static str {
    Self::flavor().protocol()
  }

  fn curve() -> &'static str {
    Self::flavor().curve()
  }

  fn pcs() -> &'static str {
    Self::flavor().pcs()
  }

  fn transcript() -> &'static str {
    Self::flavor().transcript()
  }

  fn supports_current_canonical_circuit() -> bool {
    false
  }
}

/// Placeholder config shell for the planned BLS12-381-hosted outer lane.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MidnightBls12_381HostConfigShell;

impl MidnightBls12_381HostConfigShell {
  /// Returns the host flavor represented by this placeholder config shell.
  #[must_use]
  pub const fn flavor(self) -> OuterHostFlavor {
    OuterHostFlavor::MidnightBls12_381
  }
}
