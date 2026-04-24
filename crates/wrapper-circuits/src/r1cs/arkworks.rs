use std::collections::{BTreeMap, BTreeSet};

use ark_bn254::{Bn254 as ArkGroth16Engine, Fr as ArkFr};
use ark_ff::PrimeField as ArkPrimeField;
use ark_groth16::{
  Groth16, PreparedVerifyingKey, Proof, ProvingKey, VerifyingKey, prepare_verifying_key,
};
use ark_relations::r1cs::{
  ConstraintSynthesizer, ConstraintSystemRef, LinearCombination as ArkLinearCombination,
  SynthesisError, Variable,
};
use ark_std::rand::{CryptoRng, RngCore};
use ff::{Field, PrimeField};

use super::{LinearCombination, R1csBuildError, R1csCircuit, VariableId};
use crate::NativeField;

/// Deterministic assignment container for canonical R1CS proving inputs.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct R1csAssignment<F> {
  /// Public inputs keyed by canonical variable id.
  pub public_inputs: BTreeMap<VariableId, F>,
  /// Private witnesses keyed by canonical variable id.
  pub witnesses: BTreeMap<VariableId, F>,
}

/// Arkworks-compatible wrapper over the canonical R1CS circuit.
#[derive(Clone, Debug)]
pub struct ArkworksR1csCircuit<F> {
  /// Canonical R1CS circuit.
  pub circuit: R1csCircuit,
  /// Optional concrete assignment for proving mode.
  pub assignment: Option<R1csAssignment<F>>,
}

/// Arkworks proving key for the current BN254-backed R1CS adapter.
pub type ArkworksProvingKey = ProvingKey<ArkGroth16Engine>;
/// Arkworks verifying key for the current BN254-backed R1CS adapter.
pub type ArkworksVerifyingKey = VerifyingKey<ArkGroth16Engine>;
/// Arkworks prepared verifying key for the current BN254-backed R1CS adapter.
pub type ArkworksPreparedVerifyingKey = PreparedVerifyingKey<ArkGroth16Engine>;
/// Arkworks Groth16 proof for the current BN254-backed R1CS adapter.
pub type ArkworksProof = Proof<ArkGroth16Engine>;

impl<F> ArkworksR1csCircuit<F> {
  /// Creates an adapter wrapper without concrete witness values.
  #[must_use]
  pub fn without_assignment(circuit: R1csCircuit) -> Self {
    Self { circuit, assignment: None }
  }
}

impl ArkworksR1csCircuit<ArkFr> {
  /// Creates an adapter wrapper from canonical native-field assignments.
  ///
  /// # Errors
  ///
  /// Returns an error if the assignment is incomplete or keyed incorrectly.
  pub fn from_native_assignment(
    circuit: R1csCircuit,
    assignment: R1csAssignment<NativeField>,
  ) -> Result<Self, R1csBuildError> {
    validate_assignment(&circuit, &assignment)?;
    Ok(Self { circuit, assignment: Some(convert_assignment_to_ark(&assignment)) })
  }
}

impl ConstraintSynthesizer<ArkFr> for ArkworksR1csCircuit<ArkFr> {
  fn generate_constraints(self, cs: ConstraintSystemRef<ArkFr>) -> ark_relations::r1cs::Result<()> {
    let assignment = self.assignment;
    let mut variable_map = BTreeMap::new();
    let public_variable_set =
      self.circuit.public_variables().iter().copied().collect::<BTreeSet<_>>();

    for public_variable in self.circuit.public_variables() {
      let value = assignment
        .as_ref()
        .and_then(|values| values.public_inputs.get(public_variable).copied())
        .unwrap_or(ArkFr::from(0_u64));
      let ark_variable = cs.new_input_variable(|| Ok(value))?;
      variable_map.insert(*public_variable, ark_variable);
    }

    for variable_index in 0..self.circuit.variable_count() {
      let variable_id = VariableId(variable_index as u32);
      if public_variable_set.contains(&variable_id) {
        continue;
      }

      let value = assignment
        .as_ref()
        .and_then(|values| values.witnesses.get(&variable_id).copied())
        .unwrap_or(ArkFr::from(0_u64));
      let ark_variable = cs.new_witness_variable(|| Ok(value))?;
      variable_map.insert(variable_id, ark_variable);
    }

    for constraint in &self.circuit.constraints {
      let a = to_ark_lc(&constraint.a, &variable_map)
        .map_err(|error| arkworks_synthesis_error(&error))?;
      let b = to_ark_lc(&constraint.b, &variable_map)
        .map_err(|error| arkworks_synthesis_error(&error))?;
      let c = to_ark_lc(&constraint.c, &variable_map)
        .map_err(|error| arkworks_synthesis_error(&error))?;
      cs.enforce_constraint(a, b, c)?;
    }

    Ok(())
  }
}

/// Converts a canonical linear combination into Arkworks form.
///
/// # Errors
///
/// Returns an error if a variable is missing from the allocated Arkworks map.
pub fn to_ark_lc(
  linear_combination: &LinearCombination,
  variable_map: &BTreeMap<VariableId, Variable>,
) -> Result<ArkLinearCombination<ArkFr>, R1csBuildError> {
  let normalized =
    LinearCombination::new(linear_combination.terms.clone(), linear_combination.constant);
  let mut ark_lc = ArkLinearCombination::default();

  if normalized.constant != NativeField::ZERO {
    ark_lc.0.push((native_to_ark_fr(normalized.constant), Variable::One));
  }

  for term in normalized.terms {
    let variable = variable_map
      .get(&term.var)
      .copied()
      .ok_or(R1csBuildError::UndeclaredVariable { var: term.var })?;
    ark_lc.0.push((native_to_ark_fr(term.coeff), variable));
  }

  Ok(ark_lc)
}

