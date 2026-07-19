//! Actionable failure reports for test cases.
//!
//! A [`FailureReport`] contains every field required to understand, reproduce,
//! and track a failing test case:
//!
//! - `schema_version` identifies the persisted format (required by CONTRACTS.md
//!   for every serialised root).
//! - `seed`, `stream_name`, and `case_index` together unambiguously identify
//!   the failing case for replay (note: `stream_name` is stored separately from
//!   [`CaseId`] since the `CaseId` does not embed the stream name).
//! - `case_id` is a stable corpus-lookup key derived from
//!   `CaseId::new(seed.for_case_stream(stream_name), case_index)` (the V3
//!   versioned case-sequence formula; see [`crate::rng::CASE_SEQUENCE_VERSION`]).
//! - `case_sequence_version` records the [`crate::rng::CASE_SEQUENCE_VERSION`]
//!   in effect when the report was produced, validated on load so that
//!   reports built under a stale domain-key formula are rejected rather than
//!   silently mismatching.
//! - `operation`, `inputs_json` (a [`serde_json::Value`]), tolerance context
//!   (if applicable), and failure message give full reproduction context.
//! - A [`ReproducibleCommand`] provides a pasteable shell command.
//!
//! # Construction
//!
//! [`FailureReport::new`] is fallible: it validates that `stream_name` and
//! `operation` are non-empty stable tokens and that `case_id` is consistent
//! with `(seed, stream_name, case_index)`.
//!
//! [`ReproducibleCommand::new`] is also fallible: it rejects NUL bytes and
//! ASCII control characters in `package` and `test_name`, rejects a `package`
//! starting with `-` (which would be parsed as a Cargo flag), rejects a
//! `test_name` starting with `-` (which would be parsed as a libtest flag),
//! and validates `stream_name` as a stable token.
//!
//! # Loading
//!
//! The custom [`Deserialize`] implementation for [`FailureReport`] validates,
//! in order: `schema_version.major()` must equal
//! [`REPORT_SCHEMA_VERSION`]'s major (else rejected as an incompatible major
//! version), `schema_version.minor()` must not exceed
//! [`REPORT_SCHEMA_VERSION`]'s minor (a report from a newer minor version may
//! use fields this build does not understand), and `case_sequence_version`
//! must equal [`crate::rng::CASE_SEQUENCE_VERSION`] exactly.
//!
//! # Shell renderers
//!
//! [`ReproducibleCommand`] provides two renderers:
//!
//! - [`fmt::Display`] — POSIX (`VAR=val cmd args`). Suitable for bash/zsh on
//!   Unix/macOS. Not valid for Windows `cmd.exe`; NUL bytes are rejected at
//!   construction.
//! - [`ReproducibleCommand::as_powershell`] — PowerShell
//!   (`$env:VAR='val'; cmd args`). Suitable for Windows PowerShell and
//!   PowerShell Core.  Single-quoted values require stable tokens in the
//!   `stream_name`; the renderer asserts this.
//!
//! # Diagnostics
//!
//! [`FailureReport::with_diagnostics`] sorts and deduplicates diagnostics so
//! that the JSON output is byte-stable regardless of insertion order.
//! The custom `Deserialize` implementation validates that diagnostics remain
//! sorted on load.

use core::fmt;

use amphion_foundation::{Diagnostic, SchemaVersion, ToleranceContext};
use serde::{Deserialize, Serialize};

use crate::fuzz::{
    CheckKind, ENV_TEST_CASE, ENV_TEST_CHECK, ENV_TEST_CHECK_KIND, ENV_TEST_OPERATION,
    ENV_TEST_SEED, ENV_TEST_STREAM, ENV_TEST_VERSION,
};
use crate::is_command_token;
use crate::is_stable_token;
use crate::rng::{CASE_SEQUENCE_VERSION, CaseId, TestSeed};

/// The schema version of [`FailureReport`] produced by this harness build.
pub const REPORT_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0);

// ── shell escaping helpers ─────────────────────────────────────────────────

/// Returns a POSIX single-quoted `s` safe for shell insertion.
/// Tokens that are already shell-safe are returned unquoted.
fn posix_quote(s: &str) -> String {
    if is_stable_token(s) {
        return s.to_owned();
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for ch in s.chars() {
        if ch == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(ch);
        }
    }
    out.push('\'');
    out
}

/// Returns a PowerShell single-quoted value.
/// Since our stream names are stable tokens they cannot contain `'`.
fn ps_quote(s: &str) -> String {
    // For stable tokens: no single-quote inside → wrap in single quotes.
    // For other tokens: use double quotes and escape ", `, $.
    if is_stable_token(s) {
        return format!("'{s}'");
    }
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("`\""),
            '`' => out.push_str("``"),
            '$' => out.push_str("`$"),
            other => out.push(other),
        }
    }
    out.push('"');
    out
}

// ── CommandTokenError ──────────────────────────────────────────────────────

/// Error returned when a [`ReproducibleCommand`] field is invalid.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandTokenError {
    /// The field that failed validation.
    pub field: &'static str,
    /// A description of the validation failure.
    pub reason: &'static str,
}

impl fmt::Display for CommandTokenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid command token `{}`: {}", self.field, self.reason)
    }
}

impl core::error::Error for CommandTokenError {}

// ── ReplayIdentity ──────────────────────────────────────────────────────────

/// The complete, validated identity of a generated test case used to correlate
/// [`FailureReport`]s, [`ReproducibleCommand`]s, corpus entries, and
/// [`crate::runner::ReplayFilter`]s.
///
/// All identity fields must be consistent across these types: attaching a
/// command with a different seed, stream, case index, or operation to a report
/// is rejected by [`FailureReport::with_replay_command`].
///
/// # NUL-safety of V3 domain keys
///
/// The V3 case-derivation keys use NUL bytes as component separators
/// (`"{ver}\x00{stream}\x00{index}"`).  This is safe because `stream_name`
/// and `operation` are validated as stable tokens (`[a-zA-Z0-9._:/-]`), which
/// can never contain NUL.  `ver` is a decimal digit and `index` is a decimal
/// integer — neither can contain NUL either.  Adjacent-component extension
/// attacks (e.g.  `stream="a"`, `index=23` vs.  `stream="a2"`, `index=3`)
/// are therefore impossible.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReplayIdentity {
    /// Always equals [`crate::rng::CASE_SEQUENCE_VERSION`] for current builds.
    case_sequence_version: u8,
    /// Operation label (stable token).
    operation: String,
    /// Stream name.
    stream_name: String,
    /// Primary seed used when the failure occurred.
    seed: TestSeed,
    /// Zero-based case index within the stream.
    case_index: u64,
    /// Specific invariant/property/metamorphic relation kind.
    check_kind: CheckKind,
    /// Specific invariant/property/metamorphic relation replayed.
    check_name: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplayIdentityWire {
    pub case_sequence_version: u8,
    pub operation: String,
    pub stream_name: String,
    pub seed: TestSeed,
    pub case_index: u64,
    pub check_kind: CheckKind,
    pub check_name: String,
}

