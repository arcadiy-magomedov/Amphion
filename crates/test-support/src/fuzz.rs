//! Fuzz target conventions, structured input parsing, and replay environment
//! parsing.
//!
//! # Milestone
//!
//! [`RANDOMIZED_CASE_MILESTONE`] is set to 10,000 as required by the first
//! kernel proof milestone. Test suites should assert they reach this count
//! before the milestone gate closes.
//!
//! # Corpus directory conventions
//!
//! Fuzz corpora live under `qa/fuzz/<target-name>/corpus/`. Interesting
//! minimised seeds are stored in `qa/fuzz/<target-name>/interesting/`.
//! See `qa/fuzz/README.md` for the full layout description.
//!
//! # Replay environment variables
//!
//! Replay commands emit seven environment variables, and replay parsing treats
//! them as an all-or-nothing identity tuple:
//!
//! - `AMPHION_TEST_VERSION` — case-sequence version (u8 decimal).
//! - `AMPHION_TEST_SEED` — primary seed (u64 decimal).
//! - `AMPHION_TEST_CASE` — zero-based case index (u64 decimal).
//! - `AMPHION_TEST_STREAM` — stream name used when the failure was recorded.
//! - `AMPHION_TEST_OPERATION` — stable operation token.
//! - `AMPHION_TEST_CHECK_KIND` — [`CheckKind`] discriminator.
//! - `AMPHION_TEST_CHECK` — stable invariant/relation token.
//!
//! When none of the seven variables are set, [`parse_replay_env`] returns
//! `Ok(None)`. When any one of them is set, all seven must be present.
//!
//! # Structured fuzz input
//!
//! [`FuzzInputReader`] provides typed, panic-free reads from a raw `&[u8]`.
//! Scalar reads (u8, u16, u32, u64, bool, f64) return a type-appropriate
//! zero/false default when bytes are exhausted.  `read_bytes` returns whatever
//! bytes remain (a partial read), or an empty slice when fully exhausted.

use core::{error::Error, fmt};

use crate::{
    is_stable_token,
    rng::{CASE_SEQUENCE_VERSION, TestSeed},
};

/// Target number of deterministic randomised cases for the first kernel proof
/// milestone (Q1.1 of the execution plan).
pub const RANDOMIZED_CASE_MILESTONE: u32 = 10_000;

/// Environment variable name for the replay case-sequence version.
pub const ENV_TEST_VERSION: &str = "AMPHION_TEST_VERSION";

/// Environment variable name for the replay seed.
pub const ENV_TEST_SEED: &str = "AMPHION_TEST_SEED";

/// Environment variable name for the replay case index.
pub const ENV_TEST_CASE: &str = "AMPHION_TEST_CASE";

/// Environment variable name for the replay stream name.
pub const ENV_TEST_STREAM: &str = "AMPHION_TEST_STREAM";

/// Environment variable name for the optional replay check/relation name.
///
/// Unlike the legacy three-field replay flow, this variable now participates
/// in the required seven-field replay identity parsed by [`parse_replay_env`].
pub const ENV_TEST_CHECK: &str = "AMPHION_TEST_CHECK";

/// Environment variable for the optional replay operation label.
pub const ENV_TEST_OPERATION: &str = "AMPHION_TEST_OPERATION";

/// Environment variable for the optional replay check kind.
///
/// Valid values: `"invariant"`, `"property"`, `"metamorphic_relation"`.
pub const ENV_TEST_CHECK_KIND: &str = "AMPHION_TEST_CHECK_KIND";

/// Discriminates the kind of check/relation being replayed.
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckKind {
    /// An [`crate::runner::Invariant`] check.
    Invariant,
    /// A boolean property check (via [`crate::runner::run_property_cases`]).
    Property,
    /// A metamorphic relation (via [`crate::runner::MetamorphicCase`]).
    MetamorphicRelation,
}

impl fmt::Display for CheckKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invariant => f.write_str("invariant"),
            Self::Property => f.write_str("property"),
            Self::MetamorphicRelation => f.write_str("metamorphic_relation"),
        }
    }
}

impl core::str::FromStr for CheckKind {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "invariant" => Ok(Self::Invariant),
            "property" => Ok(Self::Property),
            "metamorphic_relation" => Ok(Self::MetamorphicRelation),
            other => Err(format!(
                "unknown check kind: {other:?}; expected one of: invariant, property, metamorphic_relation"
            )),
        }
    }
}

// ── ReplayConfig ───────────────────────────────────────────────────────────

