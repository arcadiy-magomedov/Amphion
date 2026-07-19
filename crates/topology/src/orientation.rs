//! Normative B-Rep orientation semantics.

use serde::{Deserialize, Serialize};

/// Orientation relative to canonical underlying geometry.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Orientation {
    /// Uses the canonical direction or normal.
    Forward,
    /// Reverses the canonical direction or normal.
    Reversed,
}

impl Orientation {
    /// Reverses this orientation.
    #[must_use]
    pub const fn reversed(self) -> Self {
        match self {
            Self::Forward => Self::Reversed,
            Self::Reversed => Self::Forward,
        }
    }

    /// Composes two orientation changes.
    #[must_use]
    pub const fn compose(self, other: Self) -> Self {
        match (self, other) {
            (Self::Forward, Self::Forward) | (Self::Reversed, Self::Reversed) => Self::Forward,
            (Self::Forward, Self::Reversed) | (Self::Reversed, Self::Forward) => Self::Reversed,
        }
    }
}

/// Whether a shell is topologically open or closed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ShellKind {
    /// A shell with boundary edges.
    Open,
    /// A shell in which every manifold edge has two oriented uses.
    Closed,
}

/// Role of a loop on an oriented face.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum LoopKind {
    /// The outer boundary, counter-clockwise when viewed along the oriented
    /// face normal.
    Outer,
    /// A hole boundary, clockwise when viewed along the oriented face normal.
    Inner,
}
