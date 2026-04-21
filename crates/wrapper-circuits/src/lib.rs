//! Halo2-facing circuit foundations.
//!
//! Week 1 now wires BN254 foreign-field and minimal G1 operations to real
//! Midnight/Halo2 chips, together with lightweight layout reporting. Pairings,
//! verifier logic, Fp2, and G2 remain intentionally out of scope.
#![allow(clippy::multiple_crate_versions)]

use ff as _;
use halo2curves as _;

mod bn254;
pub mod error;
pub mod fp;
pub mod g1;
pub mod metrics;
pub mod outer;
pub mod planning;

pub use error::CircuitError;
pub use fp::{
  AssignedFp, Bn254FpChip, FpAddCircuit, FpMulCircuit, fp_add_k, fp_add_layout_metrics, fp_mul_k,
  fp_mul_layout_metrics,
};
pub use g1::{
  AssignedG1, Bn254EccChip, G1AddCircuit, G1OnCurveCircuit, g1_add_k, g1_add_layout_metrics,
};
pub use metrics::{CostEstimate, LayoutMetrics};
pub use outer::{CircuitBuildStatus, OuterWrapperCircuit};
pub use planning::{CircuitPlanningView, PrimitiveCostTable};
