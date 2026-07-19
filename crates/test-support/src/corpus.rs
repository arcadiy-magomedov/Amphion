//! Permanent regression-corpus file format with schema versioning and
//! minimisation metadata.
//!
//! A [`CorpusFile`] is a JSON document that holds a collection of
//! [`CorpusEntry`] values, each representing a minimised failing test case
//! that must remain reproducible forever. Loading validates the schema version
//! and rejects any malformed or incompatibly versioned file before returning.
//!
//! # Schema versioning
//!
//! The current supported version is [`CORPUS_SCHEMA_VERSION`] (1.2).
//! [`CorpusFile::load_from_str`] accepts only current-schema files. Legacy
//! v1.0/v1.1 documents load through [`LegacyCorpusDocument`], which preserves
//! their exact original bytes for byte-identical write-back.
//!
//! # Canonical ordering
//!
//! Entries within a file are always persisted in lexicographic order of their
//! [`CaseId`] hex string. [`CorpusFile::write_to_string`] always writes them
//! sorted; [`CorpusFile::load_from_str`] validates the order and rejects
//! unordered files.
//!
//! # Replay context
//!
//! Every entry records `stream_name` (the RNG stream used when the failure
//! was found) in addition to `seed` and `case_index`.  Together, these three
//! fields uniquely identify the case for replay even in the (unlikely) event
//! of a stream-seed hash collision.

use core::{error::Error, fmt};

use amphion_foundation::SchemaVersion;
use serde::{Deserialize, Serialize};

use crate::fuzz::CheckKind;
use crate::rng::{CASE_SEQUENCE_VERSION, CaseId, TestSeed};

/// The corpus schema version produced by this build of the harness.
pub const CORPUS_SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 2);

// ── CorpusError ────────────────────────────────────────────────────────────

/// Error produced when loading or constructing a corpus entry/file.
#[derive(Debug)]
pub enum CorpusError {
    /// The input is not valid JSON.
    InvalidJson(String),
    /// The file's schema version is incompatible (wrong major or unsupported
    /// minor).
    SchemaVersionMismatch {
        /// Version found in the file.
        found: SchemaVersion,
        /// Version supported by this harness build.
        supported: SchemaVersion,
    },
    /// The file is a legacy schema version and must be loaded via
    /// [`LegacyCorpusDocument`].
    LegacyVersion {
        /// Version found in the file.
        found: SchemaVersion,
    },
    /// An individual entry could not be validated.
    MalformedEntry {
        /// Zero-based entry index.
        index: usize,
        /// Reason the entry is invalid.
        reason: String,
    },
    /// Two entries share the same [`CaseId`], which is not allowed.
    DuplicateCaseId {
        /// Hex representation of the duplicated ID.
        id_hex: String,
        /// Index of the first occurrence.
        first_index: usize,
        /// Index of the duplicate.
        second_index: usize,
    },
    /// Entries are not in canonical (lexicographic `CaseId`) order.
    UnorderedEntries {
        /// Zero-based index of the first out-of-order entry.
        index: usize,
    },
    /// The `operation` field is empty or otherwise invalid.
    InvalidOperation(String),
    /// A field contains invalid characters (not a stable token `[a-zA-Z0-9._:/-]`).
    InvalidToken {
        /// The field that failed validation.
        field: &'static str,
        /// The rejected value.
        value: String,
    },
    /// The stored [`CaseId`] does not match the value derived from
    /// `CaseId::new(seed.for_case_stream(stream_name), case_index)`.
    CaseIdMismatch {
        /// The ID stored in the entry.
        actual: CaseId,
        /// The ID expected from the other fields.
        expected: CaseId,
    },
    /// A v1.2 entry omitted `check_kind`.
    MissingCheckKind {
        /// Zero-based entry index.
        index: usize,
    },
    /// A v1.2 entry omitted `check_name`.
    MissingCheckName {
        /// Zero-based entry index.
        index: usize,
    },
    /// A v1.2 entry used an invalid `check_kind` string.
    InvalidCheckKind {
        /// Zero-based entry index.
        index: usize,
        /// The invalid string value.
        value: String,
    },
    /// The migration callback returned identity fields that contradict the
    /// known fields already present in a v1.1 entry.
    MigrationIdentityMismatch {
        /// Zero-based entry index.
        index: usize,
        /// The field that was overridden.
        field: &'static str,
        /// The value already in the v1.1 entry.
        expected: String,
        /// The value the callback tried to set.
        found: String,
    },
}

impl fmt::Display for CorpusError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidJson(msg) => write!(f, "corpus file is not valid JSON: {msg}"),
            Self::SchemaVersionMismatch { found, supported } => write!(
                f,
                "corpus schema version {}.{} is incompatible with supported \
                 version {}.{}",
                found.major(),
                found.minor(),
                supported.major(),
                supported.minor(),
            ),
            Self::LegacyVersion { found } => write!(
                f,
                "corpus schema version {}.{} is legacy; load it via LegacyCorpusDocument",
                found.major(),
                found.minor(),
            ),
            Self::MalformedEntry { index, reason } => {
                write!(f, "entry {index} is malformed: {reason}")
            }
            Self::DuplicateCaseId {
                id_hex,
                first_index,
                second_index,
            } => write!(
                f,
                "duplicate CaseId {id_hex} at entries {first_index} and {second_index}"
            ),
            Self::UnorderedEntries { index } => write!(
                f,
                "entry {index} is out of canonical CaseId order; \
                 entries must be sorted lexicographically by CaseId hex"
            ),
            Self::InvalidOperation(msg) => write!(f, "invalid operation label: {msg}"),
            Self::InvalidToken { field, value } => {
                write!(
                    f,
                    "invalid field `{field}`: {value:?} is not a stable token"
                )
            }
            Self::CaseIdMismatch { actual, expected } => write!(
                f,
                "CaseId {} does not match derived id {} \
                 (CaseId::new(seed.for_case_stream(stream_name), case_index))",
                actual.to_hex(),
                expected.to_hex(),
            ),
            Self::MissingCheckKind { index } => {
                write!(f, "entry {index} is missing check_kind for schema v1.2")
            }
            Self::MissingCheckName { index } => {
                write!(f, "entry {index} is missing check_name for schema v1.2")
            }
            Self::InvalidCheckKind { index, value } => {
                write!(f, "entry {index} has invalid check_kind {value:?}")
            }
            Self::MigrationIdentityMismatch {
                index,
                field,
                expected,
                found,
            } => write!(
                f,
                "entry {index} migration callback changed {field} from {expected:?} to {found:?}"
            ),
        }
    }
}

impl Error for CorpusError {}

// ── MinimizationMeta ───────────────────────────────────────────────────────

/// Provenance of a minimised failure: the seed and index of the original
/// discovery, and how many shrink steps were taken.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MinimizationMeta {
    /// Seed of the original (un-minimised) failing run.
    pub original_seed: TestSeed,
    /// Case index within the original run.
    pub original_case_index: u64,
    /// Number of shrink steps applied during minimisation.
    pub shrink_steps: u32,
}

impl MinimizationMeta {
    /// Creates a minimisation provenance record.
    #[must_use]
    pub const fn new(original_seed: TestSeed, original_case_index: u64, shrink_steps: u32) -> Self {
        Self {
            original_seed,
            original_case_index,
            shrink_steps,
        }
    }
}

// ── CorpusEntry (serde repr) ───────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorpusEntryRepr {
    schema_version: SchemaVersion,
    id: CaseId,
    operation: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    stream_name: Option<String>,
    seed: TestSeed,
    case_index: u64,
    /// Raw wire value: v1.0 encodes this as a JSON **string** containing
    /// encoded JSON; v1.1+ stores it as an inline JSON value. Which shape is
    /// valid depends on `schema_version.minor()`, which is only known once
    /// this repr has been fully parsed — so no deserializer callback can
    /// safely branch on it. [`TryFrom<CorpusEntryRepr>`] performs the
    /// version-aware conversion after the entry's own schema version has
    /// been read (see Issue 7 in the module-level version-history notes).
    inputs_json: serde_json::Value,
    failure_message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    minimization: Option<MinimizationMeta>,
    /// The [`crate::rng::CASE_SEQUENCE_VERSION`] in effect when `id` was
    /// derived. Required (must be `Some`) for v1.1+ entries; ignored (may be
    /// absent) for v1.0 entries, which never validated `id` against a
    /// versioned domain key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    case_sequence_version: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    check_kind: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    check_name: Option<String>,
}

