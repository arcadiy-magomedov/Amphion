//! Deterministic, reproducible seeded random number generation.
//!
//! [`TestRng`] implements xoshiro256\*\* seeded from a [`TestSeed`] expanded
//! through `SplitMix64`. Every sequence is fully determined by its seed alone;
//! no wall-clock, OS, thread-local, or other non-deterministic entropy source
//! is used anywhere in this module.
//!
//! Independent streams are derived from a single primary seed by mixing the
//! stream name via FNV-1a-64 and one `SplitMix64` step.  Two distinct stream
//! names almost always produce distinct seeds; a collision is theoretically
//! possible with probability ≈ 1/2⁶⁴ per pair (birthday-bound for FNV-1a-64).
//! The probability is negligible in practice but is not zero, so callers must
//! not rely on mathematical uniqueness guarantees.
//!
//! [`CaseId`] ties a case to its stream seed and sequential index so that any
//! failure can be reproduced by replaying the stream from the same seed and
//! advancing to the same index.  The `CaseId` does **not** store the stream
//! name; callers must record the stream name separately (e.g. in
//! [`CaseFailure::stream_name`](crate::runner::CaseFailure::stream_name) or
//! a [`crate::corpus::CorpusEntry`]) for unambiguous replay.

use core::{error::Error, fmt};

use serde::{Deserialize, Serialize};

// ── Public constants ───────────────────────────────────────────────────────

/// Algorithm version used to derive per-case RNGs.
///
/// | Ver | Algorithm |
/// |-----|-----------|
/// | 1 (pre-ef4e9f4) | Sequential stream advancement: case `K` required `K−1` prior RNG draws. Not O(1). |
/// | 2 (pre-fourth-commit) | Per-case derivation via `seed.for_stream("{stream}\x00{case_index}")`. O(1) per case. |
/// | 3 (current) | Versioned per-case derivation via `seed.for_stream("{CASE_SEQUENCE_VERSION}\x00{stream}\x00{case_index}")` (see [`TestSeed::for_case_stream`]). The leading version tag domain-separates this generation's keys from V2's, so a future version bump can never alias a V2 (or V1) key/CaseId even if the rest of the key happens to collide. |
///
/// V3 is the only version produced by new runs.  V1 and V2 never had
/// committed corpora, so no migration is required.
pub const CASE_SEQUENCE_VERSION: u8 = 3;

// ── Private helpers ────────────────────────────────────────────────────────

/// FNV-1a 64-bit hash of a byte slice.
fn fnv1a_64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 14_695_981_039_346_656_037;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    hash
}

/// One `SplitMix64` step, advancing `state` and returning the output.
fn splitmix64_advance(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// Single `SplitMix64` step without a persistent mutable state (for seeding
/// stream seeds from a fixed value).
fn splitmix64_once(value: u64) -> u64 {
    let mut s = value;
    splitmix64_advance(&mut s)
}

/// xoshiro256\*\* step — advances `state` and returns the next output.
fn xoshiro256ss(state: &mut [u64; 4]) -> u64 {
    let result = state[1].wrapping_mul(5).rotate_left(7).wrapping_mul(9);
    let t = state[1] << 17;
    state[2] ^= state[0];
    state[3] ^= state[1];
    state[1] ^= state[2];
    state[0] ^= state[3];
    state[2] ^= t;
    state[3] = state[3].rotate_left(45);
    result
}

/// Convert a single hex nibble byte to its value, or `None` for invalid input.
fn hex_nibble(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

// ── TestSeed ───────────────────────────────────────────────────────────────

/// A deterministic 64-bit seed for test case generation.
///
/// Seeds appear verbatim in failure reports and replay commands so a
/// developer can reproduce any failure by re-running with the same seed.
/// The same seed combined with the same stream name always produces an
/// identical sequence of test inputs.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TestSeed(u64);

impl TestSeed {
    /// Creates a seed from a raw 64-bit value.
    #[must_use]
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    /// Returns the raw seed value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }

    /// Derives a stream-specific seed from the primary seed and a stream name.
    ///
    /// The derivation applies FNV-1a-64 to the stream name, XORs the result
    /// with the primary seed, then applies one `SplitMix64` step.  This
    /// produces distinct seeds for distinct names with high probability (≈
    /// 1/2⁶⁴ collision probability per pair).  The result is **not**
    /// guaranteed unique in the mathematical sense; document stream names in
    /// [`crate::runner::CaseFailure`] and [`crate::corpus::CorpusEntry`] to
    /// ensure any failure can be unambiguously replayed.
    #[must_use]
    pub fn for_stream(self, stream_name: &str) -> Self {
        let name_hash = fnv1a_64(stream_name.as_bytes());
        Self(splitmix64_once(name_hash ^ self.0))
    }

    /// Derives a stream-specific seed for **case identification** (V3; see
    /// [`CASE_SEQUENCE_VERSION`]).
    ///
    /// Equivalent to `self.for_stream("{CASE_SEQUENCE_VERSION}\x00{stream_name}")`.
    /// The leading version tag domain-separates the resulting seed from
    /// `self.for_stream(stream_name)` (used for other, non-case purposes) and
    /// from any future `CASE_SEQUENCE_VERSION` bump, so [`CaseId`]s derived
    /// under different case-sequence versions can never alias.
    #[must_use]
    pub fn for_case_stream(self, stream_name: &str) -> Self {
        self.for_stream(&format!("{CASE_SEQUENCE_VERSION}\x00{stream_name}"))
    }
}

impl fmt::Display for TestSeed {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.0)
    }
}

