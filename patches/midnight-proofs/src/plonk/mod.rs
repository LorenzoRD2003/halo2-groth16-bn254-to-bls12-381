//! This module provides an implementation of a variant of (Turbo)[PLONK][plonk]
//! that is designed specifically for the polynomial commitment scheme described
//! in the [Halo][halo] paper.
//!
//! [halo]: https://eprint.iacr.org/2019/1021
//! [plonk]: https://eprint.iacr.org/2019/953

use blake2b_simd::Params as Blake2bParams;
use group::ff::FromUniformBytes;
use std::{io, time::Instant};

use crate::{
    poly::{
        Coeff, EvaluationDomain, ExtendedLagrangeCoeff, LagrangeCoeff, PinnedEvaluationDomain,
        Polynomial,
    },
    transcript::{Hashable, Transcript},
    utils::{
        helpers::{
            byte_length, polynomial_slice_byte_length, read_polynomial_vec, write_polynomial_slice,
            ProcessedSerdeObject,
        },
        SerdeFormat,
    },
};

mod circuit;
mod direct_logging;
mod error;
pub(crate) mod evaluation;
mod keygen;
pub(crate) mod lookup;
pub mod permutation;
pub mod traces;
pub(crate) mod trash;
pub(crate) mod vanishing;

#[cfg(feature = "bench-internal")]
pub mod bench;

mod prover;
mod verifier;

pub use circuit::*;
pub use error::*;
pub(crate) use evaluation::Evaluator;
use ff::{PrimeField, WithSmallOrderMulGroup};
pub use keygen::*;
use midnight_curves::serde::SerdeObject;
pub use prover::*;
pub use verifier::*;

use crate::poly::commitment::PolynomialCommitmentScheme;

fn collect_fixed_columns_from_expression<F: group::ff::Field>(
    expression: &Expression<F>,
    fixed_columns: &mut std::collections::BTreeSet<usize>,
) {
    match expression {
        Expression::Fixed(query) => {
            fixed_columns.insert(query.column_index);
        }
        Expression::Negated(inner) | Expression::Scaled(inner, _) => {
            collect_fixed_columns_from_expression(inner, fixed_columns);
        }
        Expression::Sum(left, right) | Expression::Product(left, right) => {
            collect_fixed_columns_from_expression(left, fixed_columns);
            collect_fixed_columns_from_expression(right, fixed_columns);
        }
        Expression::Constant(_)
        | Expression::Selector(_)
        | Expression::Advice(_)
        | Expression::Instance(_)
        | Expression::Challenge(_) => {}
    }
}

/// This is a verifying key which allows for the verification of proofs for a
/// particular circuit.
#[derive(Clone, Debug)]
pub struct VerifyingKey<F: PrimeField, CS: PolynomialCommitmentScheme<F>> {
    domain: EvaluationDomain<F>,
    fixed_commitments: Vec<CS::Commitment>,
    permutation: permutation::VerifyingKey<F, CS>,
    cs: ConstraintSystem<F>,
    /// Cached maximum degree of `cs` (which doesn't change after construction).
    cs_degree: usize,
    /// The representative of this `VerifyingKey` in transcripts.
    transcript_repr: F,
}

// Current version of the VK
const VERSION: u8 = 0x03;

