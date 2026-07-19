//! Differential-oracle registration and result classification.
//!
//! A [`DifferentialOracle`] classifies whether a result produced by an
//! operation under test is consistent with expectations, **without depending
//! on any proprietary geometry kernel**. Oracles are pluggable: they
//! implement the trait and register in an [`OracleRegistry`], which runs all
//! of them against a given input/output pair and collects
//! [`OracleClassification`] results.
//!
//! Built-in oracles might check algebraic laws (volume conservation,
//! commutativity) or re-implement a simpler reference algorithm. External
//! proprietary kernel comparisons are possible but must be linked
//! conditionally behind a feature flag; the infrastructure here assumes none.
//!
//! # Example
//!
//! ```rust
//! # use amphion_test_support::{CaseBudget, CaseContext, DifferentialOracle, OracleId, OracleRegistry, OracleVerdict, ResourceLimitKind, TestRng, TestSeed};
//! struct VolumeConservation;
//!
//! impl DifferentialOracle<f64, f64> for VolumeConservation {
//!     fn oracle_id(&self) -> OracleId {
//!         OracleId::try_new("volume.conservation").unwrap()
//!     }
//!     fn classify(
//!         &self,
//!         _ctx: &mut CaseContext,
//!         input: &f64,
//!         result: &f64,
//!     ) -> Result<OracleVerdict, ResourceLimitKind> {
//!         if (result - input).abs() < 1e-9 {
//!             Ok(OracleVerdict::Agree)
//!         } else {
//!             Ok(OracleVerdict::Disagree {
//!                 description: format!("expected {input}, got {result}"),
//!             })
//!         }
//!     }
//! }
//!
//! let mut registry: OracleRegistry<f64, f64> = OracleRegistry::new();
//! registry.register(VolumeConservation);
//! let mut ctx = CaseContext::new(0, TestRng::from_seed(TestSeed::new(1)), CaseBudget::unlimited());
//! let classifications = registry.run_all(&mut ctx, &1.0, &1.0).unwrap();
//! assert_eq!(classifications.len(), 1);
//! ```

use core::{error::Error, fmt};

use crate::runner::{CaseContext, ResourceLimitKind};

// ── OracleIdError ──────────────────────────────────────────────────────────

/// Error returned for a malformed oracle identifier.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OracleIdError;

impl fmt::Display for OracleIdError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "oracle IDs must be non-empty ASCII alphanumeric segments separated by dots, \
             hyphens, or underscores",
        )
    }
}

impl Error for OracleIdError {}

// ── OracleId ───────────────────────────────────────────────────────────────

/// A stable identifier for a differential oracle (e.g. `"volume.conservation"`).
///
/// Valid identifiers consist of non-empty ASCII segments separated by dots.
/// Each segment may contain letters, digits, hyphens, and underscores.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct OracleId(String);

impl OracleId {
    /// Creates a validated oracle identifier.
    ///
    /// # Errors
    ///
    /// Returns [`OracleIdError`] when the value is empty, blank, or contains
    /// invalid characters.
    pub fn try_new(value: impl Into<String>) -> Result<Self, OracleIdError> {
        let value = value.into();
        let valid = !value.is_empty()
            && value.split('.').all(|segment| {
                !segment.is_empty()
                    && segment
                        .bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
            });
        if valid {
            Ok(Self(value))
        } else {
            Err(OracleIdError)
        }
    }

    /// Returns the oracle identifier string.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for OracleId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

// ── OracleVerdict ──────────────────────────────────────────────────────────

/// Classification of an operation result by a differential oracle.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OracleVerdict {
    /// The oracle agrees that the result is correct.
    Agree,
    /// The oracle disagrees: the result is incorrect or unexpected.
    Disagree {
        /// Human-readable description of the disagreement.
        description: String,
    },
    /// The oracle found a divergence between the expected and observed value.
    Diverge {
        /// The expected value, in a human-readable form.
        expected: String,
        /// The observed value, in a human-readable form.
        actual: String,
    },
    /// The oracle cannot evaluate this particular input/result pair.
    Abstain {
        /// Reason the oracle is not applicable here.
        reason: String,
    },
}

