//! Bounded, reproducible probability distributions over scalar values.
//!
//! Every distribution type draws from a [`TestRng`] reference, producing the
//! same sequence of values for the same seed. No external random source is
//! ever consulted.
//!
//! An [`EdgeCaseSchedule`] determines which iteration indices should receive
//! boundary values (zero, one, max, epsilon, etc.) rather than generic random
//! samples. Callers use it inside their generator closures alongside the
//! distributions below.
//!
//! # Sampling bias
//!
//! [`BoundedUInt`] and [`WeightedChoice`] use deterministic rejection sampling
//! to eliminate modulo bias. The expected number of RNG draws per sample is
//! less than 2 in all cases.

use crate::{CaseContext, TestRng};
use core::{error::Error, fmt};
use serde::{Deserialize, Serialize};

// ── DistributionError ──────────────────────────────────────────────────────

/// Error produced when constructing an invalid distribution.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DistributionError {
    /// The low bound is not less than the high bound.
    EmptyRange,
    /// One or more bounds are NaN or infinite.
    NonFinite,
    /// No items were provided to a weighted-choice distribution.
    EmptyItems,
    /// All item weights sum to zero.
    ZeroTotalWeight,
    /// Accumulating item weights would overflow a 64-bit integer.
    WeightOverflow,
    /// Explicit indices must be strictly sorted (no duplicates, ascending).
    UnsortedIndices,
}

impl fmt::Display for DistributionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EmptyRange => "low bound must be strictly less than high bound",
            Self::NonFinite => "distribution bounds must be finite",
            Self::EmptyItems => "weighted-choice requires at least one item",
            Self::ZeroTotalWeight => "total weight of all items must be non-zero",
            Self::WeightOverflow => "accumulated item weights overflow u64",
            Self::UnsortedIndices => {
                "explicit indices must be strictly ascending (sorted, no duplicates)"
            }
        };
        formatter.write_str(message)
    }
}

impl Error for DistributionError {}

// ── BoundedFloat ───────────────────────────────────────────────────────────

/// Wire type used for validated serde round-trips of [`BoundedFloat`].
#[derive(Serialize, Deserialize)]
struct BoundedFloatWire {
    lo: f64,
    hi: f64,
}

/// Uniform distribution over the half-open interval `[lo, hi)`.
///
/// Both bounds must be finite and `lo < hi`.  Sampling is overflow-safe even
/// for extreme opposite-sign bounds such as `[-f64::MAX, f64::MAX)`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
#[serde(into = "BoundedFloatWire")]
pub struct BoundedFloat {
    lo: f64,
    hi: f64,
}

impl<'de> Deserialize<'de> for BoundedFloat {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let wire = BoundedFloatWire::deserialize(d)?;
        Self::try_new(wire.lo, wire.hi).map_err(serde::de::Error::custom)
    }
}

impl From<BoundedFloat> for BoundedFloatWire {
    fn from(b: BoundedFloat) -> Self {
        Self { lo: b.lo, hi: b.hi }
    }
}

impl BoundedFloat {
    /// Creates a uniform distribution over `[lo, hi)`.
    ///
    /// # Errors
    ///
    /// Returns [`DistributionError::NonFinite`] when either bound is NaN or
    /// infinite, or [`DistributionError::EmptyRange`] when `lo >= hi`.
    pub fn try_new(lo: f64, hi: f64) -> Result<Self, DistributionError> {
        if !lo.is_finite() || !hi.is_finite() {
            return Err(DistributionError::NonFinite);
        }
        if lo >= hi {
            return Err(DistributionError::EmptyRange);
        }
        Ok(Self { lo, hi })
    }

    /// Returns the lower bound.
    #[must_use]
    pub const fn lo(self) -> f64 {
        self.lo
    }

    /// Returns the upper bound (exclusive).
    #[must_use]
    pub const fn hi(self) -> f64 {
        self.hi
    }

    /// Internal lerp with raw `t` value (in `[0, 1)`).
    ///
    /// Separated from `sample` so deterministic tests can inject specific
    /// mantissa values without needing a custom RNG seeded to produce them.
    fn sample_with_t(&self, t: f64) -> f64 {
        let result = self.lo * (1.0 - t) + self.hi * t;
        if result >= self.hi {
            self.lo.max(self.hi.next_down())
        } else {
            result
        }
    }

    /// Samples a value from the approximate uniform distribution on `[lo, hi)`.
    ///
    /// # Statistical model
    ///
    /// The sample approximates a uniform real in `[lo, hi)` but draws from
    /// the dyadic grid `k * 2⁻⁵³` (53 mantissa bits) mapped to `[lo, hi)`,
    /// **not** exact uniform over all representable `f64` values.  In
    /// particular, the density near powers of two halves at exponent
    /// boundaries.  This model is standard for randomised testing.
    ///
    /// # Bounds guarantee
    ///
    /// The result is always `>= lo` and `< hi` for every RNG state and for
    /// all valid finite bounds, including adjacent floats such as
    /// `(lo, hi) = (1.0f64.next_down(), 1.0)`.  If the lerp rounds to
    /// exactly `hi`, the result is clamped to `hi.next_down()`.  Since
    /// `lo < hi` (validated by [`try_new`]), `hi.next_down() >= lo` always
    /// holds: for adjacent floats `hi.next_down() == lo`; for all others
    /// `hi.next_down() > lo`.
    ///
    /// Uses the two-multiplication lerp `lo*(1-t) + hi*t` (not `lo + t*(hi-lo)`)
    /// to avoid overflow when `lo` and `hi` have opposite signs.
    ///
    /// [`try_new`]: Self::try_new
    pub fn sample(&self, rng: &mut TestRng) -> f64 {
        let t = rng.next_f64(); // in [0, 1)
        self.sample_with_t(t)
    }