impl<F, CS> VerifyingKey<F, CS>
where
    F: WithSmallOrderMulGroup<3> + FromUniformBytes<64>,
    CS: PolynomialCommitmentScheme<F>,
{
    /// Returns `n`
    pub fn n(&self) -> u64 {
        self.domain.n
    }
    /// Writes a verifying key to a buffer.
    ///
    /// Writes a curve element according to `format`:
    /// - `Processed`: Writes a compressed curve element with coordinates in
    ///   standard form. Writes a field element in standard form, with
    ///   endianness specified by the `PrimeField` implementation.
    /// - Otherwise: Writes an uncompressed curve element with coordinates in
    ///   Montgomery form Writes a field element into raw bytes in its internal
    ///   Montgomery representation, WITHOUT performing the expensive Montgomery
    ///   reduction.
    pub fn write<W: io::Write>(&self, writer: &mut W, format: SerdeFormat) -> io::Result<()> {
        // Version byte that will be checked on read.
        writer.write_all(&[VERSION])?;
        let k = &self.domain.k();
        assert!(*k <= F::S);
        // k value fits in 1 byte
        writer.write_all(&[*k as u8])?;
        writer.write_all(&(self.fixed_commitments.len() as u32).to_le_bytes())?;
        for commitment in &self.fixed_commitments {
            commitment.write(writer, format)?;
        }
        self.permutation.write(writer, format)?;

        Ok(())
    }

    /// Reads a verification key from a buffer for the associated [Circuit].
    ///
    /// Reads a curve element from the buffer and parses it according to the
    /// `format`:
    /// - `Processed`: Reads a compressed curve element and decompresses it.
    ///   Reads a field element in standard form, with endianness specified by
    ///   the `PrimeField` implementation, and checks that the element is less
    ///   than the modulus.
    /// - `RawBytes`: Reads an uncompressed curve element with coordinates in
    ///   Montgomery form. Checks that field elements are less than modulus, and
    ///   then checks that the point is on the curve.
    /// - `RawBytesUnchecked`: Reads an uncompressed curve element with
    ///   coordinates in Montgomery form; does not perform any checks.
    pub fn read<R: io::Read, ConcreteCircuit: Circuit<F>>(
        reader: &mut R,
        format: SerdeFormat,
        #[cfg(feature = "circuit-params")] params: ConcreteCircuit::Params,
    ) -> io::Result<Self> {
        let mut cs = ConstraintSystem::default();
        #[cfg(feature = "circuit-params")]
        let _config = ConcreteCircuit::configure_with_params(&mut cs, params);
        #[cfg(not(feature = "circuit-params"))]
        let _config = ConcreteCircuit::configure(&mut cs);

        Self::read_from_cs(reader, format, cs)
    }

    /// Reads a verification key from a buffer, using the provided
    /// [ConstraintSystem].
    ///
    /// Reads a curve element from the buffer and parses it according to the
    /// `format`:
    /// - `Processed`: Reads a compressed curve element and decompresses it.
    ///   Reads a field element in standard form, with endianness specified by
    ///   the `PrimeField` implementation, and checks that the element is less
    ///   than the modulus.
    /// - `RawBytes`: Reads an uncompressed curve element with coordinates in
    ///   Montgomery form. Checks that field elements are less than modulus, and
    ///   then checks that the point is on the curve.
    /// - `RawBytesUnchecked`: Reads an uncompressed curve element with
    ///   coordinates in Montgomery form; does not perform any checks.
    pub fn read_from_cs<R: io::Read>(
        reader: &mut R,
        format: SerdeFormat,
        cs: ConstraintSystem<F>,
    ) -> io::Result<Self> {
        let mut version_byte = [0u8; 1];
        reader.read_exact(&mut version_byte)?;
        if VERSION != version_byte[0] {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unexpected version byte",
            ));
        }

        let mut k = [0u8; 1];
        reader.read_exact(&mut k)?;
        let k = u8::from_le_bytes(k);
        if k as u32 > F::S {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("circuit size value (k): {} exceeds maxium: {}", k, F::S),
            ));
        }

        let domain = EvaluationDomain::new(cs.degree() as u32, k.into());

        let mut num_fixed_columns = [0u8; 4];
        reader.read_exact(&mut num_fixed_columns)?;
        let num_fixed_columns = u32::from_le_bytes(num_fixed_columns);

        let fixed_commitments: Vec<_> = (0..num_fixed_columns)
            .map(|_| CS::Commitment::read(reader, format))
            .collect::<Result<_, _>>()?;

        let permutation = permutation::VerifyingKey::read(reader, &cs.permutation, format)?;

        // we still need to replace selectors with fixed Expressions in `cs`
        let fake_selectors = vec![vec![]; cs.num_selectors];
        let (cs, _) = cs.directly_convert_selectors_to_fixed(fake_selectors);

        Ok(Self::from_parts(domain, fixed_commitments, permutation, cs))
    }

    /// Writes a verifying key to a vector of bytes using [`Self::write`].
    pub fn to_bytes(&self, format: SerdeFormat) -> Vec<u8> {
        let mut bytes = Vec::<u8>::with_capacity(self.bytes_length(format));
        Self::write(self, &mut bytes, format).expect("Writing to vector should not fail");
        bytes
    }

    /// Reads a verification key from a slice of bytes using [`Self::read`].
    pub fn from_bytes<ConcreteCircuit: Circuit<F>>(
        mut bytes: &[u8],
        format: SerdeFormat,
        #[cfg(feature = "circuit-params")] params: ConcreteCircuit::Params,
    ) -> io::Result<Self> {
        Self::read::<_, ConcreteCircuit>(
            &mut bytes,
            format,
            #[cfg(feature = "circuit-params")]
            params,
        )
    }
}

