//! Conservative interval arithmetic with outward-rounded bounds.
//!
//! [`Interval`] represents a connected subset `[lo, hi]` of the real line.
//! All arithmetic operations widen bounds outward by one ULP so that the
//! true mathematical result always lies within the computed interval. This
//! requires `f64::next_down` and `f64::next_up`, stabilized in Rust 1.87.
//!
//! Construction rejects non-finite endpoints and intervals where `lo > hi`.
//! Division by an interval that contains zero is rejected. All binary
//! arithmetic operations return `Result` so that overflow to ±infinity is
//! detected and reported rather than silently producing an invalid interval.

use core::error::Error;
use core::fmt;
use core::ops;

use serde::{Deserialize, Serialize};

/// Named serde representation for [`Interval`].
#[derive(Clone, Copy, Serialize, Deserialize)]
struct IntervalRepr {
    lo: f64,
    hi: f64,
}

/// Error from invalid interval construction or operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntervalError {
    /// An endpoint is NaN or infinite.
    NonFinite,
    /// The lower bound is greater than the upper bound.
    InvertedBounds,
    /// Division by an interval containing zero is undefined.
    DivisionByZeroInterval,
    /// A negative widen amount was supplied; widening must expand the interval.
    NegativeWiden,
    /// The arithmetic result overflowed to ±infinity.
    Overflow,
}

impl fmt::Display for IntervalError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NonFinite => "interval endpoints must be finite",
            Self::InvertedBounds => "interval lower bound must not exceed upper bound",
            Self::DivisionByZeroInterval => {
                "division by an interval containing zero is not defined"
            }
            Self::NegativeWiden => "widen amount must be non-negative",
            Self::Overflow => "interval arithmetic result overflowed to ±infinity",
        };
        formatter.write_str(message)
    }
}

impl Error for IntervalError {}

/// A closed real interval `[lo, hi]` with conservative arithmetic.
///
/// All binary operations return `Result` and produce outward-rounded results:
/// the true mathematical value is guaranteed to fall within the returned
/// interval. Operations that would overflow a bound to ±infinity return
/// [`IntervalError::Overflow`] instead of silently producing an invalid value.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "IntervalRepr", into = "IntervalRepr")]
pub struct Interval {
    lo: f64,
    hi: f64,
}

impl Interval {
    /// Constructs `[lo, hi]`.
    ///
    /// # Errors
    ///
    /// Returns [`IntervalError`] when either endpoint is non-finite or
    /// `lo > hi`.
    pub fn try_new(lo: f64, hi: f64) -> Result<Self, IntervalError> {
        if !lo.is_finite() || !hi.is_finite() {
            return Err(IntervalError::NonFinite);
        }
        if lo > hi {
            return Err(IntervalError::InvertedBounds);
        }
        Ok(Self { lo, hi })
    }

    /// Constructs a degenerate interval `[v, v]` (a single point).
    ///
    /// # Errors
    ///
    /// Returns [`IntervalError`] when `v` is non-finite.
    pub fn point(v: f64) -> Result<Self, IntervalError> {
        Self::try_new(v, v)
    }

    /// Returns the lower bound.
    #[must_use]
    pub const fn lo(self) -> f64 {
        self.lo
    }

    /// Returns the upper bound.
    #[must_use]
    pub const fn hi(self) -> f64 {
        self.hi
    }

    /// Returns `hi - lo`, or [`IntervalError::Overflow`] when the difference
    /// overflows (for example for `[-f64::MAX, f64::MAX]`).
    ///
    /// # Errors
    ///
    /// Returns [`IntervalError::Overflow`] when the interval width is not
    /// representable as a finite `f64`.
    pub fn try_width(self) -> Result<f64, IntervalError> {
        let width = self.hi - self.lo;
        if !width.is_finite() {
            return Err(IntervalError::Overflow);
        }
        Ok(width)
    }

    /// Returns the midpoint of the interval.
    ///
    /// Uses a sign-aware formula that avoids both underflow (for same-sign
    /// subnormal intervals like `[ε, ε]`) and overflow (for same-sign MAX
    /// intervals):
    ///
    /// - `lo == hi`: returns `lo` exactly.
    /// - opposite signs (`lo < 0 < hi`): `lo/2 + hi/2` (safe, no underflow).
    /// - same sign: `lo + (hi - lo)/2` (avoids overflow, `hi-lo` is finite).
    ///
    /// The result always satisfies `lo ≤ midpoint ≤ hi`.
    #[must_use]
    #[allow(clippy::float_cmp)]
    pub fn midpoint(self) -> f64 {
        if self.lo == self.hi {
            return self.lo;
        }
        if self.lo < 0.0 && self.hi > 0.0 {
            self.lo / 2.0 + self.hi / 2.0
        } else {
            self.lo + (self.hi - self.lo) / 2.0
        }
    }