/// The replay identity values needed to
/// deterministically replay a specific failing case.
///
/// Obtain via [`parse_replay_env`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReplayConfig {
    /// The case-sequence version associated with the replay command.
    pub case_sequence_version: u8,
    /// Operation label from `AMPHION_TEST_OPERATION`.
    pub operation: String,
    /// Primary seed for the run.
    pub seed: TestSeed,
    /// Zero-based case index to advance to.
    pub case_index: u64,
    /// Stream name that was used when the failure occurred.
    pub stream_name: String,
    /// Check kind from `AMPHION_TEST_CHECK_KIND`.
    pub check_kind: CheckKind,
    /// Check/relation name from `AMPHION_TEST_CHECK`.
    pub check_name: String,
}

// ── ReplayConfigError ──────────────────────────────────────────────────────

/// Error produced when the replay environment variables are missing or invalid.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReplayConfigError {
    /// A specific replay identity field was missing while parsing.
    MissingField(&'static str),
    /// Only some of the seven required variables are set.
    PartialReplayEnv {
        /// Which variables are set.
        set: Vec<&'static str>,
        /// Which variables are missing.
        missing: Vec<&'static str>,
    },
    /// `AMPHION_TEST_SEED` is present but not a valid u64.
    MalformedSeed(String),
    /// `AMPHION_TEST_CASE` is present but not a valid u64.
    MalformedCaseIndex(String),
    /// `AMPHION_TEST_VERSION` is present but not a valid u8.
    MalformedVersion(String),
    /// `AMPHION_TEST_STREAM` is not a stable token.
    InvalidStreamName(String),
    /// `AMPHION_TEST_OPERATION` is not a stable token.
    InvalidOperation(String),
    /// `AMPHION_TEST_CHECK` is not a stable token.
    InvalidCheckName(String),
    /// `AMPHION_TEST_CHECK_KIND` is invalid.
    InvalidCheckKind(String),
    /// `AMPHION_TEST_VERSION` disagrees with the current case-sequence version.
    VersionMismatch {
        /// The version found in the environment.
        found: u8,
    },
}

impl fmt::Display for ReplayConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingField(field) => {
                write!(f, "missing required replay environment field {field}")
            }
            Self::PartialReplayEnv { set, missing } => write!(
                f,
                "partial replay environment: set=[{}] missing=[{}]; \
                 all seven replay identity variables must be provided together",
                set.join(", "),
                missing.join(", "),
            ),
            Self::MalformedSeed(v) => {
                write!(f, "{ENV_TEST_SEED}={v:?} is not a valid u64 decimal seed")
            }
            Self::MalformedCaseIndex(v) => write!(
                f,
                "{ENV_TEST_CASE}={v:?} is not a valid u64 decimal case index"
            ),
            Self::MalformedVersion(v) => write!(
                f,
                "{ENV_TEST_VERSION}={v:?} is not a valid u8 decimal version"
            ),
            Self::InvalidStreamName(v) => {
                write!(f, "{ENV_TEST_STREAM}={v:?} is not a valid stable token")
            }
            Self::InvalidOperation(v) => {
                write!(f, "{ENV_TEST_OPERATION}={v:?} is not a valid stable token")
            }
            Self::InvalidCheckName(v) => {
                write!(f, "{ENV_TEST_CHECK}={v:?} is not a valid stable token")
            }
            Self::InvalidCheckKind(v) => {
                write!(f, "{ENV_TEST_CHECK_KIND}={v:?} is not a valid check kind")
            }
            Self::VersionMismatch { found } => write!(
                f,
                "{ENV_TEST_VERSION}={found} does not match current CASE_SEQUENCE_VERSION {CASE_SEQUENCE_VERSION}"
            ),
        }
    }
}

impl Error for ReplayConfigError {}

