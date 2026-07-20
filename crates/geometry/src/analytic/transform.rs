//! Checked primitive transformation via `Transform3`.
//!
//! Point and vector application is performed over exact dyadic rationals.
//! Every output coordinate must be exactly representable as `f64`; a
//! certificate-free transform never substitutes independently rounded affine
//! coefficients.
//!
//! `Line3` and `Plane` accept any non-degenerate affine transform: an affine
//! image of a line is a line, and an affine image of a plane (as long as it
//! does not collapse the spanning vectors) is a plane. `Circle3`, `Cylinder`,
//! and `Cone` require a *similarity* transform (rigid motion plus uniform
//! scale, no reflection), since only a similarity preserves circles,
//! circular cylinders, and circular cones as such. The uniform scale and any
//! scaled metric parameter must also be exactly representable as `f64`;
//! certificate-free primitive transformation cannot substitute a rounded
//! value for either.
//!
//! 2-D primitives (`Line2`, `Circle2`) are not covered: the foundation has
//! no `Transform2` type. Support will be added when one exists.
//!
//! Reflection (`det < 0`) geometrically preserves these families, but the v0
//! API deliberately rejects it via the positive-determinant check and returns
//! `TransformError::NotSimilarity`. A future release may add explicit
//! reflection support and `TransformError::ReflectionUnsupported`.

#![allow(clippy::many_single_char_names, clippy::similar_names)]

use amphion_foundation::{Point3, Transform3, Vector3};
use num_rational::BigRational;
use num_traits::{Signed, Zero};

use super::exact::{exact_rational_sqrt, f64_to_rat, rat_to_f64};

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
    /// The transform is an exact similarity, but its uniform scale or a
    /// resulting scaled metric parameter is not exactly representable as
    /// `f64`.
    UnrepresentableScale,
    /// An exact transformed point, vector, or affine basis component is not
    /// representable as `f64`.
    UnrepresentableResult,
    /// The transformed primitive is geometrically degenerate (e.g. its
    /// spanning vectors became dependent).
    DegenerateResult,
}

fn is_either_zero(value: f64) -> bool {
    value == 0.0
}

fn linear_part_is_identity(matrix: [f64; 12]) -> bool {
    matrix[0].to_bits() == 1.0_f64.to_bits()
        && is_either_zero(matrix[1])
        && is_either_zero(matrix[2])
        && is_either_zero(matrix[4])
        && matrix[5].to_bits() == 1.0_f64.to_bits()
        && is_either_zero(matrix[6])
        && is_either_zero(matrix[8])
        && is_either_zero(matrix[9])
        && matrix[10].to_bits() == 1.0_f64.to_bits()
}

/// Returns whether the transform acts as the exact identity, allowing either
/// sign bit on stored zero matrix entries.
pub(super) fn is_identity_transform(transform: &Transform3) -> bool {
    let matrix = transform.into_row_major();
    linear_part_is_identity(matrix)
        && is_either_zero(matrix[3])
        && is_either_zero(matrix[7])
        && is_either_zero(matrix[11])
}

fn exactly_representable(value: &BigRational) -> Result<f64, TransformError> {
    let candidate = rat_to_f64(value);
    if candidate.is_finite() && f64_to_rat(candidate) == *value {
        Ok(candidate)
    } else {
        Err(TransformError::UnrepresentableResult)
    }
}

fn exact_affine_components(
    matrix: [f64; 12],
    vector: [f64; 3],
    include_translation: bool,
) -> Result<[f64; 3], TransformError> {
    let values = vector.map(f64_to_rat);
    let component = |row: usize| {
        let base = f64_to_rat(matrix[row]) * &values[0]
            + f64_to_rat(matrix[row + 1]) * &values[1]
            + f64_to_rat(matrix[row + 2]) * &values[2];
        if include_translation {
            base + f64_to_rat(matrix[row + 3])
        } else {
            base
        }
    };
    Ok([
        exactly_representable(&component(0))?,
        exactly_representable(&component(4))?,
        exactly_representable(&component(8))?,
    ])
}

/// Applies an affine transform exactly and rejects any rounded coordinate.
pub(super) fn exact_transform_point(
    transform: &Transform3,
    point: Point3,
) -> Result<Point3, TransformError> {
    if is_identity_transform(transform) {
        return Ok(point);
    }
    let transformed =
        exact_affine_components(transform.into_row_major(), point.into_array(), true)?;
    Point3::try_new(transformed[0], transformed[1], transformed[2])
        .map_err(|_| TransformError::NonFiniteResult)
}

