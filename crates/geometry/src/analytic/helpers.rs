//! Private arithmetic helpers.
//!
//! # Certified rational-arithmetic backend
//!
//! Every certified distance, position, and parameter bound in this crate is
//! now derived from **exact** `BigRational` arithmetic (see
//! [`super::exact`]) rather than a fixed Higham-style floating-point error
//! constant. For computations built entirely from IEEE 754 basic operations
//! (add, sub, mul, div, sqrt — all correctly rounded per IEEE 754-2008/2019
//! §5.4), decoding every input `f64` exactly as a dyadic rational and
//! recombining in exact rational arithmetic removes floating-point rounding
//! error from the *bound* entirely: the only remaining approximation is the
//! final, directed (never-underestimating) rounding of the exact rational
//! result back to `f64`, performed by [`super::exact::sqrt_up`] and
//! [`super::exact::rat_to_f64_up`]/[`super::exact::rat_to_f64_down`].
//!
//! This closes two floating-point cancellation failure modes that a
//! constant-factor Higham bound cannot: (1) when the *true* residual is
//! itself near the smallest representable positive `f64` scale (see the
//! `minsub` regression test in `line.rs`), a constant-factor bound computed
//! from a coarse world-coordinate `scale` can be swamped by rounding in its
//! own computation; and (2) when intermediate subtraction cancels most of
//! the significant digits of a `query` far from a primitive's local frame
//! (see the `cancellation` regression test in `line.rs`), the *reported*
//! floating-point position can differ non-negligibly from the true nearest
//! point, which a Higham bound derived from the (already-cancelled) `f64`
//! residual cannot detect.
//!
//! Angles (returned parameters for `Circle2`/`Circle3`/`Cylinder`/`Cone`)
//! additionally require a transcendental (`atan2`) evaluation; those are
//! computed via the certified rational-interval backend in
//! [`super::trig`], never via any `f64` trigonometric function.
//!
//! Component-wise coordinate arithmetic below conventionally uses short
//! mathematical names (`x`/`y`/`z`, `u`/`v`, `t`, `o`, `d`, `p`, `q`, and
//! their per-axis/per-derivative variants such as `px`/`py`/`pz` or
//! `d1x`/`d1y`) to mirror the geometric formulas in the doc comments; the
//! pedantic `similar_names`/`many_single_char_names` lints are disabled for
//! this reason.
#![allow(clippy::similar_names, clippy::many_single_char_names)]

use amphion_foundation::{NormalizationError, ToleranceContext, UnitVector3, Vector3};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

use crate::{CertificationBudget, GeometryError, ParameterRange};

use super::ConstructionError;
use super::exact::{f64_to_rat, rat_to_f64, rat_to_f64_down, rat_to_f64_up, sqrt_down, sqrt_up};
use super::trig::{RatInterval, TrigError, atan2_interval, sin_cos_interval, tau_interval};

/// Converts a foundation [`NormalizationError`] into the analytic
/// geometry crate's own [`ConstructionError`].
///
/// `NonFinite` and `NotNormalized` both indicate the supplied vector cannot
/// be trusted as-is, which this crate reports as
/// [`ConstructionError::NonFiniteInput`]; `ZeroMagnitude` maps to
/// [`ConstructionError::DegenerateAxis`], matching the prior local
/// `normalize2`/`normalize3` behavior of returning `None` only for the zero
/// vector.
pub(super) fn normalization_to_construction(err: NormalizationError) -> ConstructionError {
    match err {
        NormalizationError::NonFinite | NormalizationError::NotNormalized => {
            ConstructionError::NonFiniteInput
        }
        NormalizationError::ZeroMagnitude => ConstructionError::DegenerateAxis,
    }
}

/// Derives a unit cross-product direction without changing the stored input
/// vectors. Each input is normalized scale-safely only for this derived cache.
pub(super) fn normalized_cross3(
    first: Vector3,
    second: Vector3,
) -> Result<UnitVector3, ConstructionError> {
    let preserve_or_normalize = |seed: Vector3| match UnitVector3::try_from(seed.into_array()) {
        Ok(unit) => Ok(unit),
        Err(NormalizationError::NotNormalized) => {
            UnitVector3::try_normalize(seed).map_err(normalization_to_construction)
        }
        Err(error) => Err(normalization_to_construction(error)),
    };
    let first_unit = preserve_or_normalize(first)?;
    let second_unit = preserve_or_normalize(second)?;
    UnitVector3::try_normalize(first_unit.cross(second_unit))
        .map_err(|_| ConstructionError::DependentAxes)
}

/// Ill-conditioning threshold for Gram-Schmidt orthogonalization.  If the
/// component of the supplied x-axis perpendicular to the main axis has
/// magnitude below this value (`16 · √ε ≈ 2.4e-7`), the normalization would
/// amplify rounding errors by a factor of `> 1/√ε ≈ 6.7e7` and the result
/// would be unreliable.
pub(super) const ILL_COND_THRESH: f64 = 2.384_185_791_015_625e-7;

// ─── 3-D helpers ────────────────────────────────────────────────────────────

pub(super) fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(super) fn mag3(v: [f64; 3]) -> f64 {
    let scale = v[0].abs().max(v[1].abs()).max(v[2].abs());
    if scale == 0.0 {
        0.0
    } else {
        let vs = [v[0] / scale, v[1] / scale, v[2] / scale];
        scale * (vs[0] * vs[0] + vs[1] * vs[1] + vs[2] * vs[2]).sqrt()
    }
}