/// Parses the replay environment variables into a [`ReplayConfig`].
///
/// Returns `Ok(None)` when none of the seven variables are set (normal
/// non-replay run). Returns `Ok(Some(_))` when all seven are set and valid.
/// Returns `Err(_)` when any variable is set but malformed, or when only a
/// subset of the seven variables are set.
///
/// # Errors
///
/// See [`ReplayConfigError`].
pub fn parse_replay_env() -> Result<Option<ReplayConfig>, ReplayConfigError> {
    let mut vars = Vec::new();
    for name in [
        ENV_TEST_VERSION,
        ENV_TEST_SEED,
        ENV_TEST_CASE,
        ENV_TEST_STREAM,
        ENV_TEST_OPERATION,
        ENV_TEST_CHECK_KIND,
        ENV_TEST_CHECK,
    ] {
        vars.push((name, std::env::var(name).ok()));
    }
    let set: Vec<&'static str> = vars
        .iter()
        .filter_map(|(name, value)| value.as_ref().map(|_| *name))
        .collect();
    if set.is_empty() {
        return Ok(None);
    }
    if set.len() != vars.len() {
        let missing = vars
            .iter()
            .filter_map(|(name, value)| value.is_none().then_some(*name))
            .collect();
        return Err(ReplayConfigError::PartialReplayEnv { set, missing });
    }

    let get = |name| {
        vars.iter()
            .find(|(field, _)| *field == name)
            .and_then(|(_, value)| value.clone())
            .ok_or(ReplayConfigError::MissingField(name))
    };

    let raw_version = get(ENV_TEST_VERSION)?;
    let case_sequence_version = {
        let found = raw_version
            .parse::<u8>()
            .map_err(|_| ReplayConfigError::MalformedVersion(raw_version.clone()))?;
        if found != CASE_SEQUENCE_VERSION {
            return Err(ReplayConfigError::VersionMismatch { found });
        }
        found
    };

    let seed_str = get(ENV_TEST_SEED)?;
    let case_str = get(ENV_TEST_CASE)?;
    let stream_name = get(ENV_TEST_STREAM)?;
    let operation = get(ENV_TEST_OPERATION)?;
    let raw_check_kind = get(ENV_TEST_CHECK_KIND)?;
    let check_name = get(ENV_TEST_CHECK)?;

    let seed_raw = seed_str
        .parse::<u64>()
        .map_err(|_| ReplayConfigError::MalformedSeed(seed_str))?;
    let case_index = case_str
        .parse::<u64>()
        .map_err(|_| ReplayConfigError::MalformedCaseIndex(case_str))?;

    if !is_stable_token(&stream_name) {
        return Err(ReplayConfigError::InvalidStreamName(stream_name));
    }

    if !is_stable_token(&operation) {
        return Err(ReplayConfigError::InvalidOperation(operation));
    }
    if !is_stable_token(&check_name) {
        return Err(ReplayConfigError::InvalidCheckName(check_name));
    }
    let check_kind = raw_check_kind
        .parse::<CheckKind>()
        .map_err(|_| ReplayConfigError::InvalidCheckKind(raw_check_kind))?;

    Ok(Some(ReplayConfig {
        case_sequence_version,
        operation,
        seed: TestSeed::new(seed_raw),
        case_index,
        stream_name,
        check_kind,
        check_name,
    }))
}

// ── FuzzInputReader ────────────────────────────────────────────────────────

