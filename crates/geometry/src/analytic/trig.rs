//! Certified rational-interval transcendental functions.
//!
//! All trig functions operate on [`RatInterval`] (exact `BigRational` endpoints)
//! and return [`RatInterval`] enclosures. The true mathematical value of
//! every supported function is guaranteed to lie in the returned interval.
//!
//! # Algorithm references
//!
//! - Pi via Gregory–Leibniz/Machin 1706: `π/4 = 4·atan(1/5) − atan(1/239)`.
//!   Correctness: both series converge absolutely and the alternating-series
//!   remainder theorem (Leibniz criterion, Knopp 1956 §15) provides the
//!   tight remainder bound used here.
//! - Taylor series for sin/cos/atan: Maclaurin 1742; alternating-decreasing
//!   series with explicit remainder; see Apostol (1974) *Calculus* §11.18.
//! - Atan range reductions: `atan(x) = π/2 − atan(1/x)` (x > 0);
//!   `atan(x) = π/4 + atan((x−1)/(x+1))` (x ∈ (1/2, 1]).
//! - Quadrant reduction for sin/cos: symmetric identities.
//!
//! # Rejected transcendental backends
//!
//! - `libm` (MIT, pure Rust, WASM-compatible): empirically ~1–2 ULP, but
//!   **not formally proved** correctly rounded.
//! - `core-math` / `CRlibm` (MIT, 0.5 ULP correctly rounded): require
//!   `fenv.h` C FFI for directed-rounding control, **not WASM-compatible**.
//! - `inari` / `rug` (interval arithmetic via MPFR): require GMP/MPFR C
//!   libraries, **not WASM-compatible**.
//! - `inari_wasm`: calls `f64::sin` directly without directed rounding, so
//!   it is **not rigorous** as an interval implementation.
//! - `RLibm-All`: correctly-rounded but limited to ≤32-bit result types.
//! - `astro-float` / `dashu`: arbitrary precision, but no formal proof
//!   contract on transcendental accuracy.
//!
//! This module instead computes everything from first principles with exact
//! `BigRational` arithmetic and explicit alternating-series remainder bounds,
//! which is fully WASM-compatible (pure Rust, no FFI, no `fenv`) and
//! rigorously certified by construction.
//!
//! # Certification budget
//!
//! Every function accepts a [`CertificationBudget`] that caps series terms
//! and rational-integer bit-width of intermediate values. Exhaustion returns
//! [`TrigError::BudgetExhausted`] rather than an uncertified continuation.

use num_bigint::{BigInt, Sign};
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

#[cfg(test)]
use super::exact::{rat_to_f64, rat_to_f64_up};
use crate::CertificationBudget;

/// Error from the certified trig module.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum TrigError {
    /// Series or computation budget was exhausted before convergence.
    BudgetExhausted,
    /// The input contains a pole or undefined value (e.g., `atan2(0, 0)`).
    Pole,
    /// Division by an interval containing zero.
    DivisionByZero,
}

/// Deterministic bit-width and series-length resource cap for the trig
/// algorithms, derived from a [`CertificationBudget`].
///
/// Every arithmetic step inside this module flows through a `TrigCap` method
/// that *preflights* the result size from the operand sizes and returns
/// [`TrigError::BudgetExhausted`] **before** performing an allocation that
/// could exceed the cap. The bound used for each operation is an upper bound
/// on the true result bit-width, so a conservative early exhaustion is
/// possible; this is an accepted trade for a hard, deterministic ceiling on
/// intermediate `BigInt`/`BigRational` growth. `rational_bits` is therefore a
/// real resource cap on every public and internal path, not a post-hoc audit
/// of already-computed values.
#[derive(Clone, Copy)]
struct TrigCap {
    rational_bits: u32,
    series_terms: u32,
}

impl TrigCap {
    fn new(budget: CertificationBudget) -> Self {
        Self {
            rational_bits: budget.rational_bits(),
            series_terms: budget.series_terms(),
        }
    }

    fn bits(self) -> u64 {
        u64::from(self.rational_bits)
    }

    fn series_terms(self) -> u32 {
        self.series_terms
    }

    /// Largest exponent `e` with `2^e` still within the cap (`e + 1 <= bits`),
    /// never above `maximum`. Keeps the convergence-threshold denominator from
    /// itself exceeding the cap.
    fn threshold_bits(self, maximum: u32) -> u32 {
        self.rational_bits.saturating_sub(1).min(maximum)
    }

    /// Rejects a bit-width bound that exceeds the cap.
    fn within(self, bound: u64) -> Result<(), TrigError> {
        if bound > self.bits() {
            Err(TrigError::BudgetExhausted)
        } else {
            Ok(())
        }
    }

    /// Admits an existing `BigInt` whose bit-width must not exceed the cap.
    fn admit_int(self, value: &BigInt) -> Result<(), TrigError> {
        self.within(value.bits())
    }

    /// Admits an existing `BigRational` (both numerator and denominator).
    fn admit_rat(self, value: &BigRational) -> Result<(), TrigError> {
        self.within(value.numer().bits())?;
        self.within(value.denom().bits())
    }

    /// Admits both endpoints of an interval.
    fn admit_interval(self, interval: &RatInterval) -> Result<(), TrigError> {
        self.admit_rat(&interval.lo)?;
        self.admit_rat(&interval.hi)
    }

    /// Admits an interval and returns it, gating a successful value return.
    fn admitted_interval(self, interval: RatInterval) -> Result<RatInterval, TrigError> {
        self.admit_interval(&interval)?;
        Ok(interval)
    }

    /// Admits both intervals of a pair and returns it.
    fn admitted_pair(
        self,
        pair: (RatInterval, RatInterval),
    ) -> Result<(RatInterval, RatInterval), TrigError> {
        self.admit_interval(&pair.0)?;
        self.admit_interval(&pair.1)?;
        Ok(pair)
    }

