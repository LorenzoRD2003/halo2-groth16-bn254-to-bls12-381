//! Representation of a Trace for a batch of proofs that are being generated
//! simultaneously.

use ff::{PrimeField, WithSmallOrderMulGroup};
use midnight_curves::serde::SerdeObject;
use std::io;

use crate::{
    plonk::{lookup, permutation, trash, vanishing, Any, ConstraintSystem, Evaluator, VerifyingKey},
    poly::{
        commitment::PolynomialCommitmentScheme, Coeff, EvaluationDomain, ExtendedLagrangeCoeff,
        Polynomial,
    },
    transcript::Transcript,
    utils::{helpers::{read_polynomial_vec, write_polynomial_slice}, SerdeFormat},
};

/// Prover's trace of a set of proofs. This type guarantees that the size of the
/// outer vector of its fields has the same size.
#[derive(Debug)]
pub struct ProverTrace<F: PrimeField> {
    pub(crate) advice_polys: Vec<Vec<Polynomial<F, Coeff>>>,
    pub(crate) instance_polys: Vec<Vec<Polynomial<F, Coeff>>>,
    pub(crate) vanishing: vanishing::prover::Committed<F>,
    pub(crate) lookups: Vec<Vec<lookup::prover::Committed<F>>>,
    pub(crate) trashcans: Vec<Vec<trash::prover::Committed<F>>>,
    pub(crate) permutations: Vec<permutation::prover::Committed<F>>,
    pub(crate) challenges: Vec<F>,
    pub(crate) beta: F,
    pub(crate) gamma: F,
    pub(crate) theta: F,
    pub(crate) trash_challenge: F,
    pub(crate) y: F,
}

/// Verifier's trace of a set of proofs. This type guarantees that the size of
/// the outer vector of its fields has the same size.
#[derive(Debug)]
pub struct VerifierTrace<F: PrimeField, PCS: PolynomialCommitmentScheme<F>> {
    pub(crate) advice_commitments: Vec<Vec<PCS::Commitment>>,
    pub(crate) vanishing: vanishing::verifier::Committed<F, PCS>,
    pub(crate) lookups: Vec<Vec<lookup::verifier::Committed<F, PCS>>>,
    pub(crate) trashcans: Vec<Vec<trash::verifier::Committed<F, PCS>>>,
    pub(crate) permutations: Vec<permutation::verifier::Committed<F, PCS>>,
    pub(crate) challenges: Vec<F>,
    pub(crate) beta: F,
    pub(crate) gamma: F,
    pub(crate) theta: F,
    pub(crate) trash_challenge: F,
    pub(crate) y: F,
}

/// Persisted first section consumed by proof finalization before transcript
/// evaluation and opening-query construction.
#[derive(Debug)]
pub struct PreparedFinalizationTrace<F: PrimeField> {
    transcript_prefix: Vec<u8>,
    pub(crate) advice_cosets: Vec<Vec<Option<Polynomial<F, ExtendedLagrangeCoeff>>>>,
    pub(crate) instance_cosets: Vec<Vec<Option<Polynomial<F, ExtendedLagrangeCoeff>>>>,
    pub(crate) vanishing: vanishing::prover::Committed<F>,
    pub(crate) lookups: Vec<Vec<lookup::prover::Committed<F>>>,
    pub(crate) trashcans: Vec<Vec<trash::prover::Committed<F>>>,
    pub(crate) permutations: Vec<permutation::prover::Committed<F>>,
    pub(crate) challenges: Vec<F>,
    pub(crate) beta: F,
    pub(crate) gamma: F,
    pub(crate) theta: F,
    pub(crate) trash_challenge: F,
    pub(crate) y: F,
}

impl<F: PrimeField> PreparedFinalizationTrace<F> {
    /// Reconstructs a transcript from the persisted prefix bytes.
    pub fn init_transcript<T: Transcript>(&self) -> T {
        T::init_from_bytes(&self.transcript_prefix)
    }
}

/// Persisted second section containing coefficient-form witness polynomials
/// needed for transcript evaluations and multi-opening queries.
#[derive(Debug)]
pub struct OpeningPolysTrace<F: PrimeField> {
    pub(crate) advice_polys: Vec<Vec<Polynomial<F, Coeff>>>,
    pub(crate) instance_polys: Vec<Vec<Polynomial<F, Coeff>>>,
}

