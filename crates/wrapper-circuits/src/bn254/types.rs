use ff::PrimeField;
use midnight_circuits::{
  ecc::foreign::ecc_chip::{AssignedForeignPoint, ForeignEccChip},
  field::{
    decomposition::chip::P2RDecompositionChip,
    foreign::{
      field_chip::{AssignedField, FieldChip},
      params::{FieldEmulationParams, MultiEmulationParams},
    },
    native::{native_chip::NativeChip, native_gadget::NativeGadget},
  },
  instructions::{
    ArithInstructions, AssertionInstructions, AssignmentInstructions, BinaryInstructions,
    EccInstructions, EqualityInstructions, ZeroInstructions,
  },
  midnight_proofs::{
    circuit::{Layouter, Value},
    plonk::{Column, ConstraintSystem, Error, Instance},
  },
  testing_utils::FromScratch,
};
use midnight_curves::bn256::{Fq, Fr, G1};

/// BN254-backed native Halo2 field used by the current compatibility lane.
pub type NativeField = Fr;
/// BLS12-381-backed native Halo2 field used by the future outer-host lane.
pub type Bls12HostField = midnight_curves::bls12_381::Fq;
/// BN254 base field emulated inside the Halo2 circuit.
pub type ForeignField = Fq;
/// BN254 G1 group used by the ECC chip.
pub type ForeignCurve = G1;

/// Assigned BN254 foreign-field element backed by Midnight's `FieldChip`.
pub type AssignedFp<FHost = NativeField> = AssignedField<FHost, ForeignField, MultiEmulationParams>;
/// Assigned native boolean bit backed by Midnight's native chip.
pub type AssignedBool<FHost = NativeField> = midnight_circuits::types::AssignedBit<FHost>;
/// Assigned BN254 G1 point backed by Midnight's `ForeignEccChip`.
pub type AssignedG1 = AssignedForeignPoint<NativeField, ForeignCurve, MultiEmulationParams>;
type NativeBridge<FHost> = NativeGadget<FHost, P2RDecompositionChip<FHost>, NativeChip<FHost>>;

/// Midnight chip for BN254 foreign-field arithmetic.
pub type MidnightFieldChip<FHost = NativeField> =
  FieldChip<FHost, ForeignField, MultiEmulationParams, NativeBridge<FHost>>;
/// Midnight native gadget used for boolean operations in narrow pairing checks.
pub type MidnightBoolChip<FHost = NativeField> = NativeBridge<FHost>;
/// Midnight chip for BN254 foreign G1 arithmetic on the current BN254-hosted lane.
pub type MidnightEccChip = ForeignEccChip<
  NativeField,
  ForeignCurve,
  MultiEmulationParams,
  NativeBridge<NativeField>,
  NativeBridge<NativeField>,
>;
/// Public wrapper over the Midnight BN254 foreign-field chip.
pub type Bn254FpChip<FHost = NativeField> = Bn254FieldChip<FHost>;
/// Public wrapper over the Midnight native boolean gadget used by pairing checks.
pub type Bn254BitChip<FHost = NativeField> = Bn254BoolChip<FHost>;
/// Public wrapper over the Midnight BN254 G1 chip.
pub type Bn254EccChip = Bn254G1Chip;

/// Shared configuration for the Midnight-backed BN254 foreign-field chip.
#[derive(Clone, Debug)]
pub struct Bn254FieldConfig<FHost = NativeField>(
  pub(crate) <MidnightFieldChip<FHost> as FromScratch<FHost>>::Config,
)
where
  FHost: PrimeField,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>;

impl<FHost> Bn254FieldConfig<FHost>
where
  FHost: PrimeField,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  /// Configures the foreign-field chip on a fresh constraint system.
  #[must_use]
  pub fn configure(meta: &mut ConstraintSystem<FHost>) -> Self {
    let instance_columns = [meta.instance_column(), meta.instance_column()];

    Self::configure_with_instances(meta, &instance_columns)
  }

  /// Configures the foreign-field chip using caller-provided instance columns.
  #[must_use]
  pub fn configure_with_instances(
    meta: &mut ConstraintSystem<FHost>,
    instance_columns: &[Column<Instance>; 2],
  ) -> Self {
    Self(MidnightFieldChip::configure_from_scratch(meta, instance_columns))
  }
}

/// Shared configuration for the Midnight-backed BN254 G1 chip.
#[derive(Clone, Debug)]
pub struct Bn254G1Config(pub(crate) <MidnightEccChip as FromScratch<NativeField>>::Config);

impl Bn254G1Config {
  /// Configures the foreign-ECC chip on a fresh constraint system.
  #[must_use]
  pub fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self {
    let instance_columns = [meta.instance_column(), meta.instance_column()];

    Self::configure_with_instances(meta, &instance_columns)
  }

  /// Configures the foreign-ECC chip using caller-provided instance columns.
  #[must_use]
  pub fn configure_with_instances(
    meta: &mut ConstraintSystem<NativeField>,
    instance_columns: &[Column<Instance>; 2],
  ) -> Self {
    Self(MidnightEccChip::configure_from_scratch(meta, instance_columns))
  }
}