/// Returns public inputs in canonical verifier order.
///
/// # Errors
///
/// Returns an error if the assignment does not exactly cover the circuit's
/// required public variables.
pub fn ordered_public_inputs(
  circuit: &R1csCircuit,
  assignment: &R1csAssignment<NativeField>,
) -> Result<Vec<NativeField>, R1csBuildError> {
  validate_assignment(circuit, assignment)?;
  circuit
    .public_variables()
    .iter()
    .map(|variable| {
      assignment
        .public_inputs
        .get(variable)
        .copied()
        .ok_or(R1csBuildError::MissingPublicAssignment(*variable))
    })
    .collect()
}

/// Generates random Groth16 parameters through Arkworks for the canonical R1CS.
///
/// # Errors
///
/// Returns an error if Arkworks rejects synthesis or setup.
pub fn arkworks_generate_random_parameters<R: RngCore + CryptoRng>(
  circuit: &R1csCircuit,
  rng: &mut R,
) -> Result<ArkworksProvingKey, R1csBuildError> {
  Groth16::<ArkGroth16Engine>::generate_random_parameters_with_reduction(
    ArkworksR1csCircuit::without_assignment(circuit.clone()),
    rng,
  )
  .map_err(|error| R1csBuildError::ArkworksProofError(error.to_string()))
}

/// Creates a random Groth16 proof through Arkworks for the canonical R1CS.
///
/// # Errors
///
/// Returns an error if the assignment is incomplete or Arkworks rejects proof generation.
pub fn arkworks_create_random_proof<R: RngCore + CryptoRng>(
  circuit: &R1csCircuit,
  assignment: R1csAssignment<NativeField>,
  params: &ArkworksProvingKey,
  rng: &mut R,
) -> Result<ArkworksProof, R1csBuildError> {
  let ark_circuit = ArkworksR1csCircuit::from_native_assignment(circuit.clone(), assignment)?;
  Groth16::<ArkGroth16Engine>::create_random_proof_with_reduction(ark_circuit, params, rng)
    .map_err(|error| R1csBuildError::ArkworksProofError(error.to_string()))
}

/// Verifies a Groth16 proof through Arkworks for the canonical R1CS public inputs.
///
/// # Errors
///
/// Returns an error if Arkworks rejects verification.
pub fn arkworks_verify_proof(
  vk: &ArkworksVerifyingKey,
  public_inputs: &[NativeField],
  proof: &ArkworksProof,
) -> Result<bool, R1csBuildError> {
  let ark_public_inputs = public_inputs.iter().copied().map(native_to_ark_fr).collect::<Vec<_>>();
  let prepared_vk = prepare_verifying_key(vk);
  Groth16::<ArkGroth16Engine>::verify_proof(&prepared_vk, proof, &ark_public_inputs)
    .map_err(|error| R1csBuildError::ArkworksProofError(error.to_string()))
}

fn validate_assignment<F>(
  circuit: &R1csCircuit,
  assignment: &R1csAssignment<F>,
) -> Result<(), R1csBuildError> {
  let public_variable_set = circuit.public_variables().iter().copied().collect::<BTreeSet<_>>();
  let witness_variable_set = circuit.witnesses.iter().copied().collect::<BTreeSet<_>>();

  for variable in assignment.public_inputs.keys().copied() {
    if !public_variable_set.contains(&variable) {
      return Err(R1csBuildError::UnexpectedPublicAssignment(variable));
    }
  }

  for variable in circuit.public_variables().iter().copied() {
    if assignment.witnesses.contains_key(&variable) {
      return Err(R1csBuildError::PublicVariablePassedAsWitness(variable));
    }
    if !assignment.public_inputs.contains_key(&variable) {
      return Err(R1csBuildError::MissingPublicAssignment(variable));
    }
  }

  for variable in assignment.witnesses.keys().copied() {
    if public_variable_set.contains(&variable) {
      return Err(R1csBuildError::PublicVariablePassedAsWitness(variable));
    }
    if !witness_variable_set.contains(&variable) {
      return Err(R1csBuildError::UnexpectedWitnessAssignment(variable));
    }
  }

  for variable in circuit.witnesses.iter().copied() {
    if !assignment.witnesses.contains_key(&variable) {
      return Err(R1csBuildError::MissingWitnessAssignment(variable));
    }
  }

  Ok(())
}

fn convert_assignment_to_ark(assignment: &R1csAssignment<NativeField>) -> R1csAssignment<ArkFr> {
  R1csAssignment {
    public_inputs: assignment
      .public_inputs
      .iter()
      .map(|(variable, value)| (*variable, native_to_ark_fr(*value)))
      .collect(),
    witnesses: assignment
      .witnesses
      .iter()
      .map(|(variable, value)| (*variable, native_to_ark_fr(*value)))
      .collect(),
  }
}

fn native_to_ark_fr(value: NativeField) -> ArkFr {
  ArkFr::from_le_bytes_mod_order(value.to_repr().as_ref())
}

fn arkworks_synthesis_error(error: &R1csBuildError) -> SynthesisError {
  match error {
    R1csBuildError::UndeclaredVariable { .. } => SynthesisError::AssignmentMissing,
    _ => SynthesisError::Unsatisfiable,
  }
}
