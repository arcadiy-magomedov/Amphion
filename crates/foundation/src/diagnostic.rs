//! Stable machine-readable diagnostics.

use core::error::Error;
use core::fmt;

use serde::{Deserialize, Serialize};

use crate::SemanticId;

/// Diagnostic severity independent of presentation.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum Severity {
    /// Informational context that does not invalidate a result.
    Info,
    /// A recoverable condition that deserves user attention.
    Warning,
    /// A condition that makes the requested result invalid.
    Error,
}

/// Error returned for a malformed stable diagnostic code.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiagnosticCodeError;

impl fmt::Display for DiagnosticCodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "diagnostic codes must use non-empty ASCII uppercase segments separated by dots",
        )
    }
}

impl Error for DiagnosticCodeError {}

/// A stable diagnostic identifier such as `TOPOLOGY.LOOP.NOT_CLOSED`.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct DiagnosticCode(String);

impl DiagnosticCode {
    /// Validates and creates a stable diagnostic code.
    ///
    /// # Errors
    ///
    /// Returns [`DiagnosticCodeError`] when the value is empty or contains
    /// anything other than uppercase ASCII segments separated by dots.
    pub fn try_new(value: impl Into<String>) -> Result<Self, DiagnosticCodeError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.split('.').all(|segment| {
                !segment.is_empty()
                    && segment.bytes().all(|byte| {
                        byte.is_ascii_uppercase() || byte.is_ascii_digit() || byte == b'_'
                    })
            });
        if valid {
            Ok(Self(value))
        } else {
            Err(DiagnosticCodeError)
        }
    }

    /// Returns the stable code.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for DiagnosticCode {
    type Error = DiagnosticCodeError;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::try_new(value)
    }
}

impl From<DiagnosticCode> for String {
    fn from(value: DiagnosticCode) -> Self {
        value.0
    }
}

/// One deterministic segment locating a diagnostic in structured input.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum DiagnosticPathSegment {
    /// A named field.
    Field(String),
    /// An element in an ordered collection.
    Index(u64),
    /// An opaque entity kind and deterministic local numeric ID.
    Entity {
        /// Stable lowercase entity kind.
        kind: String,
        /// Deterministic local entity ID.
        id: u64,
    },
}

/// A structured diagnostic returned across native, WASM, and service boundaries.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Diagnostic {
    severity: Severity,
    code: DiagnosticCode,
    message: String,
    path: Vec<DiagnosticPathSegment>,
    related: Vec<SemanticId>,
}

impl Diagnostic {
    /// Creates a diagnostic. Callers must supply deterministic path and related
    /// entity ordering.
    #[must_use]
    pub fn new(
        severity: Severity,
        code: DiagnosticCode,
        message: impl Into<String>,
        path: Vec<DiagnosticPathSegment>,
        related: Vec<SemanticId>,
    ) -> Self {
        Self {
            severity,
            code,
            message: message.into(),
            path,
            related,
        }
    }

    /// Returns severity.
    #[must_use]
    pub const fn severity(&self) -> Severity {
        self.severity
    }

    /// Returns the stable machine-readable code.
    #[must_use]
    pub const fn code(&self) -> &DiagnosticCode {
        &self.code
    }

    /// Returns the human-readable explanation.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the deterministic input path.
    #[must_use]
    pub fn path(&self) -> &[DiagnosticPathSegment] {
        &self.path
    }

    /// Returns semantically related entities.
    #[must_use]
    pub fn related(&self) -> &[SemanticId] {
        &self.related
    }
}

#[cfg(test)]
mod tests {
    use super::DiagnosticCode;

    #[test]
    fn diagnostic_codes_are_stable_tokens() {
        assert!(DiagnosticCode::try_new("TOPOLOGY.LOOP.NOT_CLOSED").is_ok());
        assert!(DiagnosticCode::try_new("Topology loop").is_err());
        assert!(DiagnosticCode::try_new("TOPOLOGY..LOOP").is_err());
    }
}
