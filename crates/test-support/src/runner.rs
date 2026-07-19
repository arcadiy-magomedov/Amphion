//! Reusable test runner primitives for invariant, property, and metamorphic
//! test families.
//!
//! These primitives are fully generic over the input and output types and do
//! not depend on any specific geometry or topology implementation. They
//! integrate with [`TestRng`] and [`CaseId`] from the `rng` module to
//! produce deterministic, reproducible results with structured failure records.
//!
//! # Case derivation (V3)
//!
//! Since [`crate::CASE_SEQUENCE_VERSION`] 3, each case's RNG is derived **in O(1)**
//! from `(primary_seed, stream_name, case_index)` via
//! `seed.for_stream("{CASE_SEQUENCE_VERSION}\x00{stream}\x00{case_index}")`.
//! This means adding, removing, or reordering checks does not change any
//! other case's generated inputs, and replay of case `K` is direct without
//! advancing through `K − 1` prior draws.  [`CaseId`]s use the analogous
//! [`TestSeed::for_case_stream`] derivation.
//!
//! # Replay
//!
//! Use [`RunConfig::with_replay`] to execute exactly one case (and optionally
//! one specific check) determined by a [`ReplayFilter`].  The runner skips
//! all other cases and checks, and returns [`RunnerError::ReplayMismatch`]
//! *before* generating any input if the requested check/relation name is not
//! present.  Combine with [`crate::fuzz::parse_replay_env`] (or the
//! [`configure_replay_from_env`] / [`apply_replay_config`] helpers) to
//! support the full seven-field replay identity.
//!
//! # Resource limits
//!
//! [`ResourceLimits`] caps the number of retained failures, the byte size of
//! individual and aggregate failure messages, the total byte size of generated
//! inputs, and the number of cases run. Limits are enforced at case (and
//! failure) boundaries; an in-process closure that never yields **cannot** be
//! forcibly killed. For hard wall-clock timeouts, wrap the test process with
//! an external timeout (`timeout(1)` on Linux,
//! `$proc = Start-Process cargo -ArgumentList '...' -PassThru; if (-not $proc.WaitForExit(30000)) { Stop-Process -Id $proc.Id }` on Windows PowerShell). RNG draws are hard-enforced
//! through [`CaseContext::next_u64`] / [`CaseContext::next_f64`], and oracle
//! calls are charged by the runner before each invariant or relation. Only
//! work units and minimization steps remain cooperative — see
//! [`ResourceLimits::is_hard_enforced`].
//!
//! # Invariant tests
//!
//! An invariant check asserts that some property holds for every generated
//! input. Use [`Invariant::new`] to create named checks and pass a slice of
//! them to [`run_invariant_cases`].
//!
//! # Metamorphic tests
//!
//! A metamorphic relation asserts that if a base input is transformed in a
//! known way, the relationship between the two outputs must satisfy a defined
//! constraint (e.g. commutativity, transform invariance). Use
//! [`MetamorphicCase`] and [`run_metamorphic_cases`].
//!
//! Each metamorphic relation receives its own independent O(1) RNG stream
//! derived from `(seed, stream, relation_name, case_index)` so that adding or
//! reordering relations does not change the random state of any other relation.
//!
//! # Property tests
//!
//! For single-property checks use [`run_property_cases`], which is a
//! convenience wrapper around a single invariant.

use core::{error::Error, fmt};

use crate::fuzz::CheckKind;
use crate::is_stable_token;
use crate::rng::{
    CASE_SEQUENCE_VERSION, CaseId, TestRng, TestSeed, derive_case_rng, derive_transform_rng,
};

// ── RunnerError ────────────────────────────────────────────────────────────

/// Error returned by runner functions when the configuration is invalid.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RunnerError {
    /// Two invariants or relations share the same name; names must be unique
    /// so that failure reports are unambiguous.
    DuplicateCheckName {
        /// The duplicated name.
        name: &'static str,
    },
    /// A stream name, check name, or relation name contains invalid characters
    /// or is empty.  Names must be non-empty stable tokens: `[a-zA-Z0-9._:/-]`.
    InvalidToken {
        /// The category of the invalid field (e.g. `"stream_name"`).
        field: &'static str,
        /// The rejected value.
        value: String,
    },
    /// Serializing an input for resource accounting failed.
    InputEncoding(String),
    /// [`RunConfig::with_replay`] installed a replay check name that does not
    /// match any invariant or metamorphic relation passed
    /// to the runner. Detected *before* any input is generated so a typo in
    /// a replay command fails loudly instead of silently reporting a passing
    /// (empty) run.
    ReplayMismatch {
        /// The requested check/relation name that was not found.
        name: String,
    },
    /// Replay requested a different check kind than this runner executes.
    CheckKindMismatch {
        /// The kind this runner expects.
        expected: CheckKind,
        /// The kind requested by the replay filter.
        found: CheckKind,
    },
    /// [`RunConfig::with_replay`] or an env-replay identity specified an
    /// operation that does not match the [`RunConfig`]'s expected operation.
    /// Detected *before* any input is generated.
    OperationMismatch {
        /// The operation expected by the run configuration.
        expected: String,
        /// The operation found in the replay filter.
        found: String,
    },
}

impl fmt::Display for RunnerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateCheckName { name } => {
                write!(
                    f,
                    "duplicate check/relation name {name:?} — names must be unique"
                )
            }
            Self::InvalidToken { field, value } => write!(
                f,
                "invalid {field}: {value:?} — must be a non-empty stable token \
                 ([a-zA-Z0-9._:/-])"
            ),
            Self::InputEncoding(err) => write!(f, "failed to encode input for accounting: {err}"),
            Self::ReplayMismatch { name } => write!(
                f,
                "replay check/relation {name:?} does not match any registered \
                 invariant or metamorphic relation name"
            ),
            Self::CheckKindMismatch { expected, found } => write!(
                f,
                "replay check kind mismatch: expected {expected}, found {found}"
            ),
            Self::OperationMismatch { expected, found } => write!(
                f,
                "replay operation mismatch: RunConfig expects {expected:?} but replay filter \
                 carries {found:?}"
            ),
        }
    }
}

impl Error for RunnerError {}

// ── ResourceLimits ─────────────────────────────────────────────────────────

/// Deterministic caps on runner resource consumption.
///
/// Limits are enforced **cooperatively** at case and failure boundaries.  An
/// in-process closure that never yields cannot be preempted.  For hard
/// wall-clock limits, use an external process-level timeout.
///
/// `None` means "unlimited" for every `Option` field.  An explicit
/// `Some(0)` means "cap at exactly zero" — it is deliberately distinct from
/// `None` so that, for example, `max_retained_failures: Some(0)` rejects
/// every failure rather than allowing unlimited failures.
#[derive(Clone, Debug)]
pub struct ResourceLimits {
    /// Maximum number of failure records to retain.  `None` = unlimited.
    ///
    /// **Hard-enforced**: checked immediately before each failure would be
    /// recorded, so at most this many failures are ever pushed.
    pub max_retained_failures: Option<usize>,
    /// Maximum byte length of an individual failure message.  Longer messages
    /// are truncated at the nearest valid UTF-8 boundary.  `None` = unlimited.
    ///
    /// **Hard-enforced**.
    pub max_failure_message_bytes: Option<usize>,
    /// Maximum number of cases to run before stopping early.  `None` =
    /// unlimited.
    ///
    /// **Hard-enforced**: checked at the top of the case loop, so exactly
    /// this many cases are generated and exercised before the run stops.
    pub max_cases_run: Option<u32>,
    /// Maximum total byte length across all retained failure messages
    /// (summed after per-message truncation).  `None` = unlimited.
    ///
    /// **Hard-enforced**: checked immediately *before* each failure would be
    /// recorded using `checked_add(incoming)` so that `current + incoming >
    /// cap` is the rejection condition — not merely `current >= cap`.
    pub max_total_report_bytes: Option<usize>,
    /// Maximum total byte size of all generated inputs across a run, measured
    /// via `serde_json` canonical encoding. Checked before oracle execution.
    /// `None` = unlimited. `Some(0)` rejects the first nonempty input.
    ///
    /// **Hard-enforced**.
    pub max_total_input_bytes: Option<u64>,
    /// Cap on the number of RNG draws a single case's generator (or
    /// transform) may perform. `None` = unlimited.
    ///
    /// **Hard-enforced** through [`CaseContext::next_u64`] and
    /// [`CaseContext::next_f64`].
    pub max_rng_draws_per_case: Option<u64>,
    /// Advisory cap on abstract "work units" a single case may perform.
    /// `None` = unlimited.
    ///
    /// **Not hard-enforced**, for the same reason as
    /// [`max_rng_draws_per_case`](Self::max_rng_draws_per_case): the runner
    /// cannot preempt an in-process closure.  See
    /// [`ResourceLimits::is_hard_enforced`].
    pub max_work_units: Option<u64>,
    /// Cap on oracle calls. `None` = unlimited.
    ///
    /// **Hard-enforced** by the runner immediately before each invariant or
    /// relation check.
    pub max_oracle_calls: Option<u64>,
    /// Advisory cap on minimisation steps.  `None` = unlimited.
    ///
    /// **Not hard-enforced** in the generic runner; minimisers that cooperate
    /// with the budget should consult this via [`CaseBudget`].
    pub max_minimization_steps: Option<u64>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_retained_failures: Some(1_024),
            max_failure_message_bytes: Some(65_536),
            max_cases_run: None,
            max_total_report_bytes: None,
            max_total_input_bytes: None,
            max_rng_draws_per_case: None,
            max_work_units: None,
            max_oracle_calls: None,
            max_minimization_steps: None,
        }
    }
}

impl ResourceLimits {
    /// Returns `true` when `kind` is actually enforced by the runner at case
    /// or failure boundaries.
    ///
    /// RNG draws are enforced through [`CaseContext::next_u64`] and
    /// [`CaseContext::next_f64`]; oracle calls are charged by the runner
    /// before each invariant or relation. Only work units and minimization
    /// steps remain cooperative.
    #[must_use]
    pub const fn is_hard_enforced(kind: ResourceLimitKind) -> bool {
        !matches!(
            kind,
            ResourceLimitKind::MaxWorkUnits | ResourceLimitKind::MaxMinimizationSteps
        )
    }
}

// ── ResourceLimitKind ──────────────────────────────────────────────────────

/// Indicates which resource limit was hit during a run.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceLimitKind {
    /// The run stopped because [`ResourceLimits::max_retained_failures`] was
    /// reached.
    MaxRetainedFailures,
    /// The run stopped because [`ResourceLimits::max_cases_run`] was reached.
    MaxCasesRun,
    /// The run stopped because [`ResourceLimits::max_total_report_bytes`] was
    /// reached.
    MaxTotalReportBytes,
    /// The run stopped because [`ResourceLimits::max_total_input_bytes`] was
    /// reached.
    MaxTotalInputBytes,
    /// The run stopped because [`ResourceLimits::max_rng_draws_per_case`] was
    /// reached.
    MaxRngDrawsPerCase,
    /// [`ResourceLimits::max_work_units`] was configured.  Advisory only —
    /// see [`ResourceLimits::is_hard_enforced`]; the runner never actually
    /// produces this variant itself.
    MaxWorkUnits,
    /// The run stopped because [`ResourceLimits::max_oracle_calls`] was
    /// reached.
    MaxOracleCalls,
    /// [`ResourceLimits::max_minimization_steps`] was configured.  Advisory
    /// only.
    MaxMinimizationSteps,
}

impl fmt::Display for ResourceLimitKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MaxRetainedFailures => f.write_str("maximum retained failures reached"),
            Self::MaxCasesRun => f.write_str("maximum cases run reached"),
            Self::MaxTotalReportBytes => f.write_str("maximum total report bytes reached"),
            Self::MaxTotalInputBytes => f.write_str("maximum total input bytes reached"),
            Self::MaxRngDrawsPerCase => f.write_str("maximum RNG draws per case reached"),
            Self::MaxWorkUnits => f.write_str("maximum work units (advisory)"),
            Self::MaxOracleCalls => f.write_str("maximum oracle calls reached"),
            Self::MaxMinimizationSteps => f.write_str("maximum minimization steps (advisory)"),
        }
    }
}

/// Structured error returned by check closures.
///
/// Distinguishes assertion failures from budget exhaustion so the runner can
/// propagate resource limits without recording a synthetic test failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CaseCheckError {
    /// The check failed: the invariant or relation did not hold.
    Failure(String),
    /// The check exhausted its resource budget.
    ResourceExhausted(ResourceLimitKind),
}

impl fmt::Display for CaseCheckError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Failure(message) => f.write_str(message),
            Self::ResourceExhausted(kind) => write!(f, "resource exhausted: {kind}"),
        }
    }
}

impl Error for CaseCheckError {}

impl From<ResourceLimitKind> for CaseCheckError {
    fn from(kind: ResourceLimitKind) -> Self {
        Self::ResourceExhausted(kind)
    }
}

// ── ReplayFilter ───────────────────────────────────────────────────────────

/// Selects exactly one case/check identity for replay.
///
/// Build via [`RunConfig::with_replay`].  When a [`ReplayFilter`] is
/// installed, the runner executes only `case_index` and skips every check
/// whose name does not match `check_name`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayFilter {
    /// Zero-based case index to replay.
    pub case_index: u64,
    /// Operation label recorded with the failure.
    pub operation: String,
    /// Check kind recorded with the failure.
    pub check_kind: CheckKind,
    /// Only run this specific invariant or metamorphic relation.
    pub check_name: String,
}

impl ReplayFilter {
    /// Creates a replay filter for a specific case.
    #[must_use]
    pub fn new(
        case_index: u64,
        operation: String,
        check_kind: CheckKind,
        check_name: String,
    ) -> Self {
        Self {
            case_index,
            operation,
            check_kind,
            check_name,
        }
    }
}

// ── CaseBudget / CaseContext ───────────────────────────────────────────────

/// Per-case cooperative budget for optional resource accounting.
///
/// Derived from [`ResourceLimits`] at the start of each case.  Cooperating
/// generators, transforms, oracle adapters, and minimisers may call the
/// `charge_*` methods to consume budget and detect exhaustion.
///
/// # Hard limits vs. advisory limits
///
/// Only limits for which [`ResourceLimits::is_hard_enforced`] returns `true`
/// are actually enforced by the runner.  Budget fields corresponding to
/// advisory limits are present for cooperative use only — returning
/// `Err(…)` from a `charge_*` method is the signal, not a forced termination.
#[allow(clippy::struct_field_names)]
#[derive(Clone, Debug)]
pub struct CaseBudget {
    draws_remaining: Option<u64>,
    work_units_remaining: Option<u64>,
    oracle_calls_remaining: Option<u64>,
    minimization_steps_remaining: Option<u64>,
}

impl CaseBudget {
    /// Creates a budget from the advisory limits in `limits`.
    #[must_use]
    pub fn from_limits(limits: &ResourceLimits) -> Self {
        Self {
            draws_remaining: limits.max_rng_draws_per_case,
            work_units_remaining: limits.max_work_units,
            oracle_calls_remaining: limits.max_oracle_calls,
            minimization_steps_remaining: limits.max_minimization_steps,
        }
    }

    /// Creates an unlimited budget (no advisory limits).
    #[must_use]
    pub const fn unlimited() -> Self {
        Self {
            draws_remaining: None,
            work_units_remaining: None,
            oracle_calls_remaining: None,
            minimization_steps_remaining: None,
        }
    }