// ── TestRng ────────────────────────────────────────────────────────────────

/// A deterministic pseudo-random number generator based on xoshiro256\*\*.
///
/// The state is seeded from a [`TestSeed`] using four `SplitMix64` steps so
/// that a zero seed does not produce an all-zero xoshiro state. The generator
/// is `Clone` to support forking a stream at any point.
///
/// # References
///
/// Vigna, S. (2019). "xoshiro/xoroshiro generators and the PRNG shootout."
/// <https://prng.di.unimi.it/>
#[derive(Clone, Debug)]
pub struct TestRng {
    state: [u64; 4],
}

impl TestRng {
    /// Initialises the generator from `seed` using four `SplitMix64` steps.
    ///
    /// Identical seeds always produce identical output sequences.
    #[must_use]
    pub fn from_seed(seed: TestSeed) -> Self {
        let mut s = seed.get();
        let s0 = splitmix64_advance(&mut s);
        let s1 = splitmix64_advance(&mut s);
        let s2 = splitmix64_advance(&mut s);
        let s3 = splitmix64_advance(&mut s);
        Self {
            state: [s0, s1, s2, s3],
        }
    }

    /// Returns the next uniformly distributed 64-bit integer.
    pub fn next_u64(&mut self) -> u64 {
        xoshiro256ss(&mut self.state)
    }

    /// Returns the next uniformly distributed floating-point value in `[0, 1)`.
    ///
    /// Uses the upper 53 bits of the next 64-bit output to construct a
    /// correctly rounded IEEE 754 double in `[1.0, 2.0)` then subtracts 1.
    pub fn next_f64(&mut self) -> f64 {
        let bits = (self.next_u64() >> 11) | 0x3ff0_0000_0000_0000_u64;
        f64::from_bits(bits) - 1.0
    }

    /// Returns the next random boolean.
    pub fn next_bool(&mut self) -> bool {
        (self.next_u64() & 1) != 0
    }
}

// ── CaseIdParseError ───────────────────────────────────────────────────────

/// Error returned when a [`CaseId`] cannot be parsed from a hex string.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CaseIdParseError;

impl fmt::Display for CaseIdParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("case IDs must be exactly 32 lowercase hexadecimal characters")
    }
}

impl Error for CaseIdParseError {}

// ── CaseId ─────────────────────────────────────────────────────────────────

/// A stable, deterministic identifier for a single generated test case.
///
/// A `CaseId` encodes the per-stream seed and the sequential case index.
/// Two cases with the same `CaseId` were produced from the same stream seed
/// at the same index and will produce the same inputs when replayed.
///
/// **Note**: the stream _name_ is not stored inside the `CaseId`.  To make a
/// case unambiguously replayable you must also record the stream name (e.g. in
/// [`crate::runner::CaseFailure::stream_name`] or
/// [`crate::corpus::CorpusEntry`]).  `CaseId`s from different streams that
/// happen to share the same stream seed (a ≈ 1/2⁶⁴ collision) would be
/// structurally identical even though they represent different cases.
///
/// The identifier is serialised as a 32-character lowercase hexadecimal
/// string so it can appear in failure reports and corpus file names.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct CaseId {
    stream_seed: u64,
    case_index: u64,
}

impl CaseId {
    /// Derives a case identifier from a stream seed and a sequential index.
    ///
    /// The stream seed should be obtained from [`TestSeed::for_stream`] so
    /// that IDs from different streams do not collide.
    #[must_use]
    pub const fn new(stream_seed: TestSeed, case_index: u64) -> Self {
        Self {
            stream_seed: stream_seed.get(),
            case_index,
        }
    }

    /// Returns the stream-seed component of this identifier.
    #[must_use]
    pub const fn stream_seed(self) -> u64 {
        self.stream_seed
    }

