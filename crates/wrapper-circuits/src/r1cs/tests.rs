use std::collections::BTreeMap;

use ark_bn254::Fr as ArkFr;
use ark_relations::r1cs::{ConstraintSynthesizer, ConstraintSystem};
use ark_std::rand::{SeedableRng, rngs::StdRng};
use ff::Field;

use super::{
  ArkworksR1csCircuit, CanonicalClassId, CanonicalR1csBuilder, EqualityEdge,
  Halo2CellAssignmentMap, Halo2CellLinearCombination, Halo2CellRef, Halo2CellTerm,
  Halo2Phase1R1csLowering, Halo2PublicInputRef, Halo2R1csMetadata, LinearCombination, LinearTerm,
  R1CS_IDENTITY_DOMAIN_SEPARATOR, R1csAssignment, R1csBuildError, R1csCircuit, R1csConstraint,
  VariableId, ZkInterfaceConstraint, arkworks_create_random_proof,
  arkworks_generate_random_parameters, arkworks_verify_proof, export_witness,
  ordered_public_inputs,
};
use crate::NativeField;

fn instance(column: usize, row: usize) -> Halo2CellRef {
  Halo2CellRef::Instance { column, row }
}

fn advice(column: usize, row: usize) -> Halo2CellRef {
  Halo2CellRef::Advice { column, row }
}

#[test]
fn linear_combination_normalization_sorts_combines_and_drops_zero_terms() {
  let linear = LinearCombination::new(
    vec![
      LinearTerm::new(VariableId(2), NativeField::from(9_u64)),
      LinearTerm::new(VariableId(1), NativeField::from(4_u64)),
      LinearTerm::new(VariableId(2), -NativeField::from(9_u64)),
      LinearTerm::new(VariableId(1), NativeField::from(3_u64)),
      LinearTerm::new(VariableId(3), NativeField::ZERO),
    ],
    NativeField::from(5_u64),
  );

  assert_eq!(
    linear,
    LinearCombination {
      terms: vec![LinearTerm::new(VariableId(1), NativeField::from(7_u64))],
      constant: NativeField::from(5_u64),
    }
  );
}

#[test]
fn deterministic_representatives_ignore_edge_insertion_order() {
  let cells = vec![advice(0, 0), advice(1, 0), instance(0, 0)];
  let first = Halo2CellAssignmentMap::from_cells_and_edges(
    cells.clone(),
    vec![
      EqualityEdge::new(advice(0, 0), advice(1, 0)),
      EqualityEdge::new(instance(0, 0), advice(1, 0)),
    ],
  )
  .expect("canonical assignment should build");
  let second = Halo2CellAssignmentMap::from_cells_and_edges(
    cells.into_iter().rev().collect::<Vec<_>>(),
    vec![
      EqualityEdge::new(instance(0, 0), advice(1, 0)),
      EqualityEdge::new(advice(1, 0), advice(0, 0)),
    ],
  )
  .expect("canonical assignment should build");

  assert_eq!(first, second);
  assert_eq!(
    first.class_for(advice(0, 0)).expect("class should exist"),
    CanonicalClassId::new(instance(0, 0))
  );
  assert_eq!(
    first.class_for(advice(1, 0)).expect("class should exist"),
    CanonicalClassId::new(instance(0, 0))
  );
}

#[test]
fn equality_transitivity_maps_all_cells_to_one_variable() {
  let map = Halo2CellAssignmentMap::from_cells_and_edges(
    vec![advice(0, 0), advice(1, 0), advice(2, 0)],
    vec![
      EqualityEdge::new(advice(0, 0), advice(1, 0)),
      EqualityEdge::new(advice(1, 0), advice(2, 0)),
    ],
  )
  .expect("canonical assignment should build");

  let variable = map.variable_for(advice(0, 0)).expect("variable should exist");
  assert_eq!(variable, map.variable_for(advice(1, 0)).expect("variable should exist"));
  assert_eq!(variable, map.variable_for(advice(2, 0)).expect("variable should exist"));
}