/// Shared configuration for the Midnight-backed native boolean gadget.
#[derive(Clone, Debug)]
pub struct Bn254BoolConfig<FHost = NativeField>(
  pub(crate) <MidnightBoolChip<FHost> as FromScratch<FHost>>::Config,
)
where
  FHost: PrimeField;

impl<FHost> Bn254BoolConfig<FHost>
where
  FHost: PrimeField,
{
  /// Configures the native boolean gadget on a fresh constraint system.
  #[must_use]
  pub fn configure(meta: &mut ConstraintSystem<FHost>) -> Self {
    let instance_columns = [meta.instance_column(), meta.instance_column()];

    Self::configure_with_instances(meta, &instance_columns)
  }

  /// Configures the native boolean gadget using caller-provided instance columns.
  #[must_use]
  pub fn configure_with_instances(
    meta: &mut ConstraintSystem<FHost>,
    instance_columns: &[Column<Instance>; 2],
  ) -> Self {
    Self(MidnightBoolChip::configure_from_scratch(meta, instance_columns))
  }
}

/// Thin adapter over Midnight's BN254 foreign-field chip.
#[derive(Clone, Debug)]
pub struct Bn254FieldChip<FHost = NativeField>
where
  FHost: PrimeField,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  field_chip: MidnightFieldChip<FHost>,
}

impl<FHost> Bn254FieldChip<FHost>
where
  FHost: PrimeField,
  MultiEmulationParams: FieldEmulationParams<FHost, ForeignField>,
{
  /// Instantiates the chip from an existing configuration.
  #[must_use]
  pub fn new(config: &Bn254FieldConfig<FHost>) -> Self {
    Self { field_chip: MidnightFieldChip::new_from_scratch(&config.0) }
  }

  /// Loads any required tables into the layouter.
  pub fn load(&self, layouter: &mut impl Layouter<FHost>) -> Result<(), Error> {
    self.field_chip.load_from_scratch(layouter)
  }

  /// Assigns a BN254 base-field witness.
  pub fn assign(
    &self,
    layouter: &mut impl Layouter<FHost>,
    value: Value<ForeignField>,
  ) -> Result<AssignedFp<FHost>, Error> {
    self.field_chip.assign(layouter, value)
  }

  /// Adds two BN254 base-field values inside the circuit.
  pub fn add(
    &self,
    layouter: &mut impl Layouter<FHost>,
    left: &AssignedFp<FHost>,
    right: &AssignedFp<FHost>,
  ) -> Result<AssignedFp<FHost>, Error> {
    self.field_chip.add(layouter, left, right)
  }

  /// Multiplies two BN254 base-field values inside the circuit.
  pub fn mul(
    &self,
    layouter: &mut impl Layouter<FHost>,
    left: &AssignedFp<FHost>,
    right: &AssignedFp<FHost>,
  ) -> Result<AssignedFp<FHost>, Error> {
    self.field_chip.mul(layouter, left, right, None)
  }

  /// Squares a BN254 base-field value inside the circuit.
  pub fn square(
    &self,
    layouter: &mut impl Layouter<FHost>,
    value: &AssignedFp<FHost>,
  ) -> Result<AssignedFp<FHost>, Error> {
    self.field_chip.mul(layouter, value, value, None)
  }

  /// Subtracts two BN254 base-field values inside the circuit.
  pub fn sub(
    &self,
    layouter: &mut impl Layouter<FHost>,
    left: &AssignedFp<FHost>,
    right: &AssignedFp<FHost>,
  ) -> Result<AssignedFp<FHost>, Error> {
    self.field_chip.sub(layouter, left, right)
  }

  /// Negates a BN254 base-field value inside the circuit.
  pub fn neg(
    &self,
    layouter: &mut impl Layouter<FHost>,
    value: &AssignedFp<FHost>,
  ) -> Result<AssignedFp<FHost>, Error> {
    self.field_chip.neg(layouter, value)
  }

  /// Asserts that the assigned value matches the expected constant.
  pub fn assert_equal_to_fixed(
    &self,
    layouter: &mut impl Layouter<FHost>,
    value: &AssignedFp<FHost>,
    expected: ForeignField,
  ) -> Result<(), Error> {
    self.field_chip.assert_equal_to_fixed(layouter, value, expected)
  }

  /// Asserts equality between two assigned BN254 base-field values.
  pub fn assert_equal(
    &self,
    layouter: &mut impl Layouter<FHost>,
    left: &AssignedFp<FHost>,
    right: &AssignedFp<FHost>,
  ) -> Result<(), Error> {
    self.field_chip.assert_equal(layouter, left, right)
  }

  /// Returns a native boolean indicating whether an assigned BN254 value equals a fixed constant.
  pub fn is_equal_to_fixed(
    &self,
    layouter: &mut impl Layouter<FHost>,
    value: &AssignedFp<FHost>,
    expected: ForeignField,
  ) -> Result<AssignedBool<FHost>, Error> {
    self.field_chip.is_equal_to_fixed(layouter, value, expected)
  }
}

