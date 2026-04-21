//! Layout-oriented placeholder types for CLI reporting and future circuit work.

use serde::Serialize;

/// A node kind in the planned wrapper layout tree.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum LayoutComponentKind {
  /// Root wrapper circuit node.
  WrapperRoot,
  /// BN254 foreign-field arithmetic layer.
  ForeignFieldLayer,
  /// BN254 G1 abstraction layer.
  G1Layer,
  /// Metadata normalization stage.
  ArtifactIngress,
  /// Verification-key placeholder stage.
  VerificationKeyEnvelope,
  /// Outer verifier placeholder stage.
  OuterVerifierShell,
}

/// A node in a human-readable layout tree.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LayoutNode {
  /// Node identifier.
  pub id: String,
  /// Human-readable title.
  pub title: String,
  /// Kind of layout component.
  pub kind: LayoutComponentKind,
}

/// A simple layout descriptor used for scaffolding and reporting.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct LayoutDescriptor {
  /// Display name for the layout.
  pub name: String,
  /// Top-level layout nodes.
  pub nodes: Vec<LayoutNode>,
}

impl LayoutDescriptor {
  /// Returns the default scaffold layout description.
  #[must_use]
  pub fn scaffold() -> Self {
    Self {
      name: "wrapper-scaffold".to_owned(),
      nodes: vec![
        LayoutNode {
          id: "wrapper-root".to_owned(),
          title: "Wrapper root".to_owned(),
          kind: LayoutComponentKind::WrapperRoot,
        },
        LayoutNode {
          id: "artifact-ingress".to_owned(),
          title: "Artifact ingress placeholder".to_owned(),
          kind: LayoutComponentKind::ArtifactIngress,
        },
        LayoutNode {
          id: "bn254-foreign-field".to_owned(),
          title: "BN254 foreign-field layer".to_owned(),
          kind: LayoutComponentKind::ForeignFieldLayer,
        },
        LayoutNode {
          id: "bn254-g1-layer".to_owned(),
          title: "BN254 G1 abstraction layer".to_owned(),
          kind: LayoutComponentKind::G1Layer,
        },
        LayoutNode {
          id: "vk-envelope".to_owned(),
          title: "Verification-key envelope placeholder".to_owned(),
          kind: LayoutComponentKind::VerificationKeyEnvelope,
        },
        LayoutNode {
          id: "outer-verifier-shell".to_owned(),
          title: "Outer verifier shell placeholder".to_owned(),
          kind: LayoutComponentKind::OuterVerifierShell,
        },
      ],
    }
  }
}