impl ReplayIdentity {
    /// Creates a validated replay identity.
    ///
    /// # Errors
    ///
    /// Returns [`CommandTokenError`] when any stable-token field is invalid.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        operation: impl Into<String>,
        stream_name: impl Into<String>,
        seed: TestSeed,
        case_index: u64,
        check_kind: CheckKind,
        check_name: impl Into<String>,
    ) -> Result<Self, CommandTokenError> {
        let operation = operation.into();
        let stream_name = stream_name.into();
        let check_name = check_name.into();

        for (field, value) in [
            ("operation", operation.as_str()),
            ("stream_name", stream_name.as_str()),
            ("check_name", check_name.as_str()),
        ] {
            if !is_stable_token(value) {
                return Err(CommandTokenError {
                    field,
                    reason: "must be a nonempty stable token [a-zA-Z0-9._:/-]",
                });
            }
        }

        Ok(Self {
            case_sequence_version: CASE_SEQUENCE_VERSION,
            operation,
            stream_name,
            seed,
            case_index,
            check_kind,
            check_name,
        })
    }

    /// Returns the case-sequence version recorded in this identity.
    #[must_use]
    pub const fn case_sequence_version(&self) -> u8 {
        self.case_sequence_version
    }

    /// Returns the operation label.
    #[must_use]
    pub fn operation(&self) -> &str {
        &self.operation
    }

    /// Returns the stream name.
    #[must_use]
    pub fn stream_name(&self) -> &str {
        &self.stream_name
    }

    /// Returns the replay seed.
    #[must_use]
    pub const fn seed(&self) -> TestSeed {
        self.seed
    }

    /// Returns the case index.
    #[must_use]
    pub const fn case_index(&self) -> u64 {
        self.case_index
    }

    /// Returns the check kind.
    #[must_use]
    pub const fn check_kind(&self) -> CheckKind {
        self.check_kind
    }

    /// Returns the check name.
    #[must_use]
    pub fn check_name(&self) -> &str {
        &self.check_name
    }
}

impl<'de> Deserialize<'de> for ReplayIdentity {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let wire = ReplayIdentityWire::deserialize(deserializer)?;
        if wire.case_sequence_version != CASE_SEQUENCE_VERSION {
            return Err(serde::de::Error::custom(format!(
                "case_sequence_version {} does not match current CASE_SEQUENCE_VERSION {}",
                wire.case_sequence_version, CASE_SEQUENCE_VERSION
            )));
        }
        Self::new(
            wire.operation,
            wire.stream_name,
            wire.seed,
            wire.case_index,
            wire.check_kind,
            wire.check_name,
        )
        .map_err(|err| serde::de::Error::custom(err.to_string()))
    }
}

/// A structured command that reproduces a specific failing test case.
///
/// Rendered to a POSIX shell command via [`fmt::Display`] or a PowerShell
/// command via [`as_powershell`].  Both renderers set
/// all seven replay environment variables required to reproduce the exact
/// failing check.
///
/// The `--exact` flag is always included in the rendered test filter so that a
/// test whose name is a prefix of another test's name does not accidentally
/// invoke both when replaying.
///
/// **POSIX renderer**: renders `VAR=val cmd args` suitable for bash/zsh.
/// Not valid for Windows `cmd.exe`.
///
/// [`as_powershell`]: ReproducibleCommand::as_powershell
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ReproducibleCommandWire", into = "ReproducibleCommandWire")]
pub struct ReproducibleCommand {
    /// Cargo package name (`--package`).
    pub package: String,
    /// Test function or integration test name.
    pub test_name: String,
    /// The [`crate::rng::CASE_SEQUENCE_VERSION`] in effect when the command was built.
    case_sequence_version: u8,
    /// Operation label.
    pub operation: String,
    /// The stream name.
    pub stream_name: String,
    /// The seed to replay.
    pub seed: TestSeed,
    /// The sequential case index.
    pub case_index: u64,
    /// The check kind to replay.
    pub check_kind: CheckKind,
    /// The specific invariant/property/relation name to replay.
    pub check_name: String,
}

#[derive(Serialize, Deserialize)]
struct ReproducibleCommandWire {
    package: String,
    test_name: String,
    case_sequence_version: u8,
    operation: String,
    stream_name: String,
    seed: TestSeed,
    case_index: u64,
    check_kind: CheckKind,
    check_name: String,
}

impl TryFrom<ReproducibleCommandWire> for ReproducibleCommand {
    type Error = String;

    fn try_from(w: ReproducibleCommandWire) -> Result<Self, Self::Error> {
        if w.case_sequence_version != CASE_SEQUENCE_VERSION {
            return Err(format!(
                "case_sequence_version {} does not match current CASE_SEQUENCE_VERSION {}",
                w.case_sequence_version, CASE_SEQUENCE_VERSION
            ));
        }
        let identity = ReplayIdentity::new(
            w.operation,
            w.stream_name,
            w.seed,
            w.case_index,
            w.check_kind,
            w.check_name,
        )
        .map_err(|e| e.to_string())?;
        Self::new(w.package, w.test_name, identity).map_err(|e| e.to_string())
    }
}

impl From<ReproducibleCommand> for ReproducibleCommandWire {
    fn from(c: ReproducibleCommand) -> Self {
        Self {
            package: c.package,
            test_name: c.test_name,
            case_sequence_version: c.case_sequence_version,
            operation: c.operation,
            stream_name: c.stream_name,
            seed: c.seed,
            case_index: c.case_index,
            check_kind: c.check_kind,
            check_name: c.check_name,
        }
    }
}

impl ReproducibleCommand {
    /// Creates a validated reproducible command.
    ///
    /// # Errors
    ///
    /// Returns [`CommandTokenError`] when `package` or `test_name` is invalid.
    #[allow(clippy::needless_pass_by_value)]
    pub fn new(
        package: impl Into<String>,
        test_name: impl Into<String>,
        identity: ReplayIdentity,
    ) -> Result<Self, CommandTokenError> {
        let package = package.into();
        let test_name = test_name.into();

        if !is_command_token(&package) {
            return Err(CommandTokenError {
                field: "package",
                reason: "must be nonempty and contain no NUL or control characters",
            });
        }
        if package.starts_with('-') {
            return Err(CommandTokenError {
                field: "package",
                reason: "must not start with '-' (would be treated as a Cargo flag)",
            });
        }
        if !is_command_token(&test_name) {
            return Err(CommandTokenError {
                field: "test_name",
                reason: "must be nonempty and contain no NUL or control characters",
            });
        }
        if test_name.starts_with('-') {
            return Err(CommandTokenError {
                field: "test_name",
                reason: "must not start with '-' (would be parsed as a test flag by libtest)",
            });
        }

        Ok(Self {
            package,
            test_name,
            case_sequence_version: identity.case_sequence_version(),
            operation: identity.operation().to_owned(),
            stream_name: identity.stream_name().to_owned(),
            seed: identity.seed(),
            case_index: identity.case_index(),
            check_kind: identity.check_kind(),
            check_name: identity.check_name().to_owned(),
        })
    }

    /// Returns a [`ReplayIdentity`] view of this command's identity fields.
    ///
    /// # Panics
    ///
    /// Panics only if this command somehow contains invalid identity fields.
    /// [`ReproducibleCommand::new`] validates them, so this indicates
    /// internal corruption or manual mutation within this module's tests.
    #[must_use]
    pub fn identity(&self) -> ReplayIdentity {
        ReplayIdentity::new(
            &self.operation,
            &self.stream_name,
            self.seed,
            self.case_index,
            self.check_kind,
            &self.check_name,
        )
        .expect("ReproducibleCommand always stores valid identity fields")
    }

    /// Returns the case-sequence version recorded in this replay command.
    #[must_use]
    pub const fn case_sequence_version(&self) -> u8 {
        self.case_sequence_version
    }

    /// Updates the operation label for compatibility with existing tests.
    ///
    /// # Errors
    ///
    /// Returns [`CommandTokenError`] when `operation` is not a stable token.
    pub fn with_operation(
        mut self,
        operation: impl Into<String>,
    ) -> Result<Self, CommandTokenError> {
        let operation = operation.into();
        if !is_stable_token(&operation) {
            return Err(CommandTokenError {
                field: "operation",
                reason: "must be a nonempty stable token [a-zA-Z0-9._:/-]",
            });
        }
        self.operation = operation;
        Ok(self)
    }

    /// Updates the replayed check name for compatibility with existing tests.
    ///
    /// # Errors
    ///
    /// Returns [`CommandTokenError`] when `name` is not a stable token.
    pub fn with_check_name(mut self, name: impl Into<String>) -> Result<Self, CommandTokenError> {
        let name = name.into();
        if !is_stable_token(&name) {
            return Err(CommandTokenError {
                field: "check_name",
                reason: "must be a nonempty stable token [a-zA-Z0-9._:/-]",
            });
        }
        self.check_name = name;
        Ok(self)
    }