pub(super) fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub(super) fn scale3(v: [f64; 3], s: f64) -> [f64; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

pub(super) fn all_finite3(v: [f64; 3]) -> bool {
    v[0].is_finite() && v[1].is_finite() && v[2].is_finite()
}

// ─── 2-D helpers ────────────────────────────────────────────────────────────

/// Rotates `v` by 90° CCW, giving the perpendicular direction.
pub(super) fn perp2(v: [f64; 2]) -> [f64; 2] {
    [-v[1], v[0]]
}

pub(super) fn all_finite2(v: [f64; 2]) -> bool {
    v[0].is_finite() && v[1].is_finite()
}

// ─── Domain helpers ──────────────────────────────────────────────────────────

/// Returns `true` when `t` is finite and inside the inclusive declared bounds.
///
/// A full-period upper endpoint is accepted as a seam alias and canonicalized
/// separately by [`canonicalize_periodic_endpoint`].
pub(super) fn in_range(t: f64, range: ParameterRange) -> bool {
    if !t.is_finite() {
        return false;
    }
    let lo_ok = range.lower().is_none_or(|lo| t >= lo);
    let hi_ok = range.upper().is_none_or(|hi| t <= hi);
    lo_ok && hi_ok
}

/// Maps the exact upper endpoint of a full-period range to its lower seam.
///
/// Projection results remain canonical in `[lower, upper)`; this alias exists
/// only so closed trimming intervals can evaluate both endpoints.
pub(super) fn canonicalize_periodic_endpoint(t: f64, range: ParameterRange) -> f64 {
    match (range.lower(), range.upper(), range.period()) {
        (Some(lower), Some(upper), Some(period))
            if t.to_bits() == upper.to_bits() && (upper - lower).to_bits() == period.to_bits() =>
        {
            lower
        }
        _ => t,
    }
}

// ─── Certified rational-arithmetic helpers ─────────────────────────────────

// `sqrt_up`/`sqrt_down` compare squares of exact finite-f64 candidates. The
// widest raw f64 denominator is 2^1074 (1075 bits), so its square needs 2149
// bits; three guard bits cover the integer-square and Newton-step additions.
const CERTIFIED_SQRT_WORK_BITS: u64 = 2_152;
// num-rational 0.4.2 may left-shift a cloned numerator or denominator by up
// to 56 bits while converting a rational to f64.
const RATIONAL_TO_F64_SHIFT_BITS: u64 = 56;

fn rational_budget_exhausted() -> GeometryError {
    GeometryError::Uncertified {
        reason: "intermediate exact-rational value exceeded the certification bit-width budget"
            .to_owned(),
    }
}

/// Deterministic pre-allocation cap for the exact algebra outside the
/// transcendental backend.
///
/// Every arithmetic method checks a conservative upper bound on the raw
/// numerator and denominator sizes before invoking `num-rational`. Reduction
/// can only shrink those values, so this may reject some cancellation-heavy
/// computations early but never allocates an intermediate wider than the
/// caller's cap.
#[derive(Clone, Copy)]
struct RationalCap {
    bits: u64,
}

impl RationalCap {
    fn new(budget: CertificationBudget) -> Self {
        Self {
            bits: u64::from(budget.rational_bits()),
        }
    }

    fn within(self, bound: u64) -> Result<(), GeometryError> {
        if bound > self.bits {
            Err(rational_budget_exhausted())
        } else {
            Ok(())
        }
    }

    fn admit(self, value: &BigRational) -> Result<(), GeometryError> {
        self.within(value.numer().bits())?;
        self.within(value.denom().bits())
    }

    fn admitted(self, value: BigRational) -> Result<BigRational, GeometryError> {
        self.admit(&value)?;
        Ok(value)
    }

    fn guard_f64_decode(self, value: f64) -> Result<(), GeometryError> {
        debug_assert!(value.is_finite());
        if value == 0.0 {
            self.within(1)?;
            return Ok(());
        }

        let bits = value.to_bits();
        let biased_exp = ((bits >> 52) & 0x7ff) as i32;
        let mantissa = bits & 0x000f_ffff_ffff_ffff;
        let (significand, power) = if biased_exp == 0 {
            (mantissa, -1074)
        } else {
            ((1_u64 << 52) | mantissa, biased_exp - 1075)
        };
        let significand_bits = u64::from(u64::BITS - significand.leading_zeros());
        if power >= 0 {
            #[allow(clippy::cast_sign_loss)]
            self.within(significand_bits.saturating_add(power as u64))?;
            self.within(1)?;
        } else {
            self.within(significand_bits)?;
            #[allow(clippy::cast_sign_loss)]
            self.within((-power) as u64 + 1)?;
        }
        Ok(())
    }

    fn decode_f64(self, value: f64) -> Result<BigRational, GeometryError> {
        self.guard_f64_decode(value)?;
        Ok(f64_to_rat(value))
    }

    fn integer(self, value: i64) -> Result<BigRational, GeometryError> {
        let magnitude = value.unsigned_abs();
        let bits = if magnitude == 0 {
            0
        } else {
            u64::from(u64::BITS - magnitude.leading_zeros())
        };
        self.within(bits)?;
        self.within(1)?;
        Ok(BigRational::from_integer(BigInt::from(value)))
    }

    fn guard_addsub(self, lhs: &BigRational, rhs: &BigRational) -> Result<(), GeometryError> {
        let numerator = lhs
            .numer()
            .bits()
            .saturating_add(rhs.denom().bits())
            .max(lhs.denom().bits().saturating_add(rhs.numer().bits()))
            .saturating_add(1);
        self.within(numerator)?;
        self.within(lhs.denom().bits().saturating_add(rhs.denom().bits()))
    }

    fn guard_mul(self, lhs: &BigRational, rhs: &BigRational) -> Result<(), GeometryError> {
        self.within(lhs.numer().bits().saturating_add(rhs.numer().bits()))?;
        self.within(lhs.denom().bits().saturating_add(rhs.denom().bits()))
    }

    fn guard_div(self, lhs: &BigRational, rhs: &BigRational) -> Result<(), GeometryError> {
        if rhs.is_zero() {
            return Err(GeometryError::Uncertified {
                reason: "exact-rational division by zero".to_owned(),
            });
        }
        self.within(lhs.numer().bits().saturating_add(rhs.denom().bits()))?;
        self.within(lhs.denom().bits().saturating_add(rhs.numer().bits()))
    }

    fn add(self, lhs: &BigRational, rhs: &BigRational) -> Result<BigRational, GeometryError> {
        self.guard_addsub(lhs, rhs)?;
        self.admitted(lhs + rhs)
    }

    fn sub(self, lhs: &BigRational, rhs: &BigRational) -> Result<BigRational, GeometryError> {
        self.guard_addsub(lhs, rhs)?;
        self.admitted(lhs - rhs)
    }

    fn mul(self, lhs: &BigRational, rhs: &BigRational) -> Result<BigRational, GeometryError> {
        self.guard_mul(lhs, rhs)?;
        self.admitted(lhs * rhs)
    }

    fn div(self, lhs: &BigRational, rhs: &BigRational) -> Result<BigRational, GeometryError> {
        self.guard_div(lhs, rhs)?;
        self.admitted(lhs / rhs)
    }

    fn neg(self, value: &BigRational) -> Result<BigRational, GeometryError> {
        self.admit(value)?;
        Ok(-value)
    }

    fn rounding_error_bound(self, exact: &BigRational) -> Result<(f64, f64), GeometryError> {
        self.admit(exact)?;
        self.guard_to_f64(exact)?;
        let candidate = rat_to_f64(exact);
        if candidate.is_finite() {
            self.guard_f64_decode(candidate)?;
        }
        Ok(rounding_error_bound_unchecked(exact))
    }

    fn sqrt_up(self, radicand: &BigRational) -> Result<f64, GeometryError> {
        self.admit(radicand)?;
        if radicand.is_zero() {
            return Ok(0.0);
        }
        self.guard_to_f64(radicand)?;
        self.within(CERTIFIED_SQRT_WORK_BITS)?;
        sqrt_up(radicand).map_err(|()| GeometryError::Uncertified {
            reason: "certified square-root radicand is negative".to_owned(),
        })
    }

    fn sqrt_down(self, radicand: &BigRational) -> Result<f64, GeometryError> {
        self.admit(radicand)?;
        if radicand.is_zero() {
            return Ok(0.0);
        }
        self.guard_to_f64(radicand)?;
        self.within(CERTIFIED_SQRT_WORK_BITS)?;
        sqrt_down(radicand).map_err(|()| GeometryError::Uncertified {
            reason: "certified square-root radicand is negative".to_owned(),
        })
    }

    fn rat_to_f64_up(self, value: &BigRational) -> Result<f64, GeometryError> {
        self.admit(value)?;
        self.guard_to_f64(value)?;
        let candidate = rat_to_f64(value);
        if candidate.is_finite() {
            self.guard_f64_decode(candidate)?;
        }
        Ok(rat_to_f64_up(value))
    }

    fn guard_to_f64(self, value: &BigRational) -> Result<(), GeometryError> {
        let source_bits = value.numer().bits().max(value.denom().bits());
        self.within(source_bits.saturating_add(RATIONAL_TO_F64_SHIFT_BITS))
    }
}

/// Rejects a `BigRational` whose numerator or denominator bit-width exceeds
/// the certification budget.
pub(super) fn check_rational_budget(
    budget: CertificationBudget,
    r: &BigRational,
) -> Result<(), GeometryError> {
    RationalCap::new(budget).admit(r)
}

/// Rounds an exact `BigRational` value to the nearest `f64` and returns a
/// certified (safe, outward) bound on `|rounded − exact|`.
///
/// The bound is the full width of the bracket `[rat_to_f64_down(exact),
/// rat_to_f64_up(exact)]`, which always contains both `exact` and the
/// nearest-rounded `f64`, so it safely (if slightly loosely, by at most a
/// factor of two relative to the tightest possible half-ULP bound) bounds
/// the rounding error.
fn rounding_error_bound_unchecked(exact: &BigRational) -> (f64, f64) {
    let rounded = rat_to_f64(exact);
    let up = rat_to_f64_up(exact);
    let down = rat_to_f64_down(exact);
    let width = (up - down).max(0.0);
    let bound = if width == 0.0 { 0.0 } else { width.next_up() };
    (rounded, bound)
}

/// Combines two independent, non-negative axis error bounds into a single
/// certified Euclidean bound.
///
/// The inputs are decoded as exact dyadic rationals, squared and summed in
/// rational arithmetic, then rounded outward with [`sqrt_up`].  Do not use
/// `f64::hypot` here: the component certificates themselves are proof
/// quantities and their final norm needs the same directed-rounding treatment
/// as geometric distances.
fn combine2_bounds(budget: CertificationBudget, a: f64, b: f64) -> Result<f64, GeometryError> {
    combine_bounds(budget, &[a, b])
}

/// 3-D analogue of [`combine2_bounds`].
fn combine3_bounds(
    budget: CertificationBudget,
    a: f64,
    b: f64,
    c: f64,
) -> Result<f64, GeometryError> {
    combine_bounds(budget, &[a, b, c])
}

fn combine_bounds(budget: CertificationBudget, bounds: &[f64]) -> Result<f64, GeometryError> {
    if bounds
        .iter()
        .any(|bound| !bound.is_finite() || *bound < 0.0)
    {
        return Err(GeometryError::Uncertified {
            reason: "component certificate bound is non-finite or negative".to_owned(),
        });
    }

    let cap = RationalCap::new(budget);
    let mut squared_sum = BigRational::zero();
    for bound in bounds {
        let exact = cap.decode_f64(*bound)?;
        let square = cap.mul(&exact, &exact)?;
        squared_sum = cap.add(&squared_sum, &square)?;
    }
    cap.sqrt_up(&squared_sum)
}

/// The result of an exact-rational affine evaluation `origin + t·direction`.
pub(super) struct ExactEvalResult<const N: usize> {
    pub point: [f64; N],
    pub position_error_bound: f64,
}

/// The result of an exact-rational line projection.
pub(super) struct ExactLineProjection<const N: usize> {
    pub parameter: f64,
    pub parameter_error_bound: f64,
    pub point: [f64; N],
    pub point_residual_bound: f64,
    pub distance_bound: f64,
}

/// The result of an exact-rational plane (two-parameter) projection.
pub(super) struct ExactPlaneProjection {
    pub u: f64,
    pub v: f64,
    pub parameter_error_bound: f64,
    pub point: [f64; 3],
    pub point_residual_bound: f64,
    pub distance_bound: f64,
}

fn to_rat2(cap: RationalCap, v: [f64; 2]) -> Result<[BigRational; 2], GeometryError> {
    Ok([cap.decode_f64(v[0])?, cap.decode_f64(v[1])?])
}

fn to_rat3(cap: RationalCap, v: [f64; 3]) -> Result<[BigRational; 3], GeometryError> {
    Ok([
        cap.decode_f64(v[0])?,
        cap.decode_f64(v[1])?,
        cap.decode_f64(v[2])?,
    ])
}

fn finite_or_uncertified(values: &[f64], reason: &str) -> Result<(), GeometryError> {
    if values.iter().all(|v| v.is_finite()) {
        Ok(())
    } else {
        Err(GeometryError::Uncertified {
            reason: reason.to_owned(),
        })
    }
}

/// Evaluates `origin + t·direction` in 2-D using exact rational arithmetic,
/// returning the nearest-`f64` point and a certified position error bound
/// (the rounding error incurred by returning an `f64` point rather than the
/// exact rational value).
///
/// # Errors
///
/// Returns [`GeometryError::Uncertified`] if the exact result overflows the
/// representable `f64` range, or the certification budget is exceeded.
pub(super) fn exact_affine_eval2(
    budget: CertificationBudget,
    origin: [f64; 2],
    direction: [f64; 2],
    t: f64,
) -> Result<ExactEvalResult<2>, GeometryError> {
    let cap = RationalCap::new(budget);
    let o = to_rat2(cap, origin)?;
    let d = to_rat2(cap, direction)?;
    let t_r = cap.decode_f64(t)?;
    let px = cap.add(&o[0], &cap.mul(&t_r, &d[0])?)?;
    let py = cap.add(&o[1], &cap.mul(&t_r, &d[1])?)?;
    let (x, ex) = cap.rounding_error_bound(&px)?;
    let (y, ey) = cap.rounding_error_bound(&py)?;
    finite_or_uncertified(&[x, y], "evaluated line position overflowed f64 range")?;
    Ok(ExactEvalResult {
        point: [x, y],
        position_error_bound: combine2_bounds(budget, ex, ey)?,
    })
}

/// 3-D analogue of [`exact_affine_eval2`].
pub(super) fn exact_affine_eval3(
    budget: CertificationBudget,
    origin: [f64; 3],
    direction: [f64; 3],
    t: f64,
) -> Result<ExactEvalResult<3>, GeometryError> {
    let cap = RationalCap::new(budget);
    let o = to_rat3(cap, origin)?;
    let d = to_rat3(cap, direction)?;
    let t_r = cap.decode_f64(t)?;
    let px = cap.add(&o[0], &cap.mul(&t_r, &d[0])?)?;
    let py = cap.add(&o[1], &cap.mul(&t_r, &d[1])?)?;
    let pz = cap.add(&o[2], &cap.mul(&t_r, &d[2])?)?;
    let (x, ex) = cap.rounding_error_bound(&px)?;
    let (y, ey) = cap.rounding_error_bound(&py)?;
    let (z, ez) = cap.rounding_error_bound(&pz)?;
    finite_or_uncertified(&[x, y, z], "evaluated line position overflowed f64 range")?;
    Ok(ExactEvalResult {
        point: [x, y, z],
        position_error_bound: combine3_bounds(budget, ex, ey, ez)?,
    })
}

/// Evaluates `origin + u·u_axis + v·v_axis` in 3-D using exact rational
/// arithmetic, returning the nearest-`f64` point and a certified position
/// error bound.
///
/// # Errors
///
/// Returns [`GeometryError::Uncertified`] if the exact result overflows the
/// representable `f64` range, or the certification budget is exceeded.
pub(super) fn exact_plane_eval3(
    budget: CertificationBudget,
    origin: [f64; 3],
    u_axis: [f64; 3],
    v_axis: [f64; 3],
    u: f64,
    v: f64,
) -> Result<ExactEvalResult<3>, GeometryError> {
    let cap = RationalCap::new(budget);
    let o = to_rat3(cap, origin)?;
    let ua = to_rat3(cap, u_axis)?;
    let va = to_rat3(cap, v_axis)?;
    let u_r = cap.decode_f64(u)?;
    let v_r = cap.decode_f64(v)?;
    let px = cap.add(
        &cap.add(&o[0], &cap.mul(&u_r, &ua[0])?)?,
        &cap.mul(&v_r, &va[0])?,
    )?;
    let py = cap.add(
        &cap.add(&o[1], &cap.mul(&u_r, &ua[1])?)?,
        &cap.mul(&v_r, &va[1])?,
    )?;
    let pz = cap.add(
        &cap.add(&o[2], &cap.mul(&u_r, &ua[2])?)?,
        &cap.mul(&v_r, &va[2])?,
    )?;
    let (x, ex) = cap.rounding_error_bound(&px)?;
    let (y, ey) = cap.rounding_error_bound(&py)?;
    let (z, ez) = cap.rounding_error_bound(&pz)?;
    finite_or_uncertified(&[x, y, z], "evaluated plane position overflowed f64 range")?;
    Ok(ExactEvalResult {
        point: [x, y, z],
        position_error_bound: combine3_bounds(budget, ex, ey, ez)?,
    })
}

/// Certified projection of `query` onto the line `origin + t·direction` in
/// 2-D, computed entirely in exact rational arithmetic.
///
/// The parameter is the true Euclidean projection
/// `t* = ((query − origin)·direction) / (direction·direction)`, which is
/// exact and well-defined regardless of whether `direction` is *exactly*
/// unit length (it need only be non-zero); this avoids relying on the
/// approximate unit-length guarantee of the stored direction vector.
///
/// `distance_bound` is a certified upper bound on the true Euclidean
/// distance from `query` to `origin + t*·direction` (not merely the
/// floating-point-rounded reported point), closing the cancellation and
/// minimum-subnormal-scale failure modes of the previous Higham-based
/// bound (see module documentation).
///
/// # Errors
///
/// Returns [`GeometryError::Uncertified`] if `direction` has exact zero
/// magnitude, any intermediate value overflows the representable `f64`
/// range, or the certification budget is exceeded.
pub(super) fn exact_line_project2(
    budget: CertificationBudget,
    query: [f64; 2],
    origin: [f64; 2],
    direction: [f64; 2],
) -> Result<ExactLineProjection<2>, GeometryError> {
    let cap = RationalCap::new(budget);
    let q = to_rat2(cap, query)?;
    let o = to_rat2(cap, origin)?;
    let d = to_rat2(cap, direction)?;
    let diff = [cap.sub(&q[0], &o[0])?, cap.sub(&q[1], &o[1])?];
    let dot_dd = cap.add(&cap.mul(&d[0], &d[0])?, &cap.mul(&d[1], &d[1])?)?;
    if dot_dd.is_zero() {
        return Err(GeometryError::Uncertified {
            reason: "line direction has exact zero magnitude".to_owned(),
        });
    }
    let dot_diff_d = cap.add(&cap.mul(&diff[0], &d[0])?, &cap.mul(&diff[1], &d[1])?)?;
    let t_exact = cap.div(&dot_diff_d, &dot_dd)?;
    let (t, t_err) = cap.rounding_error_bound(&t_exact)?;

    let proj = [
        cap.add(&o[0], &cap.mul(&t_exact, &d[0])?)?,
        cap.add(&o[1], &cap.mul(&t_exact, &d[1])?)?,
    ];
    let (px, px_err) = cap.rounding_error_bound(&proj[0])?;
    let (py, py_err) = cap.rounding_error_bound(&proj[1])?;
    let point_residual_bound = combine2_bounds(budget, px_err, py_err)?;

    let res_x = cap.sub(&q[0], &proj[0])?;
    let res_y = cap.sub(&q[1], &proj[1])?;
    let sq_dist = cap.add(&cap.mul(&res_x, &res_x)?, &cap.mul(&res_y, &res_y)?)?;
    let distance_bound = cap.sqrt_up(&sq_dist)?;

    finite_or_uncertified(
        &[t, px, py, distance_bound],
        "line projection overflowed f64 range",
    )?;

    Ok(ExactLineProjection {
        parameter: t,
        parameter_error_bound: t_err,
        point: [px, py],
        point_residual_bound,
        distance_bound,
    })
}

/// 3-D analogue of [`exact_line_project2`].
pub(super) fn exact_line_project3(
    budget: CertificationBudget,
    query: [f64; 3],
    origin: [f64; 3],
    direction: [f64; 3],
) -> Result<ExactLineProjection<3>, GeometryError> {
    let cap = RationalCap::new(budget);
    let q = to_rat3(cap, query)?;
    let o = to_rat3(cap, origin)?;
    let d = to_rat3(cap, direction)?;
    let diff = [
        cap.sub(&q[0], &o[0])?,
        cap.sub(&q[1], &o[1])?,
        cap.sub(&q[2], &o[2])?,
    ];
    let dot_dd = cap.add(
        &cap.add(&cap.mul(&d[0], &d[0])?, &cap.mul(&d[1], &d[1])?)?,
        &cap.mul(&d[2], &d[2])?,
    )?;
    if dot_dd.is_zero() {
        return Err(GeometryError::Uncertified {
            reason: "line direction has exact zero magnitude".to_owned(),
        });
    }
    let dot_diff_d = cap.add(
        &cap.add(&cap.mul(&diff[0], &d[0])?, &cap.mul(&diff[1], &d[1])?)?,
        &cap.mul(&diff[2], &d[2])?,
    )?;
    let t_exact = cap.div(&dot_diff_d, &dot_dd)?;
    let (t, t_err) = cap.rounding_error_bound(&t_exact)?;

    let proj = [
        cap.add(&o[0], &cap.mul(&t_exact, &d[0])?)?,
        cap.add(&o[1], &cap.mul(&t_exact, &d[1])?)?,
        cap.add(&o[2], &cap.mul(&t_exact, &d[2])?)?,
    ];
    let (px, px_err) = cap.rounding_error_bound(&proj[0])?;
    let (py, py_err) = cap.rounding_error_bound(&proj[1])?;
    let (pz, pz_err) = cap.rounding_error_bound(&proj[2])?;
    let point_residual_bound = combine3_bounds(budget, px_err, py_err, pz_err)?;

    let res_x = cap.sub(&q[0], &proj[0])?;
    let res_y = cap.sub(&q[1], &proj[1])?;
    let res_z = cap.sub(&q[2], &proj[2])?;
    let sq_dist = cap.add(
        &cap.add(&cap.mul(&res_x, &res_x)?, &cap.mul(&res_y, &res_y)?)?,
        &cap.mul(&res_z, &res_z)?,
    )?;
    let distance_bound = cap.sqrt_up(&sq_dist)?;

    finite_or_uncertified(
        &[t, px, py, pz, distance_bound],
        "line projection overflowed f64 range",
    )?;

    Ok(ExactLineProjection {
        parameter: t,
        parameter_error_bound: t_err,
        point: [px, py, pz],
        point_residual_bound,
        distance_bound,
    })
}

/// Certified projection of `query` onto the plane
/// `origin + u·u_axis + v·v_axis`, computed via an exact 2×2 Gram-matrix
/// solve (Cramer's rule) in rational arithmetic.
///
/// Unlike a formula that assumes `u_axis`/`v_axis` are *exactly*
/// orthonormal, this solves the true least-squares normal equations
/// ```text
/// [u_axis·u_axis  u_axis·v_axis] [u]   [diff·u_axis]
/// [v_axis·u_axis  v_axis·v_axis] [v] = [diff·v_axis]
/// ```
/// for the stored (approximately, not exactly, orthonormal) axes, so the
/// result is correct regardless of any residual floating-point
/// non-orthogonality left over from Gram-Schmidt construction.
///
/// # Errors
///
/// Returns [`GeometryError::Uncertified`] if the Gram determinant is
/// exactly zero (degenerate axes), any intermediate value overflows the
/// representable `f64` range, or the certification budget is exceeded.
pub(super) fn exact_plane_project3(
    budget: CertificationBudget,
    query: [f64; 3],
    origin: [f64; 3],
    u_axis: [f64; 3],
    v_axis: [f64; 3],
) -> Result<ExactPlaneProjection, GeometryError> {
    let cap = RationalCap::new(budget);
    let q = to_rat3(cap, query)?;
    let o = to_rat3(cap, origin)?;
    let ua = to_rat3(cap, u_axis)?;
    let va = to_rat3(cap, v_axis)?;
    let diff = [
        cap.sub(&q[0], &o[0])?,
        cap.sub(&q[1], &o[1])?,
        cap.sub(&q[2], &o[2])?,
    ];

    let guu = raw_dot3(budget, &ua, &ua)?;
    let guv = raw_dot3(budget, &ua, &va)?;
    let gvv = raw_dot3(budget, &va, &va)?;
    let rhs_u = raw_dot3(budget, &diff, &ua)?;
    let rhs_v = raw_dot3(budget, &diff, &va)?;

    // Cramer's rule for the 2×2 Gram system.
    let det = cap.sub(&cap.mul(&guu, &gvv)?, &cap.mul(&guv, &guv)?)?;
    if det.is_zero() {
        return Err(GeometryError::Uncertified {
            reason: "plane axes are exactly degenerate (zero Gram determinant)".to_owned(),
        });
    }
    let u_numerator = cap.sub(&cap.mul(&rhs_u, &gvv)?, &cap.mul(&rhs_v, &guv)?)?;
    let v_numerator = cap.sub(&cap.mul(&guu, &rhs_v)?, &cap.mul(&guv, &rhs_u)?)?;
    let u_exact = cap.div(&u_numerator, &det)?;
    let v_exact = cap.div(&v_numerator, &det)?;

    let (u, u_err) = cap.rounding_error_bound(&u_exact)?;
    let (v, v_err) = cap.rounding_error_bound(&v_exact)?;
    let parameter_error_bound = u_err.max(v_err);

    let proj = [
        cap.add(
            &cap.add(&o[0], &cap.mul(&u_exact, &ua[0])?)?,
            &cap.mul(&v_exact, &va[0])?,
        )?,
        cap.add(
            &cap.add(&o[1], &cap.mul(&u_exact, &ua[1])?)?,
            &cap.mul(&v_exact, &va[1])?,
        )?,
        cap.add(
            &cap.add(&o[2], &cap.mul(&u_exact, &ua[2])?)?,
            &cap.mul(&v_exact, &va[2])?,
        )?,
    ];
    let (px, px_err) = cap.rounding_error_bound(&proj[0])?;
    let (py, py_err) = cap.rounding_error_bound(&proj[1])?;
    let (pz, pz_err) = cap.rounding_error_bound(&proj[2])?;
    let point_residual_bound = combine3_bounds(budget, px_err, py_err, pz_err)?;

    let res_x = cap.sub(&q[0], &proj[0])?;
    let res_y = cap.sub(&q[1], &proj[1])?;
    let res_z = cap.sub(&q[2], &proj[2])?;
    let sq_dist = cap.add(
        &cap.add(&cap.mul(&res_x, &res_x)?, &cap.mul(&res_y, &res_y)?)?,
        &cap.mul(&res_z, &res_z)?,
    )?;
    let distance_bound = cap.sqrt_up(&sq_dist)?;

    finite_or_uncertified(
        &[u, v, px, py, pz, distance_bound],
        "plane projection overflowed f64 range",
    )?;

    Ok(ExactPlaneProjection {
        u,
        v,
        parameter_error_bound,
        point: [px, py, pz],
        point_residual_bound,
        distance_bound,
    })
}

/// Converts a certified [`TrigError`] into the crate's [`GeometryError`]
/// vocabulary. `Pole` is only produced by [`atan2_interval`] at the exact
/// origin; callers of the circle/cylinder/cone helpers below detect that
/// condition earlier (as [`GeometryError::Singular`]) via the exact
/// in-plane-displacement check, so in practice only `BudgetExhausted` is
/// ever observed here.
fn trig_err_to_uncertified(err: TrigError) -> GeometryError {
    let reason = match err {
        TrigError::BudgetExhausted => {
            "certified trigonometric computation exceeded its series or bit-width budget"
        }
        TrigError::Pole => {
            "certified atan2 is undefined at the origin (zero in-plane displacement)"
        }
        TrigError::DivisionByZero => {
            "certified trigonometric computation attempted division by an interval containing \
             zero"
        }
    };
    GeometryError::Uncertified {
        reason: reason.to_owned(),
    }
}

/// Converts a certified [`RatInterval`] into a single `(value, bound)`
/// pair: `value` is the nearest `f64` to the interval midpoint, and `bound`
/// is a certified upper bound on `|value − x|` for every `x` in the
/// interval — in particular for the true mathematical value, since the
/// interval is guaranteed to enclose it.
///
/// The bound is the sum of (a) the outward-rounded half-width of the
/// interval and (b) the outward-rounded bracket width of rounding the exact
/// midpoint to the nearest `f64`, matching the conservative-but-safe directed
/// rounding used throughout this module.
fn interval_to_f64_bound(
    budget: CertificationBudget,
    interval: &RatInterval,
) -> Result<(f64, f64), GeometryError> {
    let cap = RationalCap::new(budget);
    check_interval_budget(budget, interval)?;
    let two = cap.integer(2)?;
    let mid = cap.div(&cap.add(&interval.lo, &interval.hi)?, &two)?;
    let (value, round_err) = cap.rounding_error_bound(&mid)?;
    let half_width = cap.div(&cap.sub(&interval.hi, &interval.lo)?, &two)?;
    let half_width_bound = cap.rat_to_f64_up(&half_width.abs())?;
    Ok((value, (round_err + half_width_bound).next_up()))
}

/// Rejects a [`RatInterval`] whose endpoints exceed the certification bit
/// budget (see [`check_rational_budget`]).
fn check_interval_budget(
    budget: CertificationBudget,
    interval: &RatInterval,
) -> Result<(), GeometryError> {
    check_rational_budget(budget, &interval.lo)?;
    check_rational_budget(budget, &interval.hi)
}

fn checked_rat(
    budget: CertificationBudget,
    value: BigRational,
) -> Result<BigRational, GeometryError> {
    RationalCap::new(budget).admitted(value)
}

fn interval_point(
    budget: CertificationBudget,
    value: BigRational,
) -> Result<RatInterval, GeometryError> {
    let value = checked_rat(budget, value)?;
    Ok(RatInterval::point(value))
}

fn interval_add(
    budget: CertificationBudget,
    lhs: &RatInterval,
    rhs: &RatInterval,
) -> Result<RatInterval, GeometryError> {
    let cap = RationalCap::new(budget);
    Ok(RatInterval {
        lo: cap.add(&lhs.lo, &rhs.lo)?,
        hi: cap.add(&lhs.hi, &rhs.hi)?,
    })
}

fn interval_sub(
    budget: CertificationBudget,
    lhs: &RatInterval,
    rhs: &RatInterval,
) -> Result<RatInterval, GeometryError> {
    let cap = RationalCap::new(budget);
    Ok(RatInterval {
        lo: cap.sub(&lhs.lo, &rhs.hi)?,
        hi: cap.sub(&lhs.hi, &rhs.lo)?,
    })
}

fn interval_mul(
    budget: CertificationBudget,
    lhs: &RatInterval,
    rhs: &RatInterval,
) -> Result<RatInterval, GeometryError> {
    let cap = RationalCap::new(budget);
    let products = [
        cap.mul(&lhs.lo, &rhs.lo)?,
        cap.mul(&lhs.lo, &rhs.hi)?,
        cap.mul(&lhs.hi, &rhs.lo)?,
        cap.mul(&lhs.hi, &rhs.hi)?,
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
    Ok(RatInterval { lo, hi })
}

fn interval_scale(
    budget: CertificationBudget,
    interval: &RatInterval,
    scalar: &BigRational,
) -> Result<RatInterval, GeometryError> {
    let cap = RationalCap::new(budget);
    if scalar.is_negative() {
        Ok(RatInterval {
            lo: cap.mul(&interval.hi, scalar)?,
            hi: cap.mul(&interval.lo, scalar)?,
        })
    } else {
        Ok(RatInterval {
            lo: cap.mul(&interval.lo, scalar)?,
            hi: cap.mul(&interval.hi, scalar)?,
        })
    }
}

fn interval_neg(
    budget: CertificationBudget,
    interval: &RatInterval,
) -> Result<RatInterval, GeometryError> {
    let cap = RationalCap::new(budget);
    Ok(RatInterval {
        lo: cap.neg(&interval.hi)?,
        hi: cap.neg(&interval.lo)?,
    })
}

fn interval_div(
    budget: CertificationBudget,
    lhs: &RatInterval,
    rhs: &RatInterval,
) -> Result<RatInterval, GeometryError> {
    if rhs.contains_zero() {
        return Err(GeometryError::Uncertified {
            reason: "certified interval division contains zero in the denominator".to_owned(),
        });
    }
    let cap = RationalCap::new(budget);
    let quotients = [
        cap.div(&lhs.lo, &rhs.lo)?,
        cap.div(&lhs.lo, &rhs.hi)?,
        cap.div(&lhs.hi, &rhs.lo)?,
        cap.div(&lhs.hi, &rhs.hi)?,
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

fn interval_hull(
    budget: CertificationBudget,
    lhs: &RatInterval,
    rhs: &RatInterval,
) -> Result<RatInterval, GeometryError> {
    let value = RatInterval {
        lo: if lhs.lo <= rhs.lo {
            lhs.lo.clone()
        } else {
            rhs.lo.clone()
        },
        hi: if lhs.hi >= rhs.hi {
            lhs.hi.clone()
        } else {
            rhs.hi.clone()
        },
    };
    check_interval_budget(budget, &value)?;
    Ok(value)
}

fn raw_dot3(
    budget: CertificationBudget,
    lhs: &[BigRational; 3],
    rhs: &[BigRational; 3],
) -> Result<BigRational, GeometryError> {
    let cap = RationalCap::new(budget);
    let first = cap.mul(&lhs[0], &rhs[0])?;
    let second = cap.mul(&lhs[1], &rhs[1])?;
    let third = cap.mul(&lhs[2], &rhs[2])?;
    let partial = cap.add(&first, &second)?;
    cap.add(&partial, &third)
}

fn raw_cross3(
    budget: CertificationBudget,
    lhs: &[BigRational; 3],
    rhs: &[BigRational; 3],
) -> Result<[BigRational; 3], GeometryError> {
    let cap = RationalCap::new(budget);
    let x = cap.sub(&cap.mul(&lhs[1], &rhs[2])?, &cap.mul(&lhs[2], &rhs[1])?)?;
    let y = cap.sub(&cap.mul(&lhs[2], &rhs[0])?, &cap.mul(&lhs[0], &rhs[2])?)?;
    let z = cap.sub(&cap.mul(&lhs[0], &rhs[1])?, &cap.mul(&lhs[1], &rhs[0])?)?;
    Ok([x, y, z])
}

fn raw_sub_scaled3(
    budget: CertificationBudget,
    base: &[BigRational; 3],
    direction: &[BigRational; 3],
    scale: &BigRational,
) -> Result<[BigRational; 3], GeometryError> {
    let cap = RationalCap::new(budget);
    let x = cap.sub(&base[0], &cap.mul(scale, &direction[0])?)?;
    let y = cap.sub(&base[1], &cap.mul(scale, &direction[1])?)?;
    let z = cap.sub(&base[2], &cap.mul(scale, &direction[2])?)?;
    Ok([x, y, z])
}

fn raw_add_scaled3(
    budget: CertificationBudget,
    base: &[BigRational; 3],
    direction: &[BigRational; 3],
    scale: &BigRational,
) -> Result<[BigRational; 3], GeometryError> {
    let cap = RationalCap::new(budget);
    let x = cap.add(&base[0], &cap.mul(scale, &direction[0])?)?;
    let y = cap.add(&base[1], &cap.mul(scale, &direction[1])?)?;
    let z = cap.add(&base[2], &cap.mul(scale, &direction[2])?)?;
    Ok([x, y, z])
}

fn normalized_components(
    budget: CertificationBudget,
    components: &[BigRational],
    zero_reason: &'static str,
    underflow_reason: &'static str,
) -> Result<(Vec<RatInterval>, RatInterval), GeometryError> {
    let cap = RationalCap::new(budget);
    let mut norm_sq = BigRational::zero();
    for component in components {
        cap.admit(component)?;
        let square = cap.mul(component, component)?;
        norm_sq = cap.add(&norm_sq, &square)?;
    }
    if !norm_sq.is_positive() {
        return Err(GeometryError::Uncertified {
            reason: zero_reason.to_owned(),
        });
    }

    let lower = cap.sqrt_down(&norm_sq)?;
    let upper = cap.sqrt_up(&norm_sq)?;
    if !lower.is_finite() || !upper.is_finite() || lower <= 0.0 || upper <= 0.0 {
        return Err(GeometryError::Uncertified {
            reason: underflow_reason.to_owned(),
        });
    }
    let norm = RatInterval {
        lo: cap.decode_f64(lower)?,
        hi: cap.decode_f64(upper)?,
    };

    let mut unit = Vec::with_capacity(components.len());
    for component in components {
        unit.push(ratio_by_bracket(budget, component, &norm.lo, &norm.hi)?);
    }
    Ok((unit, norm))
}

fn normalized2(
    budget: CertificationBudget,
    components: &[BigRational; 2],
    zero_reason: &'static str,
    underflow_reason: &'static str,
) -> Result<([RatInterval; 2], RatInterval), GeometryError> {
    let (unit, norm) = normalized_components(budget, components, zero_reason, underflow_reason)?;
    let unit: [RatInterval; 2] = unit.try_into().map_err(|_| GeometryError::Uncertified {
        reason: "internal two-dimensional normalization shape mismatch".to_owned(),
    })?;
    Ok((unit, norm))
}

fn normalized3(
    budget: CertificationBudget,
    components: &[BigRational; 3],
    zero_reason: &'static str,
    underflow_reason: &'static str,
) -> Result<([RatInterval; 3], RatInterval), GeometryError> {
    let (unit, norm) = normalized_components(budget, components, zero_reason, underflow_reason)?;
    let unit: [RatInterval; 3] = unit.try_into().map_err(|_| GeometryError::Uncertified {
        reason: "internal three-dimensional normalization shape mismatch".to_owned(),
    })?;
    Ok((unit, norm))
}

struct IdealFrame2 {
    x: [RatInterval; 2],
    y: [RatInterval; 2],
}

fn ideal_frame2(
    budget: CertificationBudget,
    x_seed: [f64; 2],
) -> Result<(IdealFrame2, [BigRational; 2]), GeometryError> {
    let x_raw = to_rat2(RationalCap::new(budget), x_seed)?;
    let (x, _) = normalized2(
        budget,
        &x_raw,
        "circle frame x seed has exact zero magnitude",
        "circle frame x normalization bracket underflowed or overflowed",
    )?;
    let y = [interval_neg(budget, &x[1])?, x[0].clone()];
    check_interval_budget(budget, &y[1])?;
    Ok((IdealFrame2 { x, y }, x_raw))
}

struct IdealFrame3 {
    z: [RatInterval; 3],
    x: [RatInterval; 3],
    y: [RatInterval; 3],
    z_seed: [BigRational; 3],
    x_perp: [BigRational; 3],
    z_norm: RatInterval,
}

fn interval_cross3(
    budget: CertificationBudget,
    lhs: &[RatInterval; 3],
    rhs: &[RatInterval; 3],
) -> Result<[RatInterval; 3], GeometryError> {
    let x = interval_sub(
        budget,
        &interval_mul(budget, &lhs[1], &rhs[2])?,
        &interval_mul(budget, &lhs[2], &rhs[1])?,
    )?;
    let y = interval_sub(
        budget,
        &interval_mul(budget, &lhs[2], &rhs[0])?,
        &interval_mul(budget, &lhs[0], &rhs[2])?,
    )?;
    let z = interval_sub(
        budget,
        &interval_mul(budget, &lhs[0], &rhs[1])?,
        &interval_mul(budget, &lhs[1], &rhs[0])?,
    )?;
    Ok([x, y, z])
}

fn ideal_frame3(
    budget: CertificationBudget,
    z_seed: [f64; 3],
    x_seed: [f64; 3],
) -> Result<IdealFrame3, GeometryError> {
    let cap = RationalCap::new(budget);
    let z_raw = to_rat3(cap, z_seed)?;
    let x_raw = to_rat3(cap, x_seed)?;
    let (z, z_norm) = normalized3(
        budget,
        &z_raw,
        "three-dimensional frame z seed has exact zero magnitude",
        "three-dimensional frame z normalization bracket underflowed or overflowed",
    )?;
    let z_sq = raw_dot3(budget, &z_raw, &z_raw)?;
    let x_dot_z = raw_dot3(budget, &x_raw, &z_raw)?;
    let parallel_scale = cap.div(&x_dot_z, &z_sq)?;
    let x_perp = raw_sub_scaled3(budget, &x_raw, &z_raw, &parallel_scale)?;
    let (x, _) = normalized3(
        budget,
        &x_perp,
        "three-dimensional frame x seed is exactly parallel to z",
        "three-dimensional frame x normalization bracket underflowed or overflowed",
    )?;
    let y = interval_cross3(budget, &z, &x)?;
    Ok(IdealFrame3 {
        z,
        x,
        y,
        z_seed: z_raw,
        x_perp,
        z_norm,
    })
}

fn interval_value(
    budget: CertificationBudget,
    interval: &RatInterval,
) -> Result<(f64, f64), GeometryError> {
    let cap = RationalCap::new(budget);
    check_interval_budget(budget, interval)?;
    let two = cap.integer(2)?;
    let midpoint = cap.div(&cap.add(&interval.lo, &interval.hi)?, &two)?;
    let half_width = cap.div(&cap.sub(&interval.hi, &interval.lo)?, &two)?;
    let (value, rounding_bound) = cap.rounding_error_bound(&midpoint)?;
    let enclosure_bound = cap.rat_to_f64_up(&half_width.abs())?;
    let total = rounding_bound + enclosure_bound;
    let bound = if total == 0.0 { 0.0 } else { total.next_up() };
    if !value.is_finite() || !bound.is_finite() || bound < 0.0 {
        return Err(GeometryError::Uncertified {
            reason: "certified interval representative overflowed f64 range".to_owned(),
        });
    }
    Ok((value, bound))
}

fn report_point2(
    budget: CertificationBudget,
    point: &[RatInterval; 2],
) -> Result<([f64; 2], f64), GeometryError> {
    let (x, ex) = interval_value(budget, &point[0])?;
    let (y, ey) = interval_value(budget, &point[1])?;
    let residual = combine2_bounds(budget, ex, ey)?;
    Ok(([x, y], residual))
}

fn report_point3(
    budget: CertificationBudget,
    point: &[RatInterval; 3],
) -> Result<([f64; 3], f64), GeometryError> {
    let (x, ex) = interval_value(budget, &point[0])?;
    let (y, ey) = interval_value(budget, &point[1])?;
    let (z, ez) = interval_value(budget, &point[2])?;
    let residual = combine3_bounds(budget, ex, ey, ez)?;
    Ok(([x, y, z], residual))
}

fn squared_distance2(
    budget: CertificationBudget,
    query: &[BigRational; 2],
    point: &[RatInterval; 2],
) -> Result<RatInterval, GeometryError> {
    let dx = interval_sub(
        budget,
        &interval_point(budget, query[0].clone())?,
        &point[0],
    )?;
    let dy = interval_sub(
        budget,
        &interval_point(budget, query[1].clone())?,
        &point[1],
    )?;
    interval_add(
        budget,
        &interval_square(budget, &dx)?,
        &interval_square(budget, &dy)?,
    )
}

fn squared_distance3(
    budget: CertificationBudget,
    query: &[BigRational; 3],
    point: &[RatInterval; 3],
) -> Result<RatInterval, GeometryError> {
    let dx = interval_sub(
        budget,
        &interval_point(budget, query[0].clone())?,
        &point[0],
    )?;
    let dy = interval_sub(
        budget,
        &interval_point(budget, query[1].clone())?,
        &point[1],
    )?;
    let dz = interval_sub(
        budget,
        &interval_point(budget, query[2].clone())?,
        &point[2],
    )?;
    let xy = interval_add(
        budget,
        &interval_square(budget, &dx)?,
        &interval_square(budget, &dy)?,
    )?;
    interval_add(budget, &xy, &interval_square(budget, &dz)?)
}

fn interval_square(
    budget: CertificationBudget,
    interval: &RatInterval,
) -> Result<RatInterval, GeometryError> {
    let cap = RationalCap::new(budget);
    let lo_square = cap.mul(&interval.lo, &interval.lo)?;
    let hi_square = cap.mul(&interval.hi, &interval.hi)?;
    let hi = if lo_square >= hi_square {
        lo_square.clone()
    } else {
        hi_square.clone()
    };
    let lo = if interval.contains_zero() {
        BigRational::zero()
    } else if lo_square <= hi_square {
        lo_square
    } else {
        hi_square
    };
    Ok(RatInterval { lo, hi })
}

fn distance_bound(
    budget: CertificationBudget,
    squared_distance: &RatInterval,
) -> Result<f64, GeometryError> {
    let bound = RationalCap::new(budget).sqrt_up(&squared_distance.hi)?;
    if !bound.is_finite() || bound < 0.0 {
        return Err(GeometryError::Uncertified {
            reason: "certified distance overflowed f64 range".to_owned(),
        });
    }
    Ok(bound)
}

fn circle_component_intervals(
    budget: CertificationBudget,
    base: &BigRational,
    x: &RatInterval,
    y: &RatInterval,
    radius_cos: &RatInterval,
    radius_sin: &RatInterval,
) -> Result<(RatInterval, RatInterval, RatInterval), GeometryError> {
    let x_offset = interval_mul(budget, radius_cos, x)?;
    let y_offset = interval_mul(budget, radius_sin, y)?;
    let offset = interval_add(budget, &x_offset, &y_offset)?;
    let position = interval_add(budget, &interval_point(budget, base.clone())?, &offset)?;
    let first = interval_sub(
        budget,
        &interval_mul(budget, radius_cos, y)?,
        &interval_mul(budget, radius_sin, x)?,
    )?;
    let second = interval_neg(budget, &offset)?;
    Ok((position, first, second))
}

/// The result of a certified circle evaluation
/// `p(θ) = center + r·cos(θ)·x + r·sin(θ)·y`, including first and second
/// derivatives (always computed, regardless of the caller's requested
/// [`crate::DerivativeOrder`], since the certified `sin`/`cos` enclosure is
/// shared and cheap to reuse).
pub(super) struct ExactCircleEval<const N: usize> {
    pub point: [f64; N],
    pub position_error_bound: f64,
    pub first: [f64; N],
    pub first_error_bound: f64,
    pub second: [f64; N],
    pub second_error_bound: f64,
}

/// Certified evaluation of a 2-D circle at parameter `theta`, via the
/// rational-interval `sin`/`cos` enclosure from [`super::trig`].
///
/// # Errors
///
/// Returns [`GeometryError::Uncertified`] if the certified trig computation
/// exceeds its budget, or any intermediate value overflows `f64` range.
pub(super) fn exact_circle_eval2(
    budget: CertificationBudget,
    center: [f64; 2],
    radius: f64,
    x_seed: [f64; 2],
    theta: f64,
) -> Result<ExactCircleEval<2>, GeometryError> {
    let cap = RationalCap::new(budget);
    let theta_r = cap.decode_f64(theta)?;
    let (sin_i, cos_i) = sin_cos_interval(&theta_r, budget).map_err(trig_err_to_uncertified)?;
    let (frame, _) = ideal_frame2(budget, x_seed)?;
    let c = to_rat2(cap, center)?;
    let r = cap.decode_f64(radius)?;
    let radius_cos = interval_scale(budget, &cos_i, &r)?;
    let radius_sin = interval_scale(budget, &sin_i, &r)?;
    let (p0, d10, d20) = circle_component_intervals(
        budget,
        &c[0],
        &frame.x[0],
        &frame.y[0],
        &radius_cos,
        &radius_sin,
    )?;
    let (p1, d11, d21) = circle_component_intervals(
        budget,
        &c[1],
        &frame.x[1],
        &frame.y[1],
        &radius_cos,
        &radius_sin,
    )?;
    let (point, position_error_bound) = report_point2(budget, &[p0, p1])?;
    let (first, first_error_bound) = report_point2(budget, &[d10, d11])?;
    let (second, second_error_bound) = report_point2(budget, &[d20, d21])?;

    Ok(ExactCircleEval {
        point,
        position_error_bound,
        first,
        first_error_bound,
        second,
        second_error_bound,
    })
}

/// 3-D analogue of [`exact_circle_eval2`].
pub(super) fn exact_circle_eval3(
    budget: CertificationBudget,
    center: [f64; 3],
    radius: f64,
    normal_seed: [f64; 3],
    x_seed: [f64; 3],
    theta: f64,
) -> Result<ExactCircleEval<3>, GeometryError> {
    let cap = RationalCap::new(budget);
    let theta_r = cap.decode_f64(theta)?;
    let (sin_i, cos_i) = sin_cos_interval(&theta_r, budget).map_err(trig_err_to_uncertified)?;
    let frame = ideal_frame3(budget, normal_seed, x_seed)?;
    let c = to_rat3(cap, center)?;
    let r = cap.decode_f64(radius)?;
    let radius_cos = interval_scale(budget, &cos_i, &r)?;
    let radius_sin = interval_scale(budget, &sin_i, &r)?;
    let (p0, d10, d20) = circle_component_intervals(
        budget,
        &c[0],
        &frame.x[0],
        &frame.y[0],
        &radius_cos,
        &radius_sin,
    )?;
    let (p1, d11, d21) = circle_component_intervals(
        budget,
        &c[1],
        &frame.x[1],
        &frame.y[1],
        &radius_cos,
        &radius_sin,
    )?;
    let (p2, d12, d22) = circle_component_intervals(
        budget,
        &c[2],
        &frame.x[2],
        &frame.y[2],
        &radius_cos,
        &radius_sin,
    )?;
    let (point, position_error_bound) = report_point3(budget, &[p0, p1, p2])?;
    let (first, first_error_bound) = report_point3(budget, &[d10, d11, d12])?;
    let (second, second_error_bound) = report_point3(budget, &[d20, d21, d22])?;

    Ok(ExactCircleEval {
        point,
        position_error_bound,
        first,
        first_error_bound,
        second,
        second_error_bound,
    })
}

/// The result of a certified circle (or circular cross-section) projection.
pub(super) struct ExactCircleProjection<const N: usize> {
    pub parameter: f64,
    pub parameter_error_bound: f64,
    pub point: [f64; N],
    pub point_residual_bound: f64,
    pub distance_bound: f64,
}

/// Builds a certified rational-interval enclosure of `numer / mag`, where
/// `mag` is known only via the certified bracket `[mag_down, mag_up]`
/// (`0 < mag_down ≤ true_mag ≤ mag_up`). Since `x ↦ numer / x` is monotone
/// on `(0, ∞)` (decreasing for `numer > 0`, increasing for `numer < 0`),
/// the extreme values of the ratio occur at the bracket endpoints.
fn ratio_by_bracket(
    budget: CertificationBudget,
    numer: &BigRational,
    mag_down: &BigRational,
    mag_up: &BigRational,
) -> Result<RatInterval, GeometryError> {
    let cap = RationalCap::new(budget);
    if numer.is_zero() {
        interval_point(budget, BigRational::zero())
    } else if numer.is_positive() {
        Ok(RatInterval {
            lo: cap.div(numer, mag_up)?,
            hi: cap.div(numer, mag_down)?,
        })
    } else {
        Ok(RatInterval {
            lo: cap.div(numer, mag_down)?,
            hi: cap.div(numer, mag_up)?,
        })
    }
}

/// Reduces a certified angular interval to a canonical periodic representative
/// in `[0, τ)` together with a certified error bound — **without** any
/// `rem_euclid` reduction and never using the `f64` `τ` constant for the
/// reduction itself.
///
/// `theta_interval` must already be nominally placed in `[0, 2π)` by the
/// caller (the projection helpers add the certified `tau_interval` when
/// `atan2` returns a negative branch). Two certified cases are handled using
/// only the rational `τ` enclosure `tau_int`:
///
/// * **Interior** — when the whole interval is provably inside `[0, tau_int.lo)`
///   (`lo ≥ 0` and `hi < tau_int.lo`), the nearest-`f64` midpoint is returned
///   with the combined half-width and rounding error. If that midpoint rounds
///   up to (or past) the `f64` `τ` constant, it is clamped down to the largest
///   representable parameter and the clamp gap is folded into the bound so the
///   emitted value always lies strictly inside `[0, TAU)`.
/// * **Seam** — when the interval touches or crosses the `0 ≡ τ` seam
///   (`lo < 0` or `hi ≥ tau_int.lo`), the canonical representative is `0`
///   (matching the removed `rem_euclid` mapping `τ → 0`). The error bound is
///   the largest mod-`τ` circular distance from `0` to any point of the
///   interval, taken over both seam representatives (`0` and the certified `τ`
///   enclosure), so a tight interval sitting just below a full turn still
///   yields a small *periodic* bound.
///
/// # Errors
///
/// Returns [`GeometryError::Uncertified`] only when the certified `τ` interval
/// cannot be computed within `budget`.
pub(super) fn certified_periodic_param(
    theta_interval: &RatInterval,
    budget: CertificationBudget,
) -> Result<(f64, f64), GeometryError> {
    let cap = RationalCap::new(budget);
    check_interval_budget(budget, theta_interval)?;
    let tau_int = tau_interval(budget).map_err(trig_err_to_uncertified)?;
    check_interval_budget(budget, &tau_int)?;

    // Interior case: the whole interval is provably inside [0, τ) using the
    // certified lower bound tau_int.lo ≤ τ, so no wrap-around is needed. This is
    // the `rem_euclid`-free proof that the representative is already canonical.
    if !theta_interval.lo.is_negative() && theta_interval.hi < tau_int.lo {
        let (canonical, error_bound) = interval_to_f64_bound(budget, theta_interval)?;
        // A nonnegative interval midpoint never rounds negative, but it may
        // round up to (or past) the `f64` τ constant when the true value sits
        // just below `τ`. Clamp it to the largest representable parameter and
        // fold the clamp gap into the certified bound.
        if canonical >= core::f64::consts::TAU {
            let clamped = core::f64::consts::TAU.next_down();
            let extra = (canonical - clamped).abs();
            return Ok((clamped, (error_bound + extra).next_up()));
        }
        return Ok((canonical, error_bound));
    }

    // Seam case: the interval sits at (or crosses) the 0 ≡ τ seam. The canonical
    // representative is 0 — matching the removed `rem_euclid` mapping τ → 0 —
    // and the certified error is the largest mod-τ circular distance from 0 to
    // any point of the interval. The distance is built from the rational τ
    // enclosure (never the f64 τ constant): the seam is 0 for points near the
    // lower endpoint and τ for points near a full turn, so the minimum of the
    // two representative distances is the tight periodic bound.
    let t_lo = &theta_interval.lo;
    let t_hi = &theta_interval.hi;
    let tau_lo = &tau_int.lo;
    let tau_hi = &tau_int.hi;

    let dist_zero = {
        let a = t_lo.abs();
        let b = t_hi.abs();
        if a >= b { a } else { b }
    };
    let dist_tau = {
        let a = cap.sub(t_hi, tau_lo)?.abs();
        let b = cap.sub(tau_hi, t_lo)?.abs();
        if a >= b { a } else { b }
    };
    let bound_rat = if dist_zero <= dist_tau {
        dist_zero
    } else {
        dist_tau
    };

    Ok((0.0, cap.rat_to_f64_up(&bound_rat)?))
}

fn periodic_angle_from_interval(
    budget: CertificationBudget,
    angle: RatInterval,
    y_is_negative: bool,
) -> Result<(f64, f64), GeometryError> {
    let canonical_interval = if y_is_negative {
        let tau = tau_interval(budget).map_err(trig_err_to_uncertified)?;
        interval_add(budget, &angle, &tau)?
    } else {
        angle
    };
    certified_periodic_param(&canonical_interval, budget)
}

fn angle_from_radial2(
    budget: CertificationBudget,
    x_seed: &[BigRational; 2],
    radial: &[BigRational; 2],
) -> Result<(f64, f64), GeometryError> {
    let cap = RationalCap::new(budget);
    // Normalization cancels from both atan2 arguments:
    // atan2(perp(x̂)·r̂, x̂·r̂) = atan2(cross(x_seed, radial), x_seed·radial).
    let numerator = cap.sub(
        &cap.mul(&x_seed[0], &radial[1])?,
        &cap.mul(&x_seed[1], &radial[0])?,
    )?;
    let denominator = cap.add(
        &cap.mul(&x_seed[0], &radial[0])?,
        &cap.mul(&x_seed[1], &radial[1])?,
    )?;
    let angle =
        atan2_interval(&numerator, &denominator, budget).map_err(trig_err_to_uncertified)?;
    periodic_angle_from_interval(budget, angle, numerator.is_negative())
}

fn angle_from_radial3(
    budget: CertificationBudget,
    frame: &IdealFrame3,
    radial: &[BigRational; 3],
    flip_for_negative_nappe: bool,
) -> Result<(f64, f64), GeometryError> {
    let cap = RationalCap::new(budget);
    // With x_perp = x_seed - z_seed*(x_seed·z_seed)/(z_seed·z_seed),
    // x̂ = x_perp/||x_perp|| and ŷ = ẑ×x̂, the normalized radial factors
    // cancel and leave atan2(B, A*||z_seed||).
    let mut a = raw_dot3(budget, radial, &frame.x_perp)?;
    let z_cross_x = raw_cross3(budget, &frame.z_seed, &frame.x_perp)?;
    let mut b = raw_dot3(budget, radial, &z_cross_x)?;
    if flip_for_negative_nappe {
        a = cap.neg(&a)?;
        b = cap.neg(&b)?;
    }

    let endpoint_a = cap.mul(&a, &frame.z_norm.lo)?;
    let endpoint_b = cap.mul(&a, &frame.z_norm.hi)?;
    let denominator = RatInterval {
        lo: if endpoint_a <= endpoint_b {
            endpoint_a.clone()
        } else {
            endpoint_b.clone()
        },
        hi: if endpoint_a >= endpoint_b {
            endpoint_a
        } else {
            endpoint_b
        },
    };
    check_interval_budget(budget, &denominator)?;

    // ||z_seed|| is provably positive, so these two endpoint evaluations stay
    // in one atan2 branch. Their hull is a direct interval extension of the
    // only remaining non-rational factor in the angular coordinate.
    let lower_angle =
        atan2_interval(&b, &denominator.lo, budget).map_err(trig_err_to_uncertified)?;
    let upper_angle =
        atan2_interval(&b, &denominator.hi, budget).map_err(trig_err_to_uncertified)?;
    let angle = interval_hull(budget, &lower_angle, &upper_angle)?;
    periodic_angle_from_interval(budget, angle, b.is_negative())
}

fn raw_plane_residual3(
    budget: CertificationBudget,
    displacement: &[BigRational; 3],
    z_seed: &[BigRational; 3],
) -> Result<(BigRational, [BigRational; 3]), GeometryError> {
    let cap = RationalCap::new(budget);
    let z_sq = raw_dot3(budget, z_seed, z_seed)?;
    if !z_sq.is_positive() {
        return Err(GeometryError::Uncertified {
            reason: "axis seed has exact zero magnitude".to_owned(),
        });
    }
    let axial_numerator = raw_dot3(budget, displacement, z_seed)?;
    let raw_line_parameter = cap.div(&axial_numerator, &z_sq)?;
    let radial = raw_sub_scaled3(budget, displacement, z_seed, &raw_line_parameter)?;
    Ok((raw_line_parameter, radial))
}

fn exact_zero2(value: &[BigRational; 2]) -> bool {
    value[0].is_zero() && value[1].is_zero()
}

fn exact_zero3(value: &[BigRational; 3]) -> bool {
    value[0].is_zero() && value[1].is_zero() && value[2].is_zero()
}

/// Certified projection of `query` onto a 2-D circle
/// `center + r·cos(θ)·x_axis + r·sin(θ)·y_axis`.
///
/// The projected point and `distance_bound` are derived purely from exact
/// dot products and the certified [`sqrt_up`]/[`sqrt_down`] bracket on the
/// in-plane offset magnitude — no trigonometry is needed for either. Only
/// the reported parameter `θ` requires [`atan2_interval`]; its branch
/// (`(−π, π]` → `[0, 2π)`) is resolved from the *exact* sign of the
/// in-plane `y`-offset, never from an approximate interval midpoint.
///
/// # Errors
///
/// Returns [`GeometryError::Singular`] when `query` coincides exactly with
/// `center`'s projection onto the circle's plane (no unique nearest point),
/// and [`GeometryError::Uncertified`] if the in-plane offset underflows the
/// smallest positive `f64` magnitude, the certified trig budget is
/// exceeded, or any intermediate value overflows `f64` range.
pub(super) fn exact_circle_project2(
    budget: CertificationBudget,
    query: [f64; 2],
    center: [f64; 2],
    radius: f64,
    x_seed: [f64; 2],
) -> Result<ExactCircleProjection<2>, GeometryError> {
    let cap = RationalCap::new(budget);
    let q = to_rat2(cap, query)?;
    let c = to_rat2(cap, center)?;
    let radial = [cap.sub(&q[0], &c[0])?, cap.sub(&q[1], &c[1])?];
    if exact_zero2(&radial) {
        return Err(GeometryError::Singular);
    }
    let (_, x_raw) = ideal_frame2(budget, x_seed)?;
    let (radial_unit, _) = normalized2(
        budget,
        &radial,
        "circle projection radial vector has exact zero magnitude",
        "circle projection radial normalization bracket underflowed or overflowed",
    )?;
    let radius = cap.decode_f64(radius)?;
    let point = [
        interval_add(
            budget,
            &interval_point(budget, c[0].clone())?,
            &interval_scale(budget, &radial_unit[0], &radius)?,
        )?,
        interval_add(
            budget,
            &interval_point(budget, c[1].clone())?,
            &interval_scale(budget, &radial_unit[1], &radius)?,
        )?,
    ];
    let (point_value, point_residual_bound) = report_point2(budget, &point)?;
    let distance_bound = distance_bound(budget, &squared_distance2(budget, &q, &point)?)?;
    let (parameter, parameter_error_bound) = angle_from_radial2(budget, &x_raw, &radial)?;

    Ok(ExactCircleProjection {
        parameter,
        parameter_error_bound,
        point: point_value,
        point_residual_bound,
        distance_bound,
    })
}

/// Certified 3-D circle projection against the frozen ideal frame.  The raw
/// normal seed defines the exact support plane; using it for the plane foot is
/// valid because scaling the normal does not change that plane.
pub(super) fn exact_circle_project3(
    budget: CertificationBudget,
    query: [f64; 3],
    center: [f64; 3],
    radius: f64,
    normal_seed: [f64; 3],
    x_seed: [f64; 3],
) -> Result<ExactCircleProjection<3>, GeometryError> {
    let cap = RationalCap::new(budget);
    let q = to_rat3(cap, query)?;
    let c = to_rat3(cap, center)?;
    let displacement = [
        cap.sub(&q[0], &c[0])?,
        cap.sub(&q[1], &c[1])?,
        cap.sub(&q[2], &c[2])?,
    ];
    let frame = ideal_frame3(budget, normal_seed, x_seed)?;
    let (_, radial) = raw_plane_residual3(budget, &displacement, &frame.z_seed)?;
    if exact_zero3(&radial) {
        return Err(GeometryError::Singular);
    }
    let (radial_unit, _) = normalized3(
        budget,
        &radial,
        "circle projection planar residual has exact zero magnitude",
        "circle projection planar residual normalization bracket underflowed or overflowed",
    )?;
    let radius = cap.decode_f64(radius)?;
    let point = [
        interval_add(
            budget,
            &interval_point(budget, c[0].clone())?,
            &interval_scale(budget, &radial_unit[0], &radius)?,
        )?,
        interval_add(
            budget,
            &interval_point(budget, c[1].clone())?,
            &interval_scale(budget, &radial_unit[1], &radius)?,
        )?,
        interval_add(
            budget,
            &interval_point(budget, c[2].clone())?,
            &interval_scale(budget, &radial_unit[2], &radius)?,
        )?,
    ];
    let (point_value, point_residual_bound) = report_point3(budget, &point)?;
    let distance_bound = distance_bound(budget, &squared_distance3(budget, &q, &point)?)?;
    let (parameter, parameter_error_bound) = angle_from_radial3(budget, &frame, &radial, false)?;

    Ok(ExactCircleProjection {
        parameter,
        parameter_error_bound,
        point: point_value,
        point_residual_bound,
        distance_bound,
    })
}

/// The result of a certified cylinder surface evaluation at `(u, v)`.
/// Every returned vector is evaluated against component intervals of the
/// frozen ideal frame, including `dv = z_ideal`.
pub(super) struct ExactCylinderEval {
    pub point: [f64; 3],
    pub position_error_bound: f64,
    pub du: [f64; 3],
    pub du_error_bound: f64,
    pub dv: [f64; 3],
    pub dv_error_bound: f64,
    pub duu: [f64; 3],
    pub duu_error_bound: f64,
}

/// Certified evaluation of a cylinder surface at `(u, v)` in the frozen ideal
/// frame `z_ideal, x_ideal, y_ideal`.
#[allow(clippy::too_many_arguments)]
pub(super) fn exact_cylinder_eval(
    budget: CertificationBudget,
    axis_origin: [f64; 3],
    z_seed: [f64; 3],
    radius: f64,
    x_seed: [f64; 3],
    u: f64,
    v: f64,
) -> Result<ExactCylinderEval, GeometryError> {
    let cap = RationalCap::new(budget);
    let u_r = cap.decode_f64(u)?;
    let (sin_i, cos_i) = sin_cos_interval(&u_r, budget).map_err(trig_err_to_uncertified)?;
    let frame = ideal_frame3(budget, z_seed, x_seed)?;
    let origin = to_rat3(cap, axis_origin)?;
    let radius = cap.decode_f64(radius)?;
    let v = cap.decode_f64(v)?;
    let radius_cos = interval_scale(budget, &cos_i, &radius)?;
    let radius_sin = interval_scale(budget, &sin_i, &radius)?;

    let (without_axis0, du0, duu0) = circle_component_intervals(
        budget,
        &origin[0],
        &frame.x[0],
        &frame.y[0],
        &radius_cos,
        &radius_sin,
    )?;
    let (without_axis1, du1, duu1) = circle_component_intervals(
        budget,
        &origin[1],
        &frame.x[1],
        &frame.y[1],
        &radius_cos,
        &radius_sin,
    )?;
    let (without_axis2, du2, duu2) = circle_component_intervals(
        budget,
        &origin[2],
        &frame.x[2],
        &frame.y[2],
        &radius_cos,
        &radius_sin,
    )?;
    let point = [
        interval_add(
            budget,
            &without_axis0,
            &interval_scale(budget, &frame.z[0], &v)?,
        )?,
        interval_add(
            budget,
            &without_axis1,
            &interval_scale(budget, &frame.z[1], &v)?,
        )?,
        interval_add(
            budget,
            &without_axis2,
            &interval_scale(budget, &frame.z[2], &v)?,
        )?,
    ];
    let (point, position_error_bound) = report_point3(budget, &point)?;
    let (du, du_error_bound) = report_point3(budget, &[du0, du1, du2])?;
    let (dv, dv_error_bound) = report_point3(budget, &frame.z)?;
    let (duu, duu_error_bound) = report_point3(budget, &[duu0, duu1, duu2])?;

    Ok(ExactCylinderEval {
        point,
        position_error_bound,
        du,
        du_error_bound,
        dv,
        dv_error_bound,
        duu,
        duu_error_bound,
    })
}

/// The result of a certified cylinder projection.
pub(super) struct ExactCylinderProjection {
    pub u: f64,
    pub v: f64,
    pub u_error_bound: f64,
    pub v_error_bound: f64,
    pub point: [f64; 3],
    pub point_residual_bound: f64,
    pub distance_bound: f64,
}

/// Certified projection of `query` onto the ideal cylinder. The raw axis seed
/// defines the same axis line as `z_ideal`; the reported `v` is then converted
/// from the raw-line parameter to the coordinate along `z_ideal`.
#[allow(clippy::too_many_arguments)]
pub(super) fn exact_cylinder_project(
    budget: CertificationBudget,
    query: [f64; 3],
    axis_origin: [f64; 3],
    z_seed: [f64; 3],
    radius: f64,
    x_seed: [f64; 3],
) -> Result<ExactCylinderProjection, GeometryError> {
    let cap = RationalCap::new(budget);
    let q = to_rat3(cap, query)?;
    let origin = to_rat3(cap, axis_origin)?;
    let displacement = [
        cap.sub(&q[0], &origin[0])?,
        cap.sub(&q[1], &origin[1])?,
        cap.sub(&q[2], &origin[2])?,
    ];
    let frame = ideal_frame3(budget, z_seed, x_seed)?;
    let (raw_line_parameter, radial) = raw_plane_residual3(budget, &displacement, &frame.z_seed)?;
    if exact_zero3(&radial) {
        return Err(GeometryError::Singular);
    }
    let (radial_unit, _) = normalized3(
        budget,
        &radial,
        "cylinder projection radial residual has exact zero magnitude",
        "cylinder projection radial normalization bracket underflowed or overflowed",
    )?;
    let radius = cap.decode_f64(radius)?;
    let axis_foot = raw_add_scaled3(budget, &origin, &frame.z_seed, &raw_line_parameter)?;
    let point_interval = [
        interval_add(
            budget,
            &interval_point(budget, axis_foot[0].clone())?,
            &interval_scale(budget, &radial_unit[0], &radius)?,
        )?,
        interval_add(
            budget,
            &interval_point(budget, axis_foot[1].clone())?,
            &interval_scale(budget, &radial_unit[1], &radius)?,
        )?,
        interval_add(
            budget,
            &interval_point(budget, axis_foot[2].clone())?,
            &interval_scale(budget, &radial_unit[2], &radius)?,
        )?,
    ];
    let (point, point_residual_bound) = report_point3(budget, &point_interval)?;
    let distance_bound = distance_bound(budget, &squared_distance3(budget, &q, &point_interval)?)?;
    let v_interval = interval_mul(
        budget,
        &interval_point(budget, raw_line_parameter)?,
        &frame.z_norm,
    )?;
    let (v, v_error_bound) = interval_value(budget, &v_interval)?;
    let (u, u_error_bound) = angle_from_radial3(budget, &frame, &radial, false)?;

    Ok(ExactCylinderProjection {
        u,
        v,
        u_error_bound,
        v_error_bound,
        point,
        point_residual_bound,
        distance_bound,
    })
}

/// The result of a certified cone evaluation at `(u, v)` under the frozen
/// ideal frame. `dvv` is exactly zero and is supplied by the caller.
pub(super) struct ExactConeEval {
    pub point: [f64; 3],
    pub position_error_bound: f64,
    pub du: [f64; 3],
    pub du_error_bound: f64,
    pub dv: [f64; 3],
    pub dv_error_bound: f64,
    pub duu: [f64; 3],
    pub duu_error_bound: f64,
    pub duv: [f64; 3],
    pub duv_error_bound: f64,
}

#[allow(clippy::too_many_arguments)]
fn cone_component_intervals(
    budget: CertificationBudget,
    apex: &BigRational,
    z: &RatInterval,
    x: &RatInterval,
    y: &RatInterval,
    v: &BigRational,
    tangent: &RatInterval,
    sin_u: &RatInterval,
    cos_u: &RatInterval,
) -> Result<
    (
        RatInterval,
        RatInterval,
        RatInterval,
        RatInterval,
        RatInterval,
    ),
    GeometryError,
> {
    let direction = interval_add(
        budget,
        &interval_mul(budget, cos_u, x)?,
        &interval_mul(budget, sin_u, y)?,
    )?;
    let perpendicular = interval_sub(
        budget,
        &interval_mul(budget, cos_u, y)?,
        &interval_mul(budget, sin_u, x)?,
    )?;
    let v_tangent = interval_scale(budget, tangent, v)?;
    let radial_offset = interval_mul(budget, &v_tangent, &direction)?;
    let position = interval_add(
        budget,
        &interval_add(
            budget,
            &interval_point(budget, apex.clone())?,
            &interval_scale(budget, z, v)?,
        )?,
        &radial_offset,
    )?;
    let du = interval_mul(budget, &v_tangent, &perpendicular)?;
    let dv = interval_add(budget, z, &interval_mul(budget, tangent, &direction)?)?;
    let duu = interval_neg(budget, &radial_offset)?;
    let duv = interval_mul(budget, tangent, &perpendicular)?;
    Ok((position, du, dv, duu, duv))
}

/// Certified evaluation of a cone surface at `(u, v)` in the frozen ideal
/// frame `z_ideal, x_ideal, y_ideal`.
#[allow(clippy::too_many_arguments)]
pub(super) fn exact_cone_eval(
    budget: CertificationBudget,
    apex: [f64; 3],
    z_seed: [f64; 3],
    half_angle: f64,
    x_seed: [f64; 3],
    u: f64,
    v: f64,
) -> Result<ExactConeEval, GeometryError> {
    let cap = RationalCap::new(budget);
    let u_r = cap.decode_f64(u)?;
    let angle_r = cap.decode_f64(half_angle)?;
    let (sin_u, cos_u) = sin_cos_interval(&u_r, budget).map_err(trig_err_to_uncertified)?;
    let (sin_a, cos_a) = sin_cos_interval(&angle_r, budget).map_err(trig_err_to_uncertified)?;
    let tangent = interval_div(budget, &sin_a, &cos_a)?;
    if !tangent.lo.is_positive() {
        return Err(GeometryError::Uncertified {
            reason: "cone tangent interval is not provably positive".to_owned(),
        });
    }

    let frame = ideal_frame3(budget, z_seed, x_seed)?;
    let apex = to_rat3(cap, apex)?;
    let v = cap.decode_f64(v)?;
    let (p0, du0, dv0, duu0, duv0) = cone_component_intervals(
        budget,
        &apex[0],
        &frame.z[0],
        &frame.x[0],
        &frame.y[0],
        &v,
        &tangent,
        &sin_u,
        &cos_u,
    )?;
    let (p1, du1, dv1, duu1, duv1) = cone_component_intervals(
        budget,
        &apex[1],
        &frame.z[1],
        &frame.x[1],
        &frame.y[1],
        &v,
        &tangent,
        &sin_u,
        &cos_u,
    )?;
    let (p2, du2, dv2, duu2, duv2) = cone_component_intervals(
        budget,
        &apex[2],
        &frame.z[2],
        &frame.x[2],
        &frame.y[2],
        &v,
        &tangent,
        &sin_u,
        &cos_u,
    )?;
    let (point, position_error_bound) = report_point3(budget, &[p0, p1, p2])?;
    let (du, du_error_bound) = report_point3(budget, &[du0, du1, du2])?;
    let (dv, dv_error_bound) = report_point3(budget, &[dv0, dv1, dv2])?;
    let (duu, duu_error_bound) = report_point3(budget, &[duu0, duu1, duu2])?;
    let (duv, duv_error_bound) = report_point3(budget, &[duv0, duv1, duv2])?;

    Ok(ExactConeEval {
        point,
        position_error_bound,
        du,
        du_error_bound,
        dv,
        dv_error_bound,
        duu,
        duu_error_bound,
        duv,
        duv_error_bound,
    })
}

/// The result of a certified cone projection.
pub(super) struct ExactConeProjection {
    pub u: f64,
    pub v: f64,
    pub u_error_bound: f64,
    pub v_error_bound: f64,
    pub point: [f64; 3],
    pub point_residual_bound: f64,
    pub distance_bound: f64,
}

pub(super) struct ExactConeProjectionPair {
    pub primary: ExactConeProjection,
    pub secondary: Option<ExactConeProjection>,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum ConeCandidateKind {
    Apex,
    Positive,
    Negative,
}

struct ConeCandidate {
    kind: ConeCandidateKind,
    v: RatInterval,
    point: [RatInterval; 3],
    squared_distance: RatInterval,
}

fn ray_parameter(
    budget: CertificationBudget,
    radial_magnitude: &RatInterval,
    axial_coordinate: &RatInterval,
    tangent: &RatInterval,
    positive_nappe: bool,
) -> Result<RatInterval, GeometryError> {
    let radial_term = interval_mul(budget, radial_magnitude, tangent)?;
    let axial_term = if positive_nappe {
        axial_coordinate.clone()
    } else {
        interval_neg(budget, axial_coordinate)?
    };
    let numerator = interval_add(budget, &radial_term, &axial_term)?;
    let tangent_squared = interval_mul(budget, tangent, tangent)?;
    let denominator = interval_add(
        budget,
        &interval_point(budget, BigRational::one())?,
        &tangent_squared,
    )?;
    interval_div(budget, &numerator, &denominator)
}

fn admissible_ray_parameter(parameter: &RatInterval) -> Result<bool, GeometryError> {
    if parameter.lo.is_positive() {
        Ok(true)
    } else if !parameter.hi.is_positive() {
        Ok(false)
    } else {
        Err(GeometryError::Uncertified {
            reason: "cone ray admissibility cannot be proved from its certified parameter interval"
                .to_owned(),
        })
    }
}

#[allow(clippy::too_many_arguments)]
fn make_cone_ray_candidate(
    budget: CertificationBudget,
    kind: ConeCandidateKind,
    apex: &[BigRational; 3],
    frame: &IdealFrame3,
    radial_unit: &[RatInterval; 3],
    tangent: &RatInterval,
    ray_parameter: RatInterval,
    query: &[BigRational; 3],
) -> Result<ConeCandidate, GeometryError> {
    let v = if kind == ConeCandidateKind::Negative {
        interval_neg(budget, &ray_parameter)?
    } else {
        ray_parameter
    };
    // On the negative nappe v < 0, so -v*tan points along the query's radial
    // unit vector. On the positive nappe the scale is +v*tan.
    let radial_scale = if kind == ConeCandidateKind::Negative {
        interval_mul(budget, &interval_neg(budget, &v)?, tangent)?
    } else {
        interval_mul(budget, &v, tangent)?
    };
    let point = [
        interval_add(
            budget,
            &interval_add(
                budget,
                &interval_point(budget, apex[0].clone())?,
                &interval_mul(budget, &v, &frame.z[0])?,
            )?,
            &interval_mul(budget, &radial_scale, &radial_unit[0])?,
        )?,
        interval_add(
            budget,
            &interval_add(
                budget,
                &interval_point(budget, apex[1].clone())?,
                &interval_mul(budget, &v, &frame.z[1])?,
            )?,
            &interval_mul(budget, &radial_scale, &radial_unit[1])?,
        )?,
        interval_add(
            budget,
            &interval_add(
                budget,
                &interval_point(budget, apex[2].clone())?,
                &interval_mul(budget, &v, &frame.z[2])?,
            )?,
            &interval_mul(budget, &radial_scale, &radial_unit[2])?,
        )?,
    ];
    let squared_distance = squared_distance3(budget, query, &point)?;
    Ok(ConeCandidate {
        kind,
        v,
        point,
        squared_distance,
    })
}

fn make_apex_candidate(
    budget: CertificationBudget,
    apex: &[BigRational; 3],
    query: &[BigRational; 3],
) -> Result<ConeCandidate, GeometryError> {
    let point = [
        interval_point(budget, apex[0].clone())?,
        interval_point(budget, apex[1].clone())?,
        interval_point(budget, apex[2].clone())?,
    ];
    let squared_distance = squared_distance3(budget, query, &point)?;
    Ok(ConeCandidate {
        kind: ConeCandidateKind::Apex,
        v: interval_point(budget, BigRational::zero())?,
        point,
        squared_distance,
    })
}

fn materialize_cone_candidate(
    budget: CertificationBudget,
    candidate: &ConeCandidate,
    frame: &IdealFrame3,
    radial: &[BigRational; 3],
) -> Result<ExactConeProjection, GeometryError> {
    let (point, point_residual_bound) = report_point3(budget, &candidate.point)?;
    let distance_bound = distance_bound(budget, &candidate.squared_distance)?;
    if candidate.kind == ConeCandidateKind::Apex {
        return Ok(ExactConeProjection {
            u: 0.0,
            v: 0.0,
            u_error_bound: 0.0,
            v_error_bound: 0.0,
            point,
            point_residual_bound,
            distance_bound,
        });
    }
    let (u, u_error_bound) = angle_from_radial3(
        budget,
        frame,
        radial,
        candidate.kind == ConeCandidateKind::Negative,
    )?;
    let (v, v_error_bound) = interval_value(budget, &candidate.v)?;
    Ok(ExactConeProjection {
        u,
        v,
        u_error_bound,
        v_error_bound,
        point,
        point_residual_bound,
        distance_bound,
    })
}

fn interval_strictly_less(lhs: &RatInterval, rhs: &RatInterval) -> bool {
    lhs.hi < rhs.lo
}

/// Certified closest-point projection onto both nappes of the ideal cone.
/// The positive ray, negative ray, and apex are all constructed before their
/// certified squared-distance intervals are compared. No axial-sign shortcut
/// is used for selection.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn exact_cone_project(
    budget: CertificationBudget,
    query: [f64; 3],
    apex: [f64; 3],
    z_seed: [f64; 3],
    half_angle: f64,
    x_seed: [f64; 3],
) -> Result<ExactConeProjectionPair, GeometryError> {
    let cap = RationalCap::new(budget);
    let q = to_rat3(cap, query)?;
    let apex = to_rat3(cap, apex)?;
    let displacement = [
        cap.sub(&q[0], &apex[0])?,
        cap.sub(&q[1], &apex[1])?,
        cap.sub(&q[2], &apex[2])?,
    ];
    let frame = ideal_frame3(budget, z_seed, x_seed)?;
    let apex_candidate = make_apex_candidate(budget, &apex, &q)?;
    if exact_zero3(&displacement) {
        return Ok(ExactConeProjectionPair {
            primary: materialize_cone_candidate(budget, &apex_candidate, &frame, &displacement)?,
            secondary: None,
        });
    }

    let (raw_line_parameter, radial) = raw_plane_residual3(budget, &displacement, &frame.z_seed)?;
    if exact_zero3(&radial) {
        // Away from the apex every azimuth is a valid nearest-point direction,
        // so this inverse mapping is geometrically singular.
        return Err(GeometryError::Singular);
    }
    let (radial_unit, radial_magnitude) = normalized3(
        budget,
        &radial,
        "cone projection radial residual has exact zero magnitude",
        "cone projection radial normalization bracket underflowed or overflowed",
    )?;
    let axial_coordinate = interval_mul(
        budget,
        &interval_point(budget, raw_line_parameter.clone())?,
        &frame.z_norm,
    )?;

    let angle_r = cap.decode_f64(half_angle)?;
    let (sin_a, cos_a) = sin_cos_interval(&angle_r, budget).map_err(trig_err_to_uncertified)?;
    let tangent = interval_div(budget, &sin_a, &cos_a)?;
    if !tangent.lo.is_positive() {
        return Err(GeometryError::Uncertified {
            reason: "cone tangent interval is not provably positive".to_owned(),
        });
    }

    let positive_parameter =
        ray_parameter(budget, &radial_magnitude, &axial_coordinate, &tangent, true)?;
    let negative_parameter = ray_parameter(
        budget,
        &radial_magnitude,
        &axial_coordinate,
        &tangent,
        false,
    )?;
    let positive = if admissible_ray_parameter(&positive_parameter)? {
        Some(make_cone_ray_candidate(
            budget,
            ConeCandidateKind::Positive,
            &apex,
            &frame,
            &radial_unit,
            &tangent,
            positive_parameter,
            &q,
        )?)
    } else {
        None
    };
    let negative = if admissible_ray_parameter(&negative_parameter)? {
        Some(make_cone_ray_candidate(
            budget,
            ConeCandidateKind::Negative,
            &apex,
            &frame,
            &radial_unit,
            &tangent,
            negative_parameter,
            &q,
        )?)
    } else {
        None
    };

    // h = 0 is proved from the exact raw axis-line parameter. The two ray
    // formulas are then mirror images in the meridian plane, establishing an
    // exact positive/negative tie independently of interval overlap.
    if raw_line_parameter.is_zero()
        && let (Some(positive), Some(negative)) = (&positive, &negative)
    {
        if interval_strictly_less(&positive.squared_distance, &apex_candidate.squared_distance)
            && interval_strictly_less(&negative.squared_distance, &apex_candidate.squared_distance)
        {
            return Ok(ExactConeProjectionPair {
                primary: materialize_cone_candidate(budget, positive, &frame, &radial)?,
                secondary: Some(materialize_cone_candidate(
                    budget, negative, &frame, &radial,
                )?),
            });
        }
        if interval_strictly_less(&apex_candidate.squared_distance, &positive.squared_distance)
            && interval_strictly_less(&apex_candidate.squared_distance, &negative.squared_distance)
        {
            return Ok(ExactConeProjectionPair {
                primary: materialize_cone_candidate(budget, &apex_candidate, &frame, &radial)?,
                secondary: None,
            });
        }
        return Err(GeometryError::Uncertified {
            reason: "cone apex/ray ordering cannot be certified".to_owned(),
        });
    }

    let mut candidates = vec![&apex_candidate];
    if let Some(positive) = &positive {
        candidates.push(positive);
    }
    if let Some(negative) = &negative {
        candidates.push(negative);
    }
    let unique_minimum = candidates.iter().copied().find(|candidate| {
        candidates.iter().all(|other| {
            core::ptr::eq(*candidate, *other)
                || interval_strictly_less(&candidate.squared_distance, &other.squared_distance)
        })
    });
    let Some(unique_minimum) = unique_minimum else {
        return Err(GeometryError::Uncertified {
            reason: "cone candidate ordering cannot be certified".to_owned(),
        });
    };
    Ok(ExactConeProjectionPair {
        primary: materialize_cone_candidate(budget, unique_minimum, &frame, &radial)?,
        secondary: None,
    })
}

/// Checks a certified position/distance bound against the effective
/// tolerance at the given world-coordinate `scale`, returning
/// [`GeometryError::Uncertified`] when the bound is too loose to certify
/// at the requested precision.
pub(super) fn check_tolerance(
    tolerance: &ToleranceContext,
    bound: f64,
    scale: f64,
) -> Result<(), GeometryError> {
    let eff_tol = tolerance
        .effective_length(scale)
        .map_err(|_| GeometryError::Uncertified {
            reason: "world scale is invalid for tolerance computation".to_owned(),
        })?;
    if bound > eff_tol {
        return Err(GeometryError::Uncertified {
            reason: "certified error bound exceeds requested tolerance at this world scale"
                .to_owned(),
        });
    }
    Ok(())
}

/// Checks a certified angular parameter error bound against
/// [`ToleranceContext::angular`], returning [`GeometryError::Uncertified`]
/// when the bound exceeds the caller's angular tolerance.
///
/// The angular tolerance is a fixed value in radians, independent of any
/// world-scale factor; it must not be confused with the length tolerance.
pub(super) fn check_angular_tolerance(
    tolerance: &ToleranceContext,
    bound_radians: f64,
) -> Result<(), GeometryError> {
    if bound_radians > tolerance.angular() {
        return Err(GeometryError::Uncertified {
            reason: "certified angular parameter error bound exceeds angular tolerance".to_owned(),
        });
    }
    Ok(())
}

/// Checks a certified parameter-space error bound against
/// [`ToleranceContext::parametric`], returning [`GeometryError::Uncertified`]
/// when the bound exceeds the caller's parametric tolerance.
pub(super) fn check_parametric_tolerance(
    tolerance: &ToleranceContext,
    bound: f64,
) -> Result<(), GeometryError> {
    if bound > tolerance.parametric() {
        return Err(GeometryError::Uncertified {
            reason: "certified parameter-space error bound exceeds parametric tolerance".to_owned(),
        });
    }
    Ok(())
}

/// Checks a certified derivative error bound against a caller-supplied limit.
/// Returns `Uncertified` if the bound exceeds the limit.
///
/// `limit` is always finite and non-negative (the derivative-limit newtypes
/// guarantee it); `f64::MAX` means "no effective limit". The check is a plain
/// `bound > limit` comparison, which can never be silently disabled by a NaN
/// or `+∞` limit.
pub(super) fn check_derivative_limit(bound: f64, limit: f64) -> Result<(), GeometryError> {
    if bound > limit {
        return Err(GeometryError::Uncertified {
            reason: format!(
                "certified derivative error bound {bound:.3e} exceeds caller limit {limit:.3e}"
            ),
        });
    }
    Ok(())
}

/// Computes a certified upper bound on `tan(half_angle)` using the certified
/// trig backend.  Returns `None` and the caller should return `Uncertified`
/// if the budget is exceeded or the cosine interval straddles zero
/// (impossible for `half_angle ∈ (0, π/2)` but guarded for safety).
pub(super) fn certified_tan_upper(half_angle: f64, budget: CertificationBudget) -> Option<f64> {
    let cap = RationalCap::new(budget);
    let ha_r = cap.decode_f64(half_angle).ok()?;
    let (sin_i, cos_i) = super::trig::sin_cos_interval(&ha_r, budget).ok()?;
    if !cos_i.lo.is_positive() {
        return None; // should not occur for half_angle ∈ (0, π/2)
    }
    let tangent_upper = cap.div(&sin_i.hi, &cos_i.lo).ok()?;
    cap.rat_to_f64_up(&tangent_upper).ok()
}

#[cfg(test)]
mod tests {
    // These tests assert exact results of certified rational arithmetic
    // (e.g. `1.0 + 2.0*t` for integral `t`), so bit-exact f64 equality is
    // the intended assertion, not an approximate floating-point comparison.
    #![allow(clippy::float_cmp)]

    use std::f64::consts::TAU;

    use amphion_foundation::{NormalizationError, ToleranceContext};
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use num_traits::One;

    use super::super::trig::RatInterval;
    use super::{
        certified_periodic_param, check_angular_tolerance, check_tolerance, exact_affine_eval2,
        exact_affine_eval3, exact_cone_project, exact_line_project2, exact_line_project3,
        exact_plane_project3, normalization_to_construction, rounding_error_bound_unchecked,
    };
    use crate::CertificationBudget;
    use crate::analytic::ConstructionError;

    #[test]
    fn normalization_to_construction_maps_zero_magnitude_to_degenerate_axis() {
        assert_eq!(
            normalization_to_construction(NormalizationError::ZeroMagnitude),
            ConstructionError::DegenerateAxis
        );
    }

    #[test]
    fn normalization_to_construction_maps_non_finite_to_non_finite_input() {
        assert_eq!(
            normalization_to_construction(NormalizationError::NonFinite),
            ConstructionError::NonFiniteInput
        );
    }

    #[test]
    fn normalization_to_construction_maps_not_normalized_to_non_finite_input() {
        assert_eq!(
            normalization_to_construction(NormalizationError::NotNormalized),
            ConstructionError::NonFiniteInput
        );
    }

    fn tol() -> ToleranceContext {
        ToleranceContext::try_new(1e-9, 1e-8, 1e-10, 1e-12).unwrap()
    }

    fn budget() -> CertificationBudget {
        CertificationBudget::default()
    }

    #[test]
    fn exact_affine_eval2_reproduces_exact_arithmetic() {
        let result = exact_affine_eval2(budget(), [1.0, 2.0], [0.0, 1.0], 3.0).unwrap();
        assert_eq!(result.point, [1.0, 5.0]);
        assert!(result.position_error_bound >= 0.0);
    }

    #[test]
    fn exact_affine_eval3_reproduces_exact_arithmetic() {
        let result = exact_affine_eval3(budget(), [1.0, 2.0, 3.0], [1.0, 0.0, 0.0], 2.0).unwrap();
        assert_eq!(result.point, [3.0, 2.0, 3.0]);
        assert!(result.position_error_bound >= 0.0);
    }

    #[test]
    fn exact_affine_rejects_oversized_input_before_cancellation() {
        let tiny = f64::MIN_POSITIVE;
        let capped = CertificationBudget::try_new(200, 64).unwrap();
        assert!(
            exact_affine_eval2(capped, [tiny, 0.0], [1.0, 0.0], -tiny).is_err(),
            "the 1023-bit input denominator must be rejected before it cancels to zero"
        );
    }

    #[test]
    fn projection_rejects_values_wider_than_the_rational_cap() {
        let capped = CertificationBudget::try_new(200, 2).unwrap();
        assert!(exact_line_project2(capped, [6.0, 0.0], [3.0, 0.0], [1.0, 0.0]).is_err());
        assert!(
            exact_plane_project3(
                capped,
                [6.0, 0.0, 0.0],
                [3.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 0.0],
            )
            .is_err()
        );
    }

    #[test]
    fn projection_preflights_certified_square_root_work() {
        let capped = CertificationBudget::try_new(200, 3).unwrap();
        assert!(
            exact_line_project2(capped, [0.0, 1.0], [0.0, 0.0], [1.0, 0.0]).is_err(),
            "sqrt certification must not allocate its fixed 2048-bit comparison value"
        );
    }

    #[test]
    fn projection_preflights_rational_to_f64_shift_work() {
        let capped = CertificationBudget::try_new(200, 2_152).unwrap();
        let minsub = f64::from_bits(1);
        assert!(
            exact_line_project2(capped, [1.0, 0.0], [0.0, 0.0], [1.0, minsub]).is_err(),
            "num-rational's conversion shift must remain inside the caller cap"
        );
    }

    #[test]
    fn affine_error_norm_obeys_the_same_rational_cap() {
        let capped = CertificationBudget::try_new(200, 54).unwrap();
        assert!(
            exact_affine_eval2(capped, [1.0, 0.0], [0.6, 0.8], 1.0).is_err(),
            "rounding-bound conversion and norm work must remain inside the caller cap"
        );
    }

    #[test]
    fn exact_line_project2_certifies_zero_residual() {
        let result = exact_line_project2(budget(), [1.0, 0.0], [0.0, 0.0], [1.0, 0.0]).unwrap();
        assert!(result.distance_bound >= 0.0);
        assert!(result.distance_bound < 1e-9);
    }

    #[test]
    fn exact_line_project3_certifies_zero_residual() {
        let result =
            exact_line_project3(budget(), [1.0, 0.0, 0.0], [0.0, 0.0, 0.0], [1.0, 0.0, 0.0])
                .unwrap();
        assert!(result.distance_bound >= 0.0);
        assert!(result.distance_bound < 1e-9);
    }

    #[test]
    fn exact_line_project2_rejects_zero_direction() {
        assert!(exact_line_project2(budget(), [1.0, 2.0], [0.0, 0.0], [0.0, 0.0]).is_err());
    }

    #[test]
    fn exact_plane_project3_certifies_zero_residual() {
        let result = exact_plane_project3(
            budget(),
            [1.0, 2.0, 0.0],
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
        )
        .unwrap();
        assert!((result.u - 1.0).abs() < 1e-9);
        assert!((result.v - 2.0).abs() < 1e-9);
        assert!(result.distance_bound < 1e-9);
    }

    #[test]
    fn exact_plane_project3_rejects_degenerate_axes() {
        assert!(
            exact_plane_project3(
                budget(),
                [1.0, 2.0, 0.0],
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [2.0, 0.0, 0.0],
            )
            .is_err()
        );
    }

    #[test]
    fn check_tolerance_rejects_bound_exceeding_effective_tolerance() {
        assert!(check_tolerance(&tol(), 1.0, 1.0).is_err());
        assert!(check_tolerance(&tol(), 1e-15, 1.0).is_ok());
    }

    #[test]
    fn check_angular_tolerance_uses_unit_scale() {
        assert!(check_angular_tolerance(&tol(), 1.0).is_err());
        assert!(check_angular_tolerance(&tol(), 1e-15).is_ok());
    }

    #[test]
    fn certified_periodic_param_certifies_interior_interval() {
        // A tiny interval just above 0 (θ ≈ +δ), well inside [0, τ), is
        // certified and reduced to a near-0 representative with a small bound.
        let tiny = BigRational::new(BigInt::one(), BigInt::from(1_000_000i64));
        let near_zero = RatInterval {
            lo: tiny.clone(),
            hi: &tiny + &tiny,
        };
        let budget = CertificationBudget::default();
        let (canon_lo, err_lo) = certified_periodic_param(&near_zero, budget).unwrap();
        assert!((0.0..TAU).contains(&canon_lo));
        assert!(canon_lo < 1e-3);
        assert!((0.0..1e-3).contains(&err_lo));

        // An interval comfortably below a full turn (θ ≈ 2π − δ for a small
        // but non-seam δ) is still strictly inside [0, τ) and is certified.
        let two = BigRational::from_integer(BigInt::from(2i64));
        let delta = BigRational::new(BigInt::one(), BigInt::from(1000i64));
        let pi = crate::analytic::trig::pi_interval(budget).unwrap();
        // Use the certified *lower* π bound to build 2·π_lo − δ, which is
        // guaranteed < true 2π, hence inside [0, τ).
        let two_pi_lo = &pi.lo * &two;
        let near_turn = RatInterval {
            lo: &two_pi_lo - &delta - &delta,
            hi: &two_pi_lo - &delta,
        };
        let (canon_hi, err_hi) = certified_periodic_param(&near_turn, budget).unwrap();
        assert!((0.0..TAU).contains(&canon_hi));
        assert!((0.0..1e-2).contains(&err_hi));
    }

    #[test]
    fn certified_periodic_param_reduces_seam_to_zero() {
        let budget = CertificationBudget::default();

        // An interval straddling 0 from just below (θ ≈ ±δ) maps to the
        // canonical seam representative 0 with a small periodic bound — no
        // `rem_euclid`, no rejection.
        let tiny = BigRational::new(BigInt::one(), BigInt::from(1_000_000i64));
        let across_zero = RatInterval {
            lo: -tiny.clone(),
            hi: tiny.clone(),
        };
        let (canon_zero, err_zero) = certified_periodic_param(&across_zero, budget).unwrap();
        assert_eq!(canon_zero, 0.0);
        assert!((0.0..1e-3).contains(&err_zero));

        // An interval just above a full turn (θ ≳ 2π, produced by adding the τ
        // enclosure to a tiny negative `atan2` branch) also reduces to the
        // canonical 0 with a small *periodic* bound rather than an out-of-range
        // value near τ.
        let two = BigRational::from_integer(BigInt::from(2i64));
        let pi = crate::analytic::trig::pi_interval(budget).unwrap();
        let two_pi_hi = &pi.hi * &two;
        let near_turn = RatInterval {
            lo: two_pi_hi.clone(),
            hi: &two_pi_hi + &tiny,
        };
        let (canon_turn, err_turn) = certified_periodic_param(&near_turn, budget).unwrap();
        assert_eq!(canon_turn, 0.0);
        assert!((0.0..1e-3).contains(&err_turn));
    }

    #[test]
    fn rounding_error_bound_is_nonnegative_and_brackets_exact_value() {
        let r = BigRational::new(BigInt::one(), BigInt::from(3i64));
        let (rounded, bound) = rounding_error_bound_unchecked(&r);
        assert!(bound >= 0.0);
        assert!((rounded - 0.333_333_333_333_333_3).abs() < 1e-9);
    }

    /// Item 5 regression: a query exactly at the cone apex returns the
    /// canonical parameterization `(u = 0, v = 0)` with the apex as the point
    /// and zero certified distance/residual — never `Singular`.
    #[test]
    fn exact_cone_project_apex_returns_canonical_parameter() {
        let budget = CertificationBudget::default();
        let apex = [1.0, 2.0, 3.0];
        let pair = exact_cone_project(
            budget,
            apex,
            apex,
            [0.0, 0.0, 1.0],
            std::f64::consts::FRAC_PI_4,
            [1.0, 0.0, 0.0],
        )
        .unwrap();
        assert_eq!(pair.primary.u, 0.0);
        assert_eq!(pair.primary.v, 0.0);
        assert_eq!(pair.primary.point, apex);
        assert_eq!(pair.primary.distance_bound, 0.0);
        assert_eq!(pair.primary.point_residual_bound, 0.0);
        assert!(pair.secondary.is_none());
    }
}
