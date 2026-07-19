//! Stable identities supplied by document and operation layers.

use serde::{Deserialize, Serialize};

/// A stable semantic identity intended to survive deterministic recomputation.
///
/// The kernel never generates random semantic IDs. A caller supplies them or
/// derives them deterministically from document and feature identities.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct SemanticId([u8; 16]);

impl SemanticId {
    /// Creates an identity from its canonical 16-byte representation.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Returns the canonical byte representation.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 16] {
        self.0
    }
}

/// A stable identity for the operation that created or transformed geometry.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct OperationId([u8; 16]);

impl OperationId {
    /// Creates an operation identity from its canonical 16-byte representation.
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 16]) -> Self {
        Self(bytes)
    }

    /// Returns the canonical byte representation.
    #[must_use]
    pub const fn into_bytes(self) -> [u8; 16] {
        self.0
    }
}
