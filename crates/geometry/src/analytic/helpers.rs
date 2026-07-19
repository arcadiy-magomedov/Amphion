//! Private arithmetic helpers operating on raw `f64` arrays.
//!
//! All operations are infallible on the hot path. Callers must validate
//! finiteness at public boundaries and, when wrapping results in foundation
//! types, call [`crate::GeometryError`]-returning helpers from the parent
//! module.
//!
//! # Certified bounds are arithmetic-only
//!
//! [`arithmetic_proj_bound2`], [`arithmetic_proj_bound3`], and
//! [`arithmetic_eval_bound`] derive a certified upper bound on floating-point
//! error for computations that use **only** IEEE 754 basic operations
//! (addition, subtraction, multiplication, division, and `sqrt`), all of
//! which are correctly rounded per IEEE 754-2008/2019 §5.4. Under Higham
//! (2002), *Accuracy and Stability of Numerical Algorithms*, 2nd ed., SIAM,
//! Theorem 3.5 and §2.2: for a computation composed of `n` such operations,
//! `|fl(x) − x| ≤ γ_n · |x|` where `γ_n = n·ε / (1 − n·ε)`. Line and plane
//! evaluation/projection involve at most `~16` elementary operations
//! (dot products, additions, subtractions, one `sqrt` for magnitude); we use
//! `γ_32 ≤ 64·ε` as a conservative round-number bound covering both the 2-D
//! and 3-D cases with margin.
//!
//! **This bound is only valid for arithmetic-only computations.** No
//! certified, WASM-compatible, formally-proved correctly-rounded `sin`,
//! `cos`, or `atan2` implementation currently exists in the Rust ecosystem:
//!
//! - `libm` (MIT, pure Rust, WASM-compatible): empirically ~1–2 ULP, but
//!   **not formally proved** correctly rounded.
//! - `core-math` (MIT, 0.5 ULP correctly rounded): requires `fenv.h` C FFI
//!   for directed-rounding control, **not WASM-compatible**.
//! - `inari` / `rug` (interval arithmetic via MPFR): require GMP/MPFR C
//!   libraries, **not WASM-compatible**.
//! - `inari_wasm`: calls `f64::sin` directly without directed rounding, so
//!   it is **not rigorous** as an interval implementation.
//! - IEEE 754-2019 §9.2 only *recommends* (does not *require*)
//!   correctly-rounded `sin`/`cos`/`atan2`, so no portable error bound on
//!   these functions can be assumed by a certified kernel.
//!
//! Consequently, every evaluator whose formula involves `sin`, `cos`, or
//! `atan2` (`Circle2`, `Circle3`, `Cylinder`, `Cone` — both `evaluate()` and
//! `project_into()`) returns [`GeometryError::Uncertified`] instead of a
//! numeric bound, until a formally-proved WASM-compatible trigonometric
//! implementation is integrated.
//!
//! **Rejection criterion**: for arithmetic-only bounds, if the certified
//! floating-point error allowance exceeds `tolerance.effective_length(scale)`
//! at the relevant world-coordinate scale, we return
//! `GeometryError::Uncertified` rather than assert a bound we cannot
//! support.

use amphion_foundation::ToleranceContext;

use crate::{DistanceBound, GeometryError, ParameterRange};

use super::ConstructionError;

/// Serde validation tolerance: accept vectors whose squared-norm differs from
/// 1 by at most `4ε`, matching the magnitude-deviation guarantee of the
/// `UnitVector2`/`UnitVector3` types introduced in foundation commit
/// `cf85555`. Honest round-tripped values produced by this module's own
/// normalization have `||v||² − 1| ≤ 2ε`, so they pass with a `2×` margin;
/// clearly corrupted or hand-crafted non-unit vectors are rejected.
pub(super) const UNIT_VECTOR_TOL: f64 = 4.0 * f64::EPSILON;