/// Thin adapter over Midnight's native boolean gadget for pairing checks.
#[derive(Clone, Debug)]
pub struct Bn254BoolChip<FHost = NativeField>
where
  FHost: PrimeField,
{
  native_gadget: MidnightBoolChip<FHost>,
}

impl<FHost> Bn254BoolChip<FHost>
where
  FHost: PrimeField,
{
  /// Instantiates the native boolean gadget from an existing configuration.
  #[must_use]
  pub fn new(config: &Bn254BoolConfig<FHost>) -> Self {
    Self { native_gadget: MidnightBoolChip::new_from_scratch(&config.0) }
  }

  /// Loads any required tables into the layouter.
  pub fn load(&self, layouter: &mut impl Layouter<FHost>) -> Result<(), Error> {
    self.native_gadget.load_from_scratch(layouter)
  }

  /// Conjoins a list of native booleans.
  pub fn and(
    &self,
    layouter: &mut impl Layouter<FHost>,
    bits: &[AssignedBool<FHost>],
  ) -> Result<AssignedBool<FHost>, Error> {
    self.native_gadget.and(layouter, bits)
  }

  /// Asserts that a native boolean equals a fixed host-side boolean.
  pub fn assert_equal_to_fixed(
    &self,
    layouter: &mut impl Layouter<FHost>,
    value: &AssignedBool<FHost>,
    expected: bool,
  ) -> Result<(), Error> {
    self.native_gadget.assert_equal_to_fixed(layouter, value, expected)
  }
}

/// Thin adapter over Midnight's BN254 foreign-ECC chip.
#[derive(Clone, Debug)]
pub struct Bn254G1Chip {
  ecc_chip: MidnightEccChip,
}

impl Bn254G1Chip {
  /// Instantiates the chip from an existing configuration.
  #[must_use]
  pub fn new(config: &Bn254G1Config) -> Self {
    Self { ecc_chip: MidnightEccChip::new_from_scratch(&config.0) }
  }

  /// Loads any required tables into the layouter.
  pub fn load(&self, layouter: &mut impl Layouter<NativeField>) -> Result<(), Error> {
    self.ecc_chip.load_from_scratch(layouter)
  }

  /// Assigns a BN254 G1 witness.
  pub fn assign(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    point: Value<ForeignCurve>,
  ) -> Result<AssignedG1, Error> {
    self.ecc_chip.assign(layouter, point)
  }

  /// Assigns a BN254 base-field witness through the ECC chip's base field.
  pub fn assign_coordinate(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    value: Value<ForeignField>,
  ) -> Result<AssignedFp, Error> {
    self.ecc_chip.base_field_chip().assign(layouter, value)
  }

  /// Constructs a point from assigned coordinates and enforces the on-curve relation.
  pub fn point_from_coordinates(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    x: &AssignedFp,
    y: &AssignedFp,
  ) -> Result<AssignedG1, Error> {
    self.ecc_chip.point_from_coordinates(layouter, x, y)
  }

  /// Adds two BN254 G1 points inside the circuit.
  pub fn add(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    left: &AssignedG1,
    right: &AssignedG1,
  ) -> Result<AssignedG1, Error> {
    self.ecc_chip.add(layouter, left, right)
  }

  /// Negates a BN254 G1 point inside the circuit.
  pub fn negate(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    point: &AssignedG1,
  ) -> Result<AssignedG1, Error> {
    self.ecc_chip.negate(layouter, point)
  }

  /// Multiplies a BN254 G1 point by a fixed host-field scalar.
  pub fn mul_by_scalar_constant(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    scalar: NativeField,
    point: &AssignedG1,
  ) -> Result<AssignedG1, Error> {
    self.ecc_chip.mul_by_constant(layouter, scalar, point)
  }

  /// Returns the assigned affine x-coordinate of a BN254 G1 point.
  #[must_use]
  pub fn x_coordinate(&self, point: &AssignedG1) -> AssignedFp {
    self.ecc_chip.x_coordinate(point)
  }

  /// Returns the assigned affine y-coordinate of a BN254 G1 point.
  #[must_use]
  pub fn y_coordinate(&self, point: &AssignedG1) -> AssignedFp {
    self.ecc_chip.y_coordinate(point)
  }

  /// Returns a native boolean indicating whether the point is the identity.
  pub fn is_identity(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    point: &AssignedG1,
  ) -> Result<AssignedBool, Error> {
    self.ecc_chip.is_zero(layouter, point)
  }

  /// Asserts that the assigned point matches the expected constant point.
  pub fn assert_equal_to_fixed(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    point: &AssignedG1,
    expected: ForeignCurve,
  ) -> Result<(), Error> {
    self.ecc_chip.assert_equal_to_fixed(layouter, point, expected)
  }
}