impl<F: WithSmallOrderMulGroup<3>, CS: PolynomialCommitmentScheme<F>> VerifyingKey<F, CS> {
    /// Return the bytes_length of a VerifyingKey
    pub fn bytes_length(&self, format: SerdeFormat) -> usize {
        10 + (self.fixed_commitments.len() * byte_length::<CS::Commitment>(format))
            + self.permutation.bytes_length(format)
    }

    fn from_parts(
        domain: EvaluationDomain<F>,
        fixed_commitments: Vec<CS::Commitment>,
        permutation: permutation::VerifyingKey<F, CS>,
        cs: ConstraintSystem<F>,
    ) -> Self
    where
        F: FromUniformBytes<64>,
    {
        // Compute cached values.
        let cs_degree = cs.degree();

        let mut vk = Self {
            domain,
            fixed_commitments,
            permutation,
            cs,
            cs_degree,
            // Temporary, this is not pinned.
            transcript_repr: F::ZERO,
        };

        let mut hasher =
            Blake2bParams::new().hash_length(64).personal(b"Halo2-Verify-Key").to_state();

        // We serialise the commitments of the VK to get the `transcript_repr`.
        let mut buffer = Vec::new();
        buffer.push(VERSION);
        let k = &vk.domain.k();
        assert!(*k <= F::S);
        buffer.push(*k as u8);
        buffer.extend_from_slice(&(vk.fixed_commitments.len() as u32).to_le_bytes());
        for commitment in &vk.fixed_commitments {
            commitment
                .write(&mut buffer, SerdeFormat::RawBytesUnchecked)
                .expect("Failed to write to buffer - this is a bug.");
        }

        buffer.extend_from_slice(&(vk.permutation.commitments().len() as u32).to_le_bytes());
        for commitment in vk.permutation.commitments() {
            commitment
                .write(&mut buffer, SerdeFormat::RawBytesUnchecked)
                .expect("Failed to write to buffer - this is a bug.");
        }

        // We use the debug implementation to add the gates and domain to the hashed
        // buffer. We should eventually move away from debug implementation for
        // this purpose. See https://github.com/midnightntwrk/halo2/issues/5
        buffer.extend_from_slice(format!("{:?}", vk.get_domain().pinned()).as_bytes());
        buffer.extend_from_slice(format!("{:?}", vk.cs().pinned()).as_bytes());

        hasher.update(&buffer);

        // Hash in final Blake2bState
        vk.transcript_repr = F::from_uniform_bytes(hasher.finalize().as_array());

        vk
    }

    /// Hashes a verification key into a transcript.
    pub fn hash_into<T: Transcript>(&self, transcript: &mut T) -> io::Result<()>
    where
        F: Hashable<T::Hash>,
    {
        transcript.common(&self.transcript_repr)?;

        Ok(())
    }

    /// Obtains a pinned representation of this verification key that contains
    /// the minimal information necessary to reconstruct the verification key.
    pub fn pinned(&self) -> PinnedVerificationKey<'_, F, CS> {
        PinnedVerificationKey {
            domain: self.domain.pinned(),
            fixed_commitments: &self.fixed_commitments,
            permutation: &self.permutation,
            cs: self.cs.pinned(),
        }
    }

    /// Returns commitments of fixed polynomials
    pub fn fixed_commitments(&self) -> &Vec<CS::Commitment> {
        &self.fixed_commitments
    }

    /// Returns `VerifyingKey` of permutation
    pub fn permutation(&self) -> &permutation::VerifyingKey<F, CS> {
        &self.permutation
    }

    /// Returns `ConstraintSystem`
    pub fn cs(&self) -> &ConstraintSystem<F> {
        &self.cs
    }

    /// Returns representative of this `VerifyingKey` in transcripts
    pub fn transcript_repr(&self) -> F {
        self.transcript_repr
    }
}