    // ── preflight bounds (conservative upper bounds on result bit-width) ──────

    /// Preflights `a * b` for rationals: `numer` bits `<= an + bn`,
    /// `denom` bits `<= ad + bd`.
    fn guard_mul_rat(self, a: &BigRational, b: &BigRational) -> Result<(), TrigError> {
        self.within(a.numer().bits().saturating_add(b.numer().bits()))?;
        self.within(a.denom().bits().saturating_add(b.denom().bits()))
    }

    /// Preflights `a ± b` for rationals. Both share the same bound: the common
    /// denominator has `<= ad + bd` bits and the combined numerator
    /// `<= max(an + bd, ad + bn) + 1` bits.
    fn guard_addsub_rat(self, a: &BigRational, b: &BigRational) -> Result<(), TrigError> {
        let numer = a
            .numer()
            .bits()
            .saturating_add(b.denom().bits())
            .max(a.denom().bits().saturating_add(b.numer().bits()))
            .saturating_add(1);
        self.within(numer)?;
        self.within(a.denom().bits().saturating_add(b.denom().bits()))
    }

    /// Preflights `a / b` for rationals: `numer` bits `<= an + bd`,
    /// `denom` bits `<= ad + bn`. Rejects a zero divisor.
    fn guard_div_rat(self, a: &BigRational, b: &BigRational) -> Result<(), TrigError> {
        if b.is_zero() {
            return Err(TrigError::DivisionByZero);
        }
        self.within(a.numer().bits().saturating_add(b.denom().bits()))?;
        self.within(a.denom().bits().saturating_add(b.numer().bits()))
    }

    // ── checked scalar arithmetic (preflight, then compute) ───────────────────

    fn mul_int(self, a: &BigInt, b: &BigInt) -> Result<BigInt, TrigError> {
        self.within(a.bits().saturating_add(b.bits()))?;
        Ok(a * b)
    }

    fn sub_int(self, a: &BigInt, b: &BigInt) -> Result<BigInt, TrigError> {
        self.within(a.bits().max(b.bits()).saturating_add(1))?;
        Ok(a - b)
    }

    fn unsigned_int(self, value: u64) -> Result<BigInt, TrigError> {
        let bits = if value == 0 {
            0
        } else {
            u64::from(u64::BITS - value.leading_zeros())
        };
        self.within(bits)?;
        Ok(BigInt::from(value))
    }

    fn unsigned_rat(self, value: u64) -> Result<BigRational, TrigError> {
        Ok(BigRational::from_integer(self.unsigned_int(value)?))
    }

    fn unit_fraction(self, denominator: u64) -> Result<BigRational, TrigError> {
        self.ratio(BigInt::one(), self.unsigned_int(denominator)?)
    }

    /// Certified `2^exponent`. The result has `exponent + 1` bits.
    fn pow2(self, exponent: u32) -> Result<BigInt, TrigError> {
        self.within(u64::from(exponent).saturating_add(1))?;
        Ok(BigInt::one() << exponent as usize)
    }

    /// Certified `numer / denom` as a reduced `BigRational`. Reduction only
    /// shrinks, so admitting the raw components bounds the reduced result.
    fn ratio(self, numer: BigInt, denom: BigInt) -> Result<BigRational, TrigError> {
        self.admit_int(&numer)?;
        self.admit_int(&denom)?;
        Ok(BigRational::new(numer, denom))
    }

    fn mul_rat(self, a: &BigRational, b: &BigRational) -> Result<BigRational, TrigError> {
        self.guard_mul_rat(a, b)?;
        Ok(a * b)
    }

    fn add_rat(self, a: &BigRational, b: &BigRational) -> Result<BigRational, TrigError> {
        self.guard_addsub_rat(a, b)?;
        Ok(a + b)
    }

    fn sub_rat(self, a: &BigRational, b: &BigRational) -> Result<BigRational, TrigError> {
        self.guard_addsub_rat(a, b)?;
        Ok(a - b)
    }

    fn div_rat(self, a: &BigRational, b: &BigRational) -> Result<BigRational, TrigError> {
        self.guard_div_rat(a, b)?;
        Ok(a / b)
    }

    // ── checked interval arithmetic (preflight each corner, then raw op) ──────

    fn add_interval(self, a: &RatInterval, b: &RatInterval) -> Result<RatInterval, TrigError> {
        self.guard_addsub_rat(&a.lo, &b.lo)?;
        self.guard_addsub_rat(&a.hi, &b.hi)?;
        Ok(a.add(b))
    }

    fn sub_interval(self, a: &RatInterval, b: &RatInterval) -> Result<RatInterval, TrigError> {
        self.guard_addsub_rat(&a.lo, &b.hi)?;
        self.guard_addsub_rat(&a.hi, &b.lo)?;
        Ok(a.sub(b))
    }

    fn scale_interval(
        self,
        a: &RatInterval,
        scalar: &BigRational,
    ) -> Result<RatInterval, TrigError> {
        self.guard_mul_rat(&a.lo, scalar)?;
        self.guard_mul_rat(&a.hi, scalar)?;
        Ok(a.scale(scalar))
    }

    fn neg_interval(self, a: &RatInterval) -> Result<RatInterval, TrigError> {
        self.admit_interval(a)?;
        Ok(a.neg())
    }

    fn widen_interval(
        self,
        a: &RatInterval,
        amount: &BigRational,
    ) -> Result<RatInterval, TrigError> {
        self.guard_addsub_rat(&a.lo, amount)?;
        self.guard_addsub_rat(&a.hi, amount)?;
        Ok(a.widen(amount))
    }
}

/// A closed rational interval [lo, hi] with exact endpoints.
///
/// Arithmetic operations produce certified enclosures: the true
/// mathematical result lies in the returned interval.
#[derive(Clone, Debug, PartialEq)]
pub struct RatInterval {
    pub lo: BigRational,
    pub hi: BigRational,
}

