//! Private arithmetic helpers operating on raw `f64` arrays.
//!
//! All operations are infallible on the hot path. Callers must validate
//! finiteness at public boundaries and, when wrapping results in foundation
//! types, call [`crate::GeometryError`]-returning helpers from the parent
//! module.

use crate::ParameterRange;

use super::ConstructionError;

pub(super) const UNIT_VECTOR_TOL: f64 = 8.0 * f64::EPSILON;
pub(super) const DISTANCE_BOUND_WIDENING: f64 = 8.0 * f64::EPSILON;
pub(super) const ILL_COND_THRESH: f64 = 2.384_185_791_015_625e-7;

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

/// Maps an angle (in radians) from `[-π, π]` or any value to `[0, 2π)`.
pub(super) fn angle_to_full_turn(angle: f64) -> f64 {
    use std::f64::consts::TAU;
    angle.rem_euclid(TAU) + 0.0
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

pub(super) fn widened_distance_bound2(query: [f64; 2], projection: [f64; 2]) -> f64 {
    let residual = mag2(sub2(query, projection));
    let scale_q = mag2(query);
    let scale_proj = mag2(projection);
    residual * (1.0 + DISTANCE_BOUND_WIDENING) + DISTANCE_BOUND_WIDENING * (scale_q + scale_proj)
}

pub(super) fn widened_distance_bound3(query: [f64; 3], projection: [f64; 3]) -> f64 {
    let residual = mag3(sub3(query, projection));
    let scale_q = mag3(query);
    let scale_proj = mag3(projection);
    residual * (1.0 + DISTANCE_BOUND_WIDENING) + DISTANCE_BOUND_WIDENING * (scale_q + scale_proj)
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
    fn angle_to_full_turn_maps_tiny_negative_angles_to_last_turn_slice() {
        let angle = -1e-300_f64;
        let mapped = angle_to_full_turn(angle);
        assert!(mapped <= TAU);
        assert!((mapped - (TAU - 1e-300)).abs() < 1e-15 * TAU);
    }

    #[test]
    fn angle_to_full_turn_handles_seam_values() {
        let eps = 1e-12_f64;
        assert_eq!(angle_to_full_turn(PI), PI);
        assert!(angle_to_full_turn(TAU - eps) < TAU);
        assert!(angle_to_full_turn(TAU + eps) < TAU);
        assert_eq!(angle_to_full_turn(-0.0).to_bits(), 0.0f64.to_bits());
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
