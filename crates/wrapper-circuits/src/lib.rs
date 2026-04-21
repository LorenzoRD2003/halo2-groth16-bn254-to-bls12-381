//! Halo2-facing circuit skeletons.
//!
//! This crate intentionally avoids real cryptographic implementation during the
//! initialization phase. It exists to define module ownership and placeholder
//! interfaces for future circuit work.

pub mod outer;
pub mod planning;

pub use outer::{CircuitBuildStatus, OuterWrapperCircuit};
pub use planning::CircuitPlanningView;