    /// Consumes one RNG draw from the budget.
    ///
    /// Returns `Err(ResourceLimitKind::MaxRngDrawsPerCase)` when the draw
    /// budget is exhausted.  Returns `Ok(())` when no limit is set.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceLimitKind::MaxRngDrawsPerCase`] when exhausted.
    pub fn charge_draw(&mut self) -> Result<(), ResourceLimitKind> {
        if let Some(ref mut rem) = self.draws_remaining {
            if *rem == 0 {
                return Err(ResourceLimitKind::MaxRngDrawsPerCase);
            }
            *rem -= 1;
        }
        Ok(())
    }

    /// Consumes `units` work units from the budget.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceLimitKind::MaxWorkUnits`] when exhausted.
    pub fn charge_work(&mut self, units: u64) -> Result<(), ResourceLimitKind> {
        if let Some(ref mut rem) = self.work_units_remaining {
            *rem = rem
                .checked_sub(units)
                .ok_or(ResourceLimitKind::MaxWorkUnits)?;
        }
        Ok(())
    }

    /// Consumes one oracle invocation from the budget.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceLimitKind::MaxOracleCalls`] when exhausted.
    pub fn charge_oracle(&mut self) -> Result<(), ResourceLimitKind> {
        if let Some(ref mut rem) = self.oracle_calls_remaining {
            if *rem == 0 {
                return Err(ResourceLimitKind::MaxOracleCalls);
            }
            *rem -= 1;
        }
        Ok(())
    }

    /// Consumes one minimisation step from the budget.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceLimitKind::MaxMinimizationSteps`] when exhausted.
    pub fn charge_minimization_step(&mut self) -> Result<(), ResourceLimitKind> {
        if let Some(ref mut rem) = self.minimization_steps_remaining {
            if *rem == 0 {
                return Err(ResourceLimitKind::MaxMinimizationSteps);
            }
            *rem -= 1;
        }
        Ok(())
    }

    /// Returns the remaining draw budget, or `None` if unlimited.
    #[must_use]
    pub const fn draws_remaining(&self) -> Option<u64> {
        self.draws_remaining
    }
}

/// Per-case execution context passed to generator and transform closures.
///
/// Provides a **budget-charged RNG** so that every `next_u64` / `next_f64`
/// draw is automatically charged to the per-case draw budget before the
/// underlying RNG is called. The underlying [`TestRng`] is private and
/// cannot be extracted, preventing budget bypass.
///
/// Construct via the runner (one context per case and one per transform
/// relation); use [`CaseBudget::from_limits`] / [`CaseBudget::unlimited`]
/// together with [`CaseContext::new`] to build a context outside a runner.
#[derive(Debug)]
pub struct CaseContext {
    /// Zero-based case index within the stream.
    pub case_index: u64,
    rng: TestRng,
    budget: CaseBudget,
}

impl CaseContext {
    /// Constructs a context with the given case index, RNG, and budget.
    ///
    /// Normally constructed by the runner; this constructor is public so
    /// that generator closures under test can be called directly without
    /// a full runner invocation.
    #[must_use]
    pub fn new(case_index: u64, rng: TestRng, budget: CaseBudget) -> Self {
        Self {
            case_index,
            rng,
            budget,
        }
    }

    /// Draws a `u64`, charging one draw from the budget.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceLimitKind::MaxRngDrawsPerCase`] when the draw budget
    /// is exhausted.
    pub fn next_u64(&mut self) -> Result<u64, ResourceLimitKind> {
        self.budget.charge_draw()?;
        Ok(self.rng.next_u64())
    }

    /// Draws an `f64` in `[0, 1)`, charging one draw from the budget.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceLimitKind::MaxRngDrawsPerCase`] when the draw budget
    /// is exhausted.
    pub fn next_f64(&mut self) -> Result<f64, ResourceLimitKind> {
        self.budget.charge_draw()?;
        Ok(self.rng.next_f64())
    }

    /// Charges `units` cooperative work units from the budget.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceLimitKind::MaxWorkUnits`] when exhausted.
    pub fn consume_work(&mut self, units: u64) -> Result<(), ResourceLimitKind> {
        self.budget.charge_work(units)
    }

    /// Charges one minimisation step from the budget.
    ///
    /// # Errors
    ///
    /// Returns [`ResourceLimitKind::MaxMinimizationSteps`] when exhausted.
    pub fn charge_minimization_step(&mut self) -> Result<(), ResourceLimitKind> {
        self.budget.charge_minimization_step()
    }

    /// Returns the remaining draw budget, or `None` if unlimited.
    #[must_use]
    pub fn draws_remaining(&self) -> Option<u64> {
        self.budget.draws_remaining()
    }

    /// Runner-internal: charges one oracle call from the budget.
    pub(crate) fn charge_oracle(&mut self) -> Result<(), ResourceLimitKind> {
        self.budget.charge_oracle()
    }
}

// ── RunConfig ──────────────────────────────────────────────────────────────

/// Configuration for a test run: seed, stream name, iteration count, optional
/// replay filter, and resource limits.
///
/// The `stream_name` is mixed into the seed to derive case RNGs and
/// [`CaseId`]s (V2 derivation; see [`crate::CASE_SEQUENCE_VERSION`]).
/// Using a stable name ensures IDs remain reproducible across runs.
/// The name is also stored in every [`CaseFailure`] for unambiguous replay.
#[derive(Clone, Debug)]
pub struct RunConfig {
    pub(crate) seed: TestSeed,
    pub(crate) stream_name: &'static str,
    pub(crate) operation: &'static str,
    pub(crate) iterations: u32,
    pub(crate) stop_on_first_failure: bool,
    pub(crate) replay: Option<ReplayFilter>,
    pub(crate) limits: ResourceLimits,
}

impl RunConfig {
    /// Creates a run configuration, validating `stream_name` and `operation`.
    ///
    /// Both fields must be non-empty stable tokens (`[a-zA-Z0-9._:/-]`).
    ///
    /// # Errors
    ///
    /// Returns [`RunnerError::InvalidToken`] when either field is empty or
    /// contains invalid characters.
    pub fn new(
        seed: TestSeed,
        stream_name: &'static str,
        operation: &'static str,
        iterations: u32,
    ) -> Result<Self, RunnerError> {
        if !is_stable_token(stream_name) {
            return Err(RunnerError::InvalidToken {
                field: "stream_name",
                value: stream_name.to_owned(),
            });
        }
        if !is_stable_token(operation) {
            return Err(RunnerError::InvalidToken {
                field: "operation",
                value: operation.to_owned(),
            });
        }
        Ok(Self {
            seed,
            stream_name,
            operation,
            iterations,
            stop_on_first_failure: false,
            replay: None,
            limits: ResourceLimits::default(),
        })
    }

    /// Returns a copy that stops after the first failure.
    #[must_use]
    pub const fn stop_on_first_failure(mut self) -> Self {
        self.stop_on_first_failure = true;
        self
    }

    /// Installs a [`ReplayFilter`] so only one case (and optionally one check)
    /// is executed.
    #[must_use]
    pub fn with_replay(mut self, filter: ReplayFilter) -> Self {
        self.replay = Some(filter);
        self
    }

    /// Overrides the default [`ResourceLimits`].
    #[must_use]
    pub fn with_resource_limits(mut self, limits: ResourceLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Returns the primary seed.
    #[must_use]
    pub const fn seed(&self) -> TestSeed {
        self.seed
    }

    /// Returns the stream name.
    #[must_use]
    pub const fn stream_name(&self) -> &'static str {
        self.stream_name
    }

    /// Returns the number of cases to generate in a full (non-replay) run.
    #[must_use]
    pub const fn iterations(&self) -> u32 {
        self.iterations
    }

    /// Returns the replay filter, if installed.
    #[must_use]
    pub const fn replay(&self) -> Option<&ReplayFilter> {
        self.replay.as_ref()
    }
}

// ── Replay-from-environment helpers ────────────────────────────────────────

/// Error returned by [`configure_replay_from_env`] when the replay environment
/// is inconsistent with the provided [`RunConfig`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayEnvError {
    /// The environment variables were malformed or partially set.
    ParseError(crate::fuzz::ReplayConfigError),
    /// The stream name from the environment does not match the stream name in
    /// the [`RunConfig`].  Running the wrong stream would silently generate
    /// different inputs than those recorded in the failure report.
    StreamMismatch {
        /// The stream name from the `RunConfig`.
        config: &'static str,
        /// The stream name from the environment.
        env: String,
    },
    /// The replay identity used a different case-sequence version.
    VersionMismatch {
        /// The version found in the replay identity.
        found: u8,
    },
    /// The replay identity requested a different operation.
    OperationMismatch {
        /// The operation expected by the run configuration.
        expected: String,
        /// The operation found in the replay identity.
        found: String,
    },
    /// The replay identity requested a different check kind.
    CheckKindMismatch {
        /// The check kind expected by the run configuration.
        expected: CheckKind,
        /// The check kind found in the replay identity.
        found: CheckKind,
    },
}

impl fmt::Display for ReplayEnvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ParseError(e) => write!(f, "replay env parse error: {e}"),
            Self::StreamMismatch { config, env } => write!(
                f,
                "replay stream mismatch: env has {env:?} but RunConfig has {config:?}"
            ),
            Self::VersionMismatch { found } => write!(
                f,
                "replay case sequence version mismatch: found {found}, expected {CASE_SEQUENCE_VERSION}"
            ),
            Self::OperationMismatch { expected, found } => write!(
                f,
                "replay operation mismatch: expected {expected:?}, found {found:?}"
            ),
            Self::CheckKindMismatch { expected, found } => write!(
                f,
                "replay check kind mismatch: expected {expected}, found {found}"
            ),
        }
    }
}

impl core::error::Error for ReplayEnvError {
    fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
        match self {
            Self::ParseError(e) => Some(e),
            Self::StreamMismatch { .. }
            | Self::VersionMismatch { .. }
            | Self::OperationMismatch { .. }
            | Self::CheckKindMismatch { .. } => None,
        }
    }
}

impl From<crate::fuzz::ReplayConfigError> for ReplayEnvError {
    fn from(e: crate::fuzz::ReplayConfigError) -> Self {
        Self::ParseError(e)
    }
}

/// Applies a manually-constructed [`crate::fuzz::ReplayConfig`] to `config`,
/// replacing its seed with `replay.seed` and installing a [`ReplayFilter`]
/// for the full replay identity. Also validates stream, version, and
/// operation compatibility.
///
/// This is a pure, environment-independent helper: it does not read any
/// environment variables itself, so it is safe to call from parallel tests.
/// Use [`configure_replay_from_env`] to read the seven `AMPHION_TEST_*`
/// environment variables directly.
///
/// # Errors
///
/// Returns [`ReplayEnvError::StreamMismatch`] when `replay.stream_name`
/// disagrees with `config.stream_name`.
pub fn apply_replay_config(
    mut config: RunConfig,
    replay: &crate::fuzz::ReplayConfig,
) -> Result<RunConfig, ReplayEnvError> {
    if config.stream_name != replay.stream_name {
        return Err(ReplayEnvError::StreamMismatch {
            config: config.stream_name,
            env: replay.stream_name.clone(),
        });
    }
    if replay.case_sequence_version != CASE_SEQUENCE_VERSION {
        return Err(ReplayEnvError::VersionMismatch {
            found: replay.case_sequence_version,
        });
    }
    if replay.operation != config.operation {
        return Err(ReplayEnvError::OperationMismatch {
            expected: config.operation.to_owned(),
            found: replay.operation.clone(),
        });
    }
    config.seed = replay.seed;
    config.replay = Some(ReplayFilter::new(
        replay.case_index,
        replay.operation.clone(),
        replay.check_kind,
        replay.check_name.clone(),
    ));
    Ok(config)
}

/// Reads the full seven-field replay identity from the environment and, when
/// present, applies it to `config` via
/// [`apply_replay_config`].
///
/// Returns `Ok(None)` when none of the replay variables are set — the caller
/// should proceed with a normal, non-replay run using `config` unchanged.
/// Returns `Ok(Some(_))` with a replay-configured [`RunConfig`] when all
/// required variables are valid.
///
/// # Errors
///
/// Returns [`ReplayEnvError`] when the environment variables are partially
/// set, malformed, or when the stream name disagrees with `config`.
pub fn configure_replay_from_env(config: RunConfig) -> Result<Option<RunConfig>, ReplayEnvError> {
    match crate::fuzz::parse_replay_env()? {
        Some(replay) => Ok(Some(apply_replay_config(config, &replay)?)),
        None => Ok(None),
    }
}

// ── CaseFailure ────────────────────────────────────────────────────────────

/// A single case failure recorded during a test run.
#[derive(Clone, Debug)]
pub struct CaseFailure {
    /// The case that failed.
    pub case_id: CaseId,
    /// Validated replay identity for the failure.
    pub identity: crate::report::ReplayIdentity,
    /// Human-readable failure description (truncated if resource limits apply).
    pub message: String,
    /// Canonical JSON of the final (possibly minimised) failing input.
    ///
    /// Serialised via `serde_json::to_value` at the point of failure
    /// recording; the byte count from the same call is included in
    /// `max_total_report_bytes` accounting to prevent oversized artifacts
    /// from being retained.
    pub input_json: serde_json::Value,
}

impl CaseFailure {
    /// Returns the primary seed for the failing case.
    #[must_use]
    pub const fn seed(&self) -> TestSeed {
        self.identity.seed()
    }

    /// Returns the stream name for the failing case.
    #[must_use]
    pub fn stream_name(&self) -> &str {
        self.identity.stream_name()
    }

    /// Returns the operation label for the failing case.
    #[must_use]
    pub fn operation(&self) -> &str {
        self.identity.operation()
    }

    /// Returns the sequential case index within the stream.
    #[must_use]
    pub const fn case_index(&self) -> u64 {
        self.identity.case_index()
    }

    /// Returns the kind of check that failed.
    #[must_use]
    pub const fn check_kind(&self) -> crate::fuzz::CheckKind {
        self.identity.check_kind()
    }

    /// Returns the name of the failed check.
    #[must_use]
    pub fn check_name(&self) -> &str {
        self.identity.check_name()
    }

    /// Converts this failure into a [`crate::report::FailureReport`].
    ///
    /// All identity fields are taken directly from the failure; the caller
    /// supplies no identity overrides. External metadata (tolerance context,
    /// diagnostics, replay command) can be attached to the returned report
    /// with its builder methods.
    ///
    /// # Errors
    ///
    /// Returns [`crate::report::ReportError`] if the stored identity fields
    /// fail the [`crate::report::FailureReport`] validation (e.g. derived
    /// `case_id` mismatch — should never occur for failures produced by a
    /// runner).
    pub fn to_failure_report(
        &self,
    ) -> Result<crate::report::FailureReport, crate::report::ReportError> {
        crate::report::FailureReport::new(
            self.identity.seed(),
            self.case_id,
            self.identity.stream_name(),
            self.identity.operation(),
            self.identity.case_index(),
            self.identity.check_kind(),
            self.identity.check_name(),
            self.input_json.clone(),
            self.message.clone(),
        )
    }