// ── CorpusEntry ────────────────────────────────────────────────────────────

/// A single minimised failing test case stored in the permanent regression
/// corpus.
///
/// Each entry is self-contained: given `seed`, `case_index`, `stream_name`,
/// and `inputs_json` a developer can reproduce the failure without additional
/// context.
#[derive(Clone, Debug, PartialEq)]
pub struct CorpusEntry {
    schema_version: SchemaVersion,
    id: CaseId,
    operation: String,
    /// Stream name used during the failing run. Recorded for unambiguous
    /// replay alongside `seed` and `case_index`.
    stream_name: Option<String>,
    seed: TestSeed,
    case_index: u64,
    /// Input values as a structured JSON value (not a double-encoded string).
    inputs_json: serde_json::Value,
    failure_message: String,
    minimization: Option<MinimizationMeta>,
    check_kind: Option<CheckKind>,
    check_name: Option<String>,
}

impl CorpusEntry {
    /// Creates a new v1.2 corpus entry with validated fields.
    ///
    /// - `operation` must be a non-empty stable token (`[a-zA-Z0-9._:/-]`).
    /// - `stream_name` must be a non-empty stable token.
    /// - `id` must equal `CaseId::new(seed.for_case_stream(stream_name), case_index)`.
    ///
    /// For v1.0 legacy entries migrated from file (where `stream_name` may be
    /// absent), use `TryFrom<CorpusEntryRepr>` instead.
    ///
    /// # Errors
    ///
    /// Returns [`CorpusError::InvalidToken`] when `operation` or `stream_name`
    /// is empty or contains characters outside `[a-zA-Z0-9._:/-]`.
    ///
    /// Returns [`CorpusError::CaseIdMismatch`] when `id` does not equal the
    /// value derived from `CaseId::new(seed.for_case_stream(stream_name), case_index)`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: CaseId,
        operation: impl Into<String>,
        stream_name: impl Into<String>,
        seed: TestSeed,
        case_index: u64,
        check_kind: CheckKind,
        check_name: impl Into<String>,
        inputs_json: serde_json::Value,
        failure_message: impl Into<String>,
    ) -> Result<Self, CorpusError> {
        let operation = operation.into();
        let stream_name = stream_name.into();
        let check_name = check_name.into();

        if !crate::is_stable_token(&operation) {
            return Err(CorpusError::InvalidToken {
                field: "operation",
                value: operation,
            });
        }
        if !crate::is_stable_token(&stream_name) {
            return Err(CorpusError::InvalidToken {
                field: "stream_name",
                value: stream_name,
            });
        }

        if !crate::is_stable_token(&check_name) {
            return Err(CorpusError::InvalidToken {
                field: "check_name",
                value: check_name,
            });
        }

        let expected_id = CaseId::new(seed.for_case_stream(&stream_name), case_index);
        if id != expected_id {
            return Err(CorpusError::CaseIdMismatch {
                actual: id,
                expected: expected_id,
            });
        }

        Ok(Self {
            schema_version: CORPUS_SCHEMA_VERSION,
            id,
            operation,
            stream_name: Some(stream_name),
            seed,
            case_index,
            inputs_json,
            failure_message: failure_message.into(),
            minimization: None,
            check_kind: Some(check_kind),
            check_name: Some(check_name),
        })
    }

    /// Attaches minimisation provenance to this entry.
    #[must_use]
    pub fn with_minimization(mut self, meta: MinimizationMeta) -> Self {
        self.minimization = Some(meta);
        self
    }

    /// Returns the schema version recorded in this entry.
    #[must_use]
    pub const fn schema_version(&self) -> SchemaVersion {
        self.schema_version
    }

    /// Returns the deterministic case identifier.
    #[must_use]
    pub const fn id(&self) -> CaseId {
        self.id
    }

    /// Returns the operation label.
    #[must_use]
    pub fn operation(&self) -> &str {
        &self.operation
    }

    /// Returns the stream name, if recorded.
    #[must_use]
    pub fn stream_name(&self) -> Option<&str> {
        self.stream_name.as_deref()
    }

    /// Returns the primary seed for this failing run.
    #[must_use]
    pub const fn seed(&self) -> TestSeed {
        self.seed
    }

    /// Returns the case index within the stream.
    #[must_use]
    pub const fn case_index(&self) -> u64 {
        self.case_index
    }

    /// Returns the structured input values.
    #[must_use]
    pub fn inputs_json(&self) -> &serde_json::Value {
        &self.inputs_json
    }

    /// Returns the human-readable failure message.
    #[must_use]
    pub fn failure_message(&self) -> &str {
        &self.failure_message
    }

    /// Returns the minimisation provenance if this entry was minimised.
    #[must_use]
    pub const fn minimization(&self) -> Option<&MinimizationMeta> {
        self.minimization.as_ref()
    }

    /// Returns the check kind, if recorded.
    #[must_use]
    pub const fn check_kind(&self) -> Option<CheckKind> {
        self.check_kind
    }

    /// Returns the check name, if recorded.
    #[must_use]
    pub fn check_name(&self) -> Option<&str> {
        self.check_name.as_deref()
    }

    /// Serialises this entry using the **legacy v1.0 wire format**.
    ///
    /// Unlike [`CorpusFile::write_to_string`] (which always writes the
    /// current v1.2 format), this produces a standalone entry JSON object
    /// compatible with schema version 1.0: `schema_version` is forced to
    /// `1.0`, `inputs_json` is double-encoded as a JSON **string** (matching
    /// the historical v1.0 wire shape), `stream_name` is omitted, and
    /// `case_sequence_version` is omitted (v1.0 never validated `id` against
    /// a versioned domain key). Intended for tests and tooling that exercise
    /// backward-compatible loading of older corpus entries.
    ///
    /// # Errors
    ///
    /// Returns a `serde_json` error if serialisation fails (unreachable in
    /// practice given the validated field types).
    pub fn write_as_legacy(&self) -> Result<String, serde_json::Error> {
        let legacy = CorpusEntryRepr {
            schema_version: SchemaVersion::new(1, 0),
            id: self.id,
            operation: self.operation.clone(),
            stream_name: None,
            seed: self.seed,
            case_index: self.case_index,
            inputs_json: serde_json::Value::String(serde_json::to_string(&self.inputs_json)?),
            failure_message: self.failure_message.clone(),
            minimization: self.minimization.clone(),
            case_sequence_version: None,
            check_kind: None,
            check_name: None,
        };
        serde_json::to_string_pretty(&legacy)
    }
}

impl From<CorpusEntry> for CorpusEntryRepr {
    fn from(e: CorpusEntry) -> Self {
        let is_v1_0 = e.schema_version.minor() == 0;
        // v1.0 wire format double-encodes inputs_json as a JSON string;
        // v1.1+ stores it as an inline value.  Preserve the correct wire
        // representation so that a load→write→load round-trip is lossless.
        let inputs_json = if is_v1_0 {
            serde_json::Value::String(
                serde_json::to_string(&e.inputs_json)
                    .expect("inputs_json is a valid serde_json::Value"),
            )
        } else {
            e.inputs_json
        };
        let case_sequence_version = if is_v1_0 {
            None
        } else {
            Some(CASE_SEQUENCE_VERSION)
        };
        let is_v1_2 = e.schema_version.minor() >= 2;
        Self {
            schema_version: e.schema_version,
            id: e.id,
            operation: e.operation,
            stream_name: e.stream_name,
            seed: e.seed,
            case_index: e.case_index,
            inputs_json,
            failure_message: e.failure_message,
            minimization: e.minimization,
            case_sequence_version,
            check_kind: if is_v1_2 {
                e.check_kind.map(|kind| kind.to_string())
            } else {
                None
            },
            check_name: if is_v1_2 { e.check_name } else { None },
        }
    }
}