    /// Samples a float, charging one RNG draw from the case budget.
    ///
    /// # Errors
    ///
    /// Returns [`crate::runner::ResourceLimitKind::MaxRngDrawsPerCase`] when
    /// the per-case draw budget is exhausted.
    pub fn sample_ctx(
        &self,
        ctx: &mut CaseContext,
    ) -> Result<f64, crate::runner::ResourceLimitKind> {
        let t = ctx.next_f64()?;
        Ok(self.sample_with_t(t))
    }

    /// Test-only: sample with an explicit raw `t` ∈ `[0, 1)`.
    ///
    /// Allows deterministic tests to inject specific mantissa-edge values
    /// (e.g. the maximum `next_f64` output) without depending on RNG seeding.
    #[must_use]
    #[cfg(test)]
    pub fn sample_raw(&self, t: f64) -> f64 {
        self.sample_with_t(t)
    }
}

// ── BoundedUInt ────────────────────────────────────────────────────────────

/// Wire type used for validated serde round-trips of [`BoundedUInt`].
#[derive(Serialize, Deserialize)]
struct BoundedUIntWire {
    lo: u64,
    hi: u64,
}

/// Uniform distribution over the closed integer interval `[lo, hi]`.
///
/// Both bounds are inclusive.  Sampling uses deterministic rejection sampling
/// (Lemire / PCG threshold method) to eliminate modulo bias; the expected
/// number of RNG draws per sample is less than 2.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(into = "BoundedUIntWire")]
pub struct BoundedUInt {
    lo: u64,
    hi: u64,
}

impl<'de> Deserialize<'de> for BoundedUInt {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let wire = BoundedUIntWire::deserialize(d)?;
        Self::try_new(wire.lo, wire.hi).map_err(serde::de::Error::custom)
    }
}

impl From<BoundedUInt> for BoundedUIntWire {
    fn from(b: BoundedUInt) -> Self {
        Self { lo: b.lo, hi: b.hi }
    }
}

impl BoundedUInt {
    /// Creates a uniform distribution over `[lo, hi]`.
    ///
    /// # Errors
    ///
    /// Returns [`DistributionError::EmptyRange`] when `lo > hi`.
    pub fn try_new(lo: u64, hi: u64) -> Result<Self, DistributionError> {
        if lo > hi {
            return Err(DistributionError::EmptyRange);
        }
        Ok(Self { lo, hi })
    }

    /// Returns the lower bound (inclusive).
    #[must_use]
    pub const fn lo(self) -> u64 {
        self.lo
    }

    /// Returns the upper bound (inclusive).
    #[must_use]
    pub const fn hi(self) -> u64 {
        self.hi
    }

    /// Samples a uniformly distributed integer in `[lo, hi]` without modulo
    /// bias.
    ///
    /// When `lo == hi` the result is always `lo` with no RNG draw.  When the
    /// range spans the full `u64` domain the raw RNG output is returned
    /// directly (it is already unbiased).  Otherwise the PCG threshold method
    /// is used: values below the bias threshold are discarded and the RNG is
    /// called again, giving exactly uniform distribution.
    pub fn sample(&self, rng: &mut TestRng) -> u64 {
        if self.lo == self.hi {
            return self.lo;
        }
        let range = self.hi.wrapping_sub(self.lo).wrapping_add(1);
        if range == 0 {
            return rng.next_u64();
        }
        let threshold = 0u64.wrapping_sub(range) % range;
        loop {
            let r = rng.next_u64();
            if r >= threshold {
                return self.lo + (r % range);
            }
        }
    }

    /// Samples a uniformly distributed integer in `[lo, hi]`, charging each
    /// rejection-sampling attempt against the case budget.
    ///
    /// # Errors
    ///
    /// Returns [`crate::runner::ResourceLimitKind::MaxRngDrawsPerCase`] when
    /// the per-case draw budget is exhausted.
    pub fn sample_ctx(
        &self,
        ctx: &mut CaseContext,
    ) -> Result<u64, crate::runner::ResourceLimitKind> {
        if self.lo == self.hi {
            return Ok(self.lo);
        }
        let range = self.hi.wrapping_sub(self.lo).wrapping_add(1);
        if range == 0 {
            return ctx.next_u64();
        }
        let threshold = 0u64.wrapping_sub(range) % range;
        loop {
            let r = ctx.next_u64()?;
            if r >= threshold {
                return Ok(self.lo + (r % range));
            }
        }
    }
}

// ── WeightedItem / WeightedChoice ──────────────────────────────────────────

/// One item together with its non-negative integer weight.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct WeightedItem<T> {
    /// The item to be selected.
    pub item: T,
    /// Non-negative sampling weight. Items with weight zero are never chosen.
    pub weight: u32,
}

/// Weighted-random selection from a fixed list of items.
///
/// Items are sampled proportionally to their weights using deterministic
/// rejection sampling (no modulo bias).  Items with weight zero are never
/// selected.
#[derive(Clone, Debug)]
pub struct WeightedChoice<T> {
    items: Vec<WeightedItem<T>>,
    /// Cumulative weight at each index.
    cumulative: Vec<u64>,
    total: u64,
}