#[test]
fn public_input_stability_ignores_input_ordering() {
  let cells = vec![instance(1, 0), advice(2, 0), instance(0, 0), advice(3, 0)];
  let first = Halo2CellAssignmentMap::from_cells_and_edges(
    cells.clone(),
    vec![
      EqualityEdge::new(instance(1, 0), advice(2, 0)),
      EqualityEdge::new(instance(0, 0), advice(3, 0)),
    ],
  )
  .expect("canonical assignment should build");
  let second = Halo2CellAssignmentMap::from_cells_and_edges(
    cells.into_iter().rev().collect::<Vec<_>>(),
    vec![
      EqualityEdge::new(instance(0, 0), advice(3, 0)),
      EqualityEdge::new(advice(2, 0), instance(1, 0)),
    ],
  )
  .expect("canonical assignment should build");

  assert_eq!(first.public_variables, second.public_variables);
  assert_eq!(first.public_variables, vec![VariableId(0), VariableId(1)]);
}

#[test]
fn canonical_variable_ordering_is_instance_then_advice() {
  let map = Halo2CellAssignmentMap::from_cells_and_edges(
    vec![advice(1, 0), advice(0, 0), instance(2, 0), instance(0, 1), advice(0, 1)],
    Vec::<EqualityEdge>::new(),
  )
  .expect("canonical assignment should build");

  assert_eq!(map.variable_for(instance(0, 1)).expect("variable should exist"), VariableId(0));
  assert_eq!(map.variable_for(instance(2, 0)).expect("variable should exist"), VariableId(1));
  assert_eq!(map.variable_for(advice(0, 0)).expect("variable should exist"), VariableId(2));
  assert_eq!(map.variable_for(advice(0, 1)).expect("variable should exist"), VariableId(3));
  assert_eq!(map.variable_for(advice(1, 0)).expect("variable should exist"), VariableId(4));
  assert_eq!(map.public_variables, vec![VariableId(0), VariableId(1)]);
  assert_eq!(
    map.witness_variables().expect("witness partition should exist"),
    vec![VariableId(2), VariableId(3), VariableId(4)]
  );
}

fn build_sample_phase2_circuit(
  cells: Vec<Halo2CellRef>,
  edges: Vec<EqualityEdge>,
) -> super::R1csCircuit {
  let mut lowering =
    Halo2Phase1R1csLowering::from_cells_and_edges(cells, edges).expect("lowering should build");
  let public = instance(0, 0);
  let public_alias = advice(3, 0);
  let witness = advice(0, 0);
  let output = advice(1, 0);

  lowering
    .add_multiplication_gate(public_alias, witness, output)
    .expect("cells should lower into canonical variables");
  lowering
    .add_linear_constant_gate(
      &Halo2CellLinearCombination::new(
        vec![
          Halo2CellTerm::new(output, NativeField::from(2_u64)),
          Halo2CellTerm::new(witness, NativeField::ONE),
        ],
        NativeField::ZERO,
      ),
      NativeField::from(11_u64),
    )
    .expect("cells should lower into canonical variables");
  lowering
    .add_scaled_multiplication_gate(public, NativeField::from(5_u64), witness)
    .expect("cells should lower into canonical variables");

  lowering.build()
}

#[test]
fn repeated_lowering_runs_produce_identical_r1cs_output() {
  let cells = vec![public_cell(), advice(1, 0), advice(0, 0), advice(3, 0)];
  let first = build_sample_phase2_circuit(
    cells.clone(),
    vec![EqualityEdge::new(public_cell(), advice(3, 0))],
  );
  let second = build_sample_phase2_circuit(
    cells.into_iter().rev().collect::<Vec<_>>(),
    vec![EqualityEdge::new(advice(3, 0), public_cell())],
  );

  assert_eq!(first, second);
  assert_eq!(first.public_inputs, vec![VariableId(0)]);
  assert_eq!(first.witnesses, vec![VariableId(1), VariableId(2)]);
  assert_eq!(first.constraints[0].a, LinearCombination::from_var(VariableId(0)));
}

#[test]
fn builder_constraint_order_remains_deterministic() {
  let mut builder = CanonicalR1csBuilder::new();
  let x = builder.add_witness();
  let y = builder.add_witness();
  let z = builder.add_witness();
  let out = builder.add_witness();

  builder
    .add_multiplication_constraint(x, y, z)
    .expect("allocated variables should lower into R1CS");
  builder
    .add_scaled_multiplication_constraint(z, NativeField::from(7_u64), out)
    .expect("allocated variables should lower into R1CS");

  let circuit = builder.build();
  assert_eq!(circuit.constraints.len(), 2);
  assert_eq!(circuit.constraints[0].a, LinearCombination::from_var(x));
  assert_eq!(circuit.constraints[1].b, LinearCombination::constant(NativeField::from(7_u64)));
}