/// Minimal representation of a verification key that can be used to identify
/// its active contents.
#[allow(dead_code)]
#[derive(Debug)]
pub struct PinnedVerificationKey<'a, F: PrimeField, CS: PolynomialCommitmentScheme<F>> {
    domain: PinnedEvaluationDomain<'a, F>,
    cs: PinnedConstraintSystem<'a, F>,
    fixed_commitments: &'a Vec<CS::Commitment>,
    permutation: &'a permutation::VerifyingKey<F, CS>,
}
/// This is a proving key which allows for the creation of proofs for a
/// particular circuit.
#[derive(Clone, Debug)]
pub struct ProvingKey<F: PrimeField, CS: PolynomialCommitmentScheme<F>> {
    pub(crate) vk: VerifyingKey<F, CS>,
    pub(crate) l0: Polynomial<F, ExtendedLagrangeCoeff>,
    pub(crate) l_last: Polynomial<F, ExtendedLagrangeCoeff>,
    pub(crate) l_active_row: Polynomial<F, ExtendedLagrangeCoeff>,
    pub(crate) fixed_values: Vec<Polynomial<F, LagrangeCoeff>>,
    pub(crate) fixed_polys: Vec<Polynomial<F, Coeff>>,
    pub(crate) permutation: permutation::ProvingKey<F>,
    pub(crate) ev: Evaluator<F>,
}

/// Lean reusable proving-state setup artifact.
///
/// This persists the verification key, fixed values in Lagrange form, and the
/// permutation base data. Derived proving caches are reconstructed lazily when
/// promoted into a full [`ProvingKey`].
#[derive(Clone, Debug)]
pub struct BaseProvingKey<F: PrimeField, CS: PolynomialCommitmentScheme<F>> {
    pub(crate) vk: VerifyingKey<F, CS>,
    pub(crate) fixed_values: Vec<Polynomial<F, LagrangeCoeff>>,
    pub(crate) permutation: permutation::BaseProvingKey<F>,
}

/// Derived proving state required only during the final proof phase.
#[derive(Clone, Debug)]
pub struct FinalizingKey<F: PrimeField, CS: PolynomialCommitmentScheme<F>> {
    pub(crate) vk: VerifyingKey<F, CS>,
    pub(crate) l0: Polynomial<F, ExtendedLagrangeCoeff>,
    pub(crate) l_last: Polynomial<F, ExtendedLagrangeCoeff>,
    pub(crate) l_active_row: Polynomial<F, ExtendedLagrangeCoeff>,
    pub(crate) fixed_polys: Vec<Polynomial<F, Coeff>>,
    pub(crate) permutation: permutation::FinalizingKey<F>,
    pub(crate) ev: Evaluator<F>,
}

/// Derived proving state required only for `compute_h_poly(...)`.
#[derive(Debug)]
pub struct HPolyKey<'a, F: PrimeField, CS: PolynomialCommitmentScheme<F>> {
    pub(crate) vk: VerifyingKey<F, CS>,
    pub(crate) l0: Polynomial<F, ExtendedLagrangeCoeff>,
    pub(crate) l_last: Polynomial<F, ExtendedLagrangeCoeff>,
    pub(crate) l_active_row: Polynomial<F, ExtendedLagrangeCoeff>,
    pub(crate) fixed_coeffs: Vec<Option<Polynomial<F, Coeff>>>,
    pub(crate) custom_gate_fixed_columns: std::collections::BTreeSet<usize>,
    pub(crate) permutation_fixed_columns: std::collections::BTreeSet<usize>,
    pub(crate) lookup_fixed_columns: std::collections::BTreeSet<usize>,
    pub(crate) trash_fixed_columns: std::collections::BTreeSet<usize>,
    pub(crate) permutation: permutation::HPolyKey<'a, F>,
    pub(crate) ev: Evaluator<F>,
}

/// Derived proving state required only for transcript evaluations and opening
/// queries after `h_poly` has already been computed.
#[derive(Clone, Debug)]
pub struct OpeningKey<F: PrimeField, CS: PolynomialCommitmentScheme<F>> {
    pub(crate) vk: VerifyingKey<F, CS>,
    pub(crate) fixed_polys: Vec<Option<Polynomial<F, Coeff>>>,
    pub(crate) permutation: permutation::OpeningKey<F>,
}

impl<F: WithSmallOrderMulGroup<3>, CS: PolynomialCommitmentScheme<F>> ProvingKey<F, CS>
where
    F: FromUniformBytes<64>,
{
    /// Get the underlying [`VerifyingKey`].
    pub fn get_vk(&self) -> &VerifyingKey<F, CS> {
        &self.vk
    }

    /// Gets the total number of bytes in the serialization of `self`
    pub fn bytes_length(&self, format: SerdeFormat) -> usize {
        self.vk.bytes_length(format)
            + 12 // bytes used for encoding the length(u32) of "l0", "l_last" & "l_active_row" polys
            + polynomial_slice_byte_length(&self.fixed_values)
            + self.permutation.bytes_length()
    }
}