#[allow(clippy::too_many_lines)]
fn corpus_entry_from_repr(r: CorpusEntryRepr, index: usize) -> Result<CorpusEntry, CorpusError> {
    if !crate::is_stable_token(&r.operation) {
        return Err(CorpusError::InvalidToken {
            field: "operation",
            value: r.operation,
        });
    }

    let minor = r.schema_version.minor();
    let is_v1_1_or_newer = minor >= 1;
    let is_v1_2 = minor >= 2;

    let inputs_json = if is_v1_1_or_newer {
        // v1.1+ accepts any JSON value including strings — they are ordinary
        // input values.
        r.inputs_json
    } else {
        match r.inputs_json {
            serde_json::Value::String(s) => {
                serde_json::from_str(&s).map_err(|e| CorpusError::MalformedEntry {
                    index,
                    reason: format!("v1.0 inputs_json string is not valid JSON: {e}"),
                })?
            }
            _ => {
                return Err(CorpusError::MalformedEntry {
                    index,
                    reason: "v1.0 inputs_json must be a JSON string".to_owned(),
                });
            }
        }
    };

    if is_v1_1_or_newer {
        let stream_name = r.stream_name.as_deref().unwrap_or("");
        if !crate::is_stable_token(stream_name) {
            return Err(CorpusError::InvalidToken {
                field: "stream_name",
                value: stream_name.to_owned(),
            });
        }
        match r.case_sequence_version {
            Some(v) if v == CASE_SEQUENCE_VERSION => {}
            Some(v) => {
                return Err(CorpusError::MalformedEntry {
                    index,
                    reason: format!(
                        "case_sequence_version {v} does not match current \
                         CASE_SEQUENCE_VERSION {CASE_SEQUENCE_VERSION}"
                    ),
                });
            }
            None => {
                return Err(CorpusError::MalformedEntry {
                    index,
                    reason: "v1.1+ entries must specify case_sequence_version".to_owned(),
                });
            }
        }
        let expected = CaseId::new(r.seed.for_case_stream(stream_name), r.case_index);
        if r.id != expected {
            return Err(CorpusError::CaseIdMismatch {
                actual: r.id,
                expected,
            });
        }
    }

    let check_kind = if is_v1_2 {
        let raw = r
            .check_kind
            .ok_or(CorpusError::MissingCheckKind { index })?;
        Some(
            raw.parse::<CheckKind>()
                .map_err(|_| CorpusError::InvalidCheckKind {
                    index,
                    value: raw.clone(),
                })?,
        )
    } else {
        match r.check_kind {
            Some(raw) => {
                Some(
                    raw.parse::<CheckKind>()
                        .map_err(|_| CorpusError::InvalidCheckKind {
                            index,
                            value: raw.clone(),
                        })?,
                )
            }
            None => None,
        }
    };

    let check_name = if is_v1_2 {
        let value = r
            .check_name
            .ok_or(CorpusError::MissingCheckName { index })?;
        if !crate::is_stable_token(&value) {
            return Err(CorpusError::InvalidToken {
                field: "check_name",
                value,
            });
        }
        Some(value)
    } else {
        match r.check_name {
            Some(value) => {
                if !crate::is_stable_token(&value) {
                    return Err(CorpusError::InvalidToken {
                        field: "check_name",
                        value,
                    });
                }
                Some(value)
            }
            None => None,
        }
    };

    Ok(CorpusEntry {
        schema_version: r.schema_version,
        id: r.id,
        operation: r.operation,
        stream_name: r.stream_name,
        seed: r.seed,
        case_index: r.case_index,
        inputs_json,
        failure_message: r.failure_message,
        minimization: r.minimization,
        check_kind,
        check_name,
    })
}

impl TryFrom<CorpusEntryRepr> for CorpusEntry {
    type Error = CorpusError;

    fn try_from(r: CorpusEntryRepr) -> Result<Self, Self::Error> {
        corpus_entry_from_repr(r, 0)
    }
}

// ── CorpusFile (serde repr) ────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorpusFileRepr {
    schema_version: SchemaVersion,
    entries: Vec<CorpusEntryRepr>,
}

fn validate_canonical_entries(entries: &[CorpusEntry]) -> Result<(), CorpusError> {
    for i in 1..entries.len() {
        let prev_hex = entries[i - 1].id.to_hex();
        let curr_hex = entries[i].id.to_hex();
        if curr_hex == prev_hex {
            return Err(CorpusError::DuplicateCaseId {
                id_hex: curr_hex,
                first_index: i - 1,
                second_index: i,
            });
        }
        if curr_hex < prev_hex {
            return Err(CorpusError::UnorderedEntries { index: i });
        }
    }
    Ok(())
}