/// Ill-conditioning threshold for Gram-Schmidt orthogonalization.  If the
/// component of the supplied x-axis perpendicular to the main axis has
/// magnitude below this value (`16 · √ε ≈ 2.4e-7`), the normalization would
/// amplify rounding errors by a factor of `> 1/√ε ≈ 6.7e7` and the result
/// would be unreliable.
pub(super) const ILL_COND_THRESH: f64 = 2.384_185_791_015_625e-7;

/// Certified Higham `γ_32` bound (see module-level documentation) for
/// arithmetic-only (no-trig) computations: `γ_32 = 32ε/(1−32ε) ≤ 64ε`.
///
/// Valid ONLY for computations built exclusively from IEEE 754 basic
/// operations (add, sub, mul, div, sqrt). Never apply to a computation that
/// calls `sin`, `cos`, or `atan2`.
const GAMMA_32: f64 = 64.0 * f64::EPSILON;

/// Higham `γ_64` bound used for evaluation error, which additionally accounts
/// for the ≤ `UNIT_VECTOR_TOL` per-component deviation of a stored direction
/// vector from a true unit vector. Still arithmetic-only; still ≤ a small
/// constant multiple of ε.
const GAMMA_64: f64 = 128.0 * f64::EPSILON;

// ─── 3-D helpers ────────────────────────────────────────────────────────────

pub(super) fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(super) fn cross3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

pub(super) fn mag3_sq(v: [f64; 3]) -> f64 {
    dot3(v, v)
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

/// Returns `None` only when `v` has exactly zero squared-magnitude.
pub(super) fn normalize3(v: [f64; 3]) -> Option<[f64; 3]> {
    let scale = v[0].abs().max(v[1].abs()).max(v[2].abs());
    if scale == 0.0 || !scale.is_finite() {
        return None;
    }
    let vs = [v[0] / scale, v[1] / scale, v[2] / scale];
    let inv = (vs[0] * vs[0] + vs[1] * vs[1] + vs[2] * vs[2])
        .sqrt()
        .recip();
    Some([vs[0] * inv, vs[1] * inv, vs[2] * inv])
}

pub(super) fn add3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
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

pub(super) fn dot2(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

pub(super) fn mag2_sq(v: [f64; 2]) -> f64 {
    dot2(v, v)
}

pub(super) fn mag2(v: [f64; 2]) -> f64 {
    let scale = v[0].abs().max(v[1].abs());
    if scale == 0.0 {
        0.0
    } else {
        let vs = [v[0] / scale, v[1] / scale];
        scale * (vs[0] * vs[0] + vs[1] * vs[1]).sqrt()
    }
}

/// Returns `None` only when `v` has exactly zero squared-magnitude.
pub(super) fn normalize2(v: [f64; 2]) -> Option<[f64; 2]> {
    let scale = v[0].abs().max(v[1].abs());
    if scale == 0.0 || !scale.is_finite() {
        return None;
    }
    let vs = [v[0] / scale, v[1] / scale];
    let inv = (vs[0] * vs[0] + vs[1] * vs[1]).sqrt().recip();
    Some([vs[0] * inv, vs[1] * inv])
}

pub(super) fn add2(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] + b[0], a[1] + b[1]]
}