fn public_cell() -> Halo2CellRef {
  instance(0, 0)
}

fn metadata_with_public_inputs(
  cells: Vec<Halo2CellRef>,
  equality_edges: Vec<EqualityEdge>,
  public_inputs: Vec<Halo2PublicInputRef>,
) -> Halo2R1csMetadata {
  Halo2R1csMetadata { cells, equality_edges, public_inputs }
}

#[test]
fn metadata_rejects_non_instance_public_input() {
  let metadata = metadata_with_public_inputs(
    vec![advice(0, 0)],
    Vec::new(),
    vec![Halo2PublicInputRef { cell: advice(0, 0), public_index: 0 }],
  );

  assert_eq!(
    Halo2CellAssignmentMap::from_metadata(&metadata),
    Err(R1csBuildError::InvalidPublicInputCell(advice(0, 0)))
  );
}

#[test]
fn metadata_rejects_non_contiguous_public_indices() {
  let metadata = metadata_with_public_inputs(
    vec![instance(0, 0), instance(1, 0)],
    Vec::new(),
    vec![
      Halo2PublicInputRef { cell: instance(0, 0), public_index: 0 },
      Halo2PublicInputRef { cell: instance(1, 0), public_index: 2 },
    ],
  );

  assert_eq!(
    Halo2CellAssignmentMap::from_metadata(&metadata),
    Err(R1csBuildError::NonContiguousPublicInputIndices)
  );
}

#[test]
fn public_input_order_follows_public_index() {
  let metadata = metadata_with_public_inputs(
    vec![instance(5, 0), instance(1, 0)],
    Vec::new(),
    vec![
      Halo2PublicInputRef { cell: instance(5, 0), public_index: 1 },
      Halo2PublicInputRef { cell: instance(1, 0), public_index: 0 },
    ],
  );

  let map = Halo2CellAssignmentMap::from_metadata(&metadata).expect("metadata should build");
  assert_eq!(map.variable_for(instance(1, 0)).expect("variable should exist"), VariableId(0));
  assert_eq!(map.variable_for(instance(5, 0)).expect("variable should exist"), VariableId(1));
  assert_eq!(map.public_variables(), &[VariableId(0), VariableId(1)]);
}

#[test]
fn public_index_order_does_not_change_variable_identity() {
  let first = Halo2CellAssignmentMap::from_metadata(&metadata_with_public_inputs(
    vec![instance(5, 0), instance(1, 0)],
    Vec::new(),
    vec![
      Halo2PublicInputRef { cell: instance(5, 0), public_index: 1 },
      Halo2PublicInputRef { cell: instance(1, 0), public_index: 0 },
    ],
  ))
  .expect("metadata should build");
  let second = Halo2CellAssignmentMap::from_metadata(&metadata_with_public_inputs(
    vec![instance(5, 0), instance(1, 0)],
    Vec::new(),
    vec![
      Halo2PublicInputRef { cell: instance(5, 0), public_index: 0 },
      Halo2PublicInputRef { cell: instance(1, 0), public_index: 1 },
    ],
  ))
  .expect("metadata should build");

  assert_eq!(first.variable_for(instance(1, 0)).expect("variable should exist"), VariableId(0));
  assert_eq!(first.variable_for(instance(5, 0)).expect("variable should exist"), VariableId(1));
  assert_eq!(second.variable_for(instance(1, 0)).expect("variable should exist"), VariableId(0));
  assert_eq!(second.variable_for(instance(5, 0)).expect("variable should exist"), VariableId(1));
  assert_ne!(first.public_variables(), second.public_variables());
}

#[test]
fn public_input_through_equality_uses_unified_variable() {
  let metadata = metadata_with_public_inputs(
    vec![instance(0, 0), advice(2, 0), advice(3, 0)],
    vec![EqualityEdge::new(instance(0, 0), advice(2, 0))],
    vec![Halo2PublicInputRef { cell: instance(0, 0), public_index: 0 }],
  );

  let map = Halo2CellAssignmentMap::from_metadata(&metadata).expect("metadata should build");
  let public_variable = map.public_variables()[0];
  assert_eq!(public_variable, map.variable_for(instance(0, 0)).expect("variable should exist"));
  assert_eq!(public_variable, map.variable_for(advice(2, 0)).expect("variable should exist"));
}

