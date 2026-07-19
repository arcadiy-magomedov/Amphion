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

use amphion_foundation::Transform3;
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed};

use super::exact::{f64_to_rat, sqrt_up};

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

/// Computes the uniform scale factor of a transform's linear part and
/// verifies it is a similarity: equal-magnitude, mutually orthogonal
/// columns, non-zero scale, and a positive determinant (no reflection).
///
/// Returns `Some(scale)` when the linear part is a similarity, `None`
/// otherwise.
pub(super) fn similarity_scale(transform: &Transform3) -> Option<f64> {
    let m = transform.into_row_major();
    let lin = [m[0], m[1], m[2], m[4], m[5], m[6], m[8], m[9], m[10]];
    if !lin.iter().all(|value| value.is_finite()) {
        return None;
    }

    let col = |j: usize| -> [BigRational; 3] {
        [f64_to_rat(m[j]), f64_to_rat(m[4 + j]), f64_to_rat(m[8 + j])]
    };
    let c0 = col(0);
    let c1 = col(1);
    let c2 = col(2);

    let dot = |a: &[BigRational; 3], b: &[BigRational; 3]| -> BigRational {
        &a[0] * &b[0] + &a[1] * &b[1] + &a[2] * &b[2]
    };

    let n0 = dot(&c0, &c0);
    let n1 = dot(&c1, &c1);
    let n2 = dot(&c2, &c2);
    if !n0.is_positive() || !n1.is_positive() || !n2.is_positive() {
        return None;
    }

    let eps4 = BigRational::new(BigInt::from(4i64), BigInt::one() << 52usize);
    let tol_n = &eps4 * &n0;
    if (&n0 - &n1).abs() > tol_n || (&n0 - &n2).abs() > tol_n {
        return None;
    }

    let d01 = dot(&c0, &c1);
    let d02 = dot(&c0, &c2);
    let d12 = dot(&c1, &c2);
    if d01.abs() > tol_n || d02.abs() > tol_n || d12.abs() > tol_n {
        return None;
    }

    let det = &c0[0] * (&c1[1] * &c2[2] - &c1[2] * &c2[1])
        - &c0[1] * (&c1[0] * &c2[2] - &c1[2] * &c2[0])
        + &c0[2] * (&c1[0] * &c2[1] - &c1[1] * &c2[0]);
    if !det.is_positive() {
        return None;
    }

    let scale = sqrt_up(&n0).ok()?;
    if !scale.is_finite() || scale <= 0.0 {
        return None;
    }
    Some(scale)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use amphion_foundation::Transform3;

    use super::similarity_scale;

    #[test]
    fn exact_similarity_accepts_identity() {
        assert_eq!(similarity_scale(&Transform3::IDENTITY), Some(1.0));
    }

    #[test]
    fn exact_similarity_accepts_large_scale_rotation() {
        let scale = 1.0e200;
        let transform = Transform3::try_from_row_major([
            0.0, -scale, 0.0, 0.0, //
            scale, 0.0, 0.0, 0.0, //
            0.0, 0.0, scale, 0.0,
        ])
        .unwrap();
        assert_eq!(similarity_scale(&transform), Some(scale));
    }

    #[test]
    fn exact_similarity_rejects_shear_at_unit_scale() {
        let transform = Transform3::try_from_row_major([
            1.0, 1.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0,
        ])
        .unwrap();
        assert_eq!(similarity_scale(&transform), None);
    }

    #[test]
    fn exact_similarity_rejects_reflection() {
        let transform = Transform3::try_from_row_major([
            -1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0,
        ])
        .unwrap();
        assert_eq!(similarity_scale(&transform), None);
    }

    #[test]
    fn exact_similarity_rejects_singular() {
        let transform = Transform3::try_from_row_major([
            0.0, 0.0, 0.0, 0.0, //
            0.0, 0.0, 0.0, 0.0, //
            0.0, 0.0, 0.0, 0.0,
        ])
        .unwrap();
        assert_eq!(similarity_scale(&transform), None);
    }
}
