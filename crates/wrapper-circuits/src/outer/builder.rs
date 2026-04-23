use super::{OuterWrapperCircuit, OuterWrapperCircuitInput};

/// Builds an outer wrapper circuit using the default project config.
#[must_use]
pub fn build_outer_wrapper_circuit(input: OuterWrapperCircuitInput) -> OuterWrapperCircuit {
  OuterWrapperCircuit::from_input(input)
}