impl OracleVerdict {
    /// Returns `true` when the oracle agrees with the result.
    #[must_use]
    pub const fn is_agreement(&self) -> bool {
        matches!(self, Self::Agree)
    }

    /// Returns `true` when the oracle found a problem (disagree or diverge).
    #[must_use]
    pub const fn is_failure(&self) -> bool {
        matches!(self, Self::Disagree { .. } | Self::Diverge { .. })
    }

    /// Returns `true` when the oracle abstained from evaluating.
    #[must_use]
    pub const fn is_abstention(&self) -> bool {
        matches!(self, Self::Abstain { .. })
    }
}

// ── DifferentialOracle ─────────────────────────────────────────────────────

/// Classifies whether an operation result is consistent with expectations.
///
/// Implementations must be `Send + Sync` so they can be used across test
/// threads. They must not depend on proprietary kernels at compile time.
pub trait DifferentialOracle<I, O>: Send + Sync {
    /// Returns the stable identifier for this oracle.
    fn oracle_id(&self) -> OracleId;

    /// Classifies `result` given the original `input`.
    ///
    /// # Errors
    ///
    /// Returns a [`ResourceLimitKind`] when the oracle exhausts its budget.
    fn classify(
        &self,
        ctx: &mut CaseContext,
        input: &I,
        result: &O,
    ) -> Result<OracleVerdict, ResourceLimitKind>;
}

// ── OracleClassification ───────────────────────────────────────────────────

/// The verdict from one oracle for one input/result pair.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleClassification {
    /// Identifier of the oracle that produced this verdict.
    pub oracle_id: OracleId,
    /// The verdict.
    pub verdict: OracleVerdict,
}

impl OracleClassification {
    /// Returns `true` when the oracle reported a failure (disagree or diverge).
    #[must_use]
    pub const fn is_failure(&self) -> bool {
        self.verdict.is_failure()
    }
}

// ── OracleRegistrationError ────────────────────────────────────────────────

/// Error returned when registering an oracle whose `OracleId` is already
/// present in the registry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OracleRegistrationError {
    /// The duplicated oracle identifier.
    pub id: OracleId,
}

impl fmt::Display for OracleRegistrationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "oracle {:?} is already registered; OracleIds must be unique \
             to avoid ambiguous duplicate classifications",
            self.id.as_str()
        )
    }
}

impl Error for OracleRegistrationError {}

// ── OracleRegistry ─────────────────────────────────────────────────────────

/// A registry of differential oracles for a specific input/output type pair.
///
/// All registered oracles are run in registration order and their verdicts
/// collected. A registry with no registered oracles always returns an empty
/// classification list.
pub struct OracleRegistry<I, O> {
    oracles: Vec<Box<dyn DifferentialOracle<I, O>>>,
}

impl<I, O> OracleRegistry<I, O> {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            oracles: Vec::new(),
        }
    }

    /// Registers a new oracle.
    ///
    /// Oracles are run in the order they are registered.
    ///
    /// # Errors
    ///
    /// Returns [`OracleRegistrationError`] when an oracle with the same
    /// [`OracleId`] is already registered.
    pub fn register(
        &mut self,
        oracle: impl DifferentialOracle<I, O> + 'static,
    ) -> Result<(), OracleRegistrationError> {
        let id = oracle.oracle_id();
        if self.oracles.iter().any(|o| o.oracle_id() == id) {
            return Err(OracleRegistrationError { id });
        }
        self.oracles.push(Box::new(oracle));
        Ok(())
    }

    /// Returns the number of registered oracles.
    #[must_use]
    pub fn len(&self) -> usize {
        self.oracles.len()
    }

    /// Returns `true` when no oracles are registered.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.oracles.is_empty()
    }

    /// Runs every registered oracle against `input`/`result` and returns all
    /// classifications in registration order.
    ///
    /// # Errors
    ///
    /// Returns the first resource-limit hit from the shared [`CaseContext`].
    pub fn run_all(
        &self,
        ctx: &mut CaseContext,
        input: &I,
        result: &O,
    ) -> Result<Vec<OracleClassification>, ResourceLimitKind> {
        let mut out = Vec::with_capacity(self.oracles.len());
        for oracle in &self.oracles {
            ctx.charge_oracle()?;
            out.push(OracleClassification {
                oracle_id: oracle.oracle_id(),
                verdict: oracle.classify(ctx, input, result)?,
            });
        }
        Ok(out)
    }

    /// Returns only the classifications that represent failures.
    ///
    /// # Errors
    ///
    /// Returns the first resource-limit hit from the shared [`CaseContext`].
    pub fn run_all_failures(
        &self,
        ctx: &mut CaseContext,
        input: &I,
        result: &O,
    ) -> Result<Vec<OracleClassification>, ResourceLimitKind> {
        Ok(self
            .run_all(ctx, input, result)?
            .into_iter()
            .filter(OracleClassification::is_failure)
            .collect())
    }
}