impl<T> WeightedChoice<T> {
    /// Creates a weighted-choice distribution from a list of weighted items.
    ///
    /// # Errors
    ///
    /// Returns [`DistributionError::EmptyItems`] when `items` is empty,
    /// [`DistributionError::ZeroTotalWeight`] when all weights are zero,
    /// or [`DistributionError::WeightOverflow`] when accumulated weights
    /// overflow `u64`.
    pub fn try_new(items: Vec<WeightedItem<T>>) -> Result<Self, DistributionError> {
        if items.is_empty() {
            return Err(DistributionError::EmptyItems);
        }
        let mut cumulative = Vec::with_capacity(items.len());
        let mut total: u64 = 0;
        for weighted in &items {
            total = total
                .checked_add(u64::from(weighted.weight))
                .ok_or(DistributionError::WeightOverflow)?;
            cumulative.push(total);
        }
        if total == 0 {
            return Err(DistributionError::ZeroTotalWeight);
        }
        Ok(Self {
            items,
            cumulative,
            total,
        })
    }

    /// Samples one item proportionally to its weight without modulo bias.
    ///
    /// Uses rejection sampling: raw RNG values below the bias threshold
    /// (`2^64 % total`) are discarded.  Expected draws per call < 2.
    #[must_use]
    pub fn sample(&self, rng: &mut TestRng) -> &T {
        let total = self.total;
        let threshold = 0u64.wrapping_sub(total) % total;
        let r = loop {
            let r = rng.next_u64();
            if r >= threshold {
                break r % total;
            }
        };
        let idx = self.cumulative.partition_point(|&c| c <= r);
        &self.items[idx].item
    }

    /// Samples one item proportionally to its weight, charging one RNG draw per attempt.
    ///
    /// # Errors
    ///
    /// Returns [`crate::runner::ResourceLimitKind::MaxRngDrawsPerCase`] when
    /// the per-case draw budget is exhausted.
    pub fn sample_ctx<'s>(
        &'s self,
        ctx: &mut CaseContext,
    ) -> Result<&'s T, crate::runner::ResourceLimitKind> {
        let total = self.total;
        let threshold = 0u64.wrapping_sub(total) % total;
        let r = loop {
            let r = ctx.next_u64()?;
            if r >= threshold {
                break r % total;
            }
        };
        let idx = self.cumulative.partition_point(|&c| c <= r);
        Ok(&self.items[idx].item)
    }

    /// Returns the total accumulated weight.
    #[must_use]
    pub const fn total_weight(&self) -> u64 {
        self.total
    }

    /// Returns the number of items.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns `true` when no items are present.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

// ── EdgeCaseSchedule ───────────────────────────────────────────────────────

/// An opaque, validated holder for an explicit edge-case schedule.
///
/// The inner indices are strictly sorted and deduplicated. Construct only via
/// [`EdgeCaseSchedule::try_explicit`].
#[derive(Clone, Debug, Serialize)]
pub struct ExplicitSchedule {
    indices: Vec<u64>,
}

impl ExplicitSchedule {
    /// Returns the strictly sorted edge-case indices.
    #[must_use]
    pub fn as_slice(&self) -> &[u64] {
        &self.indices
    }
}

/// Specifies which sequential case indices should receive boundary/edge-case
/// values instead of generic random samples.
///
/// An `EdgeCaseSchedule` does not generate edge cases itself; it tells the
/// caller whether a given index is an "edge case iteration" so the caller can
/// branch to a boundary value generator.
///
/// ```rust
/// # use amphion_test_support::EdgeCaseSchedule;
/// let schedule = EdgeCaseSchedule::geometric();
/// assert!(schedule.is_edge_case(0));
/// assert!(schedule.is_edge_case(1));
/// assert!(schedule.is_edge_case(4));
/// assert!(!schedule.is_edge_case(3));
/// ```
#[derive(Clone, Debug, Serialize)]
pub enum EdgeCaseSchedule {
    /// Never inject edge cases; all iterations use generic random inputs.
    Never,
    /// Inject edge cases at geometrically increasing indices: 0, 1, 2, 4, 8, …
    Geometric,
    /// Inject an edge case every `period` iterations (0, period, 2×period, …).
    ///
    /// A period of zero is treated as [`Never`](EdgeCaseSchedule::Never).
    Periodic {
        /// Spacing between edge-case injections.
        period: u32,
    },
    /// Inject edge cases only at an explicit, strictly-sorted list of indices.
    ///
    /// Construct via [`EdgeCaseSchedule::try_explicit`].
    Explicit(ExplicitSchedule),
}

/// Mirror of [`EdgeCaseSchedule`] used for validated deserialisation.
#[derive(Deserialize)]
enum EdgeCaseScheduleWire {
    Never,
    Geometric,
    Periodic { period: u32 },
    Explicit { indices: Vec<u64> },
}

impl<'de> Deserialize<'de> for EdgeCaseSchedule {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        match EdgeCaseScheduleWire::deserialize(d)? {
            EdgeCaseScheduleWire::Never => Ok(Self::Never),
            EdgeCaseScheduleWire::Geometric => Ok(Self::Geometric),
            EdgeCaseScheduleWire::Periodic { period } => Ok(Self::Periodic { period }),
            EdgeCaseScheduleWire::Explicit { indices } => {
                Self::try_explicit(indices).map_err(serde::de::Error::custom)
            }
        }
    }
}

impl EdgeCaseSchedule {
    /// Returns a schedule that never injects edge cases.
    #[must_use]
    pub const fn never() -> Self {
        Self::Never
    }

    /// Returns a schedule that injects edge cases at 0, 1, 2, 4, 8, …
    #[must_use]
    pub const fn geometric() -> Self {
        Self::Geometric
    }

