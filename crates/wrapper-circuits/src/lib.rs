//! Halo2-facing circuit foundations.
//!
//! Week 1 now wires BN254 foreign-field and minimal G1 operations to real
//! Midnight/Halo2 chips, together with lightweight layout reporting. Week 2
//! has started with a narrow BN254 Fp2 layer, now organized under the
//! `bn254/` module. Pairings, verifier logic, and G2 remain intentionally out
//! of scope.
#![allow(clippy::multiple_crate_versions)]

use ff as _;
use halo2curves as _;

mod bn254;
pub mod metrics;
pub mod outer;
pub mod planning;

pub use bn254::{
  AssignedFp, AssignedFp2, AssignedG1, Bn254EccChip, Bn254FpChip, Fp2AddCircuit, Fp2MulCircuit,
  Fp2SquareCircuit, FpAddCircuit, FpMulCircuit, G1AddCircuit, G1OnCurveCircuit, fp_add_k,
  fp_add_layout_metrics, fp_mul_k, fp_mul_layout_metrics, fp2_add_k, fp2_add_layout_metrics,
  fp2_mul_k, fp2_mul_layout_metrics, fp2_square_k, fp2_square_layout_metrics, g1_add_k,
  g1_add_layout_metrics,
};
pub use metrics::{CostEstimate, LayoutMetrics};
pub use outer::{CircuitBuildStatus, OuterWrapperCircuit};
pub use planning::{CircuitPlanningView, PrimitiveCostTable};
