//! Versions for deterministic serialized schemas.

use serde::{Deserialize, Serialize};

/// A major/minor version attached to every persisted Amphion schema.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct SchemaVersion {
    major: u16,
    minor: u16,
}

impl SchemaVersion {
    /// Creates a schema version.
    #[must_use]
    pub const fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }

    /// Returns the compatibility-breaking version component.
    #[must_use]
    pub const fn major(self) -> u16 {
        self.major
    }

    /// Returns the backward-compatible version component.
    #[must_use]
    pub const fn minor(self) -> u16 {
        self.minor
    }
}
