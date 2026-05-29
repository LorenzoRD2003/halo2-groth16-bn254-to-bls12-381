use ff::{PrimeField, WithSmallOrderMulGroup};
use group::ff::Field;
use std::{borrow::Cow, collections::BTreeSet, time::Instant};

use super::{ConstraintSystem, Expression, direct_logging};
use crate::{
    plonk::{lookup, permutation, trash, Any},
    poly::{EvaluationDomain, LagrangeCoeff, Polynomial, PolynomialRepresentation, Rotation},
    utils::arithmetic::parallelize,
};

const H_POLY_ROW_CHUNK_ENV: &str = "WRAPPER_H_POLY_ROW_CHUNK_SIZE";
const DEFAULT_H_POLY_ROW_CHUNK_SIZE: usize = 1_usize << 15;

fn h_poly_row_chunk_size() -> usize {
    std::env::var(H_POLY_ROW_CHUNK_ENV)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_H_POLY_ROW_CHUNK_SIZE)
}

/// Return the index in the polynomial of size `isize` after rotation `rot`.
pub(crate) fn get_rotation_idx(idx: usize, rot: i32, rot_scale: i32, isize: i32) -> usize {
    (((idx as i32) + (rot * rot_scale)).rem_euclid(isize)) as usize
}

/// Value used in a calculation
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd)]
pub enum ValueSource {
    /// This is a constant value
    Constant(usize),
    /// This is an intermediate value
    Intermediate(usize),
    /// This is a fixed column
    Fixed(usize, usize),
    /// This is an advice (witness) column
    Advice(usize, usize),
    /// This is an instance (external) column
    Instance(usize, usize),
    /// This is a challenge
    Challenge(usize),
    /// beta
    Beta(),
    /// gamma
    Gamma(),
    /// theta
    Theta(),
    /// trash challenge
    TrashChallenge(),
    /// y
    Y(),
    /// Previous value
    PreviousValue(),
}

impl Default for ValueSource {
    fn default() -> Self {
        ValueSource::Constant(0)
    }
}

impl ValueSource {
    /// Get the value for this source
    #[allow(clippy::too_many_arguments)]
    pub fn get<F: Field, B: PolynomialRepresentation>(
        &self,
        rotations: &[usize],
        constants: &[F],
        intermediates: &[F],
        fixed_values: &[Option<Polynomial<F, B>>],
        advice_values: &[Option<Polynomial<F, B>>],
        instance_values: &[Option<Polynomial<F, B>>],
        challenges: &[F],
        beta: &F,
        gamma: &F,
        theta: &F,
        trash_challenge: &F,
        y: &F,
        previous_value: &F,
    ) -> F {
        match self {
            ValueSource::Constant(idx) => constants[*idx],
            ValueSource::Intermediate(idx) => intermediates[*idx],
            ValueSource::Fixed(column_index, rotation) => fixed_values[*column_index]
                .as_ref()
                .expect("fixed column required by evaluator should be materialized")
                [rotations[*rotation]],
            ValueSource::Advice(column_index, rotation) => advice_values[*column_index]
                .as_ref()
                .expect("advice column required by evaluator should be materialized")
                [rotations[*rotation]],
            ValueSource::Instance(column_index, rotation) => instance_values[*column_index]
                .as_ref()
                .expect("instance column required by evaluator should be materialized")
                [rotations[*rotation]],
            ValueSource::Challenge(index) => challenges[*index],
            ValueSource::Beta() => *beta,
            ValueSource::Gamma() => *gamma,
            ValueSource::Theta() => *theta,
            ValueSource::TrashChallenge() => *trash_challenge,
            ValueSource::Y() => *y,
            ValueSource::PreviousValue() => *previous_value,
        }
    }
}

/// Calculation
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Calculation {
    /// This is an addition
    Add(ValueSource, ValueSource),
    /// This is a subtraction
    Sub(ValueSource, ValueSource),
    /// This is a product
    Mul(ValueSource, ValueSource),
    /// This is a square
    Square(ValueSource),
    /// This is a double
    Double(ValueSource),
    /// This is a negation
    Negate(ValueSource),
    /// This is Horner's rule: `val = a; val = val * c + b[]`
    Horner(ValueSource, Vec<ValueSource>, ValueSource),
    /// This is a simple assignment
    Store(ValueSource),
}