#[test]
fn duplicate_public_variable_is_rejected() {
  let metadata = metadata_with_public_inputs(
    vec![instance(0, 0), instance(1, 0)],
    vec![EqualityEdge::new(instance(0, 0), instance(1, 0))],
    vec![
      Halo2PublicInputRef { cell: instance(0, 0), public_index: 0 },
      Halo2PublicInputRef { cell: instance(1, 0), public_index: 1 },
    ],
  );

  assert_eq!(
    Halo2CellAssignmentMap::from_metadata(&metadata),
    Err(R1csBuildError::DuplicatePublicInputVariable(VariableId(0)))
  );
}

#[test]
fn unknown_equality_endpoint_is_rejected() {
  let metadata = metadata_with_public_inputs(
    vec![instance(0, 0)],
    vec![EqualityEdge::new(instance(0, 0), advice(99, 0))],
    vec![Halo2PublicInputRef { cell: instance(0, 0), public_index: 0 }],
  );

  assert_eq!(
    Halo2CellAssignmentMap::from_metadata(&metadata),
    Err(R1csBuildError::UnknownCell(advice(99, 0)))
  );
}

fn build_sample_phase3_circuit(metadata: &Halo2R1csMetadata) -> super::R1csCircuit {
  let mut lowering =
    Halo2Phase1R1csLowering::from_metadata(metadata).expect("metadata should build");
  let witness = advice(0, 0);
  let public = instance(1, 0);
  let output = advice(1, 0);

  lowering
    .add_multiplication_gate(public, witness, output)
    .expect("cells should lower into canonical variables");
  lowering
    .add_linear_constant_gate(
      &Halo2CellLinearCombination::new(
        vec![
          Halo2CellTerm::new(output, NativeField::from(3_u64)),
          Halo2CellTerm::new(witness, NativeField::ONE),
        ],
        NativeField::ZERO,
      ),
      NativeField::from(17_u64),
    )
    .expect("cells should lower into canonical variables");

  lowering.build()
}

#[test]
fn metadata_idempotence_produces_identical_assignment_maps_and_r1cs() {
  let metadata = metadata_with_public_inputs(
    vec![instance(1, 0), advice(0, 0), advice(1, 0), advice(9, 0), instance(0, 0)],
    vec![EqualityEdge::new(instance(0, 0), advice(9, 0))],
    vec![
      Halo2PublicInputRef { cell: instance(1, 0), public_index: 0 },
      Halo2PublicInputRef { cell: instance(0, 0), public_index: 1 },
    ],
  );

  let first_map = Halo2CellAssignmentMap::from_metadata(&metadata).expect("metadata should build");
  let second_map = Halo2CellAssignmentMap::from_metadata(&metadata).expect("metadata should build");
  assert_eq!(first_map, second_map);

  let first_circuit = build_sample_phase3_circuit(&metadata);
  let second_circuit = build_sample_phase3_circuit(&metadata);
  assert_eq!(first_circuit, second_circuit);
  assert_eq!(first_circuit.public_inputs, vec![VariableId(1), VariableId(0)]);
}

#[test]
fn identity_hash_is_reproducible() {
  let metadata = metadata_with_public_inputs(
    vec![instance(1, 0), advice(0, 0), advice(1, 0), advice(9, 0), instance(0, 0)],
    vec![EqualityEdge::new(instance(0, 0), advice(9, 0))],
    vec![
      Halo2PublicInputRef { cell: instance(1, 0), public_index: 0 },
      Halo2PublicInputRef { cell: instance(0, 0), public_index: 1 },
    ],
  );

  let first = build_sample_phase3_circuit(&metadata);
  let second = build_sample_phase3_circuit(&metadata);
  assert_eq!(first.canonical_bytes(), second.canonical_bytes());
  assert_eq!(first.identity_hash(), second.identity_hash());
}

#[test]
fn normalized_linear_combinations_hash_equally() {
  let circuit_a = R1csCircuit {
    public_inputs: vec![],
    witnesses: vec![VariableId(0)],
    constraints: vec![R1csConstraint::new(
      LinearCombination::new(
        vec![
          LinearTerm::new(VariableId(0), NativeField::from(2_u64)),
          LinearTerm::new(VariableId(0), NativeField::from(3_u64)),
        ],
        NativeField::from(1_u64),
      ),
      LinearCombination::one(),
      LinearCombination::zero(),
    )],
  };
  let circuit_b = R1csCircuit {
    public_inputs: vec![],
    witnesses: vec![VariableId(0)],
    constraints: vec![R1csConstraint::new(
      LinearCombination::new(
        vec![LinearTerm::new(VariableId(0), NativeField::from(5_u64))],
        NativeField::from(1_u64),
      ),
      LinearCombination::one(),
      LinearCombination::zero(),
    )],
  };

  assert_eq!(circuit_a.canonical_bytes(), circuit_b.canonical_bytes());
  assert_eq!(circuit_a.identity_hash(), circuit_b.identity_hash());
}

