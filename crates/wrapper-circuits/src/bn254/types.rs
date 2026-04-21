use midnight_circuits::{
  ecc::foreign::ecc_chip::{AssignedForeignPoint, ForeignEccChip},
  field::{
    decomposition::chip::P2RDecompositionChip,
    foreign::{
      field_chip::{AssignedField, FieldChip},
      params::MultiEmulationParams,
    },
    native::{native_chip::NativeChip, native_gadget::NativeGadget},
  },
  instructions::{
    ArithInstructions, AssertionInstructions, AssignmentInstructions, EccInstructions,
  },
  midnight_proofs::{
    circuit::{Layouter, Value},
    plonk::{ConstraintSystem, Error},
  },
  testing_utils::FromScratch,
};
use midnight_curves::bn256::{Fq, Fr, G1};

/// Native Halo2 field used by the current Midnight-backed circuits.
pub type NativeField = Fr;
/// BN254 base field emulated inside the Halo2 circuit.
pub type ForeignField = Fq;
/// BN254 G1 group used by the ECC chip.
pub type ForeignCurve = G1;
/// Assigned BN254 foreign-field element backed by Midnight's `FieldChip`.
pub type AssignedFp = AssignedField<NativeField, ForeignField, MultiEmulationParams>;
/// Assigned BN254 G1 point backed by Midnight's `ForeignEccChip`.
pub type AssignedG1 = AssignedForeignPoint<NativeField, ForeignCurve, MultiEmulationParams>;
type NativeBridge =
  NativeGadget<NativeField, P2RDecompositionChip<NativeField>, NativeChip<NativeField>>;

/// Midnight chip for BN254 foreign-field arithmetic.
pub type MidnightFieldChip =
  FieldChip<NativeField, ForeignField, MultiEmulationParams, NativeBridge>;
/// Midnight chip for BN254 foreign G1 arithmetic.
pub type MidnightEccChip =
  ForeignEccChip<NativeField, ForeignCurve, MultiEmulationParams, NativeBridge, NativeBridge>;
/// Public wrapper over the Midnight BN254 foreign-field chip.
pub type Bn254FpChip = Bn254FieldChip;
/// Public wrapper over the Midnight BN254 G1 chip.
pub type Bn254EccChip = Bn254G1Chip;

/// Shared configuration for the Midnight-backed BN254 foreign-field chip.
#[derive(Clone, Debug)]
pub struct Bn254FieldConfig(pub(crate) <MidnightFieldChip as FromScratch<NativeField>>::Config);

impl Bn254FieldConfig {
  /// Configures the foreign-field chip on a fresh constraint system.
  #[must_use]
  pub fn configure(meta: &mut ConstraintSystem<NativeField>) -> Self {
    let instance_columns = [meta.instance_column(), meta.instance_column()];

    Self(MidnightFieldChip::configure_from_scratch(meta, &instance_columns))
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

    Self(MidnightEccChip::configure_from_scratch(meta, &instance_columns))
  }
}

/// Thin adapter over Midnight's BN254 foreign-field chip.
#[derive(Clone, Debug)]
pub struct Bn254FieldChip {
  field_chip: MidnightFieldChip,
}

impl Bn254FieldChip {
  /// Instantiates the chip from an existing configuration.
  #[must_use]
  pub fn new(config: &Bn254FieldConfig) -> Self {
    Self { field_chip: MidnightFieldChip::new_from_scratch(&config.0) }
  }

  /// Loads any required tables into the layouter.
  pub fn load(&self, layouter: &mut impl Layouter<NativeField>) -> Result<(), Error> {
    self.field_chip.load_from_scratch(layouter)
  }

  /// Assigns a BN254 base-field witness.
  pub fn assign(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    value: Value<ForeignField>,
  ) -> Result<AssignedFp, Error> {
    self.field_chip.assign(layouter, value)
  }

  /// Adds two BN254 base-field values inside the circuit.
  pub fn add(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    left: &AssignedFp,
    right: &AssignedFp,
  ) -> Result<AssignedFp, Error> {
    self.field_chip.add(layouter, left, right)
  }

  /// Multiplies two BN254 base-field values inside the circuit.
  pub fn mul(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    left: &AssignedFp,
    right: &AssignedFp,
  ) -> Result<AssignedFp, Error> {
    self.field_chip.mul(layouter, left, right, None)
  }

  /// Squares a BN254 base-field value inside the circuit.
  pub fn square(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    value: &AssignedFp,
  ) -> Result<AssignedFp, Error> {
    self.field_chip.mul(layouter, value, value, None)
  }

  /// Subtracts two BN254 base-field values inside the circuit.
  pub fn sub(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    left: &AssignedFp,
    right: &AssignedFp,
  ) -> Result<AssignedFp, Error> {
    self.field_chip.sub(layouter, left, right)
  }

  /// Negates a BN254 base-field value inside the circuit.
  pub fn neg(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    value: &AssignedFp,
  ) -> Result<AssignedFp, Error> {
    self.field_chip.neg(layouter, value)
  }

  /// Asserts that the assigned value matches the expected constant.
  pub fn assert_equal_to_fixed(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    value: &AssignedFp,
    expected: ForeignField,
  ) -> Result<(), Error> {
    self.field_chip.assert_equal_to_fixed(layouter, value, expected)
  }

  /// Asserts equality between two assigned BN254 base-field values.
  pub fn assert_equal(
    &self,
    layouter: &mut impl Layouter<NativeField>,
    left: &AssignedFp,
    right: &AssignedFp,
  ) -> Result<(), Error> {
    self.field_chip.assert_equal(layouter, left, right)
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
