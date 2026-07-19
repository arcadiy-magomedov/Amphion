//! Semantic identity and derivation records.

use amphion_foundation::{OperationId, SemanticId};
use serde::{Deserialize, Serialize};

/// Stable semantic role assigned by a feature or import operation.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ProvenanceRole(String);

impl ProvenanceRole {
    /// Creates a role from a non-empty stable token.
    ///
    /// # Errors
    ///
    /// Returns [`ProvenanceRoleError`] when the token is empty or contains
    /// unsupported characters.
    pub fn try_new(value: impl Into<String>) -> Result<Self, ProvenanceRoleError> {
        let value = value.into();
        if value.is_empty()
            || !value
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        {
            return Err(ProvenanceRoleError);
        }
        Ok(Self(value))
    }

    /// Returns the stable role token.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Error returned for an invalid provenance role.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProvenanceRoleError;

impl core::fmt::Display for ProvenanceRoleError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("provenance roles must be non-empty stable ASCII tokens")
    }
}

impl core::error::Error for ProvenanceRoleError {}

impl TryFrom<String> for ProvenanceRole {
    type Error = ProvenanceRoleError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl From<ProvenanceRole> for String {
    fn from(value: ProvenanceRole) -> Self {
        value.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
struct ProvenanceRepr {
    semantic_id: SemanticId,
    created_by: Option<OperationId>,
    derived_from: Vec<SemanticId>,
    role: ProvenanceRole,
}

/// Deterministic semantic identity and derivation metadata.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(from = "ProvenanceRepr", into = "ProvenanceRepr")]
pub struct Provenance {
    semantic_id: SemanticId,
    created_by: Option<OperationId>,
    derived_from: Vec<SemanticId>,
    role: ProvenanceRole,
}

impl From<ProvenanceRepr> for Provenance {
    fn from(value: ProvenanceRepr) -> Self {
        Self::new(
            value.semantic_id,
            value.created_by,
            value.derived_from,
            value.role,
        )
    }
}

impl From<Provenance> for ProvenanceRepr {
    fn from(value: Provenance) -> Self {
        Self {
            semantic_id: value.semantic_id,
            created_by: value.created_by,
            derived_from: value.derived_from,
            role: value.role,
        }
    }
}

impl Provenance {
    /// Creates provenance. Source semantic IDs are sorted and deduplicated.
    #[must_use]
    pub fn new(
        semantic_id: SemanticId,
        created_by: Option<OperationId>,
        mut derived_from: Vec<SemanticId>,
        role: ProvenanceRole,
    ) -> Self {
        derived_from.sort_unstable();
        derived_from.dedup();
        Self {
            semantic_id,
            created_by,
            derived_from,
            role,
        }
    }

    /// Returns the stable semantic identity.
    #[must_use]
    pub const fn semantic_id(&self) -> SemanticId {
        self.semantic_id
    }

    /// Returns the operation that created the entity, if it was feature-made.
    #[must_use]
    pub const fn created_by(&self) -> Option<OperationId> {
        self.created_by
    }

    /// Returns sorted source semantic identities.
    #[must_use]
    pub fn derived_from(&self) -> &[SemanticId] {
        &self.derived_from
    }

    /// Returns the stable semantic role.
    #[must_use]
    pub const fn role(&self) -> &ProvenanceRole {
        &self.role
    }
}