impl<F: WithSmallOrderMulGroup<3>, CS: PolynomialCommitmentScheme<F>> BaseProvingKey<F, CS>
where
    F: PrimeField + FromUniformBytes<64>,
{
    /// Get the underlying [`VerifyingKey`].
    pub fn get_vk(&self) -> &VerifyingKey<F, CS> {
        &self.vk
    }
}

impl<F: PrimeField, CS: PolynomialCommitmentScheme<F>> From<&ProvingKey<F, CS>>
    for FinalizingKey<F, CS>
{
    fn from(value: &ProvingKey<F, CS>) -> Self {
        Self {
            vk: value.vk.clone(),
            l0: value.l0.clone(),
            l_last: value.l_last.clone(),
            l_active_row: value.l_active_row.clone(),
            fixed_polys: value.fixed_polys.clone(),
            permutation: permutation::FinalizingKey {
                permutations: value.permutation.permutations.clone(),
                polys: value.permutation.polys.clone(),
            },
            ev: value.ev.clone(),
        }
    }
}

impl<'a, F: PrimeField, CS: PolynomialCommitmentScheme<F>> From<&'a ProvingKey<F, CS>>
    for HPolyKey<'a, F, CS>
{
    fn from(value: &'a ProvingKey<F, CS>) -> Self {
        Self {
            vk: value.vk.clone(),
            l0: value.l0.clone(),
            l_last: value.l_last.clone(),
            l_active_row: value.l_active_row.clone(),
            fixed_coeffs: value.fixed_polys.iter().cloned().map(Some).collect(),
            custom_gate_fixed_columns: (0..value.fixed_polys.len()).collect(),
            permutation_fixed_columns: (0..value.fixed_polys.len()).collect(),
            lookup_fixed_columns: (0..value.fixed_polys.len()).collect(),
            trash_fixed_columns: (0..value.fixed_polys.len()).collect(),
            permutation: permutation::HPolyKey {
                permutations: &value.permutation.permutations,
            },
            ev: value.ev.clone(),
        }
    }
}

impl<'a, F: PrimeField, CS: PolynomialCommitmentScheme<F>> From<&'a FinalizingKey<F, CS>>
    for HPolyKey<'a, F, CS>
{
    fn from(value: &'a FinalizingKey<F, CS>) -> Self {
        Self {
            vk: value.vk.clone(),
            l0: value.l0.clone(),
            l_last: value.l_last.clone(),
            l_active_row: value.l_active_row.clone(),
            fixed_coeffs: value.fixed_polys.iter().cloned().map(Some).collect(),
            custom_gate_fixed_columns: (0..value.fixed_polys.len()).collect(),
            permutation_fixed_columns: (0..value.fixed_polys.len()).collect(),
            lookup_fixed_columns: (0..value.fixed_polys.len()).collect(),
            trash_fixed_columns: (0..value.fixed_polys.len()).collect(),
            permutation: permutation::HPolyKey {
                permutations: &value.permutation.permutations,
            },
            ev: value.ev.clone(),
        }
    }
}

impl<F: PrimeField, CS: PolynomialCommitmentScheme<F>> From<&ProvingKey<F, CS>>
    for OpeningKey<F, CS>
{
    fn from(value: &ProvingKey<F, CS>) -> Self {
        Self {
            vk: value.vk.clone(),
            fixed_polys: value.fixed_polys.iter().cloned().map(Some).collect(),
            permutation: permutation::OpeningKey {
                polys: value.permutation.polys.clone(),
            },
        }
    }
}