    /// Converts this failure into a current-schema (v1.2)
    /// [`crate::corpus::CorpusEntry`].
    ///
    /// All identity fields are taken from the failure. Minimisation
    /// provenance can be attached afterwards with
    /// [`crate::corpus::CorpusEntry::with_minimization`].
    ///
    /// # Errors
    ///
    /// Returns [`crate::corpus::CorpusError`] if validation fails (should
    /// not occur for failures produced by a runner).
    pub fn to_corpus_entry(
        &self,
    ) -> Result<crate::corpus::CorpusEntry, crate::corpus::CorpusError> {
        crate::corpus::CorpusEntry::new(
            self.case_id,
            self.identity.operation(),
            self.identity.stream_name(),
            self.identity.seed(),
            self.identity.case_index(),
            self.identity.check_kind(),
            self.identity.check_name(),
            self.input_json.clone(),
            self.message.clone(),
        )
    }

    /// Builds a [`crate::report::ReproducibleCommand`] for this failure.
    ///
    /// `package` and `test_name` are the Cargo package and Rust test function
    /// name that house the runner invocation and are the only genuinely
    /// external pieces of information needed.
    ///
    /// # Errors
    ///
    /// Returns [`crate::report::CommandTokenError`] if `package` or
    /// `test_name` fail command-token validation.
    pub fn to_reproducible_command(
        &self,
        package: impl Into<String>,
        test_name: impl Into<String>,
    ) -> Result<crate::report::ReproducibleCommand, crate::report::CommandTokenError> {
        crate::report::ReproducibleCommand::new(package, test_name, self.identity.clone())
    }
}

// ── RunReport ──────────────────────────────────────────────────────────────

/// Summary of all cases exercised during a test run.
#[derive(Clone, Debug)]
pub struct RunReport {
    /// Total number of cases generated.
    pub total_cases: u64,
    /// Cases where every check passed.
    pub passed_cases: u64,
    /// All recorded failures. Empty when the run succeeded.
    pub failures: Vec<CaseFailure>,
    /// Set when a resource limit was hit and caused an early stop.
    pub resource_limit_hit: Option<ResourceLimitKind>,
}

impl RunReport {
    /// Returns `true` when no failures were recorded.
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.failures.is_empty()
    }
}

// ── Invariant ──────────────────────────────────────────────────────────────

/// A named invariant check applied to every generated test input.
///
/// `I` is the generated input type. The check function receives the per-case
/// [`CaseContext`] plus a reference to the input and returns `Ok(())` on
/// success or [`CaseCheckError`] on failure.
///
/// # Example
///
/// ```rust
/// # use amphion_test_support::{CaseCheckError, Invariant};
/// let check: Invariant<f64> = Invariant::new("value.finite", |_ctx, v: &f64| {
///     if v.is_finite() {
///         Ok(())
///     } else {
///         Err(CaseCheckError::Failure(format!("{v} is not finite")))
///     }
/// });
/// ```
pub struct Invariant<I> {
    name: &'static str,
    #[allow(clippy::type_complexity)]
    check: Box<dyn Fn(&mut CaseContext, &I) -> Result<(), CaseCheckError>>,
}

impl<I> Invariant<I> {
    /// Creates a named invariant from a context-aware check closure.
    pub fn new(
        name: &'static str,
        check: impl Fn(&mut CaseContext, &I) -> Result<(), CaseCheckError> + 'static,
    ) -> Self {
        Self {
            name,
            check: Box::new(check),
        }
    }

    /// Returns the invariant name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }

    /// Applies the check to `input`.
    ///
    /// # Errors
    ///
    /// Returns the check's failure or resource exhaustion result.
    pub fn apply(&self, ctx: &mut CaseContext, input: &I) -> Result<(), CaseCheckError> {
        (self.check)(ctx, input)
    }
}

// ── MetamorphicCase ────────────────────────────────────────────────────────

/// A named metamorphic relation: a base input is transformed to a second
/// input, and the two outputs must satisfy the relation.
///
/// `I` is the input type, `O` is the output type produced by the operation
/// under test.  Each relation receives its own independent O(1) RNG stream
/// derived from `(seed, stream, relation_name, case_index)`.
///
/// # Example
///
/// Commutative addition: `a + b == b + a`.
///
/// ```rust
/// # use amphion_test_support::{CaseBudget, CaseCheckError, CaseContext, MetamorphicCase, TestRng, TestSeed};
/// let rel: MetamorphicCase<(f64, f64), f64> = MetamorphicCase::new(
///     "addition.commutative",
///     |_ctx, (a, b)| Ok((*b, *a)),
///     |_ctx, _, o1: &f64, _, o2: &f64| {
///         if (o1 - o2).abs() < 1e-15 {
///             Ok(())
///         } else {
///             Err(CaseCheckError::Failure(format!("{o1} != {o2}")))
///         }
///     },
/// );
/// ```
pub struct MetamorphicCase<I, O> {
    name: &'static str,
    #[allow(clippy::type_complexity)]
    transform: Box<dyn Fn(&mut CaseContext, &I) -> Result<I, ResourceLimitKind>>,
    #[allow(clippy::type_complexity)]
    relation: Box<dyn Fn(&mut CaseContext, &I, &O, &I, &O) -> Result<(), CaseCheckError>>,
}

impl<I, O> MetamorphicCase<I, O> {
    /// Creates a named metamorphic case.
    ///
    /// - `transform(ctx, base) -> related_input`
    /// - `relation(ctx, base, base_out, related, related_out) -> Ok(()) | Err(..)`
    pub fn new(
        name: &'static str,
        transform: impl Fn(&mut CaseContext, &I) -> Result<I, ResourceLimitKind> + 'static,
        relation: impl Fn(&mut CaseContext, &I, &O, &I, &O) -> Result<(), CaseCheckError> + 'static,
    ) -> Self {
        Self {
            name,
            transform: Box::new(transform),
            relation: Box::new(relation),
        }
    }

    /// Returns the metamorphic case name.
    #[must_use]
    pub const fn name(&self) -> &'static str {
        self.name
    }
}

type MinimizerFn<I> = dyn FnMut(&mut CaseContext, &I) -> Result<Option<I>, CaseCheckError>;

/// A per-step minimizer for failing test inputs.
///
/// After a check fails, the runner calls [`Minimizer::next_candidate`] repeatedly,
/// charging one minimisation step from the budget before each call. When
/// `next_candidate` returns `Ok(Some(smaller))`, the runner re-runs the
/// failing check on the smaller input and keeps it only when the failure is
/// preserved. `Ok(None)` ends minimization.
///
/// **Outer process timeout remains required for an infinite minimizer** —
/// the runner cannot preempt a minimizer closure that never returns.
pub struct Minimizer<I> {
    f: Box<MinimizerFn<I>>,
}

impl<I> Minimizer<I> {
    /// Creates a minimizer from a step function.
    pub fn new(
        f: impl FnMut(&mut CaseContext, &I) -> Result<Option<I>, CaseCheckError> + 'static,
    ) -> Self {
        Self { f: Box::new(f) }
    }

    /// Requests the next smaller candidate for `current`.
    ///
    /// # Errors
    ///
    /// Returns [`CaseCheckError::ResourceExhausted`] when the minimizer hits a
    /// resource limit internally.
    pub fn next_candidate(
        &mut self,
        ctx: &mut CaseContext,
        current: &I,
    ) -> Result<Option<I>, CaseCheckError> {
        (self.f)(ctx, current)
    }
}

// ── helpers ────────────────────────────────────────────────────────────────

fn validate_invariant_names<I>(invariants: &[Invariant<I>]) -> Result<(), RunnerError> {
    for inv in invariants {
        if !is_stable_token(inv.name) {
            return Err(RunnerError::InvalidToken {
                field: "invariant name",
                value: inv.name.to_owned(),
            });
        }
    }
    for (i, a) in invariants.iter().enumerate() {
        for b in &invariants[i + 1..] {
            if a.name == b.name {
                return Err(RunnerError::DuplicateCheckName { name: a.name });
            }
        }
    }
    Ok(())
}

fn validate_metamorphic_names<I, O>(cases: &[MetamorphicCase<I, O>]) -> Result<(), RunnerError> {
    for c in cases {
        if !is_stable_token(c.name) {
            return Err(RunnerError::InvalidToken {
                field: "relation name",
                value: c.name.to_owned(),
            });
        }
    }
    for (i, a) in cases.iter().enumerate() {
        for b in &cases[i + 1..] {
            if a.name == b.name {
                return Err(RunnerError::DuplicateCheckName { name: a.name });
            }
        }
    }
    Ok(())
}

/// The truncation marker appended to messages that exceed the byte limit.
/// It is 3 bytes in UTF-8 and is counted toward the limit.
const TRUNCATION_MARKER: &str = "…";

/// Applies resource limits to a failure message (truncates at UTF-8 boundary).
///
/// The resulting string is guaranteed to be at most `max_failure_message_bytes`
/// bytes long.  When the cap is large enough to fit the 3-byte truncation
/// marker `…` (`cap >= TRUNCATION_MARKER.len()`), the marker is appended
/// after truncating the content to make room for it (for `cap ==
/// TRUNCATION_MARKER.len()` the content is empty and the result is exactly
/// the marker).  When the cap is too small to fit the marker at all
/// (`cap < TRUNCATION_MARKER.len()`), the message is truncated to exactly
/// `cap` bytes (at the nearest valid UTF-8 boundary) with no marker appended.
/// `max_failure_message_bytes == 0` means unlimited; the message is returned
/// unchanged.
fn apply_message_limit(msg: String, limits: &ResourceLimits) -> String {
    let Some(cap) = limits.max_failure_message_bytes else {
        return msg;
    };
    if msg.len() <= cap {
        return msg;
    }
    if cap < TRUNCATION_MARKER.len() {
        let mut end = cap;
        while end > 0 && !msg.is_char_boundary(end) {
            end -= 1;
        }
        return msg[..end].to_owned();
    }
    let budget = cap - TRUNCATION_MARKER.len();
    // Find the largest valid UTF-8 boundary within the budget.
    let mut end = budget;
    while end > 0 && !msg.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{TRUNCATION_MARKER}", &msg[..end])
}

/// Serialises `input` once, returning the canonical [`serde_json::Value`] and
/// the byte count of its compact JSON representation.
///
/// Using one call for both the byte-accounting path (`max_total_input_bytes`)
/// and the stored-failure path (`CaseFailure::input_json`) avoids
/// reserialization drift: the count and the stored value always come from
/// the same serialization.
fn serialize_input<I: serde::Serialize>(
    input: &I,
) -> Result<(serde_json::Value, u64), RunnerError> {
    let value =
        serde_json::to_value(input).map_err(|e| RunnerError::InputEncoding(e.to_string()))?;
    let bytes =
        serde_json::to_vec(&value).map_err(|e| RunnerError::InputEncoding(e.to_string()))?;
    Ok((value, bytes.len() as u64))
}

/// Unified replay-identity validation called by every runner before any input
/// is generated.
///
/// When a [`ReplayFilter`] is installed the runner must not proceed if:
///
/// 1. The filter's `operation` does not match the [`RunConfig`]'s expected
///    operation — returns [`RunnerError::OperationMismatch`].
/// 2. The filter's `check_kind` does not match the kind handled by this
///    runner — returns [`RunnerError::CheckKindMismatch`].
/// 3. The filter's `check_name` is not present among the registered
///    invariants/relations — returns [`RunnerError::ReplayMismatch`].
///
/// All three checks fire *before* the first case is generated so that typos
/// and wrong-runner mistakes fail loudly rather than producing an
/// empty/vacuous run.
fn validate_replay_identity(
    config: &RunConfig,
    expected_check_kind: CheckKind,
    names_exist: impl FnOnce(&str) -> bool,
) -> Result<(), RunnerError> {
    let Some(ref filter) = config.replay else {
        return Ok(());
    };
    if filter.operation != config.operation {
        return Err(RunnerError::OperationMismatch {
            expected: config.operation.to_owned(),
            found: filter.operation.clone(),
        });
    }
    if filter.check_kind != expected_check_kind {
        return Err(RunnerError::CheckKindMismatch {
            expected: expected_check_kind,
            found: filter.check_kind,
        });
    }
    if !names_exist(&filter.check_name) {
        return Err(RunnerError::ReplayMismatch {
            name: filter.check_name.clone(),
        });
    }
    Ok(())
}

fn make_failure_identity(
    operation: &str,
    stream_name: &str,
    seed: TestSeed,
    case_index: u64,
    check_kind: CheckKind,
    check_name: &str,
) -> crate::report::ReplayIdentity {
    crate::report::ReplayIdentity::new(
        operation,
        stream_name,
        seed,
        case_index,
        check_kind,
        check_name,
    )
    .expect("runner-validated fields are always stable tokens")
}

// ── Runners ────────────────────────────────────────────────────────────────