type OptionalPolynomialGroups<F, B> = Vec<Vec<Option<Polynomial<F, B>>>>;

/// Persisted prover-side trace artifact together with the transcript bytes
/// required to resume proof finalization later.
#[derive(Debug)]
pub struct PersistedProverTrace<F: PrimeField> {
    transcript_prefix: Vec<u8>,
    trace: ProverTrace<F>,
}

impl<F: PrimeField> PersistedProverTrace<F> {
    /// Builds a persisted prover trace artifact from transcript bytes and a
    /// prover trace.
    pub fn new(transcript_prefix: Vec<u8>, trace: ProverTrace<F>) -> Self {
        Self {
            transcript_prefix,
            trace,
        }
    }

    /// Reconstructs a transcript from the persisted prefix bytes.
    pub fn init_transcript<T: Transcript>(&self) -> T {
        T::init_from_bytes(&self.transcript_prefix)
    }

    /// Consumes the persisted artifact and returns the underlying prover trace.
    pub fn into_trace(self) -> ProverTrace<F> {
        self.trace
    }
}

impl<F: PrimeField + SerdeObject> PersistedProverTrace<F> {
    /// Writes the persisted prover trace artifact to a binary buffer in a
    /// layout that lets finalization load the prepared pre-`compute_h_poly`
    /// cosets and the coefficient-form witness polynomials in separate stages.
    pub fn write<W: io::Write, CS: PolynomialCommitmentScheme<F>>(
        &self,
        writer: &mut W,
        vk: &VerifyingKey<F, CS>,
    ) -> io::Result<()>
    where
        F: WithSmallOrderMulGroup<3>,
    {
        let domain = vk.get_domain();
        let (used_advice_columns, used_instance_columns) = collect_used_columns(vk.cs());
        writer.write_all(&(self.transcript_prefix.len() as u32).to_be_bytes())?;
        writer.write_all(&self.transcript_prefix)?;

        write_nested_optional_extended_from_coeffs(
            &self.trace.advice_polys,
            domain,
            &used_advice_columns,
            writer,
        )?;
        write_nested_optional_extended_from_coeffs(
            &self.trace.instance_polys,
            domain,
            &used_instance_columns,
            writer,
        )?;

        self.trace.vanishing.random_poly.write(writer)?;

        writer.write_all(&(self.trace.lookups.len() as u32).to_be_bytes())?;
        for lookup_group in &self.trace.lookups {
          writer.write_all(&(lookup_group.len() as u32).to_be_bytes())?;
          for lookup in lookup_group {
            lookup.permuted_input_poly.write(writer)?;
            lookup.permuted_table_poly.write(writer)?;
            lookup.product_poly.write(writer)?;
          }
        }

        writer.write_all(&(self.trace.trashcans.len() as u32).to_be_bytes())?;
        for trash_group in &self.trace.trashcans {
          writer.write_all(&(trash_group.len() as u32).to_be_bytes())?;
          for trash in trash_group {
            trash.trash_poly.write(writer)?;
          }
        }

        writer.write_all(&(self.trace.permutations.len() as u32).to_be_bytes())?;
        for permutation in &self.trace.permutations {
          writer.write_all(&(permutation.sets.len() as u32).to_be_bytes())?;
          for set in &permutation.sets {
            set.permutation_product_poly.write(writer)?;
          }
        }

        writer.write_all(&(self.trace.challenges.len() as u32).to_be_bytes())?;
        for challenge in &self.trace.challenges {
          challenge.write_raw(writer)?;
        }
        self.trace.beta.write_raw(writer)?;
        self.trace.gamma.write_raw(writer)?;
        self.trace.theta.write_raw(writer)?;
        self.trace.trash_challenge.write_raw(writer)?;
        self.trace.y.write_raw(writer)?;

        write_nested_polynomials(&self.trace.advice_polys, writer)?;
        write_nested_polynomials(&self.trace.instance_polys, writer)?;
        Ok(())
    }