impl Calculation {
    /// Get the resulting value of this calculation
    #[allow(clippy::too_many_arguments)]
    pub fn evaluate<F: Field, B: PolynomialRepresentation>(
        &self,
        rotations: &[usize],
        constants: &[F],
        intermediates: &[F],
        fixed_values: &[Option<Polynomial<F, B>>],
        advice_values: &[Option<Polynomial<F, B>>],
        instance_values: &[Option<Polynomial<F, B>>],
        challenges: &[F],
        beta: &F,
        gamma: &F,
        theta: &F,
        trash_challenge: &F,
        y: &F,
        previous_value: &F,
    ) -> F {
        let get_value = |value: &ValueSource| {
            value.get(
                rotations,
                constants,
                intermediates,
                fixed_values,
                advice_values,
                instance_values,
                challenges,
                beta,
                gamma,
                theta,
                trash_challenge,
                y,
                previous_value,
            )
        };
        match self {
            Calculation::Add(a, b) => get_value(a) + get_value(b),
            Calculation::Sub(a, b) => get_value(a) - get_value(b),
            Calculation::Mul(a, b) => get_value(a) * get_value(b),
            Calculation::Square(v) => get_value(v).square(),
            Calculation::Double(v) => get_value(v).double(),
            Calculation::Negate(v) => -get_value(v),
            Calculation::Horner(start_value, parts, factor) => {
                let factor = get_value(factor);
                let mut value = get_value(start_value);
                for part in parts.iter() {
                    value = value * factor + get_value(part);
                }
                value
            }
            Calculation::Store(v) => get_value(v),
        }
    }
}

/// Evaluator
#[derive(Clone, Default, Debug)]
pub struct Evaluator<F: PrimeField> {
    ///  Custom gates evalution
    pub custom_gates: GraphEvaluator<F>,
    ///  Lookups evalution
    pub lookups: Vec<GraphEvaluator<F>>,
    ///  Trashcans evalution
    pub trashcans: Vec<GraphEvaluator<F>>,
}

/// GraphEvaluator
#[derive(Clone, Debug)]
pub struct GraphEvaluator<F: PrimeField> {
    /// Constants
    pub constants: Vec<F>,
    /// Rotations
    pub rotations: Vec<i32>,
    /// Calculations
    pub calculations: Vec<CalculationInfo>,
    /// Number of intermediates
    pub num_intermediates: usize,
}

/// EvaluationData
#[derive(Default, Debug)]
pub struct EvaluationData<F: PrimeField> {
    /// Intermediates
    pub intermediates: Vec<F>,
    /// Rotations
    pub rotations: Vec<usize>,
}

/// CaluclationInfo
#[derive(Clone, Debug)]
pub struct CalculationInfo {
    /// Calculation
    pub calculation: Calculation,
    /// Target
    pub target: usize,
}

impl<F: WithSmallOrderMulGroup<3>> Evaluator<F> {
    /// Creates a new evaluation structure
    pub fn new(cs: &ConstraintSystem<F>) -> Self {
        let mut ev = Evaluator::default();

        // Custom gates
        let mut parts = Vec::new();
        for gate in cs.gates.iter() {
            parts
                .extend(gate.polynomials().iter().map(|poly| ev.custom_gates.add_expression(poly)));
        }
        ev.custom_gates.add_calculation(Calculation::Horner(
            ValueSource::PreviousValue(),
            parts,
            ValueSource::Y(),
        ));

        // Lookups
        for lookup in cs.lookups.iter() {
            let mut graph = GraphEvaluator::default();

            let mut evaluate_lc = |expressions: &Vec<Expression<_>>| {
                let parts = expressions.iter().map(|expr| graph.add_expression(expr)).collect();
                graph.add_calculation(Calculation::Horner(
                    ValueSource::Constant(0),
                    parts,
                    ValueSource::Theta(),
                ))
            };

            // Input coset
            let compressed_input_coset = evaluate_lc(&lookup.input_expressions);
            // table coset
            let compressed_table_coset = evaluate_lc(&lookup.table_expressions);
            // z(\omega X) (a'(X) + \beta) (s'(X) + \gamma)
            let right_gamma = graph.add_calculation(Calculation::Add(
                compressed_table_coset,
                ValueSource::Gamma(),
            ));
            let lc = graph.add_calculation(Calculation::Add(
                compressed_input_coset,
                ValueSource::Beta(),
            ));
            graph.add_calculation(Calculation::Mul(lc, right_gamma));

            ev.lookups.push(graph);
        }

        // Trashcans
        for trash in cs.trashcans.iter() {
            let mut graph = GraphEvaluator::default();

            let parts = trash
                .constraint_expressions()
                .iter()
                .map(|expr| graph.add_expression(expr))
                .collect();

            graph.add_calculation(Calculation::Horner(
                ValueSource::Constant(0),
                parts,
                ValueSource::TrashChallenge(),
            ));

            ev.trashcans.push(graph);
        }

        ev
    }