pub(super) fn sub2(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
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

pub(super) fn validate_unit2(v: [f64; 2]) -> Result<(), ConstructionError> {
    if !all_finite2(v) {
        return Err(ConstructionError::NonFiniteInput);
    }
    if (mag2_sq(v) - 1.0).abs() > UNIT_VECTOR_TOL {
        return Err(ConstructionError::DegenerateAxis);
    }
    Ok(())
}

pub(super) fn validate_unit3(v: [f64; 3]) -> Result<(), ConstructionError> {
    if !all_finite3(v) {
        return Err(ConstructionError::NonFiniteInput);
    }
    if (mag3_sq(v) - 1.0).abs() > UNIT_VECTOR_TOL {
        return Err(ConstructionError::DegenerateAxis);
    }
    Ok(())
}

pub(super) fn validate_orthogonal3(a: [f64; 3], b: [f64; 3]) -> Result<(), ConstructionError> {
    if !all_finite3(a) || !all_finite3(b) {
        return Err(ConstructionError::NonFiniteInput);
    }
    if dot3(a, b).abs() > UNIT_VECTOR_TOL {
        return Err(ConstructionError::DependentAxes);
    }
    Ok(())
}

/// Certified upper bound on `|query − projection|` for a LINE or PLANE
/// projection in 2-D (no trig involved), and checks that the certified
/// floating-point error allowance fits within `tolerance`.
///
/// # Derivation (Higham 2002, §2.2, Theorem 3.5)
///
/// Line/plane projection involves only IEEE 754 basic operations (add, sub,
/// mul, sqrt — all correctly rounded by IEEE 754-2008/2019 mandate). For `n`
/// elementary operations: `|fl(x) − x| ≤ γ_n · |x|` where
/// `γ_n = n·ε / (1 − n·ε)`. 2-D line/plane projection plus residual
/// magnitude involves `≤ 16` such operations, so `γ_16 ≤ 32ε`; we use the
/// conservative round number `γ_32 ≤ 64ε` ([`GAMMA_32`]) to cover both the
/// 2-D and 3-D cases with margin.
///
/// The characteristic scale `scale = |query| + |projection|` is a
/// world-coordinate magnitude (not translation-invariant): it bounds every
/// intermediate magnitude the projection arithmetic can plausibly produce,
/// so the resulting `fp_err` allowance is valid regardless of where `query`
/// sits relative to the primitive's anchor.
///
/// This function is **only** valid for arithmetic-only computations (lines,
/// planes). Functions involving `sin`/`cos`/`atan2` must return
/// [`GeometryError::Uncertified`] directly instead of calling this helper.
///
/// # Errors
///
/// Returns [`GeometryError::Uncertified`] if the certified FP error
/// allowance exceeds `tolerance.effective_length(scale)`, i.e. the
/// primitive's scale is too large relative to the requested tolerance for
/// `f64` arithmetic to certify a bound.
pub(super) fn arithmetic_proj_bound2(
    query: [f64; 2],
    projection: [f64; 2],
    tolerance: &ToleranceContext,
) -> Result<f64, GeometryError> {
    let residual = mag2(sub2(query, projection));
    let scale = mag2(query) + mag2(projection);
    let fp_err = GAMMA_32 * scale;
    let bound = residual + fp_err;
    if !bound.is_finite() {
        return Err(GeometryError::Uncertified {
            reason: "projection distance overflowed representable range".to_owned(),
        });
    }
    let eff_tol = tolerance
        .effective_length(scale)
        .map_err(|_| GeometryError::Uncertified {
            reason: "world scale is invalid for tolerance computation".to_owned(),
        })?;
    if fp_err > eff_tol {
        return Err(GeometryError::Uncertified {
            reason: "f64 coordinate granularity exceeds requested tolerance at this world \
                     scale; consider reducing the coordinate magnitude or relaxing the tolerance"
                .to_owned(),
        });
    }
    Ok(bound)
}

/// Certified upper bound on `|query − projection|` for a LINE or PLANE
/// projection in 3-D.  See [`arithmetic_proj_bound2`] for the derivation.
pub(super) fn arithmetic_proj_bound3(
    query: [f64; 3],
    projection: [f64; 3],
    tolerance: &ToleranceContext,
) -> Result<f64, GeometryError> {
    let residual = mag3(sub3(query, projection));
    let scale = mag3(query) + mag3(projection);
    let fp_err = GAMMA_32 * scale;
    let bound = residual + fp_err;
    if !bound.is_finite() {
        return Err(GeometryError::Uncertified {
            reason: "projection distance overflowed representable range".to_owned(),
        });
    }
    let eff_tol = tolerance
        .effective_length(scale)
        .map_err(|_| GeometryError::Uncertified {
            reason: "world scale is invalid for tolerance computation".to_owned(),
        })?;
    if fp_err > eff_tol {
        return Err(GeometryError::Uncertified {
            reason: "f64 coordinate granularity exceeds requested tolerance at this world scale"
                .to_owned(),
        });
    }
    Ok(bound)
}

/// Certified upper bound on `‖evaluated_position − true_p(t)‖` for
/// arithmetic-only evaluators (`Line2`/`Line3`, `Plane`).
///
/// Uses the [`GAMMA_64`] bound, which combines the `γ_32` arithmetic-error
/// bound with an allowance for the `UNIT_VECTOR_TOL` per-component deviation
/// of a stored direction vector from a true unit vector. `eval_scale` is the
/// world-coordinate magnitude of the evaluation result (e.g.
/// `|origin| + |t · direction|`).
///
/// # Errors
///
/// Returns [`GeometryError::Uncertified`] if `eval_scale` is not finite, so
/// the bound itself cannot be certified as finite and non-negative.
pub(super) fn arithmetic_eval_bound(eval_scale: f64) -> Result<DistanceBound, GeometryError> {
    if !eval_scale.is_finite() {
        return Err(GeometryError::Uncertified {
            reason: "evaluation scale is non-finite".to_owned(),
        });
    }
    let bound = (GAMMA_64 * eval_scale.abs()).max(0.0);
    DistanceBound::try_new(bound).map_err(|_| GeometryError::Uncertified {
        reason: "arithmetic evaluation error bound overflowed representable range".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::{
        UNIT_VECTOR_TOL, mag2, mag3, normalize2, normalize3, validate_unit2, validate_unit3,
    };
    use crate::analytic::ConstructionError;

    #[test]
    fn normalize3_handles_max_scale_inputs() {
        let unit = normalize3([f64::MAX, f64::MAX, f64::MAX]).unwrap();
        assert!((mag3(unit) - 1.0).abs() < 1e-15);
    }

    #[test]
    fn normalize3_handles_subnormal_inputs() {
        let unit = normalize3([f64::MIN_POSITIVE, f64::MIN_POSITIVE, 0.0]).unwrap();
        assert!((mag3(unit) - 1.0).abs() < 1e-15);
    }

    #[test]
    fn normalize2_handles_extreme_scales() {
        let unit_large = normalize2([f64::MAX, -f64::MAX]).unwrap();
        let unit_small = normalize2([f64::MIN_POSITIVE, f64::MIN_POSITIVE]).unwrap();
        assert!((mag2(unit_large) - 1.0).abs() < 1e-15);
        assert!((mag2(unit_small) - 1.0).abs() < 1e-15);
    }

    #[test]
    fn unit_vector_tol_rejects_vector_deviating_by_more_than_4eps() {
        // v = 1.0 + 4*eps is 4 ULPs away from 1.0 on the f64 grid near 1.0
        // (grid spacing there is exactly `eps`), so this is exact, not a
        // rounding artefact of the decimal literal.
        let eps = f64::EPSILON;
        let v = 1.0 + 4.0 * eps;
        let deviation = (v * v - 1.0).abs();
        assert!(
            deviation > UNIT_VECTOR_TOL,
            "test fixture must exceed the tightened 4ε tolerance: deviation={deviation:e}, \
             tol={UNIT_VECTOR_TOL:e}"
        );
        assert_eq!(
            validate_unit2([v, 0.0]),
            Err(ConstructionError::DegenerateAxis)
        );
        assert_eq!(
            validate_unit3([v, 0.0, 0.0]),
            Err(ConstructionError::DegenerateAxis)
        );
    }

    #[test]
    fn unit_vector_tol_accepts_normalize_round_trip_with_2x_margin() {
        // This module's own normalization guarantees `||v||² − 1| ≤ 2ε`, so
        // round-tripped unit vectors must pass the tightened 4ε tolerance
        // with a 2× margin.
        let v2 = normalize2([3.0, 4.0]).unwrap();
        assert!(validate_unit2(v2).is_ok());
        let v3 = normalize3([1.0, 2.0, 2.0]).unwrap();
        assert!(validate_unit3(v3).is_ok());
    }
}
