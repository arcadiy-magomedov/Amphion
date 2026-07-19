//! Checked primitive transformation via `Transform3`.
//!
//! All transform operations here are **provisional** pending the `cf85555`
//! foundation integration, which adds `try_apply_to_point`,
//! `try_apply_to_vector`, and similarity classification directly to
//! `amphion-foundation`. Until then, this module manually applies the raw
//! row-major 3×4 matrix returned by `Transform3::into_row_major` and
//! re-derives the checks that a future foundation API would provide
//! natively (finiteness of the transformed result, and — for primitives
//! whose shape is only preserved under a similarity — a heuristic
//! orthogonality/equal-scale/no-reflection check on the transform's linear
//! part).
//!
//! `Line3` and `Plane` accept any non-degenerate affine transform: an affine
//! image of a line is a line, and an affine image of a plane (as long as it
//! does not collapse the spanning vectors) is a plane. `Circle3`, `Cylinder`,
//! and `Cone` require a *similarity* transform (rigid motion plus uniform
//! scale, no reflection), because only a similarity preserves circles,
//! circular cylinders, and circular cones as such.
//!
//! 2-D primitives (`Line2`, `Circle2`) are intentionally **not** covered:
//! the current foundation has no `Transform2` type. Support should be added
//! once one exists.

#![allow(clippy::many_single_char_names, clippy::similar_names)]

use super::helpers::{dot3, mag3};

/// A failure from applying a `Transform3` to an analytic primitive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum TransformError {
    /// Applying the transform produced a NaN or infinite coordinate.
    NonFiniteResult,
    /// The transform's linear part is (numerically) singular.
    SingularTransform,
    /// The primitive requires a similarity transform (uniform scale, no
    /// shear, no reflection), and the supplied transform is not one.
    NotSimilarity,
    /// The transformed primitive is geometrically degenerate (e.g. its
    /// spanning vectors became dependent).
    DegenerateResult,
}

/// Relative tolerance used to classify a transform's linear part as a
/// similarity (equal per-axis scale, mutual orthogonality, positive
/// determinant). This is a heuristic threshold, not a certified bound; see
/// the module-level documentation.
const SIMILARITY_TOL: f64 = 1e-10;

/// Applies a `Transform3` (3×4 row-major) to a 3-D point.
///
/// Returns `None` if any resulting coordinate is non-finite.
pub(super) fn apply_to_point(m: [f64; 12], p: [f64; 3]) -> Option<[f64; 3]> {
    let x = m[0] * p[0] + m[1] * p[1] + m[2] * p[2] + m[3];
    let y = m[4] * p[0] + m[5] * p[1] + m[6] * p[2] + m[7];
    let z = m[8] * p[0] + m[9] * p[1] + m[10] * p[2] + m[11];
    if x.is_finite() && y.is_finite() && z.is_finite() {
        Some([x, y, z])
    } else {
        None
    }
}

/// Applies a `Transform3` to a 3-D direction vector (linear part only, no
/// translation).
///
/// Returns `None` if any resulting coordinate is non-finite.
pub(super) fn apply_to_vector(m: [f64; 12], v: [f64; 3]) -> Option<[f64; 3]> {
    let x = m[0] * v[0] + m[1] * v[1] + m[2] * v[2];
    let y = m[4] * v[0] + m[5] * v[1] + m[6] * v[2];
    let z = m[8] * v[0] + m[9] * v[1] + m[10] * v[2];
    if x.is_finite() && y.is_finite() && z.is_finite() {
        Some([x, y, z])
    } else {
        None
    }
}

/// Computes the uniform scale factor of a transform's linear part and
/// verifies it is a similarity: equal-magnitude, mutually orthogonal
/// columns, non-zero scale, and a positive determinant (no reflection).
///
/// Returns `Some(scale)` when the linear part is a similarity, `None`
/// otherwise.
pub(super) fn similarity_scale(m: [f64; 12]) -> Option<f64> {
    let tol = SIMILARITY_TOL;
    let c0 = [m[0], m[4], m[8]];
    let c1 = [m[1], m[5], m[9]];
    let c2 = [m[2], m[6], m[10]];
    let s0 = mag3(c0);
    let s1 = mag3(c1);
    let s2 = mag3(c2);
    if s0 < 1e-300 {
        return None;
    }
    // Equal scale across all three columns.
    if (s0 - s1).abs() > tol * s0 || (s0 - s2).abs() > tol * s0 {
        return None;
    }
    // Mutual orthogonality.
    if dot3(c0, c1).abs() > tol * s0 * s1 {
        return None;
    }
    if dot3(c0, c2).abs() > tol * s0 * s2 {
        return None;
    }
    if dot3(c1, c2).abs() > tol * s1 * s2 {
        return None;
    }
    // Positive determinant: reject reflections.
    let det = m[0] * (m[5] * m[10] - m[9] * m[6]) - m[1] * (m[4] * m[10] - m[8] * m[6])
        + m[2] * (m[4] * m[9] - m[8] * m[5]);
    if det < 0.0 {
        return None;
    }
    Some(s0)
}
