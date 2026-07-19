//! Exact f64 ↔ `BigRational` conversion and directed rounding.
//!
//! Every finite f64 is an exact dyadic rational m·2^e. This module decodes
//! f64 values exactly, performs directed f64 → rational → f64 conversion
//! using only the f64 ordered representation, and computes certified sqrt
//! bounds via exact candidate-square comparisons.
//!
//! # References
//! - IEEE 754-2019 §3.4 (encoding), §5.3 (conversions), §5.4 (sqrt, correctly rounded).
//! - Goldberg (1991) "What Every Computer Scientist Should Know About Floating-Point Arithmetic."

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

/// Decodes a finite f64 exactly as a `BigRational` (m·2^e).
///
/// The caller must ensure `x.is_finite()`; this is checked with a debug
/// assertion only, since every call site in this crate first validates
/// finiteness through a foundation type constructor.
#[must_use]
pub(super) fn f64_to_rat(x: f64) -> BigRational {
    debug_assert!(x.is_finite(), "f64_to_rat requires finite input");
    if x == 0.0_f64 {
        return BigRational::zero();
    }
    let bits = x.to_bits();
    let sign_negative = (bits >> 63) != 0;
    let biased_exp = ((bits >> 52) & 0x7ff) as i32;
    let mantissa_bits = bits & 0x000f_ffff_ffff_ffff;

    let (significand, power): (BigInt, i32) = if biased_exp == 0 {
        // Subnormal: value = ±mantissa_bits × 2^(−1074)
        (BigInt::from(mantissa_bits), -1074)
    } else {
        // Normal: value = ±(2^52 + mantissa_bits) × 2^(biased_exp − 1075)
        let sig = (1u64 << 52) | mantissa_bits;
        (BigInt::from(sig), biased_exp - 1075)
    };

    let numer = if sign_negative {
        -significand
    } else {
        significand
    };
    if power >= 0 {
        #[allow(clippy::cast_sign_loss)]
        let shift = power as usize;
        BigRational::from_integer(numer << shift)
    } else {
        #[allow(clippy::cast_sign_loss)]
        let shift = (-power) as usize;
        BigRational::new(numer, BigInt::one() << shift)
    }
}

/// Returns the smallest representable f64 ≥ r (directed upward).
///
/// Uses exact comparison: converts the nearest-f64 candidate back to
/// `BigRational` and checks; if the candidate is too small, returns
/// `candidate.next_up()`.
#[must_use]
pub(super) fn rat_to_f64_up(r: &BigRational) -> f64 {
    if r.is_zero() {
        return 0.0_f64;
    }
    // to_f64() gives nearest f64 (rounds to nearest even).
    let Some(candidate) = r.to_f64() else {
        return if r.is_positive() {
            f64::INFINITY
        } else {
            -f64::MAX
        };
    };
    if !candidate.is_finite() {
        return if r.is_positive() {
            f64::INFINITY
        } else {
            -f64::MAX
        };
    }
    // Exact comparison: convert candidate back to BigRational.
    let c_rat = f64_to_rat(candidate);
    if c_rat >= *r {
        candidate
    } else {
        candidate.next_up()
    }
}

/// Returns the largest representable f64 ≤ r (directed downward).
#[must_use]
pub(super) fn rat_to_f64_down(r: &BigRational) -> f64 {
    if r.is_zero() {
        return 0.0_f64;
    }
    let Some(candidate) = r.to_f64() else {
        return if r.is_negative() {
            f64::NEG_INFINITY
        } else {
            f64::MAX
        };
    };
    if !candidate.is_finite() {
        return if r.is_negative() {
            f64::NEG_INFINITY
        } else {
            f64::MAX
        };
    }
    let c_rat = f64_to_rat(candidate);
    if c_rat <= *r {
        candidate
    } else {
        candidate.next_down()
    }
}

/// Returns the nearest f64 to r (round-to-nearest-even tie-breaking).
///
/// Used for midpoints where the direction of rounding does not affect
/// certified bounds.
#[must_use]
pub(super) fn rat_to_f64(r: &BigRational) -> f64 {
    r.to_f64().unwrap_or(if r.is_positive() {
        f64::INFINITY
    } else {
        f64::NEG_INFINITY
    })
}