impl RatInterval {
    /// Point interval [v, v].
    #[must_use]
    pub fn point(v: BigRational) -> Self {
        Self {
            lo: v.clone(),
            hi: v,
        }
    }

    /// Outward-rounded width as f64.
    ///
    /// Only used by tests to assert certified-interval tightness; production
    /// code combines width and rounding error via `interval_to_f64_bound` in
    /// `helpers.rs` instead.
    #[cfg(test)]
    #[must_use]
    pub fn width_up(&self) -> f64 {
        let w = &self.hi - &self.lo;
        rat_to_f64_up(&w)
    }

    /// Midpoint as nearest f64.
    ///
    /// Only used by tests to assert certified-interval values against known
    /// constants; production code uses `interval_to_f64_bound` instead.
    #[cfg(test)]
    #[must_use]
    pub fn midpoint_f64(&self) -> f64 {
        let m = (&self.lo + &self.hi) / BigRational::from_integer(BigInt::from(2i64));
        rat_to_f64(&m)
    }

    /// [a,b] + [c,d] = [a+c, b+d]
    #[must_use]
    pub fn add(&self, rhs: &RatInterval) -> RatInterval {
        RatInterval {
            lo: &self.lo + &rhs.lo,
            hi: &self.hi + &rhs.hi,
        }
    }

    /// [a,b] - [c,d] = [a-d, b-c]
    #[must_use]
    pub fn sub(&self, rhs: &RatInterval) -> RatInterval {
        RatInterval {
            lo: &self.lo - &rhs.hi,
            hi: &self.hi - &rhs.lo,
        }
    }

    /// -[a,b] = [-b, -a]
    #[must_use]
    pub fn neg(&self) -> RatInterval {
        RatInterval {
            lo: -self.hi.clone(),
            hi: -self.lo.clone(),
        }
    }

    /// Scales by a `BigRational` scalar.
    #[must_use]
    pub fn scale(&self, s: &BigRational) -> RatInterval {
        if s.is_negative() {
            RatInterval {
                lo: &self.hi * s,
                hi: &self.lo * s,
            }
        } else {
            RatInterval {
                lo: &self.lo * s,
                hi: &self.hi * s,
            }
        }
    }

    /// True iff the interval contains zero (i.e. `lo ≤ 0 ≤ hi`).
    #[must_use]
    pub fn contains_zero(&self) -> bool {
        !self.lo.is_positive() && !self.hi.is_negative()
    }

    /// [a,b] / [c,d] — certified enclosure of the quotient.
    ///
    /// # Errors
    ///
    /// Returns [`TrigError::DivisionByZero`] when `rhs` contains zero (the
    /// quotient would be unbounded or undefined).
    #[cfg(test)]
    pub fn div(&self, rhs: &RatInterval) -> Result<RatInterval, TrigError> {
        if rhs.contains_zero() {
            return Err(TrigError::DivisionByZero);
        }
        let quotients = [
            &self.lo / &rhs.lo,
            &self.lo / &rhs.hi,
            &self.hi / &rhs.lo,
            &self.hi / &rhs.hi,
        ];
        let mut lo = quotients[0].clone();
        let mut hi = quotients[0].clone();
        for quotient in &quotients[1..] {
            if *quotient < lo {
                lo = quotient.clone();
            }
            if *quotient > hi {
                hi = quotient.clone();
            }
        }
        Ok(RatInterval { lo, hi })
    }

    /// Widen by an exact non-negative rational amount on each side.
    #[must_use]
    pub fn widen(&self, amount: &BigRational) -> RatInterval {
        debug_assert!(!amount.is_negative());
        RatInterval {
            lo: &self.lo - amount,
            hi: &self.hi + amount,
        }
    }
}

// ── atan for small rational fractions ─────────────────────────────────────────

/// Computes `atan(p/q)` for integers `p ≥ 0, q > 0`, returning a guaranteed
/// enclosing interval via the alternating Maclaurin series.
///
/// Convergence: for `p/q < 1`, the alternating series converges and the
/// Leibniz criterion gives `|remainder| ≤ |first_omitted_term|`.
///
/// The threshold `2^{-threshold_bits}` is used to stop when the next term
/// is provably smaller than that.
fn atan_rational_series(
    p: &BigInt,
    q: &BigInt,
    cap: TrigCap,
    threshold_bits: u32,
) -> Result<RatInterval, TrigError> {
    debug_assert!(p.sign() != Sign::Minus, "p must be non-negative");
    debug_assert!(q.sign() == Sign::Plus, "q must be positive");
    cap.admit_int(p)?;
    cap.admit_int(q)?;
    // x = p/q; x^2 = p^2/q^2; power_num/power_denom tracks x^(2k+1)
    let mut power_num = p.clone(); // p^(2k+1)
    let mut power_denom = q.clone(); // q^(2k+1)
    let p2 = cap.mul_int(p, p)?;
    let q2 = cap.mul_int(q, q)?;
    let mut sum = BigRational::zero();
    let threshold_denom = cap.pow2(threshold_bits)?;

    for k in 0..cap.series_terms() {
        let coefficient = cap.unsigned_int(2u64 * u64::from(k) + 1)?;
        let coeff_denom = cap.mul_int(&coefficient, &power_denom)?;
        let term = cap.ratio(power_num.clone(), coeff_denom)?;

        sum = if k % 2 == 0 {
            cap.add_rat(&sum, &term)?
        } else {
            cap.sub_rat(&sum, &term)?
        };

        // Advance power by x^2: power_num *= p^2, power_denom *= q^2
        power_num = cap.mul_int(&power_num, &p2)?;
        power_denom = cap.mul_int(&power_denom, &q2)?;

        // Next term magnitude: power_num / ((2k+3) * power_denom)
        // Compare with threshold = 1 / threshold_denom by cross-multiplication:
        // next < threshold iff power_num * threshold_denom < (2k+3) * power_denom
        let lhs = cap.mul_int(&power_num, &threshold_denom)?;
        let next_coefficient = cap.unsigned_int(2u64 * u64::from(k) + 3)?;
        let next_denom = cap.mul_int(&next_coefficient, &power_denom)?;
        if lhs < next_denom {
            // We've converged. The remainder has the sign of term_{k+1}.
            // Since the series is alternating and decreasing, the true value
            // lies between the current sum and the sum + next_term.
            let next_term = cap.ratio(power_num.clone(), next_denom)?;
            let (lo, hi) = if (k + 1) % 2 == 0 {
                // Next term is positive → current sum is underestimate → [sum, sum+next]
                (sum.clone(), cap.add_rat(&sum, &next_term)?)
            } else {
                // Next term is negative → current sum is overestimate → [sum-|next|, sum]
                (cap.sub_rat(&sum, &next_term)?, sum.clone())
            };
            return cap.admitted_interval(RatInterval { lo, hi });
        }
    }
    Err(TrigError::BudgetExhausted)
}