    /// Reads the first section of the persisted prover trace artifact from a
    /// binary buffer.
    pub fn read_prepared<R: io::Read>(reader: &mut R) -> io::Result<PreparedFinalizationTrace<F>> {
        let mut prefix_len_bytes = [0u8; 4];
        reader.read_exact(&mut prefix_len_bytes)?;
        let prefix_len = u32::from_be_bytes(prefix_len_bytes) as usize;
        let mut transcript_prefix = vec![0u8; prefix_len];
        reader.read_exact(&mut transcript_prefix)?;

        let advice_cosets = read_nested_optional_polynomials(reader)?;
        let instance_cosets = read_nested_optional_polynomials(reader)?;

        let vanishing = vanishing::prover::Committed {
          random_poly: Polynomial::read(reader, SerdeFormat::Processed)?,
        };

        let mut count_bytes = [0u8; 4];
        reader.read_exact(&mut count_bytes)?;
        let lookup_outer_len = u32::from_be_bytes(count_bytes) as usize;
        let mut lookups = Vec::with_capacity(lookup_outer_len);
        for _ in 0..lookup_outer_len {
          reader.read_exact(&mut count_bytes)?;
          let lookup_inner_len = u32::from_be_bytes(count_bytes) as usize;
          let mut lookup_group = Vec::with_capacity(lookup_inner_len);
          for _ in 0..lookup_inner_len {
            lookup_group.push(lookup::prover::Committed {
              permuted_input_poly: Polynomial::read(reader, SerdeFormat::Processed)?,
              permuted_table_poly: Polynomial::read(reader, SerdeFormat::Processed)?,
              product_poly: Polynomial::read(reader, SerdeFormat::Processed)?,
            });
          }
          lookups.push(lookup_group);
        }

        reader.read_exact(&mut count_bytes)?;
        let trash_outer_len = u32::from_be_bytes(count_bytes) as usize;
        let mut trashcans = Vec::with_capacity(trash_outer_len);
        for _ in 0..trash_outer_len {
          reader.read_exact(&mut count_bytes)?;
          let trash_inner_len = u32::from_be_bytes(count_bytes) as usize;
          let mut trash_group = Vec::with_capacity(trash_inner_len);
          for _ in 0..trash_inner_len {
            trash_group.push(trash::prover::Committed {
              trash_poly: Polynomial::read(reader, SerdeFormat::Processed)?,
            });
          }
          trashcans.push(trash_group);
        }

        reader.read_exact(&mut count_bytes)?;
        let permutation_len = u32::from_be_bytes(count_bytes) as usize;
        let mut permutations = Vec::with_capacity(permutation_len);
        for _ in 0..permutation_len {
          reader.read_exact(&mut count_bytes)?;
          let set_len = u32::from_be_bytes(count_bytes) as usize;
          let mut sets = Vec::with_capacity(set_len);
          for _ in 0..set_len {
            sets.push(permutation::prover::CommittedSet {
              permutation_product_poly: Polynomial::read(reader, SerdeFormat::Processed)?,
            });
          }
          permutations.push(permutation::prover::Committed { sets });
        }

        reader.read_exact(&mut count_bytes)?;
        let challenges_len = u32::from_be_bytes(count_bytes) as usize;
        let mut challenges = Vec::with_capacity(challenges_len);
        for _ in 0..challenges_len {
          challenges.push(F::read_raw(reader)?);
        }
        let beta = F::read_raw(reader)?;
        let gamma = F::read_raw(reader)?;
        let theta = F::read_raw(reader)?;
        let trash_challenge = F::read_raw(reader)?;
        let y = F::read_raw(reader)?;

        Ok(PreparedFinalizationTrace {
          transcript_prefix,
          advice_cosets,
          instance_cosets,
          vanishing,
          lookups,
          trashcans,
          permutations,
          challenges,
          beta,
          gamma,
          theta,
          trash_challenge,
          y,
        })
    }

    /// Reads the second section of the persisted prover trace artifact from a
    /// binary buffer.
    pub fn read_opening_polys<R: io::Read>(reader: &mut R) -> io::Result<OpeningPolysTrace<F>> {
        let advice_polys = read_nested_polynomials(reader)?;
        let instance_polys = read_nested_polynomials(reader)?;
        Ok(OpeningPolysTrace { advice_polys, instance_polys })
    }
}