    /// Evaluate h poly
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn evaluate_h<B: PolynomialRepresentation>(
        &self,
        domain: &EvaluationDomain<F>,
        cs: &ConstraintSystem<F>,
        advice: &[&[Option<Polynomial<F, B>>]],
        instance: &[&[Option<Polynomial<F, B>>]],
        fixed: &[Option<Polynomial<F, B>>],
        challenges: &[F],
        y: F,
        beta: F,
        gamma: F,
        theta: F,
        trash_challenge: F,
        lookups: &[Vec<lookup::prover::Committed<F>>],
        trashcans: &[Vec<trash::prover::Committed<F>>],
        permutations: &[permutation::prover::Committed<F>],
        l0: &Polynomial<F, B>,
        l_last: &Polynomial<F, B>,
        l_active_row: &Polynomial<F, B>,
        permutation_pk_values: &[Polynomial<F, LagrangeCoeff>],
    ) -> Polynomial<F, B> {
        let size = B::len(domain);
        let rot_scale = 1 << (B::k(domain) - domain.k());
        let omega = B::omega(domain);
        let isize = size as i32;
        let one = F::ONE;

        let p = &cs.permutation;

        let values_allocation_started_at = Instant::now();
        direct_logging::log_event(
            "h_poly",
            "values_allocation",
            "start",
            &format!("domain_len={size} rot_scale={rot_scale}"),
        );
        let mut values = B::empty(domain);
        direct_logging::log_elapsed(
            "h_poly",
            "values_allocation",
            values_allocation_started_at,
            &format!("domain_len={size} rot_scale={rot_scale}"),
        );

        // Core expression evaluations
        let num_threads = rayon::current_num_threads();
        for (proof_index, ((((advice, instance), lookups), trashcans), permutation)) in advice
            .iter()
            .zip(instance.iter())
            .zip(lookups.iter())
            .zip(trashcans.iter())
            .zip(permutations.iter())
            .enumerate()
        {
            // Custom gates
            let custom_gates_started_at = Instant::now();
            direct_logging::log_event(
                "h_poly",
                "custom_gates",
                "start",
                &format!("proof_index={proof_index} num_threads={num_threads}"),
            );
            rayon::scope(|scope| {
                let chunk_size = size.div_ceil(num_threads);
                for (thread_idx, values) in values.chunks_mut(chunk_size).enumerate() {
                    let start = thread_idx * chunk_size;
                    direct_logging::log_event(
                        "h_poly",
                        "custom_gates_chunk",
                        "iter",
                        &format!(
                            "proof_index={proof_index} iteration={} chunk_start={} chunk_len={}",
                            thread_idx,
                            start,
                            values.len()
                        ),
                    );
                    scope.spawn(move |_| {
                        let mut eval_data = self.custom_gates.instance();
                        for (i, value) in values.iter_mut().enumerate() {
                            let idx = start + i;
                            *value = self.custom_gates.evaluate::<B>(
                                &mut eval_data,
                                fixed,
                                advice,
                                instance,
                                challenges,
                                &beta,
                                &gamma,
                                &theta,
                                &trash_challenge,
                                &y,
                                value,
                                idx,
                                rot_scale,
                                isize,
                            );
                        }
                    });
                }
            });
            direct_logging::log_elapsed(
                "h_poly",
                "custom_gates",
                custom_gates_started_at,
                &format!("proof_index={proof_index} num_threads={num_threads}"),
            );

            // Permutations
            let sets = &permutation.sets;
            if !sets.is_empty() {
                let permutation_constraints_started_at = Instant::now();
                let row_chunk_size = h_poly_row_chunk_size();
                direct_logging::log_event(
                    "h_poly",
                    "permutation_constraints",
                    "start",
                    &format!(
                        "proof_index={proof_index} set_count={} row_chunk_size={row_chunk_size}",
                        sets.len()
                    ),
                );
                let blinding_factors = cs.blinding_factors();
                let last_rotation = Rotation(-((blinding_factors + 1) as i32));
                let chunk_len = cs.degree() - 2;
                let delta_start = beta * &B::g_coset(domain);

                let mut previous_set_permutation_product_coset: Option<Cow<'_, Polynomial<F, B>>> =
                    None;
                for (set_idx, ((set, columns), permutation_values)) in sets
                    .iter()
                    .zip(p.columns.chunks(chunk_len))
                    .zip(permutation_pk_values.chunks(chunk_len))
                    .enumerate()
                {
                    let total_sets = sets.len();
                    direct_logging::log_event(
                        "h_poly",
                        "permutation_set",
                        "iter",
                        &format!(
                            "proof_index={proof_index} iteration={} set_idx={} set_count={} row_chunk_size={row_chunk_size}",
                            set_idx,
                            set_idx + 1,
                            total_sets
                        ),
                    );
                    let permutation_set_cosets_started_at = Instant::now();
                    direct_logging::log_event(
                        "h_poly",
                        "permutation_set_cosets",
                        "start",
                        &format!(
                            "proof_index={proof_index} set_idx={} set_count={}",
                            set_idx + 1,
                            total_sets
                        ),
                    );
                    let current_set_permutation_product_coset: Cow<'_, Polynomial<F, B>> =
                        Cow::Owned(B::coeff_to_self(domain, set.permutation_product_poly.clone()));
                    direct_logging::log_elapsed(
                        "h_poly",
                        "permutation_set_cosets",
                        permutation_set_cosets_started_at,
                        &format!(
                            "proof_index={proof_index} set_idx={} set_count={}",
                            set_idx + 1,
                            total_sets
                        ),
                    );
                    let materialize_columns_started_at = Instant::now();
                    direct_logging::log_event(
                        "h_poly",
                        "permutation_column_views",
                        "start",
                        &format!(
                            "proof_index={proof_index} set_idx={} set_count={}",
                            set_idx + 1,
                            total_sets
                        ),
                    );
                    let column_values = columns
                        .iter()
                        .map(|&column| match column.column_type() {
                            Any::Advice(_) => advice[column.index()]
                                .as_ref()
                                .expect("permutation advice column should be materialized"),
                            Any::Fixed => fixed[column.index()]
                                .as_ref()
                                .expect("permutation fixed column should be materialized"),
                            Any::Instance => instance[column.index()]
                                .as_ref()
                                .expect("permutation instance column should be materialized"),
                        })
                        .collect::<Vec<_>>();
                    direct_logging::log_elapsed(
                        "h_poly",
                        "permutation_column_views",
                        materialize_columns_started_at,
                        &format!(
                            "proof_index={proof_index} set_idx={} set_count={} column_count={}",
                            set_idx + 1,
                            total_sets,
                            column_values.len()
                        ),
                    );

                    let left_products_init_started_at = Instant::now();
                    direct_logging::log_event(
                        "h_poly",
                        "permutation_left_products_init",
                        "start",
                        &format!(
                            "proof_index={proof_index} set_idx={} set_count={} column_count={}",
                            set_idx + 1,
                            total_sets,
                            column_values.len()
                        ),
                    );
                    let mut left_products = vec![F::ZERO; size];
                    parallelize(&mut left_products, |left_chunk, start| {
                        direct_logging::log_event(
                            "h_poly",
                            "permutation_left_products_init_chunk",
                            "iter",
                            &format!(
                                "proof_index={proof_index} set_idx={} chunk_start={} chunk_len={}",
                                set_idx + 1,
                                start,
                                left_chunk.len()
                            ),
                        );
                        for (i, left) in left_chunk.iter_mut().enumerate() {
                            let idx = start + i;
                            let r_next = get_rotation_idx(idx, 1, rot_scale, isize);
                            *left = current_set_permutation_product_coset[r_next];
                        }
                    });
                    direct_logging::log_elapsed(
                        "h_poly",
                        "permutation_left_products_init",
                        left_products_init_started_at,
                        &format!(
                            "proof_index={proof_index} set_idx={} set_count={} column_count={}",
                            set_idx + 1,
                            total_sets,
                            column_values.len()
                        ),
                    );

                    for (column_idx, (values, permutation_value)) in column_values
                        .iter()
                        .zip(permutation_values.iter())
                        .enumerate()
                    {
                        let sigma_started_at = Instant::now();
                        direct_logging::log_event(
                            "h_poly",
                            "permutation_sigma",
                            "iter",
                            &format!(
                                "proof_index={proof_index} iteration={} set_idx={} set_count={} column_idx={} column_count={}",
                                column_idx,
                                set_idx + 1,
                                total_sets,
                                column_idx + 1,
                                column_values.len()
                            ),
                        );
                        direct_logging::log_event(
                            "h_poly",
                            "permutation_sigma",
                            "start",
                            &format!(
                                "proof_index={proof_index} set_idx={} set_count={} column_idx={} column_count={}",
                                set_idx + 1,
                                total_sets,
                                column_idx + 1,
                                column_values.len()
                            ),
                        );
                        let sigma_coset = domain.lagrange_to_extended(permutation_value.clone());
                        parallelize(&mut left_products, |left_chunk, start| {
                            direct_logging::log_event(
                                "h_poly",
                                "permutation_sigma_chunk",
                                "iter",
                                &format!(
                                    "proof_index={proof_index} set_idx={} column_idx={} chunk_start={} chunk_len={}",
                                    set_idx + 1,
                                    column_idx + 1,
                                    start,
                                    left_chunk.len()
                                ),
                            );
                            for (i, left) in left_chunk.iter_mut().enumerate() {
                                let idx = start + i;
                                *left *= values[idx] + beta * sigma_coset[idx] + gamma;
                            }
                        });
                        direct_logging::log_elapsed(
                            "h_poly",
                            "permutation_sigma",
                            sigma_started_at,
                            &format!(
                                "proof_index={proof_index} set_idx={} set_count={} column_idx={} column_count={}",
                                set_idx + 1,
                                total_sets,
                                column_idx + 1,
                                column_values.len()
                            ),
                        );
                    }

                    let final_accumulation_started_at = Instant::now();
                    direct_logging::log_event(
                        "h_poly",
                        "permutation_final_accumulation",
                        "start",
                        &format!(
                            "proof_index={proof_index} set_idx={} set_count={} column_count={}",
                            set_idx + 1,
                            total_sets,
                            column_values.len()
                        ),
                    );
                    parallelize(&mut values, |values_chunk, start| {
                        direct_logging::log_event(
                            "h_poly",
                            "permutation_final_accumulation_chunk",
                            "iter",
                            &format!(
                                "proof_index={proof_index} set_idx={} chunk_start={} chunk_len={}",
                                set_idx + 1,
                                start,
                                values_chunk.len()
                            ),
                        );
                        let mut beta_term = omega.pow_vartime([start as u64, 0, 0, 0]);
                        for (i, value) in values_chunk.iter_mut().enumerate() {
                            let idx = start + i;
                            let current = current_set_permutation_product_coset[idx];

                            if set_idx == 0 {
                                *value = *value * y + ((one - current) * l0[idx]);
                            }
                            if set_idx + 1 == sets.len() && sets.len() > 1 {
                                *value = *value * y + ((current * current - current) * l_last[idx]);
                            }
                            if let Some(previous_set) = previous_set_permutation_product_coset.as_ref() {
                                let r_last = get_rotation_idx(idx, last_rotation.0, rot_scale, isize);
                                *value = *value * y + ((current - previous_set[r_last]) * l0[idx]);
                            }

                            let mut right = current;
                            let mut current_delta = delta_start * beta_term;
                            for values in &column_values {
                                right *= values[idx] + current_delta + gamma;
                                current_delta *= &F::DELTA;
                            }

                            *value = *value * y + ((left_products[idx] - right) * l_active_row[idx]);
                            beta_term *= &omega;
                        }
                    });
                    direct_logging::log_elapsed(
                        "h_poly",
                        "permutation_final_accumulation",
                        final_accumulation_started_at,
                        &format!(
                            "proof_index={proof_index} set_idx={} set_count={} column_count={}",
                            set_idx + 1,
                            total_sets,
                            column_values.len()
                        ),
                    );

                    previous_set_permutation_product_coset = Some(current_set_permutation_product_coset);
                }
                direct_logging::log_elapsed(
                    "h_poly",
                    "permutation_constraints",
                    permutation_constraints_started_at,
                    &format!(
                        "proof_index={proof_index} set_count={} row_chunk_size={row_chunk_size}",
                        sets.len()
                    ),
                );
            }

            // Lookups
            for (n, lookup) in lookups.iter().enumerate() {
                let lookup_cosets_started_at = Instant::now();
                direct_logging::log_event(
                    "h_poly",
                    "lookup_cosets",
                    "iter",
                    &format!("proof_index={proof_index} iteration={n} lookup_idx={n}"),
                );
                direct_logging::log_event(
                    "h_poly",
                    "lookup_cosets",
                    "start",
                    &format!("proof_index={proof_index} lookup_idx={n}"),
                );
                // Polynomials required for this lookup.
                // Calculated here so these only have to be kept in memory for the short time
                // they are actually needed.
                let product_coset = B::coeff_to_self(domain, lookup.product_poly.clone());
                let permuted_input_coset =
                    B::coeff_to_self(domain, lookup.permuted_input_poly.clone());
                let permuted_table_coset =
                    B::coeff_to_self(domain, lookup.permuted_table_poly.clone());
                direct_logging::log_elapsed(
                    "h_poly",
                    "lookup_cosets",
                    lookup_cosets_started_at,
                    &format!("proof_index={proof_index} lookup_idx={n}"),
                );

                // Lookup constraints
                let lookup_constraints_started_at = Instant::now();
                direct_logging::log_event(
                    "h_poly",
                    "lookup_constraints",
                    "start",
                    &format!("proof_index={proof_index} lookup_idx={n}"),
                );
                parallelize(&mut values, |values, start| {
                    direct_logging::log_event(
                        "h_poly",
                        "lookup_constraints_chunk",
                        "iter",
                        &format!(
                            "proof_index={proof_index} lookup_idx={n} chunk_start={} chunk_len={}",
                            start,
                            values.len()
                        ),
                    );
                    let lookup_evaluator = &self.lookups[n];
                    let mut eval_data = lookup_evaluator.instance();
                    for (i, value) in values.iter_mut().enumerate() {
                        let idx = start + i;

                        let table_value = lookup_evaluator.evaluate(
                            &mut eval_data,
                            fixed,
                            advice,
                            instance,
                            challenges,
                            &beta,
                            &gamma,
                            &theta,
                            &trash_challenge,
                            &y,
                            &F::ZERO,
                            idx,
                            rot_scale,
                            isize,
                        );

                        let r_next = get_rotation_idx(idx, 1, rot_scale, isize);
                        let r_prev = get_rotation_idx(idx, -1, rot_scale, isize);

                        let a_minus_s = permuted_input_coset[idx] - permuted_table_coset[idx];
                        // l_0(X) * (1 - z(X)) = 0
                        *value = *value * y + ((one - product_coset[idx]) * l0[idx]);
                        // l_last(X) * (z(X)^2 - z(X)) = 0
                        *value = *value * y
                            + ((product_coset[idx] * product_coset[idx] - product_coset[idx])
                                * l_last[idx]);
                        // (1 - (l_last(X) + l_blind(X))) * (
                        //   z(\omega X) (a'(X) + \beta) (s'(X) + \gamma)
                        //   - z(X) (\theta^{m-1} a_0(X) + ... + a_{m-1}(X) + \beta) (\theta^{m-1}
                        //     s_0(X) + ... + s_{m-1}(X) + \gamma)
                        // ) = 0
                        *value = *value * y
                            + ((product_coset[r_next]
                                * (permuted_input_coset[idx] + beta)
                                * (permuted_table_coset[idx] + gamma)
                                - product_coset[idx] * table_value)
                                * l_active_row[idx]);
                        // Check that the first values in the permuted input expression and permuted
                        // fixed expression are the same.
                        // l_0(X) * (a'(X) - s'(X)) = 0
                        *value = *value * y + (a_minus_s * l0[idx]);
                        // Check that each value in the permuted lookup input expression is either
                        // equal to the value above it, or the value at the same index in the
                        // permuted table expression.
                        // (1 - (l_last + l_blind)) * (a′(X) − s′(X))⋅(a′(X) − a′(\omega^{-1} X)) =
                        // 0
                        *value = *value * y
                            + (a_minus_s
                                * (permuted_input_coset[idx] - permuted_input_coset[r_prev])
                                * l_active_row[idx]);
                    }
                });
                direct_logging::log_elapsed(
                    "h_poly",
                    "lookup_constraints",
                    lookup_constraints_started_at,
                    &format!("proof_index={proof_index} lookup_idx={n}"),
                );
            }

            // Trashcans
            for (n, trash) in trashcans.iter().enumerate() {
                let trash_coset_started_at = Instant::now();
                direct_logging::log_event(
                    "h_poly",
                    "trash_coset",
                    "iter",
                    &format!("proof_index={proof_index} iteration={n} trash_idx={n}"),
                );
                direct_logging::log_event(
                    "h_poly",
                    "trash_coset",
                    "start",
                    &format!("proof_index={proof_index} trash_idx={n}"),
                );
                // Polynomials required for this trash argument.
                // Calculated here so these only have to be kept in memory for the short time
                // they are actually needed.
                let trash_poly = B::coeff_to_self(domain, trash.trash_poly.clone());
                direct_logging::log_elapsed(
                    "h_poly",
                    "trash_coset",
                    trash_coset_started_at,
                    &format!("proof_index={proof_index} trash_idx={n}"),
                );

                // Trash argument constraints.
                let trash_constraints_started_at = Instant::now();
                direct_logging::log_event(
                    "h_poly",
                    "trash_constraints",
                    "start",
                    &format!("proof_index={proof_index} trash_idx={n}"),
                );
                parallelize(&mut values, |values, start| {
                    direct_logging::log_event(
                        "h_poly",
                        "trash_constraints_chunk",
                        "iter",
                        &format!(
                            "proof_index={proof_index} trash_idx={n} chunk_start={} chunk_len={}",
                            start,
                            values.len()
                        ),
                    );
                    let trash_evaluator = &self.trashcans[n];
                    let argument = &cs.trashcans[n];
                    let mut eval_data = trash_evaluator.instance();
                    for (i, value) in values.iter_mut().enumerate() {
                        let idx = start + i;

                        let compressed_expression = trash_evaluator.evaluate(
                            &mut eval_data,
                            fixed,
                            advice,
                            instance,
                            challenges,
                            &beta,
                            &gamma,
                            &theta,
                            &trash_challenge,
                            &y,
                            &F::ZERO,
                            idx,
                            rot_scale,
                            isize,
                        );

                        let q = match argument.selector() {
                            Expression::Fixed(query) => fixed[query.column_index()]
                                .as_ref()
                                .expect("trash selector fixed column should be materialized")[idx],
                            _ => unreachable!(),
                        };

                        // compressed_expressions - (1 - q) * trash
                        *value = *value * y + (compressed_expression - (one - q) * trash_poly[idx]);
                    }
                });
                direct_logging::log_elapsed(
                    "h_poly",
                    "trash_constraints",
                    trash_constraints_started_at,
                    &format!("proof_index={proof_index} trash_idx={n}"),
                );
            }
        }
        values
    }
}