/// Applies a linear transform exactly and rejects any rounded component.
pub(super) fn exact_transform_vector(
    transform: &Transform3,
    vector: Vector3,
) -> Result<Vector3, TransformError> {
    let matrix = transform.into_row_major();
    if linear_part_is_identity(matrix) {
        return Ok(vector);
    }
    let transformed = exact_affine_components(matrix, vector.into_array(), false)?;
    Vector3::try_new(transformed[0], transformed[1], transformed[2])
        .map_err(|_| TransformError::NonFiniteResult)
}

/// Computes the uniform scale factor of a transform's linear part and
/// verifies it is a similarity: equal-magnitude, mutually orthogonal
/// columns, non-zero scale, and a positive determinant (no reflection).
///
/// Returns the exactly representable uniform scale when the linear part is a
/// supported similarity.
pub(super) fn similarity_scale(transform: &Transform3) -> Result<f64, TransformError> {
    let m = transform.into_row_major();
    let lin = [m[0], m[1], m[2], m[4], m[5], m[6], m[8], m[9], m[10]];
    if !lin.iter().all(|value| value.is_finite()) {
        return Err(TransformError::NotSimilarity);
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
        return Err(TransformError::NotSimilarity);
    }

    if n0 != n1 || n0 != n2 {
        return Err(TransformError::NotSimilarity);
    }

    let d01 = dot(&c0, &c1);
    let d02 = dot(&c0, &c2);
    let d12 = dot(&c1, &c2);
    if !d01.is_zero() || !d02.is_zero() || !d12.is_zero() {
        return Err(TransformError::NotSimilarity);
    }

    let det = &c0[0] * (&c1[1] * &c2[2] - &c1[2] * &c2[1])
        - &c0[1] * (&c1[0] * &c2[2] - &c1[2] * &c2[0])
        + &c0[2] * (&c1[0] * &c2[1] - &c1[1] * &c2[0]);
    if !det.is_positive() {
        return Err(TransformError::NotSimilarity);
    }

    let scale_rat = exact_rational_sqrt(&n0).ok_or(TransformError::UnrepresentableScale)?;
    let scale = rat_to_f64(&scale_rat);
    if !scale.is_finite() || scale <= 0.0 {
        return Err(TransformError::UnrepresentableScale);
    }
    if f64_to_rat(scale) != scale_rat {
        return Err(TransformError::UnrepresentableScale);
    }
    Ok(scale)
}