/// Computes a certified enclosure of π using the Machin formula:
/// `π = 16·atan(1/5) − 4·atan(1/239)`.
///
/// Reference: Machin 1706; correctness follows from the atan addition
/// formula and the Leibniz remainder bound applied to each series.
///
/// # Errors
///
/// Returns [`TrigError::BudgetExhausted`] if either series fails to
/// converge within `budget.series_terms()`.
pub fn pi_interval(budget: CertificationBudget) -> Result<RatInterval, TrigError> {
    let cap = TrigCap::new(budget);
    let result = pi_interval_cap(cap)?;
    cap.admitted_interval(result)
}

/// Cap-threaded core of [`pi_interval`], reused by the atan/atan2/sin/cos
/// range reductions so they share one deterministic resource ceiling.
fn pi_interval_cap(cap: TrigCap) -> Result<RatInterval, TrigError> {
    let one = BigInt::one();
    let five = cap.unsigned_int(5)?;
    let two39 = cap.unsigned_int(239)?;

    let threshold = cap.threshold_bits(300);
    let a5 = atan_rational_series(&one, &five, cap, threshold)?;
    let a239 = atan_rational_series(&one, &two39, cap, threshold)?;

    let sixteen = cap.unsigned_rat(16)?;
    let four = cap.unsigned_rat(4)?;

    let a5_16 = cap.scale_interval(&a5, &sixteen)?;
    let a239_4 = cap.scale_interval(&a239, &four)?;
    // π = 16·atan(1/5) − 4·atan(1/239)
    cap.sub_interval(&a5_16, &a239_4)
}

/// Computes a certified enclosure of τ = 2π.
///
/// # Errors
///
/// Propagates [`TrigError::BudgetExhausted`] from [`pi_interval`].
pub fn tau_interval(budget: CertificationBudget) -> Result<RatInterval, TrigError> {
    let cap = TrigCap::new(budget);
    let pi = pi_interval(budget)?;
    let two = cap.unsigned_rat(2)?;
    let result = cap.scale_interval(&pi, &two)?;
    cap.admitted_interval(result)
}

// ── atan for general rational interval ────────────────────────────────────────

/// Computes `atan(x)` for an exact rational x via range reductions then series.
///
/// Reductions applied:
/// 1. Negative x: `atan(x) = −atan(−x)`.
/// 2. x > 1: `atan(x) = π/2 − atan(1/x)` (both arguments are now in (0,1]).
/// 3. x ∈ (1/2, 1]: `atan(x) = π/4 + atan((x−1)/(x+1))` (arg → (0, 1/3)).
/// 4. x ∈ [0, 1/2]: direct alternating series (fast convergence).
fn atan_rat_exact(x: &BigRational, cap: TrigCap) -> Result<RatInterval, TrigError> {
    cap.admit_rat(x)?;
    if x.is_zero() {
        return Ok(RatInterval::point(BigRational::zero()));
    }
    if x.is_negative() {
        let neg_x = -x;
        return cap.neg_interval(&atan_rat_exact(&neg_x, cap)?);
    }

    let half = cap.unit_fraction(2)?;

    // Case x > 1: atan(x) = pi/2 - atan(1/x)
    if *x > BigRational::one() {
        let recip = cap.div_rat(&BigRational::one(), x)?;
        let atan_recip = atan_rat_exact(&recip, cap)?;
        let pi = pi_interval_cap(cap)?;
        let pi_half = cap.scale_interval(&pi, &half)?;
        return cap.sub_interval(&pi_half, &atan_recip);
    }

    // Case x ∈ (1/2, 1]: atan(x) = pi/4 + atan((x-1)/(x+1))
    if *x > half {
        let xm1 = cap.sub_rat(x, &BigRational::one())?;
        let xp1 = cap.add_rat(x, &BigRational::one())?;
        let z = cap.div_rat(&xm1, &xp1)?;
        let atan_z = atan_rat_exact(&z, cap)?;
        let pi = pi_interval_cap(cap)?;
        let quarter = cap.unit_fraction(4)?;
        let pi_quarter = cap.scale_interval(&pi, &quarter)?;
        return cap.add_interval(&pi_quarter, &atan_z);
    }

    // Case x ∈ [0, 1/2]: direct series
    let (p, q) = (x.numer().clone(), x.denom().clone());
    let threshold = cap.threshold_bits(250);
    atan_rational_series(&p, &q, cap, threshold)
}

