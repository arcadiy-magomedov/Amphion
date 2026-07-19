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

use amphion_foundation::{NormalizationError, ToleranceContext};
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

pub(super) fn mag2(v: [f64; 2]) -> f64 {
    let scale = v[0].abs().max(v[1].abs());
    if scale == 0.0 {
        0.0
    } else {
        let vs = [v[0] / scale, v[1] / scale];
        scale * (vs[0] * vs[0] + vs[1] * vs[1]).sqrt()
    }
}

pub(super) fn scale2(v: [f64; 2], s: f64) -> [f64; 2] {
    [v[0] * s, v[1] * s]
}

/// Rotates `v` by 90° CCW, giving the perpendicular direction.
pub(super) fn perp2(v: [f64; 2]) -> [f64; 2] {
    [-v[1], v[0]]
}

pub(super) fn all_finite2(v: [f64; 2]) -> bool {
    v[0].is_finite() && v[1].is_finite()
}

// ─── Domain helpers ──────────────────────────────────────────────────────────

/// Returns `true` when `t` is finite and inside the declared parameter range.
///
/// For a periodic domain `[lower, upper)` the upper bound is **exclusive**.
/// For a bounded non-periodic domain `[lower, upper]` both bounds are inclusive.
/// For a half-open or infinite domain the absent bound is unchecked.
pub(super) fn in_range(t: f64, range: ParameterRange) -> bool {
    if !t.is_finite() {
        return false;
    }
    let lo_ok = range.lower().is_none_or(|lo| t >= lo);
    let hi_ok = range.upper().is_none_or(|hi| {
        if range.period().is_some() {
            t < hi
        } else {
            t <= hi
        }
    });
    lo_ok && hi_ok
}

// ─── Certified rational-arithmetic helpers ─────────────────────────────────

/// Rejects a `BigRational` whose numerator or denominator bit-width exceeds
/// the certification budget, guarding against unbounded memory growth on
/// adversarial input (e.g. coordinates crafted to blow up intermediate
/// exact-rational bit-width).
pub(super) fn check_rational_budget(
    budget: CertificationBudget,
    r: &BigRational,
) -> Result<(), GeometryError> {
    let bits = r.numer().bits().max(r.denom().bits());
    if bits > u64::from(budget.rational_bits) {
        Err(GeometryError::Uncertified {
            reason: "intermediate exact-rational value exceeded the certification bit-width \
                     budget"
                .to_owned(),
        })
    } else {
        Ok(())
    }
}

/// Rounds an exact `BigRational` value to the nearest `f64` and returns a
/// certified (safe, outward) bound on `|rounded − exact|`.
///
/// The bound is the full width of the bracket `[rat_to_f64_down(exact),
/// rat_to_f64_up(exact)]`, which always contains both `exact` and the
/// nearest-rounded `f64`, so it safely (if slightly loosely, by at most a
/// factor of two relative to the tightest possible half-ULP bound) bounds
/// the rounding error.
pub(super) fn rounding_error_bound(exact: &BigRational) -> (f64, f64) {
    let rounded = rat_to_f64(exact);
    let up = rat_to_f64_up(exact);
    let down = rat_to_f64_down(exact);
    let width = (up - down).max(0.0);
    (rounded, width.next_up())
}

/// Combines two independent, non-negative axis error bounds into a single
/// certified Euclidean bound, outward-rounded for safety.
pub(super) fn combine2_bounds(a: f64, b: f64) -> f64 {
    a.hypot(b).next_up()
}