    /// Returns a display wrapper that renders this command as a PowerShell
    /// command.
    #[must_use]
    pub fn as_powershell(&self) -> PowerShellDisplay<'_> {
        PowerShellDisplay(self)
    }
}

impl fmt::Display for ReproducibleCommand {
    /// Renders a POSIX shell command (`VAR=val cmd args`).
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{env_version}={version} {env_seed}={seed} {env_case}={idx} {env_stream}={stream} {env_operation}={operation} {env_kind}={kind} {env_check}={check} cargo test --package={pkg} -- --exact {test}",
            env_version = ENV_TEST_VERSION,
            version = self.case_sequence_version,
            env_seed = ENV_TEST_SEED,
            seed = self.seed,
            env_case = ENV_TEST_CASE,
            idx = self.case_index,
            env_stream = ENV_TEST_STREAM,
            stream = posix_quote(&self.stream_name),
            env_operation = ENV_TEST_OPERATION,
            operation = posix_quote(&self.operation),
            env_kind = ENV_TEST_CHECK_KIND,
            kind = self.check_kind,
            env_check = ENV_TEST_CHECK,
            check = posix_quote(&self.check_name),
            pkg = posix_quote(&self.package),
            test = posix_quote(&self.test_name),
        )
    }
}

/// A display wrapper for a [`ReproducibleCommand`] that renders PowerShell syntax.
pub struct PowerShellDisplay<'a>(&'a ReproducibleCommand);

impl fmt::Display for PowerShellDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let c = self.0;
        write!(
            f,
            "$env:{env_version}={version}; $env:{env_seed}={seed}; $env:{env_case}={idx}; $env:{env_stream}={stream}; $env:{env_operation}={operation}; $env:{env_kind}={kind}; $env:{env_check}={check}; cargo test --package={pkg} -- --exact {test}",
            env_version = ENV_TEST_VERSION,
            version = ps_quote(&c.case_sequence_version.to_string()),
            env_seed = ENV_TEST_SEED,
            seed = ps_quote(&c.seed.to_string()),
            env_case = ENV_TEST_CASE,
            idx = ps_quote(&c.case_index.to_string()),
            env_stream = ENV_TEST_STREAM,
            stream = ps_quote(&c.stream_name),
            env_operation = ENV_TEST_OPERATION,
            operation = ps_quote(&c.operation),
            env_kind = ENV_TEST_CHECK_KIND,
            kind = ps_quote(&c.check_kind.to_string()),
            env_check = ENV_TEST_CHECK,
            check = ps_quote(&c.check_name),
            pkg = ps_quote(&c.package),
            test = ps_quote(&c.test_name),
        )
    }
}

// ── ReportError ───────────────────────────────────────────────────────────

/// Error returned when constructing a [`FailureReport`] with invalid fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReportError {
    /// A required string field is empty or contains invalid characters.
    InvalidToken {
        /// The field that failed validation.
        field: &'static str,
        /// The rejected value.
        value: String,
    },
    /// The `case_id` does not match
    /// `CaseId::new(seed.for_case_stream(stream_name), case_index)`.
    CaseIdMismatch {
        /// The `case_id` supplied by the caller.
        supplied: CaseId,
        /// The `case_id` that would be derived from the other fields.
        expected: CaseId,
    },
    /// The `failure_message` is empty.
    EmptyMessage,
    /// A [`ReproducibleCommand`] passed to
    /// [`FailureReport::with_replay_command`] describes a different case than
    /// the report itself (seed, stream name, or case index disagree).
    CommandMismatch {
        /// The mismatched field (`"seed"`, `"stream_name"`, or `"case_index"`).
        field: &'static str,
        /// The value on the report.
        expected: String,
        /// The value on the supplied command.
        found: String,
    },
    /// The `case_sequence_version` field does not match
    /// [`crate::rng::CASE_SEQUENCE_VERSION`].
    SequenceVersionMismatch {
        /// The `case_sequence_version` found in the loaded record.
        found: u8,
    },
}

impl fmt::Display for ReportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidToken { field, value } => {
                write!(
                    f,
                    "invalid field `{field}`: {value:?} is not a stable token"
                )
            }
            Self::CaseIdMismatch { supplied, expected } => write!(
                f,
                "case_id {} does not match derived id {} \
                 (CaseId::new(seed.for_case_stream(stream_name), case_index))",
                supplied.to_hex(),
                expected.to_hex(),
            ),
            Self::EmptyMessage => f.write_str("failure_message must not be empty"),
            Self::CommandMismatch {
                field,
                expected,
                found,
            } => write!(
                f,
                "replay command field `{field}` ({found}) does not match report ({expected})"
            ),
            Self::SequenceVersionMismatch { found } => write!(
                f,
                "case_sequence_version {found} does not match current \
                 CASE_SEQUENCE_VERSION {CASE_SEQUENCE_VERSION}"
            ),
        }
    }
}

impl core::error::Error for ReportError {}

// ── FailureReport ──────────────────────────────────────────────────────────

/// Raw wire type for deserialising [`FailureReport`].
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FailureReportWire {
    schema_version: SchemaVersion,
    case_sequence_version: u8,
    seed: TestSeed,
    stream_name: String,
    case_id: CaseId,
    operation: String,
    case_index: u64,
    check_kind: CheckKind,
    check_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tolerance_context: Option<ToleranceContext>,
    inputs_json: serde_json::Value,
    failure_message: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    diagnostics: Vec<Diagnostic>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    replay_command: Option<ReproducibleCommand>,
}

/// A complete, actionable record of a single test-case failure.
///
/// Every field is deterministic: identical inputs always produce identical
/// JSON output. The report is suitable for storage in a regression corpus or
/// for display in CI output.
///
/// `inputs_json` stores the failure inputs as a structured [`serde_json::Value`]
/// (not a double-encoded JSON string).
///
/// Diagnostics are sorted and deduplicated on attachment and validated as
/// sorted on load so that the JSON output is byte-stable.
///
/// Use the consuming builder methods ([`with_tolerance_context`],
/// [`with_diagnostics`], [`with_replay_command`]) to attach optional fields.
///
/// [`with_tolerance_context`]: FailureReport::with_tolerance_context
/// [`with_diagnostics`]: FailureReport::with_diagnostics
/// [`with_replay_command`]: FailureReport::with_replay_command
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FailureReport {
    /// Schema version of this report record (see [`REPORT_SCHEMA_VERSION`]).
    schema_version: SchemaVersion,
    /// The [`crate::rng::CASE_SEQUENCE_VERSION`] in effect when this report
    /// was produced; validated on load to reject stale-formula records.
    case_sequence_version: u8,
    /// The primary seed used when the failure occurred.
    seed: TestSeed,
    /// Stream name — combined with seed and `case_index`, uniquely identifies
    /// the failing case for replay.
    stream_name: String,
    /// Stable case identifier for corpus lookup.
    case_id: CaseId,
    /// Stable dot-separated operation label.
    operation: String,
    /// Sequential case index within the stream.
    case_index: u64,
    /// Kind of check that failed.
    check_kind: CheckKind,
    /// Name of the specific check that failed.
    check_name: String,
    /// Tolerance context in effect at the time of failure.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    tolerance_context: Option<ToleranceContext>,
    /// Structured inputs sufficient to reproduce the failure.
    inputs_json: serde_json::Value,
    /// Human-readable failure description.
    failure_message: String,
    /// Diagnostics attached to the failure (sorted, deduplicated).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    diagnostics: Vec<Diagnostic>,
    /// Command to reproduce this failure from the command line.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    replay_command: Option<ReproducibleCommand>,
}