fn load_entries_from_repr(
    found: SchemaVersion,
    entry_reprs: Vec<CorpusEntryRepr>,
) -> Result<Vec<CorpusEntry>, CorpusError> {
    let entries = entry_reprs
        .into_iter()
        .enumerate()
        .map(|(i, entry_repr)| {
            if entry_repr.schema_version != found {
                return Err(CorpusError::MalformedEntry {
                    index: i,
                    reason: format!(
                        "entry schema version {}.{} does not match file version {}.{}",
                        entry_repr.schema_version.major(),
                        entry_repr.schema_version.minor(),
                        found.major(),
                        found.minor(),
                    ),
                });
            }
            corpus_entry_from_repr(entry_repr, i).map_err(|e| match e {
                CorpusError::MalformedEntry { .. }
                | CorpusError::MissingCheckKind { .. }
                | CorpusError::MissingCheckName { .. }
                | CorpusError::InvalidCheckKind { .. } => e,
                other => CorpusError::MalformedEntry {
                    index: i,
                    reason: other.to_string(),
                },
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    validate_canonical_entries(&entries)?;
    Ok(entries)
}

#[derive(Clone, Debug, PartialEq)]
/// A loaded legacy corpus document (schema v1.0 or v1.1) that preserves the
/// exact original JSON bytes for byte-identical write-back.
pub struct LegacyCorpusDocument {
    schema_version: SchemaVersion,
    raw: String,
}

impl LegacyCorpusDocument {
    /// Loads a legacy v1.0 or v1.1 corpus document, preserving the exact bytes.
    ///
    /// # Errors
    ///
    /// Returns [`CorpusError::SchemaVersionMismatch`] when the file is not a
    /// legacy v1.x document.
    pub fn load_from_str(json: &str) -> Result<Self, CorpusError> {
        let repr: CorpusFileRepr =
            serde_json::from_str(json).map_err(|e| CorpusError::InvalidJson(e.to_string()))?;
        let found = repr.schema_version;
        if found.major() != CORPUS_SCHEMA_VERSION.major() {
            return Err(CorpusError::SchemaVersionMismatch {
                found,
                supported: CORPUS_SCHEMA_VERSION,
            });
        }
        if found.minor() >= CORPUS_SCHEMA_VERSION.minor() {
            return Err(CorpusError::SchemaVersionMismatch {
                found,
                supported: SchemaVersion::new(1, 1),
            });
        }
        Ok(Self {
            schema_version: found,
            raw: json.to_owned(),
        })
    }

    /// Returns the exact original JSON bytes.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.raw
    }

    /// Returns the schema version of this legacy document.
    #[must_use]
    pub const fn schema_version(&self) -> SchemaVersion {
        self.schema_version
    }

    /// Migrates this legacy document to the current corpus schema.
    ///
    /// # Errors
    ///
    /// Returns [`CorpusError`] when migration fails or produces invalid
    /// current-schema entries.
    pub fn migrate_to_current<F>(&self, migration_fn: F) -> Result<CorpusFile, CorpusError>
    where
        F: Fn(usize, TestSeed, u64) -> Result<(String, String, CheckKind, String), CorpusError>,
    {
        let repr: CorpusFileRepr =
            serde_json::from_str(&self.raw).map_err(|e| CorpusError::InvalidJson(e.to_string()))?;
        let mut entries = Vec::with_capacity(repr.entries.len());
        for (i, entry_repr) in repr.entries.into_iter().enumerate() {
            if self.schema_version.minor() >= 1 {
                let known_stream_name = entry_repr.stream_name.as_deref().unwrap_or("");
                let expected_id = CaseId::new(
                    entry_repr.seed.for_case_stream(known_stream_name),
                    entry_repr.case_index,
                );
                if entry_repr.id != expected_id {
                    return Err(CorpusError::CaseIdMismatch {
                        actual: entry_repr.id,
                        expected: expected_id,
                    });
                }
            }
            let (operation, stream_name, check_kind, check_name) =
                migration_fn(i, entry_repr.seed, entry_repr.case_index).map_err(|e| {
                    CorpusError::MalformedEntry {
                        index: i,
                        reason: e.to_string(),
                    }
                })?;
            if self.schema_version.minor() >= 1 {
                if operation != entry_repr.operation {
                    return Err(CorpusError::MigrationIdentityMismatch {
                        index: i,
                        field: "operation",
                        expected: entry_repr.operation.clone(),
                        found: operation,
                    });
                }
                let expected_stream_name = entry_repr.stream_name.as_deref().unwrap_or("");
                if stream_name != expected_stream_name {
                    return Err(CorpusError::MigrationIdentityMismatch {
                        index: i,
                        field: "stream_name",
                        expected: expected_stream_name.to_owned(),
                        found: stream_name,
                    });
                }
            }
            let inputs_json = match self.schema_version.minor() {
                0 => match entry_repr.inputs_json {
                    serde_json::Value::String(s) => {
                        serde_json::from_str(&s).map_err(|e| CorpusError::MalformedEntry {
                            index: i,
                            reason: e.to_string(),
                        })?
                    }
                    _ => {
                        return Err(CorpusError::MalformedEntry {
                            index: i,
                            reason: "v1.0 inputs_json must be string".to_owned(),
                        });
                    }
                },
                _ => entry_repr.inputs_json,
            };
            let id = CaseId::new(
                entry_repr.seed.for_case_stream(&stream_name),
                entry_repr.case_index,
            );
            entries.push(CorpusEntry::new(
                id,
                operation,
                stream_name,
                entry_repr.seed,
                entry_repr.case_index,
                check_kind,
                check_name,
                inputs_json,
                entry_repr.failure_message,
            )?);
        }
        CorpusFile::new(entries)
    }
}

// ── CorpusFile ─────────────────────────────────────────────────────────────

/// A versioned, canonically-ordered collection of permanent regression corpus
/// entries.
///
/// Load with [`CorpusFile::load_from_str`] and write with
/// [`CorpusFile::write_to_string`]. Both operations validate the schema
/// version and reject incompatible files.
///
/// Entries are always persisted in lexicographic order of their [`CaseId`]
/// hex string so that the file is deterministic regardless of insertion order.
#[derive(Clone, Debug, PartialEq)]
pub struct CorpusFile {
    schema_version: SchemaVersion,
    entries: Vec<CorpusEntry>,
}

impl CorpusFile {
    /// Creates a corpus file from an unsorted list of entries.
    ///
    /// Entries are sorted into canonical order on construction.
    ///
    /// # Errors
    ///
    /// Returns [`CorpusError::DuplicateCaseId`] when two entries share the
    /// same [`CaseId`].
    pub fn new(mut entries: Vec<CorpusEntry>) -> Result<Self, CorpusError> {
        entries.sort_by_key(|e| e.id.to_hex());
        validate_canonical_entries(&entries)?;
        Ok(Self {
            schema_version: CORPUS_SCHEMA_VERSION,
            entries,
        })
    }

    /// Returns the schema version recorded in this file.
    #[must_use]
    pub const fn schema_version(&self) -> SchemaVersion {
        self.schema_version
    }

    /// Returns the corpus entries in canonical order.
    #[must_use]
    pub fn entries(&self) -> &[CorpusEntry] {
        &self.entries
    }

    /// Consumes the file and returns the owned entry list (canonical order).
    #[must_use]
    pub fn into_entries(self) -> Vec<CorpusEntry> {
        self.entries
    }

    /// Appends a new entry, maintaining canonical ordering.
    ///
    /// # Errors
    ///
    /// Returns [`CorpusError::DuplicateCaseId`] when `entry.id()` is already
    /// present in the file.
    pub fn push(&mut self, entry: CorpusEntry) -> Result<(), CorpusError> {
        let hex = entry.id.to_hex();
        let pos = self.entries.partition_point(|e| e.id.to_hex() < hex);
        if pos < self.entries.len() && self.entries[pos].id == entry.id {
            return Err(CorpusError::DuplicateCaseId {
                id_hex: hex,
                first_index: pos,
                second_index: pos,
            });
        }
        self.entries.insert(pos, entry);
        Ok(())
    }

    /// Parses and validates a corpus file from a JSON string.
    ///
    /// # Errors
    ///
    /// - [`CorpusError::InvalidJson`] — not valid JSON or missing fields.
    /// - [`CorpusError::SchemaVersionMismatch`] — major version ≠ 1, or
    ///   minor version > [`CORPUS_SCHEMA_VERSION`].
    /// - [`CorpusError::MalformedEntry`] — individual entry validation failed.
    /// - [`CorpusError::InvalidOperation`] — empty operation label.
    /// - [`CorpusError::DuplicateCaseId`] — two entries share a `CaseId`.
    /// - [`CorpusError::UnorderedEntries`] — entries are not in canonical order.
    pub fn load_from_str(json: &str) -> Result<Self, CorpusError> {
        let repr: CorpusFileRepr =
            serde_json::from_str(json).map_err(|e| CorpusError::InvalidJson(e.to_string()))?;

        let found = repr.schema_version;
        if found.major() != CORPUS_SCHEMA_VERSION.major() {
            return Err(CorpusError::SchemaVersionMismatch {
                found,
                supported: CORPUS_SCHEMA_VERSION,
            });
        }
        if found.minor() > CORPUS_SCHEMA_VERSION.minor() {
            return Err(CorpusError::SchemaVersionMismatch {
                found,
                supported: CORPUS_SCHEMA_VERSION,
            });
        }
        if found.minor() < CORPUS_SCHEMA_VERSION.minor() {
            return Err(CorpusError::LegacyVersion { found });
        }
        let entries = load_entries_from_repr(found, repr.entries)?;

        Ok(Self {
            schema_version: found,
            entries,
        })
    }

    /// Serialises the corpus file to a deterministic pretty-printed JSON
    /// string.
    ///
    /// Entries are written in canonical (lexicographic `CaseId`) order so the
    /// output is byte-for-byte identical for identical content.
    ///
    /// # Errors
    ///
    /// Returns a `serde_json` error when serialisation fails (unreachable in
    /// practice given the validated field types).
    pub fn write_to_string(&self) -> Result<String, serde_json::Error> {
        validate_canonical_entries(&self.entries)
            .map_err(|e| serde_json::Error::io(std::io::Error::other(e.to_string())))?;
        let repr = CorpusFileRepr {
            schema_version: self.schema_version,
            entries: self
                .entries
                .iter()
                .cloned()
                .map(CorpusEntryRepr::from)
                .collect(),
        };
        serde_json::to_string_pretty(&repr)
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{
        CORPUS_SCHEMA_VERSION, CorpusEntry, CorpusError, CorpusFile, LegacyCorpusDocument,
        MinimizationMeta,
    };
    use crate::fuzz::CheckKind;
    use crate::rng::{CASE_SEQUENCE_VERSION, CaseId, TestSeed};
    use amphion_foundation::SchemaVersion;

    fn new_entry(
        id: CaseId,
        operation: &str,
        stream_name: &str,
        seed: TestSeed,
        case_index: u64,
        inputs_json: serde_json::Value,
        failure_message: impl Into<String>,
    ) -> Result<CorpusEntry, CorpusError> {
        CorpusEntry::new(
            id,
            operation,
            stream_name,
            seed,
            case_index,
            CheckKind::Invariant,
            "check",
            inputs_json,
            failure_message,
        )
    }

    fn make_entry() -> CorpusEntry {
        let seed = TestSeed::new(42);
        let id = CaseId::new(seed.for_case_stream("corpus.test"), 7);
        new_entry(
            id,
            "primitive.cuboid",
            "corpus.test",
            seed,
            7,
            serde_json::json!({"width": 1.0}),
            "orientation violated",
        )
        .expect("valid entry")
        .with_minimization(MinimizationMeta::new(TestSeed::new(9999), 1234, 3))
    }

    #[test]
    fn corpus_file_round_trips_through_json() {
        let file = CorpusFile::new(vec![make_entry()]).expect("valid corpus");
        let json = file.write_to_string().expect("serialise");
        let loaded = CorpusFile::load_from_str(&json).expect("round-trip must succeed");
        assert_eq!(file, loaded, "loaded corpus must match original");
    }

    #[test]
    fn minimization_metadata_is_preserved() {
        let entry = make_entry();
        let meta = entry.minimization().expect("meta must be present");
        assert_eq!(meta.original_seed, TestSeed::new(9999));
        assert_eq!(meta.original_case_index, 1234);
        assert_eq!(meta.shrink_steps, 3);

        let file = CorpusFile::new(vec![entry]).expect("valid corpus");
        let json = file.write_to_string().expect("serialise");
        let loaded = CorpusFile::load_from_str(&json).expect("round-trip");
        let loaded_meta = loaded.entries()[0].minimization().expect("meta after RT");
        assert_eq!(loaded_meta.original_seed, TestSeed::new(9999));
        assert_eq!(loaded_meta.shrink_steps, 3);
    }

    #[test]
    fn malformed_json_is_rejected() {
        let result = CorpusFile::load_from_str("not json at all");
        assert!(
            matches!(result, Err(CorpusError::InvalidJson(_))),
            "non-JSON must produce InvalidJson"
        );
    }

    #[test]
    fn missing_required_field_is_rejected() {
        let result = CorpusFile::load_from_str("{}");
        assert!(
            matches!(result, Err(CorpusError::InvalidJson(_))),
            "missing fields must be rejected"
        );
    }

    #[test]
    fn wrong_major_schema_version_is_rejected() {
        let incompatible = r#"{"schema_version":{"major":99,"minor":0},"entries":[]}"#;
        let result = CorpusFile::load_from_str(incompatible);
        assert!(
            matches!(result, Err(CorpusError::SchemaVersionMismatch { .. })),
            "incompatible major version must be rejected"
        );
    }

    #[test]
    fn unknown_top_level_field_is_rejected() {
        let json = r#"{"schema_version":{"major":1,"minor":1},"entries":[],"surprise":"field"}"#;
        let result = CorpusFile::load_from_str(json);
        assert!(
            result.is_err(),
            "unknown fields must be rejected due to deny_unknown_fields"
        );
    }

    #[test]
    fn future_minor_version_is_rejected() {
        let json = r#"{"schema_version":{"major":1,"minor":99},"entries":[]}"#;
        let result = CorpusFile::load_from_str(json);
        assert!(
            matches!(result, Err(CorpusError::SchemaVersionMismatch { .. })),
            "unsupported minor version must be rejected, not assumed forward-compatible"
        );
    }

    #[test]
    fn v1_0_files_load_via_legacy_document() {
        let json = r#"{"schema_version":{"major":1,"minor":0},"entries":[]}"#;
        assert!(matches!(
            CorpusFile::load_from_str(json),
            Err(CorpusError::LegacyVersion { .. })
        ));
        let result = LegacyCorpusDocument::load_from_str(json);
        assert!(result.is_ok(), "v1.0 files must load as legacy documents");
    }

    #[test]
    fn entry_schema_version_mismatch_is_rejected() {
        let valid_id = "00000000000000000000000000000000";
        let json = format!(
            concat!(
                r#"{{"schema_version":{{"major":1,"minor":0}},"entries":["#,
                r#"{{"schema_version":{{"major":1,"minor":1}},"id":"{id}","#,
                r#""operation":"op","seed":1,"case_index":0,
                    "inputs_json":{{}},"failure_message":"fail"}}"#,
                r#"]}}"#,
            ),
            id = valid_id
        );
        let result = CorpusFile::load_from_str(&json);
        assert!(
            result.is_err(),
            "entry version mismatch with file version must be rejected"
        );
    }

    #[test]
    fn duplicate_case_ids_are_rejected() {
        let seed = TestSeed::new(1);
        // Use for_case_stream so CaseId is consistent with stream_name.
        let id = CaseId::new(seed.for_case_stream("s"), 0);
        let e1 = new_entry(id, "op.a", "s", seed, 0, serde_json::json!({}), "fail a").unwrap();
        let e2 = new_entry(id, "op.b", "s", seed, 0, serde_json::json!({}), "fail b").unwrap();
        let result = CorpusFile::new(vec![e1, e2]);
        assert!(
            matches!(result, Err(CorpusError::DuplicateCaseId { .. })),
            "duplicate CaseIds must be rejected"
        );
    }

    #[test]
    fn unordered_entries_are_rejected_on_load() {
        let sa = TestSeed::new(0xff);
        let sb = TestSeed::new(0x01);
        let id_a = CaseId::new(sa.for_case_stream("s"), 0);
        let id_b = CaseId::new(sb.for_case_stream("s"), 0);
        let ea = new_entry(id_a, "op", "s", sa, 0, serde_json::json!({}), "a").unwrap();
        let eb = new_entry(id_b, "op", "s", sb, 0, serde_json::json!({}), "b").unwrap();

        let file_a = CorpusFile::new(vec![ea.clone()]).expect("valid corpus");
        let file_b = CorpusFile::new(vec![eb.clone()]).expect("valid corpus");
        let json_a = file_a.write_to_string().expect("ser a");
        let json_b = file_b.write_to_string().expect("ser b");

        let va: serde_json::Value = serde_json::from_str(&json_a).unwrap();
        let vb: serde_json::Value = serde_json::from_str(&json_b).unwrap();
        let entries_a = va["entries"].clone();
        let entries_b = vb["entries"].clone();
        let (first, second) = if id_a.to_hex() > id_b.to_hex() {
            (entries_a[0].clone(), entries_b[0].clone())
        } else {
            (entries_b[0].clone(), entries_a[0].clone())
        };
        let combined = serde_json::json!({
            "schema_version": {"major": 1, "minor": 2},
            "entries": [first, second]
        });
        let result = CorpusFile::load_from_str(&serde_json::to_string(&combined).unwrap());
        assert!(
            matches!(result, Err(CorpusError::UnorderedEntries { .. })),
            "out-of-order entries must be rejected"
        );
    }

    #[test]
    fn invalid_operation_token_is_rejected_by_new() {
        let seed = TestSeed::new(1);
        let id = CaseId::new(seed.for_case_stream("s"), 0);
        // Empty operation.
        assert!(matches!(
            new_entry(id, "", "s", seed, 0, serde_json::json!({}), "fail"),
            Err(CorpusError::InvalidToken {
                field: "operation",
                ..
            })
        ));
        // Space in operation (not a stable token).
        assert!(matches!(
            new_entry(id, "op bad", "s", seed, 0, serde_json::json!({}), "fail"),
            Err(CorpusError::InvalidToken {
                field: "operation",
                ..
            })
        ));
    }

    #[test]
    fn invalid_stream_name_token_is_rejected_by_new() {
        let seed = TestSeed::new(2);
        let id = CaseId::new(seed.for_case_stream("s"), 0);
        // Empty stream_name.
        assert!(matches!(
            new_entry(id, "op", "", seed, 0, serde_json::json!({}), "fail"),
            Err(CorpusError::InvalidToken {
                field: "stream_name",
                ..
            })
        ));
        // Control character in stream_name.
        assert!(matches!(
            new_entry(
                id,
                "op",
                "bad\x01name",
                seed,
                0,
                serde_json::json!({}),
                "fail"
            ),
            Err(CorpusError::InvalidToken {
                field: "stream_name",
                ..
            })
        ));
    }

    #[test]
    fn case_id_mismatch_is_rejected_by_new() {
        let seed = TestSeed::new(3);
        // Use primary seed instead of for_case_stream — should mismatch.
        let wrong_id = CaseId::new(seed, 0);
        assert!(matches!(
            new_entry(wrong_id, "op", "s", seed, 0, serde_json::json!({}), "fail"),
            Err(CorpusError::CaseIdMismatch { .. })
        ));
    }

    #[test]
    fn v1_1_entry_missing_stream_name_is_rejected_on_load() {
        // Build a v1.1 entry without stream_name — should fail validation in TryFrom.
        let valid_id = "00000000000000000000000000000000";
        let json = serde_json::json!({
            "schema_version": {"major": 1, "minor": 1},
            "entries": [{
                "schema_version": {"major": 1, "minor": 1},
                "id": valid_id,
                "operation": "op",
                "seed": 1,
                "case_index": 0,
                "inputs_json": {},
                "failure_message": "fail"
                // No stream_name
            }]
        });
        let result = CorpusFile::load_from_str(&serde_json::to_string(&json).unwrap());
        assert!(
            result.is_err(),
            "v1.1 entry without stream_name must be rejected"
        );
    }

    #[test]
    fn v1_0_entry_without_stream_name_loads_as_legacy_document() {
        // v1.0 entry with no stream_name and a matching ID (derived from primary seed).
        // In v1.0 the CaseId formula was CaseId::new(seed, case_index).
        let seed_val: u64 = 1;
        let case_index: u64 = 0;
        // We need an ID that matches the v1.0 formula. Build a v1.0 repr and let TryFrom accept it.
        let raw_id = CaseId::new(TestSeed::new(seed_val), case_index);
        let id_hex = raw_id.to_hex();
        let json = serde_json::json!({
            "schema_version": {"major": 1, "minor": 0},
            "entries": [{
                "schema_version": {"major": 1, "minor": 0},
                "id": id_hex,
                "operation": "op",
                "seed": seed_val,
                "case_index": case_index,
                "inputs_json": "{}",
                "failure_message": "fail"
            }]
        });
        let result = LegacyCorpusDocument::load_from_str(&serde_json::to_string(&json).unwrap());
        assert!(
            result.is_ok(),
            "v1.0 entry without stream_name must load as legacy: {result:?}"
        );
        assert_eq!(result.unwrap().schema_version(), SchemaVersion::new(1, 0));
    }

    #[test]
    fn inputs_json_as_value_round_trips() {
        let seed = TestSeed::new(5);
        let id = CaseId::new(seed.for_case_stream("rt"), 0);
        let inputs = serde_json::json!({"x": 1.0, "y": -3.5, "flag": true});
        let entry = new_entry(id, "op", "rt", seed, 0, inputs.clone(), "fail").unwrap();
        let file = CorpusFile::new(vec![entry]).expect("valid corpus");
        let json = file.write_to_string().expect("serialise");
        let loaded = CorpusFile::load_from_str(&json).expect("load");
        assert_eq!(loaded.entries()[0].inputs_json(), &inputs);
    }

    #[test]
    fn v1_0_string_inputs_json_is_parsed_on_migration() {
        let valid_id = "00000000000000000000000000000000";
        let file_repr = serde_json::json!({
            "schema_version": {"major": 1, "minor": 0},
            "entries": [{
                "schema_version": {"major": 1, "minor": 0},
                "id": valid_id,
                "operation": "op",
                "seed": 1,
                "case_index": 0,
                "inputs_json": r#"{"width":1.0}"#,
                "failure_message": "fail"
            }]
        });
        let json = serde_json::to_string(&file_repr).unwrap();
        let loaded = LegacyCorpusDocument::load_from_str(&json)
            .expect("v1.0 legacy load")
            .migrate_to_current(|_, _, _| {
                Ok((
                    "op".to_string(),
                    "legacy.stream".to_string(),
                    CheckKind::Invariant,
                    "check".to_string(),
                ))
            })
            .expect("migrate");
        let inputs = loaded.entries()[0].inputs_json();
        assert!(
            inputs.is_object(),
            "inputs_json must be parsed from v1.0 string format into an object"
        );
        assert_eq!(inputs["width"], serde_json::json!(1.0));
    }

    #[test]
    fn v1_1_string_inputs_json_is_valid_inline_value() {
        let seed = TestSeed::new(1);
        let id = CaseId::new(seed.for_case_stream("s"), 0);
        let file_repr = serde_json::json!({
            "schema_version": {"major": 1, "minor": 1},
            "entries": [{
                "schema_version": {"major": 1, "minor": 1},
                "id": id.to_hex(),
                "operation": "op",
                "stream_name": "s",
                "seed": 1,
                "case_index": 0,
                "inputs_json": r#"{"width":1.0}"#,
                "failure_message": "fail",
                "case_sequence_version": CASE_SEQUENCE_VERSION
            }]
        });
        let loaded =
            LegacyCorpusDocument::load_from_str(&serde_json::to_string(&file_repr).unwrap())
                .expect("v1.1 legacy load")
                .migrate_to_current(|_, _, _| {
                    Ok((
                        "op".to_string(),
                        "s".to_string(),
                        CheckKind::Invariant,
                        "check".to_string(),
                    ))
                })
                .expect("v1.1 inline string input must migrate");
        assert_eq!(
            loaded.entries()[0].inputs_json(),
            &serde_json::json!(r#"{"width":1.0}"#)
        );
    }

    #[test]
    fn v1_2_string_input_corpus_roundtrip() {
        let seed = TestSeed::new(7);
        let entry = new_entry(
            CaseId::new(seed.for_case_stream("strings"), 0),
            "op",
            "strings",
            seed,
            0,
            serde_json::json!("hello world"),
            "fail",
        )
        .unwrap();
        let json = CorpusFile::new(vec![entry.clone()])
            .expect("valid corpus")
            .write_to_string()
            .expect("serialise");
        let loaded = CorpusFile::load_from_str(&json).expect("round-trip");
        assert_eq!(loaded.entries()[0].inputs_json(), entry.inputs_json());
    }

    #[test]
    fn v1_0_non_string_inputs_json_is_rejected() {
        // v1.0 MUST double-encode inputs_json as a string; an inline object
        // must be rejected rather than silently accepted.
        let valid_id = "00000000000000000000000000000000";
        let file_repr = serde_json::json!({
            "schema_version": {"major": 1, "minor": 0},
            "entries": [{
                "schema_version": {"major": 1, "minor": 0},
                "id": valid_id,
                "operation": "op",
                "seed": 1,
                "case_index": 0,
                "inputs_json": {"width": 1.0},
                "failure_message": "fail"
            }]
        });
        let result = CorpusFile::load_from_str(&serde_json::to_string(&file_repr).unwrap());
        assert!(
            result.is_err(),
            "v1.0 inputs_json stored as an inline object must be rejected"
        );
    }

    #[test]
    fn v1_1_missing_case_sequence_version_is_rejected() {
        let seed = TestSeed::new(1);
        let id = CaseId::new(seed.for_case_stream("s"), 0);
        let file_repr = serde_json::json!({
            "schema_version": {"major": 1, "minor": 1},
            "entries": [{
                "schema_version": {"major": 1, "minor": 1},
                "id": id.to_hex(),
                "operation": "op",
                "stream_name": "s",
                "seed": 1,
                "case_index": 0,
                "inputs_json": {},
                "failure_message": "fail"
                // No case_sequence_version.
            }]
        });
        let result = CorpusFile::load_from_str(&serde_json::to_string(&file_repr).unwrap());
        assert!(
            result.is_err(),
            "v1.1 entry missing case_sequence_version must be rejected"
        );
    }

    #[test]
    fn v1_1_mismatched_case_sequence_version_is_rejected() {
        let seed = TestSeed::new(1);
        let id = CaseId::new(seed.for_case_stream("s"), 0);
        let file_repr = serde_json::json!({
            "schema_version": {"major": 1, "minor": 1},
            "entries": [{
                "schema_version": {"major": 1, "minor": 1},
                "id": id.to_hex(),
                "operation": "op",
                "stream_name": "s",
                "seed": 1,
                "case_index": 0,
                "inputs_json": {},
                "failure_message": "fail",
                "case_sequence_version": 255
            }]
        });
        let result = CorpusFile::load_from_str(&serde_json::to_string(&file_repr).unwrap());
        assert!(
            result.is_err(),
            "v1.1 entry with mismatched case_sequence_version must be rejected"
        );
    }

    #[test]
    fn rewriting_v1_0_entry_to_v1_2_produces_inline_value() {
        // Load a v1.0 entry with string-encoded inputs_json.
        let valid_id = "00000000000000000000000000000000";
        let file_repr = serde_json::json!({
            "schema_version": {"major": 1, "minor": 0},
            "entries": [{
                "schema_version": {"major": 1, "minor": 0},
                "id": valid_id,
                "operation": "op",
                "seed": 1,
                "case_index": 0,
                "inputs_json": r#"{"width":1.0}"#,
                "failure_message": "fail"
            }]
        });
        let file = LegacyCorpusDocument::load_from_str(&serde_json::to_string(&file_repr).unwrap())
            .expect("v1.0 load")
            .migrate_to_current(|_, _, _| {
                Ok((
                    "op".to_string(),
                    "rewritten.stream".to_string(),
                    CheckKind::Invariant,
                    "check".to_string(),
                ))
            })
            .expect("migrate");
        let json = file.write_to_string().expect("serialise");
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert!(
            value["entries"][0]["inputs_json"].is_object(),
            "rewritten current entry must store inputs_json inline, not as a string"
        );
    }

    #[test]
    fn write_as_legacy_double_encodes_inputs_json_as_string() {
        let entry = make_entry();
        let legacy_json = entry.write_as_legacy().expect("serialise legacy");
        let value: serde_json::Value = serde_json::from_str(&legacy_json).unwrap();
        assert_eq!(value["schema_version"]["major"], serde_json::json!(1));
        assert_eq!(value["schema_version"]["minor"], serde_json::json!(0));
        assert!(
            value["inputs_json"].is_string(),
            "legacy format must double-encode inputs_json as a string"
        );
        assert!(
            value.get("stream_name").is_none(),
            "legacy format omits stream_name"
        );
        assert!(
            value.get("case_sequence_version").is_none(),
            "legacy format omits case_sequence_version"
        );
    }

    #[test]
    fn write_as_legacy_round_trips_through_legacy_document_load() {
        let entry = make_entry();
        let legacy_json = entry.write_as_legacy().expect("serialise legacy");
        let legacy_value: serde_json::Value = serde_json::from_str(&legacy_json).unwrap();
        let file_json = serde_json::json!({
            "schema_version": {"major": 1, "minor": 0},
            "entries": [legacy_value]
        });
        let legacy =
            LegacyCorpusDocument::load_from_str(&serde_json::to_string(&file_json).unwrap())
                .expect("legacy entry must load");
        let migrated = legacy
            .migrate_to_current(|_, _, _| {
                Ok((
                    entry.operation().to_string(),
                    "corpus.test".to_string(),
                    CheckKind::Invariant,
                    "check".to_string(),
                ))
            })
            .expect("migrate");
        assert_eq!(migrated.entries()[0].inputs_json(), entry.inputs_json());
        assert_eq!(legacy.schema_version(), SchemaVersion::new(1, 0));
    }

    #[test]
    fn empty_corpus_file_serialises_and_loads() {
        let file = CorpusFile::new(vec![]).expect("valid corpus");
        let json = file.write_to_string().expect("serialise");
        let loaded = CorpusFile::load_from_str(&json).expect("empty corpus must load");
        assert!(loaded.entries().is_empty());
    }

    #[test]
    fn corpus_schema_version_is_stable() {
        assert_eq!(CORPUS_SCHEMA_VERSION.major(), 1);
        assert_eq!(CORPUS_SCHEMA_VERSION.minor(), 2);
    }

    #[test]
    fn write_to_string_is_deterministic() {
        let file = CorpusFile::new(vec![make_entry()]).expect("valid corpus");
        assert_eq!(
            file.write_to_string().expect("ser a"),
            file.write_to_string().expect("ser b"),
            "serialisation must be deterministic"
        );
    }

    #[test]
    fn corpus_entry_getters_are_consistent() {
        let entry = make_entry();
        assert_eq!(entry.schema_version(), SchemaVersion::new(1, 2));
        assert_eq!(entry.seed(), TestSeed::new(42));
        assert_eq!(entry.case_index(), 7);
        assert_eq!(entry.operation(), "primitive.cuboid");
        assert_eq!(entry.stream_name(), Some("corpus.test"));
        assert!(entry.inputs_json()["width"].is_number());
        assert!(entry.failure_message().contains("orientation"));
    }

    #[test]
    fn push_maintains_canonical_order() {
        let sa = TestSeed::new(0x10);
        let sb = TestSeed::new(0x20);
        let sc = TestSeed::new(0x30);
        let ea = new_entry(
            CaseId::new(sa.for_case_stream("s"), 0),
            "op",
            "s",
            sa,
            0,
            serde_json::json!({}),
            "a",
        )
        .unwrap();
        let ec = new_entry(
            CaseId::new(sc.for_case_stream("s"), 0),
            "op",
            "s",
            sc,
            0,
            serde_json::json!({}),
            "c",
        )
        .unwrap();
        let eb = new_entry(
            CaseId::new(sb.for_case_stream("s"), 0),
            "op",
            "s",
            sb,
            0,
            serde_json::json!({}),
            "b",
        )
        .unwrap();

        let mut file = CorpusFile::new(vec![ea, ec]).expect("valid corpus");
        file.push(eb).expect("valid push");

        let hexes: Vec<String> = file.entries().iter().map(|e| e.id().to_hex()).collect();
        let mut sorted = hexes.clone();
        sorted.sort();
        assert_eq!(hexes, sorted, "push must maintain canonical order");
    }

    #[test]
    fn corpus_file_push_rejects_duplicate_id() {
        let entry = make_entry();
        let mut file = CorpusFile::new(vec![entry.clone()]).expect("valid corpus");
        let result = file.push(entry);
        assert!(
            matches!(result, Err(CorpusError::DuplicateCaseId { .. })),
            "push must reject duplicate CaseId"
        );
    }

    #[test]
    fn multi_entry_corpus_round_trips_in_canonical_order() {
        let seeds: &[u64] = &[0xAA, 0x55, 0xFF, 0x01, 0x10];
        let entries: Vec<CorpusEntry> = seeds
            .iter()
            .map(|&v| {
                let seed = TestSeed::new(v);
                let id = CaseId::new(seed.for_case_stream("multi"), 0);
                new_entry(id, "op", "multi", seed, 0, serde_json::json!({}), "fail").unwrap()
            })
            .collect();

        let file = CorpusFile::new(entries).expect("valid corpus");
        let json = file.write_to_string().expect("serialise");
        let loaded = CorpusFile::load_from_str(&json).expect("round-trip");

        // Verify canonical order.
        let hexes: Vec<String> = loaded.entries().iter().map(|e| e.id().to_hex()).collect();
        let mut sorted_hexes = hexes.clone();
        sorted_hexes.sort();
        assert_eq!(
            hexes, sorted_hexes,
            "loaded entries must be in canonical order"
        );
    }

    #[test]
    fn legacy_document_write_is_byte_identical_to_load() {
        let entry = make_entry();
        let legacy_entry = entry.write_as_legacy().expect("legacy entry");
        let fixture = format!(
            "{{\n  \"schema_version\": {{ \"major\": 1, \"minor\": 0 }},\n  \"entries\": [\n{}\n  ]\n}}",
            legacy_entry
                .lines()
                .map(|line| format!("    {line}"))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let document = LegacyCorpusDocument::load_from_str(&fixture).expect("legacy load");
        assert_eq!(
            document.as_str(),
            fixture,
            "legacy bytes must round-trip exactly"
        );
    }

    #[test]
    fn v1_1_load_write_load_round_trip_is_lossless() {
        let entry = make_entry();
        let file = CorpusFile::new(vec![entry]).expect("valid corpus");
        let json1 = file.write_to_string().expect("first write");
        let loaded = CorpusFile::load_from_str(&json1).expect("load");
        let json2 = loaded.write_to_string().expect("second write");
        assert_eq!(
            json1, json2,
            "v1.1 load→write→load must produce byte-identical output"
        );
    }
    #[test]
    fn corpus_entry_records_check_metadata() {
        let entry = make_entry();
        assert_eq!(entry.check_kind(), Some(CheckKind::Invariant));
        assert_eq!(entry.check_name(), Some("check"));
    }

    #[test]
    fn v1_2_entry_missing_check_kind_is_rejected() {
        let seed = TestSeed::new(1);
        let id = CaseId::new(seed.for_case_stream("s"), 0);
        let json = serde_json::json!({
            "schema_version": {"major": 1, "minor": 2},
            "entries": [{
                "schema_version": {"major": 1, "minor": 2},
                "id": id.to_hex(),
                "operation": "op",
                "stream_name": "s",
                "seed": 1,
                "case_index": 0,
                "inputs_json": {},
                "failure_message": "fail",
                "case_sequence_version": CASE_SEQUENCE_VERSION,
                "check_name": "check"
            }]
        });
        assert!(matches!(
            CorpusFile::load_from_str(&json.to_string()),
            Err(CorpusError::MissingCheckKind { .. })
        ));
    }

    #[test]
    fn v1_2_entry_missing_check_name_is_rejected() {
        let seed = TestSeed::new(1);
        let id = CaseId::new(seed.for_case_stream("s"), 0);
        let json = serde_json::json!({
            "schema_version": {"major": 1, "minor": 2},
            "entries": [{
                "schema_version": {"major": 1, "minor": 2},
                "id": id.to_hex(),
                "operation": "op",
                "stream_name": "s",
                "seed": 1,
                "case_index": 0,
                "inputs_json": {},
                "failure_message": "fail",
                "case_sequence_version": CASE_SEQUENCE_VERSION,
                "check_kind": "invariant"
            }]
        });
        assert!(matches!(
            CorpusFile::load_from_str(&json.to_string()),
            Err(CorpusError::MissingCheckName { .. })
        ));
    }

    #[test]
    fn v1_2_invalid_check_kind_is_rejected() {
        let seed = TestSeed::new(1);
        let id = CaseId::new(seed.for_case_stream("s"), 0);
        let json = serde_json::json!({
            "schema_version": {"major": 1, "minor": 2},
            "entries": [{
                "schema_version": {"major": 1, "minor": 2},
                "id": id.to_hex(),
                "operation": "op",
                "stream_name": "s",
                "seed": 1,
                "case_index": 0,
                "inputs_json": {},
                "failure_message": "fail",
                "case_sequence_version": CASE_SEQUENCE_VERSION,
                "check_kind": "bad_kind",
                "check_name": "check"
            }]
        });
        assert!(matches!(
            CorpusFile::load_from_str(&json.to_string()),
            Err(CorpusError::InvalidCheckKind { .. })
        ));
    }

    #[test]
    fn v1_1_entries_load_as_legacy_documents() {
        let seed = TestSeed::new(1);
        let id = CaseId::new(seed.for_case_stream("legacy"), 0);
        let json = serde_json::json!({
            "schema_version": {"major": 1, "minor": 1},
            "entries": [{
                "schema_version": {"major": 1, "minor": 1},
                "id": id.to_hex(),
                "operation": "op",
                "stream_name": "legacy",
                "seed": 1,
                "case_index": 0,
                "inputs_json": {},
                "failure_message": "fail",
                "case_sequence_version": CASE_SEQUENCE_VERSION
            }]
        });
        let loaded = LegacyCorpusDocument::load_from_str(&json.to_string()).unwrap();
        let migrated = loaded
            .migrate_to_current(|_, _, _| {
                Ok((
                    "op".to_string(),
                    "legacy".to_string(),
                    CheckKind::Property,
                    "migrated.check".to_string(),
                ))
            })
            .expect("migrate");
        assert_eq!(
            migrated.entries()[0].check_kind(),
            Some(CheckKind::Property)
        );
        assert_eq!(migrated.entries()[0].check_name(), Some("migrated.check"));
    }

    #[test]
    fn v1_1_migration_hostile_operation_replacement_rejected() {
        let seed = TestSeed::new(1);
        let id = CaseId::new(seed.for_case_stream("legacy"), 0);
        let json = serde_json::json!({
            "schema_version": {"major": 1, "minor": 1},
            "entries": [{
                "schema_version": {"major": 1, "minor": 1},
                "id": id.to_hex(),
                "operation": "op",
                "stream_name": "legacy",
                "seed": 1,
                "case_index": 0,
                "inputs_json": {},
                "failure_message": "fail",
                "case_sequence_version": CASE_SEQUENCE_VERSION
            }]
        });
        let result = LegacyCorpusDocument::load_from_str(&json.to_string())
            .unwrap()
            .migrate_to_current(|_, _, _| {
                Ok((
                    "different.op".to_string(),
                    "legacy".to_string(),
                    CheckKind::Invariant,
                    "check".to_string(),
                ))
            });
        assert!(matches!(
            result,
            Err(CorpusError::MigrationIdentityMismatch {
                field: "operation",
                expected,
                found,
                ..
            }) if expected == "op" && found == "different.op"
        ));
    }

    #[test]
    fn v1_1_migration_hostile_stream_replacement_rejected() {
        let seed = TestSeed::new(1);
        let id = CaseId::new(seed.for_case_stream("legacy"), 0);
        let json = serde_json::json!({
            "schema_version": {"major": 1, "minor": 1},
            "entries": [{
                "schema_version": {"major": 1, "minor": 1},
                "id": id.to_hex(),
                "operation": "op",
                "stream_name": "legacy",
                "seed": 1,
                "case_index": 0,
                "inputs_json": {},
                "failure_message": "fail",
                "case_sequence_version": CASE_SEQUENCE_VERSION
            }]
        });
        let result = LegacyCorpusDocument::load_from_str(&json.to_string())
            .unwrap()
            .migrate_to_current(|_, _, _| {
                Ok((
                    "op".to_string(),
                    "other.stream".to_string(),
                    CheckKind::Invariant,
                    "check".to_string(),
                ))
            });
        assert!(matches!(
            result,
            Err(CorpusError::MigrationIdentityMismatch {
                field: "stream_name",
                expected,
                found,
                ..
            }) if expected == "legacy" && found == "other.stream"
        ));
    }

    #[test]
    fn v1_1_migration_with_matching_identity_succeeds() {
        let seed = TestSeed::new(1);
        let id = CaseId::new(seed.for_case_stream("legacy"), 0);
        let json = serde_json::json!({
            "schema_version": {"major": 1, "minor": 1},
            "entries": [{
                "schema_version": {"major": 1, "minor": 1},
                "id": id.to_hex(),
                "operation": "op",
                "stream_name": "legacy",
                "seed": 1,
                "case_index": 0,
                "inputs_json": {"kind": "inline"},
                "failure_message": "fail",
                "case_sequence_version": CASE_SEQUENCE_VERSION
            }]
        });
        let migrated = LegacyCorpusDocument::load_from_str(&json.to_string())
            .unwrap()
            .migrate_to_current(|_, _, _| {
                Ok((
                    "op".to_string(),
                    "legacy".to_string(),
                    CheckKind::Property,
                    "migrated.check".to_string(),
                ))
            })
            .expect("matching identity must migrate");
        let entry = &migrated.entries()[0];
        assert_eq!(entry.operation(), "op");
        assert_eq!(entry.stream_name(), Some("legacy"));
        assert_eq!(entry.check_kind(), Some(CheckKind::Property));
        assert_eq!(entry.check_name(), Some("migrated.check"));
    }

    #[test]
    fn v1_2_serialization_includes_check_metadata() {
        let entry = make_entry();
        let json = CorpusFile::new(vec![entry])
            .expect("valid corpus")
            .write_to_string()
            .unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        assert_eq!(value["schema_version"]["minor"], serde_json::json!(2));
        assert_eq!(
            value["entries"][0]["check_kind"],
            serde_json::json!("invariant")
        );
        assert_eq!(
            value["entries"][0]["check_name"],
            serde_json::json!("check")
        );
    }
}