impl<F: WithSmallOrderMulGroup<3>, CS: PolynomialCommitmentScheme<F>> BaseProvingKey<F, CS>
where
    F: PrimeField + FromUniformBytes<64> + SerdeObject,
{
    /// Writes a lean proving-state setup artifact to a buffer.
    pub fn write<W: io::Write>(&self, writer: &mut W, format: SerdeFormat) -> io::Result<()> {
        self.vk.write(writer, format)?;
        write_polynomial_slice(&self.fixed_values, writer)?;
        self.permutation.write(writer)?;
        Ok(())
    }

    /// Reads a lean proving-state setup artifact from a buffer.
    pub fn read<R: io::Read, ConcreteCircuit: Circuit<F>>(
        reader: &mut R,
        format: SerdeFormat,
        #[cfg(feature = "circuit-params")] params: ConcreteCircuit::Params,
    ) -> io::Result<Self> {
        let vk = VerifyingKey::<F, CS>::read::<R, ConcreteCircuit>(
            reader,
            format,
            #[cfg(feature = "circuit-params")]
            params,
        )?;
        let fixed_values = read_polynomial_vec(reader, format)?;
        let permutation = permutation::BaseProvingKey::read(reader, format)?;
        Ok(Self {
            vk,
            fixed_values,
            permutation,
        })
    }

    /// Writes a lean proving-state setup artifact to a vector of bytes.
    pub fn to_bytes(&self, format: SerdeFormat) -> Vec<u8> {
        let mut bytes = Vec::<u8>::new();
        Self::write(self, &mut bytes, format).expect("Writing to vector should not fail");
        bytes
    }

    /// Reads a lean proving-state setup artifact from a slice of bytes.
    pub fn from_bytes<ConcreteCircuit: Circuit<F>>(
        mut bytes: &[u8],
        format: SerdeFormat,
        #[cfg(feature = "circuit-params")] params: ConcreteCircuit::Params,
    ) -> io::Result<Self> {
        Self::read::<_, ConcreteCircuit>(
            &mut bytes,
            format,
            #[cfg(feature = "circuit-params")]
            params,
        )
    }
}

