//! QA harness infrastructure for the Amphion B-Rep kernel.
//!
//! This crate provides all shared test-support infrastructure for kernel test
//! families: deterministic seed/replay, bounded distributions, edge-case
//! schedules, invariant/property/metamorphic runners, a permanent
//! regression-corpus format, differential-oracle registration, and actionable
//! failure reports.
//!
//! # Design principles
//!
//! - **Determinism**: every random source is seeded. No wall-clock, OS, or
//!   thread-local entropy is consulted.
//! - **Reproducibility**: a [`TestSeed`] plus a stream name uniquely
//!   determines an entire generation sequence. [`CaseId`] pins any single
//!   case within that sequence; the stream name is additionally stored in
//!   [`runner::CaseFailure`] and [`corpus::CorpusEntry`] for unambiguous
//!   replay.
//! - **Isolation**: [`TestSeed::for_stream`] derives independent streams per
//!   stream name. Collision probability is ≈ 1/2⁶⁴ per name pair (FNV-1a-64
//!   birthday bound); stream names must be documented in failure records.
//! - **Versioning**: [`CorpusFile`] carries an explicit schema version that is
//!   validated on every load. Incompatible files are rejected before any
//!   entry is inspected.
//! - **Composability**: runner primitives ([`Invariant`], [`MetamorphicCase`])
//!   are generic over input and output types and carry no geometry-specific
//!   dependency.
//! - **Actionability**: [`FailureReport`] and [`ReproducibleCommand`] contain
//!   every field a developer needs to understand and replay a failure from the
//!   command line.
//!
//! # Fuzz milestone
//!
//! [`RANDOMIZED_CASE_MILESTONE`] = 10,000 is the required randomised-case
//! count for the first kernel proof milestone. [`FuzzInputReader`] converts
//! raw fuzzer bytes into typed values without panicking even on empty input.
//!
//! # Integration with other crates
//!
//! Dependencies: `serde` and `serde_json` (both workspace-level) are required
//! for corpus file I/O and failure-report serialisation. Root `Cargo.lock`
//! integration: run `cargo build -p amphion-test-support` once after merging
//! to refresh the lockfile.

pub mod corpus;
pub mod distribution;
pub mod fuzz;
pub mod oracle;
pub mod report;
pub mod rng;
pub mod runner;

// ── Crate-internal token validation ───────────────────────────────────────

/// Returns `true` when `s` is a valid **stable token** for use as a stream
/// name, operation label, check/relation name, or oracle ID.
///
/// Stable tokens are nonempty strings whose bytes are all in
/// `[a-zA-Z0-9._:/-]`.  NUL bytes and ASCII control characters are rejected.
/// Spaces and most punctuation are rejected so that tokens can be embedded in
/// replay commands and corpus file names without ambiguity.
pub(crate) fn is_stable_token(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b':' | b'-' | b'/'))
}

/// Returns `true` when `s` is a valid **command token** for use as a cargo
/// package name or test function name in a [`crate::report::ReproducibleCommand`].
///
/// Command tokens must be nonempty, must not contain NUL (`\x00`), and must
/// not contain ASCII control characters (0x01–0x1F and 0x7F).  Spaces and
/// other printable characters are allowed; they will be shell-quoted by the
/// renderer.
pub(crate) fn is_command_token(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b != 0 && !b.is_ascii_control())
}

pub use corpus::{
    CORPUS_SCHEMA_VERSION, CorpusEntry, CorpusError, CorpusFile, LegacyCorpusDocument,
    MinimizationMeta,
};
pub use distribution::{
    BoundedFloat, BoundedUInt, DistributionError, EdgeCaseSchedule, ExplicitSchedule,
    WeightedChoice, WeightedItem,
};
pub use fuzz::{
    CheckKind, ENV_TEST_CASE, ENV_TEST_CHECK, ENV_TEST_CHECK_KIND, ENV_TEST_OPERATION,
    ENV_TEST_SEED, ENV_TEST_STREAM, ENV_TEST_VERSION, FuzzInputReader, RANDOMIZED_CASE_MILESTONE,
    ReplayConfig, ReplayConfigError, parse_replay_env,
};
pub use oracle::{
    DifferentialOracle, OracleClassification, OracleId, OracleIdError, OracleRegistrationError,
    OracleRegistry, OracleVerdict,
};
pub use report::{
    CommandTokenError, FailureReport, REPORT_SCHEMA_VERSION, ReplayIdentity, ReportError,
    ReproducibleCommand,
};
pub use rng::{CASE_SEQUENCE_VERSION, CaseId, CaseIdParseError, TestRng, TestSeed};
pub use runner::{
    CaseBudget, CaseCheckError, CaseContext, CaseFailure, Invariant, MetamorphicCase, Minimizer,
    ReplayEnvError, ReplayFilter, ResourceLimitKind, ResourceLimits, RunConfig, RunReport,
    RunnerError, apply_replay_config, configure_replay_from_env, run_invariant_cases,
    run_metamorphic_cases, run_property_cases,
};