impl<'de> Deserialize<'de> for FailureReport {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let w = FailureReportWire::deserialize(d)?;

        // Validate schema version compatibility before inspecting anything
        // else: a wrong major version means the format itself may be
        // incompatible; a newer minor version may use fields this build
        // does not understand.
        if w.schema_version.major() != REPORT_SCHEMA_VERSION.major() {
            return Err(serde::de::Error::custom(format!(
                "unsupported schema_version {}.{} (expected major {})",
                w.schema_version.major(),
                w.schema_version.minor(),
                REPORT_SCHEMA_VERSION.major()
            )));
        }
        if w.schema_version.minor() > REPORT_SCHEMA_VERSION.minor() {
            return Err(serde::de::Error::custom(format!(
                "unsupported schema_version {}.{} (this build supports up to minor {})",
                w.schema_version.major(),
                w.schema_version.minor(),
                REPORT_SCHEMA_VERSION.minor()
            )));
        }

        // Validate the case-sequence (domain-key) version separately from
        // the record schema version: it governs how `case_id` was derived.
        if w.case_sequence_version != CASE_SEQUENCE_VERSION {
            let err = ReportError::SequenceVersionMismatch {
                found: w.case_sequence_version,
            };
            return Err(serde::de::Error::custom(err.to_string()));
        }

        // Validate tokens.
        if !is_stable_token(&w.stream_name) {
            return Err(serde::de::Error::custom(format!(
                "invalid stream_name: {:?}",
                w.stream_name
            )));
        }
        if !is_stable_token(&w.operation) {
            return Err(serde::de::Error::custom(format!(
                "invalid operation: {:?}",
                w.operation
            )));
        }
        if !is_stable_token(&w.check_name) {
            return Err(serde::de::Error::custom(format!(
                "invalid check_name: {:?}",
                w.check_name
            )));
        }
        if w.failure_message.is_empty() {
            return Err(serde::de::Error::custom(
                "failure_message must not be empty",
            ));
        }

        // Validate CaseId consistency.
        let expected = CaseId::new(w.seed.for_case_stream(&w.stream_name), w.case_index);
        if w.case_id != expected {
            return Err(serde::de::Error::custom(format!(
                "case_id {} does not match derived {}",
                w.case_id.to_hex(),
                expected.to_hex()
            )));
        }

        // Validate diagnostics are strictly increasing: no descending entries and
        // no adjacent equals.  Sorted, deduplicated output is validated here so
        // that persisted diagnostics are byte-stable and unambiguous.
        for i in 1..w.diagnostics.len() {
            if w.diagnostics[i] <= w.diagnostics[i - 1] {
                return Err(serde::de::Error::custom(
                    "diagnostics must be strictly sorted (no duplicates, no descending pairs)",
                ));
            }
        }

        let report = Self {
            schema_version: w.schema_version,
            case_sequence_version: w.case_sequence_version,
            seed: w.seed,
            stream_name: w.stream_name,
            case_id: w.case_id,
            operation: w.operation,
            case_index: w.case_index,
            check_kind: w.check_kind,
            check_name: w.check_name,
            tolerance_context: w.tolerance_context,
            inputs_json: w.inputs_json,
            failure_message: w.failure_message,
            diagnostics: w.diagnostics,
            replay_command: None,
        };
        if let Some(cmd) = w.replay_command {
            report
                .with_replay_command(cmd)
                .map_err(|e| serde::de::Error::custom(e.to_string()))
        } else {
            Ok(report)
        }
    }
}

