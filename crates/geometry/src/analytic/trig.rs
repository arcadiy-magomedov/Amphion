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

/// Checks a `BigRational` value against the certification budget's bit-width
/// limit, returning [`TrigError::BudgetExhausted`] when exceeded.
fn check_budget(budget: CertificationBudget, r: &BigRational) -> Result<(), TrigError> {
    let numer_bits = r.numer().bits();
    let denom_bits = r.denom().bits();
    if numer_bits > u64::from(budget.rational_bits) || denom_bits > u64::from(budget.rational_bits)
    {
        Err(TrigError::BudgetExhausted)
    } else {
        Ok(())
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

    /// [a,b] * [c,d] = [min products, max products]
    #[must_use]
    pub fn mul(&self, rhs: &RatInterval) -> RatInterval {
        let products = [
            &self.lo * &rhs.lo,
            &self.lo * &rhs.hi,
            &self.hi * &rhs.lo,
            &self.hi * &rhs.hi,
        ];
        let mut lo = products[0].clone();
        let mut hi = products[0].clone();
        for product in &products[1..] {
            if *product < lo {
                lo = product.clone();
            }
            if *product > hi {
                hi = product.clone();
            }
        }
        RatInterval { lo, hi }
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
    budget: CertificationBudget,
    threshold_bits: u32,
) -> Result<RatInterval, TrigError> {
    debug_assert!(p.sign() != Sign::Minus, "p must be non-negative");
    debug_assert!(q.sign() == Sign::Plus, "q must be positive");
    // x = p/q; x^2 = p^2/q^2; power_num/power_denom tracks x^(2k+1)
    let mut power_num = p.clone(); // p^(2k+1)
    let mut power_denom = q.clone(); // q^(2k+1)
    let p2 = p * p;
    let q2 = q * q;
    let mut sum = BigRational::zero();
    let threshold_denom = BigInt::one() << threshold_bits as usize;

    for k in 0..budget.series_terms {
        let coeff_denom = BigInt::from(2u64 * u64::from(k) + 1) * &power_denom;
        let term = BigRational::new(power_num.clone(), coeff_denom);

        if k % 2 == 0 {
            sum += &term;
        } else {
            sum -= &term;
        }

        // Advance power by x^2: power_num *= p^2, power_denom *= q^2
        power_num *= &p2;
        power_denom *= &q2;

        // Next term magnitude: power_num / ((2k+3) * power_denom)
        // Compare with threshold = 1 / threshold_denom by cross-multiplication:
        // next < threshold iff power_num * threshold_denom < (2k+3) * power_denom
        let next_coeff = BigInt::from(2u64 * u64::from(k) + 3);
        let lhs = &power_num * &threshold_denom;
        let rhs = &next_coeff * &power_denom;
        if lhs < rhs {
            // We've converged. The remainder has the sign of term_{k+1}.
            // Since the series is alternating and decreasing, the true value
            // lies between the current sum and the sum + next_term.
            let next_term = BigRational::new(power_num.clone(), next_coeff * &power_denom);
            let (lo, hi) = if (k + 1) % 2 == 0 {
                // Next term is positive → current sum is underestimate → [sum, sum+next]
                (sum.clone(), &sum + &next_term)
            } else {
                // Next term is negative → current sum is overestimate → [sum-|next|, sum]
                (&sum - &next_term, sum.clone())
            };
            return Ok(RatInterval { lo, hi });
        }

        check_budget(budget, &sum)?;
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
/// converge within `budget.series_terms`.
pub fn pi_interval(budget: CertificationBudget) -> Result<RatInterval, TrigError> {
    let one = BigInt::one();
    let five = BigInt::from(5i64);
    let two39 = BigInt::from(239i64);

    let threshold = budget.rational_bits.min(300);
    let a5 = atan_rational_series(&one, &five, budget, threshold)?;
    let a239 = atan_rational_series(&one, &two39, budget, threshold)?;

    let sixteen = BigRational::from_integer(BigInt::from(16i64));
    let four = BigRational::from_integer(BigInt::from(4i64));

    let a5_16 = a5.scale(&sixteen);
    let a239_4 = a239.scale(&four);
    // π = 16·atan(1/5) − 4·atan(1/239)
    Ok(a5_16.sub(&a239_4))
}

/// Computes a certified enclosure of τ = 2π.
///
/// # Errors
///
/// Propagates [`TrigError::BudgetExhausted`] from [`pi_interval`].
pub fn tau_interval(budget: CertificationBudget) -> Result<RatInterval, TrigError> {
    let pi = pi_interval(budget)?;
    let two = BigRational::from_integer(BigInt::from(2i64));
    Ok(pi.scale(&two))
}

// ── atan for general rational interval ────────────────────────────────────────

/// Computes `atan(x)` for an exact rational x via range reductions then series.
///
/// Reductions applied:
/// 1. Negative x: `atan(x) = −atan(−x)`.
/// 2. x > 1: `atan(x) = π/2 − atan(1/x)` (both arguments are now in (0,1]).
/// 3. x ∈ (1/2, 1]: `atan(x) = π/4 + atan((x−1)/(x+1))` (arg → (0, 1/3)).
/// 4. x ∈ [0, 1/2]: direct alternating series (fast convergence).
fn atan_rat_exact(x: &BigRational, budget: CertificationBudget) -> Result<RatInterval, TrigError> {
    if x.is_zero() {
        return Ok(RatInterval::point(BigRational::zero()));
    }
    if x.is_negative() {
        return Ok(atan_rat_exact(&(-x), budget)?.neg());
    }

    let half = BigRational::new(BigInt::one(), BigInt::from(2i64));

    // Case x > 1: atan(x) = pi/2 - atan(1/x)
    if *x > BigRational::one() {
        let recip = BigRational::one() / x;
        let atan_recip = atan_rat_exact(&recip, budget)?;
        let pi = pi_interval(budget)?;
        let pi_half = pi.scale(&half);
        return Ok(pi_half.sub(&atan_recip));
    }

    // Case x ∈ (1/2, 1]: atan(x) = pi/4 + atan((x-1)/(x+1))
    if *x > half {
        let xm1 = x - BigRational::one();
        let xp1 = x + BigRational::one();
        let z = xm1 / xp1;
        let atan_z = atan_rat_exact(&z, budget)?;
        let pi = pi_interval(budget)?;
        let quarter = BigRational::new(BigInt::one(), BigInt::from(4i64));
        let pi_quarter = pi.scale(&quarter);
        return Ok(pi_quarter.add(&atan_z));
    }

    // Case x ∈ [0, 1/2]: direct series
    let (p, q) = (x.numer().clone(), x.denom().clone());
    let threshold = budget.rational_bits.min(250);
    atan_rational_series(&p, &q, budget, threshold)
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
    if x.is_zero() && y.is_zero() {
        return Err(TrigError::Pole);
    }

    if x.is_zero() {
        // y != 0
        let pi = pi_interval(budget)?;
        let half = BigRational::new(BigInt::one(), BigInt::from(2i64));
        let pi_half = pi.scale(&half);
        return if y.is_positive() {
            Ok(pi_half)
        } else {
            Ok(pi_half.neg())
        };
    }

    let ratio = y / x; // exact rational
    let atan_ratio = atan_rat_exact(&ratio, budget)?;

    // Adjust for quadrant:
    // x > 0: result = atan(y/x)
    // x < 0, y >= 0: result = atan(y/x) + pi
    // x < 0, y < 0: result = atan(y/x) - pi
    if x.is_positive() {
        Ok(atan_ratio)
    } else {
        let pi = pi_interval(budget)?;
        if y.is_negative() {
            Ok(atan_ratio.sub(&pi))
        } else {
            Ok(atan_ratio.add(&pi))
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
    budget: CertificationBudget,
    threshold_bits: u32,
) -> Result<RatInterval, TrigError> {
    let x2 = x * x;
    let threshold_denom = BigInt::one() << threshold_bits as usize;

    // Initial term: x^1/1! for sin, x^0/0! = 1 for cos.
    let mut power = if is_sin {
        x.clone()
    } else {
        BigRational::one()
    };
    let mut fact = BigInt::one(); // factorial denominator for the current term
    let mut sum = BigRational::zero();

    for k in 0..budget.series_terms {
        let term = BigRational::new(power.numer().clone(), power.denom().clone() * &fact);

        if k % 2 == 0 {
            sum += &term;
        } else {
            sum -= &term;
        }

        // Advance power and factorial to the next term.
        if is_sin {
            // sin: term k → k+1: multiply x^(2k+1) by x^2, (2k+1)! by (2k+2)(2k+3)
            power = &power * &x2;
            fact *= BigInt::from(2u64 * u64::from(k) + 2) * BigInt::from(2u64 * u64::from(k) + 3);
        } else {
            // cos: term k → k+1: multiply x^(2k) by x^2, (2k)! by (2k+1)(2k+2)
            power = &power * &x2;
            fact *= BigInt::from(2u64 * u64::from(k) + 1) * BigInt::from(2u64 * u64::from(k) + 2);
        }

        // Convergence check: next_term = power_numer / (power_denom * fact).
        // We want next_term < 2^{-threshold_bits}, i.e.
        // power_numer * 2^{threshold_bits} < power_denom * fact.
        let lhs = power.numer().abs() * &threshold_denom;
        let rhs = power.denom() * &fact;
        if lhs < rhs {
            // Converged. Next term has sign (-1)^(k+1).
            let next_term = BigRational::new(power.numer().abs(), power.denom().clone() * &fact);
            let (lo, hi) = if (k + 1) % 2 == 0 {
                (sum.clone(), &sum + &next_term)
            } else {
                (&sum - &next_term, sum.clone())
            };
            return Ok(RatInterval { lo, hi });
        }

        check_budget(budget, &sum)?;
    }
    Err(TrigError::BudgetExhausted)
}

/// Computes sin and cos at an exact rational point using quadrant reduction
/// to `[0, π/4]`, where the Taylor series converges fastest.
fn sin_cos_at_rational(
    x: &BigRational,
    budget: CertificationBudget,
    pi: &RatInterval,
) -> Result<(RatInterval, RatInterval), TrigError> {
    let zero = BigRational::zero();
    let two = BigRational::from_integer(BigInt::from(2i64));
    let pi_mid = (&pi.lo + &pi.hi) / &two;
    let half_pi_mid = &pi_mid / &two;
    let quarter_pi_mid = &pi_mid / BigRational::from_integer(BigInt::from(4i64));
    let pi_half_width = (&pi.hi - &pi.lo) / &two;

    // Handle negative x: sin(-x) = -sin(x), cos(-x) = cos(x)
    if *x < zero {
        let (s, c) = sin_cos_at_rational(&(-x), budget, pi)?;
        return Ok((s.neg(), c));
    }

    // Reduce to [0, π]: sin(x) = -sin(x - π), cos(x) = -cos(x - π) for x > π.
    if *x > pi_mid {
        let x_reduced = x - &pi_mid;
        let (s, c) = sin_cos_at_rational(&x_reduced, budget, pi)?;
        return Ok((s.neg().widen(&pi_half_width), c.neg().widen(&pi_half_width)));
    }

    // Now x ∈ [0, π]. Reduce to [0, π/2]: sin(π - x) = sin(x), cos(π - x) = -cos(x)
    if *x > half_pi_mid {
        let x_reduced = &pi_mid - x;
        let (s, c) = sin_cos_at_rational(&x_reduced, budget, pi)?;
        return Ok((s.widen(&pi_half_width), c.neg().widen(&pi_half_width)));
    }

    // Now x ∈ [0, π/2]. Reduce to [0, π/4]: cos(π/2 - x) = sin(x), sin(π/2 - x) = cos(x)
    if *x > quarter_pi_mid {
        let x_reduced = &half_pi_mid - x;
        let (s, c) = sin_cos_at_rational(&x_reduced, budget, pi)?;
        let quarter_pi_half_width = &pi_half_width / &two;
        return Ok((
            c.widen(&quarter_pi_half_width),
            s.widen(&quarter_pi_half_width),
        ));
    }

    // x ∈ [0, π/4]: apply the Taylor series directly.
    let threshold = budget.rational_bits.min(200);
    let sin_v = taylor_alternating_series(x, true, budget, threshold)?;
    let cos_v = taylor_alternating_series(x, false, budget, threshold)?;
    Ok((sin_v, cos_v))
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
    let pi = pi_interval(budget)?;
    let two = BigRational::from_integer(BigInt::from(2i64));
    let pi2_lo = &pi.lo * &two; // 2π lower bound
    let pi2_hi = &pi.hi * &two; // 2π upper bound

    // Compute k = floor(x / tau_hi) exactly. Since tau_hi ≥ τ, this keeps the
    // reduced lower bound non-negative without saturating on huge arguments.
    let k = bigrat_floor(&(x / &pi2_hi));
    let (x_red_lo, x_red_hi) = if k.sign() == Sign::Minus {
        let k_abs = BigRational::from_integer(-k);
        (x + &k_abs * &pi2_lo, x + &k_abs * &pi2_hi)
    } else {
        let k_rat = BigRational::from_integer(k);
        (x - &k_rat * &pi2_hi, x - &k_rat * &pi2_lo)
    };
    check_budget(budget, &x_red_lo)?;
    check_budget(budget, &x_red_hi)?;
    if &x_red_hi - &x_red_lo > pi2_hi {
        return Err(TrigError::BudgetExhausted);
    }

    let x_mid = (&x_red_lo + &x_red_hi) / &two;
    let interval_half_width = (&x_red_hi - &x_red_lo) / &two;

    let (sin_mid, cos_mid) = sin_cos_at_rational(&x_mid, budget, &pi)?;

    // Lipschitz bound: |sin(x) - sin(x_mid)| ≤ |x - x_mid| ≤ interval_half_width
    // (since |d/dx sin(x)| = |cos(x)| ≤ 1, likewise for cos).
    let sin_result = sin_mid.widen(&interval_half_width);
    let cos_result = cos_mid.widen(&interval_half_width);

    Ok((sin_result, cos_result))
}

fn bigrat_floor(r: &BigRational) -> BigInt {
    if r.is_integer() {
        r.numer().clone()
    } else {
        let q = r.numer() / r.denom();
        if r.is_negative() {
            q - BigInt::one()
        } else {
            q
        }
    }
}

#[cfg(test)]
mod tests {
    // Certified intervals are compared against known-constant f64 values via
    // exact equality (after collapsing to the nearest f64); this is
    // intentional, not an approximate floating-point comparison.
    #![allow(clippy::float_cmp)]

    use num_rational::BigRational;
    use num_traits::{One, Zero};

    use super::{TrigError, atan2_interval, pi_interval, sin_cos_interval, tau_interval};
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
        let budget = CertificationBudget {
            series_terms: 1,
            rational_bits: 64,
        };
        match pi_interval(budget) {
            Ok(_) | Err(TrigError::BudgetExhausted) => {}
            Err(e) => panic!("unexpected: {e:?}"),
        }
    }

    #[test]
    fn trig_budget_zero_terms_always_exhausts() {
        let budget = CertificationBudget {
            series_terms: 0,
            rational_bits: 4096,
        };
        assert_eq!(pi_interval(budget), Err(TrigError::BudgetExhausted));
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
        let budget = CertificationBudget {
            series_terms: u32::MAX,
            rational_bits: 256,
        };
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
        let budget = CertificationBudget {
            series_terms: u32::MAX,
            rational_bits: 256,
        };
        let one = BigRational::one();
        // atan2(1, 1) = pi/4; calls atan_rational_series.
        let result = atan2_interval(&one, &one, budget);
        assert!(
            result.is_ok() || result == Err(TrigError::BudgetExhausted),
            "unexpected error: {result:?}"
        );
    }
}