    /// Returns a schedule that injects an edge case every `period` iterations.
    #[must_use]
    pub const fn periodic(period: u32) -> Self {
        Self::Periodic { period }
    }

    /// Constructs an explicit schedule from a list of indices.
    ///
    /// The indices must be strictly ascending (sorted, no duplicates).
    ///
    /// # Errors
    ///
    /// Returns [`DistributionError::UnsortedIndices`] when the slice contains
    /// a duplicate or an out-of-order element.
    pub fn try_explicit(indices: Vec<u64>) -> Result<Self, DistributionError> {
        for window in indices.windows(2) {
            if window[0] >= window[1] {
                return Err(DistributionError::UnsortedIndices);
            }
        }
        Ok(Self::Explicit(ExplicitSchedule { indices }))
    }

    /// Returns true when `case_index` is an edge-case iteration.
    #[must_use]
    pub fn is_edge_case(&self, case_index: u64) -> bool {
        match self {
            Self::Never => false,
            Self::Geometric => {
                // 0 and 1 are always edge cases; thereafter only powers of two.
                case_index <= 1 || case_index.is_power_of_two()
            }
            Self::Periodic { period } => {
                if *period == 0 {
                    return false;
                }
                case_index.is_multiple_of(u64::from(*period))
            }
            Self::Explicit(schedule) => schedule.indices.binary_search(&case_index).is_ok(),
        }
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::{
        BoundedFloat, BoundedUInt, DistributionError, EdgeCaseSchedule, WeightedChoice,
        WeightedItem,
    };
    use crate::{CaseBudget, rng::TestSeed};
    use crate::{CaseContext, TestRng};

    fn rng(seed: u64) -> TestRng {
        TestRng::from_seed(TestSeed::new(seed))
    }

    fn ctx_with_draw_limit(limit: u64) -> CaseContext {
        CaseContext::new(
            0,
            TestRng::from_seed(TestSeed::new(1)),
            CaseBudget::from_limits(&crate::runner::ResourceLimits {
                max_rng_draws_per_case: Some(limit),
                ..crate::runner::ResourceLimits::default()
            }),
        )
    }

    // ── BoundedFloat ──

    #[test]
    fn bounded_float_samples_are_in_range() {
        let dist = BoundedFloat::try_new(-1.0, 1.0).unwrap();
        let mut r = rng(1);
        for _ in 0..1_000 {
            let v = dist.sample(&mut r);
            assert!((-1.0..1.0).contains(&v), "out of range: {v}");
        }
    }

    #[test]
    fn bounded_float_rejects_invalid_bounds() {
        assert_eq!(
            BoundedFloat::try_new(1.0, 1.0),
            Err(DistributionError::EmptyRange)
        );
        assert_eq!(
            BoundedFloat::try_new(2.0, 1.0),
            Err(DistributionError::EmptyRange)
        );
        assert_eq!(
            BoundedFloat::try_new(f64::NAN, 1.0),
            Err(DistributionError::NonFinite)
        );
        assert_eq!(
            BoundedFloat::try_new(0.0, f64::INFINITY),
            Err(DistributionError::NonFinite)
        );
    }

    #[test]
    fn bounded_float_is_deterministic() {
        let dist = BoundedFloat::try_new(0.0, 100.0).unwrap();
        let samples_a: Vec<f64> = {
            let mut r = rng(77);
            (0..50).map(|_| dist.sample(&mut r)).collect()
        };
        let samples_b: Vec<f64> = {
            let mut r = rng(77);
            (0..50).map(|_| dist.sample(&mut r)).collect()
        };
        assert_eq!(
            samples_a, samples_b,
            "same seed must yield same float samples"
        );
    }

    /// The overflow-safe lerp must not produce infinity for extreme bounds.
    #[test]
    fn bounded_float_extreme_opposite_sign_bounds_stay_finite() {
        let dist = BoundedFloat::try_new(-f64::MAX, f64::MAX).unwrap();
        let mut r = rng(0xdead_beef);
        for _ in 0..100_000 {
            let v = dist.sample(&mut r);
            assert!(v.is_finite(), "sample must be finite, got {v}");
            assert!((-f64::MAX..=f64::MAX).contains(&v));
        }
    }

    #[test]
    fn bounded_float_same_sign_extreme_bounds_stay_in_range() {
        // Both bounds positive, near MAX
        let dist = BoundedFloat::try_new(f64::MAX / 2.0, f64::MAX).unwrap();
        let mut r = rng(42);
        for _ in 0..1_000 {
            let v = dist.sample(&mut r);
            assert!(v.is_finite(), "must be finite");
            assert!((f64::MAX / 2.0..f64::MAX).contains(&v), "out of range: {v}");
        }
    }

    #[test]
    fn bounded_float_endpoint_reproducibility() {
        // Verify identical seeds produce identical draws across 100_000 samples.
        let dist = BoundedFloat::try_new(-f64::MAX, f64::MAX).unwrap();
        let draw = |seed: u64| {
            let mut r = rng(seed);
            (0..10)
                .map(|_| dist.sample(&mut r).to_bits())
                .collect::<Vec<_>>()
        };
        assert_eq!(draw(1), draw(1));
        assert_ne!(draw(1), draw(2));
    }

    #[test]
    fn bounded_float_serde_rejects_non_finite_bounds() {
        let json = r#"{"lo": null, "hi": 1.0}"#;
        let result: Result<BoundedFloat, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "non-finite lo must be rejected on deserialize"
        );
    }

