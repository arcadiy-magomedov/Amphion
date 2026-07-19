//! Validation contracts shared by topology builders and public operations.

use amphion_foundation::{Diagnostic, ToleranceContext};
use serde::{Deserialize, Serialize};

/// Cost and depth of validation requested by a caller.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum ValidationLevel {
    /// Handle integrity and inexpensive local invariants.
    Cheap,
    /// Full topology, manifoldness, orientation, and geometry consistency.
    Deep,
}

/// Deterministically ordered validation diagnostics.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ValidationReport {
    diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    /// Creates a report and canonicalizes diagnostic order.
    #[must_use]
    pub fn new(mut diagnostics: Vec<Diagnostic>) -> Self {
        diagnostics.sort_unstable();
        diagnostics.dedup();
        Self { diagnostics }
    }

    /// Returns every validation diagnostic.
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Returns true when the report contains no error-level diagnostic.
    #[must_use]
    pub fn is_valid(&self) -> bool {
        self.diagnostics
            .iter()
            .all(|diagnostic| diagnostic.severity() != amphion_foundation::Severity::Error)
    }
}

/// A value that can prove its invariants under an explicit tolerance context.
pub trait Validate: Send + Sync {
    /// Performs the requested validation level without mutating or healing the
    /// input.
    fn validate(&self, level: ValidationLevel, tolerance: &ToleranceContext) -> ValidationReport;
}