impl FailureReport {
    /// Creates a validated minimal failure report.
    ///
    /// # Errors
    ///
    /// Returns [`ReportError`] when:
    /// - `stream_name` or `operation` is not a valid stable token.
    /// - `failure_message` is empty.
    /// - `case_id` does not equal
    ///   `CaseId::new(seed.for_case_stream(stream_name), case_index)`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        seed: TestSeed,
        case_id: CaseId,
        stream_name: impl Into<String>,
        operation: impl Into<String>,
        case_index: u64,
        check_kind: CheckKind,
        check_name: impl Into<String>,
        inputs_json: serde_json::Value,
        failure_message: impl Into<String>,
    ) -> Result<Self, ReportError> {
        let stream_name = stream_name.into();
        let operation = operation.into();
        let check_name = check_name.into();
        let failure_message = failure_message.into();

        if !is_stable_token(&stream_name) {
            return Err(ReportError::InvalidToken {
                field: "stream_name",
                value: stream_name,
            });
        }
        if !is_stable_token(&operation) {
            return Err(ReportError::InvalidToken {
                field: "operation",
                value: operation,
            });
        }
        if !is_stable_token(&check_name) {
            return Err(ReportError::InvalidToken {
                field: "check_name",
                value: check_name,
            });
        }
        if failure_message.is_empty() {
            return Err(ReportError::EmptyMessage);
        }

        let expected_id = CaseId::new(seed.for_case_stream(&stream_name), case_index);
        if case_id != expected_id {
            return Err(ReportError::CaseIdMismatch {
                supplied: case_id,
                expected: expected_id,
            });
        }

        Ok(Self {
            schema_version: REPORT_SCHEMA_VERSION,
            case_sequence_version: CASE_SEQUENCE_VERSION,
            seed,
            stream_name,
            case_id,
            operation,
            case_index,
            check_kind,
            check_name,
            tolerance_context: None,
            inputs_json,
            failure_message,
            diagnostics: Vec::new(),
            replay_command: None,
        })
    }

    /// Attaches a tolerance context to this report.
    #[must_use]
    pub fn with_tolerance_context(mut self, ctx: ToleranceContext) -> Self {
        self.tolerance_context = Some(ctx);
        self
    }

    /// Attaches diagnostics, sorting and deduplicating them for byte-stable output.
    #[must_use]
    pub fn with_diagnostics(mut self, mut diagnostics: Vec<Diagnostic>) -> Self {
        diagnostics.sort();
        diagnostics.dedup();
        self.diagnostics = diagnostics;
        self
    }

    /// Attaches a reproducible replay command.
    ///
    /// # Errors
    ///
    /// Returns [`ReportError::CommandMismatch`] when any identity field disagrees.
    pub fn with_replay_command(mut self, cmd: ReproducibleCommand) -> Result<Self, ReportError> {
        for (field, expected, found) in [
            ("seed", self.seed.to_string(), cmd.seed.to_string()),
            (
                "stream_name",
                self.stream_name.clone(),
                cmd.stream_name.clone(),
            ),
            (
                "case_index",
                self.case_index.to_string(),
                cmd.case_index.to_string(),
            ),
            ("operation", self.operation.clone(), cmd.operation.clone()),
            (
                "check_kind",
                self.check_kind.to_string(),
                cmd.check_kind.to_string(),
            ),
            (
                "check_name",
                self.check_name.clone(),
                cmd.check_name.clone(),
            ),
        ] {
            if expected != found {
                return Err(ReportError::CommandMismatch {
                    field,
                    expected,
                    found,
                });
            }
        }
        if cmd.case_sequence_version != CASE_SEQUENCE_VERSION {
            return Err(ReportError::SequenceVersionMismatch {
                found: cmd.case_sequence_version,
            });
        }
        self.replay_command = Some(cmd);
        Ok(self)
    }

    /// Returns a [`ReplayIdentity`] view of this report's identity fields.
    ///
    /// # Panics
    ///
    /// Panics only if this report somehow contains invalid identity fields.
    /// [`FailureReport::new`] validates them, so this indicates internal
    /// corruption.
    #[must_use]
    pub fn identity(&self) -> ReplayIdentity {
        ReplayIdentity::new(
            &self.operation,
            &self.stream_name,
            self.seed,
            self.case_index,
            self.check_kind,
            &self.check_name,
        )
        .expect("FailureReport always stores valid identity fields")
    }

    /// Returns the schema version of this report.
    #[must_use]
    pub const fn schema_version(&self) -> SchemaVersion {
        self.schema_version
    }

    /// Returns the [`crate::rng::CASE_SEQUENCE_VERSION`] in effect when this
    /// report was produced.
    #[must_use]
    pub const fn case_sequence_version(&self) -> u8 {
        self.case_sequence_version
    }

    /// Returns the seed.
    #[must_use]
    pub const fn seed(&self) -> TestSeed {
        self.seed
    }

    /// Returns the stream name.
    #[must_use]
    pub fn stream_name(&self) -> &str {
        &self.stream_name
    }

    /// Returns the case identifier.
    #[must_use]
    pub const fn case_id(&self) -> CaseId {
        self.case_id
    }

    /// Returns the operation label.
    #[must_use]
    pub fn operation(&self) -> &str {
        &self.operation
    }

    /// Returns the sequential case index.
    #[must_use]
    pub const fn case_index(&self) -> u64 {
        self.case_index
    }

    /// Returns the failed check kind.
    #[must_use]
    pub const fn check_kind(&self) -> CheckKind {
        self.check_kind
    }

    /// Returns the failed check name.
    #[must_use]
    pub fn check_name(&self) -> &str {
        &self.check_name
    }

    /// Returns the tolerance context, if any.
    #[must_use]
    pub const fn tolerance_context(&self) -> Option<ToleranceContext> {
        self.tolerance_context
    }

    /// Returns the structured input values.
    #[must_use]
    pub fn inputs_json(&self) -> &serde_json::Value {
        &self.inputs_json
    }

    /// Returns the failure message.
    #[must_use]
    pub fn failure_message(&self) -> &str {
        &self.failure_message
    }

    /// Returns attached diagnostics (sorted, deduplicated).
    #[must_use]
    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    /// Returns the reproducible replay command, if set.
    #[must_use]
    pub const fn replay_command(&self) -> Option<&ReproducibleCommand> {
        self.replay_command.as_ref()
    }

    /// Serialises this report to a deterministic JSON string.
    ///
    /// Identical reports always produce identical strings.
    ///
    /// # Errors
    ///
    /// Returns a `serde_json` error if serialisation fails (unreachable in
    /// practice given the validated field types).
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{
        CommandTokenError, FailureReport, REPORT_SCHEMA_VERSION, ReportError, ReproducibleCommand,
        posix_quote,
    };
    use crate::fuzz::CheckKind;
    use crate::rng::{CASE_SEQUENCE_VERSION, CaseId, TestSeed};

    fn seed(v: u64) -> TestSeed {
        TestSeed::new(v)
    }

    fn identity(operation: &str, stream: &str, seed: TestSeed, idx: u64) -> super::ReplayIdentity {
        super::ReplayIdentity::new(operation, stream, seed, idx, CheckKind::Invariant, "check")
            .expect("valid identity")
    }

    fn new_command(
        package: &str,
        test_name: &str,
        seed: TestSeed,
        case_index: u64,
        stream_name: &str,
    ) -> Result<ReproducibleCommand, CommandTokenError> {
        let identity = super::ReplayIdentity::new(
            "op",
            stream_name,
            seed,
            case_index,
            CheckKind::Invariant,
            "check",
        )?;
        ReproducibleCommand::new(package, test_name, identity)
    }

    fn new_failure_report(
        seed: TestSeed,
        case_id: CaseId,
        stream_name: &str,
        operation: &str,
        case_index: u64,
        inputs_json: serde_json::Value,
        failure_message: &str,
    ) -> Result<FailureReport, ReportError> {
        FailureReport::new(
            seed,
            case_id,
            stream_name,
            operation,
            case_index,
            CheckKind::Invariant,
            "check",
            inputs_json,
            failure_message,
        )
    }

    fn make_report() -> FailureReport {
        let s = seed(12345);
        let case_id = CaseId::new(s.for_case_stream("report.test"), 3);
        new_failure_report(
            s,
            case_id,
            "report.test",
            "primitive.cuboid",
            3,
            serde_json::json!({"w": 1.0}),
            "invariant violated",
        )
        .expect("valid report")
        .with_replay_command(
            ReproducibleCommand::new(
                "amphion-test-support",
                "test_cuboid",
                identity("primitive.cuboid", "report.test", s, 3),
            )
            .expect("valid cmd"),
        )
        .expect("matching replay command")
    }

    #[test]
    fn report_schema_version_is_stable() {
        let r = make_report();
        assert_eq!(r.schema_version(), REPORT_SCHEMA_VERSION);
        let json = r.to_json().expect("ser");
        assert!(
            json.contains("schema_version"),
            "schema_version must be present in JSON"
        );
    }

    #[test]
    fn failure_report_getters_are_consistent() {
        let r = make_report();
        assert_eq!(r.seed(), seed(12345));
        assert_eq!(r.case_index(), 3);
        assert_eq!(r.stream_name(), "report.test");
        assert_eq!(r.operation(), "primitive.cuboid");
        assert!(r.inputs_json().is_object());
        assert!(r.failure_message().contains("invariant"));
        assert!(r.replay_command().is_some());
        assert!(r.tolerance_context().is_none());
        assert!(r.diagnostics().is_empty());
    }

    #[test]
    fn failure_report_json_is_deterministic() {
        let report = make_report();
        let json1 = report.to_json().expect("ser1");
        let json2 = report.to_json().expect("ser2");
        assert_eq!(json1, json2);
    }

    #[test]
    fn failure_report_serde_round_trips() {
        let report = make_report();
        let json = report.to_json().expect("ser");
        let decoded: FailureReport = serde_json::from_str(&json).expect("round-trip");
        assert_eq!(report, decoded);
    }

    #[test]
    fn report_new_rejects_invalid_stream_name() {
        let s = seed(1);
        let id = CaseId::new(s.for_case_stream("s"), 0);
        assert!(matches!(
            new_failure_report(s, id, "", "op", 0, serde_json::json!({}), "fail"),
            Err(ReportError::InvalidToken {
                field: "stream_name",
                ..
            })
        ));
        assert!(matches!(
            new_failure_report(s, id, "bad stream", "op", 0, serde_json::json!({}), "fail"),
            Err(ReportError::InvalidToken {
                field: "stream_name",
                ..
            })
        ));
    }

    #[test]
    fn report_new_rejects_invalid_operation() {
        let s = seed(2);
        let id = CaseId::new(s.for_case_stream("s"), 0);
        assert!(matches!(
            new_failure_report(s, id, "s", "", 0, serde_json::json!({}), "fail"),
            Err(ReportError::InvalidToken {
                field: "operation",
                ..
            })
        ));
    }

    #[test]
    fn report_new_rejects_empty_message() {
        let s = seed(3);
        let id = CaseId::new(s.for_case_stream("s"), 0);
        assert!(matches!(
            new_failure_report(s, id, "s", "op", 0, serde_json::json!({}), ""),
            Err(ReportError::EmptyMessage)
        ));
    }

    #[test]
    fn report_new_rejects_mismatched_case_id() {
        let s = seed(4);
        let wrong_id = CaseId::new(s, 0); // uses primary seed, not stream seed
        assert!(matches!(
            new_failure_report(s, wrong_id, "s", "op", 0, serde_json::json!({}), "fail"),
            Err(ReportError::CaseIdMismatch { .. })
        ));
    }

    #[test]
    fn inputs_json_stored_as_value_not_string() {
        let s = seed(5);
        let id = CaseId::new(s.for_case_stream("s"), 0);
        let inputs = serde_json::json!({"x": 1.5, "flag": true});
        let r = new_failure_report(s, id, "s", "op", 0, inputs.clone(), "fail").expect("valid");
        assert_eq!(r.inputs_json(), &inputs);
        // In JSON output it must be an inline object, not a string.
        let json = r.to_json().expect("ser");
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(
            parsed["inputs_json"].is_object(),
            "inputs_json must be inline object in JSON"
        );
    }

    #[test]
    fn reproducible_command_new_validates_fields() {
        let s = seed(1);
        // NUL byte rejected.
        assert!(matches!(
            new_command("pkg\x00bad", "test", s, 0, "s"),
            Err(CommandTokenError {
                field: "package",
                ..
            })
        ));
        // Control char rejected.
        assert!(matches!(
            new_command("pkg", "test\x01fn", s, 0, "s"),
            Err(CommandTokenError {
                field: "test_name",
                ..
            })
        ));
        // Invalid stream_name.
        assert!(matches!(
            new_command("pkg", "test", s, 0, "bad stream"),
            Err(CommandTokenError {
                field: "stream_name",
                ..
            })
        ));
        // Empty package.
        assert!(matches!(
            new_command("", "test", s, 0, "s"),
            Err(CommandTokenError {
                field: "package",
                ..
            })
        ));
    }

    #[test]
    fn reproducible_command_rejects_package_starting_with_dash() {
        let s = seed(1);
        assert!(matches!(
            new_command("-p", "test", s, 0, "s"),
            Err(CommandTokenError {
                field: "package",
                ..
            })
        ));
        assert!(matches!(
            new_command("--flag-like", "test", s, 0, "s"),
            Err(CommandTokenError {
                field: "package",
                ..
            })
        ));
    }

    #[test]
    fn reproducible_command_accepts_valid_package_names() {
        let s = seed(1);
        assert!(new_command("amphion-test-support", "test", s, 0, "s").is_ok());
        assert!(new_command("pkg_name", "test", s, 0, "s").is_ok());
    }

    #[test]
    fn test_name_starting_with_dash_is_rejected() {
        let s = TestSeed::new(0);
        assert!(new_command("pkg", "--nocapture", s, 0, "stream").is_err());
        assert!(new_command("pkg", "-h", s, 0, "stream").is_err());
        assert!(new_command("pkg", "valid_name", s, 0, "stream").is_ok());
    }

    #[test]
    fn exact_flag_with_dash_prefix_name_properly_rejected() {
        // Ensure that "-- --exact --nocapture" cannot happen because
        // "--nocapture" is rejected at construction.
        let s = TestSeed::new(0);
        let result = new_command("pkg", "--nocapture", s, 0, "stream");
        assert!(result.is_err(), "dash-prefix test name must be rejected");
    }

    #[test]
    fn reproducible_command_posix_display_is_correct() {
        let s = seed(42);
        let cmd = new_command("amphion-foundation", "my_test", s, 7, "ops.sphere").expect("valid");
        let display = cmd.to_string();
        assert!(display.contains("AMPHION_TEST_SEED=42"));
        assert!(display.contains("AMPHION_TEST_CASE=7"));
        assert!(display.contains("AMPHION_TEST_STREAM=ops.sphere"));
        assert!(
            display.contains("--package=amphion-foundation"),
            "must use --package= instead of -p: {display}"
        );
        assert!(display.contains("--exact"), "must pass --exact: {display}");
        assert!(display.contains("my_test"));
        assert!(display.contains("AMPHION_TEST_VERSION=3"));
        assert!(display.contains("AMPHION_TEST_OPERATION=op"));
        assert!(display.contains("AMPHION_TEST_CHECK_KIND=invariant"));
        assert!(display.contains("AMPHION_TEST_CHECK=check"));
    }

    #[test]
    fn exact_flag_prevents_prefix_matching() {
        // Without --exact, "my_test" would also run "my_test_helper".
        // With --exact, only "my_test" runs. Verify the flag is in the rendered cmd.
        let s = seed(77);
        let cmd = new_command("pkg", "my_test", s, 0, "s").expect("valid");
        let display = cmd.to_string();
        // The rendered args must be `-- --exact my_test`, not `-- my_test`.
        assert!(
            display.contains("-- --exact my_test"),
            "rendered command must use --exact: {display}"
        );
    }

    #[test]
    fn reproducible_command_posix_quotes_special_chars() {
        let s = seed(1);
        let cmd = new_command("my pkg", "test fn", s, 0, "s").expect("valid");
        let display = cmd.to_string();
        assert!(
            display.contains("'my pkg'"),
            "space in package must be quoted"
        );
        assert!(
            display.contains("'test fn'"),
            "space in test must be quoted"
        );
    }

    #[test]
    fn reproducible_command_with_check_name_renders_env_var() {
        let s = seed(1);
        let cmd = new_command("pkg", "test", s, 0, "s")
            .expect("valid")
            .with_check_name("my.invariant")
            .expect("valid check name");
        let display = cmd.to_string();
        assert!(display.contains("AMPHION_TEST_CHECK=my.invariant"));
        let ps = cmd.as_powershell().to_string();
        assert!(ps.contains("$env:AMPHION_TEST_CHECK='my.invariant'"));
    }

    #[test]
    fn reproducible_command_with_check_name_rejects_invalid_token() {
        let s = seed(1);
        let cmd = new_command("pkg", "test", s, 0, "s").expect("valid");
        assert!(matches!(
            cmd.with_check_name("bad name"),
            Err(CommandTokenError {
                field: "check_name",
                ..
            })
        ));
    }

    #[test]
    fn reproducible_command_powershell_display_is_correct() {
        let s = seed(99);
        let cmd = new_command("pkg", "test_fn", s, 3, "ops.stream").expect("valid");
        let ps = cmd.as_powershell().to_string();
        assert!(ps.contains("$env:AMPHION_TEST_SEED"), "PS must set seed");
        assert!(
            ps.contains("$env:AMPHION_TEST_STREAM"),
            "PS must set stream"
        );
        assert!(ps.contains("cargo test"));
        assert!(
            ps.contains("--package='pkg'"),
            "PS must use --package= too: {ps}"
        );
        assert!(ps.contains("--exact"), "PS must pass --exact: {ps}");
        assert!(ps.contains("ops.stream"));
    }

    #[test]
    fn with_replay_command_operation_mismatch_rejected() {
        let s = seed(20);
        let report = base_report(s, "op.mismatch", 0);
        // report has operation="op"; command has operation="other.op"
        let cmd = new_command("pkg", "test", s, 0, "op.mismatch")
            .expect("valid")
            .with_operation("other.op")
            .expect("valid op");
        assert!(
            matches!(
                report.with_replay_command(cmd),
                Err(ReportError::CommandMismatch {
                    field: "operation",
                    ..
                })
            ),
            "mismatched operation must be rejected"
        );
    }

    #[test]
    fn with_replay_command_matching_operation_accepted() {
        let s = seed(21);
        let id = CaseId::new(s.for_case_stream("op.match"), 0);
        let report = new_failure_report(
            s,
            id,
            "op.match",
            "prim.sphere",
            0,
            serde_json::json!({}),
            "fail",
        )
        .expect("valid");
        let cmd = new_command("pkg", "test", s, 0, "op.match")
            .expect("valid")
            .with_operation("prim.sphere")
            .expect("valid op");
        assert!(
            report.with_replay_command(cmd).is_ok(),
            "matching operation must be accepted"
        );
    }

    #[test]
    fn replay_identity_is_consistent_between_report_and_command() {
        let s = seed(22);
        let id = CaseId::new(s.for_case_stream("identity.test"), 5);
        let report = new_failure_report(
            s,
            id,
            "identity.test",
            "prim.box",
            5,
            serde_json::json!({}),
            "fail",
        )
        .expect("valid");
        let cmd = new_command("pkg", "test", s, 5, "identity.test")
            .expect("valid")
            .with_operation("prim.box")
            .expect("valid op");
        let r2 = report.with_replay_command(cmd).expect("ok");
        let ri = r2.identity();
        assert_eq!(
            ri.case_sequence_version(),
            crate::rng::CASE_SEQUENCE_VERSION
        );
        assert_eq!(ri.seed(), s);
        assert_eq!(ri.stream_name(), "identity.test");
        assert_eq!(ri.case_index(), 5);
        assert_eq!(ri.operation(), "prim.box");
    }

    #[test]
    fn with_replay_command_rejects_old_case_sequence_version() {
        let s = seed(23);
        let report = base_report(s, "old.ver", 0);
        let mut cmd = new_command("pkg", "test", s, 0, "old.ver").expect("valid");
        cmd.case_sequence_version = 1; // deliberately stale
        assert!(
            matches!(
                report.with_replay_command(cmd),
                Err(ReportError::SequenceVersionMismatch { .. })
            ),
            "stale case_sequence_version must be rejected"
        );
    }

    #[test]
    fn reproducible_command_deserialize_rejects_stale_version() {
        let json = serde_json::json!({
            "package": "my-pkg",
            "test_name": "my_test",
            "case_sequence_version": 99u8,
            "operation": "test.op",
            "stream_name": "test.stream",
            "seed": 42u64,
            "case_index": 0u64,
            "check_kind": "invariant",
            "check_name": "my.check"
        });
        let result: Result<ReproducibleCommand, _> = serde_json::from_value(json);
        assert!(
            result.is_err(),
            "stale case_sequence_version must be rejected by Deserialize"
        );
    }

    #[test]
    fn posix_quote_safe_tokens_are_not_quoted() {
        assert_eq!(posix_quote("abc-123_def"), "abc-123_def");
        assert_eq!(posix_quote("pkg:suffix"), "pkg:suffix");
        assert_eq!(posix_quote("ops.sphere"), "ops.sphere");
    }

    #[test]
    fn posix_quote_embeds_single_quote_escape() {
        assert_eq!(posix_quote("it's"), "'it'\\''s'");
    }

    #[test]
    fn with_diagnostics_sorts_and_deduplicates() {
        use amphion_foundation::{Diagnostic, DiagnosticCode, Severity};
        let code_a = DiagnosticCode::try_new("A.ONE").unwrap();
        let code_b = DiagnosticCode::try_new("B.TWO").unwrap();
        let d_b = Diagnostic::new(Severity::Error, code_b, "b", vec![], vec![]);
        let d_a = Diagnostic::new(Severity::Error, code_a, "a", vec![], vec![]);
        let r = make_report().with_diagnostics(vec![d_b.clone(), d_a.clone(), d_b.clone()]);
        let diags = r.diagnostics();
        assert_eq!(diags.len(), 2);
        assert_eq!(diags[0], d_a);
        assert_eq!(diags[1], d_b);
    }

    #[test]
    fn failure_report_without_optional_fields_omits_them() {
        let s = seed(1);
        let id = CaseId::new(s.for_case_stream("s"), 0);
        let r =
            new_failure_report(s, id, "s", "op", 0, serde_json::json!({}), "fail").expect("valid");
        let json = r.to_json().expect("ser");
        assert!(!json.contains("tolerance_context"));
        assert!(!json.contains("replay_command"));
        assert!(!json.contains("diagnostics"));
    }

    #[test]
    fn with_tolerance_context_round_trips() {
        use amphion_foundation::ToleranceContext;
        let ctx = ToleranceContext::try_new(1e-9, 1e-8, 1e-10, 1e-12).expect("valid tolerances");
        let r = make_report().with_tolerance_context(ctx);
        assert_eq!(r.tolerance_context(), Some(ctx));
        let json = r.to_json().expect("ser");
        let decoded: FailureReport = serde_json::from_str(&json).expect("round-trip");
        assert_eq!(decoded.tolerance_context(), Some(ctx));
    }

    // ── Issue 3: with_replay_command validation ────────────────────────────

    fn base_report(s: TestSeed, stream: &'static str, case_index: u64) -> FailureReport {
        let id = CaseId::new(s.for_case_stream(stream), case_index);
        new_failure_report(
            s,
            id,
            stream,
            "op",
            case_index,
            serde_json::json!({}),
            "fail",
        )
        .expect("valid report")
    }

    #[test]
    fn with_replay_command_accepts_matching_command() {
        let s = seed(10);
        let report = base_report(s, "report.match", 5);
        let cmd = new_command("pkg", "test", s, 5, "report.match").expect("valid");
        assert!(report.with_replay_command(cmd).is_ok());
    }

    #[test]
    fn with_replay_command_rejects_mismatched_seed() {
        let s = seed(11);
        let report = base_report(s, "report.mismatch.seed", 0);
        let cmd = new_command("pkg", "test", seed(999), 0, "report.mismatch.seed").expect("valid");
        assert!(matches!(
            report.with_replay_command(cmd),
            Err(ReportError::CommandMismatch { field: "seed", .. })
        ));
    }

    #[test]
    fn with_replay_command_rejects_mismatched_stream_name() {
        let s = seed(12);
        let report = base_report(s, "report.mismatch.stream", 0);
        let cmd = new_command("pkg", "test", s, 0, "other.stream").expect("valid");
        assert!(matches!(
            report.with_replay_command(cmd),
            Err(ReportError::CommandMismatch {
                field: "stream_name",
                ..
            })
        ));
    }

    #[test]
    fn with_replay_command_rejects_mismatched_case_index() {
        let s = seed(13);
        let report = base_report(s, "report.mismatch.case", 3);
        let cmd = new_command("pkg", "test", s, 99, "report.mismatch.case").expect("valid");
        assert!(matches!(
            report.with_replay_command(cmd),
            Err(ReportError::CommandMismatch {
                field: "case_index",
                ..
            })
        ));
    }

    // ── Issue 6: schema/sequence version validation on load ────────────────

    #[test]
    fn future_schema_major_version_is_rejected() {
        let report = make_report();
        let json = report.to_json().expect("ser");
        let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
        value["schema_version"]["major"] = serde_json::json!(REPORT_SCHEMA_VERSION.major() + 1);
        let bad_json = serde_json::to_string(&value).unwrap();
        let result: Result<FailureReport, _> = serde_json::from_str(&bad_json);
        assert!(result.is_err(), "wrong major version must be rejected");
    }

    #[test]
    fn future_schema_minor_version_is_rejected() {
        let report = make_report();
        let json = report.to_json().expect("ser");
        let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
        value["schema_version"]["minor"] = serde_json::json!(REPORT_SCHEMA_VERSION.minor() + 1);
        let bad_json = serde_json::to_string(&value).unwrap();
        let result: Result<FailureReport, _> = serde_json::from_str(&bad_json);
        assert!(
            result.is_err(),
            "unknown future minor version must be rejected"
        );
    }

    #[test]
    fn sequence_version_mismatch_is_rejected() {
        let report = make_report();
        let json = report.to_json().expect("ser");
        let mut value: serde_json::Value = serde_json::from_str(&json).unwrap();
        value["case_sequence_version"] = serde_json::json!(255);
        let bad_json = serde_json::to_string(&value).unwrap();
        let result: Result<FailureReport, _> = serde_json::from_str(&bad_json);
        assert!(
            result.is_err(),
            "mismatched case_sequence_version must be rejected"
        );
    }

    #[test]
    fn duplicate_equal_diagnostics_rejected_on_load() {
        use amphion_foundation::{Diagnostic, DiagnosticCode, Severity};
        let code = DiagnosticCode::try_new("D.ONE").unwrap();
        let d = Diagnostic::new(Severity::Error, code, "msg", vec![], vec![]);
        // Manually craft a report JSON with duplicate equal diagnostics.
        let s = seed(99);
        let id = CaseId::new(s.for_case_stream("dup.diag"), 0);
        let r = new_failure_report(s, id, "dup.diag", "op", 0, serde_json::json!({}), "fail")
            .expect("valid")
            .with_diagnostics(vec![d.clone()]);
        let json = r.to_json().expect("ser");
        // Inject two identical diagnostics at the JSON level.
        let mut v: serde_json::Value = serde_json::from_str(&json).unwrap();
        let d_val = v["diagnostics"][0].clone();
        v["diagnostics"] = serde_json::json!([d_val.clone(), d_val]);
        let bad_json = serde_json::to_string(&v).unwrap();
        let result: Result<FailureReport, _> = serde_json::from_str(&bad_json);
        assert!(
            result.is_err(),
            "duplicate equal diagnostics must be rejected on load"
        );
    }

    #[test]
    fn matching_versions_round_trip_successfully() {
        let report = make_report();
        let json = report.to_json().expect("ser");
        let decoded: FailureReport = serde_json::from_str(&json).expect("must load");
        assert_eq!(
            decoded.case_sequence_version(),
            report.case_sequence_version()
        );
    }
    #[test]
    fn replay_identity_new_rejects_empty_operation() {
        let err =
            super::ReplayIdentity::new("", "stream", seed(1), 0, CheckKind::Invariant, "check")
                .unwrap_err();
        assert_eq!(err.field, "operation");
    }

    #[test]
    fn replay_identity_new_rejects_empty_stream_name() {
        let err = super::ReplayIdentity::new("op", "", seed(1), 0, CheckKind::Invariant, "check")
            .unwrap_err();
        assert_eq!(err.field, "stream_name");
    }

    #[test]
    fn replay_identity_new_rejects_empty_check_name() {
        let err = super::ReplayIdentity::new("op", "stream", seed(1), 0, CheckKind::Invariant, "")
            .unwrap_err();
        assert_eq!(err.field, "check_name");
    }

    #[test]
    fn reproducible_command_renderers_include_all_identity_fields() {
        let cmd = ReproducibleCommand::new(
            "pkg",
            "test",
            super::ReplayIdentity::new(
                "shape.box",
                "stream.box",
                seed(1),
                2,
                CheckKind::MetamorphicRelation,
                "check.box",
            )
            .unwrap(),
        )
        .unwrap();
        let posix = cmd.to_string();
        assert!(posix.contains("AMPHION_TEST_VERSION=3"));
        assert!(posix.contains("AMPHION_TEST_OPERATION=shape.box"));
        assert!(posix.contains("AMPHION_TEST_CHECK_KIND=metamorphic_relation"));
        assert!(posix.contains("AMPHION_TEST_CHECK=check.box"));

        let ps = cmd.as_powershell().to_string();
        assert!(ps.contains("$env:AMPHION_TEST_VERSION='3'"));
        assert!(ps.contains("$env:AMPHION_TEST_OPERATION='shape.box'"));
        assert!(ps.contains("$env:AMPHION_TEST_CHECK_KIND='metamorphic_relation'"));
        assert!(ps.contains("$env:AMPHION_TEST_CHECK='check.box'"));
    }

    #[test]
    fn failure_report_round_trips_check_kind_and_name() {
        let s = seed(24);
        let id = CaseId::new(s.for_case_stream("roundtrip.check"), 1);
        let report = FailureReport::new(
            s,
            id,
            "roundtrip.check",
            "op",
            1,
            CheckKind::Invariant,
            "check.name",
            serde_json::json!({}),
            "fail",
        )
        .unwrap();
        let decoded: FailureReport = serde_json::from_str(&report.to_json().unwrap()).unwrap();
        assert_eq!(decoded.check_kind(), CheckKind::Invariant);
        assert_eq!(decoded.check_name(), "check.name");
    }

    #[test]
    fn with_replay_command_rejects_mismatched_check_kind() {
        let s = seed(25);
        let id = CaseId::new(s.for_case_stream("mismatch.kind"), 0);
        let report = FailureReport::new(
            s,
            id,
            "mismatch.kind",
            "op",
            0,
            CheckKind::Invariant,
            "check",
            serde_json::json!({}),
            "fail",
        )
        .unwrap();
        let cmd = ReproducibleCommand::new(
            "pkg",
            "test",
            super::ReplayIdentity::new("op", "mismatch.kind", s, 0, CheckKind::Property, "check")
                .unwrap(),
        )
        .unwrap();
        assert!(matches!(
            report.with_replay_command(cmd),
            Err(ReportError::CommandMismatch {
                field: "check_kind",
                ..
            })
        ));
    }

    #[test]
    fn with_replay_command_rejects_mismatched_check_name() {
        let s = seed(26);
        let id = CaseId::new(s.for_case_stream("mismatch.name"), 0);
        let report = FailureReport::new(
            s,
            id,
            "mismatch.name",
            "op",
            0,
            CheckKind::Invariant,
            "expected.check",
            serde_json::json!({}),
            "fail",
        )
        .unwrap();
        let cmd = ReproducibleCommand::new(
            "pkg",
            "test",
            super::ReplayIdentity::new(
                "op",
                "mismatch.name",
                s,
                0,
                CheckKind::Invariant,
                "other.check",
            )
            .unwrap(),
        )
        .unwrap();
        assert!(matches!(
            report.with_replay_command(cmd),
            Err(ReportError::CommandMismatch {
                field: "check_name",
                ..
            })
        ));
    }

    #[test]
    fn replay_identity_new_accepts_valid_fields() {
        let identity = super::ReplayIdentity::new(
            "shape.box",
            "stream.box",
            seed(2),
            3,
            CheckKind::Property,
            "check.box",
        )
        .unwrap();
        assert_eq!(identity.operation(), "shape.box");
        assert_eq!(identity.stream_name(), "stream.box");
        assert_eq!(identity.check_kind(), CheckKind::Property);
        assert_eq!(identity.check_name(), "check.box");
    }

    #[test]
    fn replay_identity_fields_are_private_inaccessible_directly() {
        let identity = super::ReplayIdentity::new(
            "shape.box",
            "stream.box",
            seed(2),
            3,
            CheckKind::Property,
            "check.box",
        )
        .unwrap();
        assert_eq!(
            identity.case_sequence_version(),
            crate::rng::CASE_SEQUENCE_VERSION
        );
        assert_eq!(identity.operation(), "shape.box");
        assert_eq!(identity.stream_name(), "stream.box");
        assert_eq!(identity.seed(), seed(2));
        assert_eq!(identity.case_index(), 3);
        assert_eq!(identity.check_kind(), CheckKind::Property);
        assert_eq!(identity.check_name(), "check.box");
    }

    #[test]
    fn replay_identity_stale_version_deserialize_rejected() {
        let identity = super::ReplayIdentity::new(
            "shape.box",
            "stream.box",
            seed(2),
            3,
            CheckKind::Property,
            "check.box",
        )
        .unwrap();
        let mut value = serde_json::to_value(&identity).unwrap();
        value["case_sequence_version"] = serde_json::json!(CASE_SEQUENCE_VERSION - 1);
        let result: Result<super::ReplayIdentity, _> = serde_json::from_value(value);
        assert!(
            result.is_err(),
            "stale case_sequence_version must be rejected"
        );
    }

    #[test]
    fn replay_identity_invalid_token_construction_rejected() {
        assert!(matches!(
            super::ReplayIdentity::new("", "stream", seed(1), 0, CheckKind::Invariant, "check"),
            Err(CommandTokenError {
                field: "operation",
                ..
            })
        ));
        assert!(matches!(
            super::ReplayIdentity::new(
                "bad operation",
                "stream",
                seed(1),
                0,
                CheckKind::Invariant,
                "check",
            ),
            Err(CommandTokenError {
                field: "operation",
                ..
            })
        ));
    }

    #[test]
    fn reproducible_command_self_roundtrip() {
        let command = ReproducibleCommand::new(
            "pkg",
            "test",
            super::ReplayIdentity::new(
                "shape.box",
                "stream.box",
                seed(1),
                2,
                CheckKind::Invariant,
                "check.box",
            )
            .unwrap(),
        )
        .unwrap();
        let json = serde_json::to_string(&command).unwrap();
        let decoded: ReproducibleCommand = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, command);
    }
}