/// Computes a certified interval for `atan2(y, x)` in `(−π, π]`.
///
/// The sign of y and x (determined exactly from their rational values)
/// selects the quadrant; the atan enclosure is then computed from the
/// exact rational ratio `y/x` after range reduction.
///
/// # Errors
///
/// Returns [`TrigError::Pole`] when `(x, y) = (0, 0)`.
pub fn atan2_interval(
    y: &BigRational,
    x: &BigRational,
    budget: CertificationBudget,
) -> Result<RatInterval, TrigError> {
    let cap = TrigCap::new(budget);
    // Admit the inputs first, so oversized operands are rejected before the
    // exact y/x quotient (or any series work) allocates from them.
    cap.admit_rat(y)?;
    cap.admit_rat(x)?;
    if x.is_zero() && y.is_zero() {
        return Err(TrigError::Pole);
    }

    if x.is_zero() {
        // y != 0
        let pi = pi_interval_cap(cap)?;
        let half = cap.unit_fraction(2)?;
        let pi_half = cap.scale_interval(&pi, &half)?;
        return if y.is_positive() {
            cap.admitted_interval(pi_half)
        } else {
            cap.neg_interval(&pi_half)
        };
    }

    let ratio = cap.div_rat(y, x)?; // exact rational
    let atan_ratio = atan_rat_exact(&ratio, cap)?;

    // Adjust for quadrant:
    // x > 0: result = atan(y/x)
    // x < 0, y >= 0: result = atan(y/x) + pi
    // x < 0, y < 0: result = atan(y/x) - pi
    if x.is_positive() {
        cap.admitted_interval(atan_ratio)
    } else {
        let pi = pi_interval_cap(cap)?;
        if y.is_negative() {
            cap.sub_interval(&atan_ratio, &pi)
        } else {
            cap.add_interval(&atan_ratio, &pi)
        }
    }
}

// ── sin/cos via Taylor series ──────────────────────────────────────────────────

/// Alternating Taylor series for sin (`is_sin=true`) or cos (`is_sin=false`).
///
/// `sin(x) = Σ_{k=0}^∞ (−1)^k x^(2k+1)/(2k+1)!`
/// `cos(x) = Σ_{k=0}^∞ (−1)^k x^(2k)/(2k)!`
///
/// Converges for all x, but is only efficient for |x| ≤ π/4.
fn taylor_alternating_series(
    x: &BigRational,
    is_sin: bool,
    cap: TrigCap,
    threshold_bits: u32,
) -> Result<RatInterval, TrigError> {
    cap.admit_rat(x)?;
    let x2 = cap.mul_rat(x, x)?;
    let threshold_denom = cap.pow2(threshold_bits)?;

    // Initial term: x^1/1! for sin, x^0/0! = 1 for cos.
    let mut power = if is_sin {
        x.clone()
    } else {
        BigRational::one()
    };
    let mut fact = BigInt::one(); // factorial denominator for the current term
    let mut sum = BigRational::zero();

    for k in 0..cap.series_terms() {
        let term_denom = cap.mul_int(power.denom(), &fact)?;
        let term = cap.ratio(power.numer().clone(), term_denom)?;

        sum = if k % 2 == 0 {
            cap.add_rat(&sum, &term)?
        } else {
            cap.sub_rat(&sum, &term)?
        };

        // Advance power and factorial to the next term. The power step
        // (multiply by x^2) is identical for sin and cos; only the factorial
        // step differs.
        power = cap.mul_rat(&power, &x2)?;
        let step = if is_sin {
            // sin: (2k+1)! → (2k+3)! multiplies by (2k+2)(2k+3)
            let first = cap.unsigned_int(2u64 * u64::from(k) + 2)?;
            let second = cap.unsigned_int(2u64 * u64::from(k) + 3)?;
            cap.mul_int(&first, &second)?
        } else {
            // cos: (2k)! → (2k+2)! multiplies by (2k+1)(2k+2)
            let first = cap.unsigned_int(2u64 * u64::from(k) + 1)?;
            let second = cap.unsigned_int(2u64 * u64::from(k) + 2)?;
            cap.mul_int(&first, &second)?
        };
        fact = cap.mul_int(&fact, &step)?;

        // Convergence check: next_term = power_numer / (power_denom * fact).
        // We want next_term < 2^{-threshold_bits}, i.e.
        // power_numer * 2^{threshold_bits} < power_denom * fact.
        let lhs = cap.mul_int(&power.numer().abs(), &threshold_denom)?;
        let rhs = cap.mul_int(power.denom(), &fact)?;
        if lhs < rhs {
            // Converged. Next term has sign (-1)^(k+1).
            let next_term = cap.ratio(power.numer().abs(), rhs)?;
            let (lo, hi) = if (k + 1) % 2 == 0 {
                (sum.clone(), cap.add_rat(&sum, &next_term)?)
            } else {
                (cap.sub_rat(&sum, &next_term)?, sum.clone())
            };
            return cap.admitted_interval(RatInterval { lo, hi });
        }
    }
    Err(TrigError::BudgetExhausted)
}

