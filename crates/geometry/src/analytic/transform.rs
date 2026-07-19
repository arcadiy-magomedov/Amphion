//! Checked primitive transformation via `Transform3`.
//!
//! Uses `Transform3::try_apply_to_point`, `try_apply_to_vector`, and
//! `try_compose` from the accepted foundation (670516d).
//!
//! `Line3` and `Plane` accept any non-degenerate affine transform: an affine
//! image of a line is a line, and an affine image of a plane (as long as it
//! does not collapse the spanning vectors) is a plane. `Circle3`, `Cylinder`,
//! and `Cone` require a *similarity* transform (rigid motion plus uniform
//! scale, no reflection), since only a similarity preserves circles,
//! circular cylinders, and circular cones as such.
//!
//! 2-D primitives (`Line2`, `Circle2`) are not covered: the foundation has
//! no `Transform2` type. Support will be added when one exists.
//!
//! Reflection (`det < 0`) is supported as a similarity and preserves
//! geometric families; whether to reject it is a v0 policy choice.
//! Currently the implementation rejects it via positive-determinant check,
//! returning `TransformError::NotSimilarity`. This is a deliberate v0
//! limitation; a future release may add `TransformError::ReflectionUnsupported`.

#![allow(clippy::many_single_char_names, clippy::similar_names)]

use amphion_foundation::{Transform3, Vector3};

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

/// Computes the uniform scale factor of a transform's linear part and
/// verifies it is a similarity: equal-magnitude, mutually orthogonal
/// columns, non-zero scale, and a positive determinant (no reflection).
///
/// Returns `Some(scale)` when the linear part is a similarity, `None`
/// otherwise.
pub(super) fn similarity_scale(transform: &Transform3) -> Option<f64> {
    let m = transform.into_row_major();
    // Extract columns of the linear part as Vector3 (using checked constructor).
    let c0 = Vector3::try_new(m[0], m[4], m[8]).ok()?;
    let c1 = Vector3::try_new(m[1], m[5], m[9]).ok()?;
    let c2 = Vector3::try_new(m[2], m[6], m[10]).ok()?;
    let s0 = c0.try_magnitude().ok()?;
    let s1 = c1.try_magnitude().ok()?;
    let s2 = c2.try_magnitude().ok()?;
    if s0 < 1e-300 {
        return None;
    }
    let tol = SIMILARITY_TOL;
    // Equal scale across all three columns.
    if (s0 - s1).abs() > tol * s0 || (s0 - s2).abs() > tol * s0 {
        return None;
    }
    // Mutual orthogonality using checked dot products.
    if c0.try_dot(c1).ok()?.abs() > tol * s0 * s1 {
        return None;
    }
    if c0.try_dot(c2).ok()?.abs() > tol * s0 * s2 {
        return None;
    }
    if c1.try_dot(c2).ok()?.abs() > tol * s1 * s2 {
        return None;
    }
    // Positive determinant (no reflection).
    let det = m[0] * (m[5] * m[10] - m[9] * m[6]) - m[1] * (m[4] * m[10] - m[8] * m[6])
        + m[2] * (m[4] * m[9] - m[8] * m[5]);
    if det < 0.0 {
        return None;
    }
    Some(s0)
}
