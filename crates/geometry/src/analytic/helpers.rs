//! Private arithmetic helpers operating on raw `f64` arrays.
//!
//! All operations are infallible on the hot path. Callers must validate
//! finiteness at public boundaries and, when wrapping results in foundation
//! types, call [`crate::GeometryError`]-returning helpers from the parent
//! module.
//!
//! # Certified distance bounds
//!
//! `certified_distance_bound2` and `certified_distance_bound3` derive a
//! certified upper bound on the true geometric distance from a query point
//! to a projected surface/curve point, accounting for accumulated
//! floating-point rounding.
//!
//! ## Error analysis (Higham 2002)
//!
//! We use the standard model (Higham, *Accuracy and Stability of Numerical
//! Algorithms*, 2nd ed., SIAM 2002, §2.2): for any elementary floating-point
//! operation `fl(a ⊕ b) = (a ⊕ b)(1 + δ)` with `|δ| ≤ u = ε/2`.
//!
//! For an inner product of two 3-vectors, Higham Theorem 3.5 gives
//! `|fl(aᵀb) − aᵀb| ≤ γ₃|a||b|` where `γₙ = nε/(1−nε) ≤ 2nε`.
//!
//! A projection computation involves `≤ 64` elementary operations (dot
//! products, additions, trig calls count as a single correctly-rounded op by
//! IEEE 754). Using `n = 64` and `γ₆₄ ≤ 128ε`:
//!
//! ```text
//! |bound_computed − actual_distance| ≤ 128ε · s_local
//! ```
//!
//! where `s_local = |query − anchor|` is the **translation-invariant** local
//! displacement from the primitive's anchor (origin/center/apex), avoiding
//! world-origin inflation that would arise from using `|query| + |proj|`.
//!
//! **Rejection criterion**: if `128ε · s_local > effective_length(s_local)`,
//! the numerical error exceeds the caller's tolerance, and we return
//! `GeometryError::Uncertified`.  For typical tolerances (absolute 1e-9,
//! relative 1e-8) this triggers only when `s_local ≫ 1/(128ε·relative) ≈ 4e13`,
//! i.e., astronomical coordinate scales.
//!
//! ## Foundation API gap
//!
//! Interval arithmetic primitives (`UnitVector2/3`, checked norms, tight
//! interval residuals) would tighten these bounds and remove the constant-64
//! conservatism.  Until `amphion-foundation` exposes such types, this module
//! duplicates their logic locally.

use amphion_foundation::ToleranceContext;

use crate::{GeometryError, ParameterRange};

use super::ConstructionError;

/// Serde validation tolerance: accept vectors whose squared-norm differs from
/// 1 by at most `8ε`.  This passes honest round-tripped values (which have
/// `||v||² − 1| ≤ 2ε`) while rejecting clearly non-unit vectors.
pub(super) const UNIT_VECTOR_TOL: f64 = 8.0 * f64::EPSILON;

/// Ill-conditioning threshold for Gram-Schmidt orthogonalization.  If the
/// component of the supplied x-axis perpendicular to the main axis has
/// magnitude below this value (`16 · √ε ≈ 2.4e-7`), the normalization would
/// amplify rounding errors by a factor of `> 1/√ε ≈ 6.7e7` and the result
/// would be unreliable.
pub(super) const ILL_COND_THRESH: f64 = 2.384_185_791_015_625e-7;

/// Conservative FP-operation count used in `certified_distance_bound*`.
/// Covers all projection types at `n ≤ 64` elementary operations.
/// See module-level documentation for the derivation.
const C_FP: f64 = 128.0;

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

// ─── Angular helpers ─────────────────────────────────────────────────────────