/// Multiplies a positive finite metric value by an exact similarity scale and
/// rejects the result unless the exact rational product is representable as
/// `f64`.
pub(super) fn exact_scaled_length(value: f64, scale: f64) -> Result<f64, TransformError> {
    let candidate = value * scale;
    if !candidate.is_finite() || candidate <= 0.0 {
        return Err(TransformError::UnrepresentableScale);
    }
    let exact = f64_to_rat(value) * f64_to_rat(scale);
    if f64_to_rat(candidate) != exact {
        return Err(TransformError::UnrepresentableScale);
    }
    Ok(candidate)
}

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use amphion_foundation::{Transform3, Vector3};

    use super::{
        TransformError, exact_scaled_length, exact_transform_vector, is_identity_transform,
        similarity_scale,
    };

    #[test]
    fn exact_similarity_accepts_identity() {
        assert_eq!(similarity_scale(&Transform3::IDENTITY), Ok(1.0));
        assert!(is_identity_transform(&Transform3::IDENTITY));
    }

    #[test]
    fn exact_vector_transform_recovers_cancelled_dyadic_component() {
        let transform = Transform3::try_from_row_major([
            3.0, -4.0, 0.0, 0.0, //
            4.0, 3.0, 0.0, 0.0, //
            0.0, 0.0, 5.0, 0.0,
        ])
        .unwrap();
        let vector = Vector3::try_new(1.649_161_156_696_929, 1.236_870_867_522_697, 0.0).unwrap();
        let transformed = exact_transform_vector(&transform, vector)
            .unwrap()
            .into_array();
        assert_eq!(transformed[0].to_bits(), (-2.0_f64.powi(-51)).to_bits());
    }

    #[test]
    fn exact_vector_transform_rejects_rounded_component() {
        let transform = Transform3::try_from_row_major([
            0.1, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0,
        ])
        .unwrap();
        let vector = Vector3::try_new(0.1, 0.0, 0.0).unwrap();
        assert_eq!(
            exact_transform_vector(&transform, vector),
            Err(TransformError::UnrepresentableResult)
        );
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
        assert_eq!(similarity_scale(&transform), Ok(scale));
    }

    #[test]
    fn exact_similarity_rejects_shear_at_unit_scale() {
        let transform = Transform3::try_from_row_major([
            1.0, 1.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0,
        ])
        .unwrap();
        assert_eq!(
            similarity_scale(&transform),
            Err(TransformError::NotSimilarity)
        );
    }

    #[test]
    fn exact_similarity_rejects_near_uniform_anisotropic_scale() {
        let transform = Transform3::try_from_row_major([
            1.0,
            0.0,
            0.0,
            0.0, //
            0.0,
            1.0 + f64::EPSILON,
            0.0,
            0.0, //
            0.0,
            0.0,
            1.0,
            0.0,
        ])
        .unwrap();
        assert_eq!(
            similarity_scale(&transform),
            Err(TransformError::NotSimilarity)
        );
    }

    #[test]
    fn exact_similarity_rejects_reflection() {
        let transform = Transform3::try_from_row_major([
            -1.0, 0.0, 0.0, 0.0, //
            0.0, 1.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0,
        ])
        .unwrap();
        assert_eq!(
            similarity_scale(&transform),
            Err(TransformError::NotSimilarity)
        );
    }

    #[test]
    fn exact_similarity_rejects_singular() {
        let transform = Transform3::try_from_row_major([
            0.0, 0.0, 0.0, 0.0, //
            0.0, 0.0, 0.0, 0.0, //
            0.0, 0.0, 0.0, 0.0,
        ])
        .unwrap();
        assert_eq!(
            similarity_scale(&transform),
            Err(TransformError::NotSimilarity)
        );
    }

    /// Regression: uniform scale 1e-200.
    ///
    /// Column norm-squared = (1e-200)² = 1e-400, which is below f64 minsub.
    /// The old `sqrt_up` fast path returned sqrt(minsub) ≈ 2.22e-162 instead of
    /// the tight result 1e-200, causing the transform to be classified with
    /// the wrong scale.  With the `BigInt` isqrt fix, the exact scale is returned.
    #[test]
    fn exact_similarity_accepts_tiny_scale() {
        let scale = 1.0e-200_f64;
        let transform = Transform3::try_from_row_major([
            scale, 0.0, 0.0, 0.0, //
            0.0, scale, 0.0, 0.0, //
            0.0, 0.0, scale, 0.0,
        ])
        .unwrap();
        let result = similarity_scale(&transform);
        assert_eq!(result, Ok(scale), "scale should be exactly 1e-200");
    }

    /// Regression: directed conversion of the exact squared norm can move
    /// `sqrt_up` one float above an otherwise exactly representable scale.
    #[test]
    fn exact_similarity_accepts_representable_scale_with_rounded_squared_norm() {
        let scale = f64::from_bits(0x3ffe_0ede_166d_d683);
        let transform = Transform3::try_from_row_major([
            scale, 0.0, 0.0, 0.0, //
            0.0, scale, 0.0, 0.0, //
            0.0, 0.0, scale, 0.0,
        ])
        .unwrap();
        assert_eq!(similarity_scale(&transform), Ok(scale));
    }

    /// An exact similarity can have a rational scale that is not representable
    /// as `f64`; using a directed upper bound would silently enlarge radii.
    #[test]
    fn exact_similarity_rejects_unrepresentable_scale() {
        let q = 729_000_054_000_001.0;
        let transform = Transform3::try_from_row_major([
            -3.0 * q,
            4.0 * q,
            12.0 * q,
            0.0,
            12.0 * q,
            -3.0 * q,
            4.0 * q,
            0.0,
            4.0 * q,
            12.0 * q,
            -3.0 * q,
            0.0,
        ])
        .unwrap();
        assert_eq!(
            similarity_scale(&transform),
            Err(TransformError::UnrepresentableScale)
        );
    }

    #[test]
    fn exact_scaled_length_rejects_rounded_product() {
        assert_eq!(
            exact_scaled_length(0.1, 0.1),
            Err(TransformError::UnrepresentableScale)
        );
        assert_eq!(exact_scaled_length(0.5, 2.0), Ok(1.0));
    }
}