impl<F: WithSmallOrderMulGroup<3>, CS: PolynomialCommitmentScheme<F>> BaseProvingKey<F, CS>
where
    F: PrimeField + FromUniformBytes<64>,
{
    /// Promotes the lean setup artifact into a full proving key by rebuilding
    /// only the derived proving caches.
    pub fn finalize(self) -> ProvingKey<F, CS> {
        let [l0, l_last, l_active_row] = keygen::compute_lagrange_polys(&self.vk, &self.vk.cs);
        let fixed_polys: Vec<_> = self
            .fixed_values
            .iter()
            .map(|poly| self.vk.domain.lagrange_to_coeff(poly.clone()))
            .collect();
        let permutation = self
            .permutation
            .finalize(&self.vk.domain, &self.vk.cs.permutation);
        let ev = Evaluator::new(self.vk.cs());

        ProvingKey {
            vk: self.vk,
            l0,
            l_last,
            l_active_row,
            fixed_values: self.fixed_values,
            fixed_polys,
            permutation,
            ev,
        }
    }

    /// Promotes the lean setup artifact into only the derived proving state
    /// required during the final proof phase.
    pub fn finalize_for_finalise(self) -> FinalizingKey<F, CS> {
        let [l0, l_last, l_active_row] = keygen::compute_lagrange_polys(&self.vk, &self.vk.cs);
        let fixed_polys: Vec<_> = self
            .fixed_values
            .iter()
            .map(|poly| self.vk.domain.lagrange_to_coeff(poly.clone()))
            .collect();
        let permutation = self
            .permutation
            .finalize_for_finalise(&self.vk.domain, &self.vk.cs.permutation);
        let ev = Evaluator::new(self.vk.cs());

        FinalizingKey {
            vk: self.vk,
            l0,
            l_last,
            l_active_row,
            fixed_polys,
            permutation,
            ev,
        }
    }

    /// Promotes the lean setup artifact into only the state required to compute
    /// `h_poly`.
    pub fn finalize_for_h_poly(&self) -> HPolyKey<'_, F, CS> {
        let mut custom_gate_fixed_columns = std::collections::BTreeSet::new();
        let mut lookup_fixed_columns = std::collections::BTreeSet::new();
        let mut trash_fixed_columns = std::collections::BTreeSet::new();
        let mut permutation_fixed_columns = std::collections::BTreeSet::new();
        let mut used_advice_columns = std::collections::BTreeSet::new();
        let mut used_instance_columns = std::collections::BTreeSet::new();
        let ev = Evaluator::new(self.vk.cs());
        for evaluator in &ev.custom_gates {
            evaluator.collect_used_columns(
                &mut custom_gate_fixed_columns,
                &mut used_advice_columns,
                &mut used_instance_columns,
            );
        }
        for evaluator in &ev.lookups {
            evaluator.collect_used_columns(
                &mut lookup_fixed_columns,
                &mut used_advice_columns,
                &mut used_instance_columns,
            );
        }
        for lookup in &self.vk.cs.lookups {
            for expression in lookup
                .input_expressions()
                .iter()
                .chain(lookup.table_expressions().iter())
            {
                collect_fixed_columns_from_expression(expression, &mut lookup_fixed_columns);
            }
        }
        for evaluator in &ev.trashcans {
            evaluator.collect_used_columns(
                &mut trash_fixed_columns,
                &mut used_advice_columns,
                &mut used_instance_columns,
            );
        }
        for trash in &self.vk.cs.trashcans {
            collect_fixed_columns_from_expression(trash.selector(), &mut trash_fixed_columns);
            for expression in trash.constraint_expressions() {
                collect_fixed_columns_from_expression(expression, &mut trash_fixed_columns);
            }
        }
        for column in &self.vk.cs.permutation.columns {
            if let Any::Fixed = column.column_type() {
                permutation_fixed_columns.insert(column.index());
            }
        }
        let mut used_fixed_columns = custom_gate_fixed_columns.clone();
        used_fixed_columns.extend(lookup_fixed_columns.iter().copied());
        used_fixed_columns.extend(trash_fixed_columns.iter().copied());
        used_fixed_columns.extend(permutation_fixed_columns.iter().copied());

        let base_context = format!(
            "k={} domain_n={} used_fixed_columns={} used_advice_columns={} used_instance_columns={} permutation_columns={}",
            self.vk.domain.k(),
            self.vk.domain.n,
            used_fixed_columns.len(),
            used_advice_columns.len(),
            used_instance_columns.len(),
            self.vk.cs.permutation.columns.len()
        );

        let compute_lagrange_started_at = Instant::now();
        direct_logging::log_event(
            "finalize_for_h_poly",
            "compute_lagrange_polys",
            "start",
            &base_context,
        );
        let [l0, l_last, l_active_row] = keygen::compute_lagrange_polys(&self.vk, &self.vk.cs);
        direct_logging::log_elapsed(
            "finalize_for_h_poly",
            "compute_lagrange_polys",
            compute_lagrange_started_at,
            &base_context,
        );

        let sparse_fixed_started_at = Instant::now();
        direct_logging::log_event(
            "finalize_for_h_poly",
            "sparse_fixed_coeffs",
            "start",
            &base_context,
        );
        let fixed_coeffs: Vec<_> = self
            .fixed_values
            .iter()
            .enumerate()
            .map(|(column_index, value)| {
                used_fixed_columns
                    .contains(&column_index)
                    .then(|| self.vk.domain.lagrange_to_coeff(value.clone()))
            })
            .collect();
        let materialized_fixed_coeffs = fixed_coeffs.iter().filter(|value| value.is_some()).count();
        direct_logging::log_elapsed(
            "finalize_for_h_poly",
            "sparse_fixed_coeffs",
            sparse_fixed_started_at,
            &format!(
                "{base_context} materialized_fixed_coeffs={materialized_fixed_coeffs}"
            ),
        );

        let permutation_h_key_started_at = Instant::now();
        direct_logging::log_event(
            "finalize_for_h_poly",
            "permutation_h_key",
            "start",
            &format!("{base_context} permutation_sets={}", self.vk.cs.permutation.columns.len()),
        );
        let permutation = self
            .permutation
            .finalize_for_h_poly(&self.vk.domain, &self.vk.cs.permutation);
        direct_logging::log_elapsed(
            "finalize_for_h_poly",
            "permutation_h_key",
            permutation_h_key_started_at,
            &base_context,
        );

        HPolyKey {
            vk: self.vk.clone(),
            l0,
            l_last,
            l_active_row,
            fixed_coeffs,
            custom_gate_fixed_columns,
            permutation_fixed_columns,
            lookup_fixed_columns,
            trash_fixed_columns,
            permutation,
            ev,
        }
    }

    /// Promotes the lean setup artifact into only the state required for
    /// transcript evaluations and opening queries after `h_poly`.
    pub fn finalize_for_openings(self) -> OpeningKey<F, CS> {
        let mut used_fixed_columns = std::collections::BTreeSet::new();
        for &(column, _) in self.vk.cs.fixed_queries.iter() {
            used_fixed_columns.insert(column.index());
        }
        let fixed_polys = self
            .fixed_values
            .iter()
            .enumerate()
            .map(|(column_index, poly)| {
                used_fixed_columns
                    .contains(&column_index)
                    .then(|| self.vk.domain.lagrange_to_coeff(poly.clone()))
            })
            .collect();
        let permutation = self
            .permutation
            .finalize_for_openings(&self.vk.domain, &self.vk.cs.permutation);

        OpeningKey {
            vk: self.vk,
            fixed_polys,
            permutation,
        }
    }
}