    /// Returns the sequential index component of this identifier.
    #[must_use]
    pub const fn case_index(self) -> u64 {
        self.case_index
    }

    /// Returns the canonical 16-byte little-endian representation.
    ///
    /// The first eight bytes encode the stream seed; the last eight encode
    /// the case index, both in little-endian order.
    #[must_use]
    pub fn to_bytes(self) -> [u8; 16] {
        let mut bytes = [0u8; 16];
        bytes[..8].copy_from_slice(&self.stream_seed.to_le_bytes());
        bytes[8..].copy_from_slice(&self.case_index.to_le_bytes());
        bytes
    }

    /// Formats the identifier as a 32-character lowercase hex string.
    #[must_use]
    pub fn to_hex(self) -> String {
        use core::fmt::Write as _;
        self.to_bytes()
            .iter()
            .fold(String::with_capacity(32), |mut s, b| {
                // Writing to a String is infallible.
                let _ = write!(s, "{b:02x}");
                s
            })
    }

    /// Parses a case identifier from its 32-character hexadecimal string.
    ///
    /// # Errors
    ///
    /// Returns [`CaseIdParseError`] when the input is not exactly 32
    /// hexadecimal characters.
    pub fn from_hex(s: &str) -> Result<Self, CaseIdParseError> {
        if s.len() != 32 {
            return Err(CaseIdParseError);
        }
        let mut bytes = [0u8; 16];
        for (i, chunk) in s.as_bytes().chunks(2).enumerate() {
            let hi = hex_nibble(chunk[0]).ok_or(CaseIdParseError)?;
            let lo = hex_nibble(chunk[1]).ok_or(CaseIdParseError)?;
            bytes[i] = (hi << 4) | lo;
        }
        let mut seed_bytes = [0u8; 8];
        seed_bytes.copy_from_slice(&bytes[..8]);
        let mut idx_bytes = [0u8; 8];
        idx_bytes.copy_from_slice(&bytes[8..]);
        Ok(Self {
            stream_seed: u64::from_le_bytes(seed_bytes),
            case_index: u64::from_le_bytes(idx_bytes),
        })
    }
}

impl fmt::Display for CaseId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.to_hex())
    }
}

impl TryFrom<String> for CaseId {
    type Error = CaseIdParseError;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        Self::from_hex(&s)
    }
}

impl From<CaseId> for String {
    fn from(id: CaseId) -> Self {
        id.to_hex()
    }
}

// ── Per-case RNG derivation (V3) ───────────────────────────────────────────

/// Derives an independent per-case [`TestRng`] in O(1) from a primary seed,
/// stream name, and case index (V3 derivation; see [`CASE_SEQUENCE_VERSION`]).
///
/// The NUL bytes between `CASE_SEQUENCE_VERSION`, `stream_name`, and
/// `case_index` prevent domain-extension collisions: since valid tokens
/// never contain NUL, the pair `(stream="a", idx=23)` produces key
/// `"3\x00a\x0023"` which is distinct from `(stream="a2", idx=3)` →
/// `"3\x00a2\x003"`.  The leading version tag additionally prevents any
/// future `CASE_SEQUENCE_VERSION` bump from aliasing a V3 key.
pub(crate) fn derive_case_rng(seed: TestSeed, stream_name: &str, case_index: u64) -> TestRng {
    let key = format!("{CASE_SEQUENCE_VERSION}\x00{stream_name}\x00{case_index}");
    TestRng::from_seed(seed.for_stream(&key))
}