/// Shared inner loop for [`run_invariant_cases`] and [`run_property_cases`].
///
/// Parameterised by `check_kind` so that the correct [`CheckKind`] is stored
/// in every [`CaseFailure`] and replay-identity validation fires *before* the
/// first input is generated — without any post-hoc relabelling that would
/// incorrectly accept a mismatched replay filter.
#[allow(clippy::too_many_lines)]
fn run_invariant_property_inner<I, G>(
    config: &RunConfig,
    check_kind: CheckKind,
    mut generator: G,
    invariants: &[Invariant<I>],
    mut minimizer: Option<&mut Minimizer<I>>,
) -> Result<RunReport, RunnerError>
where
    I: serde::Serialize + Clone,
    G: FnMut(&mut CaseContext) -> Result<I, ResourceLimitKind>,
{
    validate_invariant_names(invariants)?;
    validate_replay_identity(config, check_kind, |name| {
        invariants.iter().any(|inv| inv.name == name)
    })?;

    let stream_seed = config.seed.for_case_stream(config.stream_name);
    let mut failures: Vec<CaseFailure> = Vec::new();
    let mut passed: u64 = 0;
    let mut total: u64 = 0;
    let mut report_bytes: usize = 0;
    let mut input_bytes_so_far: u64 = 0;

    for i in case_iter(config) {
        if let Some(hit) = check_case_count_limit(total, &config.limits) {
            return Ok(RunReport {
                total_cases: total,
                passed_cases: passed,
                failures,
                resource_limit_hit: Some(hit),
            });
        }
        total += 1;

        let case_id = CaseId::new(stream_seed, i);
        let case_rng = derive_case_rng(config.seed, config.stream_name, i);
        let case_budget = CaseBudget::from_limits(&config.limits);
        let mut ctx = CaseContext::new(i, case_rng, case_budget);

        let input = match generator(&mut ctx) {
            Ok(v) => v,
            Err(limit_kind) => {
                return Ok(RunReport {
                    total_cases: total,
                    passed_cases: passed,
                    failures,
                    resource_limit_hit: Some(limit_kind),
                });
            }
        };

        // Serialise once (eagerly) only when `max_total_input_bytes` is active,
        // so the byte count and the stored JSON value always come from the same
        // serialisation call (no drift).  For passing cases when the limit is
        // not set we avoid the serialisation cost entirely; we serialise lazily
        // inside the failure-recording block below.
        let current_serialized: Option<(serde_json::Value, u64)> =
            if config.limits.max_total_input_bytes.is_some() {
                Some(serialize_input(&input)?)
            } else {
                None
            };

        if let Some(max) = config.limits.max_total_input_bytes {
            let input_bytes = current_serialized.as_ref().map_or(0, |&(_, b)| b);
            input_bytes_so_far = match input_bytes_so_far.checked_add(input_bytes) {
                Some(n) if n <= max => n,
                _ => {
                    return Ok(RunReport {
                        total_cases: total,
                        passed_cases: passed,
                        failures,
                        resource_limit_hit: Some(ResourceLimitKind::MaxTotalInputBytes),
                    });
                }
            };
        }
        let mut case_passed = true;

        for invariant in invariants {
            if skip_check(config, invariant.name) {
                continue;
            }
            if let Err(limit_kind) = ctx.charge_oracle() {
                return Ok(RunReport {
                    total_cases: total,
                    passed_cases: passed,
                    failures,
                    resource_limit_hit: Some(limit_kind),
                });
            }
            match invariant.apply(&mut ctx, &input) {
                Ok(()) => {}
                Err(CaseCheckError::ResourceExhausted(limit_kind)) => {
                    return Ok(RunReport {
                        total_cases: total,
                        passed_cases: passed,
                        failures,
                        resource_limit_hit: Some(limit_kind),
                    });
                }
                Err(CaseCheckError::Failure(raw_msg)) => {
                    let mut failure_input = input.clone();
                    let mut failure_message = raw_msg;
                    // Track the serialised form: reuse the eagerly-computed
                    // value when available (`max_total_input_bytes` was set),
                    // or serialise lazily once minimization is complete.
                    let mut failure_serialized: Option<(serde_json::Value, u64)> =
                        current_serialized.clone();

                    if let Some(ref mut minimizer) = minimizer {
                        'minimize: loop {
                            match ctx.charge_minimization_step() {
                                Ok(()) => {}
                                Err(limit) => {
                                    return Ok(RunReport {
                                        total_cases: total,
                                        passed_cases: passed,
                                        failures,
                                        resource_limit_hit: Some(limit),
                                    });
                                }
                            }
                            match minimizer.next_candidate(&mut ctx, &failure_input) {
                                Err(CaseCheckError::ResourceExhausted(limit)) => {
                                    return Ok(RunReport {
                                        total_cases: total,
                                        passed_cases: passed,
                                        failures,
                                        resource_limit_hit: Some(limit),
                                    });
                                }
                                Ok(None) | Err(CaseCheckError::Failure(_)) => break 'minimize,
                                Ok(Some(candidate)) => {
                                    // Serialise every candidate: the byte count is needed for
                                    // max_total_input_bytes (if set) and the JSON value is
                                    // retained when the candidate preserves the failure.
                                    let (cand_value, cand_bytes) = serialize_input(&candidate)?;

                                    if let Some(max_bytes) = config.limits.max_total_input_bytes {
                                        input_bytes_so_far =
                                            match input_bytes_so_far.checked_add(cand_bytes) {
                                                Some(n) if n <= max_bytes => n,
                                                _ => {
                                                    return Ok(RunReport {
                                                        total_cases: total,
                                                        passed_cases: passed,
                                                        failures,
                                                        resource_limit_hit: Some(
                                                            ResourceLimitKind::MaxTotalInputBytes,
                                                        ),
                                                    });
                                                }
                                            };
                                    }

                                    match ctx.charge_oracle() {
                                        Ok(()) => {}
                                        Err(limit) => {
                                            return Ok(RunReport {
                                                total_cases: total,
                                                passed_cases: passed,
                                                failures,
                                                resource_limit_hit: Some(limit),
                                            });
                                        }
                                    }

                                    match invariant.apply(&mut ctx, &candidate) {
                                        Err(CaseCheckError::Failure(msg)) => {
                                            // Candidate preserves the failure — shrink.
                                            failure_input = candidate;
                                            failure_message = msg;
                                            // Cache the serialisation we already computed.
                                            failure_serialized = Some((cand_value, cand_bytes));
                                        }
                                        Ok(()) => {}
                                        Err(CaseCheckError::ResourceExhausted(limit)) => {
                                            return Ok(RunReport {
                                                total_cases: total,
                                                passed_cases: passed,
                                                failures,
                                                resource_limit_hit: Some(limit),
                                            });
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Resolve the final input JSON: reuse the cached serialised
                    // value (from eager or minimizer path) or serialise lazily now.
                    let (final_input_json, input_json_bytes) = match failure_serialized {
                        Some((v, b)) => (v, b),
                        None => serialize_input(&failure_input)?,
                    };

                    let message = apply_message_limit(failure_message, &config.limits);
                    // Include input JSON bytes in report-byte accounting so that
                    // `max_total_report_bytes` can prevent retaining oversized artifacts.
                    let input_bytes_for_report =
                        usize::try_from(input_json_bytes).unwrap_or(usize::MAX);
                    let incoming_for_report = message.len().saturating_add(input_bytes_for_report);

                    if let Some(hit) = check_push_limits(
                        &failures,
                        report_bytes,
                        incoming_for_report,
                        &config.limits,
                    ) {
                        return Ok(RunReport {
                            total_cases: total,
                            passed_cases: passed,
                            failures,
                            resource_limit_hit: Some(hit),
                        });
                    }
                    report_bytes = match report_bytes.checked_add(incoming_for_report) {
                        Some(next) => next,
                        None => {
                            return Ok(RunReport {
                                total_cases: total,
                                passed_cases: passed,
                                failures,
                                resource_limit_hit: Some(ResourceLimitKind::MaxTotalReportBytes),
                            });
                        }
                    };
                    let identity = make_failure_identity(
                        config.operation,
                        config.stream_name,
                        config.seed,
                        i,
                        check_kind,
                        invariant.name,
                    );
                    failures.push(CaseFailure {
                        case_id,
                        identity,
                        message,
                        input_json: final_input_json,
                    });
                    case_passed = false;
                    if config.stop_on_first_failure {
                        return Ok(RunReport {
                            total_cases: total,
                            passed_cases: passed,
                            failures,
                            resource_limit_hit: None,
                        });
                    }
                }
            }
        }
        if case_passed {
            passed += 1;
        }
    }

    Ok(RunReport {
        total_cases: total,
        passed_cases: passed,
        failures,
        resource_limit_hit: None,
    })
}

/// Generates cases and asserts every invariant holds for each.
///
/// Each case's inputs are derived in **O(1)** from `(seed, stream, index)`.
/// When [`RunConfig::with_replay`] is set the runner executes exactly the
/// requested case (and optionally one specific invariant).
///
/// # Errors
///
/// Returns [`RunnerError::InvalidToken`] when a stream or invariant name is
/// invalid, [`RunnerError::DuplicateCheckName`] when two invariants share a
/// name, [`RunnerError::OperationMismatch`] when the replay filter's operation
/// differs from the config, or [`RunnerError::ReplayMismatch`] when a replay
/// target name does not match any invariant — all validated before any cases
/// are generated.
pub fn run_invariant_cases<I, G>(
    config: &RunConfig,
    generator: G,
    invariants: &[Invariant<I>],
    minimizer: Option<&mut Minimizer<I>>,
) -> Result<RunReport, RunnerError>
where
    I: serde::Serialize + Clone,
    G: FnMut(&mut CaseContext) -> Result<I, ResourceLimitKind>,
{
    run_invariant_property_inner(
        config,
        CheckKind::Invariant,
        generator,
        invariants,
        minimizer,
    )
}

/// Generates cases and asserts the named property holds for each.
///
/// Uses a shared inner loop parameterised by `CheckKind::Property`, so
/// replay-identity validation fires before any case is generated and
/// [`CaseFailure`] records are stamped with [`crate::fuzz::CheckKind::Property`]
/// from the start — no post-hoc relabelling.
///
/// A [`ReplayFilter`] with `check_kind = CheckKind::Invariant` returns
/// [`RunnerError::CheckKindMismatch`] *before* any input is generated.
///
/// # Errors
///
/// See [`run_invariant_cases`].
pub fn run_property_cases<I, G>(
    config: &RunConfig,
    generator: G,
    property_name: &'static str,
    property: impl Fn(&mut CaseContext, &I) -> Result<(), CaseCheckError> + 'static,
    minimizer: Option<&mut Minimizer<I>>,
) -> Result<RunReport, RunnerError>
where
    I: serde::Serialize + Clone,
    G: FnMut(&mut CaseContext) -> Result<I, ResourceLimitKind>,
{
    let invariants = [Invariant::new(property_name, property)];
    run_invariant_property_inner(
        config,
        CheckKind::Property,
        generator,
        &invariants,
        minimizer,
    )
}

/// Generates cases and for each applies every metamorphic relation.
///
/// Each relation's transform RNG is derived **O(1)** from
/// `(seed, stream, relation_name, case_index)`.  Adding or reordering
/// relations does not change any other relation's draws.
///
/// # Errors
///
/// Returns [`RunnerError::InvalidToken`] or [`RunnerError::DuplicateCheckName`]
/// when relation names are invalid or duplicated,
/// [`RunnerError::OperationMismatch`] when the replay filter's operation
/// differs from the config, or [`RunnerError::ReplayMismatch`] when a replay
/// target name does not match any relation — all validated before any cases
/// are generated.
#[allow(clippy::too_many_lines)]
pub fn run_metamorphic_cases<I, O, G>(
    config: &RunConfig,
    mut generator: G,
    mut operation: impl FnMut(&mut CaseContext, &I) -> Result<O, ResourceLimitKind>,
    cases: &[MetamorphicCase<I, O>],
) -> Result<RunReport, RunnerError>
where
    I: serde::Serialize + Clone,
    G: FnMut(&mut CaseContext) -> Result<I, ResourceLimitKind>,
{
    validate_metamorphic_names(cases)?;
    validate_replay_identity(config, CheckKind::MetamorphicRelation, |name| {
        cases.iter().any(|c| c.name == name)
    })?;

    let stream_seed = config.seed.for_case_stream(config.stream_name);
    let mut failures: Vec<CaseFailure> = Vec::new();
    let mut passed: u64 = 0;
    let mut total: u64 = 0;
    let mut report_bytes: usize = 0;
    let mut input_bytes_so_far: u64 = 0;

    for i in case_iter(config) {
        if let Some(hit) = check_case_count_limit(total, &config.limits) {
            return Ok(RunReport {
                total_cases: total,
                passed_cases: passed,
                failures,
                resource_limit_hit: Some(hit),
            });
        }
        total += 1;

        let case_id = CaseId::new(stream_seed, i);
        let case_rng = derive_case_rng(config.seed, config.stream_name, i);
        let case_budget = CaseBudget::from_limits(&config.limits);
        let mut ctx = CaseContext::new(i, case_rng, case_budget);
        let base_input = match generator(&mut ctx) {
            Ok(v) => v,
            Err(limit_kind) => {
                return Ok(RunReport {
                    total_cases: total,
                    passed_cases: passed,
                    failures,
                    resource_limit_hit: Some(limit_kind),
                });
            }
        };

        // Serialise base_input once when needed; cache value for failure report.
        let base_serialized: Option<(serde_json::Value, u64)> =
            if config.limits.max_total_input_bytes.is_some() {
                Some(serialize_input(&base_input)?)
            } else {
                None
            };

        if let Some(max) = config.limits.max_total_input_bytes {
            let input_bytes = base_serialized.as_ref().map_or(0, |&(_, b)| b);
            input_bytes_so_far = match input_bytes_so_far.checked_add(input_bytes) {
                Some(n) if n <= max => n,
                _ => {
                    return Ok(RunReport {
                        total_cases: total,
                        passed_cases: passed,
                        failures,
                        resource_limit_hit: Some(ResourceLimitKind::MaxTotalInputBytes),
                    });
                }
            };
        }
        let base_output = match operation(&mut ctx, &base_input) {
            Ok(v) => v,
            Err(limit_kind) => {
                return Ok(RunReport {
                    total_cases: total,
                    passed_cases: passed,
                    failures,
                    resource_limit_hit: Some(limit_kind),
                });
            }
        };
        let mut case_passed = true;

        for meta in cases {
            if skip_check(config, meta.name) {
                continue;
            }
            let transform_rng = derive_transform_rng(config.seed, config.stream_name, meta.name, i);
            let transform_budget = CaseBudget::from_limits(&config.limits);
            let mut transform_ctx = CaseContext::new(i, transform_rng, transform_budget);
            let related_input = match (meta.transform)(&mut transform_ctx, &base_input) {
                Ok(v) => v,
                Err(limit_kind) => {
                    return Ok(RunReport {
                        total_cases: total,
                        passed_cases: passed,
                        failures,
                        resource_limit_hit: Some(limit_kind),
                    });
                }
            };
            if let Some(max) = config.limits.max_total_input_bytes {
                let (_, rel_bytes) = serialize_input(&related_input)?;
                input_bytes_so_far = match input_bytes_so_far.checked_add(rel_bytes) {
                    Some(n) if n <= max => n,
                    _ => {
                        return Ok(RunReport {
                            total_cases: total,
                            passed_cases: passed,
                            failures,
                            resource_limit_hit: Some(ResourceLimitKind::MaxTotalInputBytes),
                        });
                    }
                };
            }
            let related_output = match operation(&mut transform_ctx, &related_input) {
                Ok(v) => v,
                Err(limit_kind) => {
                    return Ok(RunReport {
                        total_cases: total,
                        passed_cases: passed,
                        failures,
                        resource_limit_hit: Some(limit_kind),
                    });
                }
            };

            if let Err(limit_kind) = ctx.charge_oracle() {
                return Ok(RunReport {
                    total_cases: total,
                    passed_cases: passed,
                    failures,
                    resource_limit_hit: Some(limit_kind),
                });
            }
            match (meta.relation)(
                &mut transform_ctx,
                &base_input,
                &base_output,
                &related_input,
                &related_output,
            ) {
                Ok(()) => {}
                Err(CaseCheckError::ResourceExhausted(limit_kind)) => {
                    return Ok(RunReport {
                        total_cases: total,
                        passed_cases: passed,
                        failures,
                        resource_limit_hit: Some(limit_kind),
                    });
                }
                Err(CaseCheckError::Failure(raw_msg)) => {
                    // Resolve base_input JSON: reuse cached or serialise lazily.
                    let (base_input_json, base_input_bytes) = match base_serialized.clone() {
                        Some((v, b)) => (v, b),
                        None => serialize_input(&base_input)?,
                    };
                    let message = apply_message_limit(raw_msg, &config.limits);
                    let input_bytes_for_report =
                        usize::try_from(base_input_bytes).unwrap_or(usize::MAX);
                    let incoming_for_report = message.len().saturating_add(input_bytes_for_report);
                    if let Some(hit) = check_push_limits(
                        &failures,
                        report_bytes,
                        incoming_for_report,
                        &config.limits,
                    ) {
                        return Ok(RunReport {
                            total_cases: total,
                            passed_cases: passed,
                            failures,
                            resource_limit_hit: Some(hit),
                        });
                    }
                    report_bytes = match report_bytes.checked_add(incoming_for_report) {
                        Some(next) => next,
                        None => {
                            return Ok(RunReport {
                                total_cases: total,
                                passed_cases: passed,
                                failures,
                                resource_limit_hit: Some(ResourceLimitKind::MaxTotalReportBytes),
                            });
                        }
                    };
                    let identity = make_failure_identity(
                        config.operation,
                        config.stream_name,
                        config.seed,
                        i,
                        CheckKind::MetamorphicRelation,
                        meta.name,
                    );
                    failures.push(CaseFailure {
                        case_id,
                        identity,
                        message,
                        input_json: base_input_json,
                    });
                    case_passed = false;
                    if config.stop_on_first_failure {
                        return Ok(RunReport {
                            total_cases: total,
                            passed_cases: passed,
                            failures,
                            resource_limit_hit: None,
                        });
                    }
                }
            }
        }
        if case_passed {
            passed += 1;
        }
    }

    Ok(RunReport {
        total_cases: total,
        passed_cases: passed,
        failures,
        resource_limit_hit: None,
    })
}

// ── private runner helpers ─────────────────────────────────────────────────

/// An iterator over the case indices to execute for a given [`RunConfig`]:
/// either exactly one index (replay mode) or `0..iterations` (full run).
///
/// Using an iterator instead of a `(start, end)` index range avoids all
/// arithmetic on `case_index` (in particular `case_index + 1`, which would
/// overflow when replaying `case_index == u64::MAX`).
enum CaseIter {
    Replay(core::iter::Once<u64>),
    Full(core::ops::Range<u64>),
}

impl Iterator for CaseIter {
    type Item = u64;

    fn next(&mut self) -> Option<u64> {
        match self {
            Self::Replay(it) => it.next(),
            Self::Full(it) => it.next(),
        }
    }
}

/// Returns the case-index iterator for this config (see [`CaseIter`]).
fn case_iter(config: &RunConfig) -> CaseIter {
    if let Some(ref f) = config.replay {
        CaseIter::Replay(core::iter::once(f.case_index))
    } else {
        CaseIter::Full(0..u64::from(config.iterations))
    }
}

/// Returns `true` when `check_name` should be skipped given the replay filter.
fn skip_check(config: &RunConfig, check_name: &str) -> bool {
    config
        .replay
        .as_ref()
        .is_some_and(|filter| filter.check_name != check_name)
}

/// Returns `Some(kind)` when running one more case would exceed
/// [`ResourceLimits::max_cases_run`].
fn check_case_count_limit(
    cases_run_so_far: u64,
    limits: &ResourceLimits,
) -> Option<ResourceLimitKind> {
    if limits
        .max_cases_run
        .is_some_and(|max| cases_run_so_far >= u64::from(max))
    {
        Some(ResourceLimitKind::MaxCasesRun)
    } else {
        None
    }
}

/// Returns `Some(kind)` when recording one more failure (with `incoming_bytes`
/// message bytes) would exceed [`ResourceLimits::max_retained_failures`] or
/// [`ResourceLimits::max_total_report_bytes`].
///
/// Called *before* applying the message limit and pushing a new
/// [`CaseFailure`], using `checked_add` so that `current + incoming > cap` is
/// the rejection condition — not merely `current >= cap`.
fn check_push_limits(
    failures: &[CaseFailure],
    report_bytes_so_far: usize,
    incoming_bytes: usize,
    limits: &ResourceLimits,
) -> Option<ResourceLimitKind> {
    if limits
        .max_retained_failures
        .is_some_and(|max| failures.len() >= max)
    {
        return Some(ResourceLimitKind::MaxRetainedFailures);
    }
    if let Some(max) = limits.max_total_report_bytes {
        match report_bytes_so_far.checked_add(incoming_bytes) {
            Some(new_total) if new_total <= max => {}
            _ => return Some(ResourceLimitKind::MaxTotalReportBytes),
        }
    }
    None
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{
        CaseBudget, CaseCheckError, CaseContext, Invariant, MetamorphicCase, Minimizer,
        ReplayEnvError, ReplayFilter, ResourceLimitKind, ResourceLimits, RunConfig, RunnerError,
        apply_replay_config, check_push_limits, run_invariant_cases, run_metamorphic_cases,
        run_property_cases,
    };
    use crate::fuzz::{CheckKind, ReplayConfig};
    use crate::rng::{CASE_SEQUENCE_VERSION, TestRng, TestSeed};

    fn seed(v: u64) -> TestSeed {
        TestSeed::new(v)
    }

    fn cfg(v: u64, name: &'static str, op: &'static str, n: u32) -> RunConfig {
        RunConfig::new(seed(v), name, op, n).expect("valid config")
    }

    fn invariant_replay(case_index: u64, check_name: &str) -> ReplayFilter {
        ReplayFilter::new(
            case_index,
            "test.op".to_string(),
            CheckKind::Invariant,
            check_name.to_string(),
        )
    }

    fn metamorphic_replay(case_index: u64, check_name: &str) -> ReplayFilter {
        ReplayFilter::new(
            case_index,
            "test.op".to_string(),
            CheckKind::MetamorphicRelation,
            check_name.to_string(),
        )
    }

    #[test]
    fn case_sequence_version_is_3() {
        assert_eq!(CASE_SEQUENCE_VERSION, 3, "V3 versioned per-case derivation");
    }

    #[test]
    fn run_config_rejects_empty_stream_name() {
        assert!(matches!(
            RunConfig::new(seed(0), "", "test.op", 10),
            Err(RunnerError::InvalidToken {
                field: "stream_name",
                ..
            })
        ));
    }

    #[test]
    fn run_config_rejects_stream_name_with_space() {
        assert!(matches!(
            RunConfig::new(seed(0), "bad name", "test.op", 10),
            Err(RunnerError::InvalidToken {
                field: "stream_name",
                ..
            })
        ));
    }

    #[test]
    fn run_config_rejects_operation_with_space() {
        assert!(matches!(
            RunConfig::new(seed(0), "valid.stream", "bad op", 10),
            Err(RunnerError::InvalidToken {
                field: "operation",
                ..
            })
        ));
    }

    #[test]
    fn run_config_accepts_valid_tokens() {
        assert!(RunConfig::new(seed(0), "valid.stream-name_1:2/3", "test.op", 1).is_ok());
    }

    #[test]
    fn all_passing_invariants_produce_ok_report() {
        let config = cfg(1, "test.pass", "test.op", 100);
        let invariants = [Invariant::new("always.ok", |_ctx, _: &u64| Ok(()))];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &invariants, None).expect("valid");
        assert!(report.is_ok());
        assert_eq!(report.total_cases, 100);
        assert_eq!(report.passed_cases, 100);
    }

    #[test]
    fn failing_invariant_records_failure() {
        let config = cfg(2, "test.fail", "test.op", 50);
        let invariants = [Invariant::new("always.fails", |_ctx, _: &u64| {
            Err(CaseCheckError::Failure("intentional".to_string()))
        })];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &invariants, None).expect("valid");
        assert_eq!(report.failures.len(), 50);
        assert_eq!(report.passed_cases, 0);
        for f in &report.failures {
            assert_eq!(f.identity.check_name(), "always.fails");
            assert_eq!(f.identity.stream_name(), "test.fail");
        }
    }

    #[test]
    fn stop_on_first_failure_returns_one_failure() {
        let config = cfg(3, "test.stop", "test.op", 100).stop_on_first_failure();
        let invariants = [Invariant::new("always.fails", |_ctx, _: &u64| {
            Err(CaseCheckError::Failure("x".to_string()))
        })];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &invariants, None).expect("valid");
        assert_eq!(report.failures.len(), 1);
    }

    #[test]
    fn duplicate_check_names_are_rejected() {
        let config = cfg(0, "dup.test", "test.op", 10);
        let inv = [
            Invariant::new("check", |_ctx, _: &u64| Ok(())),
            Invariant::new("check", |_ctx, _: &u64| Ok(())),
        ];
        assert!(matches!(
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None),
            Err(RunnerError::DuplicateCheckName { name: "check" })
        ));
    }

    #[test]
    fn empty_check_name_is_rejected() {
        let config = cfg(0, "token.test", "test.op", 1);
        let inv = [Invariant::new("", |_ctx, _: &u64| Ok(()))];
        assert!(matches!(
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None),
            Err(RunnerError::InvalidToken {
                field: "invariant name",
                ..
            })
        ));
    }

    #[test]
    fn property_runner_reports_ok_for_true_property() {
        let config = cfg(5, "test.prop", "test.op", 200);
        let report = run_property_cases(
            &config,
            CaseContext::next_f64,
            "in_unit",
            |_ctx, v: &f64| {
                if (0.0..1.0).contains(v) {
                    Ok(())
                } else {
                    Err(CaseCheckError::Failure(format!("failed: {v}")))
                }
            },
            None,
        )
        .expect("valid");
        assert!(report.is_ok());
    }

    #[test]
    fn property_runner_reports_failure_for_false_property() {
        let config = cfg(6, "test.prop.fail", "test.op", 10);
        let report = run_property_cases(
            &config,
            CaseContext::next_u64,
            "never",
            |_ctx, _: &u64| Err(CaseCheckError::Failure("failed".to_string())),
            None,
        )
        .expect("valid");
        assert!(!report.is_ok());
        assert_eq!(report.failures.len(), 10);
    }

    #[test]
    fn property_callback_resource_exhausted_propagates_as_limit_hit() {
        let config = RunConfig::new(seed(77), "prop.resource.test", "test.op", 10)
            .unwrap()
            .with_resource_limits(ResourceLimits {
                max_work_units: Some(0),
                ..ResourceLimits::default()
            });
        let report = run_property_cases(
            &config,
            CaseContext::next_u64,
            "exhausting_prop",
            |ctx, _: &u64| {
                ctx.consume_work(1)?;
                Ok(())
            },
            None,
        )
        .unwrap();
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxWorkUnits),
            "ResourceExhausted from property must propagate as resource_limit_hit, not assertion failure"
        );
        assert!(
            report.failures.is_empty(),
            "resource exhaustion must not become a test failure"
        );
    }

    #[test]
    fn metamorphic_runner_passes_for_identity_transform() {
        let config = cfg(7, "test.meta", "test.op", 100);
        let cases = [MetamorphicCase::new(
            "identity",
            |_ctx, x: &u64| Ok(*x),
            |_ctx, _, o1: &u64, _, o2: &u64| {
                if o1 == o2 {
                    Ok(())
                } else {
                    Err(CaseCheckError::Failure(format!("{o1} != {o2}")))
                }
            },
        )];
        let report =
            run_metamorphic_cases(&config, CaseContext::next_u64, |_ctx, x| Ok(*x), &cases)
                .expect("valid");
        assert!(report.is_ok());
    }

    #[test]
    fn duplicate_relation_names_are_rejected() {
        let config = cfg(0, "dup.rel", "test.op", 10);
        let cases = [
            MetamorphicCase::new(
                "rel_a",
                |_ctx, x: &u64| Ok(*x),
                |_ctx, _, o1: &u64, _, o2: &u64| {
                    if o1 == o2 {
                        Ok(())
                    } else {
                        Err(CaseCheckError::Failure(String::new()))
                    }
                },
            ),
            MetamorphicCase::new(
                "rel_a",
                |_ctx, x: &u64| Ok(*x),
                |_ctx, _, o1: &u64, _, o2: &u64| {
                    if o1 == o2 {
                        Ok(())
                    } else {
                        Err(CaseCheckError::Failure(String::new()))
                    }
                },
            ),
        ];
        let err = run_metamorphic_cases(&config, CaseContext::next_u64, |_ctx, x| Ok(*x), &cases)
            .unwrap_err();
        assert!(matches!(
            err,
            RunnerError::DuplicateCheckName { name: "rel_a" }
        ));
    }

    #[test]
    fn per_case_rng_is_o1_independent_of_case_index() {
        // V2: case K's RNG must not change when we skip cases 0..K-1.
        let seed = seed(42);
        let stream = "test.replay";
        let idx: u64 = 9999;

        // Derive directly for case 9999.
        let mut direct_rng = super::super::rng::derive_case_rng(seed, stream, idx);
        let direct_val = direct_rng.next_u64();

        // Same derivation through the runner in replay mode.
        let config = RunConfig::new(seed, stream, "test.op", 10_000)
            .expect("valid config")
            .with_replay(invariant_replay(idx, "capture"));
        let invariants = [Invariant::new("capture", |_ctx, _v: &u64| {
            Ok(()) // just generate; no assertion
        })];
        // We need the generated value. Use a generator that captures.
        let mut cap: Vec<u64> = Vec::new();
        let _ = run_invariant_cases(
            &config,
            |ctx| {
                let v = ctx.next_u64()?;
                cap.push(v);
                Ok(v)
            },
            &invariants,
            None,
        )
        .expect("valid");
        assert_eq!(cap.len(), 1, "replay must run exactly one case");
        assert_eq!(
            cap[0], direct_val,
            "replay case {idx} must match direct O(1) derivation"
        );
    }

    #[test]
    fn replay_filter_runs_exactly_one_case() {
        let config =
            cfg(10, "test.replay.one", "test.op", 1000).with_replay(invariant_replay(500, "count"));
        let invariants = [Invariant::new("count", |_ctx, _: &u64| Ok(()))];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &invariants, None).expect("valid");
        assert_eq!(report.total_cases, 1, "replay runs one case");
    }

    #[test]
    fn replay_filter_with_check_name_skips_other_checks() {
        let config = cfg(11, "test.replay.check", "test.op", 10)
            .with_replay(invariant_replay(3, "target.check"));
        let invariants = [
            Invariant::new("other.check", |_ctx, _: &u64| Ok(())),
            Invariant::new("target.check", |_ctx, _: &u64| Ok(())),
        ];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &invariants, None).expect("valid");
        assert_eq!(report.total_cases, 1);
    }

    #[test]
    fn resource_limit_caps_retained_failures() {
        let config = cfg(12, "test.rlimit", "test.op", 100).with_resource_limits(ResourceLimits {
            max_retained_failures: Some(5),
            max_failure_message_bytes: None,
            ..ResourceLimits::default()
        });
        let invariants = [Invariant::new("always.fails", |_ctx, _: &u64| {
            Err(CaseCheckError::Failure("x".to_string()))
        })];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &invariants, None).expect("valid");
        assert_eq!(report.failures.len(), 5, "must be capped at 5");
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxRetainedFailures)
        );
    }

    #[test]
    fn resource_limit_truncates_long_messages() {
        let config = cfg(13, "test.msglen", "test.op", 5).with_resource_limits(ResourceLimits {
            max_retained_failures: None,
            max_failure_message_bytes: Some(10),
            ..ResourceLimits::default()
        });
        let invariants = [Invariant::new("long.msg", |_ctx, _: &u64| {
            Err(CaseCheckError::Failure("a".repeat(1000)))
        })];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &invariants, None).expect("valid");
        for f in &report.failures {
            assert!(
                f.message.len() <= 10 + 4, // 4 bytes for "…" (3-byte UTF-8 + some slack)
                "message too long: {} bytes",
                f.message.len()
            );
        }
    }

    #[test]
    fn metamorphic_per_relation_rng_is_independent_of_other_relations() {
        use std::sync::{Arc, Mutex};

        let seed_v = seed(99);
        let captures_single: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::new()));
        let captures_with_extra: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::new()));

        // Run with only rel_b.
        let config_a = RunConfig::new(seed_v, "meta.iso", "test.op", 20).expect("valid");
        {
            let cap = Arc::clone(&captures_single);
            let cases = [MetamorphicCase::new(
                "rel_b",
                move |ctx, _x: &u64| {
                    let v = ctx.next_u64()?;
                    cap.lock().unwrap().push(v);
                    Ok(v)
                },
                |_ctx, _, o1, _, o2| {
                    if o1 == o2 {
                        Ok(())
                    } else {
                        Err(CaseCheckError::Failure(String::new()))
                    }
                },
            )];
            let _ =
                run_metamorphic_cases(&config_a, CaseContext::next_u64, |_ctx, x| Ok(*x), &cases)
                    .expect("valid");
        }

        // Run with rel_a added BEFORE rel_b — must not change rel_b's draws.
        let config_b = RunConfig::new(seed_v, "meta.iso", "test.op", 20).expect("valid");
        {
            let cap = Arc::clone(&captures_with_extra);
            let cases = [
                MetamorphicCase::new(
                    "rel_a",
                    |ctx, _x: &u64| ctx.next_u64(),
                    |_ctx, _, o1, _, o2| {
                        if o1 == o2 {
                            Ok(())
                        } else {
                            Err(CaseCheckError::Failure(String::new()))
                        }
                    },
                ),
                MetamorphicCase::new(
                    "rel_b",
                    move |ctx, _x: &u64| {
                        let v = ctx.next_u64()?;
                        cap.lock().unwrap().push(v);
                        Ok(v)
                    },
                    |_ctx, _, o1, _, o2| {
                        if o1 == o2 {
                            Ok(())
                        } else {
                            Err(CaseCheckError::Failure(String::new()))
                        }
                    },
                ),
            ];
            let _ =
                run_metamorphic_cases(&config_b, CaseContext::next_u64, |_ctx, x| Ok(*x), &cases)
                    .expect("valid");
        }

        let single = captures_single.lock().unwrap().clone();
        let with_extra = captures_with_extra.lock().unwrap().clone();
        assert_eq!(
            single, with_extra,
            "rel_b's transform stream must be unaffected by adding rel_a"
        );
    }

    #[test]
    fn case_ids_are_deterministic_across_runs() {
        let stream = seed(4).for_stream("test.ids");
        let ids_a: Vec<_> = (0u64..10)
            .map(|i| crate::rng::CaseId::new(stream, i))
            .collect();
        let ids_b: Vec<_> = (0u64..10)
            .map(|i| crate::rng::CaseId::new(stream, i))
            .collect();
        assert_eq!(ids_a, ids_b, "case IDs must be reproducible");
    }

    #[test]
    fn run_report_total_cases_matches_iterations() {
        let config = cfg(20, "test.count", "test.op", 77);
        let inv = [Invariant::new("ok", |_ctx, _: &u64| Ok(()))];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None).expect("valid");
        assert_eq!(report.total_cases, 77);
    }

    #[test]
    fn stream_name_recorded_in_case_failure() {
        let config = cfg(21, "test.stream.record", "test.op", 3);
        let inv = [Invariant::new("fail", |_ctx, _: &u64| {
            Err(CaseCheckError::Failure("x".to_string()))
        })];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None).expect("valid");
        for f in &report.failures {
            assert_eq!(f.identity.stream_name(), "test.stream.record");
        }
    }

    #[test]
    fn identical_configs_produce_identical_case_sequences() {
        let config_a = cfg(777, "test.repro", "test.op", 50);
        let config_b = cfg(777, "test.repro", "test.op", 50);
        let inv = [Invariant::new("always.ok", |_ctx, _: &u64| Ok(()))];
        let mut vals_a: Vec<u64> = Vec::new();
        let mut vals_b: Vec<u64> = Vec::new();
        run_invariant_cases(
            &config_a,
            |ctx| {
                let v = ctx.next_u64()?;
                vals_a.push(v);
                Ok(v)
            },
            &inv,
            None,
        )
        .expect("valid");
        run_invariant_cases(
            &config_b,
            |ctx| {
                let v = ctx.next_u64()?;
                vals_b.push(v);
                Ok(v)
            },
            &inv,
            None,
        )
        .expect("valid");
        assert_eq!(
            vals_a, vals_b,
            "identical configs must produce identical sequences"
        );
    }

    // ── Issue 1: u64::MAX replay + ReplayMismatch ──────────────────────────

    #[test]
    fn replay_at_u64_max_case_index_does_not_panic() {
        let config = cfg(30, "test.replay.max", "test.op", 5)
            .with_replay(invariant_replay(u64::MAX, "always.ok"));
        let inv = [Invariant::new("always.ok", |_ctx, _: &u64| Ok(()))];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None).expect("valid");
        assert_eq!(report.total_cases, 1, "replay must run exactly one case");
        let cases: Vec<u64> = Vec::new();
        let _ = cases; // no arithmetic on the case index should have occurred
    }

    #[test]
    fn replay_at_u64_max_case_index_metamorphic_does_not_panic() {
        let config = cfg(31, "test.replay.max.meta", "test.op", 5)
            .with_replay(metamorphic_replay(u64::MAX, "identity"));
        let cases = [MetamorphicCase::new(
            "identity",
            |_ctx, x: &u64| Ok(*x),
            |_ctx, _, o1, _, o2| {
                if o1 == o2 {
                    Ok(())
                } else {
                    Err(CaseCheckError::Failure(String::new()))
                }
            },
        )];
        let report =
            run_metamorphic_cases(&config, CaseContext::next_u64, |_ctx, x| Ok(*x), &cases)
                .expect("valid");
        assert_eq!(report.total_cases, 1, "replay must run exactly one case");
    }

    #[test]
    fn replay_mismatch_rejected_for_invariants_before_generating_input() {
        let config = cfg(32, "test.replay.mismatch", "test.op", 5)
            .with_replay(invariant_replay(0, "does.not.exist"));
        let inv = [Invariant::new("real.check", |_ctx, _: &u64| Ok(()))];
        let mut generated = false;
        let result = run_invariant_cases(
            &config,
            |ctx| {
                generated = true;
                ctx.next_u64()
            },
            &inv,
            None,
        );
        assert!(
            matches!(
                result,
                Err(RunnerError::ReplayMismatch { ref name }) if name == "does.not.exist"
            ),
            "expected ReplayMismatch, got {result:?}"
        );
        assert!(!generated, "no input must be generated before validation");
    }

    #[test]
    fn replay_mismatch_rejected_for_metamorphic_before_generating_input() {
        let config = cfg(33, "test.replay.mismatch.meta", "test.op", 5)
            .with_replay(metamorphic_replay(0, "no.such.relation"));
        let cases = [MetamorphicCase::new(
            "real.relation",
            |_ctx, x: &u64| Ok(*x),
            |_ctx, _, o1, _, o2| {
                if o1 == o2 {
                    Ok(())
                } else {
                    Err(CaseCheckError::Failure(String::new()))
                }
            },
        )];
        let mut generated = false;
        let result = run_metamorphic_cases(
            &config,
            |ctx| {
                generated = true;
                ctx.next_u64()
            },
            |_ctx, x| Ok(*x),
            &cases,
        );
        assert!(
            matches!(
                result,
                Err(RunnerError::ReplayMismatch { ref name }) if name == "no.such.relation"
            ),
            "expected ReplayMismatch, got {result:?}"
        );
        assert!(!generated, "no input must be generated before validation");
    }

    #[test]
    fn replay_matching_check_name_runs_normally() {
        let config = cfg(34, "test.replay.match", "test.op", 5)
            .with_replay(invariant_replay(0, "real.check"));
        let inv = [Invariant::new("real.check", |_ctx, _: &u64| Ok(()))];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None).expect("valid");
        assert_eq!(report.total_cases, 1);
    }

    // ── Issue 2: replay-from-environment helpers ───────────────────────────

    #[test]
    fn apply_replay_config_overrides_seed_and_installs_filter() {
        let config = cfg(1, "e2e.fixture", "test.op", 100);
        let replay = ReplayConfig {
            case_sequence_version: CASE_SEQUENCE_VERSION,
            operation: "test.op".to_string(),
            seed: seed(999),
            case_index: 7,
            stream_name: "e2e.fixture".to_string(),
            check_kind: CheckKind::Invariant,
            check_name: "always.ok".to_string(),
        };
        let replayed = apply_replay_config(config, &replay).expect("matching stream");
        assert_eq!(replayed.seed(), seed(999));
        assert_eq!(
            replayed.replay().map(|f| f.case_index),
            Some(7),
            "replay filter must select the requested case"
        );

        let inv = [Invariant::new("always.ok", |_ctx, _: &u64| Ok(()))];
        let report =
            run_invariant_cases(&replayed, CaseContext::next_u64, &inv, None).expect("valid");
        assert_eq!(report.total_cases, 1, "must run exactly one replayed case");
    }

    #[test]
    fn apply_replay_config_rejects_stream_mismatch() {
        let config = cfg(1, "correct.stream", "test.op", 100);
        let replay = ReplayConfig {
            case_sequence_version: CASE_SEQUENCE_VERSION,
            operation: "test.op".to_string(),
            seed: seed(1),
            case_index: 0,
            stream_name: "different.stream".to_string(),
            check_kind: CheckKind::Invariant,
            check_name: "always.ok".to_string(),
        };
        assert!(
            matches!(
                apply_replay_config(config, &replay),
                Err(ReplayEnvError::StreamMismatch { .. })
            ),
            "stream mismatch must be detected"
        );
    }

    #[test]
    fn plain_run_config_new_rejects_wrong_operation_in_replay() {
        let config = RunConfig::new(seed(1), "some.stream", "correct.op", 10).unwrap();
        let replay = ReplayConfig {
            case_sequence_version: CASE_SEQUENCE_VERSION,
            operation: "wrong.op".to_string(),
            seed: seed(1),
            case_index: 0,
            stream_name: "some.stream".to_string(),
            check_kind: CheckKind::Invariant,
            check_name: "check".to_string(),
        };
        assert!(matches!(
            apply_replay_config(config, &replay),
            Err(ReplayEnvError::OperationMismatch { .. })
        ));
    }

    #[test]
    fn apply_replay_config_installs_check_name_from_replay() {
        let config = cfg(1, "stream.check", "op.check", 100);
        let replay = ReplayConfig {
            case_sequence_version: CASE_SEQUENCE_VERSION,
            operation: "op.check".to_string(),
            seed: seed(1),
            case_index: 5,
            stream_name: "stream.check".to_string(),
            check_kind: crate::fuzz::CheckKind::Invariant,
            check_name: "my.invariant".to_string(),
        };
        let replayed = apply_replay_config(config, &replay).expect("ok");
        assert_eq!(
            replayed.replay().map(|f| f.check_name.as_str()),
            Some("my.invariant")
        );
    }

    // ── Issue 4/5: resource limit enforcement ──────────────────────────────

    #[test]
    fn failure_limit_enforced_before_push_within_single_case() {
        // A single case with two failing invariants must not push more than
        // `max_retained_failures` failures, even though both invariants fail
        // within the *same* case (before the old per-case check would fire).
        let config =
            cfg(40, "test.push.order", "test.op", 5).with_resource_limits(ResourceLimits {
                max_retained_failures: Some(1),
                ..ResourceLimits::default()
            });
        let inv = [
            Invariant::new("fail.a", |_ctx, _: &u64| {
                Err(CaseCheckError::Failure("a".to_string()))
            }),
            Invariant::new("fail.b", |_ctx, _: &u64| {
                Err(CaseCheckError::Failure("b".to_string()))
            }),
        ];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None).expect("valid");
        assert_eq!(
            report.failures.len(),
            1,
            "must stop at exactly max_retained_failures, even within one case"
        );
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxRetainedFailures)
        );
    }

    #[test]
    fn max_cases_run_stops_after_configured_count() {
        let config =
            cfg(41, "test.max.cases", "test.op", 1000).with_resource_limits(ResourceLimits {
                max_cases_run: Some(7),
                ..ResourceLimits::default()
            });
        let inv = [Invariant::new("always.ok", |_ctx, _: &u64| Ok(()))];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None).expect("valid");
        assert_eq!(report.total_cases, 7);
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxCasesRun)
        );
    }

    #[test]
    fn max_cases_run_none_means_unlimited() {
        let config = cfg(42, "test.max.cases.unlimited", "test.op", 25).with_resource_limits(
            ResourceLimits {
                max_cases_run: None,
                ..ResourceLimits::default()
            },
        );
        let inv = [Invariant::new("always.ok", |_ctx, _: &u64| Ok(()))];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None).expect("valid");
        assert_eq!(report.total_cases, 25);
        assert_eq!(report.resource_limit_hit, None);
    }

    #[test]
    fn max_cases_run_enforced_for_metamorphic_runner() {
        let config =
            cfg(43, "test.max.cases.meta", "test.op", 1000).with_resource_limits(ResourceLimits {
                max_cases_run: Some(4),
                ..ResourceLimits::default()
            });
        let cases = [MetamorphicCase::new(
            "identity",
            |_ctx, x: &u64| Ok(*x),
            |_ctx, _, o1, _, o2| {
                if o1 == o2 {
                    Ok(())
                } else {
                    Err(CaseCheckError::Failure(String::new()))
                }
            },
        )];
        let report =
            run_metamorphic_cases(&config, CaseContext::next_u64, |_ctx, x| Ok(*x), &cases)
                .expect("valid");
        assert_eq!(report.total_cases, 4);
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxCasesRun)
        );
    }

    #[test]
    fn max_total_report_bytes_stops_further_failures() {
        // Generator returns 0u64 → JSON "0" = 1 byte.
        // Message is 10 bytes.  incoming_for_report = 10 + 1 = 11.
        // Cap = 11: first failure (0+11=11 ≤ 11) retained; second (11+11=22 > 11) rejected.
        let config =
            cfg(44, "test.max.bytes", "test.op", 1000).with_resource_limits(ResourceLimits {
                max_total_report_bytes: Some(11),
                ..ResourceLimits::default()
            });
        let inv = [Invariant::new("always.fails", |_ctx, _: &u64| {
            Err(CaseCheckError::Failure("a".repeat(10)))
        })];
        let report = run_invariant_cases(
            &config,
            |_ctx: &mut CaseContext| Ok::<u64, ResourceLimitKind>(0),
            &inv,
            None,
        )
        .expect("valid");
        // Only one 11-byte failure retained; second push (11+11=22 > 11) is rejected.
        assert_eq!(
            report.failures.len(),
            1,
            "checked_add fix: second push exceeds cap → exactly 1 failure retained"
        );
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxTotalReportBytes)
        );
    }

    #[test]
    fn max_total_report_bytes_none_means_unlimited() {
        let config = cfg(45, "test.max.bytes.unlimited", "test.op", 20).with_resource_limits(
            ResourceLimits {
                max_total_report_bytes: None,
                ..ResourceLimits::default()
            },
        );
        let inv = [Invariant::new("always.fails", |_ctx, _: &u64| {
            Err(CaseCheckError::Failure("a".repeat(10)))
        })];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None).expect("valid");
        assert_eq!(report.failures.len(), 20);
        assert_eq!(report.resource_limit_hit, None);
    }

    #[test]
    fn max_rng_draws_stops_generator() {
        let config =
            cfg(60, "test.draws.zero", "test.op", 10).with_resource_limits(ResourceLimits {
                max_rng_draws_per_case: Some(0),
                ..ResourceLimits::default()
            });
        let inv = [Invariant::new("ok", |_ctx, _: &u64| Ok(()))];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None).expect("valid");
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxRngDrawsPerCase)
        );
        assert_eq!(report.total_cases, 1, "one case attempted before limit hit");
    }

    #[test]
    fn max_oracle_calls_zero_stops_before_first_invariant() {
        let config =
            cfg(61, "test.oracle.zero", "test.op", 10).with_resource_limits(ResourceLimits {
                max_oracle_calls: Some(0),
                ..ResourceLimits::default()
            });
        let inv = [Invariant::new("should.not.run", |_ctx, _: &u64| {
            panic!("invariant must not be called with zero oracle budget")
        })];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None).expect("valid");
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxOracleCalls)
        );
    }

    #[test]
    fn max_oracle_calls_n_allows_n_and_stops_at_n_plus_1() {
        let config = cfg(62, "test.oracle.n", "test.op", 5).with_resource_limits(ResourceLimits {
            max_oracle_calls: Some(3),
            ..ResourceLimits::default()
        });
        let inv: Vec<Invariant<u64>> = (0..5)
            .map(|i| Invariant::new(["a", "b", "c", "d", "e"][i], |_ctx, _: &u64| Ok(())))
            .collect();
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None).expect("valid");
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxOracleCalls)
        );
    }

    #[test]
    fn max_total_input_bytes_zero_rejects_first_nonempty_input() {
        let config =
            cfg(63, "test.input.bytes", "test.op", 10).with_resource_limits(ResourceLimits {
                max_total_input_bytes: Some(0),
                ..ResourceLimits::default()
            });
        let inv = [Invariant::new("ok", |_ctx, _: &u64| Ok(()))];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None).expect("valid");
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxTotalInputBytes)
        );
    }

    #[test]
    fn max_total_input_bytes_stops_after_accumulation_exceeds_cap() {
        let config =
            cfg(64, "test.input.bytes.acc", "test.op", 1000).with_resource_limits(ResourceLimits {
                max_total_input_bytes: Some(3),
                ..ResourceLimits::default()
            });
        let inv = [Invariant::new("ok", |_ctx, _: &u64| Ok(()))];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None).expect("valid");
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxTotalInputBytes)
        );
        assert!(report.total_cases <= 10, "must stop quickly");
    }

    #[test]
    fn metamorphic_transform_rng_draws_are_charged_to_transform_budget() {
        let config =
            cfg(65, "test.transform.draws", "test.op", 5).with_resource_limits(ResourceLimits {
                max_rng_draws_per_case: Some(0),
                ..ResourceLimits::default()
            });
        let cases = [MetamorphicCase::new(
            "needs.draw",
            |ctx, x: &u64| {
                let _ = ctx.next_u64()?;
                Ok(*x)
            },
            |_ctx, _, o1, _, o2| {
                if o1 == o2 {
                    Ok(())
                } else {
                    Err(CaseCheckError::Failure(String::new()))
                }
            },
        )];
        let report = run_metamorphic_cases(&config, |_ctx| Ok(42u64), |_ctx, x| Ok(*x), &cases)
            .expect("valid");
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxRngDrawsPerCase)
        );
    }

    #[test]
    fn is_hard_enforced_rng_and_oracle_are_hard_enforced() {
        assert!(ResourceLimits::is_hard_enforced(
            ResourceLimitKind::MaxRetainedFailures
        ));
        assert!(ResourceLimits::is_hard_enforced(
            ResourceLimitKind::MaxCasesRun
        ));
        assert!(ResourceLimits::is_hard_enforced(
            ResourceLimitKind::MaxTotalReportBytes
        ));
        assert!(ResourceLimits::is_hard_enforced(
            ResourceLimitKind::MaxTotalInputBytes
        ));
        assert!(ResourceLimits::is_hard_enforced(
            ResourceLimitKind::MaxRngDrawsPerCase
        ));
        assert!(ResourceLimits::is_hard_enforced(
            ResourceLimitKind::MaxOracleCalls
        ));
        assert!(!ResourceLimits::is_hard_enforced(
            ResourceLimitKind::MaxWorkUnits
        ));
        assert!(!ResourceLimits::is_hard_enforced(
            ResourceLimitKind::MaxMinimizationSteps
        ));
    }

    // ── Issue 4: apply_message_limit tiny-cap matrix ───────────────────────

    #[test]
    fn apply_message_limit_cap_zero_returns_unchanged() {
        let config =
            cfg(50, "test.msglimit.zero", "test.op", 3).with_resource_limits(ResourceLimits {
                max_failure_message_bytes: None,
                ..ResourceLimits::default()
            });
        let inv = [Invariant::new("fail", |_ctx, _: &u64| {
            Err(CaseCheckError::Failure("hello world".to_string()))
        })];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None).expect("valid");
        for f in &report.failures {
            assert_eq!(
                f.message, "hello world",
                "cap=0 must leave message unchanged"
            );
        }
    }

    #[test]
    fn apply_message_limit_cap_one_has_no_marker() {
        let config =
            cfg(51, "test.msglimit.one", "test.op", 3).with_resource_limits(ResourceLimits {
                max_failure_message_bytes: Some(1),
                ..ResourceLimits::default()
            });
        let inv = [Invariant::new("fail", |_ctx, _: &u64| {
            Err(CaseCheckError::Failure("hello world".to_string()))
        })];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None).expect("valid");
        for f in &report.failures {
            assert_eq!(f.message.len(), 1, "cap=1 must produce exactly 1 byte");
            assert!(
                !f.message.contains('…'),
                "cap=1 must not contain the marker"
            );
        }
    }

    #[test]
    fn apply_message_limit_cap_two_has_no_marker() {
        let config =
            cfg(52, "test.msglimit.two", "test.op", 3).with_resource_limits(ResourceLimits {
                max_failure_message_bytes: Some(2),
                ..ResourceLimits::default()
            });
        let inv = [Invariant::new("fail", |_ctx, _: &u64| {
            Err(CaseCheckError::Failure("hello world".to_string()))
        })];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None).expect("valid");
        for f in &report.failures {
            assert!(
                f.message.len() <= 2,
                "cap=2 must produce at most 2 bytes: {}",
                f.message.len()
            );
            assert!(
                !f.message.contains('…'),
                "cap=2 must not contain the marker"
            );
        }
    }

    #[test]
    fn apply_message_limit_cap_three_is_exactly_the_marker() {
        let config =
            cfg(53, "test.msglimit.three", "test.op", 3).with_resource_limits(ResourceLimits {
                max_failure_message_bytes: Some(3),
                ..ResourceLimits::default()
            });
        let inv = [Invariant::new("fail", |_ctx, _: &u64| {
            Err(CaseCheckError::Failure("hello world".to_string()))
        })];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None).expect("valid");
        for f in &report.failures {
            assert_eq!(f.message, "…", "cap=3 must be exactly the marker");
        }
    }

    #[test]
    fn apply_message_limit_cap_ten_truncates_with_marker() {
        let config =
            cfg(54, "test.msglimit.ten", "test.op", 3).with_resource_limits(ResourceLimits {
                max_failure_message_bytes: Some(10),
                ..ResourceLimits::default()
            });
        let inv = [Invariant::new("fail", |_ctx, _: &u64| {
            Err(CaseCheckError::Failure("a".repeat(1000)))
        })];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, None).expect("valid");
        for f in &report.failures {
            assert!(
                f.message.len() <= 10,
                "message exceeds cap: {}",
                f.message.len()
            );
            assert!(f.message.ends_with('…'), "must be truncated with marker");
            assert!(f.message.len() > 3, "must retain some original content");
        }
    }

    #[test]
    fn minimizer_step_exhaustion_returns_structured_report_not_silent_break() {
        use std::{cell::Cell, rc::Rc};

        let call_count = Rc::new(Cell::new(0usize));
        let counter = Rc::clone(&call_count);
        let mut minimizer = Minimizer::new(move |_ctx, input: &u64| {
            counter.set(counter.get() + 1);
            Ok(Some(input.saturating_sub(1)))
        });

        let config = RunConfig::new(seed(1), "test.min.steps", "test.op", 1)
            .unwrap()
            .with_resource_limits(ResourceLimits {
                max_minimization_steps: Some(3),
                ..ResourceLimits::default()
            });
        let inv = [Invariant::new("always.fail", |_ctx, _: &u64| {
            Err(CaseCheckError::Failure("fail".to_string()))
        })];
        let report = run_invariant_cases(
            &config,
            |ctx| ctx.next_u64().map(|v| v + 100),
            &inv,
            Some(&mut minimizer),
        )
        .unwrap();
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxMinimizationSteps),
            "minimizer step exhaustion must yield structured resource_limit_hit"
        );
        assert_eq!(
            call_count.get(),
            3,
            "exactly max_minimization_steps callbacks must fire"
        );
    }

    #[test]
    fn minimizer_some_zero_invokes_zero_callbacks_returns_limit() {
        use std::{cell::Cell, rc::Rc};

        let call_count = Rc::new(Cell::new(0usize));
        let counter = Rc::clone(&call_count);
        let mut minimizer = Minimizer::new(move |_ctx, _input: &u64| {
            counter.set(counter.get() + 1);
            Ok(None)
        });

        let config = RunConfig::new(seed(2), "test.min.zero", "test.op", 1)
            .unwrap()
            .with_resource_limits(ResourceLimits {
                max_minimization_steps: Some(0),
                ..ResourceLimits::default()
            });
        let inv = [Invariant::new("fail", |_ctx, _: &u64| {
            Err(CaseCheckError::Failure("f".to_string()))
        })];
        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, Some(&mut minimizer))
                .unwrap();
        assert_eq!(
            call_count.get(),
            0,
            "zero minimization steps means zero callbacks"
        );
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxMinimizationSteps),
            "Some(0) must immediately return structured limit, not silent skip"
        );
    }

    #[test]
    fn minimizer_shrinks_to_smaller_failing_input() {
        let mut minimizer = Minimizer::new(|_ctx, input: &u64| {
            if *input > 5 {
                Ok(Some(input - 1))
            } else {
                Ok(None)
            }
        });

        let config =
            cfg(1, "test.minimize.shrink", "test.op", 1).with_resource_limits(ResourceLimits {
                max_minimization_steps: Some(100),
                ..ResourceLimits::default()
            });
        let inv = [Invariant::new("value.lt.5", |_ctx, input: &u64| {
            if *input < 5 {
                Ok(())
            } else {
                Err(CaseCheckError::Failure(format!("input {input} >= 5")))
            }
        })];
        let report =
            run_invariant_cases(&config, |_ctx| Ok(10u64), &inv, Some(&mut minimizer)).unwrap();
        assert!(!report.failures.is_empty(), "must have a failure");
        assert_eq!(report.failures[0].message, "input 5 >= 5");
    }

    #[test]
    fn minimizer_none_return_does_not_charge_oracle() {
        let mut minimizer = Minimizer::new(|_ctx, _: &u64| Ok(None));

        let config = RunConfig::new(seed(3), "test.min.none.oracle", "test.op", 1)
            .unwrap()
            .with_resource_limits(ResourceLimits {
                max_oracle_calls: Some(2),
                max_minimization_steps: Some(10),
                ..ResourceLimits::default()
            });
        let inv = [Invariant::new("fail", |_ctx, _: &u64| {
            Err(CaseCheckError::Failure("fail".to_string()))
        })];

        let report =
            run_invariant_cases(&config, CaseContext::next_u64, &inv, Some(&mut minimizer))
                .unwrap();

        assert_eq!(
            report.resource_limit_hit, None,
            "None from minimizer must not charge oracle"
        );
        assert!(!report.failures.is_empty());
    }

    #[test]
    fn minimizer_n_steps_n_plus_1_returns_structured_limit() {
        use std::{cell::Cell, rc::Rc};

        let call_count = Rc::new(Cell::new(0usize));
        let counter = Rc::clone(&call_count);
        let n = 5u64;
        let mut minimizer = Minimizer::new(move |_ctx, input: &u64| {
            counter.set(counter.get() + 1);
            Ok(Some(input.saturating_sub(1)))
        });

        let config = RunConfig::new(seed(4), "test.min.n", "test.op", 1)
            .unwrap()
            .with_resource_limits(ResourceLimits {
                max_minimization_steps: Some(n),
                ..ResourceLimits::default()
            });
        let inv = [Invariant::new("fail", |_ctx, _: &u64| {
            Err(CaseCheckError::Failure("f".to_string()))
        })];
        let report = run_invariant_cases(
            &config,
            |ctx| ctx.next_u64().map(|v| v + 50),
            &inv,
            Some(&mut minimizer),
        )
        .unwrap();
        assert_eq!(
            call_count.get(),
            usize::try_from(n).expect("small test constant"),
            "exactly n minimization callbacks"
        );
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxMinimizationSteps)
        );
    }

    #[test]
    fn check_push_limits_catches_overflow_near_usize_max() {
        let limits = ResourceLimits {
            max_total_report_bytes: Some(usize::MAX),
            ..ResourceLimits::default()
        };
        let failures = Vec::new();
        let result = check_push_limits(&failures, usize::MAX - 5, 10, &limits);
        assert_eq!(result, Some(ResourceLimitKind::MaxTotalReportBytes));
    }

    #[test]
    fn check_push_limits_some_0_rejects_first_byte() {
        let limits = ResourceLimits {
            max_total_report_bytes: Some(0),
            ..ResourceLimits::default()
        };
        let failures = Vec::new();
        let result = check_push_limits(&failures, 0, 1, &limits);
        assert_eq!(result, Some(ResourceLimitKind::MaxTotalReportBytes));
    }

    #[test]
    fn check_push_limits_exact_n_allows_n_n_plus_1_fails() {
        let n = 10usize;
        let limits = ResourceLimits {
            max_total_report_bytes: Some(n),
            ..ResourceLimits::default()
        };
        let failures = Vec::new();
        assert_eq!(check_push_limits(&failures, 0, n, &limits), None);
        assert_eq!(
            check_push_limits(&failures, 0, n + 1, &limits),
            Some(ResourceLimitKind::MaxTotalReportBytes)
        );
        assert_eq!(
            check_push_limits(&failures, n / 2, n / 2 + 1, &limits),
            Some(ResourceLimitKind::MaxTotalReportBytes)
        );
    }

    // ── CaseBudget tests ───────────────────────────────────────────────────

    #[test]
    fn case_budget_from_limits_none_is_unlimited() {
        let limits = ResourceLimits {
            max_rng_draws_per_case: None,
            max_work_units: None,
            max_oracle_calls: None,
            max_minimization_steps: None,
            ..ResourceLimits::default()
        };
        let mut budget = CaseBudget::from_limits(&limits);
        assert!(budget.charge_draw().is_ok());
        assert!(budget.charge_work(u64::MAX).is_ok());
        assert!(budget.charge_oracle().is_ok());
        assert!(budget.charge_minimization_step().is_ok());
    }

    #[test]
    fn case_budget_draw_exhaustion() {
        let limits = ResourceLimits {
            max_rng_draws_per_case: Some(2),
            ..ResourceLimits::default()
        };
        let mut budget = CaseBudget::from_limits(&limits);
        assert!(budget.charge_draw().is_ok()); // draw 1
        assert!(budget.charge_draw().is_ok()); // draw 2
        assert_eq!(
            budget.charge_draw(),
            Err(ResourceLimitKind::MaxRngDrawsPerCase)
        );
    }

    #[test]
    fn case_budget_oracle_exhaustion() {
        let limits = ResourceLimits {
            max_oracle_calls: Some(1),
            ..ResourceLimits::default()
        };
        let mut budget = CaseBudget::from_limits(&limits);
        assert!(budget.charge_oracle().is_ok());
        assert_eq!(
            budget.charge_oracle(),
            Err(ResourceLimitKind::MaxOracleCalls)
        );
    }

    #[test]
    fn case_budget_work_unit_exhaustion() {
        let limits = ResourceLimits {
            max_work_units: Some(5),
            ..ResourceLimits::default()
        };
        let mut budget = CaseBudget::from_limits(&limits);
        assert!(budget.charge_work(3).is_ok());
        assert_eq!(budget.charge_work(3), Err(ResourceLimitKind::MaxWorkUnits));
    }

    #[test]
    fn case_context_exposes_case_index() {
        let rng = TestRng::from_seed(crate::rng::TestSeed::new(1));
        let ctx = CaseContext::new(42, rng, CaseBudget::unlimited());
        assert_eq!(ctx.case_index, 42);
        assert!(ctx.draws_remaining().is_none());
    }
    #[test]
    fn case_check_error_resource_exhaustion_sets_resource_limit_hit() {
        let config = cfg(70, "test.casecheck.resource", "test.op", 5);
        let invariants = [Invariant::new("budgeted", |ctx, _: &u64| {
            ctx.consume_work(1)
                .map_err(CaseCheckError::ResourceExhausted)?;
            Ok(())
        })];
        let report = run_invariant_cases(
            &config.with_resource_limits(ResourceLimits {
                max_work_units: Some(0),
                ..ResourceLimits::default()
            }),
            CaseContext::next_u64,
            &invariants,
            None,
        )
        .expect("valid");
        assert!(report.failures.is_empty());
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxWorkUnits)
        );
    }

    #[test]
    fn generator_max_work_units_zero_stops_before_invariant_runs() {
        let config =
            cfg(70, "test.generator.work", "test.op", 1).with_resource_limits(ResourceLimits {
                max_work_units: Some(0),
                ..ResourceLimits::default()
            });
        let inv = [Invariant::new("must.not.run", |_ctx, _: &u64| {
            panic!("invariant must not run when generator exhausts work budget")
        })];
        let report = run_invariant_cases(
            &config,
            |ctx| {
                ctx.consume_work(1)?;
                Ok(1u64)
            },
            &inv,
            None,
        )
        .expect("valid");
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxWorkUnits)
        );
    }

    #[test]
    fn property_runner_failures_are_tagged_as_property() {
        let report = run_property_cases(
            &cfg(71, "test.property.kind", "test.op", 2),
            CaseContext::next_u64,
            "always_false",
            |_ctx, _: &u64| Err(CaseCheckError::Failure("failed".to_string())),
            None,
        )
        .expect("valid");
        assert!(
            report
                .failures
                .iter()
                .all(|f| f.identity.check_kind() == CheckKind::Property)
        );
    }

    #[test]
    fn input_encoding_error_is_reported() {
        #[derive(Debug, Clone)]
        struct BadInput;

        impl serde::Serialize for BadInput {
            fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                Err(serde::ser::Error::custom("encode failed"))
            }
        }

        let report = run_invariant_cases(
            &cfg(72, "test.input.encoding", "test.op", 1).with_resource_limits(ResourceLimits {
                max_total_input_bytes: Some(10),
                ..ResourceLimits::default()
            }),
            |_ctx| Ok(BadInput),
            &[Invariant::new("ok", |_ctx, _: &BadInput| Ok(()))],
            None,
        );
        assert!(
            matches!(report, Err(RunnerError::InputEncoding(msg)) if msg.contains("encode failed"))
        );
    }

    #[test]
    fn metamorphic_operation_receives_case_context() {
        let config =
            cfg(73, "test.meta.operation.ctx", "test.op", 1).with_resource_limits(ResourceLimits {
                max_rng_draws_per_case: Some(0),
                ..ResourceLimits::default()
            });
        let cases = [MetamorphicCase::new(
            "identity",
            |_ctx, x: &u64| Ok(*x),
            |_ctx, _, o1, _, o2| {
                if o1 == o2 {
                    Ok(())
                } else {
                    Err(CaseCheckError::Failure(String::new()))
                }
            },
        )];
        let report = run_metamorphic_cases(
            &config,
            |_ctx| Ok(1u64),
            |ctx, _| {
                let _ = ctx.next_u64()?;
                Ok(1u64)
            },
            &cases,
        )
        .expect("valid");
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxRngDrawsPerCase)
        );
    }

    #[test]
    fn metamorphic_relation_resource_exhaustion_sets_resource_limit_hit() {
        let config = cfg(74, "test.meta.relation.resource", "test.op", 1).with_resource_limits(
            ResourceLimits {
                max_work_units: Some(0),
                ..ResourceLimits::default()
            },
        );
        let cases = [MetamorphicCase::new(
            "budgeted.relation",
            |_ctx, x: &u64| Ok(*x),
            |ctx, _, _, _, _| {
                ctx.consume_work(1)
                    .map_err(CaseCheckError::ResourceExhausted)?;
                Ok(())
            },
        )];
        let report = run_metamorphic_cases(&config, |_ctx| Ok(1u64), |_ctx, x| Ok(*x), &cases)
            .expect("valid");
        assert!(report.failures.is_empty());
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxWorkUnits)
        );
    }

    #[test]
    fn resource_exhausted_invariant_does_not_record_failure() {
        let config = cfg(75, "test.casecheck.no-failure", "test.op", 1).with_resource_limits(
            ResourceLimits {
                max_work_units: Some(0),
                ..ResourceLimits::default()
            },
        );
        let report = run_invariant_cases(
            &config,
            |_ctx| Ok(1u64),
            &[Invariant::new("budgeted", |ctx, _: &u64| {
                ctx.consume_work(1)
                    .map_err(CaseCheckError::ResourceExhausted)?;
                Ok(())
            })],
            None,
        )
        .unwrap();
        assert_eq!(report.failures.len(), 0);
        assert_eq!(report.passed_cases, 0);
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxWorkUnits)
        );
    }

    // ── Replay identity / operation-mismatch tests ─────────────────────────

    fn property_replay(case_index: u64, check_name: &str) -> ReplayFilter {
        ReplayFilter::new(
            case_index,
            "test.op".to_string(),
            CheckKind::Property,
            check_name.to_string(),
        )
    }

    /// Direct replay: filter operation != config operation → `OperationMismatch`
    /// before any generation.
    #[test]
    fn direct_replay_wrong_operation_returns_operation_mismatch_invariant() {
        let filter = ReplayFilter::new(
            0,
            "different.op".to_string(),
            CheckKind::Invariant,
            "ok".to_string(),
        );
        let config = cfg(99, "test.op.mismatch", "test.op", 10).with_replay(filter);
        let inv = [Invariant::new("ok", |_ctx, _: &u64| Ok(()))];
        let mut gen_called = false;
        let err = run_invariant_cases(
            &config,
            |_ctx: &mut CaseContext| {
                gen_called = true;
                Ok::<u64, ResourceLimitKind>(0)
            },
            &inv,
            None,
        )
        .unwrap_err();
        assert!(
            !gen_called,
            "generator must not be called when operation mismatches"
        );
        assert!(
            matches!(err, RunnerError::OperationMismatch { ref expected, ref found }
                if expected == "test.op" && found == "different.op"),
            "expected OperationMismatch, got {err:?}"
        );
    }

    /// Direct replay wrong operation for metamorphic runner.
    #[test]
    fn direct_replay_wrong_operation_returns_operation_mismatch_metamorphic() {
        let filter = ReplayFilter::new(
            0,
            "other.op".to_string(),
            CheckKind::MetamorphicRelation,
            "rel".to_string(),
        );
        let config = cfg(100, "test.meta.op.mismatch", "test.op", 5).with_replay(filter);
        let cases = [MetamorphicCase::new(
            "rel",
            |_ctx, x: &u64| Ok(*x),
            |_ctx, _, _, _, _| Ok(()),
        )];
        let mut gen_called = false;
        let err = run_metamorphic_cases(
            &config,
            |_ctx: &mut CaseContext| {
                gen_called = true;
                Ok::<u64, ResourceLimitKind>(0)
            },
            |_ctx, x| Ok(*x),
            &cases,
        )
        .unwrap_err();
        assert!(
            !gen_called,
            "generator must not be called when operation mismatches"
        );
        assert!(
            matches!(err, RunnerError::OperationMismatch { .. }),
            "expected OperationMismatch, got {err:?}"
        );
    }

    /// `Property` `ReplayFilter` passed to invariant runner → `CheckKindMismatch`.
    #[test]
    fn property_replay_filter_on_invariant_runner_returns_check_kind_mismatch() {
        let filter = property_replay(0, "ok");
        let config = cfg(101, "test.crosskind1", "test.op", 5).with_replay(filter);
        let inv = [Invariant::new("ok", |_ctx, _: &u64| Ok(()))];
        let mut gen_called = false;
        let err = run_invariant_cases(
            &config,
            |_ctx: &mut CaseContext| {
                gen_called = true;
                Ok::<u64, ResourceLimitKind>(0)
            },
            &inv,
            None,
        )
        .unwrap_err();
        assert!(
            !gen_called,
            "generator must not be called on check-kind mismatch"
        );
        assert!(
            matches!(
                err,
                RunnerError::CheckKindMismatch {
                    expected: CheckKind::Invariant,
                    found: CheckKind::Property
                }
            ),
            "expected CheckKindMismatch, got {err:?}"
        );
    }

    /// `Invariant` `ReplayFilter` passed to property runner → `CheckKindMismatch`.
    #[test]
    fn invariant_replay_filter_on_property_runner_returns_check_kind_mismatch() {
        let filter = invariant_replay(0, "prop");
        let config = cfg(102, "test.crosskind2", "test.op", 5).with_replay(filter);
        let mut gen_called = false;
        let err = run_property_cases(
            &config,
            |_ctx: &mut CaseContext| {
                gen_called = true;
                Ok::<u64, ResourceLimitKind>(0)
            },
            "prop",
            |_ctx, _: &u64| Ok(()),
            None,
        )
        .unwrap_err();
        assert!(
            !gen_called,
            "generator must not be called on check-kind mismatch"
        );
        assert!(
            matches!(
                err,
                RunnerError::CheckKindMismatch {
                    expected: CheckKind::Property,
                    found: CheckKind::Invariant
                }
            ),
            "expected CheckKindMismatch, got {err:?}"
        );
    }

    /// `Property` `ReplayFilter` on property runner executes exactly one case.
    #[test]
    fn property_replay_filter_on_property_runner_runs_exactly_one_case() {
        let filter = property_replay(3, "my.prop");
        let config = cfg(103, "test.prop.replay", "test.op", 100).with_replay(filter);
        let report = run_property_cases(
            &config,
            CaseContext::next_u64,
            "my.prop",
            |_ctx, _: &u64| Ok(()),
            None,
        )
        .expect("valid");
        assert_eq!(report.total_cases, 1, "replay runs exactly one case");
        assert_eq!(report.passed_cases, 1);
        assert!(report.failures.is_empty());
    }

    // ── CaseFailure fields + artifact conversions ──────────────────────────

    /// Failures record operation and `input_json`.
    #[test]
    fn case_failure_stores_operation_and_input_json() {
        let config = cfg(200, "test.failure.fields", "my.operation", 5);
        let inv = [Invariant::new("always.fails", |_ctx, v: &u64| {
            Err(CaseCheckError::Failure(format!("bad: {v}")))
        })];
        let report = run_invariant_cases(
            &config,
            |_ctx: &mut CaseContext| Ok::<u64, ResourceLimitKind>(42),
            &inv,
            None,
        )
        .expect("valid");
        assert!(!report.failures.is_empty());
        let f = &report.failures[0];
        assert_eq!(f.identity.operation(), "my.operation");
        assert_eq!(f.identity.check_kind(), CheckKind::Invariant);
        assert_eq!(f.identity.check_name(), "always.fails");
        assert_eq!(f.input_json, serde_json::json!(42u64));
    }

    /// Property runner failure stores `CheckKind::Property` (no relabelling).
    #[test]
    fn property_runner_failure_stores_property_check_kind() {
        let config = cfg(201, "test.prop.kind", "test.op", 5);
        let report = run_property_cases(
            &config,
            |_ctx: &mut CaseContext| Ok::<u64, ResourceLimitKind>(1),
            "always.fails",
            |_ctx, _: &u64| Err(CaseCheckError::Failure("x".into())),
            None,
        )
        .expect("valid");
        assert!(!report.failures.is_empty());
        assert_eq!(
            report.failures[0].identity.check_kind(),
            CheckKind::Property,
            "property runner must store Property kind, not Invariant"
        );
    }

    /// Metamorphic runner failure stores correct operation and `input_json`.
    #[test]
    fn metamorphic_failure_stores_operation_and_input_json() {
        let config = cfg(202, "test.meta.fields", "meta.op", 5);
        let cases = [MetamorphicCase::new(
            "always.fails",
            |_ctx, x: &u64| Ok(*x),
            |_ctx, _, _, _, _| Err(CaseCheckError::Failure("fail".into())),
        )];
        let report =
            run_metamorphic_cases(&config, |_ctx| Ok(7u64), |_ctx, x| Ok(*x), &cases).unwrap();
        assert!(!report.failures.is_empty());
        let f = &report.failures[0];
        assert_eq!(f.identity.operation(), "meta.op");
        assert_eq!(f.identity.check_kind(), CheckKind::MetamorphicRelation);
        assert_eq!(f.input_json, serde_json::json!(7u64));
    }

    /// `to_failure_report` and `to_corpus_entry` round-trip through the stored
    /// identity and `input_json`.
    #[test]
    fn case_failure_to_failure_report_and_corpus_entry_round_trip() {
        let config = cfg(203, "test.artifact.roundtrip", "roundtrip.op", 5);
        let inv = [Invariant::new("rt.check", |_ctx, v: &u64| {
            Err(CaseCheckError::Failure(format!("v={v}")))
        })];
        let report = run_invariant_cases(
            &config,
            |_ctx: &mut CaseContext| Ok::<u64, ResourceLimitKind>(99),
            &inv,
            None,
        )
        .expect("valid");
        let failure = &report.failures[0];

        // to_failure_report
        let fr = failure.to_failure_report().expect("to_failure_report ok");
        assert_eq!(fr.operation(), failure.identity.operation());
        assert_eq!(fr.check_kind(), failure.identity.check_kind());
        assert_eq!(fr.check_name(), failure.identity.check_name());
        assert_eq!(fr.inputs_json(), &failure.input_json);

        // to_corpus_entry
        let entry = failure.to_corpus_entry().expect("to_corpus_entry ok");
        assert_eq!(entry.check_kind(), Some(failure.identity.check_kind()));
        assert_eq!(entry.check_name(), Some(failure.identity.check_name()));

        // to_reproducible_command
        let cmd = failure
            .to_reproducible_command("my-pkg", "my_test_fn")
            .expect("to_reproducible_command ok");
        let posix = cmd.to_string();
        assert!(posix.contains("my-pkg"), "POSIX command contains package");
        assert!(
            posix.contains("my_test_fn"),
            "POSIX command contains test name"
        );
        assert!(
            posix.contains("roundtrip.op"),
            "POSIX command contains operation"
        );
    }

    /// Input JSON bytes contribute to report-byte accounting.
    #[test]
    fn report_bytes_includes_input_json_bytes() {
        // input = 0u64 → JSON "0" = 1 byte; message = 10 bytes; total = 11.
        // cap = 11: first failure OK (11 ≤ 11); second rejected (22 > 11).
        let config = cfg(204, "test.report.bytes.json", "test.op", 100).with_resource_limits(
            ResourceLimits {
                max_total_report_bytes: Some(11),
                ..ResourceLimits::default()
            },
        );
        let inv = [Invariant::new("fails", |_ctx, _: &u64| {
            Err(CaseCheckError::Failure("a".repeat(10)))
        })];
        let report = run_invariant_cases(
            &config,
            |_ctx: &mut CaseContext| Ok::<u64, ResourceLimitKind>(0),
            &inv,
            None,
        )
        .expect("valid");
        assert_eq!(report.failures.len(), 1);
        assert_eq!(
            report.resource_limit_hit,
            Some(ResourceLimitKind::MaxTotalReportBytes)
        );
    }
}