fn bigint_isqrt(n: &BigInt) -> BigInt {
    if n.is_zero() {
        return BigInt::zero();
    }
    #[allow(clippy::cast_possible_truncation)]
    let mut x = BigInt::one() << (n.bits().div_ceil(2) as usize);
    loop {
        let x1 = (&x + n / &x) >> 1usize;
        if x1 >= x {
            break;
        }
        x = x1;
    }
    x
}

fn sqrt_search_up(sq_rat: &BigRational, lower_hint: &BigRational, upper_hint: &BigRational) -> f64 {
    let mut lo = rat_to_f64_down(lower_hint).to_bits();
    let mut hi = rat_to_f64_up(upper_hint).to_bits();
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        let candidate = f64::from_bits(mid);
        let candidate_rat = f64_to_rat(candidate);
        let candidate_sq = &candidate_rat * &candidate_rat;
        if candidate_sq >= *sq_rat {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    f64::from_bits(lo)
}

fn sqrt_search_down(
    sq_rat: &BigRational,
    lower_hint: &BigRational,
    upper_hint: &BigRational,
) -> f64 {
    let mut lo = rat_to_f64_down(lower_hint).to_bits();
    let mut hi = rat_to_f64_up(upper_hint).to_bits();
    while lo < hi {
        let mid = lo + (hi - lo).div_ceil(2);
        let candidate = f64::from_bits(mid);
        let candidate_rat = f64_to_rat(candidate);
        let candidate_sq = &candidate_rat * &candidate_rat;
        if candidate_sq <= *sq_rat {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    f64::from_bits(lo)
}

/// Returns a certified upper bound on √`sq_rat` (the non-negative square root).
///
/// Since IEEE 754 mandates correctly-rounded sqrt (§5.4), `fl(√x).next_up()`
/// is guaranteed to be ≥ √x for any finite non-negative x.
///
/// # Errors
///
/// Returns `Err(())` when `sq_rat` is negative (caller error).
pub(super) fn sqrt_up(sq_rat: &BigRational) -> Result<f64, ()> {
    if sq_rat.is_negative() {
        return Err(());
    }
    if sq_rat.is_zero() {
        return Ok(0.0_f64);
    }
    let max = f64_to_rat(f64::MAX);
    let max_sq = &max * &max;
    if sq_rat > &max_sq {
        return Ok(f64::INFINITY);
    }
    // Fast host-sqrt path: only valid when the radicand lies in the *normal*
    // f64 range (sq_rat >= MIN_POSITIVE).  Below MIN_POSITIVE the directed
    // conversions rat_to_f64_{down,up} land on adjacent subnormals whose square
    // roots differ by a large relative factor.  Concrete counterexample:
    // sq_rat = 3·2^{-1075} rounds down to 2^{-1074} (minsub) and up to
    // 2^{-1073} (2·minsub); sqrt(minsub) = 2^{-537} and sqrt(2·minsub) =
    // √2·2^{-537} differ by ~41%, so a single next_up cannot close the bracket
    // around the true root √3·2^{-537.5}.  Route every subnormal-range radicand
    // to the exact BigInt isqrt path, which yields a tight adjacent bound.
    if sq_rat <= &max && sq_rat >= &f64_to_rat(f64::MIN_POSITIVE) {
        let sq_hi = rat_to_f64_up(sq_rat);
        let candidate = sq_hi.sqrt();
        let candidate_rat = f64_to_rat(candidate);
        let candidate_sq = &candidate_rat * &candidate_rat;
        return Ok(if candidate_sq >= *sq_rat {
            candidate
        } else {
            candidate.next_up()
        });
    }

    // BigInt isqrt path: handles both huge radicands (sq_rat > MAX) and
    // subnormal-range radicands (rat_to_f64_down(sq_rat) == 0).
    let p = sq_rat.numer();
    let q = sq_rat.denom();
    let isqrt_p = bigint_isqrt(p);
    let isqrt_q = bigint_isqrt(q);
    let p_is_square = &isqrt_p * &isqrt_p == *p;
    let q_is_square = &isqrt_q * &isqrt_q == *q;
    if p_is_square && q_is_square {
        return Ok(rat_to_f64_up(&BigRational::new_raw(isqrt_p, isqrt_q)));
    }

    let upper_rat = BigRational::new_raw(isqrt_p.clone() + BigInt::one(), isqrt_q.clone());
    let lower_rat = BigRational::new_raw(isqrt_p, isqrt_q + BigInt::one());
    Ok(sqrt_search_up(sq_rat, &lower_rat, &upper_rat))
}

/// Returns a certified lower bound on √`sq_rat`.
///
/// # Errors
///
/// Returns `Err(())` when `sq_rat` is negative (caller error).
pub(super) fn sqrt_down(sq_rat: &BigRational) -> Result<f64, ()> {
    if sq_rat.is_negative() {
        return Err(());
    }
    if sq_rat.is_zero() {
        return Ok(0.0_f64);
    }
    let max = f64_to_rat(f64::MAX);
    let max_sq = &max * &max;
    if sq_rat > &max_sq {
        return Ok(f64::MAX);
    }
    // Fast host-sqrt path: valid only in the *normal* f64 range
    // (sq_rat >= MIN_POSITIVE).  Subnormal-range radicands are routed to the
    // exact BigInt isqrt path (see the sqrt_up counterexample for why directed
    // conversions on subnormals cannot produce an adjacent bracket).
    if sq_rat <= &max && sq_rat >= &f64_to_rat(f64::MIN_POSITIVE) {
        let sq_lo = rat_to_f64_down(sq_rat);
        let candidate = sq_lo.sqrt();
        let candidate_rat = f64_to_rat(candidate);
        let candidate_sq = &candidate_rat * &candidate_rat;
        return Ok(if candidate_sq <= *sq_rat {
            candidate
        } else {
            candidate.next_down()
        });
    }

    // BigInt isqrt path: handles both huge radicands (sq_rat > MAX) and
    // subnormal-range radicands (rat_to_f64_down(sq_rat) == 0).
    let p = sq_rat.numer();
    let q = sq_rat.denom();
    let isqrt_p = bigint_isqrt(p);
    let isqrt_q = bigint_isqrt(q);
    let p_is_square = &isqrt_p * &isqrt_p == *p;
    let q_is_square = &isqrt_q * &isqrt_q == *q;
    if p_is_square && q_is_square {
        return Ok(rat_to_f64_down(&BigRational::new_raw(isqrt_p, isqrt_q)));
    }

    let upper_rat = BigRational::new_raw(isqrt_p.clone() + BigInt::one(), isqrt_q.clone());
    let lower_rat = BigRational::new_raw(isqrt_p, isqrt_q + BigInt::one());
    Ok(sqrt_search_down(sq_rat, &lower_rat, &upper_rat))
}

#[cfg(test)]
mod tests {
    // Exact equality is the point of these tests: they verify certified,
    // bit-exact f64<->BigRational round trips and directed-rounding
    // boundaries, not approximate floating-point results.
    #![allow(clippy::float_cmp)]

    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::One;

    use super::{f64_to_rat, rat_to_f64, rat_to_f64_down, rat_to_f64_up, sqrt_down, sqrt_up};

    #[test]
    fn f64_to_rat_round_trip() {
        let xs = [
            0.0_f64,
            1.0,
            -1.0,
            2.0_f64.powi(-52),
            f64::MAX,
            f64::MIN_POSITIVE * 0.5,
        ];
        for x in xs {
            let r = f64_to_rat(x);
            let back = rat_to_f64(&r);
            assert_eq!(back, x, "round trip failed for {x}");
        }
    }

    #[test]
    fn rat_to_f64_up_is_upper_bound() {
        use num_rational::BigRational;
        let r = BigRational::new(BigInt::one(), BigInt::from(3i64));
        let up = rat_to_f64_up(&r);
        let up_rat = f64_to_rat(up);
        assert!(up_rat >= r, "rat_to_f64_up is not an upper bound");

        let nearest = rat_to_f64(&r);
        assert!(up <= nearest.next_up());
    }

    #[test]
    fn rat_to_f64_down_is_lower_bound() {
        let r = BigRational::new(BigInt::one(), BigInt::from(3i64));
        let down = rat_to_f64_down(&r);
        let down_rat = f64_to_rat(down);
        assert!(down_rat <= r, "rat_to_f64_down is not a lower bound");
    }

    #[test]
    fn rat_to_f64_up_large_negative_returns_neg_max() {
        let max = f64_to_rat(f64::MAX);
        let r = -(&max * BigRational::from_integer(BigInt::from(2i64)));
        assert_eq!(rat_to_f64_up(&r), -f64::MAX);
    }

    #[test]
    fn rat_to_f64_down_large_positive_returns_max() {
        let max = f64_to_rat(f64::MAX);
        let r = &max * BigRational::from_integer(BigInt::from(2i64));
        assert_eq!(rat_to_f64_down(&r), f64::MAX);
    }

    #[test]
    fn sqrt_up_and_down_bracket_true_root() {
        let two = f64_to_rat(2.0);
        let up = sqrt_up(&two).unwrap();
        let down = sqrt_down(&two).unwrap();
        assert!(down <= std::f64::consts::SQRT_2);
        assert!(up >= std::f64::consts::SQRT_2);
        assert!(down <= up);
    }

    #[test]
    fn sqrt_up_of_zero_is_zero() {
        assert_eq!(sqrt_up(&f64_to_rat(0.0)).unwrap(), 0.0);
        assert_eq!(sqrt_down(&f64_to_rat(0.0)).unwrap(), 0.0);
    }

    #[test]
    fn sqrt_up_huge_radicand_returns_finite() {
        let sq_rat = BigRational::from_integer(BigInt::one() << 1100usize);
        let up = sqrt_up(&sq_rat).unwrap();
        assert!(up.is_finite());
        let up_rat = f64_to_rat(up);
        let up_sq = &up_rat * &up_rat;
        assert!(up_sq >= sq_rat);
    }

    #[test]
    fn sqrt_down_huge_radicand() {
        let sq_rat = BigRational::from_integer(BigInt::one() << 1100usize);
        let down = sqrt_down(&sq_rat).unwrap();
        assert!(down.is_finite());
        let down_rat = f64_to_rat(down);
        let down_sq = &down_rat * &down_rat;
        assert!(down_sq <= sq_rat);
    }

    #[test]
    fn sqrt_rejects_negative() {
        assert!(sqrt_up(&f64_to_rat(-1.0)).is_err());
        assert!(sqrt_down(&f64_to_rat(-1.0)).is_err());
    }

    /// Regression: subnormal-range radicand.
    ///
    /// `sq_rat` = (1e-200)² = 1e-400, which is below f64 minsub ≈ 5e-324.
    /// True sqrt is 1e-200 (a normal f64). The fast host-sqrt path previously
    /// returned sqrt(minsub) ≈ 2.22e-162 as `sqrt_up` — wildly too large.
    /// With the `BigInt` isqrt path the result must bracket 1e-200 tightly.
    #[test]
    fn sqrt_tiny_radicand_subnormal_range() {
        let base = f64_to_rat(1e-200_f64);
        let sq_rat = &base * &base; // exact rational for (1e-200)²

        let up = sqrt_up(&sq_rat).unwrap();
        let down = sqrt_down(&sq_rat).unwrap();

        // Both must bracket the true root.
        assert!(
            down <= 1e-200_f64,
            "sqrt_down should be ≤ 1e-200, got {down}"
        );
        assert!(up >= 1e-200_f64, "sqrt_up should be ≥ 1e-200, got {up}");

        // sqrt_up must NOT be the wrong fast-path result (sqrt(minsub) ≈ 2.22e-162).
        // The wildly-wrong path returns a value ≈ 2.22e-162 >> 1e-200.
        assert!(
            up < 1e-160_f64,
            "sqrt_up regressed to the loose minsub-sqrt path: {up}"
        );

        // Verify the corrected endpoints are each valid bounds.
        let down_rat = f64_to_rat(down);
        let up_rat = f64_to_rat(up);
        let down_sq = &down_rat * &down_rat;
        let up_sq = &up_rat * &up_rat;
        assert!(down_sq <= sq_rat, "sqrt_down² must be ≤ sq_rat");
        assert!(up_sq >= sq_rat, "sqrt_up² must be ≥ sq_rat");
    }

    /// Regression: minsub as radicand (subnormal root boundary).
    ///
    /// minsub = 2^{-1074}; sqrt(minsub) = 2^{-537} is representable as f64.
    /// Both paths must return finite, tight bounds.
    #[test]
    fn sqrt_minsub_radicand() {
        let minsub = f64::from_bits(1); // 2^{-1074}
        let sq_rat = f64_to_rat(minsub);

        let up = sqrt_up(&sq_rat).unwrap();
        let down = sqrt_down(&sq_rat).unwrap();

        let true_root = minsub.sqrt(); // 2^{-537}, finite normal f64
        assert!(
            down <= true_root,
            "sqrt_down should be ≤ sqrt(minsub), got {down}"
        );
        assert!(
            up >= true_root,
            "sqrt_up should be ≥ sqrt(minsub), got {up}"
        );
        assert!(up.is_finite(), "sqrt_up(minsub) must be finite");
        assert!(down >= 0.0, "sqrt_down(minsub) must be non-negative");
    }

    /// Regression: root-overflow — radicand > MAX² returns Inf / MAX.
    #[test]
    fn sqrt_root_overflow_inf() {
        let sq_rat = BigRational::from_integer(BigInt::one() << 2200usize);
        assert_eq!(sqrt_up(&sq_rat).unwrap(), f64::INFINITY);
        assert_eq!(sqrt_down(&sq_rat).unwrap(), f64::MAX);
    }

    /// Regression (Correction 10-A): subnormal radicand 3·2^{-1075}.
    ///
    /// `rat_to_f64_down(3·2^{-1075}) = 2^{-1074}` (minsub, strictly positive),
    /// so the old `sq_lo > 0.0` fast-path fired and `sqrt(minsub) = 2^{-537}`
    /// left a ~41% gap around the true root √3·2^{-537.5}.  Gating on
    /// `MIN_POSITIVE` routes it to the exact `BigInt` path, which must produce an
    /// adjacent bracket: `sqrt_up == sqrt_down.next_up()`.
    #[test]
    fn sqrt_regression_subnormal_3_times_2_neg_1075() {
        let sq_rat = BigRational::new(BigInt::from(3i64), BigInt::one() << 1075usize);
        let up = sqrt_up(&sq_rat).unwrap();
        let down = sqrt_down(&sq_rat).unwrap();

        let down_rat = f64_to_rat(down);
        let up_rat = f64_to_rat(up);
        assert!(
            &down_rat * &down_rat <= sq_rat,
            "sqrt_down² must be ≤ sq_rat"
        );
        assert!(&up_rat * &up_rat >= sq_rat, "sqrt_up² must be ≥ sq_rat");
        assert_eq!(
            down.next_up(),
            up,
            "bracket must be adjacent: sqrt_down.next_up() == sqrt_up"
        );
    }

    /// Regression (Correction 10-A): radicands straddling `MIN_POSITIVE`.
    #[test]
    fn sqrt_regression_around_min_positive() {
        let min_pos = f64_to_rat(f64::MIN_POSITIVE);
        let cases = [
            rat_to_f64_down(&min_pos).next_down(), // just below MIN_POSITIVE
            f64::MIN_POSITIVE,
            f64::MIN_POSITIVE.next_up(),
        ]
        .map(f64_to_rat);
        for sq_rat in cases {
            let up = sqrt_up(&sq_rat).unwrap();
            let down = sqrt_down(&sq_rat).unwrap();
            let down_rat = f64_to_rat(down);
            let up_rat = f64_to_rat(up);
            assert!(
                &down_rat * &down_rat <= sq_rat,
                "sqrt_down² must be ≤ sq_rat"
            );
            assert!(&up_rat * &up_rat >= sq_rat, "sqrt_up² must be ≥ sq_rat");
            assert!(
                down == up || down.next_up() == up,
                "bracket must be exact or adjacent, got down={down} up={up}"
            );
        }
    }

    /// Regression (Correction 10-A): root far below minsub.
    ///
    /// `sq_rat = minsub / 4` has true root `2^{-538}` which is itself a
    /// subnormal f64; the exact path must bracket it adjacently.
    #[test]
    fn sqrt_regression_tiny_below_minsub() {
        let minsub_rat = f64_to_rat(f64::from_bits(1));
        let sq_rat = &minsub_rat / BigRational::from_integer(BigInt::from(4i64));
        let up = sqrt_up(&sq_rat).unwrap();
        let down = sqrt_down(&sq_rat).unwrap();
        let down_rat = f64_to_rat(down);
        let up_rat = f64_to_rat(up);
        assert!(
            &down_rat * &down_rat <= sq_rat,
            "sqrt_down² must be ≤ sq_rat"
        );
        assert!(&up_rat * &up_rat >= sq_rat, "sqrt_up² must be ≥ sq_rat");
        assert!(
            down == up || down.next_up() == up,
            "bracket must be exact or adjacent, got down={down} up={up}"
        );
    }
}