fn write_nested_polynomials<F: PrimeField + SerdeObject, B, W: io::Write>(
    values: &[Vec<Polynomial<F, B>>],
    writer: &mut W,
) -> io::Result<()> {
    writer.write_all(&(values.len() as u32).to_be_bytes())?;
    for group in values {
        write_polynomial_slice(group, writer)?;
    }
    Ok(())
}

fn read_nested_polynomials<F: PrimeField + SerdeObject, B, R: io::Read>(
    reader: &mut R,
) -> io::Result<Vec<Vec<Polynomial<F, B>>>> {
    let mut outer_len = [0u8; 4];
    reader.read_exact(&mut outer_len)?;
    let outer_len = u32::from_be_bytes(outer_len) as usize;
    (0..outer_len)
        .map(|_| read_polynomial_vec(reader, SerdeFormat::Processed))
        .collect()
}

fn write_nested_optional_extended_from_coeffs<
    F: PrimeField + WithSmallOrderMulGroup<3> + SerdeObject,
    W: io::Write,
>(
    values: &[Vec<Polynomial<F, Coeff>>],
    domain: &EvaluationDomain<F>,
    used_columns: &[bool],
    writer: &mut W,
) -> io::Result<()> {
    writer.write_all(&(values.len() as u32).to_be_bytes())?;
    for group in values {
        writer.write_all(&(group.len() as u32).to_be_bytes())?;
        for (column_index, poly) in group.iter().enumerate() {
            if used_columns.get(column_index).copied().unwrap_or(false) {
                writer.write_all(&[1_u8])?;
                domain.coeff_to_extended(poly.clone()).write(writer)?;
            } else {
                writer.write_all(&[0_u8])?;
            }
        }
    }
    Ok(())
}

fn read_nested_optional_polynomials<
    F: PrimeField + SerdeObject,
    B,
    R: io::Read,
>(
    reader: &mut R,
) -> io::Result<OptionalPolynomialGroups<F, B>> {
    let mut outer_len = [0_u8; 4];
    reader.read_exact(&mut outer_len)?;
    let outer_len = u32::from_be_bytes(outer_len) as usize;
    let mut groups = Vec::with_capacity(outer_len);
    for _ in 0..outer_len {
        let mut inner_len = [0_u8; 4];
        reader.read_exact(&mut inner_len)?;
        let inner_len = u32::from_be_bytes(inner_len) as usize;
        let mut group = Vec::with_capacity(inner_len);
        for _ in 0..inner_len {
            let mut present = [0_u8; 1];
            reader.read_exact(&mut present)?;
            if present[0] == 0 {
                group.push(None);
            } else {
                group.push(Some(Polynomial::read(reader, SerdeFormat::Processed)?));
            }
        }
        groups.push(group);
    }
    Ok(groups)
}

fn collect_used_columns<F: PrimeField + WithSmallOrderMulGroup<3>>(
    cs: &ConstraintSystem<F>,
) -> (Vec<bool>, Vec<bool>) {
    let ev = Evaluator::new(cs);
    let mut used_fixed_columns = std::collections::BTreeSet::new();
    let mut used_advice_columns = std::collections::BTreeSet::new();
    let mut used_instance_columns = std::collections::BTreeSet::new();
    ev.custom_gates.collect_used_columns(
        &mut used_fixed_columns,
        &mut used_advice_columns,
        &mut used_instance_columns,
    );
    for evaluator in &ev.lookups {
        evaluator.collect_used_columns(
            &mut used_fixed_columns,
            &mut used_advice_columns,
            &mut used_instance_columns,
        );
    }
    for evaluator in &ev.trashcans {
        evaluator.collect_used_columns(
            &mut used_fixed_columns,
            &mut used_advice_columns,
            &mut used_instance_columns,
        );
    }
    for column in &cs.permutation.columns {
        match column.column_type() {
            Any::Advice(_) => {
                used_advice_columns.insert(column.index());
            }
            Any::Fixed => {
                used_fixed_columns.insert(column.index());
            }
            Any::Instance => {
                used_instance_columns.insert(column.index());
            }
        }
    }
    let _ = used_fixed_columns;
    let advice = (0..cs.num_advice_columns)
        .map(|index| used_advice_columns.contains(&index))
        .collect();
    let instance = (0..cs.num_instance_columns)
        .map(|index| used_instance_columns.contains(&index))
        .collect();
    (advice, instance)
}
