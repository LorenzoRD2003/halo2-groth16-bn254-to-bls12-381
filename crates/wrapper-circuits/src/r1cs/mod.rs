//! Canonical R1CS data model and deterministic Halo2 lowering boundary.
//!
//! The current module supports:
//!
//! - canonical sparse linear combinations
//! - deterministic constraint ordering
//! - deterministic Halo2 cell identity
//! - equality/copy lowering via variable unification only
//! - a narrow lowering boundary from Halo2-style cells into canonical R1CS
//!
//! The current module intentionally does not support lookups, permutations, or
//! full Halo2 circuit introspection.

mod arkworks;
mod builder;
mod cells;
mod equality;
mod error;
mod identity;
mod lowering;
mod metadata;
mod model;
pub mod non_native;
mod zkinterface;

pub use arkworks::{
  ArkworksPreparedVerifyingKey, ArkworksProof, ArkworksProvingKey, ArkworksR1csCircuit,
  ArkworksVerifyingKey, R1csAssignment, arkworks_create_random_proof,
  arkworks_generate_random_parameters, arkworks_verify_proof, ordered_public_inputs, to_ark_lc,
};
pub use builder::CanonicalR1csBuilder;
pub use cells::{
  CanonicalClassId, EqualityEdge, Halo2CellLinearCombination, Halo2CellRef, Halo2CellTerm,
};
pub use equality::{CanonicalCellUnionFind, Halo2CellAssignmentMap};
pub use error::R1csBuildError;
pub use identity::{R1CS_IDENTITY_DOMAIN_SEPARATOR, R1csIdentityHash};
pub use lowering::Halo2Phase1R1csLowering;
pub use metadata::{Halo2PublicInputRef, Halo2R1csMetadata};
pub use model::{LinearCombination, LinearTerm, R1csCircuit, R1csConstraint, VariableId};
pub use zkinterface::{
  ZkInterfaceConstraint, ZkInterfaceLinearCombination, ZkInterfaceR1csExport, ZkInterfaceTerm,
  ZkInterfaceWitnessAssignment, ZkInterfaceWitnessExport, export_witness,
};

#[cfg(test)]
mod tests;