#[test]
fn public_input_order_affects_identity() {
  let first = Halo2CellAssignmentMap::from_metadata(&metadata_with_public_inputs(
    vec![instance(5, 0), instance(1, 0)],
    Vec::new(),
    vec![
      Halo2PublicInputRef { cell: instance(5, 0), public_index: 1 },
      Halo2PublicInputRef { cell: instance(1, 0), public_index: 0 },
    ],
  ))
  .expect("metadata should build");
  let second = Halo2CellAssignmentMap::from_metadata(&metadata_with_public_inputs(
    vec![instance(5, 0), instance(1, 0)],
    Vec::new(),
    vec![
      Halo2PublicInputRef { cell: instance(5, 0), public_index: 0 },
      Halo2PublicInputRef { cell: instance(1, 0), public_index: 1 },
    ],
  ))
  .expect("metadata should build");

  let first_circuit = R1csCircuit {
    public_inputs: first.public_variables().to_vec(),
    witnesses: first.witness_variables().expect("witnesses should exist"),
    constraints: vec![],
  };
  let second_circuit = R1csCircuit {
    public_inputs: second.public_variables().to_vec(),
    witnesses: second.witness_variables().expect("witnesses should exist"),
    constraints: vec![],
  };

  assert_ne!(first_circuit.canonical_bytes(), second_circuit.canonical_bytes());
  assert_ne!(first_circuit.identity_hash(), second_circuit.identity_hash());
}

#[test]
fn constraint_order_affects_identity() {
  let circuit_a = R1csCircuit {
    public_inputs: vec![],
    witnesses: vec![VariableId(0), VariableId(1), VariableId(2)],
    constraints: vec![
      R1csConstraint::new(
        LinearCombination::from_var(VariableId(0)),
        LinearCombination::from_var(VariableId(1)),
        LinearCombination::from_var(VariableId(2)),
      ),
      R1csConstraint::new(
        LinearCombination::from_var(VariableId(2)),
        LinearCombination::one(),
        LinearCombination::constant(NativeField::from(5_u64)),
      ),
    ],
  };
  let circuit_b = R1csCircuit {
    public_inputs: vec![],
    witnesses: vec![VariableId(0), VariableId(1), VariableId(2)],
    constraints: vec![
      R1csConstraint::new(
        LinearCombination::from_var(VariableId(2)),
        LinearCombination::one(),
        LinearCombination::constant(NativeField::from(5_u64)),
      ),
      R1csConstraint::new(
        LinearCombination::from_var(VariableId(0)),
        LinearCombination::from_var(VariableId(1)),
        LinearCombination::from_var(VariableId(2)),
      ),
    ],
  };

  assert_ne!(circuit_a.canonical_bytes(), circuit_b.canonical_bytes());
  assert_ne!(circuit_a.identity_hash(), circuit_b.identity_hash());
}

#[test]
fn equality_edge_insertion_order_does_not_affect_identity() {
  let metadata_a = metadata_with_public_inputs(
    vec![instance(1, 0), advice(0, 0), advice(1, 0), advice(2, 0), advice(3, 0)],
    vec![
      EqualityEdge::new(instance(1, 0), advice(1, 0)),
      EqualityEdge::new(advice(1, 0), advice(2, 0)),
    ],
    vec![Halo2PublicInputRef { cell: instance(1, 0), public_index: 0 }],
  );
  let metadata_b = metadata_with_public_inputs(
    vec![advice(3, 0), advice(2, 0), advice(1, 0), advice(0, 0), instance(1, 0)],
    vec![
      EqualityEdge::new(advice(1, 0), advice(2, 0)),
      EqualityEdge::new(advice(1, 0), instance(1, 0)),
    ],
    vec![Halo2PublicInputRef { cell: instance(1, 0), public_index: 0 }],
  );

  let first = build_sample_phase3_circuit(&metadata_a);
  let second = build_sample_phase3_circuit(&metadata_b);
  assert_eq!(first.canonical_bytes(), second.canonical_bytes());
  assert_eq!(first.identity_hash(), second.identity_hash());
}