impl<F: WithSmallOrderMulGroup<3>, CS: PolynomialCommitmentScheme<F>> ProvingKey<F, CS>
where
    F: PrimeField + FromUniformBytes<64> + SerdeObject,
{
    /// Writes a proving key to a buffer.
    ///
    /// Writes a curve element according to `format`:
    /// - `Processed`: Writes a compressed curve element with coordinates in
    ///   standard form. Writes a field element in standard form, with
    ///   endianness specified by the `PrimeField` implementation.
    /// - Otherwise: Writes an uncompressed curve element with coordinates in
    ///   Montgomery form Writes a field element into raw bytes in its internal
    ///   Montgomery representation, WITHOUT performing the expensive Montgomery
    ///   reduction. Does so by first writing the verifying key and then
    ///   serializing the rest of the data (in the form of field polynomials)
    pub fn write<W: io::Write>(&self, writer: &mut W, format: SerdeFormat) -> io::Result<()> {
        self.vk.write(writer, format)?;
        write_polynomial_slice(&self.fixed_values, writer)?;
        self.permutation.write(writer)?;
        Ok(())
    }

    /// Reads a proving key from a buffer.
    /// Does so by reading verification key first, and then deserializing the
    /// rest of the file into the remaining proving key data.
    ///
    /// Reads a curve element from the buffer and parses it according to the
    /// `format`:
    /// - `Processed`: Reads a compressed curve element and decompresses it.
    ///   Reads a field element in standard form, with endianness specified by
    ///   the `PrimeField` implementation, and checks that the element is less
    ///   than the modulus.
    /// - `RawBytes`: Reads an uncompressed curve element with coordinates in
    ///   Montgomery form. Checks that field elements are less than modulus, and
    ///   then checks that the point is on the curve.
    /// - `RawBytesUnchecked`: Reads an uncompressed curve element with
    ///   coordinates in Montgomery form; does not perform any checks
    pub fn read<R: io::Read, ConcreteCircuit: Circuit<F>>(
        reader: &mut R,
        format: SerdeFormat,
        #[cfg(feature = "circuit-params")] params: ConcreteCircuit::Params,
    ) -> io::Result<Self> {
        let vk = VerifyingKey::<F, CS>::read::<R, ConcreteCircuit>(
            reader,
            format,
            #[cfg(feature = "circuit-params")]
            params,
        )?;
        let [l0, l_last, l_active_row] = compute_lagrange_polys(&vk, &vk.cs);
        let fixed_values = read_polynomial_vec(reader, format)?;
        let fixed_polys: Vec<_> = fixed_values
            .iter()
            .map(|poly| vk.domain.lagrange_to_coeff(poly.clone()))
            .collect();
        let permutation =
            permutation::ProvingKey::read(reader, format, &vk.domain, &vk.cs.permutation)?;
        let ev = Evaluator::new(vk.cs());
        Ok(Self {
            vk,
            l0,
            l_last,
            l_active_row,
            fixed_values,
            fixed_polys,
            permutation,
            ev,
        })
    }

    /// Writes a proving key to a vector of bytes using [`Self::write`].
    pub fn to_bytes(&self, format: SerdeFormat) -> Vec<u8> {
        let mut bytes = Vec::<u8>::with_capacity(self.bytes_length(format));
        Self::write(self, &mut bytes, format).expect("Writing to vector should not fail");
        bytes
    }

    /// Reads a proving key from a slice of bytes using [`Self::read`].
    pub fn from_bytes<ConcreteCircuit: Circuit<F>>(
        mut bytes: &[u8],
        format: SerdeFormat,
        #[cfg(feature = "circuit-params")] params: ConcreteCircuit::Params,
    ) -> io::Result<Self> {
        Self::read::<_, ConcreteCircuit>(
            &mut bytes,
            format,
            #[cfg(feature = "circuit-params")]
            params,
        )
    }
}

impl<F: PrimeField, CS: PolynomialCommitmentScheme<F>> VerifyingKey<F, CS> {
    /// Get the underlying [`EvaluationDomain`].
    pub fn get_domain(&self) -> &EvaluationDomain<F> {
        &self.domain
    }
}