/// Combines three independent, non-negative axis error bounds into a single
/// certified Euclidean bound, outward-rounded for safety.
pub(super) fn combine3_bounds(a: f64, b: f64, c: f64) -> f64 {
    a.hypot(b).hypot(c).next_up()
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

fn to_rat2(v: [f64; 2]) -> [BigRational; 2] {
    [f64_to_rat(v[0]), f64_to_rat(v[1])]
}

fn to_rat3(v: [f64; 3]) -> [BigRational; 3] {
    [f64_to_rat(v[0]), f64_to_rat(v[1]), f64_to_rat(v[2])]
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
    let o = to_rat2(origin);
    let d = to_rat2(direction);
    let t_r = f64_to_rat(t);
    let px = &o[0] + &t_r * &d[0];
    let py = &o[1] + &t_r * &d[1];
    check_rational_budget(budget, &px)?;
    check_rational_budget(budget, &py)?;
    let (x, ex) = rounding_error_bound(&px);
    let (y, ey) = rounding_error_bound(&py);
    finite_or_uncertified(&[x, y], "evaluated line position overflowed f64 range")?;
    Ok(ExactEvalResult {
        point: [x, y],
        position_error_bound: combine2_bounds(ex, ey),
    })
}

/// 3-D analogue of [`exact_affine_eval2`].
pub(super) fn exact_affine_eval3(
    budget: CertificationBudget,
    origin: [f64; 3],
    direction: [f64; 3],
    t: f64,
) -> Result<ExactEvalResult<3>, GeometryError> {
    let o = to_rat3(origin);
    let d = to_rat3(direction);
    let t_r = f64_to_rat(t);
    let px = &o[0] + &t_r * &d[0];
    let py = &o[1] + &t_r * &d[1];
    let pz = &o[2] + &t_r * &d[2];
    check_rational_budget(budget, &px)?;
    check_rational_budget(budget, &py)?;
    check_rational_budget(budget, &pz)?;
    let (x, ex) = rounding_error_bound(&px);
    let (y, ey) = rounding_error_bound(&py);
    let (z, ez) = rounding_error_bound(&pz);
    finite_or_uncertified(&[x, y, z], "evaluated line position overflowed f64 range")?;
    Ok(ExactEvalResult {
        point: [x, y, z],
        position_error_bound: combine3_bounds(ex, ey, ez),
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
    let o = to_rat3(origin);
    let ua = to_rat3(u_axis);
    let va = to_rat3(v_axis);
    let u_r = f64_to_rat(u);
    let v_r = f64_to_rat(v);
    let px = &o[0] + &u_r * &ua[0] + &v_r * &va[0];
    let py = &o[1] + &u_r * &ua[1] + &v_r * &va[1];
    let pz = &o[2] + &u_r * &ua[2] + &v_r * &va[2];
    check_rational_budget(budget, &px)?;
    check_rational_budget(budget, &py)?;
    check_rational_budget(budget, &pz)?;
    let (x, ex) = rounding_error_bound(&px);
    let (y, ey) = rounding_error_bound(&py);
    let (z, ez) = rounding_error_bound(&pz);
    finite_or_uncertified(&[x, y, z], "evaluated plane position overflowed f64 range")?;
    Ok(ExactEvalResult {
        point: [x, y, z],
        position_error_bound: combine3_bounds(ex, ey, ez),
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
    let q = to_rat2(query);
    let o = to_rat2(origin);
    let d = to_rat2(direction);
    let diff = [&q[0] - &o[0], &q[1] - &o[1]];
    let dot_dd = &d[0] * &d[0] + &d[1] * &d[1];
    if dot_dd.is_zero() {
        return Err(GeometryError::Uncertified {
            reason: "line direction has exact zero magnitude".to_owned(),
        });
    }
    let dot_diff_d = &diff[0] * &d[0] + &diff[1] * &d[1];
    let t_exact = &dot_diff_d / &dot_dd;
    check_rational_budget(budget, &t_exact)?;
    let (t, t_err) = rounding_error_bound(&t_exact);

    let proj = [&o[0] + &t_exact * &d[0], &o[1] + &t_exact * &d[1]];
    let (px, px_err) = rounding_error_bound(&proj[0]);
    let (py, py_err) = rounding_error_bound(&proj[1]);
    let point_residual_bound = combine2_bounds(px_err, py_err);

    let res_x = &q[0] - &proj[0];
    let res_y = &q[1] - &proj[1];
    let sq_dist = &res_x * &res_x + &res_y * &res_y;
    check_rational_budget(budget, &sq_dist)?;
    let distance_bound = sqrt_up(&sq_dist).map_err(|()| GeometryError::Uncertified {
        reason: "line projection distance is negative (unreachable)".to_owned(),
    })?;

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
    let q = to_rat3(query);
    let o = to_rat3(origin);
    let d = to_rat3(direction);
    let diff = [&q[0] - &o[0], &q[1] - &o[1], &q[2] - &o[2]];
    let dot_dd = &d[0] * &d[0] + &d[1] * &d[1] + &d[2] * &d[2];
    if dot_dd.is_zero() {
        return Err(GeometryError::Uncertified {
            reason: "line direction has exact zero magnitude".to_owned(),
        });
    }
    let dot_diff_d = &diff[0] * &d[0] + &diff[1] * &d[1] + &diff[2] * &d[2];
    let t_exact = &dot_diff_d / &dot_dd;
    check_rational_budget(budget, &t_exact)?;
    let (t, t_err) = rounding_error_bound(&t_exact);

    let proj = [
        &o[0] + &t_exact * &d[0],
        &o[1] + &t_exact * &d[1],
        &o[2] + &t_exact * &d[2],
    ];
    let (px, px_err) = rounding_error_bound(&proj[0]);
    let (py, py_err) = rounding_error_bound(&proj[1]);
    let (pz, pz_err) = rounding_error_bound(&proj[2]);
    let point_residual_bound = combine3_bounds(px_err, py_err, pz_err);

    let res_x = &q[0] - &proj[0];
    let res_y = &q[1] - &proj[1];
    let res_z = &q[2] - &proj[2];
    let sq_dist = &res_x * &res_x + &res_y * &res_y + &res_z * &res_z;
    check_rational_budget(budget, &sq_dist)?;
    let distance_bound = sqrt_up(&sq_dist).map_err(|()| GeometryError::Uncertified {
        reason: "line projection distance is negative (unreachable)".to_owned(),
    })?;

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
    let q = to_rat3(query);
    let o = to_rat3(origin);
    let ua = to_rat3(u_axis);
    let va = to_rat3(v_axis);
    let diff = [&q[0] - &o[0], &q[1] - &o[1], &q[2] - &o[2]];

    let dot = |a: &[BigRational; 3], b: &[BigRational; 3]| -> BigRational {
        &a[0] * &b[0] + &a[1] * &b[1] + &a[2] * &b[2]
    };

    let guu = dot(&ua, &ua);
    let guv = dot(&ua, &va);
    let gvv = dot(&va, &va);
    let rhs_u = dot(&diff, &ua);
    let rhs_v = dot(&diff, &va);

    // Cramer's rule for the 2×2 Gram system.
    let det = &guu * &gvv - &guv * &guv;
    if det.is_zero() {
        return Err(GeometryError::Uncertified {
            reason: "plane axes are exactly degenerate (zero Gram determinant)".to_owned(),
        });
    }
    let u_exact = (&rhs_u * &gvv - &rhs_v * &guv) / &det;
    let v_exact = (&guu * &rhs_v - &guv * &rhs_u) / &det;
    check_rational_budget(budget, &u_exact)?;
    check_rational_budget(budget, &v_exact)?;

    let (u, u_err) = rounding_error_bound(&u_exact);
    let (v, v_err) = rounding_error_bound(&v_exact);
    let parameter_error_bound = u_err.max(v_err);

    let proj = [
        &o[0] + &u_exact * &ua[0] + &v_exact * &va[0],
        &o[1] + &u_exact * &ua[1] + &v_exact * &va[1],
        &o[2] + &u_exact * &ua[2] + &v_exact * &va[2],
    ];
    let (px, px_err) = rounding_error_bound(&proj[0]);
    let (py, py_err) = rounding_error_bound(&proj[1]);
    let (pz, pz_err) = rounding_error_bound(&proj[2]);
    let point_residual_bound = combine3_bounds(px_err, py_err, pz_err);

    let res_x = &q[0] - &proj[0];
    let res_y = &q[1] - &proj[1];
    let res_z = &q[2] - &proj[2];
    let sq_dist = &res_x * &res_x + &res_y * &res_y + &res_z * &res_z;
    check_rational_budget(budget, &sq_dist)?;
    let distance_bound = sqrt_up(&sq_dist).map_err(|()| GeometryError::Uncertified {
        reason: "plane projection distance is negative (unreachable)".to_owned(),
    })?;

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
            "certified trigonometric computation exceeded the certification bit-width budget"
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
/// midpoint to the nearest `f64`, matching the conservative-but-safe style
/// of [`rounding_error_bound`].
fn interval_to_f64_bound(interval: &RatInterval) -> (f64, f64) {
    let two = BigRational::from_integer(BigInt::from(2i64));
    let mid = (&interval.lo + &interval.hi) / &two;
    let (value, round_err) = rounding_error_bound(&mid);
    let half_width = (&interval.hi - &interval.lo) / &two;
    let half_width_bound = rat_to_f64_up(&half_width.abs());
    (value, (round_err + half_width_bound).next_up())
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
    x_axis: [f64; 2],
    y_axis: [f64; 2],
    theta: f64,
) -> Result<ExactCircleEval<2>, GeometryError> {
    let theta_r = f64_to_rat(theta);
    let (sin_i, cos_i) = sin_cos_interval(&theta_r, budget).map_err(trig_err_to_uncertified)?;
    let c = to_rat2(center);
    let xa = to_rat2(x_axis);
    let ya = to_rat2(y_axis);
    let r = f64_to_rat(radius);
    let r_xa = [&r * &xa[0], &r * &xa[1]];
    let r_ya = [&r * &ya[0], &r * &ya[1]];

    // p(θ) = center + r·cos(θ)·x_axis + r·sin(θ)·y_axis
    let offset_x = cos_i.scale(&r_xa[0]).add(&sin_i.scale(&r_ya[0]));
    let offset_y = cos_i.scale(&r_xa[1]).add(&sin_i.scale(&r_ya[1]));
    let pos_x = RatInterval::point(c[0].clone()).add(&offset_x);
    let pos_y = RatInterval::point(c[1].clone()).add(&offset_y);
    check_interval_budget(budget, &pos_x)?;
    check_interval_budget(budget, &pos_y)?;
    let (px, ex) = interval_to_f64_bound(&pos_x);
    let (py, ey) = interval_to_f64_bound(&pos_y);
    finite_or_uncertified(&[px, py], "evaluated circle position overflowed f64 range")?;

    // p′(θ) = r·(−sin(θ)·x_axis + cos(θ)·y_axis)
    let d1_x = cos_i.scale(&r_ya[0]).sub(&sin_i.scale(&r_xa[0]));
    let d1_y = cos_i.scale(&r_ya[1]).sub(&sin_i.scale(&r_xa[1]));
    check_interval_budget(budget, &d1_x)?;
    check_interval_budget(budget, &d1_y)?;
    let (d1x, e1x) = interval_to_f64_bound(&d1_x);
    let (d1y, e1y) = interval_to_f64_bound(&d1_y);
    finite_or_uncertified(
        &[d1x, d1y],
        "evaluated circle first derivative overflowed f64 range",
    )?;

    // p″(θ) = −(p(θ) − center) = −offset
    let d2_x = offset_x.neg();
    let d2_y = offset_y.neg();
    let (d2x, e2x) = interval_to_f64_bound(&d2_x);
    let (d2y, e2y) = interval_to_f64_bound(&d2_y);
    finite_or_uncertified(
        &[d2x, d2y],
        "evaluated circle second derivative overflowed f64 range",
    )?;

    Ok(ExactCircleEval {
        point: [px, py],
        position_error_bound: combine2_bounds(ex, ey),
        first: [d1x, d1y],
        first_error_bound: combine2_bounds(e1x, e1y),
        second: [d2x, d2y],
        second_error_bound: combine2_bounds(e2x, e2y),
    })
}

/// 3-D analogue of [`exact_circle_eval2`].
pub(super) fn exact_circle_eval3(
    budget: CertificationBudget,
    center: [f64; 3],
    radius: f64,
    x_axis: [f64; 3],
    y_axis: [f64; 3],
    theta: f64,
) -> Result<ExactCircleEval<3>, GeometryError> {
    let theta_r = f64_to_rat(theta);
    let (sin_i, cos_i) = sin_cos_interval(&theta_r, budget).map_err(trig_err_to_uncertified)?;
    let c = to_rat3(center);
    let xa = to_rat3(x_axis);
    let ya = to_rat3(y_axis);
    let r = f64_to_rat(radius);
    let r_xa = [&r * &xa[0], &r * &xa[1], &r * &xa[2]];
    let r_ya = [&r * &ya[0], &r * &ya[1], &r * &ya[2]];

    let offset_x = cos_i.scale(&r_xa[0]).add(&sin_i.scale(&r_ya[0]));
    let offset_y = cos_i.scale(&r_xa[1]).add(&sin_i.scale(&r_ya[1]));
    let offset_z = cos_i.scale(&r_xa[2]).add(&sin_i.scale(&r_ya[2]));
    let pos_x = RatInterval::point(c[0].clone()).add(&offset_x);
    let pos_y = RatInterval::point(c[1].clone()).add(&offset_y);
    let pos_z = RatInterval::point(c[2].clone()).add(&offset_z);
    check_interval_budget(budget, &pos_x)?;
    check_interval_budget(budget, &pos_y)?;
    check_interval_budget(budget, &pos_z)?;
    let (px, ex) = interval_to_f64_bound(&pos_x);
    let (py, ey) = interval_to_f64_bound(&pos_y);
    let (pz, ez) = interval_to_f64_bound(&pos_z);
    finite_or_uncertified(
        &[px, py, pz],
        "evaluated circle position overflowed f64 range",
    )?;

    let d1_x = cos_i.scale(&r_ya[0]).sub(&sin_i.scale(&r_xa[0]));
    let d1_y = cos_i.scale(&r_ya[1]).sub(&sin_i.scale(&r_xa[1]));
    let d1_z = cos_i.scale(&r_ya[2]).sub(&sin_i.scale(&r_xa[2]));
    check_interval_budget(budget, &d1_x)?;
    check_interval_budget(budget, &d1_y)?;
    check_interval_budget(budget, &d1_z)?;
    let (d1x, e1x) = interval_to_f64_bound(&d1_x);
    let (d1y, e1y) = interval_to_f64_bound(&d1_y);
    let (d1z, e1z) = interval_to_f64_bound(&d1_z);
    finite_or_uncertified(
        &[d1x, d1y, d1z],
        "evaluated circle first derivative overflowed f64 range",
    )?;

    let d2_x = offset_x.neg();
    let d2_y = offset_y.neg();
    let d2_z = offset_z.neg();
    let (d2x, e2x) = interval_to_f64_bound(&d2_x);
    let (d2y, e2y) = interval_to_f64_bound(&d2_y);
    let (d2z, e2z) = interval_to_f64_bound(&d2_z);
    finite_or_uncertified(
        &[d2x, d2y, d2z],
        "evaluated circle second derivative overflowed f64 range",
    )?;

    Ok(ExactCircleEval {
        point: [px, py, pz],
        position_error_bound: combine3_bounds(ex, ey, ez),
        first: [d1x, d1y, d1z],
        first_error_bound: combine3_bounds(e1x, e1y, e1z),
        second: [d2x, d2y, d2z],
        second_error_bound: combine3_bounds(e2x, e2y, e2z),
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
    numer: &BigRational,
    mag_down: &BigRational,
    mag_up: &BigRational,
) -> RatInterval {
    if numer.is_zero() {
        RatInterval::point(BigRational::zero())
    } else if numer.is_positive() {
        RatInterval {
            lo: numer / mag_up,
            hi: numer / mag_down,
        }
    } else {
        RatInterval {
            lo: numer / mag_down,
            hi: numer / mag_up,
        }
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
    let tau_int = tau_interval(budget).map_err(trig_err_to_uncertified)?;

    // Interior case: the whole interval is provably inside [0, τ) using the
    // certified lower bound tau_int.lo ≤ τ, so no wrap-around is needed. This is
    // the `rem_euclid`-free proof that the representative is already canonical.
    if !theta_interval.lo.is_negative() && theta_interval.hi < tau_int.lo {
        let (canonical, error_bound) = interval_to_f64_bound(theta_interval);
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
        let a = (t_hi - tau_lo).abs();
        let b = (tau_hi - t_lo).abs();
        if a >= b { a } else { b }
    };
    let bound_rat = if dist_zero <= dist_tau {
        dist_zero
    } else {
        dist_tau
    };

    Ok((0.0, rat_to_f64_up(&bound_rat)))
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
    x_axis: [f64; 2],
    y_axis: [f64; 2],
) -> Result<ExactCircleProjection<2>, GeometryError> {
    let q = to_rat2(query);
    let c = to_rat2(center);
    let xa = to_rat2(x_axis);
    let ya = to_rat2(y_axis);
    let r = f64_to_rat(radius);

    let diff = [&q[0] - &c[0], &q[1] - &c[1]];
    let cx = &diff[0] * &xa[0] + &diff[1] * &xa[1];
    let cy = &diff[0] * &ya[0] + &diff[1] * &ya[1];
    check_rational_budget(budget, &cx)?;
    check_rational_budget(budget, &cy)?;

    let sq_inplane = &cx * &cx + &cy * &cy;
    if sq_inplane.is_zero() {
        return Err(GeometryError::Singular);
    }
    check_rational_budget(budget, &sq_inplane)?;

    let gxx = &xa[0] * &xa[0] + &xa[1] * &xa[1];
    let gyy = &ya[0] * &ya[0] + &ya[1] * &ya[1];
    let gxy = &xa[0] * &ya[0] + &xa[1] * &ya[1];
    let det_g = &gxx * &gyy - &gxy * &gxy;
    if !det_g.is_positive() {
        return Err(GeometryError::Uncertified {
            reason: "circle frame Gram determinant non-positive".to_owned(),
        });
    }
    let gram_s = (&gyy * &cx - &gxy * &cy) / &det_g;
    let gram_t = (&gxx * &cy - &gxy * &cx) / &det_g;
    let in_plane_sq = &cx * &gram_s + &cy * &gram_t;
    check_rational_budget(budget, &gram_s)?;
    check_rational_budget(budget, &gram_t)?;
    check_rational_budget(budget, &in_plane_sq)?;

    let mag_down = sqrt_down(&in_plane_sq).map_err(|()| GeometryError::Uncertified {
        reason: "circle projection in-plane magnitude is negative (unreachable)".to_owned(),
    })?;
    let mag_up = sqrt_up(&in_plane_sq).map_err(|()| GeometryError::Uncertified {
        reason: "circle projection in-plane magnitude is negative (unreachable)".to_owned(),
    })?;
    let mag_down_r = f64_to_rat(mag_down);
    let mag_up_r = f64_to_rat(mag_up);
    if mag_down_r.is_zero() {
        return Err(GeometryError::Uncertified {
            reason: "circle projection in-plane magnitude underflows the smallest positive f64; \
                     cannot certify the projected point direction"
                .to_owned(),
        });
    }

    let ux_interval = ratio_by_bracket(&gram_s, &mag_down_r, &mag_up_r);
    let uy_interval = ratio_by_bracket(&gram_t, &mag_down_r, &mag_up_r);
    let point_x = RatInterval::point(c[0].clone()).add(&ux_interval.scale(&(&r * &xa[0])));
    let point_x = point_x.add(&uy_interval.scale(&(&r * &ya[0])));
    let point_y = RatInterval::point(c[1].clone()).add(&ux_interval.scale(&(&r * &xa[1])));
    let point_y = point_y.add(&uy_interval.scale(&(&r * &ya[1])));
    check_interval_budget(budget, &point_x)?;
    check_interval_budget(budget, &point_y)?;

    // Certified distance bound is the full 2-D ‖q − P′‖ to the *returned*
    // point interval, coherent with the emitted point rather than the radial
    // |‖in-plane‖ − r| deviation (which is only exact for an orthonormal frame).
    let qp_x = RatInterval::point(q[0].clone()).sub(&point_x);
    let qp_y = RatInterval::point(q[1].clone()).sub(&point_y);
    let sq_dist = qp_x.mul(&qp_x).add(&qp_y.mul(&qp_y));
    check_interval_budget(budget, &sq_dist)?;
    let distance_bound = sqrt_up(&sq_dist.hi).map_err(|()| GeometryError::Uncertified {
        reason: "circle projection distance is negative (unreachable)".to_owned(),
    })?;

    let (px, ex) = interval_to_f64_bound(&point_x);
    let (py, ey) = interval_to_f64_bound(&point_y);
    let point_residual_bound = combine2_bounds(ex, ey);
    finite_or_uncertified(
        &[px, py, distance_bound],
        "circle projection overflowed f64 range",
    )?;

    let theta_interval = atan2_interval(&cy, &cx, budget).map_err(trig_err_to_uncertified)?;
    let theta_final = if cy.is_negative() {
        let tau = tau_interval(budget).map_err(trig_err_to_uncertified)?;
        theta_interval.add(&tau)
    } else {
        theta_interval
    };
    let (theta, theta_err) = certified_periodic_param(&theta_final, budget)?;

    Ok(ExactCircleProjection {
        parameter: theta,
        parameter_error_bound: theta_err,
        point: [px, py],
        point_residual_bound,
        distance_bound,
    })
}

/// 3-D analogue of [`exact_circle_project2`].
///
/// The out-of-plane distance is computed via `||diff||² − in_plane_sq`
/// (where `in_plane_sq` is the Gram-projected in-plane squared distance),
/// rather than `(diff·normal)²`. This is correct regardless of whether the
/// stored normal is exactly unit or orthogonal to the x/y axes.
///
/// # Errors
///
/// See [`exact_circle_project2`].
pub(super) fn exact_circle_project3(
    budget: CertificationBudget,
    query: [f64; 3],
    center: [f64; 3],
    radius: f64,
    x_axis: [f64; 3],
    y_axis: [f64; 3],
) -> Result<ExactCircleProjection<3>, GeometryError> {
    let q = to_rat3(query);
    let c = to_rat3(center);
    let xa = to_rat3(x_axis);
    let ya = to_rat3(y_axis);
    let r = f64_to_rat(radius);

    let diff = [&q[0] - &c[0], &q[1] - &c[1], &q[2] - &c[2]];
    let cx = &diff[0] * &xa[0] + &diff[1] * &xa[1] + &diff[2] * &xa[2];
    let cy = &diff[0] * &ya[0] + &diff[1] * &ya[1] + &diff[2] * &ya[2];
    check_rational_budget(budget, &cx)?;
    check_rational_budget(budget, &cy)?;

    let sq_inplane = &cx * &cx + &cy * &cy;
    if sq_inplane.is_zero() {
        return Err(GeometryError::Singular);
    }
    check_rational_budget(budget, &sq_inplane)?;

    let gxx = &xa[0] * &xa[0] + &xa[1] * &xa[1] + &xa[2] * &xa[2];
    let gyy = &ya[0] * &ya[0] + &ya[1] * &ya[1] + &ya[2] * &ya[2];
    let gxy = &xa[0] * &ya[0] + &xa[1] * &ya[1] + &xa[2] * &ya[2];
    let det_g = &gxx * &gyy - &gxy * &gxy;
    if !det_g.is_positive() {
        return Err(GeometryError::Uncertified {
            reason: "circle frame Gram determinant non-positive".to_owned(),
        });
    }
    let gram_s = (&gyy * &cx - &gxy * &cy) / &det_g;
    let gram_t = (&gxx * &cy - &gxy * &cx) / &det_g;
    let in_plane_sq = &cx * &gram_s + &cy * &gram_t;
    check_rational_budget(budget, &gram_s)?;
    check_rational_budget(budget, &gram_t)?;
    check_rational_budget(budget, &in_plane_sq)?;

    let mag_down = sqrt_down(&in_plane_sq).map_err(|()| GeometryError::Uncertified {
        reason: "circle projection in-plane magnitude is negative (unreachable)".to_owned(),
    })?;
    let mag_up = sqrt_up(&in_plane_sq).map_err(|()| GeometryError::Uncertified {
        reason: "circle projection in-plane magnitude is negative (unreachable)".to_owned(),
    })?;
    let mag_down_r = f64_to_rat(mag_down);
    let mag_up_r = f64_to_rat(mag_up);
    if mag_down_r.is_zero() {
        return Err(GeometryError::Uncertified {
            reason: "circle projection in-plane magnitude underflows the smallest positive f64; \
                     cannot certify the projected point direction"
                .to_owned(),
        });
    }

    // Out-of-plane squared distance: use ||diff||² - in_plane_sq (exact,
    // independent of the stored normal direction). This correctly handles
    // non-unit/non-orthogonal stored axes without relying on a `cz = diff·normal`
    // component that would be incorrect for approximate frames.
    let sq_diff = &diff[0] * &diff[0] + &diff[1] * &diff[1] + &diff[2] * &diff[2];
    let sq_out_of_plane = &sq_diff - &in_plane_sq;
    check_rational_budget(budget, &sq_out_of_plane)?;
    // A negative out-of-plane residual is geometrically impossible: it means
    // the Gram in-plane component exceeds the total, which only happens when
    // the stored frame is too far from orthonormal to certify. Reject rather
    // than clamp to zero — clamping would silently emit a false certificate.
    if sq_out_of_plane.is_negative() {
        return Err(GeometryError::Uncertified {
            reason: "circle3 Gram projection yields an impossible negative out-of-plane residual"
                .to_owned(),
        });
    }

    let ux_interval = ratio_by_bracket(&gram_s, &mag_down_r, &mag_up_r);
    let uy_interval = ratio_by_bracket(&gram_t, &mag_down_r, &mag_up_r);
    let point_x = RatInterval::point(c[0].clone()).add(&ux_interval.scale(&(&r * &xa[0])));
    let point_x = point_x.add(&uy_interval.scale(&(&r * &ya[0])));
    let point_y = RatInterval::point(c[1].clone()).add(&ux_interval.scale(&(&r * &xa[1])));
    let point_y = point_y.add(&uy_interval.scale(&(&r * &ya[1])));
    let point_z = RatInterval::point(c[2].clone()).add(&ux_interval.scale(&(&r * &xa[2])));
    let point_z = point_z.add(&uy_interval.scale(&(&r * &ya[2])));
    check_interval_budget(budget, &point_x)?;
    check_interval_budget(budget, &point_y)?;
    check_interval_budget(budget, &point_z)?;

    // Certified distance bound is the full 3-D ‖q − P′‖ to the *returned*
    // point interval, coherent with the emitted point (not a radial-plus-
    // out-of-plane bound around an idealized in-plane foot).
    let qp_x = RatInterval::point(q[0].clone()).sub(&point_x);
    let qp_y = RatInterval::point(q[1].clone()).sub(&point_y);
    let qp_z = RatInterval::point(q[2].clone()).sub(&point_z);
    let sq_dist = qp_x.mul(&qp_x).add(&qp_y.mul(&qp_y)).add(&qp_z.mul(&qp_z));
    check_interval_budget(budget, &sq_dist)?;
    let distance_bound = sqrt_up(&sq_dist.hi).map_err(|()| GeometryError::Uncertified {
        reason: "circle projection distance is negative (unreachable)".to_owned(),
    })?;

    let (px, ex) = interval_to_f64_bound(&point_x);
    let (py, ey) = interval_to_f64_bound(&point_y);
    let (pz, ez) = interval_to_f64_bound(&point_z);
    let point_residual_bound = combine3_bounds(ex, ey, ez);
    finite_or_uncertified(
        &[px, py, pz, distance_bound],
        "circle projection overflowed f64 range",
    )?;

    let theta_interval = atan2_interval(&cy, &cx, budget).map_err(trig_err_to_uncertified)?;
    let theta_final = if cy.is_negative() {
        let tau = tau_interval(budget).map_err(trig_err_to_uncertified)?;
        theta_interval.add(&tau)
    } else {
        theta_interval
    };
    let (theta, theta_err) = certified_periodic_param(&theta_final, budget)?;

    Ok(ExactCircleProjection {
        parameter: theta,
        parameter_error_bound: theta_err,
        point: [px, py, pz],
        point_residual_bound,
        distance_bound,
    })
}

/// The result of a certified cylinder surface evaluation at `(u, v)`.
///
/// `du`/`duu` come from the same certified `sin`/`cos` enclosure used for
/// [`ExactCircleEval`]; `dv = axis_dir` (exact, zero error) and `duv = dvv =
/// 0` (exact) are not part of this struct since `p(u,v) = axis_origin +
/// v·axis_dir + r·cos(u)·x_axis + r·sin(u)·y_axis` is affine in `v` — the
/// caller supplies those directly.
pub(super) struct ExactCylinderEval {
    pub point: [f64; 3],
    pub position_error_bound: f64,
    pub du: [f64; 3],
    pub du_error_bound: f64,
    pub duu: [f64; 3],
    pub duu_error_bound: f64,
}

/// Certified evaluation of a cylinder surface at `(u, v)`.
///
/// # Errors
///
/// Returns [`GeometryError::Uncertified`] if the certified trig computation
/// exceeds its budget, or any intermediate value overflows `f64` range.
#[allow(clippy::too_many_arguments)]
pub(super) fn exact_cylinder_eval(
    budget: CertificationBudget,
    axis_origin: [f64; 3],
    axis_dir: [f64; 3],
    radius: f64,
    x_axis: [f64; 3],
    y_axis: [f64; 3],
    u: f64,
    v: f64,
) -> Result<ExactCylinderEval, GeometryError> {
    let u_r = f64_to_rat(u);
    let (sin_i, cos_i) = sin_cos_interval(&u_r, budget).map_err(trig_err_to_uncertified)?;
    let o = to_rat3(axis_origin);
    let ad = to_rat3(axis_dir);
    let xa = to_rat3(x_axis);
    let ya = to_rat3(y_axis);
    let r = f64_to_rat(radius);
    let v_r = f64_to_rat(v);
    let r_xa = [&r * &xa[0], &r * &xa[1], &r * &xa[2]];
    let r_ya = [&r * &ya[0], &r * &ya[1], &r * &ya[2]];

    // p(u,v) = axis_origin + v·axis_dir + r·cos(u)·x_axis + r·sin(u)·y_axis
    let offset_x = cos_i.scale(&r_xa[0]).add(&sin_i.scale(&r_ya[0]));
    let offset_y = cos_i.scale(&r_xa[1]).add(&sin_i.scale(&r_ya[1]));
    let offset_z = cos_i.scale(&r_xa[2]).add(&sin_i.scale(&r_ya[2]));
    let base_x = &o[0] + &v_r * &ad[0];
    let base_y = &o[1] + &v_r * &ad[1];
    let base_z = &o[2] + &v_r * &ad[2];
    let pos_x = RatInterval::point(base_x).add(&offset_x);
    let pos_y = RatInterval::point(base_y).add(&offset_y);
    let pos_z = RatInterval::point(base_z).add(&offset_z);
    check_interval_budget(budget, &pos_x)?;
    check_interval_budget(budget, &pos_y)?;
    check_interval_budget(budget, &pos_z)?;
    let (px, ex) = interval_to_f64_bound(&pos_x);
    let (py, ey) = interval_to_f64_bound(&pos_y);
    let (pz, ez) = interval_to_f64_bound(&pos_z);
    finite_or_uncertified(
        &[px, py, pz],
        "evaluated cylinder position overflowed f64 range",
    )?;

    // ∂p/∂u = r·(−sin(u)·x_axis + cos(u)·y_axis)
    let d1_x = cos_i.scale(&r_ya[0]).sub(&sin_i.scale(&r_xa[0]));
    let d1_y = cos_i.scale(&r_ya[1]).sub(&sin_i.scale(&r_xa[1]));
    let d1_z = cos_i.scale(&r_ya[2]).sub(&sin_i.scale(&r_xa[2]));
    check_interval_budget(budget, &d1_x)?;
    check_interval_budget(budget, &d1_y)?;
    check_interval_budget(budget, &d1_z)?;
    let (d1x, e1x) = interval_to_f64_bound(&d1_x);
    let (d1y, e1y) = interval_to_f64_bound(&d1_y);
    let (d1z, e1z) = interval_to_f64_bound(&d1_z);
    finite_or_uncertified(
        &[d1x, d1y, d1z],
        "evaluated cylinder first u-derivative overflowed f64 range",
    )?;

    // ∂²p/∂u² = −r·(cos(u)·x_axis + sin(u)·y_axis) = −offset
    let d2_x = offset_x.neg();
    let d2_y = offset_y.neg();
    let d2_z = offset_z.neg();
    let (d2x, e2x) = interval_to_f64_bound(&d2_x);
    let (d2y, e2y) = interval_to_f64_bound(&d2_y);
    let (d2z, e2z) = interval_to_f64_bound(&d2_z);
    finite_or_uncertified(
        &[d2x, d2y, d2z],
        "evaluated cylinder second u-derivative overflowed f64 range",
    )?;

    Ok(ExactCylinderEval {
        point: [px, py, pz],
        position_error_bound: combine3_bounds(ex, ey, ez),
        du: [d1x, d1y, d1z],
        du_error_bound: combine3_bounds(e1x, e1y, e1z),
        duu: [d2x, d2y, d2z],
        duu_error_bound: combine3_bounds(e2x, e2y, e2z),
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

/// Certified projection of `query` onto a cylinder surface.
///
/// The axial coordinate `v = ((q − axis_origin)·axis_dir) / (axis_dir·axis_dir)`
/// is the exact least-squares projection parameter — this correctly handles
/// stored `axis_dir` that is only approximately unit (dividing by
/// `axis_dir·axis_dir` removes the norm-deviation error that would otherwise
/// corrupt `v`). The in-plane part uses a 2×2 Gram matrix as in the circle
/// projection.
///
/// # Errors
///
/// Returns [`GeometryError::Singular`] when `query` lies exactly on the
/// cylinder axis (no unique nearest point / undefined angle). Returns
/// [`GeometryError::Uncertified`] if the certified trig computation exceeds
/// its budget, the in-plane magnitude underflows the smallest positive
/// `f64`, or any intermediate value overflows `f64` range.
#[allow(clippy::too_many_lines)]
pub(super) fn exact_cylinder_project(
    budget: CertificationBudget,
    query: [f64; 3],
    axis_origin: [f64; 3],
    axis_dir: [f64; 3],
    radius: f64,
    x_axis: [f64; 3],
    y_axis: [f64; 3],
) -> Result<ExactCylinderProjection, GeometryError> {
    let q = to_rat3(query);
    let o = to_rat3(axis_origin);
    let ad = to_rat3(axis_dir);
    let xa = to_rat3(x_axis);
    let ya = to_rat3(y_axis);
    let r = f64_to_rat(radius);

    let diff = [&q[0] - &o[0], &q[1] - &o[1], &q[2] - &o[2]];

    // Correct axial parameter: (diff · axis_dir) / ||axis_dir||².
    // For an exactly-unit axis this is identical to diff·axis_dir, but for
    // stored UnitVector axes that are only approximately unit this removes
    // the norm-deviation error from v.
    let dot_diff_ad = &diff[0] * &ad[0] + &diff[1] * &ad[1] + &diff[2] * &ad[2];
    let ad_sq = &ad[0] * &ad[0] + &ad[1] * &ad[1] + &ad[2] * &ad[2];
    if ad_sq.is_zero() {
        return Err(GeometryError::Uncertified {
            reason: "cylinder axis has exact zero magnitude".to_owned(),
        });
    }
    let v_exact = &dot_diff_ad / &ad_sq;
    check_rational_budget(budget, &v_exact)?;
    let (v, v_err) = rounding_error_bound(&v_exact);

    let cx = &diff[0] * &xa[0] + &diff[1] * &xa[1] + &diff[2] * &xa[2];
    let cy = &diff[0] * &ya[0] + &diff[1] * &ya[1] + &diff[2] * &ya[2];
    check_rational_budget(budget, &cx)?;
    check_rational_budget(budget, &cy)?;

    let sq_inplane = &cx * &cx + &cy * &cy;
    if sq_inplane.is_zero() {
        return Err(GeometryError::Singular);
    }
    check_rational_budget(budget, &sq_inplane)?;

    let gxx = &xa[0] * &xa[0] + &xa[1] * &xa[1] + &xa[2] * &xa[2];
    let gyy = &ya[0] * &ya[0] + &ya[1] * &ya[1] + &ya[2] * &ya[2];
    let gxy = &xa[0] * &ya[0] + &xa[1] * &ya[1] + &xa[2] * &ya[2];
    let det_g = &gxx * &gyy - &gxy * &gxy;
    if !det_g.is_positive() {
        return Err(GeometryError::Uncertified {
            reason: "cylinder frame Gram determinant non-positive".to_owned(),
        });
    }
    let gram_s = (&gyy * &cx - &gxy * &cy) / &det_g;
    let gram_t = (&gxx * &cy - &gxy * &cx) / &det_g;
    let in_plane_sq = &cx * &gram_s + &cy * &gram_t;
    check_rational_budget(budget, &gram_s)?;
    check_rational_budget(budget, &gram_t)?;
    check_rational_budget(budget, &in_plane_sq)?;

    let mag_down = sqrt_down(&in_plane_sq).map_err(|()| GeometryError::Uncertified {
        reason: "cylinder projection in-plane magnitude is negative (unreachable)".to_owned(),
    })?;
    let mag_up = sqrt_up(&in_plane_sq).map_err(|()| GeometryError::Uncertified {
        reason: "cylinder projection in-plane magnitude is negative (unreachable)".to_owned(),
    })?;
    let mag_down_r = f64_to_rat(mag_down);
    let mag_up_r = f64_to_rat(mag_up);
    if mag_down_r.is_zero() {
        return Err(GeometryError::Uncertified {
            reason: "cylinder projection in-plane magnitude underflows the smallest positive \
                     f64; cannot certify the projected point direction"
                .to_owned(),
        });
    }
    let ux_interval = ratio_by_bracket(&gram_s, &mag_down_r, &mag_up_r);
    let uy_interval = ratio_by_bracket(&gram_t, &mag_down_r, &mag_up_r);
    let base_x = &o[0] + &v_exact * &ad[0];
    let base_y = &o[1] + &v_exact * &ad[1];
    let base_z = &o[2] + &v_exact * &ad[2];
    let point_x = RatInterval::point(base_x).add(&ux_interval.scale(&(&r * &xa[0])));
    let point_x = point_x.add(&uy_interval.scale(&(&r * &ya[0])));
    let point_y = RatInterval::point(base_y).add(&ux_interval.scale(&(&r * &xa[1])));
    let point_y = point_y.add(&uy_interval.scale(&(&r * &ya[1])));
    let point_z = RatInterval::point(base_z).add(&ux_interval.scale(&(&r * &xa[2])));
    let point_z = point_z.add(&uy_interval.scale(&(&r * &ya[2])));
    check_interval_budget(budget, &point_x)?;
    check_interval_budget(budget, &point_y)?;
    check_interval_budget(budget, &point_z)?;

    // Certified distance bound is the full 3-D ‖q − P′‖ to the *returned*
    // point interval — not just the radial `|‖in-plane‖ − r|` deviation. A
    // radial-only bound is a false certificate whenever the stored frame is
    // not exactly orthonormal: the returned point can carry an out-of-plane
    // component (e.g. from an x-axis with a small axial leak) that a radial
    // bound ignores.
    let qp_x = RatInterval::point(q[0].clone()).sub(&point_x);
    let qp_y = RatInterval::point(q[1].clone()).sub(&point_y);
    let qp_z = RatInterval::point(q[2].clone()).sub(&point_z);
    let sq_dist = qp_x.mul(&qp_x).add(&qp_y.mul(&qp_y)).add(&qp_z.mul(&qp_z));
    check_interval_budget(budget, &sq_dist)?;
    let distance_bound = sqrt_up(&sq_dist.hi).map_err(|()| GeometryError::Uncertified {
        reason: "cylinder projection distance is negative (unreachable)".to_owned(),
    })?;

    let (px, ex) = interval_to_f64_bound(&point_x);
    let (py, ey) = interval_to_f64_bound(&point_y);
    let (pz, ez) = interval_to_f64_bound(&point_z);
    let point_residual_bound = combine3_bounds(ex, ey, ez);
    finite_or_uncertified(
        &[px, py, pz, distance_bound],
        "cylinder projection overflowed f64 range",
    )?;

    let theta_interval = atan2_interval(&cy, &cx, budget).map_err(trig_err_to_uncertified)?;
    let theta_final = if cy.is_negative() {
        let tau = tau_interval(budget).map_err(trig_err_to_uncertified)?;
        theta_interval.add(&tau)
    } else {
        theta_interval
    };
    let (u, u_err) = certified_periodic_param(&theta_final, budget)?;
    finite_or_uncertified(
        &[u, v],
        "cylinder projection parameter overflowed f64 range",
    )?;

    Ok(ExactCylinderProjection {
        u,
        v,
        u_error_bound: u_err,
        v_error_bound: v_err,
        point: [px, py, pz],
        point_residual_bound,
        distance_bound,
    })
}

/// The result of a certified cone evaluation at `(u, v)` under the
/// parameterization `p(u,v) = apex + v·axis + v·tan(α)·(cos(u)·x_axis +
/// sin(u)·y_axis)` (see the `cone` module docs). `dvv = 0` exactly and is
/// not part of this struct (supplied directly by the caller, as with
/// [`ExactCylinderEval`]'s `dv`).
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

/// Certified evaluation of a cone surface at `(u, v)`.
///
/// # Errors
///
/// Returns [`GeometryError::Uncertified`] if the certified trig computation
/// exceeds its budget, `tan(half_angle) = sin(half_angle)/cos(half_angle)`
/// cannot be certified (practically unreachable since `half_angle ∈ (0,
/// π/2)` strictly, so the certified `cos(half_angle)` interval cannot
/// straddle zero at any budget large enough to represent `half_angle`
/// itself), or any intermediate value overflows `f64` range.
#[allow(clippy::too_many_arguments)]
pub(super) fn exact_cone_eval(
    budget: CertificationBudget,
    apex: [f64; 3],
    axis: [f64; 3],
    half_angle: f64,
    x_axis: [f64; 3],
    y_axis: [f64; 3],
    u: f64,
    v: f64,
) -> Result<ExactConeEval, GeometryError> {
    let u_r = f64_to_rat(u);
    let a_r = f64_to_rat(half_angle);
    let (sin_u, cos_u) = sin_cos_interval(&u_r, budget).map_err(trig_err_to_uncertified)?;
    let (sin_a, cos_a) = sin_cos_interval(&a_r, budget).map_err(trig_err_to_uncertified)?;
    let tan_a = sin_a.div(&cos_a).map_err(trig_err_to_uncertified)?;
    check_interval_budget(budget, &tan_a)?;

    let ap = to_rat3(apex);
    let ax = to_rat3(axis);
    let xa = to_rat3(x_axis);
    let ya = to_rat3(y_axis);
    let v_r = f64_to_rat(v);

    // dir(u) = cos(u)·x_axis + sin(u)·y_axis (unit azimuthal direction)
    let dir_x = cos_u.scale(&xa[0]).add(&sin_u.scale(&ya[0]));
    let dir_y = cos_u.scale(&xa[1]).add(&sin_u.scale(&ya[1]));
    let dir_z = cos_u.scale(&xa[2]).add(&sin_u.scale(&ya[2]));

    // perp(u) = dir′(u) = −sin(u)·x_axis + cos(u)·y_axis
    let perp_x = cos_u.scale(&ya[0]).sub(&sin_u.scale(&xa[0]));
    let perp_y = cos_u.scale(&ya[1]).sub(&sin_u.scale(&xa[1]));
    let perp_z = cos_u.scale(&ya[2]).sub(&sin_u.scale(&xa[2]));

    // v_tan_a = v·tan(α)
    let v_tan_a = tan_a.scale(&v_r);

    // offset = v·tan(α)·dir(u)
    let offset_x = v_tan_a.mul(&dir_x);
    let offset_y = v_tan_a.mul(&dir_y);
    let offset_z = v_tan_a.mul(&dir_z);

    // p(u,v) = apex + v·axis + offset
    let base_x = &ap[0] + &v_r * &ax[0];
    let base_y = &ap[1] + &v_r * &ax[1];
    let base_z = &ap[2] + &v_r * &ax[2];
    let pos_x = RatInterval::point(base_x).add(&offset_x);
    let pos_y = RatInterval::point(base_y).add(&offset_y);
    let pos_z = RatInterval::point(base_z).add(&offset_z);
    check_interval_budget(budget, &pos_x)?;
    check_interval_budget(budget, &pos_y)?;
    check_interval_budget(budget, &pos_z)?;
    let (px, ex) = interval_to_f64_bound(&pos_x);
    let (py, ey) = interval_to_f64_bound(&pos_y);
    let (pz, ez) = interval_to_f64_bound(&pos_z);
    finite_or_uncertified(
        &[px, py, pz],
        "evaluated cone position overflowed f64 range",
    )?;

    // ∂p/∂u = v·tan(α)·perp(u)
    let d1_x = v_tan_a.mul(&perp_x);
    let d1_y = v_tan_a.mul(&perp_y);
    let d1_z = v_tan_a.mul(&perp_z);
    check_interval_budget(budget, &d1_x)?;
    check_interval_budget(budget, &d1_y)?;
    check_interval_budget(budget, &d1_z)?;
    let (d1x, e1x) = interval_to_f64_bound(&d1_x);
    let (d1y, e1y) = interval_to_f64_bound(&d1_y);
    let (d1z, e1z) = interval_to_f64_bound(&d1_z);
    finite_or_uncertified(
        &[d1x, d1y, d1z],
        "evaluated cone first u-derivative overflowed f64 range",
    )?;

    // ∂p/∂v = axis + tan(α)·dir(u)
    let dv_x = RatInterval::point(ax[0].clone()).add(&tan_a.mul(&dir_x));
    let dv_y = RatInterval::point(ax[1].clone()).add(&tan_a.mul(&dir_y));
    let dv_z = RatInterval::point(ax[2].clone()).add(&tan_a.mul(&dir_z));
    check_interval_budget(budget, &dv_x)?;
    check_interval_budget(budget, &dv_y)?;
    check_interval_budget(budget, &dv_z)?;
    let (dvx, evx) = interval_to_f64_bound(&dv_x);
    let (dvy, evy) = interval_to_f64_bound(&dv_y);
    let (dvz, evz) = interval_to_f64_bound(&dv_z);
    finite_or_uncertified(
        &[dvx, dvy, dvz],
        "evaluated cone first v-derivative overflowed f64 range",
    )?;

    // ∂²p/∂u² = −v·tan(α)·dir(u) = −offset
    let d2_x = offset_x.neg();
    let d2_y = offset_y.neg();
    let d2_z = offset_z.neg();
    let (d2x, f2x) = interval_to_f64_bound(&d2_x);
    let (d2y, f2y) = interval_to_f64_bound(&d2_y);
    let (d2z, f2z) = interval_to_f64_bound(&d2_z);
    finite_or_uncertified(
        &[d2x, d2y, d2z],
        "evaluated cone second u-derivative overflowed f64 range",
    )?;

    // ∂²p/∂u∂v = tan(α)·perp(u)
    let duv_x = tan_a.mul(&perp_x);
    let duv_y = tan_a.mul(&perp_y);
    let duv_z = tan_a.mul(&perp_z);
    check_interval_budget(budget, &duv_x)?;
    check_interval_budget(budget, &duv_y)?;
    check_interval_budget(budget, &duv_z)?;
    let (duvx, g2x) = interval_to_f64_bound(&duv_x);
    let (duvy, g2y) = interval_to_f64_bound(&duv_y);
    let (duvz, g2z) = interval_to_f64_bound(&duv_z);
    finite_or_uncertified(
        &[duvx, duvy, duvz],
        "evaluated cone mixed second derivative overflowed f64 range",
    )?;

    Ok(ExactConeEval {
        point: [px, py, pz],
        position_error_bound: combine3_bounds(ex, ey, ez),
        du: [d1x, d1y, d1z],
        du_error_bound: combine3_bounds(e1x, e1y, e1z),
        dv: [dvx, dvy, dvz],
        dv_error_bound: combine3_bounds(evx, evy, evz),
        duu: [d2x, d2y, d2z],
        duu_error_bound: combine3_bounds(f2x, f2y, f2z),
        duv: [duvx, duvy, duvz],
        duv_error_bound: combine3_bounds(g2x, g2y, g2z),
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

/// Certified projection of `query` onto a cone surface (either nappe).
///
/// # Nappe selection
///
/// The correct nappe (sign of the reported `v`) is `s = sign(h)`, where
/// `h = (q − apex)·axis` is the exact axial coordinate of `query`
/// relative to the apex. When `h = 0` exactly, both nappes are equidistant
/// and this helper returns both certified projections.
///
/// # Errors
///
/// Returns [`GeometryError::Singular`] when `query` lies exactly on the
/// cone's axis (`radial_sq = 0`; no unique nearest point or azimuthal
/// angle). Returns [`GeometryError::Uncertified`] if the certified trig
/// computation exceeds its budget, the in-plane magnitude underflows the
/// smallest positive `f64`, or any intermediate value overflows `f64`
/// range.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn exact_cone_project_nappe(
    budget: CertificationBudget,
    query: [f64; 3],
    apex: [f64; 3],
    axis: [f64; 3],
    half_angle: f64,
    x_axis: [f64; 3],
    y_axis: [f64; 3],
    s_positive: bool,
) -> Result<ExactConeProjection, GeometryError> {
    let q = to_rat3(query);
    let ap = to_rat3(apex);
    let ax = to_rat3(axis);
    let xa = to_rat3(x_axis);
    let ya = to_rat3(y_axis);
    let a_r = f64_to_rat(half_angle);

    let diff = [&q[0] - &ap[0], &q[1] - &ap[1], &q[2] - &ap[2]];
    // Correct axial coordinate: (diff · axis) / ||axis||².
    // Dividing by the norm-square removes axis-deviation error in h.
    let dot_diff_ax = &diff[0] * &ax[0] + &diff[1] * &ax[1] + &diff[2] * &ax[2];
    let ax_sq = &ax[0] * &ax[0] + &ax[1] * &ax[1] + &ax[2] * &ax[2];
    if ax_sq.is_zero() {
        return Err(GeometryError::Uncertified {
            reason: "cone axis has exact zero magnitude".to_owned(),
        });
    }
    let h = &dot_diff_ax / &ax_sq;
    let cx = &diff[0] * &xa[0] + &diff[1] * &xa[1] + &diff[2] * &xa[2];
    let cy = &diff[0] * &ya[0] + &diff[1] * &ya[1] + &diff[2] * &ya[2];
    check_rational_budget(budget, &h)?;
    check_rational_budget(budget, &cx)?;
    check_rational_budget(budget, &cy)?;

    let radial_sq = &cx * &cx + &cy * &cy;
    if radial_sq.is_zero() {
        return Err(GeometryError::Singular);
    }
    check_rational_budget(budget, &radial_sq)?;

    let gxx = &xa[0] * &xa[0] + &xa[1] * &xa[1] + &xa[2] * &xa[2];
    let gyy = &ya[0] * &ya[0] + &ya[1] * &ya[1] + &ya[2] * &ya[2];
    let gxy = &xa[0] * &ya[0] + &xa[1] * &ya[1] + &xa[2] * &ya[2];
    let det_g = &gxx * &gyy - &gxy * &gxy;
    if !det_g.is_positive() {
        return Err(GeometryError::Uncertified {
            reason: "cone frame Gram determinant non-positive".to_owned(),
        });
    }
    let gram_s = (&gyy * &cx - &gxy * &cy) / &det_g;
    let gram_t = (&gxx * &cy - &gxy * &cx) / &det_g;
    let in_plane_sq = &cx * &gram_s + &cy * &gram_t;
    check_rational_budget(budget, &gram_s)?;
    check_rational_budget(budget, &gram_t)?;
    check_rational_budget(budget, &in_plane_sq)?;

    let mag_down = sqrt_down(&in_plane_sq).map_err(|()| GeometryError::Uncertified {
        reason: "cone projection in-plane magnitude is negative (unreachable)".to_owned(),
    })?;
    let mag_up = sqrt_up(&in_plane_sq).map_err(|()| GeometryError::Uncertified {
        reason: "cone projection in-plane magnitude is negative (unreachable)".to_owned(),
    })?;
    let mag_down_r = f64_to_rat(mag_down);
    let mag_up_r = f64_to_rat(mag_up);
    if mag_down_r.is_zero() {
        return Err(GeometryError::Uncertified {
            reason: "cone projection in-plane magnitude underflows the smallest positive f64; \
                     cannot certify the projected point direction"
                .to_owned(),
        });
    }
    let radial_mag = RatInterval {
        lo: mag_down_r.clone(),
        hi: mag_up_r.clone(),
    };

    let (sin_a, cos_a) = sin_cos_interval(&a_r, budget).map_err(trig_err_to_uncertified)?;

    let h_abs = h.abs();
    let t_star = cos_a.scale(&h_abs).add(&sin_a.mul(&radial_mag));
    check_interval_budget(budget, &t_star)?;

    let s_val = if s_positive {
        BigRational::one()
    } else {
        -BigRational::one()
    };

    let v_interval = t_star.mul(&cos_a).scale(&s_val);
    let rho = t_star.mul(&sin_a);
    check_interval_budget(budget, &v_interval)?;
    check_interval_budget(budget, &rho)?;

    let ux = ratio_by_bracket(&gram_s, &mag_down_r, &mag_up_r);
    let uy = ratio_by_bracket(&gram_t, &mag_down_r, &mag_up_r);

    let point_x = RatInterval::point(ap[0].clone())
        .add(&v_interval.scale(&ax[0]))
        .add(&rho.mul(&ux).scale(&xa[0]))
        .add(&rho.mul(&uy).scale(&ya[0]));
    let point_y = RatInterval::point(ap[1].clone())
        .add(&v_interval.scale(&ax[1]))
        .add(&rho.mul(&ux).scale(&xa[1]))
        .add(&rho.mul(&uy).scale(&ya[1]));
    let point_z = RatInterval::point(ap[2].clone())
        .add(&v_interval.scale(&ax[2]))
        .add(&rho.mul(&ux).scale(&xa[2]))
        .add(&rho.mul(&uy).scale(&ya[2]));
    check_interval_budget(budget, &point_x)?;
    check_interval_budget(budget, &point_y)?;
    check_interval_budget(budget, &point_z)?;
    let (px, ex) = interval_to_f64_bound(&point_x);
    let (py, ey) = interval_to_f64_bound(&point_y);
    let (pz, ez) = interval_to_f64_bound(&point_z);
    let point_residual_bound = combine3_bounds(ex, ey, ez);

    // Certified distance bound is the full 3-D ‖q − P′‖ to the *returned*
    // point interval, coherent with the emitted point. The previous
    // `h² + ‖in-plane‖² − t*²` foot-of-perpendicular form is only valid for an
    // exactly orthonormal stored frame; the direct q-to-point distance stays
    // certified even when the stored axes leak slightly out of the ideal cone.
    let qp_x = RatInterval::point(q[0].clone()).sub(&point_x);
    let qp_y = RatInterval::point(q[1].clone()).sub(&point_y);
    let qp_z = RatInterval::point(q[2].clone()).sub(&point_z);
    let sq_dist = qp_x.mul(&qp_x).add(&qp_y.mul(&qp_y)).add(&qp_z.mul(&qp_z));
    check_interval_budget(budget, &sq_dist)?;
    let distance_bound = sqrt_up(&sq_dist.hi).map_err(|()| GeometryError::Uncertified {
        reason: "cone projection distance is negative (unreachable)".to_owned(),
    })?;
    finite_or_uncertified(
        &[px, py, pz, distance_bound],
        "cone projection overflowed f64 range",
    )?;

    // u requires a genuine +π shift for the s = −1 nappe (see module docs);
    // this is applied by flipping the sign of both in-plane components
    // before calling atan2 (which is scale-invariant), then wrapping
    // (−π, π] → [0, 2π) using the exact sign of the (possibly flipped)
    // y-component — never by comparing against an approximate `pi_interval`
    // midpoint.
    let ecx = &cx * &s_val;
    let ecy = &cy * &s_val;
    let theta_interval = atan2_interval(&ecy, &ecx, budget).map_err(trig_err_to_uncertified)?;
    let theta_final = if ecy.is_negative() {
        let tau = tau_interval(budget).map_err(trig_err_to_uncertified)?;
        theta_interval.add(&tau)
    } else {
        theta_interval
    };
    let (u, u_err) = certified_periodic_param(&theta_final, budget)?;
    let (v, v_err) = interval_to_f64_bound(&v_interval);
    finite_or_uncertified(&[u, v], "cone projection parameter overflowed f64 range")?;

    Ok(ExactConeProjection {
        u,
        v,
        u_error_bound: u_err,
        v_error_bound: v_err,
        point: [px, py, pz],
        point_residual_bound,
        distance_bound,
    })
}

/// Certified projection of `query` onto the nearest nappe of a cone surface.
///
/// # Nappe selection via the exact axial sign
///
/// The nappe sign is chosen from the **exact** axial coordinate
/// `h = ((q−apex)·axis) / ‖axis‖²`, computed in `BigRational`:
/// - `h > 0` exactly: positive nappe is the global minimum → return primary only.
/// - `h < 0` exactly: negative nappe is the global minimum → return secondary only.
/// - `h = 0` exactly: both nappes are equidistant → return both, positive first.
///
/// This is a rigorous global-minimum determination, not a heuristic. In the
/// half-plane spanned by the axis and the query's radial direction (apex at
/// the origin, radial coordinate `ρ ≥ 0`), the squared distance to the
/// positive-nappe generator is `(h·sinα − ρ·cosα)²` and to the negative-nappe
/// generator is `(h·sinα + ρ·cosα)²`. For `h > 0` the first is strictly
/// smaller; when the negative nappe's perpendicular foot falls behind the
/// apex, its nearest point is the apex at squared distance `h² + ρ²`, which is
/// still ≥ the positive-nappe distance. Hence `sign(h)` selects the exact
/// global minimum. The per-nappe foot parameter uses `|h|`, which is the
/// correct (non-negative) foot on the same-sign nappe.
///
/// # Apex / axis cases
///
/// When `q` is exactly on the cone's axis (radial component = 0), the
/// helper returns [`GeometryError::Singular`] for the positive nappe, which
/// is propagated.  When `q = apex` exactly (h = 0 and radial = 0), the
/// result is also `Singular`.
#[allow(clippy::too_many_arguments)]
pub(super) fn exact_cone_project(
    budget: CertificationBudget,
    query: [f64; 3],
    apex: [f64; 3],
    axis: [f64; 3],
    half_angle: f64,
    x_axis: [f64; 3],
    y_axis: [f64; 3],
) -> Result<ExactConeProjectionPair, GeometryError> {
    let q = to_rat3(query);
    let ap = to_rat3(apex);
    let ax = to_rat3(axis);
    let diff = [&q[0] - &ap[0], &q[1] - &ap[1], &q[2] - &ap[2]];
    let dot_diff_ax = &diff[0] * &ax[0] + &diff[1] * &ax[1] + &diff[2] * &ax[2];
    let ax_sq = &ax[0] * &ax[0] + &ax[1] * &ax[1] + &ax[2] * &ax[2];
    if ax_sq.is_zero() {
        return Err(GeometryError::Uncertified {
            reason: "cone axis has exact zero magnitude".to_owned(),
        });
    }
    let h = &dot_diff_ax / &ax_sq;
    check_rational_budget(budget, &h)?;

    // Query exactly at the apex (`diff = 0`): the nearest cone point is the
    // apex itself. Every azimuth is equally valid, so report the canonical
    // parameterization `(u = 0, v = 0)` with a zero distance/residual rather
    // than `Singular` — the apex is a genuine, unique nearest point, unlike
    // an on-axis non-apex query (which has no unique azimuth).
    if diff[0].is_zero() && diff[1].is_zero() && diff[2].is_zero() {
        return Ok(ExactConeProjectionPair {
            primary: ExactConeProjection {
                u: 0.0,
                v: 0.0,
                u_error_bound: 0.0,
                v_error_bound: 0.0,
                point: apex,
                point_residual_bound: 0.0,
                distance_bound: 0.0,
            },
            secondary: None,
        });
    }

    // Compute positive nappe projection first (always needed).
    let pos =
        exact_cone_project_nappe(budget, query, apex, axis, half_angle, x_axis, y_axis, true)?;

    if h.is_zero() {
        // h = 0 exactly: both nappes equidistant.  Compute negative nappe
        // and return both in deterministic order (positive first).
        let neg =
            exact_cone_project_nappe(budget, query, apex, axis, half_angle, x_axis, y_axis, false)?;
        Ok(ExactConeProjectionPair {
            primary: pos,
            secondary: Some(neg),
        })
    } else if h.is_positive() {
        // h > 0: positive nappe is the exact global minimum (see the proof in
        // this function's doc comment). No approximate comparison is involved.
        Ok(ExactConeProjectionPair {
            primary: pos,
            secondary: None,
        })
    } else {
        // h < 0: negative nappe is strictly closer.
        let neg =
            exact_cone_project_nappe(budget, query, apex, axis, half_angle, x_axis, y_axis, false)?;
        Ok(ExactConeProjectionPair {
            primary: neg,
            secondary: None,
        })
    }
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

/// Checks a certified linear parameter error bound against
/// [`ToleranceContext::parametric`], returning [`GeometryError::Uncertified`]
/// when the bound exceeds the caller's parametric tolerance.
pub(super) fn check_parametric_tolerance(
    tolerance: &ToleranceContext,
    bound: f64,
) -> Result<(), GeometryError> {
    if bound > tolerance.parametric() {
        return Err(GeometryError::Uncertified {
            reason: "certified linear parameter error bound exceeds parametric tolerance"
                .to_owned(),
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

/// Certified componentwise enclosure of the ideal unit vector `v / ‖v‖`.
///
/// Each returned interval encloses the corresponding component of the exact
/// normalization of `v`, using the certified magnitude bracket
/// `[sqrt_down(‖v‖²), sqrt_up(‖v‖²)]` and the sign-aware [`ratio_by_bracket`].
/// Returns `None` when `v` has exact zero magnitude or the magnitude
/// underflows the smallest positive `f64` (a degenerate axis the caller must
/// treat as uncertifiable).
fn ideal_unit_components(v: &[BigRational]) -> Option<Vec<RatInterval>> {
    let mut norm_sq = BigRational::zero();
    for c in v {
        norm_sq += c * c;
    }
    if !norm_sq.is_positive() {
        return None;
    }
    let mag_down = sqrt_down(&norm_sq).ok().filter(|&m| m > 0.0)?;
    let mag_up = sqrt_up(&norm_sq).ok()?;
    let mag_down_r = f64_to_rat(mag_down);
    let mag_up_r = f64_to_rat(mag_up);
    if mag_down_r.is_zero() {
        return None;
    }
    Some(
        v.iter()
            .map(|c| ratio_by_bracket(c, &mag_down_r, &mag_up_r))
            .collect(),
    )
}

/// Certified `f64` upper bound on `‖stored − ideal‖`, where each `ideal[i]`
/// is a certified enclosure of the i-th ideal-vector component.
///
/// The per-component deviation is the larger endpoint distance
/// `max(|stored − lo|, |stored − hi|)` (an exact rational); the vector bound
/// is the outward [`sqrt_up`] of the summed squares.
fn vector_deviation_bound(stored: &[BigRational], ideal: &[RatInterval]) -> f64 {
    let mut sq = BigRational::zero();
    for (s, comp) in stored.iter().zip(ideal.iter()) {
        let dlo = (s - &comp.lo).abs();
        let dhi = (s - &comp.hi).abs();
        let d = if dlo >= dhi { dlo } else { dhi };
        sq += &d * &d;
    }
    sqrt_up(&sq).unwrap_or(f64::INFINITY)
}

/// Certified upper bound on the model-space position error introduced by an
/// approximate stored 2-D frame `(x_stored, y_stored)` relative to the ideal
/// orthonormal frame `x_ideal = x_stored / ‖x_stored‖`,
/// `y_ideal = perp2(x_ideal)`.
///
/// For an evaluation `p = base + cx·x_stored + cy·y_stored` with
/// `|cx|, |cy| ≤ radial_scale`, the triangle inequality bounds the position
/// error by `radial_scale · (‖x_stored − x_ideal‖ + ‖y_stored − y_ideal‖)`.
/// The two per-axis deviations are certified exactly via
/// [`vector_deviation_bound`] from componentwise ideal enclosures — not a
/// magnitude-only `|‖axis‖ − 1|` proxy. Returns `+∞` for a degenerate x-axis
/// (forcing the caller into an `Uncertified` result).
pub(super) fn frame_deviation_bound_2d(
    x_stored: [f64; 2],
    y_stored: [f64; 2],
    radial_scale: f64,
) -> f64 {
    let xs = to_rat2(x_stored);
    let ys = to_rat2(y_stored);
    let Some(x_ideal) = ideal_unit_components(&xs) else {
        return f64::INFINITY;
    };
    // y_ideal = perp2(x_ideal) = (−x_ideal[1], x_ideal[0]).
    let y_ideal = [x_ideal[1].neg(), x_ideal[0].clone()];
    let dev_x = vector_deviation_bound(&xs, &x_ideal);
    let dev_y = vector_deviation_bound(&ys, &y_ideal);
    let total = radial_scale.abs() * (dev_x + dev_y);
    if total.is_finite() {
        total
    } else {
        f64::INFINITY
    }
}

/// Certified upper bound on the model-space position error introduced by an
/// approximate stored 3-D frame `(z_stored, x_stored, y_stored)` relative to
/// its ideal orthonormal Gram–Schmidt frame, defined by
///
/// ```text
/// z_ideal = z_stored / ||z_stored||
/// x_ideal = normalize(x_stored - z_ideal*(x_stored . z_ideal))
/// y_ideal = z_ideal x x_ideal
/// ```
///
/// For a point `base + cx*x_stored + cy*y_stored + cz*z_stored` with `|cx|` and
/// `|cy|` at most `radial_scale` and `|cz|` at most `axial_scale`, the triangle
/// inequality bounds the position error by
///
/// ```text
/// radial_scale*(||x_stored - x_ideal|| + ||y_stored - y_ideal||)
///   + axial_scale*||z_stored - z_ideal||
/// ```
///
/// Unlike a magnitude-only deviation, this captures the full Gram–Schmidt
/// orthogonalization error, so an x-axis that is near-unit but *not
/// orthogonal* to z (e.g. `z = (0,0,1)`, `x = (1,0,4eps)`) yields the correct
/// `4eps` deviation rather than the spurious `eps^2` of a norm-only bound. The
/// deviations are certified from exact componentwise ideal enclosures (the
/// Gram–Schmidt subtraction is exact; the two normalizations and the cross
/// product use outward-rounded intervals). Returns `+inf` for a degenerate z-
/// or (post-projection) x-axis.
pub(super) fn frame_deviation_bound_3d(
    z_stored: [f64; 3],
    x_stored: [f64; 3],
    y_stored: [f64; 3],
    radial_scale: f64,
    axial_scale: f64,
) -> f64 {
    let zs = to_rat3(z_stored);
    let xs = to_rat3(x_stored);
    let ys = to_rat3(y_stored);

    let Some(z_ideal) = ideal_unit_components(&zs) else {
        return f64::INFINITY;
    };

    // Gram–Schmidt: remove the exact z-parallel part of x, then normalize.
    let mut z_norm_sq = BigRational::zero();
    let mut dot_xz = BigRational::zero();
    for (zi, xi) in zs.iter().zip(xs.iter()) {
        z_norm_sq += zi * zi;
        dot_xz += xi * zi;
    }
    if !z_norm_sq.is_positive() {
        return f64::INFINITY;
    }
    let coeff = &dot_xz / &z_norm_sq;
    let x_perp = [
        &xs[0] - &coeff * &zs[0],
        &xs[1] - &coeff * &zs[1],
        &xs[2] - &coeff * &zs[2],
    ];
    let Some(x_ideal) = ideal_unit_components(&x_perp) else {
        return f64::INFINITY;
    };

    // y_ideal = z_ideal × x_ideal (interval cross product; already unit since
    // z_ideal ⟂ x_ideal, so no renormalization is required — the enclosure
    // simply widens slightly).
    let y_ideal = [
        z_ideal[1]
            .mul(&x_ideal[2])
            .sub(&z_ideal[2].mul(&x_ideal[1])),
        z_ideal[2]
            .mul(&x_ideal[0])
            .sub(&z_ideal[0].mul(&x_ideal[2])),
        z_ideal[0]
            .mul(&x_ideal[1])
            .sub(&z_ideal[1].mul(&x_ideal[0])),
    ];

    let dev_x = vector_deviation_bound(&xs, &x_ideal);
    let dev_y = vector_deviation_bound(&ys, &y_ideal);
    let dev_z = vector_deviation_bound(&zs, &z_ideal);

    let total = radial_scale.abs() * (dev_x + dev_y) + axial_scale.abs() * dev_z;
    if total.is_finite() {
        total
    } else {
        f64::INFINITY
    }
}

/// Computes a certified upper bound on `tan(half_angle)` using the certified
/// trig backend.  Returns `None` and the caller should return `Uncertified`
/// if the budget is exceeded or the cosine interval straddles zero
/// (impossible for `half_angle ∈ (0, π/2)` but guarded for safety).
pub(super) fn certified_tan_upper(half_angle: f64, budget: CertificationBudget) -> Option<f64> {
    use super::exact::{f64_to_rat, rat_to_f64_up};
    let ha_r = f64_to_rat(half_angle);
    let (sin_i, cos_i) = super::trig::sin_cos_interval(&ha_r, budget).ok()?;
    if !cos_i.lo.is_positive() {
        return None; // should not occur for half_angle ∈ (0, π/2)
    }
    Some(rat_to_f64_up(&(&sin_i.hi / &cos_i.lo)))
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
        exact_affine_eval3, exact_cone_project, exact_cylinder_project, exact_line_project2,
        exact_line_project3, exact_plane_project3, frame_deviation_bound_2d,
        frame_deviation_bound_3d, normalization_to_construction, rounding_error_bound,
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
        let (rounded, bound) = rounding_error_bound(&r);
        assert!(bound >= 0.0);
        assert!((rounded - 0.333_333_333_333_333_3).abs() < 1e-9);
    }

    /// Item 2 regression: an exactly orthonormal 2-D frame has (near) zero
    /// certified deviation, so it contributes no spurious position error.
    #[test]
    fn frame_deviation_bound_2d_orthonormal_is_zero() {
        let bound = frame_deviation_bound_2d([1.0, 0.0], [0.0, 1.0], 5.0);
        assert_eq!(bound, 0.0, "orthonormal 2-D frame must have zero deviation");
    }

    /// Item 2 regression: a degenerate 2-D x-axis forces `+∞` (Uncertified).
    #[test]
    fn frame_deviation_bound_2d_degenerate_is_infinite() {
        let bound = frame_deviation_bound_2d([0.0, 0.0], [0.0, 1.0], 1.0);
        assert!(bound.is_infinite());
    }

    /// Item 2 regression: an exactly orthonormal 3-D frame has zero deviation.
    #[test]
    fn frame_deviation_bound_3d_orthonormal_is_zero() {
        let bound =
            frame_deviation_bound_3d([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], 2.0, 3.0);
        assert_eq!(bound, 0.0, "orthonormal 3-D frame must have zero deviation");
    }

    /// Item 2 counterexample regression (reviewer blocker 2): a near-unit but
    /// **non-orthogonal** stored x-axis `x = (1, 0, 4ε)` against `z = (0,0,1)`
    /// must yield the true `≈ 4ε` frame deviation, not the spurious `≈ ε²` of
    /// a magnitude-only bound. This is what makes the `z≈(0,0,1), x=(1,0,4ε)`
    /// cylinder projection certify honestly instead of emitting a false ε²
    /// certificate.
    #[test]
    fn frame_deviation_bound_3d_captures_non_orthogonal_x_axis() {
        let eps = f64::EPSILON;
        let bound = frame_deviation_bound_3d(
            [0.0, 0.0, 1.0],
            [1.0, 0.0, 4.0 * eps],
            [0.0, 1.0, 0.0],
            1.0,
            0.0,
        );
        // True deviation is 4ε ≈ 8.88e-16; certified value is that up to a
        // 1-ulp outward rounding. It must be far above the ε² ≈ 4.9e-32 that a
        // norm-only bound would (wrongly) report.
        assert!(bound >= 3.5 * eps, "expected ≳ 4ε deviation, got {bound:e}");
        assert!(bound <= 1e-14, "deviation far too large: {bound:e}");
        assert!(
            bound > 1e-20,
            "bound collapsed to the spurious ε² magnitude-only value: {bound:e}"
        );
    }

    /// Item 2 regression: a degenerate 3-D z-axis forces `+∞` (Uncertified).
    #[test]
    fn frame_deviation_bound_3d_degenerate_is_infinite() {
        let bound =
            frame_deviation_bound_3d([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], 1.0, 1.0);
        assert!(bound.is_infinite());
    }

    /// Item 4 regression (reviewer blocker 2): a stored cylinder x-axis with a
    /// `4ε` axial leak makes the returned point carry a `≈4ε` out-of-plane
    /// z-component. The certified distance bound must be the full `‖q − P′‖`
    /// (`≈4ε`), not the radial-only `≈ε²` that produced a false certificate.
    #[test]
    fn exact_cylinder_project_full_distance_captures_out_of_plane() {
        let eps = f64::EPSILON;
        let budget = CertificationBudget::default();
        let result = exact_cylinder_project(
            budget,
            [1.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 1.0],
            1.0,
            [1.0, 0.0, 4.0 * eps],
            [0.0, 1.0, 0.0],
        )
        .unwrap();
        assert!(
            result.distance_bound >= 3.5 * eps,
            "distance bound collapsed below the true ≈4ε out-of-plane distance: {:e}",
            result.distance_bound
        );
        assert!(
            result.distance_bound <= 1e-14,
            "distance bound implausibly large: {:e}",
            result.distance_bound
        );
        assert!(
            result.distance_bound > 1e-20,
            "distance bound is the spurious radial-only ε² value: {:e}",
            result.distance_bound
        );
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
            [0.0, 1.0, 0.0],
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