#[test]
fn public_index_order_is_reflected_in_identity() {
  let metadata_a = metadata_with_public_inputs(
    vec![instance(5, 0), instance(1, 0)],
    Vec::new(),
    vec![
      Halo2PublicInputRef { cell: instance(5, 0), public_index: 1 },
      Halo2PublicInputRef { cell: instance(1, 0), public_index: 0 },
    ],
  );
  let metadata_b = metadata_with_public_inputs(
    vec![instance(5, 0), instance(1, 0)],
    Vec::new(),
    vec![
      Halo2PublicInputRef { cell: instance(5, 0), public_index: 0 },
      Halo2PublicInputRef { cell: instance(1, 0), public_index: 1 },
    ],
  );

  let first_map =
    Halo2CellAssignmentMap::from_metadata(&metadata_a).expect("metadata should build");
  let second_map =
    Halo2CellAssignmentMap::from_metadata(&metadata_b).expect("metadata should build");
  assert_eq!(first_map.variable_for(instance(1, 0)).expect("variable should exist"), VariableId(0));
  assert_eq!(first_map.variable_for(instance(5, 0)).expect("variable should exist"), VariableId(1));
  assert_eq!(
    second_map.variable_for(instance(1, 0)).expect("variable should exist"),
    VariableId(0)
  );
  assert_eq!(
    second_map.variable_for(instance(5, 0)).expect("variable should exist"),
    VariableId(1)
  );

  let first_circuit = R1csCircuit {
    public_inputs: first_map.public_variables().to_vec(),
    witnesses: first_map.witness_variables().expect("witnesses should exist"),
    constraints: vec![],
  };
  let second_circuit = R1csCircuit {
    public_inputs: second_map.public_variables().to_vec(),
    witnesses: second_map.witness_variables().expect("witnesses should exist"),
    constraints: vec![],
  };

  assert_ne!(first_circuit.identity_hash(), second_circuit.identity_hash());
}

#[test]
fn canonical_bytes_begin_with_domain_separator() {
  let circuit = R1csCircuit {
    public_inputs: vec![VariableId(0)],
    witnesses: vec![VariableId(1)],
    constraints: vec![],
  };

  assert!(circuit.canonical_bytes().starts_with(R1CS_IDENTITY_DOMAIN_SEPARATOR));
}

fn multiplication_public_output_circuit() -> R1csCircuit {
  R1csCircuit {
    public_inputs: vec![VariableId(2)],
    witnesses: vec![VariableId(0), VariableId(1)],
    constraints: vec![R1csConstraint::new(
      LinearCombination::from_var(VariableId(0)),
      LinearCombination::from_var(VariableId(1)),
      LinearCombination::from_var(VariableId(2)),
    )],
  }
}

fn valid_multiplication_assignment() -> R1csAssignment<NativeField> {
  R1csAssignment {
    public_inputs: BTreeMap::from([(VariableId(2), NativeField::from(6_u64))]),
    witnesses: BTreeMap::from([
      (VariableId(0), NativeField::from(2_u64)),
      (VariableId(1), NativeField::from(3_u64)),
    ]),
  }
}

#[test]
fn arkworks_constraint_satisfaction_accepts_valid_assignment() {
  let circuit = multiplication_public_output_circuit();
  let arkworks =
    ArkworksR1csCircuit::from_native_assignment(circuit, valid_multiplication_assignment())
      .expect("valid assignment should adapt to arkworks");
  let cs = ConstraintSystem::<ArkFr>::new_ref();

  arkworks.generate_constraints(cs.clone()).expect("constraint synthesis should succeed");
  assert!(cs.is_satisfied().expect("constraint system should report satisfaction state"));
}

#[test]
fn arkworks_constraint_satisfaction_rejects_invalid_assignment() {
  let circuit = multiplication_public_output_circuit();
  let arkworks = ArkworksR1csCircuit::from_native_assignment(
    circuit,
    R1csAssignment {
      public_inputs: BTreeMap::from([(VariableId(2), NativeField::from(7_u64))]),
      witnesses: BTreeMap::from([
        (VariableId(0), NativeField::from(2_u64)),
        (VariableId(1), NativeField::from(3_u64)),
      ]),
    },
  )
  .expect("invalid values still form a complete assignment");
  let cs = ConstraintSystem::<ArkFr>::new_ref();

  arkworks.generate_constraints(cs.clone()).expect("constraint synthesis should succeed");
  assert!(!cs.is_satisfied().expect("constraint system should report satisfaction state"));
}