/// Derives an independent per-(case, relation) [`TestRng`] for metamorphic
/// transform sampling (V3 derivation).
///
/// Key: `"{CASE_SEQUENCE_VERSION}\x00t\x00{stream}\x00{relation}\x00{case_index}"`
/// — distinct from the generator key and from other relation keys.
pub(crate) fn derive_transform_rng(
    seed: TestSeed,
    stream_name: &str,
    relation_name: &str,
    case_index: u64,
) -> TestRng {
    let key =
        format!("{CASE_SEQUENCE_VERSION}\x00t\x00{stream_name}\x00{relation_name}\x00{case_index}");
    TestRng::from_seed(seed.for_stream(&key))
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{CaseId, TestRng, TestSeed};

    #[test]
    fn identical_seeds_produce_identical_sequences() {
        let seed = TestSeed::new(0xdead_beef_cafe_babe);
        let mut rng_a = TestRng::from_seed(seed);
        let mut rng_b = TestRng::from_seed(seed);
        for _ in 0..256 {
            assert_eq!(rng_a.next_u64(), rng_b.next_u64());
        }
    }

    #[test]
    fn different_seeds_produce_different_sequences() {
        let mut rng_a = TestRng::from_seed(TestSeed::new(1));
        let mut rng_b = TestRng::from_seed(TestSeed::new(2));
        let any_differs = (0..16).any(|_| rng_a.next_u64() != rng_b.next_u64());
        assert!(any_differs, "distinct seeds must not share state");
    }

    #[test]
    fn stream_seeds_are_isolated_from_each_other() {
        let seed = TestSeed::new(42);
        let stream_a = seed.for_stream("geometry");
        let stream_b = seed.for_stream("topology");
        assert_ne!(
            stream_a, stream_b,
            "distinct names must produce distinct seeds"
        );
        let mut rng_a = TestRng::from_seed(stream_a);
        let mut rng_b = TestRng::from_seed(stream_b);
        let any_differs = (0..16).any(|_| rng_a.next_u64() != rng_b.next_u64());
        assert!(any_differs, "streams must not share RNG state");
    }

    #[test]
    fn same_stream_name_is_deterministic() {
        let seed = TestSeed::new(99);
        assert_eq!(seed.for_stream("foo"), seed.for_stream("foo"));
    }

    #[test]
    fn next_f64_is_in_unit_interval() {
        let mut rng = TestRng::from_seed(TestSeed::new(7));
        for _ in 0..1_000 {
            let v = rng.next_f64();
            assert!(
                (0.0..1.0).contains(&v),
                "next_f64 produced {v}, expected [0, 1)"
            );
        }
    }

    #[test]
    fn case_id_hex_round_trips() {
        let seed = TestSeed::new(0xffff_ffff_0000_0000);
        let id = CaseId::new(seed, 42);
        let hex = id.to_hex();
        assert_eq!(hex.len(), 32, "case ID hex must be 32 chars");
        let parsed = CaseId::from_hex(&hex).expect("round-trip must succeed");
        assert_eq!(id, parsed);
    }

    #[test]
    fn case_id_serde_round_trips() {
        let seed = TestSeed::new(1234);
        let id = CaseId::new(seed.for_stream("test"), 7);
        let json = serde_json::to_string(&id).expect("serialisation");
        let decoded: CaseId = serde_json::from_str(&json).expect("deserialisation");
        assert_eq!(id, decoded);
    }

    #[test]
    fn malformed_case_id_hex_is_rejected() {
        assert!(
            CaseId::from_hex("not-hex").is_err(),
            "non-hex must be rejected"
        );
        assert!(
            CaseId::from_hex("").is_err(),
            "empty string must be rejected"
        );
        assert!(
            CaseId::from_hex("abcd").is_err(),
            "too-short hex must be rejected"
        );
        // Correct length but invalid hex character
        assert!(
            CaseId::from_hex("zzzzzzzzzzzzzzzzzzzzzzzzzzzzzzzz").is_err(),
            "invalid hex chars must be rejected"
        );
    }

    #[test]
    fn case_ids_from_different_streams_differ() {
        let primary = TestSeed::new(100);
        let id_a = CaseId::new(primary.for_stream("alpha"), 0);
        let id_b = CaseId::new(primary.for_stream("beta"), 0);
        assert_ne!(
            id_a, id_b,
            "same index in different streams produces different IDs for these names"
        );
    }

    #[test]
    fn sequential_case_ids_are_unique() {
        let stream = TestSeed::new(5).for_stream("s");
        let ids: Vec<CaseId> = (0..100).map(|i| CaseId::new(stream, i)).collect();
        let deduped: std::collections::HashSet<_> = ids.iter().collect();
        assert_eq!(ids.len(), deduped.len(), "sequential IDs must be unique");
    }

    #[test]
    fn case_sequence_version_is_3() {
        assert_eq!(super::CASE_SEQUENCE_VERSION, 3, "V3 versioned domain keys");
    }

    #[test]
    fn for_case_stream_differs_from_plain_for_stream() {
        let seed = TestSeed::new(123);
        assert_ne!(
            seed.for_case_stream("s"),
            seed.for_stream("s"),
            "the version tag must domain-separate for_case_stream from for_stream"
        );
    }

    #[test]
    fn for_case_stream_is_deterministic() {
        let seed = TestSeed::new(456);
        assert_eq!(seed.for_case_stream("x"), seed.for_case_stream("x"));
    }

    #[test]
    fn for_case_stream_matches_versioned_formula() {
        let seed = TestSeed::new(789);
        let expected = seed.for_stream(&format!("{}\x00stream", super::CASE_SEQUENCE_VERSION));
        assert_eq!(seed.for_case_stream("stream"), expected);
    }
}