impl<F: PrimeField> Default for GraphEvaluator<F> {
    fn default() -> Self {
        Self {
            // Fixed positions to allow easy access
            constants: vec![F::ZERO, F::ONE, F::from(2u64)],
            rotations: Vec::new(),
            calculations: Vec::new(),
            num_intermediates: 0,
        }
    }
}

impl<F: PrimeField> GraphEvaluator<F> {
    fn collect_used_columns_from_value(
        value: &ValueSource,
        fixed: &mut BTreeSet<usize>,
        advice: &mut BTreeSet<usize>,
        instance: &mut BTreeSet<usize>,
    ) {
        match value {
            ValueSource::Fixed(column_index, _) => {
                fixed.insert(*column_index);
            }
            ValueSource::Advice(column_index, _) => {
                advice.insert(*column_index);
            }
            ValueSource::Instance(column_index, _) => {
                instance.insert(*column_index);
            }
            _ => {}
        }
    }

    /// Collects fixed/advice/instance columns referenced by this evaluator.
    pub fn collect_used_columns(
        &self,
        fixed: &mut BTreeSet<usize>,
        advice: &mut BTreeSet<usize>,
        instance: &mut BTreeSet<usize>,
    ) {
        for calc in &self.calculations {
            match &calc.calculation {
                Calculation::Add(a, b) | Calculation::Sub(a, b) | Calculation::Mul(a, b) => {
                    Self::collect_used_columns_from_value(a, fixed, advice, instance);
                    Self::collect_used_columns_from_value(b, fixed, advice, instance);
                }
                Calculation::Square(v)
                | Calculation::Double(v)
                | Calculation::Negate(v)
                | Calculation::Store(v) => {
                    Self::collect_used_columns_from_value(v, fixed, advice, instance);
                }
                Calculation::Horner(start_value, parts, factor) => {
                    Self::collect_used_columns_from_value(start_value, fixed, advice, instance);
                    Self::collect_used_columns_from_value(factor, fixed, advice, instance);
                    for part in parts {
                        Self::collect_used_columns_from_value(part, fixed, advice, instance);
                    }
                }
            }
        }
    }

