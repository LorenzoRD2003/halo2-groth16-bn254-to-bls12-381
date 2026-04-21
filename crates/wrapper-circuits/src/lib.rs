//! Halo2-facing circuit foundations.
//!
//! Week 1 now wires BN254 foreign-field and minimal G1 operations to real
//! Midnight/Halo2 chips, together with lightweight layout reporting. The
//! current primitive surface is consolidated under `bn254.rs`. Pairings,
//! verifier logic, Fp2, and G2 remain intentionally out of scope.
#![allow(clippy::multiple_crate_versions)]

use ff as _;
use halo2curves as _;

mod bn254;
pub mod metrics;
pub mod outer;
pub mod planning;

pub use bn254::{
  AssignedFp, AssignedG1, Bn254EccChip, Bn254FpChip, FpAddCircuit, FpMulCircuit, G1AddCircuit,
  G1OnCurveCircuit, fp_add_k, fp_add_layout_metrics, fp_mul_k, fp_mul_layout_metrics, g1_add_k,
  g1_add_layout_metrics,
};
pub use metrics::{CostEstimate, LayoutMetrics};
pub use outer::{CircuitBuildStatus, OuterWrapperCircuit};
pub use planning::{CircuitPlanningView, PrimitiveCostTable};