    /// Tests whether `v` lies in `[lo, hi]`.
    ///
    /// # Errors
    ///
    /// Returns [`IntervalError::NonFinite`] when `v` is NaN or infinite.
    /// Callers must not rely on NaN being silently classified as outside.
    pub fn contains(self, v: f64) -> Result<bool, IntervalError> {
        if !v.is_finite() {
            return Err(IntervalError::NonFinite);
        }
        Ok(v >= self.lo && v <= self.hi)
    }

    /// Returns `true` when this interval overlaps `other`.
    #[must_use]
    pub fn intersects(self, other: Self) -> bool {
        self.lo <= other.hi && other.lo <= self.hi
    }

    /// Returns the smallest interval that contains both `self` and `other`.
    #[must_use]
    pub fn hull(self, other: Self) -> Self {
        Self {
            lo: self.lo.min(other.lo),
            hi: self.hi.max(other.hi),
        }
    }

    /// Returns an interval that is widened outward by `amount` on each side.
    ///
    /// # Errors
    ///
    /// Returns [`IntervalError::NegativeWiden`] when `amount < 0.0`.
    /// Returns [`IntervalError::NonFinite`] when `amount` is NaN or infinite.
    /// Returns [`IntervalError::Overflow`] when widening would push a bound
    /// to ±infinity.
    pub fn widen(self, amount: f64) -> Result<Self, IntervalError> {
        if amount.is_nan() || amount.is_infinite() {
            return Err(IntervalError::NonFinite);
        }
        if amount < 0.0 {
            return Err(IntervalError::NegativeWiden);
        }
        if amount == 0.0 {
            return Ok(self);
        }
        // Conservative widening: if the rounded bound reaches ±infinity, we return
        // Err(Overflow) even though the exact bound may be f64::MAX. This is a known
        // deliberate limitation: interval arithmetic with bounds at f64::MAX cannot
        // be conservatively widened and remain representable.
        let lo = (self.lo - amount).next_down();
        let hi = (self.hi + amount).next_up();
        if !lo.is_finite() || !hi.is_finite() {
            return Err(IntervalError::Overflow);
        }
        Ok(Self { lo, hi })
    }

    /// Adds `rhs` with outward-rounded bounds.
    ///
    /// # Errors
    ///
    /// Returns [`IntervalError::Overflow`] when any bound overflows to
    /// ±infinity after rounding.
    pub fn try_add(self, rhs: Self) -> Result<Self, IntervalError> {
        let lo = (self.lo + rhs.lo).next_down();
        let hi = (self.hi + rhs.hi).next_up();
        if !lo.is_finite() || !hi.is_finite() {
            return Err(IntervalError::Overflow);
        }
        Ok(Self { lo, hi })
    }

    /// Subtracts `rhs` with outward-rounded bounds.
    ///
    /// # Errors
    ///
    /// Returns [`IntervalError::Overflow`] when any bound overflows.
    pub fn try_sub(self, rhs: Self) -> Result<Self, IntervalError> {
        let lo = (self.lo - rhs.hi).next_down();
        let hi = (self.hi - rhs.lo).next_up();
        if !lo.is_finite() || !hi.is_finite() {
            return Err(IntervalError::Overflow);
        }
        Ok(Self { lo, hi })
    }

