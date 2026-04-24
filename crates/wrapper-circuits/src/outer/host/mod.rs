//! Explicit outer-host boundaries for the canonical outer wrapper circuit.

mod bls12_381;
mod bn254;

use crate::NativeField;

pub use bls12_381::{MidnightBls12_381HostConfigShell, MidnightBls12_381HostLane};
pub use bn254::{MidnightBn254HostConfig, MidnightBn254HostLane};

/// Current host field used by the wired outer circuit lane.
pub type OuterHostField = NativeField;

/// Current host config used by the wired outer circuit lane.
pub type OuterHostConfig = MidnightBn254HostConfig;

/// Inner verifier family consumed by the canonical outer wrapper circuit.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InnerVerifierFlavor {
  /// Groth16 over BN254, currently sourced from `snarkjs`-compatible artifacts.
  Groth16Bn254,
}

impl InnerVerifierFlavor {
  /// Returns a stable identifier for the selected verifier semantics.
  #[must_use]
  pub const fn id(self) -> &'static str {
    match self {
      Self::Groth16Bn254 => "groth16-bn254",
    }
  }
}

/// Stable trait describing one outer proving host lane.
pub trait OuterHostLane {
  /// Host field used by the outer proof system.
  type Field;

  /// Returns the stable outer host flavor for this lane.
  fn flavor() -> OuterHostFlavor;

  /// Returns the outer proof-system protocol label for this host lane.
  fn protocol() -> &'static str;

  /// Returns the host curve label for this lane.
  fn curve() -> &'static str;

  /// Returns the polynomial-commitment scheme label for this lane.
  fn pcs() -> &'static str;

  /// Returns the transcript family label for this lane.
  fn transcript() -> &'static str;

  /// Returns whether the current repository can synthesize the canonical outer
  /// circuit on this host lane today.
  fn supports_current_canonical_circuit() -> bool;
}

/// Host proving lane selected for the outer proof.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OuterHostFlavor {
  /// Current Halo2/Midnight host lane over BN254.
  MidnightBn254,
  /// Planned Halo2/Midnight host lane over BLS12-381.
  MidnightBls12_381,
}

impl OuterHostFlavor {
  /// Returns the outer proof-system protocol label for the selected host lane.
  #[must_use]
  pub const fn protocol(self) -> &'static str {
    match self {
      Self::MidnightBn254 => "halo2-plonkish",
      Self::MidnightBls12_381 => "halo2-plonkish",
    }
  }

  /// Returns a stable identifier for the selected outer host lane.
  #[must_use]
  pub const fn id(self) -> &'static str {
    match self {
      Self::MidnightBn254 => "midnight-bn254-host",
      Self::MidnightBls12_381 => "midnight-bls12-381-host",
    }
  }

  /// Returns the host curve label for the selected outer proving lane.
  #[must_use]
  pub const fn curve(self) -> &'static str {
    match self {
      Self::MidnightBn254 => "bn254",
      Self::MidnightBls12_381 => "bls12-381",
    }
  }

  /// Returns the polynomial-commitment scheme label for the selected host lane.
  #[must_use]
  pub const fn pcs(self) -> &'static str {
    match self {
      Self::MidnightBn254 => "kzg",
      Self::MidnightBls12_381 => "kzg",
    }
  }

  /// Returns the transcript family label for the selected host lane.
  #[must_use]
  pub const fn transcript(self) -> &'static str {
    match self {
      Self::MidnightBn254 => "blake2b",
      Self::MidnightBls12_381 => "blake2b",
    }
  }

  /// Returns whether the current repository can synthesize the canonical outer
  /// circuit on this host lane today.
  #[must_use]
  pub const fn supports_current_canonical_circuit(self) -> bool {
    matches!(self, Self::MidnightBn254)
  }
}

/// Serialization contract for produced outer proof artifacts.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OuterArtifactSerializationFlavor {
  /// `serde` JSON carrying hex-encoded processed proof-system payloads.
  SerdeJsonHexEncodedProcessed,
}

impl OuterArtifactSerializationFlavor {
  /// Returns the payload encoding label used by this serialization family.
  #[must_use]
  pub const fn payload_encoding(self) -> &'static str {
    match self {
      Self::SerdeJsonHexEncodedProcessed => "hex",
    }
  }

  /// Returns a stable identifier for the serialization contract.
  #[must_use]
  pub const fn id(self) -> &'static str {
    match self {
      Self::SerdeJsonHexEncodedProcessed => "serde-json-hex-processed",
    }
  }
}

/// Explicit flavor profile tying inner verifier semantics to one outer host
/// lane and one artifact serialization contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OuterWrapperFlavorProfile {
  /// Selected inner verifier semantics.
  pub inner_verifier: InnerVerifierFlavor,
  /// Selected outer host lane.
  pub outer_host: OuterHostFlavor,
  /// Selected outer artifact serialization contract.
  pub serialization: OuterArtifactSerializationFlavor,
}

impl OuterWrapperFlavorProfile {
  /// Returns the current compatible repo-default flavor profile.
  #[must_use]
  pub const fn current() -> Self {
    Self {
      inner_verifier: InnerVerifierFlavor::Groth16Bn254,
      outer_host: OuterHostFlavor::MidnightBn254,
      serialization: OuterArtifactSerializationFlavor::SerdeJsonHexEncodedProcessed,
    }
  }

  /// Returns a copy of the current profile with a different outer host lane.
  #[must_use]
  pub const fn with_outer_host(self, outer_host: OuterHostFlavor) -> Self {
    Self { outer_host, ..self }
  }
}