/// Maps an angle (in radians) to the canonical domain `[0, 2π)`.
///
/// Uses `rem_euclid` for correct signed-remainder behaviour across the full
/// range of finite `f64` inputs.  The subsequent clamp `if r >= TAU { 0.0 }`
/// handles the edge case where `rem_euclid` returns exactly `TAU` due to
/// IEEE 754 rounding (e.g. `(-1e-300).rem_euclid(TAU) == TAU` on x86-64).
/// The result is always in `[0, 2π)` for every finite input.
pub(super) fn angle_to_full_turn(angle: f64) -> f64 {
    use std::f64::consts::TAU;
    let r = angle.rem_euclid(TAU);
    // Clamp the floating-point artefact where rem_euclid rounds up to TAU.
    // Add 0.0 to canonicalize -0.0 to +0.0 (IEEE 754: -0.0 + 0.0 = +0.0).
    if r >= TAU { 0.0 } else { r + 0.0 }
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

/// Returns a certified upper bound on `|query − projection|` for a 2-D
/// projection, and checks that the floating-point error certificate fits
/// within `tolerance`.
///
/// `anchor` is a translation-invariant reference point of the primitive
/// (e.g. circle center).  The characteristic scale `s_local = |query − anchor|`
/// avoids world-origin inflation.
///
/// # Errors
///
/// Returns [`GeometryError::Uncertified`] if the FP error bound exceeds
/// `tolerance.effective_length(s_local)`.
pub(super) fn certified_distance_bound2(
    anchor: [f64; 2],
    query: [f64; 2],
    projection: [f64; 2],
    tolerance: &ToleranceContext,
) -> Result<f64, GeometryError> {
    let s_local = mag2(sub2(query, anchor));
    let residual = mag2(sub2(query, projection));
    let fp_err = C_FP * f64::EPSILON * s_local;
    let bound = residual + fp_err;
    let eff_tol = tolerance
        .effective_length(s_local)
        .map_err(|_| GeometryError::Uncertified {
            reason: "invalid local scale for certification".to_owned(),
        })?;
    if fp_err > eff_tol {
        return Err(GeometryError::Uncertified {
            reason: "floating-point error bound exceeds requested tolerance at this scale"
                .to_owned(),
        });
    }
    Ok(bound)
}

/// Returns a certified upper bound on `|query − projection|` for a 3-D
/// projection.  See [`certified_distance_bound2`] for the derivation.
pub(super) fn certified_distance_bound3(
    anchor: [f64; 3],
    query: [f64; 3],
    projection: [f64; 3],
    tolerance: &ToleranceContext,
) -> Result<f64, GeometryError> {
    let s_local = mag3(sub3(query, anchor));
    let residual = mag3(sub3(query, projection));
    let fp_err = C_FP * f64::EPSILON * s_local;
    let bound = residual + fp_err;
    let eff_tol = tolerance
        .effective_length(s_local)
        .map_err(|_| GeometryError::Uncertified {
            reason: "invalid local scale for certification".to_owned(),
        })?;
    if fp_err > eff_tol {
        return Err(GeometryError::Uncertified {
            reason: "floating-point error bound exceeds requested tolerance at this scale"
                .to_owned(),
        });
    }
    Ok(bound)
}

/// Certifies the sign of the axial component `h` for cone projection.
///
/// Returns `Some(Ordering)` when the sign can be certified from the FP error
/// bound, or `None` when `|h|` is too small relative to `d_mag` to determine
/// whether the query is above, below, or in the equatorial plane.
///
/// The error bound `C_H · ε · d_mag` is derived from Higham Theorem 3.5:
/// a 3-D dot product after a 3-component vector subtraction accumulates at
/// most 8ε relative error on `d_mag`.
pub(super) fn certify_h_sign(h: f64, d_mag: f64) -> Option<core::cmp::Ordering> {
    const C_H: f64 = 8.0;
    let err_bound = C_H * f64::EPSILON * d_mag;
    if h == 0.0 {
        Some(core::cmp::Ordering::Equal)
    } else if h > err_bound {
        Some(core::cmp::Ordering::Greater)
    } else if h < -err_bound {
        Some(core::cmp::Ordering::Less)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use std::f64::consts::{PI, TAU};

    use super::{angle_to_full_turn, mag2, mag3, normalize2, normalize3};

    #[test]
    fn angle_to_full_turn_preserves_tiny_positive_angles() {
        let angle = 1e-300_f64;
        let mapped = angle_to_full_turn(angle);
        assert!(mapped > 0.0);
        assert!((mapped - angle).abs() < 1e-320);
    }

    #[test]
    fn angle_to_full_turn_maps_tiny_negative_angles_below_tau() {
        // (-1e-300).rem_euclid(TAU) == TAU on x86-64 due to IEEE 754 rounding.
        // angle_to_full_turn must clamp that to 0.0, not TAU.
        let mapped = angle_to_full_turn(-1e-300_f64);
        assert!(mapped < TAU, "result must be < TAU, got {mapped:.20e}");
        // Periodic equivalent: 0.0 is acceptable (same congruence class as -1e-300 mod 2π)
        assert_eq!(mapped.to_bits(), 0.0_f64.to_bits());
    }

    #[test]
    fn angle_to_full_turn_handles_seam_values() {
        let eps = 1e-12_f64;
        // PI is exactly representable, rem_euclid should be exact.
        assert_eq!(angle_to_full_turn(PI).to_bits(), PI.to_bits());
        assert!(angle_to_full_turn(TAU - eps) < TAU);
        assert!(angle_to_full_turn(TAU + eps) < TAU);
        assert_eq!(angle_to_full_turn(-0.0).to_bits(), 0.0f64.to_bits());
        // Tiny positive: must survive without collapsing to 0.
        assert!(angle_to_full_turn(1e-300_f64) > 0.0);
        assert!(angle_to_full_turn(1e-300_f64) < TAU);
    }

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
}