    /// Adds a rotation
    fn add_rotation(&mut self, rotation: &Rotation) -> usize {
        let position = self.rotations.iter().position(|&c| c == rotation.0);
        match position {
            Some(pos) => pos,
            None => {
                self.rotations.push(rotation.0);
                self.rotations.len() - 1
            }
        }
    }

    /// Adds a constant
    fn add_constant(&mut self, constant: &F) -> ValueSource {
        let position = self.constants.iter().position(|&c| c == *constant);
        ValueSource::Constant(match position {
            Some(pos) => pos,
            None => {
                self.constants.push(*constant);
                self.constants.len() - 1
            }
        })
    }

    /// Adds a calculation.
    /// Currently does the simplest thing possible: just stores the
    /// resulting value so the result can be reused  when that calculation
    /// is done multiple times.
    fn add_calculation(&mut self, calculation: Calculation) -> ValueSource {
        let existing_calculation = self.calculations.iter().find(|c| c.calculation == calculation);
        match existing_calculation {
            Some(existing_calculation) => ValueSource::Intermediate(existing_calculation.target),
            None => {
                let target = self.num_intermediates;
                self.calculations.push(CalculationInfo {
                    calculation,
                    target,
                });
                self.num_intermediates += 1;
                ValueSource::Intermediate(target)
            }
        }
    }

