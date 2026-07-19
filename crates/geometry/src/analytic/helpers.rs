//! Private arithmetic helpers operating on raw `f64` arrays.
//!
//! All operations are infallible on the hot path. Callers must validate
//! finiteness at public boundaries and, when wrapping results in foundation
//! types, call [`crate::GeometryError`]-returning helpers from the parent
//! module.

use crate::ParameterRange;

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
    mag3_sq(v).sqrt()
}

/// Returns `None` only when `v` has exactly zero squared-magnitude.
pub(super) fn normalize3(v: [f64; 3]) -> Option<[f64; 3]> {
    let msq = mag3_sq(v);
    if msq == 0.0 {
        None
    } else {
        Some(scale3(v, 1.0 / msq.sqrt()))
    }
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
    mag2_sq(v).sqrt()
}

/// Returns `None` only when `v` has exactly zero squared-magnitude.
pub(super) fn normalize2(v: [f64; 2]) -> Option<[f64; 2]> {
    let msq = mag2_sq(v);
    if msq == 0.0 {
        None
    } else {
        Some(scale2(v, 1.0 / msq.sqrt()))
    }
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
    ((angle % TAU) + TAU) % TAU
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