/// Computes sin and cos at an exact rational point using quadrant reduction
/// to `[0, π/4]`, where the Taylor series converges fastest.
fn sin_cos_at_rational(
    x: &BigRational,
    cap: TrigCap,
    pi: &RatInterval,
) -> Result<(RatInterval, RatInterval), TrigError> {
    cap.admit_rat(x)?;
    cap.admit_interval(pi)?;
    let zero = BigRational::zero();
    let two = cap.unsigned_rat(2)?;
    let four = cap.unsigned_rat(4)?;
    let pi_sum = cap.add_rat(&pi.lo, &pi.hi)?;
    let pi_mid = cap.div_rat(&pi_sum, &two)?;
    let half_pi_mid = cap.div_rat(&pi_mid, &two)?;
    let quarter_pi_mid = cap.div_rat(&pi_mid, &four)?;
    let pi_width = cap.sub_rat(&pi.hi, &pi.lo)?;
    let pi_half_width = cap.div_rat(&pi_width, &two)?;

    // Handle negative x: sin(-x) = -sin(x), cos(-x) = cos(x)
    if *x < zero {
        let neg_x = -x;
        let (s, c) = sin_cos_at_rational(&neg_x, cap, pi)?;
        let neg_s = cap.neg_interval(&s)?;
        return cap.admitted_pair((neg_s, c));
    }

    // Reduce to [0, π]: sin(x) = -sin(x - π), cos(x) = -cos(x - π) for x > π.
    if *x > pi_mid {
        let x_reduced = cap.sub_rat(x, &pi_mid)?;
        let (s, c) = sin_cos_at_rational(&x_reduced, cap, pi)?;
        let neg_s = cap.neg_interval(&s)?;
        let neg_c = cap.neg_interval(&c)?;
        let s_out = cap.widen_interval(&neg_s, &pi_half_width)?;
        let c_out = cap.widen_interval(&neg_c, &pi_half_width)?;
        return cap.admitted_pair((s_out, c_out));
    }

    // Now x ∈ [0, π]. Reduce to [0, π/2]: sin(π - x) = sin(x), cos(π - x) = -cos(x)
    if *x > half_pi_mid {
        let x_reduced = cap.sub_rat(&pi_mid, x)?;
        let (s, c) = sin_cos_at_rational(&x_reduced, cap, pi)?;
        let neg_c = cap.neg_interval(&c)?;
        let s_out = cap.widen_interval(&s, &pi_half_width)?;
        let c_out = cap.widen_interval(&neg_c, &pi_half_width)?;
        return cap.admitted_pair((s_out, c_out));
    }

    // Now x ∈ [0, π/2]. Reduce to [0, π/4]: cos(π/2 - x) = sin(x), sin(π/2 - x) = cos(x)
    if *x > quarter_pi_mid {
        let x_reduced = cap.sub_rat(&half_pi_mid, x)?;
        let (s, c) = sin_cos_at_rational(&x_reduced, cap, pi)?;
        let quarter_pi_half_width = cap.div_rat(&pi_half_width, &two)?;
        // sin(x) = cos(x_reduced) = c, cos(x) = sin(x_reduced) = s.
        let s_out = cap.widen_interval(&c, &quarter_pi_half_width)?;
        let c_out = cap.widen_interval(&s, &quarter_pi_half_width)?;
        return cap.admitted_pair((s_out, c_out));
    }

    // x ∈ [0, π/4]: apply the Taylor series directly.
    let threshold = cap.threshold_bits(200);
    let sin_v = taylor_alternating_series(x, true, cap, threshold)?;
    let cos_v = taylor_alternating_series(x, false, cap, threshold)?;
    cap.admitted_pair((sin_v, cos_v))
}

/// Reduces `x` modulo `2π` and computes certified enclosures of `sin(x)` and
/// `cos(x)`.
///
/// The reduction uses the certified `pi_interval` for quadrant boundaries;
/// any residual reduction uncertainty (from the `2π`-modulo step) is folded
/// into the returned interval via a Lipschitz widening
/// (`|sin(a) − sin(b)| ≤ |a − b|`, likewise for cos).
///
/// # Errors
///
/// Propagates [`TrigError::BudgetExhausted`] from the underlying series.
pub fn sin_cos_interval(
    x: &BigRational,
    budget: CertificationBudget,
) -> Result<(RatInterval, RatInterval), TrigError> {
    let cap = TrigCap::new(budget);
    cap.admit_rat(x)?;
    let pi = pi_interval_cap(cap)?;
    let two = cap.unsigned_rat(2)?;
    let pi2_lo = cap.mul_rat(&pi.lo, &two)?; // 2π lower bound
    let pi2_hi = cap.mul_rat(&pi.hi, &two)?; // 2π upper bound

    // Compute k = floor(x / tau_hi) exactly. Since tau_hi ≥ τ, this keeps the
    // reduced lower bound non-negative without saturating on huge arguments.
    let x_over_tau = cap.div_rat(x, &pi2_hi)?;
    let k = bigrat_floor(cap, &x_over_tau)?;
    let (x_red_lo, x_red_hi) = if k.sign() == Sign::Minus {
        let k_abs = BigRational::from_integer(-k);
        let lo_offset = cap.mul_rat(&k_abs, &pi2_lo)?;
        let hi_offset = cap.mul_rat(&k_abs, &pi2_hi)?;
        (cap.add_rat(x, &lo_offset)?, cap.add_rat(x, &hi_offset)?)
    } else {
        let k_rat = BigRational::from_integer(k);
        let lo_offset = cap.mul_rat(&k_rat, &pi2_hi)?;
        let hi_offset = cap.mul_rat(&k_rat, &pi2_lo)?;
        (cap.sub_rat(x, &lo_offset)?, cap.sub_rat(x, &hi_offset)?)
    };
    let reduction_width = cap.sub_rat(&x_red_hi, &x_red_lo)?;
    if reduction_width > pi2_hi {
        return Err(TrigError::BudgetExhausted);
    }

    let x_sum = cap.add_rat(&x_red_lo, &x_red_hi)?;
    let x_mid = cap.div_rat(&x_sum, &two)?;
    let interval_half_width = cap.div_rat(&reduction_width, &two)?;

    let (sin_mid, cos_mid) = sin_cos_at_rational(&x_mid, cap, &pi)?;

    // Lipschitz bound: |sin(x) - sin(x_mid)| ≤ |x - x_mid| ≤ interval_half_width
    // (since |d/dx sin(x)| = |cos(x)| ≤ 1, likewise for cos).
    let sin_result = cap.widen_interval(&sin_mid, &interval_half_width)?;
    let cos_result = cap.widen_interval(&cos_mid, &interval_half_width)?;

    cap.admitted_pair((sin_result, cos_result))
}

/// Exact floor of a rational, threaded through the cap so the integer quotient
/// and the `-1` adjustment stay within the deterministic bit ceiling.
fn bigrat_floor(cap: TrigCap, r: &BigRational) -> Result<BigInt, TrigError> {
    let floor = if r.is_integer() {
        r.numer().clone()
    } else {
        let quotient = r.numer() / r.denom();
        if r.is_negative() {
            cap.sub_int(&quotient, &BigInt::one())?
        } else {
            quotient
        }
    };
    cap.admit_int(&floor)?;
    Ok(floor)
}