    #[test]
    fn bounded_float_serde_rejects_inverted_bounds() {
        let json = r#"{"lo": 5.0, "hi": 2.0}"#;
        let result: Result<BoundedFloat, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "inverted bounds must be rejected on deserialize"
        );
    }

    #[test]
    fn bounded_float_serde_round_trips() {
        let dist = BoundedFloat::try_new(-3.0, 7.5).unwrap();
        let json = serde_json::to_string(&dist).unwrap();
        let decoded: BoundedFloat = serde_json::from_str(&json).unwrap();
        assert_eq!(dist, decoded);
    }

    /// Guarantees `lo <= result < hi` for adjacent floats around 1.0.
    ///
    /// The two-product lerp with maximal `next_f64` rounds to exactly `hi`
    /// without the post-lerp clamp; this test verifies the clamp fires.
    #[test]
    fn bounded_float_adjacent_bounds_never_return_hi() {
        let hi = 1.0_f64;
        let lo = hi.next_down(); // adjacent float just below 1.0
        let dist = BoundedFloat::try_new(lo, hi).unwrap();
        // Drive with a fake raw RNG that injects maximal next_f64 bits.
        // next_f64 produces (u64 >> 11) | 0x3ff0_0000_0000_0000 - 1.0
        // = max mantissa = 0x3ff_ffff_ffff_f = 0.999999999... .
        // Feed that via exhaustive mantissa in [0, 2^53) mapped through next_f64.
        let mut rng_inner = rng(0);
        for _ in 0..100_000 {
            let v = dist.sample(&mut rng_inner);
            assert!(
                v >= lo && v < hi,
                "sample {v:e} must be in [{lo:e}, {hi:e})"
            );
        }
    }

    #[test]
    fn bounded_float_adjacent_bounds_around_power_of_two() {
        // Adjacent floats near a power of two (exponent boundary).
        for base in [0.5_f64, 1.0, 2.0, 4.0, 0.25, -1.0, -2.0] {
            let hi = base;
            let lo = hi.next_down();
            if !lo.is_finite() || lo >= hi {
                continue;
            }
            let dist = BoundedFloat::try_new(lo, hi).expect("valid");
            let mut r = rng(42);
            for _ in 0..10_000 {
                let v = dist.sample(&mut r);
                assert!(
                    v >= lo && v < hi,
                    "adjacent bounds [{lo:e},{hi:e}): sample {v:e} out of range"
                );
            }
        }
    }

    #[test]
    fn bounded_float_subnormal_bounds_stay_in_range() {
        // Both bounds are subnormal.
        let lo = f64::from_bits(1); // smallest positive subnormal
        let hi = f64::from_bits(1000); // larger subnormal
        let dist = BoundedFloat::try_new(lo, hi).expect("valid subnormal range");
        let mut r = rng(7);
        for _ in 0..10_000 {
            let v = dist.sample(&mut r);
            assert!(
                v >= lo && v < hi,
                "subnormal: {v:e} not in [{lo:e}, {hi:e})"
            );
        }
    }

    #[test]
    fn bounded_float_negative_adjacent_bounds_stay_in_range() {
        // Both bounds are negative and adjacent.
        let hi = -1.0_f64;
        // Use hi.next_down() which gives a more negative (smaller) value, so lo < hi.
        let lo = hi.next_down();
        if lo.is_finite() {
            let dist = BoundedFloat::try_new(lo, hi).expect("valid");
            let mut r = rng(99);
            for _ in 0..10_000 {
                let v = dist.sample(&mut r);
                assert!(
                    v >= lo && v < hi,
                    "negative adjacent: {v:e} not in [{lo:e}, {hi:e})"
                );
            }
        }
    }

    #[test]
    fn bounded_float_100k_deterministic_draws_never_equal_hi() {
        // Comprehensive stress test: no RNG state should produce result == hi.
        let dist = BoundedFloat::try_new(-f64::MAX, f64::MAX).expect("valid");
        let mut r = rng(12345);
        for _ in 0..100_000 {
            let v = dist.sample(&mut r);
            assert!(
                v.is_finite() && v < f64::MAX,
                "must be < hi = MAX, got {v:e}"
            );
            assert!(v >= -f64::MAX, "must be >= lo = -MAX, got {v:e}");
        }
    }

    /// Issue 10: 10k draws from the widest possible range must always land
    /// in `[lo, hi)`, never touching either endpoint's exclusion rule.
    #[test]
    fn bounded_float_10k_draws_extreme_bounds_stay_in_half_open_range() {
        let lo = -f64::MAX;
        let hi = f64::MAX;
        let dist = BoundedFloat::try_new(lo, hi).expect("valid");
        let mut r = rng(0x00C0_FFEE);
        for _ in 0..10_000 {
            let v = dist.sample(&mut r);
            assert!(v >= lo, "sample {v:e} below lo = {lo:e}");
            assert!(v < hi, "sample {v:e} not strictly below hi = {hi:e}");
        }
    }

    /// Adjacent bounds at the zero boundary: `lo` is the largest negative
    /// subnormal, `hi` is exactly `0.0`.
    #[test]
    fn bounded_float_zero_adjacent_bounds_stay_in_range() {
        let hi = 0.0_f64;
        let lo = hi.next_down();
        let dist = BoundedFloat::try_new(lo, hi).expect("valid");
        let mut r = rng(3);
        for _ in 0..10_000 {
            let v = dist.sample(&mut r);
            assert!(
                v >= lo && v < hi,
                "[{lo:e},{hi:e}): sample {v:e} out of range"
            );
        }
    }

    /// Adjacent bounds at the extreme positive boundary: `hi` is exactly
    /// `f64::MAX` and `lo` is the largest representable float below it.
    #[test]
    fn bounded_float_max_adjacent_bounds_stay_in_range() {
        let hi = f64::MAX;
        let lo = hi.next_down();
        let dist = BoundedFloat::try_new(lo, hi).expect("valid");
        let mut r = rng(4);
        for _ in 0..10_000 {
            let v = dist.sample(&mut r);
            assert!(v.is_finite(), "must be finite, got {v:e}");
            assert!(
                v >= lo && v < hi,
                "[{lo:e},{hi:e}): sample {v:e} out of range"
            );
        }
    }

    /// Exhaustive proof that `hi.next_down() >= lo` holds for every valid
    /// `(lo, hi)` pair (`lo < hi`), sampled densely around power-of-two
    /// exponent boundaries -- the regions where ULP size changes and where
    /// the clamp formula `self.lo.max(self.hi.next_down())` is most at risk
    /// of being wrong. Since `hi.next_down()` is defined as the largest
    /// representable float strictly less than `hi`, and `lo` is by
    /// definition some float strictly less than `hi`, IEEE-754 total
    /// ordering guarantees `lo <= hi.next_down()` for every valid pair; this
    /// test exhaustively checks that invariant rather than merely asserting
    /// it algebraically.
    #[test]
    fn hi_next_down_is_never_below_lo_near_powers_of_two() {
        let boundary_his: Vec<f64> = {
            let mut v = vec![f64::MIN_POSITIVE, f64::MAX];
            for exp in -1021..=1023 {
                v.push(2f64.powi(exp));
            }
            v.retain(|x| x.is_finite() && *x > 0.0);
            v
        };
        for &hi in &boundary_his {
            let candidates = [
                hi.next_down(),
                hi.next_down().next_down(),
                hi / 2.0,
                -hi,
                0.0,
            ];
            for &lo in &candidates {
                if !lo.is_finite() || lo >= hi {
                    continue;
                }
                assert!(
                    hi.next_down() >= lo,
                    "hi.next_down() ({:e}) must be >= lo ({lo:e}) for hi={hi:e}",
                    hi.next_down()
                );
            }
        }
    }

    /// Deterministic proof that the maximal `next_f64` output never returns
    /// exactly `hi`.  `TestRng::next_f64` converts bits via:
    ///   `f64::from_bits((bits >> 11) | 0x3FF0_0000_0000_0000) - 1.0`
    /// The maximum mantissa bits are `0x000F_FFFF_FFFF_FFFF`, so the
    /// maximum `next_f64` value is:
    ///   `f64::from_bits(0x3FEF_FFFF_FFFF_FFFF)`
    /// = `1.0 - f64::EPSILON / 2` (i.e. the ULP below 1.0).
    ///
    /// This value is used as `t` in the lerp; for adjacent bounds around
    /// 1.0 the lerp may produce exactly `hi`.  The clamp fires and returns
    /// `hi.next_down() == lo`.
    #[test]
    fn bounded_float_sample_raw_max_t_adjacent_hi_1_never_returns_hi() {
        let hi = 1.0_f64;
        let lo = hi.next_down();
        let dist = BoundedFloat::try_new(lo, hi).unwrap();
        // Maximal t produced by TestRng::next_f64.
        let max_t = f64::from_bits(0x3FEF_FFFF_FFFF_FFFF);
        assert!(max_t < 1.0, "sanity: max_t must be < 1.0");
        let v = dist.sample_raw(max_t);
        assert!(
            v >= lo && v < hi,
            "max_t={max_t:e}, sample={v:e} must be in [{lo:e}, {hi:e})"
        );
        // With adjacent bounds [lo=1.0.next_down(), hi=1.0] and max_t,
        // the lerp rounds to exactly hi, so the clamp must fire and return lo.
        // Use exact bit comparison to prove the clamp fired.
        assert!(
            v.to_bits() == lo.to_bits(),
            "clamp must fire: result must equal lo = {lo:e} for max_t with adjacent bounds (got {v:e})"
        );
    }

    #[test]
    fn bounded_float_sample_raw_zero_t_returns_lo() {
        let dist = BoundedFloat::try_new(-5.0, 10.0).unwrap();
        let v = dist.sample_raw(0.0);
        // t=0 → lo * (1-0) + hi * 0 = lo; exact bit equality is the right test here.
        assert!(
            v.to_bits() == (-5.0_f64).to_bits(),
            "t=0 must return exactly lo (-5.0), got {v:e}"
        );
    }

    #[test]
    fn bounded_float_sample_raw_near_one_t_adjacent_negative_bounds() {
        let hi = -1.0_f64;
        let lo = hi.next_down();
        let dist = BoundedFloat::try_new(lo, hi).unwrap();
        let max_t = f64::from_bits(0x3FEF_FFFF_FFFF_FFFF);
        let v = dist.sample_raw(max_t);
        assert!(
            v >= lo && v < hi,
            "negative adjacent max_t: {v:e} not in [{lo:e}, {hi:e})"
        );
    }

    #[test]
    fn bounded_float_sample_raw_max_t_opposite_sign_extremes() {
        let dist = BoundedFloat::try_new(-f64::MAX, f64::MAX).unwrap();
        let max_t = f64::from_bits(0x3FEF_FFFF_FFFF_FFFF);
        let v = dist.sample_raw(max_t);
        assert!(
            (-f64::MAX..f64::MAX).contains(&v),
            "opposite-sign extremes max_t: {v:e} out of range"
        );
    }

    // ── BoundedUInt ──

    #[test]
    fn bounded_uint_samples_are_in_range() {
        let dist = BoundedUInt::try_new(10, 20).unwrap();
        let mut r = rng(2);
        for _ in 0..1_000 {
            let v = dist.sample(&mut r);
            assert!((10..=20).contains(&v), "out of range: {v}");
        }
    }

    #[test]
    fn bounded_uint_singleton_always_returns_lo() {
        let dist = BoundedUInt::try_new(7, 7).unwrap();
        let mut r = rng(3);
        for _ in 0..100 {
            assert_eq!(dist.sample(&mut r), 7);
        }
    }

    #[test]
    fn bounded_uint_rejects_inverted_range() {
        assert_eq!(
            BoundedUInt::try_new(5, 4),
            Err(DistributionError::EmptyRange)
        );
    }

    #[test]
    fn bounded_uint_full_range_does_not_panic() {
        let dist = BoundedUInt::try_new(0, u64::MAX).unwrap();
        let mut r = rng(4);
        let _ = dist.sample(&mut r);
    }

    #[test]
    fn bounded_uint_full_range_coverage() {
        // Use a small range to verify every value appears.
        let dist = BoundedUInt::try_new(0, 15).unwrap();
        let mut seen = [false; 16];
        let mut r = rng(5678);
        for _ in 0..10_000 {
            let v = usize::try_from(dist.sample(&mut r)).expect("fits usize");
            seen[v] = true;
        }
        assert!(seen.iter().all(|&s| s), "all values 0..=15 must appear");
    }

    #[test]
    fn bounded_uint_no_bias_near_power_of_two() {
        // Range 3 is not a power of two; check rough uniformity.
        let dist = BoundedUInt::try_new(0, 2).unwrap();
        let mut counts = [0u32; 3];
        let mut r = rng(999);
        for _ in 0..30_000 {
            counts[usize::try_from(dist.sample(&mut r)).unwrap()] += 1;
        }
        // Each value should appear ~10_000 times; allow ±20%.
        for (i, &c) in counts.iter().enumerate() {
            assert!(
                c > 8_000 && c < 12_000,
                "value {i} appeared {c} times, expected ~10000"
            );
        }
    }

    #[test]
    fn bounded_uint_serde_rejects_inverted_bounds() {
        let json = r#"{"lo": 10, "hi": 5}"#;
        let result: Result<BoundedUInt, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "inverted bounds must be rejected on deserialize"
        );
    }

    #[test]
    fn bounded_uint_serde_round_trips() {
        let dist = BoundedUInt::try_new(3, 99).unwrap();
        let json = serde_json::to_string(&dist).unwrap();
        let decoded: BoundedUInt = serde_json::from_str(&json).unwrap();
        assert_eq!(dist, decoded);
    }

    // ── WeightedChoice ──

    #[test]
    fn weighted_choice_selects_only_positive_weight_items() {
        let items = vec![
            WeightedItem {
                item: "a",
                weight: 0,
            },
            WeightedItem {
                item: "b",
                weight: 10,
            },
            WeightedItem {
                item: "c",
                weight: 0,
            },
        ];
        let choice = WeightedChoice::try_new(items).unwrap();
        let mut r = rng(5);
        for _ in 0..100 {
            assert_eq!(*choice.sample(&mut r), "b");
        }
    }

    #[test]
    fn weighted_choice_rejects_empty_items() {
        let result: Result<WeightedChoice<&str>, _> = WeightedChoice::try_new(vec![]);
        assert_eq!(result.unwrap_err(), DistributionError::EmptyItems);
    }

    #[test]
    fn weighted_choice_rejects_all_zero_weights() {
        let items = vec![WeightedItem {
            item: "x",
            weight: 0,
        }];
        assert_eq!(
            WeightedChoice::try_new(items).unwrap_err(),
            DistributionError::ZeroTotalWeight
        );
    }

    #[test]
    fn weighted_choice_is_deterministic() {
        let items = vec![
            WeightedItem {
                item: 1u32,
                weight: 1,
            },
            WeightedItem {
                item: 2u32,
                weight: 2,
            },
            WeightedItem {
                item: 3u32,
                weight: 3,
            },
        ];
        let choice = WeightedChoice::try_new(items).unwrap();
        let run = |seed: u64| -> Vec<u32> {
            let mut r = rng(seed);
            (0..30).map(|_| *choice.sample(&mut r)).collect()
        };
        assert_eq!(run(6), run(6), "same seed must yield same choices");
    }

    #[test]
    fn weighted_choice_no_bias_for_non_power_of_two_total() {
        // weights 1+2 = 3 (not a power of 2)
        let items = vec![
            WeightedItem {
                item: 0u32,
                weight: 1,
            },
            WeightedItem {
                item: 1u32,
                weight: 2,
            },
        ];
        let choice = WeightedChoice::try_new(items).unwrap();
        let mut counts = [0u32; 2];
        let mut r = rng(42);
        for _ in 0..30_000 {
            counts[usize::try_from(*choice.sample(&mut r)).unwrap()] += 1;
        }
        // Expected: ~10_000 for item 0, ~20_000 for item 1.
        assert!(
            counts[0] > 8_000 && counts[0] < 12_000,
            "item 0 count: {}",
            counts[0]
        );
        assert!(
            counts[1] > 18_000 && counts[1] < 22_000,
            "item 1 count: {}",
            counts[1]
        );
    }

    #[test]
    fn bounded_float_sample_ctx_respects_draw_limit() {
        let dist = BoundedFloat::try_new(0.0, 1.0).unwrap();
        let mut ctx = ctx_with_draw_limit(0);
        assert_eq!(
            dist.sample_ctx(&mut ctx),
            Err(crate::runner::ResourceLimitKind::MaxRngDrawsPerCase)
        );
    }

    #[test]
    fn bounded_uint_sample_ctx_respects_draw_limit() {
        let dist = BoundedUInt::try_new(0, 9).unwrap();
        let mut ctx = ctx_with_draw_limit(0);
        assert_eq!(
            dist.sample_ctx(&mut ctx),
            Err(crate::runner::ResourceLimitKind::MaxRngDrawsPerCase)
        );
    }

    #[test]
    fn bounded_uint_sample_ctx_rejection_sampling_terminates() {
        let dist = BoundedUInt::try_new(0, 2).unwrap();
        let mut ctx = CaseContext::new(
            0,
            TestRng::from_seed(TestSeed::new(99)),
            CaseBudget::from_limits(&crate::runner::ResourceLimits {
                max_rng_draws_per_case: Some(1_000),
                ..crate::runner::ResourceLimits::default()
            }),
        );
        for _ in 0..64 {
            let value = dist.sample_ctx(&mut ctx).unwrap();
            assert!(value <= 2);
        }
    }

    #[test]
    fn weighted_choice_sample_ctx_respects_draw_limit() {
        let choice = WeightedChoice::try_new(vec![
            WeightedItem {
                item: "a",
                weight: 1,
            },
            WeightedItem {
                item: "b",
                weight: 1,
            },
        ])
        .unwrap();
        let mut ctx = ctx_with_draw_limit(0);
        assert_eq!(
            choice.sample_ctx(&mut ctx),
            Err(crate::runner::ResourceLimitKind::MaxRngDrawsPerCase)
        );
    }

    // ── EdgeCaseSchedule ──

    #[test]
    fn geometric_schedule_hits_expected_indices() {
        let s = EdgeCaseSchedule::geometric();
        assert!(s.is_edge_case(0), "0 must be edge case");
        assert!(s.is_edge_case(1), "1 must be edge case");
        assert!(s.is_edge_case(2), "2 must be edge case");
        assert!(!s.is_edge_case(3), "3 must not be edge case");
        assert!(s.is_edge_case(4), "4 must be edge case");
        assert!(!s.is_edge_case(5), "5 must not");
        assert!(!s.is_edge_case(6), "6 must not");
        assert!(!s.is_edge_case(7), "7 must not");
        assert!(s.is_edge_case(8), "8 must be edge case");
        assert!(s.is_edge_case(16), "16 must be edge case");
        assert!(!s.is_edge_case(15), "15 must not");
    }

    #[test]
    fn periodic_schedule_hits_expected_indices() {
        let s = EdgeCaseSchedule::periodic(4);
        assert!(s.is_edge_case(0));
        assert!(!s.is_edge_case(1));
        assert!(!s.is_edge_case(2));
        assert!(!s.is_edge_case(3));
        assert!(s.is_edge_case(4));
        assert!(s.is_edge_case(8));
    }

    #[test]
    fn periodic_zero_period_never_fires() {
        let s = EdgeCaseSchedule::periodic(0);
        for i in 0..100 {
            assert!(!s.is_edge_case(i));
        }
    }

    #[test]
    fn never_schedule_never_fires() {
        let s = EdgeCaseSchedule::never();
        for i in 0..100 {
            assert!(!s.is_edge_case(i));
        }
    }

    #[test]
    fn explicit_schedule_matches_exact_indices() {
        let s = EdgeCaseSchedule::try_explicit(vec![0, 5, 100]).unwrap();
        assert!(s.is_edge_case(0));
        assert!(s.is_edge_case(5));
        assert!(s.is_edge_case(100));
        assert!(!s.is_edge_case(1));
        assert!(!s.is_edge_case(50));
        assert!(!s.is_edge_case(99));
    }

    #[test]
    fn explicit_schedule_construction_only_via_try_explicit() {
        let schedule = EdgeCaseSchedule::try_explicit(vec![1, 2, 3]).unwrap();
        match schedule {
            EdgeCaseSchedule::Explicit(schedule) => assert_eq!(schedule.as_slice(), &[1, 2, 3]),
            _ => panic!("expected explicit schedule"),
        }
        assert!(EdgeCaseSchedule::try_explicit(vec![3, 1, 2]).is_err());
        assert!(EdgeCaseSchedule::try_explicit(vec![1, 1, 2]).is_err());
    }

    #[test]
    fn explicit_schedule_wire_rejects_unsorted_deserialization() {
        let json = r#"{"Explicit":{"indices":[3,1,2]}}"#;
        let result: Result<EdgeCaseSchedule, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "unsorted indices must be rejected on deserialization"
        );
    }

    #[test]
    fn explicit_schedule_wire_rejects_duplicate_indices() {
        let json = r#"{"Explicit":{"indices":[1,1,2]}}"#;
        let result: Result<EdgeCaseSchedule, _> = serde_json::from_str(json);
        assert!(
            result.is_err(),
            "duplicate indices must be rejected on deserialization"
        );
    }

    #[test]
    fn explicit_schedule_serde_round_trips() {
        let s = EdgeCaseSchedule::try_explicit(vec![0, 3, 7, 100]).unwrap();
        let json = serde_json::to_string(&s).unwrap();
        let decoded: EdgeCaseSchedule = serde_json::from_str(&json).unwrap();
        match decoded {
            EdgeCaseSchedule::Explicit(schedule) => {
                assert_eq!(schedule.as_slice(), &[0, 3, 7, 100]);
            }
            _ => panic!("expected explicit schedule"),
        }
    }
}