impl<I, O> Default for OracleRegistry<I, O> {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{DifferentialOracle, OracleId, OracleIdError, OracleRegistry, OracleVerdict};
    use crate::rng::{TestRng, TestSeed};
    use crate::runner::{CaseBudget, CaseContext, ResourceLimitKind};

    // ── OracleId ──

    #[test]
    fn oracle_id_accepts_valid_formats() {
        assert!(OracleId::try_new("volume.conservation").is_ok());
        assert!(OracleId::try_new("a").is_ok());
        assert!(OracleId::try_new("a.b.c").is_ok());
        assert!(OracleId::try_new("my-oracle_v2").is_ok());
        assert!(OracleId::try_new("UPPER.CASE").is_ok());
    }

    #[test]
    fn oracle_id_rejects_invalid_formats() {
        assert_eq!(OracleId::try_new(""), Err(OracleIdError));
        assert_eq!(OracleId::try_new(".leading"), Err(OracleIdError));
        assert_eq!(OracleId::try_new("trailing."), Err(OracleIdError));
        assert_eq!(OracleId::try_new("double..dot"), Err(OracleIdError));
        assert_eq!(OracleId::try_new("has space"), Err(OracleIdError));
    }

    // ── OracleVerdict helpers ──

    #[test]
    fn verdict_helpers_are_correct() {
        assert!(OracleVerdict::Agree.is_agreement());
        assert!(!OracleVerdict::Agree.is_failure());
        assert!(
            OracleVerdict::Disagree {
                description: String::from("x")
            }
            .is_failure()
        );
        assert!(
            OracleVerdict::Diverge {
                expected: String::from("1"),
                actual: String::from("2")
            }
            .is_failure()
        );
        assert!(
            OracleVerdict::Abstain {
                reason: String::from("n/a")
            }
            .is_abstention()
        );
    }

    // ── OracleRegistry ──

    struct AlwaysAgrees;

    impl DifferentialOracle<i64, i64> for AlwaysAgrees {
        fn oracle_id(&self) -> OracleId {
            OracleId::try_new("always.agrees").unwrap()
        }
        fn classify(
            &self,
            _ctx: &mut CaseContext,
            _input: &i64,
            _result: &i64,
        ) -> Result<OracleVerdict, ResourceLimitKind> {
            Ok(OracleVerdict::Agree)
        }
    }

    struct RejectNegative;

    impl DifferentialOracle<i64, i64> for RejectNegative {
        fn oracle_id(&self) -> OracleId {
            OracleId::try_new("reject.negative").unwrap()
        }
        fn classify(
            &self,
            _ctx: &mut CaseContext,
            _input: &i64,
            result: &i64,
        ) -> Result<OracleVerdict, ResourceLimitKind> {
            if *result >= 0 {
                Ok(OracleVerdict::Agree)
            } else {
                Ok(OracleVerdict::Disagree {
                    description: format!("negative result: {result}"),
                })
            }
        }
    }

    #[test]
    fn empty_registry_returns_no_classifications() {
        let registry: OracleRegistry<i64, i64> = OracleRegistry::new();
        let mut ctx = CaseContext::new(
            0,
            TestRng::from_seed(TestSeed::new(1)),
            CaseBudget::unlimited(),
        );
        assert!(registry.run_all(&mut ctx, &1, &1).unwrap().is_empty());
    }