    /// Multiplies with outward-rounded bounds.
    ///
    /// Computes all four corner products and takes their min/max.
    ///
    /// # Errors
    ///
    /// Returns [`IntervalError::Overflow`] when any bound overflows.
    pub fn try_mul(self, rhs: Self) -> Result<Self, IntervalError> {
        let candidates = [
            self.lo * rhs.lo,
            self.lo * rhs.hi,
            self.hi * rhs.lo,
            self.hi * rhs.hi,
        ];
        let lo = candidates
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min)
            .next_down();
        let hi = candidates
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
            .next_up();
        if !lo.is_finite() || !hi.is_finite() {
            return Err(IntervalError::Overflow);
        }
        Ok(Self { lo, hi })
    }

    /// Divides `self` by `rhs` with outward-rounded bounds.
    ///
    /// # Errors
    ///
    /// Returns [`IntervalError::DivisionByZeroInterval`] when `rhs` contains
    /// zero (i.e., `rhs.lo <= 0 <= rhs.hi`).
    /// Returns [`IntervalError::Overflow`] when a bound overflows.
    pub fn try_div(self, rhs: Self) -> Result<Self, IntervalError> {
        if rhs.lo <= 0.0 && rhs.hi >= 0.0 {
            return Err(IntervalError::DivisionByZeroInterval);
        }
        // rhs is strictly positive or strictly negative; both safe.
        let candidates = [
            self.lo / rhs.lo,
            self.lo / rhs.hi,
            self.hi / rhs.lo,
            self.hi / rhs.hi,
        ];
        let lo = candidates
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min)
            .next_down();
        let hi = candidates
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max)
            .next_up();
        if !lo.is_finite() || !hi.is_finite() {
            return Err(IntervalError::Overflow);
        }
        Ok(Self { lo, hi })
    }
}

impl TryFrom<[f64; 2]> for Interval {
    type Error = IntervalError;

    fn try_from(value: [f64; 2]) -> Result<Self, Self::Error> {
        Self::try_new(value[0], value[1])
    }
}

impl From<Interval> for [f64; 2] {
    fn from(value: Interval) -> Self {
        [value.lo, value.hi]
    }
}

impl TryFrom<IntervalRepr> for Interval {
    type Error = IntervalError;

    fn try_from(value: IntervalRepr) -> Result<Self, Self::Error> {
        Self::try_new(value.lo, value.hi)
    }
}

impl From<Interval> for IntervalRepr {
    fn from(value: Interval) -> Self {
        Self {
            lo: value.lo,
            hi: value.hi,
        }
    }
}

// ── Conservative arithmetic ───────────────────────────────────────────────────

impl ops::Neg for Interval {
    type Output = Self;