#[test]
fn groth16_roundtrip_verifies_for_small_canonical_circuit() {
  let circuit = multiplication_public_output_circuit();
  let assignment = valid_multiplication_assignment();
  let ordered_public =
    ordered_public_inputs(&circuit, &assignment).expect("public order should resolve");
  let mut rng = StdRng::seed_from_u64(7);

  let params = arkworks_generate_random_parameters(&circuit, &mut rng)
    .expect("parameter generation should succeed");
  let proof = arkworks_create_random_proof(&circuit, assignment, &params, &mut rng)
    .expect("proof generation should succeed");

  assert!(
    arkworks_verify_proof(&params.vk, &ordered_public, &proof).expect("verification should run")
  );
}

#[test]
fn groth16_rejects_wrong_public_input() {
  let circuit = multiplication_public_output_circuit();
  let assignment = valid_multiplication_assignment();
  let mut ordered_public =
    ordered_public_inputs(&circuit, &assignment).expect("public order should resolve");
  let mut rng = StdRng::seed_from_u64(9);

  let params = arkworks_generate_random_parameters(&circuit, &mut rng)
    .expect("parameter generation should succeed");
  let proof = arkworks_create_random_proof(&circuit, assignment, &params, &mut rng)
    .expect("proof generation should succeed");

  ordered_public[0] = NativeField::from(7_u64);
  assert!(
    !arkworks_verify_proof(&params.vk, &ordered_public, &proof).expect("verification should run")
  );
}

#[test]
fn missing_witness_assignment_is_rejected() {
  let circuit = multiplication_public_output_circuit();
  let assignment = R1csAssignment {
    public_inputs: BTreeMap::from([(VariableId(2), NativeField::from(6_u64))]),
    witnesses: BTreeMap::from([(VariableId(0), NativeField::from(2_u64))]),
  };

  assert_eq!(
    ArkworksR1csCircuit::from_native_assignment(circuit, assignment).map(|_| ()),
    Err(R1csBuildError::MissingWitnessAssignment(VariableId(1)))
  );
}

#[test]
fn missing_public_assignment_is_rejected() {
  let circuit = multiplication_public_output_circuit();
  let assignment = R1csAssignment {
    public_inputs: BTreeMap::new(),
    witnesses: BTreeMap::from([
      (VariableId(0), NativeField::from(2_u64)),
      (VariableId(1), NativeField::from(3_u64)),
    ]),
  };

  assert_eq!(
    ArkworksR1csCircuit::from_native_assignment(circuit, assignment).map(|_| ()),
    Err(R1csBuildError::MissingPublicAssignment(VariableId(2)))
  );
}

#[test]
fn arkworks_adapter_does_not_change_canonical_identity() {
  let circuit = multiplication_public_output_circuit();
  let assignment = valid_multiplication_assignment();
  let original_hash = circuit.identity_hash();
  let original_bytes = circuit.canonical_bytes();
  let mut rng = StdRng::seed_from_u64(11);

  let params = arkworks_generate_random_parameters(&circuit, &mut rng)
    .expect("parameter generation should succeed");
  let _proof = arkworks_create_random_proof(&circuit, assignment, &params, &mut rng)
    .expect("proof generation should succeed");

  assert_eq!(circuit.identity_hash(), original_hash);
  assert_eq!(circuit.canonical_bytes(), original_bytes);
}

#[test]
fn zkinterface_export_preserves_identity_hash() {
  let metadata = metadata_with_public_inputs(
    vec![instance(1, 0), advice(0, 0), advice(1, 0), advice(9, 0), instance(0, 0)],
    vec![EqualityEdge::new(instance(0, 0), advice(9, 0))],
    vec![
      Halo2PublicInputRef { cell: instance(1, 0), public_index: 0 },
      Halo2PublicInputRef { cell: instance(0, 0), public_index: 1 },
    ],
  );
  let circuit = build_sample_phase3_circuit(&metadata);
  let export = circuit.to_zkinterface_export();

  assert_eq!(export.identity_hash, circuit.identity_hash());
}

#[test]
fn zkinterface_export_preserves_public_input_order() {
  let metadata = metadata_with_public_inputs(
    vec![instance(5, 0), instance(1, 0)],
    Vec::new(),
    vec![
      Halo2PublicInputRef { cell: instance(5, 0), public_index: 1 },
      Halo2PublicInputRef { cell: instance(1, 0), public_index: 0 },
    ],
  );
  let circuit = R1csCircuit {
    public_inputs: Halo2CellAssignmentMap::from_metadata(&metadata)
      .expect("metadata should build")
      .public_variables()
      .to_vec(),
    witnesses: vec![],
    constraints: vec![],
  };

  let export = circuit.to_zkinterface_export();
  assert_eq!(export.public_variables, circuit.public_variables());
}