    /// Generates an optimized evaluation for the expression
    fn add_expression(&mut self, expr: &Expression<F>) -> ValueSource {
        match expr {
            Expression::Constant(scalar) => self.add_constant(scalar),
            Expression::Selector(_selector) => unreachable!(),
            Expression::Fixed(query) => {
                let rot_idx = self.add_rotation(&query.rotation);
                self.add_calculation(Calculation::Store(ValueSource::Fixed(
                    query.column_index,
                    rot_idx,
                )))
            }
            Expression::Advice(query) => {
                let rot_idx = self.add_rotation(&query.rotation);
                self.add_calculation(Calculation::Store(ValueSource::Advice(
                    query.column_index,
                    rot_idx,
                )))
            }
            Expression::Instance(query) => {
                let rot_idx = self.add_rotation(&query.rotation);
                self.add_calculation(Calculation::Store(ValueSource::Instance(
                    query.column_index,
                    rot_idx,
                )))
            }
            Expression::Challenge(challenge) => self.add_calculation(Calculation::Store(
                ValueSource::Challenge(challenge.index()),
            )),
            Expression::Negated(a) => match **a {
                Expression::Constant(scalar) => self.add_constant(&-scalar),
                _ => {
                    let result_a = self.add_expression(a);
                    match result_a {
                        ValueSource::Constant(0) => result_a,
                        _ => self.add_calculation(Calculation::Negate(result_a)),
                    }
                }
            },
            Expression::Sum(a, b) => {
                // Undo subtraction stored as a + (-b) in expressions
                match &**b {
                    Expression::Negated(b_int) => {
                        let result_a = self.add_expression(a);
                        let result_b = self.add_expression(b_int);
                        if result_a == ValueSource::Constant(0) {
                            self.add_calculation(Calculation::Negate(result_b))
                        } else if result_b == ValueSource::Constant(0) {
                            result_a
                        } else {
                            self.add_calculation(Calculation::Sub(result_a, result_b))
                        }
                    }
                    _ => {
                        let result_a = self.add_expression(a);
                        let result_b = self.add_expression(b);
                        if result_a == ValueSource::Constant(0) {
                            result_b
                        } else if result_b == ValueSource::Constant(0) {
                            result_a
                        } else if result_a <= result_b {
                            self.add_calculation(Calculation::Add(result_a, result_b))
                        } else {
                            self.add_calculation(Calculation::Add(result_b, result_a))
                        }
                    }
                }
            }
            Expression::Product(a, b) => {
                let result_a = self.add_expression(a);
                let result_b = self.add_expression(b);
                if result_a == ValueSource::Constant(0) || result_b == ValueSource::Constant(0) {
                    ValueSource::Constant(0)
                } else if result_a == ValueSource::Constant(1) {
                    result_b
                } else if result_b == ValueSource::Constant(1) {
                    result_a
                } else if result_a == ValueSource::Constant(2) {
                    self.add_calculation(Calculation::Double(result_b))
                } else if result_b == ValueSource::Constant(2) {
                    self.add_calculation(Calculation::Double(result_a))
                } else if result_a == result_b {
                    self.add_calculation(Calculation::Square(result_a))
                } else if result_a <= result_b {
                    self.add_calculation(Calculation::Mul(result_a, result_b))
                } else {
                    self.add_calculation(Calculation::Mul(result_b, result_a))
                }
            }
            Expression::Scaled(a, f) => {
                if *f == F::ZERO {
                    ValueSource::Constant(0)
                } else if *f == F::ONE {
                    self.add_expression(a)
                } else {
                    let cst = self.add_constant(f);
                    let result_a = self.add_expression(a);
                    self.add_calculation(Calculation::Mul(result_a, cst))
                }
            }
        }
    }