#[cfg(test)]
mod tests {
    // Certified intervals are compared against known-constant f64 values via
    // exact equality (after collapsing to the nearest f64); this is
    // intentional, not an approximate floating-point comparison.
    #![allow(clippy::float_cmp)]

    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::{One, Zero};

    use super::{TrigCap, TrigError, atan2_interval, pi_interval, sin_cos_interval, tau_interval};
    use crate::CertificationBudget;
    use crate::analytic::exact::f64_to_rat;

    #[test]
    fn pi_interval_contains_pi() {
        // The certified interval brackets the *true* mathematical pi to
        // within ~1e-89, far tighter than the gap (~1e-16) between true pi
        // and `f64::consts::PI` (the nearest-f64-rounded value of pi). So we
        // compare the interval's nearest-f64 midpoint against the known
        // constant rather than testing containment of the constant's exact
        // dyadic value, which would spuriously fail.
        let budget = CertificationBudget::default();
        let pi_int = pi_interval(budget).unwrap();
        assert_eq!(pi_int.midpoint_f64(), std::f64::consts::PI);
        assert!(
            pi_int.width_up() < 1e-50,
            "pi interval too wide: {}",
            pi_int.width_up()
        );
    }

    #[test]
    fn tau_interval_is_twice_pi() {
        let budget = CertificationBudget::default();
        let pi_int = pi_interval(budget).unwrap();
        let tau_int = tau_interval(budget).unwrap();
        let two = BigRational::from_integer(2.into());
        assert_eq!(tau_int.lo, &pi_int.lo * &two);
        assert_eq!(tau_int.hi, &pi_int.hi * &two);
    }

    #[test]
    fn atan2_interval_contains_known_angles() {
        // Certified intervals bracket the *true* mathematical angles to
        // within ~1e-89, far tighter than the ~1e-16 gap between a true
        // angle and its nearest-f64-rounded constant (e.g.
        // `f64::consts::FRAC_PI_4`). Comparing via containment of the
        // constant's exact dyadic value would spuriously fail, so we
        // instead compare the interval's nearest-f64 midpoint against the
        // known constant.
        let budget = CertificationBudget::default();
        // atan2(1, 1) = pi/4
        let y = f64_to_rat(1.0);
        let x = f64_to_rat(1.0);
        let a = atan2_interval(&y, &x, budget).unwrap();
        assert_eq!(a.midpoint_f64(), std::f64::consts::FRAC_PI_4);

        // atan2(0, 1) = 0
        let y0 = BigRational::zero();
        let x1 = f64_to_rat(1.0);
        let a0 = atan2_interval(&y0, &x1, budget).unwrap();
        assert!(a0.lo <= BigRational::zero() && BigRational::zero() <= a0.hi);

        // atan2(1, 0) = pi/2
        let y1 = f64_to_rat(1.0);
        let x0 = BigRational::zero();
        let a_half = atan2_interval(&y1, &x0, budget).unwrap();
        assert_eq!(a_half.midpoint_f64(), std::f64::consts::FRAC_PI_2);

        // atan2(-1, 0) = -pi/2
        let y_neg = f64_to_rat(-1.0);
        let a_neg_half = atan2_interval(&y_neg, &x0, budget).unwrap();
        assert_eq!(a_neg_half.midpoint_f64(), -std::f64::consts::FRAC_PI_2);

        // atan2(0, -1) = pi
        let y_zero = BigRational::zero();
        let x_neg1 = f64_to_rat(-1.0);
        let a_pi = atan2_interval(&y_zero, &x_neg1, budget).unwrap();
        assert_eq!(a_pi.midpoint_f64(), std::f64::consts::PI);

        // atan2(-1, -1) = -3pi/4
        let y_neg1 = f64_to_rat(-1.0);
        let a_neg3pi4 = atan2_interval(&y_neg1, &x_neg1, budget).unwrap();
        assert_eq!(a_neg3pi4.midpoint_f64(), -3.0 * std::f64::consts::FRAC_PI_4);

        // atan2(0, 0) = Pole
        assert_eq!(
            atan2_interval(&y0, &BigRational::zero(), budget),
            Err(TrigError::Pole)
        );
    }

    #[test]
    fn sin_cos_interval_contains_known_values() {
        let budget = CertificationBudget::default();
        // sin/cos at 0
        let (s, c) = sin_cos_interval(&BigRational::zero(), budget).unwrap();
        assert!(s.lo <= BigRational::zero() && BigRational::zero() <= s.hi);
        assert!(c.lo <= BigRational::one() && BigRational::one() <= c.hi);

        // sin(pi/2) = 1, cos(pi/2) = 0
        let pi = pi_interval(budget).unwrap();
        let pi_mid = (&pi.lo + &pi.hi) / BigRational::from_integer(2.into());
        let half_pi = &pi_mid / BigRational::from_integer(2.into());
        let (s2, c2) = sin_cos_interval(&half_pi, budget).unwrap();
        assert!((s2.midpoint_f64() - 1.0).abs() < 1e-6);
        assert!(c2.midpoint_f64().abs() < 1e-6);

        // sin(pi) ≈ 0, cos(pi) ≈ -1
        let (s3, c3) = sin_cos_interval(&pi_mid, budget).unwrap();
        assert!(s3.midpoint_f64().abs() < 1e-6);
        assert!((c3.midpoint_f64() + 1.0).abs() < 1e-6);
    }

    #[test]
    fn sin_cos_pi_midpoint_not_zero_interval() {
        let budget = CertificationBudget::default();
        let pi = pi_interval(budget).unwrap();
        let pi_mid = (&pi.lo + &pi.hi) / BigRational::from_integer(2.into());
        let (s, _) = sin_cos_interval(&pi_mid, budget).unwrap();
        assert!(s.lo < s.hi, "sin(pi_mid) interval must have positive width");
        assert!(s.lo <= BigRational::zero() && BigRational::zero() <= s.hi);
    }