/// A deterministic, panic-free reader that extracts typed values from a raw
/// fuzz-provided byte slice.
///
/// # Read behaviours
///
/// **Scalar reads** (`read_u8`, `read_u16_le`, `read_u32_le`, `read_u64_le`,
/// `read_bool`, `read_f64_le`): each constituent byte that is beyond the end
/// of the slice returns `0`.  This means a short scalar read is zero-padded
/// rather than truncated.
///
/// **`read_bytes(n)`**: returns whatever bytes remain up to `n` (partial
/// read), or an empty slice when fully exhausted.  The cursor always advances
/// by `min(n, remaining)`.  Overflow of `pos + n` is handled with saturating
/// arithmetic — passing `usize::MAX` is safe.
///
/// The reader never clones or copies the underlying slice.
///
/// # Example
///
/// ```rust
/// # use amphion_test_support::FuzzInputReader;
/// // flag=1, pad=0, then 8 bytes encoding 1.0_f64 in little-endian
/// let data = [0x01u8, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xf0, 0x3f];
/// let mut reader = FuzzInputReader::new(&data);
/// let flag = reader.read_bool();     // 0x01 → true
/// let _pad = reader.read_u8();       // 0x00
/// let value = reader.read_f64_le();  // 8 bytes → 1.0
/// assert!(flag);
/// assert_eq!(value, 1.0);
/// ```
pub struct FuzzInputReader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> FuzzInputReader<'a> {
    /// Creates a reader over a raw fuzz input byte slice.
    #[must_use]
    pub const fn new(data: &'a [u8]) -> Self {
        Self { data, pos: 0 }
    }

    /// Returns the number of bytes not yet consumed.
    #[must_use]
    pub fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.pos)
    }

    /// Returns `true` when all bytes have been consumed.
    #[must_use]
    pub fn is_exhausted(&self) -> bool {
        self.pos >= self.data.len()
    }

    /// Reads one byte, returning `0` when exhausted.
    pub fn read_u8(&mut self) -> u8 {
        if self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;
            b
        } else {
            0
        }
    }

    /// Reads a little-endian `u16`.  Any missing bytes are treated as `0`.
    pub fn read_u16_le(&mut self) -> u16 {
        let lo = u16::from(self.read_u8());
        let hi = u16::from(self.read_u8());
        lo | (hi << 8)
    }

    /// Reads a little-endian `u32`.  Any missing bytes are treated as `0`.
    pub fn read_u32_le(&mut self) -> u32 {
        let lo = u32::from(self.read_u16_le());
        let hi = u32::from(self.read_u16_le());
        lo | (hi << 16)
    }

    /// Reads a little-endian `u64`.  Any missing bytes are treated as `0`.
    pub fn read_u64_le(&mut self) -> u64 {
        let lo = u64::from(self.read_u32_le());
        let hi = u64::from(self.read_u32_le());
        lo | (hi << 32)
    }

    /// Reads one byte and returns `true` when its lowest bit is set.
    pub fn read_bool(&mut self) -> bool {
        (self.read_u8() & 1) != 0
    }

    /// Reads eight bytes as a little-endian `f64`.
    ///
    /// The result may be NaN, infinite, or any other IEEE 754 value — callers
    /// that pass this to kernel APIs should expect rejection of non-finite
    /// inputs, which is part of the invariant being tested.
    pub fn read_f64_le(&mut self) -> f64 {
        f64::from_bits(self.read_u64_le())
    }

    /// Reads up to `n` bytes and returns the available sub-slice.
    ///
    /// Returns whatever bytes remain when fewer than `n` bytes are available,
    /// or an empty slice when the reader is exhausted.  Overflow of `pos + n`
    /// is handled with saturating arithmetic; `n = usize::MAX` is safe.
    pub fn read_bytes(&mut self, n: usize) -> &'a [u8] {
        let start = self.pos;
        let end = self.pos.saturating_add(n).min(self.data.len());
        self.pos = end;
        &self.data[start..end]
    }

    /// Peeks at the next byte without advancing the cursor. Returns `None`
    /// when exhausted.
    #[must_use]
    pub fn peek_u8(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{
        CheckKind, ENV_TEST_CASE, ENV_TEST_CHECK, ENV_TEST_CHECK_KIND, ENV_TEST_OPERATION,
        ENV_TEST_SEED, ENV_TEST_STREAM, ENV_TEST_VERSION, FuzzInputReader,
        RANDOMIZED_CASE_MILESTONE,
    };

    #[test]
    fn milestone_is_ten_thousand() {
        assert_eq!(RANDOMIZED_CASE_MILESTONE, 10_000);
    }

    #[test]
    fn env_var_names_are_stable() {
        assert_eq!(ENV_TEST_VERSION, "AMPHION_TEST_VERSION");
        assert_eq!(ENV_TEST_SEED, "AMPHION_TEST_SEED");
        assert_eq!(ENV_TEST_CASE, "AMPHION_TEST_CASE");
        assert_eq!(ENV_TEST_STREAM, "AMPHION_TEST_STREAM");
        assert_eq!(ENV_TEST_CHECK, "AMPHION_TEST_CHECK");
        assert_eq!(ENV_TEST_OPERATION, "AMPHION_TEST_OPERATION");
        assert_eq!(ENV_TEST_CHECK_KIND, "AMPHION_TEST_CHECK_KIND");
    }

    #[test]
    fn check_kind_round_trips_through_strings() {
        assert_eq!("invariant".parse::<CheckKind>(), Ok(CheckKind::Invariant));
        assert_eq!("property".parse::<CheckKind>(), Ok(CheckKind::Property));
        assert_eq!(
            "metamorphic_relation".parse::<CheckKind>(),
            Ok(CheckKind::MetamorphicRelation)
        );
        assert_eq!(CheckKind::Invariant.to_string(), "invariant");
        assert!("unknown".parse::<CheckKind>().is_err());
    }

    #[test]
    fn reader_returns_defaults_when_exhausted() {
        let mut r = FuzzInputReader::new(&[]);
        assert_eq!(r.read_u8(), 0);
        assert_eq!(r.read_u16_le(), 0);
        assert_eq!(r.read_u32_le(), 0);
        assert_eq!(r.read_u64_le(), 0);
        assert!(!r.read_bool(), "exhausted bool must be false");
        assert_eq!(
            r.read_f64_le().to_bits(),
            0u64,
            "exhausted f64 must have zero bits"
        );
        assert_eq!(
            r.read_bytes(5),
            &[] as &[u8],
            "exhausted bytes must be empty"
        );
    }

    #[test]
    fn reader_reads_typed_values_correctly() {
        let data: &[u8] = &[
            0x07, // u8
            0x01, 0x02, // u16 LE → 0x0201
            0x03, 0x04, 0x05, 0x06, // u32 LE → 0x06050403
        ];
        let mut r = FuzzInputReader::new(data);
        assert_eq!(r.read_u8(), 0x07);
        assert_eq!(r.read_u16_le(), 0x0201);
        assert_eq!(r.read_u32_le(), 0x0605_0403);
        assert!(r.is_exhausted());
    }

    #[test]
    fn reader_bool_checks_low_bit() {
        let mut r = FuzzInputReader::new(&[0x00, 0x01, 0x02, 0x03]);
        assert!(!r.read_bool()); // 0x00 → false
        assert!(r.read_bool()); // 0x01 → true
        assert!(!r.read_bool()); // 0x02 → false (bit 0 is 0)
        assert!(r.read_bool()); // 0x03 → true (bit 0 is 1)
    }

    #[test]
    fn reader_f64_decodes_one_correctly() {
        let one_bits: u64 = 1.0_f64.to_bits();
        let bytes = one_bits.to_le_bytes();
        let mut r = FuzzInputReader::new(&bytes);
        assert_eq!(r.read_f64_le().to_bits(), one_bits);
    }

    #[test]
    fn reader_remaining_tracks_position() {
        let data = [0u8; 10];
        let mut r = FuzzInputReader::new(&data);
        assert_eq!(r.remaining(), 10);
        r.read_u32_le();
        assert_eq!(r.remaining(), 6);
        r.read_u64_le();
        assert_eq!(r.remaining(), 0);
        assert!(r.is_exhausted());
    }

    #[test]
    fn reader_read_bytes_returns_correct_slice() {
        let data: &[u8] = &[1, 2, 3, 4, 5];
        let mut r = FuzzInputReader::new(data);
        assert_eq!(r.read_bytes(3), &[1u8, 2, 3]);
        assert_eq!(r.read_bytes(10), &[4u8, 5]); // partial read
        assert_eq!(r.read_bytes(1), &[] as &[u8]); // exhausted
    }

    #[test]
    fn reader_peek_does_not_advance_cursor() {
        let data = [42u8, 7];
        let mut r = FuzzInputReader::new(&data);
        assert_eq!(r.peek_u8(), Some(42));
        assert_eq!(r.peek_u8(), Some(42)); // still at position 0
        r.read_u8();
        assert_eq!(r.peek_u8(), Some(7));
        r.read_u8();
        assert_eq!(r.peek_u8(), None);
    }

    #[test]
    fn reader_partial_reads_at_boundary() {
        // Only 3 bytes, reading a u32 should use 3 real bytes + 1 default zero.
        let data = [0x01u8, 0x02, 0x03];
        let mut r = FuzzInputReader::new(&data);
        let v = r.read_u32_le();
        // LE: lo16 = (0x01 | 0x02<<8) = 0x0201, hi16 = (0x03 | 0x00<<8) = 0x0003
        assert_eq!(v, 0x0003_0201);
    }

    #[test]
    fn reader_read_bytes_with_usize_max_does_not_panic() {
        let data = [1u8, 2, 3];
        let mut r = FuzzInputReader::new(&data);
        // usize::MAX must not cause integer overflow.
        let bytes = r.read_bytes(usize::MAX);
        assert_eq!(bytes, &[1u8, 2, 3], "should return all remaining bytes");
        assert!(r.is_exhausted());
    }

    #[test]
    fn reader_read_bytes_usize_max_on_exhausted() {
        let mut r = FuzzInputReader::new(&[]);
        let bytes = r.read_bytes(usize::MAX);
        assert_eq!(bytes, &[] as &[u8]);
    }

    #[test]
    fn reader_read_bytes_exact_boundary() {
        let data = [10u8, 20, 30];
        let mut r = FuzzInputReader::new(&data);
        r.read_u8(); // consume 1
        let bytes = r.read_bytes(2); // exactly what remains
        assert_eq!(bytes, &[20u8, 30]);
        assert!(r.is_exhausted());
    }
}