    /// Creates a new evaluation structure
    pub fn instance(&self) -> EvaluationData<F> {
        EvaluationData {
            intermediates: vec![F::ZERO; self.num_intermediates],
            rotations: vec![0usize; self.rotations.len()],
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn evaluate<B: PolynomialRepresentation>(
        &self,
        data: &mut EvaluationData<F>,
        fixed: &[Option<Polynomial<F, B>>],
        advice: &[Option<Polynomial<F, B>>],
        instance: &[Option<Polynomial<F, B>>],
        challenges: &[F],
        beta: &F,
        gamma: &F,
        theta: &F,
        trash_challenge: &F,
        y: &F,
        previous_value: &F,
        idx: usize,
        rot_scale: i32,
        isize: i32,
    ) -> F {
        // All rotation index values
        for (rot_idx, rot) in self.rotations.iter().enumerate() {
            data.rotations[rot_idx] = get_rotation_idx(idx, *rot, rot_scale, isize);
        }

        // All calculations, with cached intermediate results
        for calc in self.calculations.iter() {
            data.intermediates[calc.target] = calc.calculation.evaluate(
                &data.rotations,
                &self.constants,
                &data.intermediates,
                fixed,
                advice,
                instance,
                challenges,
                beta,
                gamma,
                theta,
                trash_challenge,
                y,
                previous_value,
            );
        }

        // Return the result of the last calculation (if any)
        if let Some(calc) = self.calculations.last() {
            data.intermediates[calc.target]
        } else {
            F::ZERO
        }
    }
}

/// Simple evaluation of an expression
pub fn evaluate<F: Field, B: PolynomialRepresentation>(
    expression: &Expression<F>,
    size: usize,
    rot_scale: i32,
    fixed: &[Polynomial<F, B>],
    advice: &[Polynomial<F, B>],
    instance: &[Polynomial<F, B>],
    challenges: &[F],
) -> Vec<F> {
    let mut values = vec![F::ZERO; size];
    let isize = size as i32;
    parallelize(&mut values, |values, start| {
        for (i, value) in values.iter_mut().enumerate() {
            let idx = start + i;
            *value = expression.evaluate(
                &|scalar| scalar,
                &|_| panic!("virtual selectors are removed during optimization"),
                &|query| {
                    fixed[query.column_index]
                        [get_rotation_idx(idx, query.rotation.0, rot_scale, isize)]
                },
                &|query| {
                    advice[query.column_index]
                        [get_rotation_idx(idx, query.rotation.0, rot_scale, isize)]
                },
                &|query| {
                    instance[query.column_index]
                        [get_rotation_idx(idx, query.rotation.0, rot_scale, isize)]
                },
                &|challenge| challenges[challenge.index()],
                &|a| -a,
                &|a, b| a + &b,
                &|a, b| a * b,
                &|a, scalar| a * scalar,
            );
        }
    });
    values
}