    #[test]
    fn registry_runs_all_registered_oracles() {
        let mut registry: OracleRegistry<i64, i64> = OracleRegistry::new();
        registry.register(AlwaysAgrees).expect("unique");
        registry.register(RejectNegative).expect("unique");
        let mut ctx = CaseContext::new(
            0,
            TestRng::from_seed(TestSeed::new(1)),
            CaseBudget::unlimited(),
        );
        let results = registry.run_all(&mut ctx, &5, &5).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn registry_reports_failures_correctly() {
        let mut registry: OracleRegistry<i64, i64> = OracleRegistry::new();
        registry.register(AlwaysAgrees).expect("unique");
        registry.register(RejectNegative).expect("unique");

        let mut ctx = CaseContext::new(
            0,
            TestRng::from_seed(TestSeed::new(1)),
            CaseBudget::unlimited(),
        );
        let failures = registry.run_all_failures(&mut ctx, &5, &-1).unwrap();
        assert_eq!(failures.len(), 1);
        assert_eq!(failures[0].oracle_id.as_str(), "reject.negative");
    }

    #[test]
    fn registry_is_deterministic_in_registration_order() {
        let mut registry: OracleRegistry<i64, i64> = OracleRegistry::new();
        registry.register(AlwaysAgrees).expect("unique");
        registry.register(RejectNegative).expect("unique");
        let mut ctx = CaseContext::new(
            0,
            TestRng::from_seed(TestSeed::new(1)),
            CaseBudget::unlimited(),
        );
        let results = registry.run_all(&mut ctx, &0, &0).unwrap();
        assert_eq!(results[0].oracle_id.as_str(), "always.agrees");
        assert_eq!(results[1].oracle_id.as_str(), "reject.negative");
    }

    #[test]
    fn registry_rejects_duplicate_oracle_ids() {
        let mut registry: OracleRegistry<i64, i64> = OracleRegistry::new();
        registry.register(AlwaysAgrees).expect("first registration");
        let err = registry.register(AlwaysAgrees).unwrap_err();
        assert_eq!(err.id.as_str(), "always.agrees");
    }

    #[test]
    fn oracle_send_sync_constraint_compiles() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AlwaysAgrees>();
        assert_send_sync::<RejectNegative>();
    }
    #[test]
    fn registry_respects_oracle_budget() {
        let mut registry: OracleRegistry<i64, i64> = OracleRegistry::new();
        registry.register(AlwaysAgrees).expect("unique");
        registry.register(RejectNegative).expect("unique");
        let mut ctx = CaseContext::new(
            0,
            TestRng::from_seed(TestSeed::new(1)),
            CaseBudget::from_limits(&crate::runner::ResourceLimits {
                max_oracle_calls: Some(1),
                ..crate::runner::ResourceLimits::default()
            }),
        );
        let result = registry.run_all(&mut ctx, &1, &1);
        assert_eq!(result, Err(ResourceLimitKind::MaxOracleCalls));
    }

    #[test]
    fn registry_propagates_resource_errors_from_oracles() {
        struct BudgetedOracle;

        impl DifferentialOracle<i64, i64> for BudgetedOracle {
            fn oracle_id(&self) -> OracleId {
                OracleId::try_new("budgeted.oracle").unwrap()
            }

            fn classify(
                &self,
                ctx: &mut CaseContext,
                _input: &i64,
                _result: &i64,
            ) -> Result<OracleVerdict, ResourceLimitKind> {
                ctx.consume_work(1)?;
                Ok(OracleVerdict::Agree)
            }
        }

        let mut registry: OracleRegistry<i64, i64> = OracleRegistry::new();
        registry.register(BudgetedOracle).expect("unique");
        let mut ctx = CaseContext::new(
            0,
            TestRng::from_seed(TestSeed::new(1)),
            CaseBudget::from_limits(&crate::runner::ResourceLimits {
                max_work_units: Some(0),
                ..crate::runner::ResourceLimits::default()
            }),
        );
        let result = registry.run_all(&mut ctx, &1, &1);
        assert_eq!(result, Err(ResourceLimitKind::MaxWorkUnits));
    }
}