#[test]
fn zkinterface_export_preserves_constraint_order() {
  let circuit = R1csCircuit {
    public_inputs: vec![],
    witnesses: vec![VariableId(0), VariableId(1), VariableId(2)],
    constraints: vec![
      R1csConstraint::new(
        LinearCombination::from_var(VariableId(0)),
        LinearCombination::from_var(VariableId(1)),
        LinearCombination::from_var(VariableId(2)),
      ),
      R1csConstraint::new(
        LinearCombination::from_var(VariableId(2)),
        LinearCombination::one(),
        LinearCombination::constant(NativeField::from(9_u64)),
      ),
    ],
  };

  let export = circuit.to_zkinterface_export();
  assert_eq!(
    export.constraints,
    vec![
      ZkInterfaceConstraint {
        a: circuit.to_zkinterface_export().constraints[0].a.clone(),
        b: circuit.to_zkinterface_export().constraints[0].b.clone(),
        c: circuit.to_zkinterface_export().constraints[0].c.clone(),
      },
      ZkInterfaceConstraint {
        a: circuit.to_zkinterface_export().constraints[1].a.clone(),
        b: circuit.to_zkinterface_export().constraints[1].b.clone(),
        c: circuit.to_zkinterface_export().constraints[1].c.clone(),
      },
    ]
  );
}

#[test]
fn zkinterface_export_is_deterministic() {
  let metadata = metadata_with_public_inputs(
    vec![instance(1, 0), advice(0, 0), advice(1, 0), advice(9, 0), instance(0, 0)],
    vec![EqualityEdge::new(instance(0, 0), advice(9, 0))],
    vec![
      Halo2PublicInputRef { cell: instance(1, 0), public_index: 0 },
      Halo2PublicInputRef { cell: instance(0, 0), public_index: 1 },
    ],
  );
  let circuit = build_sample_phase3_circuit(&metadata);

  assert_eq!(circuit.to_zkinterface_export(), circuit.to_zkinterface_export());
}

#[test]
fn zkinterface_witness_export_is_sorted() {
  let assignments = BTreeMap::from([
    (VariableId(2), NativeField::from(22_u64)),
    (VariableId(0), NativeField::from(11_u64)),
    (VariableId(1), NativeField::from(17_u64)),
  ]);

  let witness = export_witness(&assignments);
  assert_eq!(
    witness.assignments.iter().map(|assignment| assignment.variable).collect::<Vec<_>>(),
    vec![VariableId(0), VariableId(1), VariableId(2)]
  );
}

#[test]
fn zkinterface_export_validation_passes_for_matching_circuit() {
  let metadata = metadata_with_public_inputs(
    vec![instance(1, 0), advice(0, 0), advice(1, 0), advice(9, 0), instance(0, 0)],
    vec![EqualityEdge::new(instance(0, 0), advice(9, 0))],
    vec![
      Halo2PublicInputRef { cell: instance(1, 0), public_index: 0 },
      Halo2PublicInputRef { cell: instance(0, 0), public_index: 1 },
    ],
  );
  let circuit = build_sample_phase3_circuit(&metadata);
  let export = circuit.to_zkinterface_export();

  assert_eq!(export.validate_against_circuit(&circuit), Ok(()));
}

#[test]
fn zkinterface_export_validation_fails_for_mismatched_public_variables() {
  let metadata = metadata_with_public_inputs(
    vec![instance(1, 0), advice(0, 0), advice(1, 0), advice(9, 0), instance(0, 0)],
    vec![EqualityEdge::new(instance(0, 0), advice(9, 0))],
    vec![
      Halo2PublicInputRef { cell: instance(1, 0), public_index: 0 },
      Halo2PublicInputRef { cell: instance(0, 0), public_index: 1 },
    ],
  );
  let circuit = build_sample_phase3_circuit(&metadata);
  let mut export = circuit.to_zkinterface_export();
  export.public_variables.reverse();

  assert_eq!(
    export.validate_against_circuit(&circuit),
    Err(R1csBuildError::ZkInterfaceExportMismatch { context: "public variables" })
  );
}