    /// Negation is exact: `[-hi, -lo]`. Negating a finite value is always
    /// finite, so this is the only infallible arithmetic operation.
    fn neg(self) -> Self {
        Self {
            lo: -self.hi,
            hi: -self.lo,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Interval, IntervalError};

    fn iv(lo: f64, hi: f64) -> Interval {
        Interval::try_new(lo, hi).unwrap()
    }

    fn is_exact_max_magnitude(value: f64) -> bool {
        value.abs().to_bits() == f64::MAX.to_bits()
    }

    #[test]
    fn construction_validates_finiteness() {
        assert_eq!(
            Interval::try_new(f64::NAN, 1.0).unwrap_err(),
            IntervalError::NonFinite
        );
        assert_eq!(
            Interval::try_new(1.0, f64::INFINITY).unwrap_err(),
            IntervalError::NonFinite
        );
    }

    #[test]
    fn construction_validates_ordering() {
        assert_eq!(
            Interval::try_new(2.0, 1.0).unwrap_err(),
            IntervalError::InvertedBounds
        );
    }

    #[test]
    fn degenerate_interval_contains_only_point() {
        let p = Interval::point(3.5).unwrap();
        assert!(p.contains(3.5).unwrap());
        assert!(!p.contains(3.5_f64.next_up()).unwrap());
        assert!(!p.contains(3.5_f64.next_down()).unwrap());
    }

    #[test]
    fn contains_rejects_nan() {
        let i = iv(-1.0, 1.0);
        assert_eq!(i.contains(f64::NAN).unwrap_err(), IntervalError::NonFinite);
    }

    #[test]
    fn contains_rejects_infinity() {
        let i = iv(-1.0, 1.0);
        assert_eq!(
            i.contains(f64::INFINITY).unwrap_err(),
            IntervalError::NonFinite
        );
    }

    #[test]
    fn contains_is_inclusive() {
        let i = iv(-1.0, 1.0);
        assert!(i.contains(-1.0).unwrap());
        assert!(i.contains(0.0).unwrap());
        assert!(i.contains(1.0).unwrap());
        assert!(!i.contains(1.0_f64.next_up()).unwrap());
    }

    #[test]
    fn try_add_is_conservative() {
        let a = iv(1.0, 2.0);
        let b = iv(3.0, 4.0);
        let c = a.try_add(b).unwrap();
        assert!(c.lo() <= 4.0, "lo must not exceed the exact result");
        assert!(c.hi() >= 6.0, "hi must not be below the exact result");
    }

    #[test]
    fn try_sub_is_conservative() {
        let a = iv(3.0, 4.0);
        let b = iv(1.0, 2.0);
        let c = a.try_sub(b).unwrap();
        assert!(c.lo() <= 1.0);
        assert!(c.hi() >= 3.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn neg_is_exact() {
        let a = iv(-3.0, 2.0);
        let na = -a;
        assert_eq!(na.lo(), -2.0);
        assert_eq!(na.hi(), 3.0);
    }

    #[test]
    fn try_mul_mixed_sign_is_conservative() {
        let a = iv(-1.0, 2.0);
        let b = iv(-3.0, 4.0);
        let c = a.try_mul(b).unwrap();
        // Products: 3, -4, -6, 8 → exact [−6, 8].
        assert!(c.lo() <= -6.0);
        assert!(c.hi() >= 8.0);
    }

    #[test]
    fn div_by_zero_interval_is_rejected() {
        let a = iv(1.0, 2.0);
        let b = iv(-1.0, 1.0);
        assert_eq!(
            a.try_div(b).unwrap_err(),
            IntervalError::DivisionByZeroInterval
        );
    }

    #[test]
    fn div_positive_divisor_is_conservative() {
        let a = iv(2.0, 4.0);
        let b = iv(1.0, 2.0);
        let c = a.try_div(b).unwrap();
        assert!(c.lo() <= 1.0);
        assert!(c.hi() >= 4.0);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn hull_spans_both_intervals() {
        let a = iv(1.0, 3.0);
        let b = iv(2.0, 5.0);
        let h = a.hull(b);
        assert_eq!(h.lo(), 1.0);
        assert_eq!(h.hi(), 5.0);
    }

    #[test]
    fn intersects_works_correctly() {
        let a = iv(0.0, 2.0);
        let b = iv(1.0, 3.0);
        assert!(a.intersects(b));
        assert!(!a.intersects(iv(2.5, 4.0)));
    }

    #[test]
    fn serde_round_trip() {
        let i = iv(1.5, 3.7);
        let json = serde_json::to_string(&i).unwrap();
        let j: Interval = serde_json::from_str(&json).unwrap();
        assert_eq!(i, j);
    }

    #[test]
    fn serde_rejects_non_finite() {
        let bad: Result<Interval, _> = serde_json::from_str(r#"{"lo":1.0,"hi":1e400}"#);
        assert!(bad.is_err(), "infinite endpoint must be rejected");
    }

    #[test]
    fn serde_rejects_inverted_bounds() {
        let bad: Result<Interval, _> = serde_json::from_str(r#"{"lo":3.0,"hi":1.0}"#);
        assert!(bad.is_err(), "inverted bounds must be rejected");
    }

    #[test]
    fn serde_json_shape_is_named_fields() {
        let interval = Interval::try_new(1.0, 2.0).unwrap();
        let json = serde_json::to_string(&interval).unwrap();
        assert!(
            json.contains("\"lo\"") && json.contains("\"hi\""),
            "JSON must use named fields lo/hi, got: {json}"
        );
    }

    #[test]
    fn arithmetic_is_deterministic() {
        let a = iv(1.0, 2.0);
        let b = iv(3.0, 4.0);
        let r1 = a.try_add(b).unwrap();
        let r2 = a.try_add(b).unwrap();
        assert_eq!(r1, r2);
    }

    #[test]
    fn widen_expands_symmetrically() {
        let i = iv(1.0, 3.0);
        let w = i.widen(0.5).unwrap();
        assert!(w.lo() <= 0.5);
        assert!(w.hi() >= 3.5);
    }

    #[test]
    fn widen_negative_amount_returns_negative_widen_error() {
        let i = iv(1.0, 3.0);
        assert_eq!(i.widen(-0.1).unwrap_err(), IntervalError::NegativeWiden);
    }

    #[test]
    fn midpoint_is_center() {
        let i = iv(1.0, 3.0);
        assert!((i.midpoint() - 2.0).abs() < f64::EPSILON);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn midpoint_degenerate_subnormal_stays_in_interval() {
        let epsilon = f64::from_bits(1);
        let interval = Interval::try_new(epsilon, epsilon).unwrap();
        let midpoint = interval.midpoint();
        assert_eq!(midpoint, epsilon, "midpoint of [ε,ε] must be ε, not 0");
        assert!(
            interval.contains(midpoint).unwrap(),
            "midpoint must be contained in interval"
        );
    }

    #[test]
    fn midpoint_opposite_sign_max() {
        let interval = Interval::try_new(-f64::MAX, f64::MAX).unwrap();
        let midpoint = interval.midpoint();
        assert!(
            interval.contains(midpoint).unwrap(),
            "midpoint of [-MAX,MAX] must be contained"
        );
        assert!(midpoint.abs() < f64::MAX / 2.0);
    }

    #[test]
    fn midpoint_same_sign_max() {
        let interval = Interval::try_new(f64::MAX / 2.0, f64::MAX).unwrap();
        let midpoint = interval.midpoint();
        assert!(
            interval.contains(midpoint).unwrap(),
            "midpoint must be in interval"
        );
        assert!(midpoint.is_finite(), "midpoint must be finite");
    }

    #[test]
    fn midpoint_same_sign_subnormal() {
        let lo = f64::from_bits(1);
        let hi = f64::from_bits(3);
        let interval = Interval::try_new(lo, hi).unwrap();
        let midpoint = interval.midpoint();
        assert!(
            interval.contains(midpoint).unwrap(),
            "midpoint must be in [lo, hi]"
        );
    }

    #[test]
    fn midpoint_property_always_contained() {
        let cases = [
            (-1.0, 1.0),
            (0.0, 0.0),
            (1.0, 1.0),
            (f64::MIN_POSITIVE, f64::MIN_POSITIVE),
            (f64::from_bits(1), f64::from_bits(2)),
            (-f64::MAX, f64::MAX),
            (f64::MAX / 2.0, f64::MAX),
            (-f64::MAX, -f64::MAX / 2.0),
            (0.0, f64::MAX),
            (-f64::MAX, 0.0),
        ];
        for (lo, hi) in cases {
            if let Ok(interval) = Interval::try_new(lo, hi) {
                let midpoint = interval.midpoint();
                assert!(
                    interval.contains(midpoint).unwrap_or(false),
                    "midpoint {midpoint} not in [{lo}, {hi}]"
                );
            }
        }
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn try_width_normal() {
        assert_eq!(iv(1.0, 3.0).try_width().unwrap(), 2.0);
    }

    #[test]
    fn try_width_overflow() {
        assert_eq!(
            iv(-f64::MAX, f64::MAX).try_width(),
            Err(IntervalError::Overflow)
        );
    }

    #[test]
    fn try_add_contains_sum_of_midpoints() {
        let a = iv(1.0, 2.0);
        let b = iv(3.0, 4.0);
        let c = a.try_add(b).unwrap();
        let expected_mid = a.midpoint() + b.midpoint();
        assert!(c.contains(expected_mid).unwrap());
    }

    // ── Overflow tests ────────────────────────────────────────────────────────

    #[test]
    fn try_add_overflow_is_rejected() {
        let huge = iv(f64::MAX / 2.0, f64::MAX);
        assert_eq!(huge.try_add(huge).unwrap_err(), IntervalError::Overflow);
    }

    #[test]
    fn try_add_at_max_boundary_returns_overflow() {
        // [MAX/2, MAX/2] + [MAX/2, MAX/2] = [MAX, MAX] exactly, but next_up(MAX) = Inf.
        // Conservative widening cannot represent this; Overflow is correct behavior.
        let half = iv(f64::MAX / 2.0, f64::MAX / 2.0);
        assert_eq!(half.try_add(half).unwrap_err(), IntervalError::Overflow);
    }

    #[test]
    fn try_sub_overflow_is_rejected() {
        let pos = iv(f64::MAX / 2.0, f64::MAX);
        let neg = iv(-f64::MAX, -f64::MAX / 2.0);
        assert_eq!(pos.try_sub(neg).unwrap_err(), IntervalError::Overflow);
    }

    #[test]
    fn try_mul_overflow_is_rejected() {
        let huge = iv(f64::MAX / 2.0, f64::MAX);
        assert_eq!(huge.try_mul(huge).unwrap_err(), IntervalError::Overflow);
    }

    // ── Property: sampled exact results stay enclosed ─────────────────────────

    /// Verifies the containment property for a grid of (a, b) pairs.
    ///
    /// For each exact pair of scalars, the scalar result must be contained
    /// within the interval produced by the corresponding interval operation.
    #[test]
    fn property_add_encloses_exact_result() {
        let values = [
            -f64::MAX,
            -0.0,
            0.0,
            f64::from_bits(1),
            f64::MIN_POSITIVE,
            0.1,
            0.3,
            1.0,
            2.5,
            10.0,
            100.0,
            f64::MAX,
        ];
        for &x in &values {
            for &y in &values {
                let exact = x + y;
                let a = Interval::point(x).unwrap();
                let b = Interval::point(y).unwrap();
                match a.try_add(b) {
                    Ok(result) => assert!(
                        result.contains(exact).unwrap(),
                        "add: [{x}] + [{y}] ⊅ {result:?}"
                    ),
                    Err(IntervalError::Overflow) => assert!(
                        !exact.is_finite() || is_exact_max_magnitude(exact),
                        "finite exact sum unexpectedly overflowed"
                    ),
                    Err(other) => panic!("unexpected add error: {other:?}"),
                }
            }
        }
    }

    #[test]
    fn property_sub_encloses_exact_result() {
        let values = [
            -f64::MAX,
            -0.0,
            0.0,
            f64::from_bits(1),
            f64::MIN_POSITIVE,
            0.1,
            0.3,
            1.0,
            2.5,
            10.0,
            100.0,
            f64::MAX,
        ];
        for &x in &values {
            for &y in &values {
                let exact = x - y;
                let a = Interval::point(x).unwrap();
                let b = Interval::point(y).unwrap();
                match a.try_sub(b) {
                    Ok(result) => assert!(
                        result.contains(exact).unwrap(),
                        "sub: [{x}] - [{y}] ⊅ {result:?}"
                    ),
                    Err(IntervalError::Overflow) => assert!(
                        !exact.is_finite() || is_exact_max_magnitude(exact),
                        "finite exact difference unexpectedly overflowed"
                    ),
                    Err(other) => panic!("unexpected sub error: {other:?}"),
                }
            }
        }
    }

    #[test]
    fn property_mul_encloses_exact_result() {
        let values = [
            -f64::MAX,
            -1.0,
            -0.1,
            -0.0,
            0.0,
            f64::from_bits(1),
            f64::MIN_POSITIVE,
            0.1,
            1.0,
            10.0,
            f64::MAX,
        ];
        for &x in &values {
            for &y in &values {
                let exact = x * y;
                let a = Interval::point(x).unwrap();
                let b = Interval::point(y).unwrap();
                match a.try_mul(b) {
                    Ok(result) => assert!(
                        result.contains(exact).unwrap(),
                        "mul: [{x}] * [{y}] ⊅ {result:?}"
                    ),
                    Err(IntervalError::Overflow) => assert!(
                        !exact.is_finite() || is_exact_max_magnitude(exact),
                        "finite exact product unexpectedly overflowed"
                    ),
                    Err(other) => panic!("unexpected mul error: {other:?}"),
                }
            }
        }
    }

    #[test]
    fn property_div_encloses_exact_result() {
        let num_values = [
            -f64::MAX,
            -1.0,
            -0.1,
            -0.0,
            0.0,
            f64::from_bits(1),
            f64::MIN_POSITIVE,
            0.1,
            1.0,
            10.0,
            f64::MAX,
        ];
        let den_values = [
            -f64::MAX,
            -1.0,
            -0.1,
            f64::from_bits(1),
            f64::MIN_POSITIVE,
            0.1,
            1.0,
            10.0,
            f64::MAX,
        ];
        for &x in &num_values {
            for &y in &den_values {
                let a = Interval::point(x).unwrap();
                let b = Interval::point(y).unwrap();
                let exact = x / y;
                match a.try_div(b) {
                    Ok(result) => assert!(
                        result.contains(exact).unwrap(),
                        "div: [{x}] / [{y}] ⊅ {result:?}"
                    ),
                    Err(IntervalError::Overflow) => assert!(
                        !exact.is_finite() || is_exact_max_magnitude(exact),
                        "finite exact quotient unexpectedly overflowed"
                    ),
                    Err(other) => panic!("unexpected div error: {other:?}"),
                }
            }
        }
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn widen_zero_does_not_mutate() {
        let interval = iv(3.5, 7.0);
        let widened = interval.widen(0.0).unwrap();
        assert_eq!(widened.lo(), interval.lo());
        assert_eq!(widened.hi(), interval.hi());
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn point_accepts_negative_zero() {
        let interval = Interval::point(-0.0).unwrap();
        assert_eq!(interval.lo(), -0.0);
        assert_eq!(interval.hi(), -0.0);
    }

    #[test]
    fn try_add_with_signed_zero_inputs() {
        let pos_zero = Interval::point(0.0).unwrap();
        let neg_zero = Interval::point(-0.0).unwrap();
        assert!(pos_zero.try_add(neg_zero).is_ok());
    }
}