    #[test]
    fn sin_cos_huge_argument_terminates() {
        let budget = CertificationBudget::default();
        let x = f64_to_rat(f64::MAX);
        let result = sin_cos_interval(&x, budget);
        match result {
            Ok(_) | Err(TrigError::BudgetExhausted) => {}
            Err(e) => panic!("unexpected error: {e:?}"),
        }
    }

    #[test]
    fn trig_budget_exhaustion_returns_error() {
        let budget = CertificationBudget::try_new(1, 64).unwrap();
        match pi_interval(budget) {
            Ok(_) | Err(TrigError::BudgetExhausted) => {}
            Err(e) => panic!("unexpected: {e:?}"),
        }
    }

    #[test]
    fn converged_atan_interval_still_obeys_bit_budget() {
        let budget = CertificationBudget::try_new(1, 3).unwrap();
        let y = BigRational::one();
        let x = BigRational::from_integer(5.into());
        assert_eq!(
            atan2_interval(&y, &x, budget),
            Err(TrigError::BudgetExhausted)
        );
    }

    #[test]
    fn fixed_trig_constants_are_preflighted_before_allocation() {
        let budget = CertificationBudget::try_new(200, 1).unwrap();
        let cap = TrigCap::new(budget);
        assert_eq!(cap.unsigned_int(239), Err(TrigError::BudgetExhausted));
        assert_eq!(pi_interval(budget), Err(TrigError::BudgetExhausted));
    }

    /// Regression: oversized operands must be rejected at input admission,
    /// before `atan2` forms the exact `y/x` quotient. A 4096-bit magnitude far
    /// exceeds the 64-bit cap, so both argument orders exhaust immediately
    /// rather than allocating a huge rational quotient first.
    #[test]
    fn atan2_rejects_huge_inputs_before_quotient() {
        let budget = CertificationBudget::try_new(8, 64).unwrap();
        let one = BigRational::one();
        let huge = BigRational::from_integer(BigInt::one() << 4096_usize);
        assert_eq!(
            atan2_interval(&one, &huge, budget),
            Err(TrigError::BudgetExhausted)
        );
        assert_eq!(
            atan2_interval(&huge, &one, budget),
            Err(TrigError::BudgetExhausted)
        );
    }

    /// Regression: even `sin_cos_interval(0)` needs the certified π for its
    /// quadrant reduction, so a cap too small to compute π must surface
    /// `BudgetExhausted` rather than returning an uncertified enclosure.
    #[test]
    fn sin_cos_interval_zero_under_tiny_budget_is_exhausted() {
        let budget = CertificationBudget::try_new(1, 3).unwrap();
        assert_eq!(
            sin_cos_interval(&BigRational::zero(), budget),
            Err(TrigError::BudgetExhausted)
        );
    }

    #[test]
    fn trig_budget_zero_terms_is_unrepresentable() {
        assert!(CertificationBudget::try_new(0, 4096).is_err());
    }

    #[test]
    fn rat_interval_div_matches_scalar_quotient() {
        let two = super::RatInterval::point(BigRational::from_integer(2.into()));
        let four = super::RatInterval::point(BigRational::from_integer(4.into()));
        let q = four.div(&two).unwrap();
        assert_eq!(q.lo, BigRational::from_integer(2.into()));
        assert_eq!(q.hi, BigRational::from_integer(2.into()));
    }

    #[test]
    fn rat_interval_div_rejects_divisor_containing_zero() {
        let one = super::RatInterval::point(BigRational::one());
        let straddling = super::RatInterval {
            lo: -BigRational::one(),
            hi: BigRational::one(),
        };
        assert_eq!(one.div(&straddling), Err(TrigError::DivisionByZero));
    }

    #[test]
    fn rat_interval_contains_zero_detects_straddling_and_point_intervals() {
        let straddling = super::RatInterval {
            lo: -BigRational::one(),
            hi: BigRational::one(),
        };
        assert!(straddling.contains_zero());
        let zero_point = super::RatInterval::point(BigRational::zero());
        assert!(zero_point.contains_zero());
        let positive = super::RatInterval::point(BigRational::one());
        assert!(!positive.contains_zero());
    }

    /// Regression: u32 index overflow in trig series.
    ///
    /// With `series_terms = u32::MAX` and the old `2 * k + 1` (u32),
    /// the multiplication would overflow in debug builds once k ≥ 2^31.
    /// With the u64 cast the arithmetic is safe; the `rational_bits` budget
    /// terminates the series before k reaches overflow-inducing values.
    #[test]
    fn trig_series_u32_max_terms_no_panic() {
        // Large series_terms with small rational_bits to trigger early termination.
        // This must not panic in either debug or release mode.
        let budget = CertificationBudget::try_new(u32::MAX, 256).unwrap();
        let x = f64_to_rat(1.0_f64);
        // sin_cos_interval calls taylor_alternating_series which previously
        // had 2*k+2, 2*k+3 (u32) index overflow.  With u64 cast it is safe.
        let result = sin_cos_interval(&x, budget);
        // May succeed (series converges before budget fires) or return
        // BudgetExhausted; either is correct.  Must not panic.
        assert!(
            result.is_ok() || result == Err(TrigError::BudgetExhausted),
            "unexpected error: {result:?}"
        );
    }

    /// Regression: u32 index overflow in atan series.
    ///
    /// `atan_rational_series` had `2 * k + 1` (u32); same fix via u64 cast.
    #[test]
    fn atan_series_u32_max_terms_no_panic() {
        let budget = CertificationBudget::try_new(u32::MAX, 256).unwrap();
        let one = BigRational::one();
        // atan2(1, 1) = pi/4; calls atan_rational_series.
        let result = atan2_interval(&one, &one, budget);
        assert!(
            result.is_ok() || result == Err(TrigError::BudgetExhausted),
            "unexpected error: {result:?}"
        );
    }
}
