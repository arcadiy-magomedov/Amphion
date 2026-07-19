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
        let shift =
            usize::try_from(power).expect("f64 exponent shift is bounded and non-negative here");
        BigRational::from_integer(numer << shift)
    } else {
        let shift =
            usize::try_from(-power).expect("f64 exponent shift is bounded and non-negative here");
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
            f64::NEG_INFINITY
        };
    };
    if !candidate.is_finite() {
        return if r.is_positive() {
            f64::INFINITY
        } else {
            candidate
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
            f64::INFINITY
        };
    };
    if !candidate.is_finite() {
        return if r.is_negative() {
            f64::NEG_INFINITY
        } else {
            candidate
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
    // Round sq_rat itself upward first (sqrt is monotone increasing):
    let sq_hi = rat_to_f64_up(sq_rat);
    if sq_hi == f64::INFINITY {
        return Ok(f64::INFINITY);
    }
    // sqrt(sq_hi) is correctly rounded → it is the nearest f64 to √sq_hi.
    let sqrt_hi = sq_hi.sqrt();
    // Since sqrt_hi = fl(√sq_hi) is within 0.5 ULP of the true √sq_hi
    // (IEEE 754 correctly-rounded guarantee), and sq_hi ≥ sq_rat, we have
    // √sq_hi ≥ √sq_rat; one further next_up() absorbs the ≤0.5 ULP rounding
    // gap, certifying sqrt_hi.next_up() ≥ √sq_rat.
    Ok(sqrt_hi.next_up())
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
    let sq_lo = rat_to_f64_down(sq_rat);
    if sq_lo <= 0.0_f64 {
        return Ok(0.0_f64);
    }
    let sqrt_lo = sq_lo.sqrt();
    // sqrt_lo = fl(√sq_lo) is within 0.5 ULP of the true √sq_lo, and
    // sq_lo ≤ sq_rat, so sqrt_lo.next_down() ≤ √sq_rat (conservative).
    Ok(sqrt_lo.next_down())
}

#[cfg(test)]
mod tests {
    // Exact equality is the point of these tests: they verify certified,
    // bit-exact f64<->BigRational round trips and directed-rounding
    // boundaries, not approximate floating-point results.
    #![allow(clippy::float_cmp)]

    use num_bigint::BigInt;
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
        use num_rational::BigRational;
        let r = BigRational::new(BigInt::one(), BigInt::from(3i64));
        let down = rat_to_f64_down(&r);
        let down_rat = f64_to_rat(down);
        assert!(down_rat <= r, "rat_to_f64_down is not a lower bound");
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
    fn sqrt_rejects_negative() {
        assert!(sqrt_up(&f64_to_rat(-1.0)).is_err());
        assert!(sqrt_down(&f64_to_rat(-1.0)).is_err());
    }
}
