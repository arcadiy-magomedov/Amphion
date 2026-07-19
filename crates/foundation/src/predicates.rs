//! Exact orientation predicates with explicit degenerate and uncertain
//! outcomes.
//!
//! Every finite `f64` coordinate is decoded exactly as a dyadic integer
//! `mantissa × 2^exp`; determinants and tolerance thresholds are computed with
//! multi-precision integer arithmetic (`num-bigint`).  The result therefore
//! certifies the determinant of the **original** `f64` inputs without any
//! rounding — even for extreme, subnormal, or near-boundary configurations.
//!
//! There is deliberately no uncertified fast-path classifier.  A certified
//! floating-point interval filter may be added in a future optimisation pass;
//! until then the exact path is authoritative for every call.
//!
//! # Performance note
//!
//! All calls allocate heap-resident `BigInt` values.  Typical CAD workloads
//! tolerate this because predicate calls are sparse relative to mesh
//! traversal; callers with hot inner loops should batch or cache results.

use core::error::Error;
use core::fmt;

use num_bigint::BigInt;
use num_traits::{Signed, Zero};

use crate::math::{Point2, Point3};
use crate::tolerance::{ToleranceContext, ToleranceError};

/// The signed orientation returned by a geometric predicate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OrientationSign {
    /// Determinant is positive and exceeds the modeling-tolerance threshold.
    Positive,
    /// Determinant is negative and its magnitude exceeds the threshold.
    Negative,
    /// The exact determinant is zero; the points are provably collinear or
    /// coplanar.
    Degenerate,
    /// The exact sign is nonzero, but the determinant magnitude is within the
    /// scale-aware modeling tolerance.
    Uncertain,
}

/// Error returned when a predicate cannot be evaluated safely.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PredicateError {
    /// Tolerance evaluation failed (for example because the effective length
    /// scale overflowed).
    Tolerance(ToleranceError),
}

impl fmt::Display for PredicateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tolerance(error) => write!(formatter, "tolerance evaluation failed: {error}"),
        }
    }
}

impl Error for PredicateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Tolerance(error) => Some(error),
        }
    }
}

impl From<ToleranceError> for PredicateError {
    fn from(value: ToleranceError) -> Self {
        Self::Tolerance(value)
    }
}

/// Decode a finite `f64` as `significand × 2^exp`.
///
/// Returns `(0, 0)` for both signed zeros. The caller must guarantee that `x`
/// is finite.
#[must_use]
fn decode_f64(x: f64) -> (i64, i32) {
    debug_assert!(x.is_finite());
    let bits = x.to_bits();
    let sign = if bits >> 63 == 0 { 1_i64 } else { -1_i64 };
    let biased_exp = ((bits >> 52) & 0x7ff) as i32;
    let mantissa_bits = (bits & 0x000f_ffff_ffff_ffff).cast_signed();
    if biased_exp == 0 {
        if mantissa_bits == 0 {
            (0, 0)
        } else {
            (sign * mantissa_bits, -1074)
        }
    } else {
        (sign * ((1_i64 << 52) + mantissa_bits), biased_exp - 1075)
    }
}

/// Exact dyadic value `m × 2^e`.
#[derive(Clone, Debug)]
struct Dyadic {
    m: BigInt,
    e: i32,
}

impl Dyadic {
    #[must_use]
    fn from_f64(x: f64) -> Self {
        let (mantissa, exponent) = decode_f64(x);
        Self::with_parts(BigInt::from(mantissa), exponent)
    }

    #[must_use]
    fn zero() -> Self {
        Self {
            m: BigInt::zero(),
            e: 0,
        }
    }

    #[must_use]
    fn with_parts(mantissa: BigInt, exponent: i32) -> Self {
        if mantissa.is_zero() {
            Self::zero()
        } else {
            Self {
                m: mantissa,
                e: exponent,
            }
        }
    }

    #[must_use]
    fn plus(&self, rhs: &Self) -> Self {
        let min_exponent = self.e.min(rhs.e);
        let lhs = &self.m << (self.e - min_exponent).unsigned_abs();
        let rhs = &rhs.m << (rhs.e - min_exponent).unsigned_abs();
        Self::with_parts(lhs + rhs, min_exponent)
    }

    #[must_use]
    fn minus(&self, rhs: &Self) -> Self {
        let min_exponent = self.e.min(rhs.e);
        let lhs = &self.m << (self.e - min_exponent).unsigned_abs();
        let rhs = &rhs.m << (rhs.e - min_exponent).unsigned_abs();
        Self::with_parts(lhs - rhs, min_exponent)
    }

    #[must_use]
    fn times(&self, rhs: &Self) -> Self {
        Self::with_parts(&self.m * &rhs.m, self.e + rhs.e)
    }

    #[must_use]
    fn squared(&self) -> Self {
        Self::with_parts(&self.m * &self.m, self.e + self.e)
    }

    #[must_use]
    fn sign(&self) -> i32 {
        if self.m.is_positive() {
            1
        } else if self.m.is_negative() {
            -1
        } else {
            0
        }
    }

    #[must_use]
    fn is_zero(&self) -> bool {
        self.m.is_zero()
    }

    /// Returns true when `|self| <= |rhs|`.
    #[must_use]
    fn abs_le(&self, rhs: &Self) -> bool {
        let lhs_abs = self.m.magnitude();
        let rhs_abs = rhs.m.magnitude();
        let exponent_diff = rhs.e - self.e;
        if exponent_diff >= 0 {
            lhs_abs <= &(rhs_abs << exponent_diff.unsigned_abs())
        } else {
            &(lhs_abs << exponent_diff.unsigned_abs()) <= rhs_abs
        }
    }

    /// Returns the larger of two non-negative dyadics.
    #[must_use]
    fn max_nonneg(&self, other: &Self) -> Self {
        if self.abs_le(other) {
            other.clone()
        } else {
            self.clone()
        }
    }
}

#[must_use]
fn point2_dyadic(point: Point2) -> [Dyadic; 2] {
    [Dyadic::from_f64(point.x()), Dyadic::from_f64(point.y())]
}

#[must_use]
fn point3_dyadic(point: Point3) -> [Dyadic; 3] {
    [
        Dyadic::from_f64(point.x()),
        Dyadic::from_f64(point.y()),
        Dyadic::from_f64(point.z()),
    ]
}

#[must_use]
fn dyadic_diff<const N: usize>(lhs: &[Dyadic; N], rhs: &[Dyadic; N]) -> [Dyadic; N] {
    core::array::from_fn(|index| lhs[index].minus(&rhs[index]))
}

#[must_use]
fn vec_sq<const N: usize>(vector: &[Dyadic; N]) -> Dyadic {
    let mut sum = Dyadic::zero();
    for component in vector {
        sum = sum.plus(&component.squared());
    }
    sum
}

#[must_use]
fn det2(edge_lhs: &[Dyadic; 2], edge_rhs: &[Dyadic; 2]) -> Dyadic {
    edge_lhs[0]
        .times(&edge_rhs[1])
        .minus(&edge_rhs[0].times(&edge_lhs[1]))
}

#[must_use]
fn cross3(lhs: &[Dyadic; 3], rhs: &[Dyadic; 3]) -> [Dyadic; 3] {
    [
        lhs[1].times(&rhs[2]).minus(&lhs[2].times(&rhs[1])),
        lhs[2].times(&rhs[0]).minus(&lhs[0].times(&rhs[2])),
        lhs[0].times(&rhs[1]).minus(&lhs[1].times(&rhs[0])),
    ]
}

#[must_use]
fn cross3_sq(lhs: &[Dyadic; 3], rhs: &[Dyadic; 3]) -> Dyadic {
    vec_sq(&cross3(lhs, rhs))
}

#[must_use]
fn det3(edge_base: &[Dyadic; 3], edge_left: &[Dyadic; 3], edge_right: &[Dyadic; 3]) -> Dyadic {
    let cross_left_right = cross3(edge_left, edge_right);
    edge_base[0]
        .times(&cross_left_right[0])
        .plus(&edge_base[1].times(&cross_left_right[1]))
        .plus(&edge_base[2].times(&cross_left_right[2]))
}

/// Exact `f64::MAX² = (2^53 - 1)² × 2^1942`.
#[must_use]
fn f64_max_sq() -> Dyadic {
    let max_significand = BigInt::from((1_u64 << 53) - 1);
    Dyadic::with_parts(&max_significand * &max_significand, 1942)
}

fn check_overflow(rel_tol: f64, max_edge_sq: &Dyadic) -> Result<(), ToleranceError> {
    if rel_tol == 0.0 {
        return Ok(());
    }
    let rel_sq_times_edge_sq = Dyadic::from_f64(rel_tol).squared().times(max_edge_sq);
    if !rel_sq_times_edge_sq.abs_le(&f64_max_sq()) {
        return Err(ToleranceError::Overflow);
    }
    Ok(())
}

/// Returns the orientation encoded by a nonzero sign integer.
#[must_use]
fn orientation_from_sign(sign: i32) -> OrientationSign {
    if sign > 0 {
        OrientationSign::Positive
    } else {
        OrientationSign::Negative
    }
}

/// 2-D orientation of three points using exact dyadic integer arithmetic.
///
/// Positive means that `a → b → c` is counter-clockwise in a right-handed
/// frame.
///
/// Every input coordinate is decoded exactly as a dyadic rational; the
/// determinant sign and tolerance threshold are computed with `BigInt`
/// arithmetic and certify the result for the **original** `f64` values.
///
/// # Errors
///
/// Returns [`PredicateError::Tolerance`] when the supplied relative tolerance
/// overflows its effective comparison scale.
pub fn orient2d(
    a: Point2,
    b: Point2,
    c: Point2,
    ctx: ToleranceContext,
) -> Result<OrientationSign, PredicateError> {
    let exact_a = point2_dyadic(a);
    let exact_b = point2_dyadic(b);
    let exact_c = point2_dyadic(c);
    let edge_primary = dyadic_diff(&exact_b, &exact_a);
    let edge_secondary = dyadic_diff(&exact_c, &exact_a);
    let det = det2(&edge_primary, &edge_secondary);
    if det.is_zero() {
        return Ok(OrientationSign::Degenerate);
    }

    let edge_between = dyadic_diff(&exact_c, &exact_b);
    let edge_sq = [
        vec_sq(&edge_primary),
        vec_sq(&edge_secondary),
        vec_sq(&edge_between),
    ];
    let max_edge_sq = edge_sq[0].max_nonneg(&edge_sq[1]).max_nonneg(&edge_sq[2]);

    check_overflow(ctx.relative_length(), &max_edge_sq)?;

    let det_sq = det.squared();
    let abs_threshold = Dyadic::from_f64(ctx.absolute_length())
        .squared()
        .times(&max_edge_sq);
    if det_sq.abs_le(&abs_threshold) {
        return Ok(OrientationSign::Uncertain);
    }

    let rel_tol = ctx.relative_length();
    if rel_tol > 0.0 {
        let rel_threshold = Dyadic::from_f64(rel_tol)
            .squared()
            .times(&max_edge_sq.squared());
        if det_sq.abs_le(&rel_threshold) {
            return Ok(OrientationSign::Uncertain);
        }
    }

    Ok(orientation_from_sign(det.sign()))
}

/// 3-D orientation of four points using exact dyadic integer arithmetic.
///
/// Returns [`OrientationSign::Positive`] when `d` lies on the positive side of
/// the plane through `a`, `b`, `c` in a right-handed frame (scalar triple
/// product `(b−a) · ((c−a) × (d−a)) > 0`).
///
/// Every input coordinate is decoded exactly as a dyadic rational; the
/// determinant sign and tolerance threshold are computed with `BigInt`
/// arithmetic and certify the result for the **original** `f64` values.
///
/// # Errors
///
/// Returns [`PredicateError::Tolerance`] when the supplied relative tolerance
/// overflows its effective comparison scale.
pub fn orient3d(
    a: Point3,
    b: Point3,
    c: Point3,
    d: Point3,
    ctx: ToleranceContext,
) -> Result<OrientationSign, PredicateError> {
    let exact_a = point3_dyadic(a);
    let exact_b = point3_dyadic(b);
    let exact_c = point3_dyadic(c);
    let exact_d = point3_dyadic(d);
    let edge_from_a = [
        dyadic_diff(&exact_b, &exact_a),
        dyadic_diff(&exact_c, &exact_a),
        dyadic_diff(&exact_d, &exact_a),
    ];
    let det = det3(&edge_from_a[0], &edge_from_a[1], &edge_from_a[2]);
    if det.is_zero() {
        return Ok(OrientationSign::Degenerate);
    }

    let edge_from_b = [
        dyadic_diff(&exact_c, &exact_b),
        dyadic_diff(&exact_d, &exact_b),
    ];
    let edge_from_c = [dyadic_diff(&exact_d, &exact_c)];
    let edge_sq = [
        vec_sq(&edge_from_a[0]),
        vec_sq(&edge_from_a[1]),
        vec_sq(&edge_from_a[2]),
        vec_sq(&edge_from_b[0]),
        vec_sq(&edge_from_b[1]),
        vec_sq(&edge_from_c[0]),
    ];
    let max_edge_sq = edge_sq[0]
        .max_nonneg(&edge_sq[1])
        .max_nonneg(&edge_sq[2])
        .max_nonneg(&edge_sq[3])
        .max_nonneg(&edge_sq[4])
        .max_nonneg(&edge_sq[5]);
    let face_para_sq = [
        cross3_sq(&edge_from_a[0], &edge_from_a[1]),
        cross3_sq(&edge_from_a[0], &edge_from_a[2]),
        cross3_sq(&edge_from_a[1], &edge_from_a[2]),
        cross3_sq(&edge_from_b[0], &edge_from_b[1]),
    ];
    let max_face_para_sq = face_para_sq[0]
        .max_nonneg(&face_para_sq[1])
        .max_nonneg(&face_para_sq[2])
        .max_nonneg(&face_para_sq[3]);

    check_overflow(ctx.relative_length(), &max_edge_sq)?;

    let det_sq = det.squared();
    let abs_threshold = Dyadic::from_f64(ctx.absolute_length())
        .squared()
        .times(&max_face_para_sq);
    if det_sq.abs_le(&abs_threshold) {
        return Ok(OrientationSign::Uncertain);
    }

    let rel_tol = ctx.relative_length();
    if rel_tol > 0.0 {
        let rel_threshold = Dyadic::from_f64(rel_tol)
            .squared()
            .times(&max_edge_sq)
            .times(&max_face_para_sq);
        if det_sq.abs_le(&rel_threshold) {
            return Ok(OrientationSign::Uncertain);
        }
    }

    Ok(orientation_from_sign(det.sign()))
}

#[cfg(test)]
mod tests {
    use super::{OrientationSign, PredicateError, orient2d, orient3d};
    use crate::math::{Point2, Point3};
    use crate::tolerance::{ToleranceContext, ToleranceError};

    fn ctx() -> ToleranceContext {
        ToleranceContext::try_new(1.0e-9, 1.0e-8, 1.0e-7, 1.0e-10).unwrap()
    }

    #[test]
    fn orient2d_ccw_is_positive() {
        let a = Point2::try_new(0.0, 0.0).unwrap();
        let b = Point2::try_new(1.0, 0.0).unwrap();
        let c = Point2::try_new(0.0, 1.0).unwrap();
        assert_eq!(orient2d(a, b, c, ctx()).unwrap(), OrientationSign::Positive);
    }

    #[test]
    fn orient2d_cw_is_negative() {
        let a = Point2::try_new(0.0, 0.0).unwrap();
        let b = Point2::try_new(0.0, 1.0).unwrap();
        let c = Point2::try_new(1.0, 0.0).unwrap();
        assert_eq!(orient2d(a, b, c, ctx()).unwrap(), OrientationSign::Negative);
    }

    #[test]
    fn orient2d_exact_collinear_is_degenerate() {
        let a = Point2::try_new(0.0, 0.0).unwrap();
        let b = Point2::try_new(1.0, 0.0).unwrap();
        let c = Point2::try_new(2.0, 0.0).unwrap();
        assert_eq!(
            orient2d(a, b, c, ctx()).unwrap(),
            OrientationSign::Degenerate
        );
    }

    #[test]
    fn orient2d_near_collinear_is_uncertain() {
        let a = Point2::try_new(0.0, 0.0).unwrap();
        let b = Point2::try_new(1.0, 0.0).unwrap();
        let c = Point2::try_new(0.5, 1.0e-12).unwrap();
        assert_eq!(
            orient2d(a, b, c, ctx()).unwrap(),
            OrientationSign::Uncertain
        );
    }

    #[test]
    fn orient3d_positive_tetrahedron() {
        let a = Point3::try_new(0.0, 0.0, 0.0).unwrap();
        let b = Point3::try_new(1.0, 0.0, 0.0).unwrap();
        let c = Point3::try_new(0.0, 1.0, 0.0).unwrap();
        let d = Point3::try_new(0.0, 0.0, 1.0).unwrap();
        assert_eq!(
            orient3d(a, b, c, d, ctx()).unwrap(),
            OrientationSign::Positive
        );
    }

    #[test]
    fn orient3d_negative_tetrahedron() {
        let a = Point3::try_new(0.0, 0.0, 0.0).unwrap();
        let b = Point3::try_new(0.0, 1.0, 0.0).unwrap();
        let c = Point3::try_new(1.0, 0.0, 0.0).unwrap();
        let d = Point3::try_new(0.0, 0.0, 1.0).unwrap();
        assert_eq!(
            orient3d(a, b, c, d, ctx()).unwrap(),
            OrientationSign::Negative
        );
    }

    #[test]
    fn orient3d_exact_coplanar_is_degenerate() {
        let a = Point3::try_new(0.0, 0.0, 0.0).unwrap();
        let b = Point3::try_new(1.0, 0.0, 0.0).unwrap();
        let c = Point3::try_new(0.0, 1.0, 0.0).unwrap();
        let d = Point3::try_new(1.0, 1.0, 0.0).unwrap();
        assert_eq!(
            orient3d(a, b, c, d, ctx()).unwrap(),
            OrientationSign::Degenerate
        );
    }

    #[test]
    fn orient3d_near_coplanar_is_uncertain() {
        let a = Point3::try_new(0.0, 0.0, 0.0).unwrap();
        let b = Point3::try_new(1.0, 0.0, 0.0).unwrap();
        let c = Point3::try_new(0.0, 1.0, 0.0).unwrap();
        let d = Point3::try_new(0.0, 0.0, 1.0e-11).unwrap();
        assert_eq!(
            orient3d(a, b, c, d, ctx()).unwrap(),
            OrientationSign::Uncertain
        );
    }

    #[test]
    fn orient2d_scale_change_preserves_sign() {
        let scale = 1000.0_f64;
        let a = Point2::try_new(0.0, 0.0).unwrap();
        let b = Point2::try_new(scale, 0.0).unwrap();
        let c = Point2::try_new(0.0, scale).unwrap();
        assert_eq!(orient2d(a, b, c, ctx()).unwrap(), OrientationSign::Positive);
    }

    #[test]
    fn orient2d_deterministic() {
        let a = Point2::try_new(1.0, 2.0).unwrap();
        let b = Point2::try_new(3.0, 4.0).unwrap();
        let c = Point2::try_new(5.0, 7.0).unwrap();
        let first = orient2d(a, b, c, ctx()).unwrap();
        let second = orient2d(a, b, c, ctx()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn orient2d_adversarial_cancellation_gives_correct_sign() {
        let a = Point2::try_new(0.5, 0.5).unwrap();
        let b = Point2::try_new(12.0, 12.0).unwrap();
        let c = Point2::try_new(24.0, 24.0 - 1.0e-10).unwrap();
        let result = orient2d(a, b, c, ctx()).unwrap();
        assert!(
            result == OrientationSign::Negative || result == OrientationSign::Uncertain,
            "adversarial near-collinear must be negative or uncertain, got {result:?}"
        );
    }

    #[test]
    fn orient2d_translated_preserves_sign() {
        let offset_x = 1_000_000.0_f64;
        let a = Point2::try_new(offset_x, 0.0).unwrap();
        let b = Point2::try_new(1.0 + offset_x, 0.0).unwrap();
        let c = Point2::try_new(offset_x, 1.0).unwrap();
        assert_eq!(orient2d(a, b, c, ctx()).unwrap(), OrientationSign::Positive);
    }

    #[test]
    fn orient2d_huge_coordinates_give_consistent_sign() {
        let huge = 1.0e150_f64;
        let a = Point2::try_new(huge, huge).unwrap();
        let b = Point2::try_new(huge * 2.0, huge).unwrap();
        let c = Point2::try_new(huge, huge * 2.0).unwrap();
        assert_eq!(orient2d(a, b, c, ctx()).unwrap(), OrientationSign::Positive);
    }

    #[test]
    fn orient3d_permutation_changes_sign() {
        let a = Point3::try_new(0.0, 0.0, 0.0).unwrap();
        let b = Point3::try_new(1.0, 0.0, 0.0).unwrap();
        let c = Point3::try_new(0.0, 1.0, 0.0).unwrap();
        let d = Point3::try_new(0.0, 0.0, 1.0).unwrap();
        let positive = orient3d(a, b, c, d, ctx()).unwrap();
        let negative = orient3d(a, c, b, d, ctx()).unwrap();
        assert_eq!(positive, OrientationSign::Positive);
        assert_eq!(negative, OrientationSign::Negative);
    }

    #[test]
    fn orient2d_degenerate_not_from_nan_path() {
        let a = Point2::try_new(1.0, 1.0).unwrap();
        assert_eq!(
            orient2d(a, a, a, ctx()).unwrap(),
            OrientationSign::Degenerate
        );
    }

    #[test]
    fn orient2d_boundary_just_below_uncertain() {
        let tolerance = ctx().effective_length(1.0).unwrap();
        let a = Point2::try_new(0.0, 0.0).unwrap();
        let b = Point2::try_new(1.0, 0.0).unwrap();
        let c = Point2::try_new(1.0, tolerance - 1.0e-12).unwrap();
        assert_eq!(
            orient2d(a, b, c, ctx()).unwrap(),
            OrientationSign::Uncertain
        );
    }

    #[test]
    fn orient2d_boundary_just_above_uncertain() {
        let tolerance = ctx().effective_length(1.0).unwrap();
        let a = Point2::try_new(0.0, 0.0).unwrap();
        let b = Point2::try_new(1.0, 0.0).unwrap();
        let c = Point2::try_new(1.0, tolerance + 1.0e-6).unwrap();
        assert_eq!(orient2d(a, b, c, ctx()).unwrap(), OrientationSign::Positive);
    }

    #[test]
    fn orient2d_scale1000_preserves_sign() {
        let scale = 1000.0_f64;
        let a = Point2::try_new(0.0, 0.0).unwrap();
        let b = Point2::try_new(scale, 0.0).unwrap();
        let c = Point2::try_new(scale, scale).unwrap();
        assert_eq!(orient2d(a, b, c, ctx()).unwrap(), OrientationSign::Positive);
    }

    #[test]
    fn orient2d_skinny_base_triangle() {
        let a = Point2::try_new(0.0, 0.0).unwrap();
        let b = Point2::try_new(1.0e9, 0.0).unwrap();
        let c = Point2::try_new(1.0e9, 100.0).unwrap();
        assert_eq!(orient2d(a, b, c, ctx()).unwrap(), OrientationSign::Positive);
    }

    #[test]
    fn orient3d_deterministic() {
        let a = Point3::try_new(0.0, 0.0, 0.0).unwrap();
        let b = Point3::try_new(3.0, 0.0, 0.0).unwrap();
        let c = Point3::try_new(0.0, 4.0, 0.0).unwrap();
        let d = Point3::try_new(0.0, 0.0, 5.0).unwrap();
        let first = orient3d(a, b, c, d, ctx()).unwrap();
        let second = orient3d(a, b, c, d, ctx()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn orient3d_scale_100_works() {
        let a = Point3::try_new(0.0, 0.0, 0.0).unwrap();
        let b = Point3::try_new(100.0, 0.0, 0.0).unwrap();
        let c = Point3::try_new(0.0, 100.0, 0.0).unwrap();
        let d = Point3::try_new(0.0, 0.0, 100.0).unwrap();
        assert_eq!(
            orient3d(a, b, c, d, ctx()).unwrap(),
            OrientationSign::Positive
        );
    }

    #[test]
    fn orient3d_huge_scale_gives_valid_result() {
        let huge = f64::MAX * 0.75;
        let a = Point3::try_new(0.0, 0.0, 0.0).unwrap();
        let b = Point3::try_new(huge, huge, huge).unwrap();
        let c = Point3::try_new(huge, 0.0, 0.0).unwrap();
        let d = Point3::try_new(0.0, huge, 0.0).unwrap();
        let result = orient3d(a, b, c, d, ctx()).unwrap();
        assert!(result == OrientationSign::Positive || result == OrientationSign::Negative);
    }

    #[test]
    fn orient2d_regression_huge_ccw() {
        let local_ctx = ToleranceContext::try_new(1e-9, 1e-8, 1e-7, 1e-10).unwrap();
        let a = Point2::try_new(0.0, 0.0).unwrap();
        let b = Point2::try_new(1e200, 0.0).unwrap();
        let c = Point2::try_new(0.0, 1e200).unwrap();
        assert_eq!(
            orient2d(a, b, c, local_ctx).unwrap(),
            OrientationSign::Positive
        );
    }

    #[test]
    fn orient3d_regression_huge_positive() {
        let local_ctx = ToleranceContext::try_new(1e-9, 1e-9, 1e-7, 1e-10).unwrap();
        let scale = 1e110_f64;
        let a = Point3::try_new(0.0, 0.0, 0.0).unwrap();
        let b = Point3::try_new(scale, 0.0, 0.0).unwrap();
        let c = Point3::try_new(0.0, scale, 0.0).unwrap();
        let d = Point3::try_new(0.0, 0.0, scale).unwrap();
        assert_eq!(
            orient3d(a, b, c, d, local_ctx).unwrap(),
            OrientationSign::Positive
        );
    }

    #[test]
    fn orient2d_subnormal_geometry_is_uncertain_not_degenerate() {
        let epsilon = f64::from_bits(1);
        let local_ctx = ToleranceContext::try_new(1e-9, 1e-8, 1e-7, 1e-10).unwrap();
        let a = Point2::try_new(0.0, 0.0).unwrap();
        let b = Point2::try_new(epsilon, 0.0).unwrap();
        let c = Point2::try_new(0.0, epsilon).unwrap();
        let result = orient2d(a, b, c, local_ctx).unwrap();
        assert_ne!(
            result,
            OrientationSign::Degenerate,
            "subnormal geometry must not be classified as Degenerate"
        );
        assert_eq!(result, OrientationSign::Uncertain);
    }

    #[test]
    fn orient2d_permutation_sign_consistency() {
        let a = Point2::try_new(0.0, 0.0).unwrap();
        let b = Point2::try_new(1.0, 0.0).unwrap();
        let c = Point2::try_new(0.0, 1.0).unwrap();
        let local_ctx = ToleranceContext::try_new(1e-9, 1e-8, 1e-7, 1e-10).unwrap();
        assert_eq!(
            orient2d(a, b, c, local_ctx).unwrap(),
            OrientationSign::Positive
        );
        assert_eq!(
            orient2d(a, c, b, local_ctx).unwrap(),
            OrientationSign::Negative
        );
        assert_eq!(
            orient2d(b, c, a, local_ctx).unwrap(),
            OrientationSign::Positive
        );
        assert_eq!(
            orient2d(c, a, b, local_ctx).unwrap(),
            OrientationSign::Positive
        );
    }

    #[test]
    fn orient2d_uncertain_is_permutation_invariant() {
        let a = Point2::try_new(0.0, 0.0).unwrap();
        let b = Point2::try_new(1.0, 0.0).unwrap();
        let c = Point2::try_new(0.5, 1e-12).unwrap();
        let local_ctx = ToleranceContext::try_new(1e-9, 1e-8, 1e-7, 1e-10).unwrap();
        let first = orient2d(a, b, c, local_ctx).unwrap();
        let second = orient2d(b, c, a, local_ctx).unwrap();
        let third = orient2d(c, a, b, local_ctx).unwrap();
        assert_eq!(first, OrientationSign::Uncertain);
        assert_eq!(second, OrientationSign::Uncertain);
        assert_eq!(third, OrientationSign::Uncertain);
    }

    #[test]
    fn orient3d_uncertain_is_permutation_invariant() {
        let a = Point3::try_new(0.0, 0.0, 0.0).unwrap();
        let b = Point3::try_new(1.0, 0.0, 0.0).unwrap();
        let c = Point3::try_new(0.0, 1.0, 0.0).unwrap();
        let d = Point3::try_new(0.0, 0.0, 1e-11).unwrap();
        let local_ctx = ToleranceContext::try_new(1e-9, 1e-8, 1e-7, 1e-10).unwrap();
        let first = orient3d(a, b, c, d, local_ctx).unwrap();
        let second = orient3d(b, a, c, d, local_ctx).unwrap();
        assert_eq!(first, OrientationSign::Uncertain);
        assert_eq!(second, OrientationSign::Uncertain);
    }

    #[test]
    fn orient2d_regression_cc1_sign_preserved_near_unity() {
        let eps = 2.0_f64.powi(-54);
        let local_ctx = ToleranceContext::try_new(f64::from_bits(1), 0.0, 1e-7, 1e-10).unwrap();
        let a = Point2::try_new(1.0, 1.0).unwrap();
        let b = Point2::try_new(0.0, -eps).unwrap();
        let c = Point2::try_new(-eps, 0.0).unwrap();
        assert_eq!(
            orient2d(a, b, c, local_ctx).unwrap(),
            OrientationSign::Negative,
            "det = 1-(1+2^-54)^2 < 0; scaled code would return Degenerate"
        );
    }

    #[test]
    fn orient2d_regression_cc2_scaled_gives_wrong_positive() {
        let bx = 3.0_f64;
        let by = f64::from_bits(0x3FFF_D830_6FE2_69C6_u64);
        let cx = f64::from_bits(0x3FFF_D830_6FE2_69C6_u64);
        let cy = f64::from_bits(0x3FF5_2061_99F9_9F21_u64);
        let local_ctx = ToleranceContext::try_new(f64::from_bits(1), 0.0, 1e-7, 1e-10).unwrap();
        let a = Point2::try_new(0.0, 0.0).unwrap();
        let b = Point2::try_new(bx, by).unwrap();
        let c = Point2::try_new(cx, cy).unwrap();
        assert_eq!(
            orient2d(a, b, c, local_ctx).unwrap(),
            OrientationSign::Negative,
            "exact det of original f64 coords is negative; scaled code returned Positive"
        );
    }

    #[test]
    fn orient2d_regression_cc3_subnormal_y_not_erased() {
        let subnormal = f64::from_bits(1);
        let local_ctx = ToleranceContext::try_new(f64::from_bits(1), 0.0, 1e-7, 1e-10).unwrap();
        let a = Point2::try_new(0.0, 0.0).unwrap();
        let b = Point2::try_new(f64::MAX, 0.0).unwrap();
        let c = Point2::try_new(0.0, subnormal).unwrap();
        let result = orient2d(a, b, c, local_ctx).unwrap();
        assert_ne!(
            result,
            OrientationSign::Degenerate,
            "det = MAX × subnormal ≠ 0; must not be Degenerate"
        );
        assert!(result == OrientationSign::Positive || result == OrientationSign::Uncertain);
    }

    #[test]
    fn orient2d_tolerance_overflow_is_error() {
        let local_ctx = ToleranceContext::try_new(1e-9, f64::MAX, 1e-7, 1e-10).unwrap();
        let a = Point2::try_new(0.0, 0.0).unwrap();
        let b = Point2::try_new(2.0, 0.0).unwrap();
        let c = Point2::try_new(0.0, 1.0).unwrap();
        match orient2d(a, b, c, local_ctx) {
            Err(PredicateError::Tolerance(ToleranceError::Overflow)) => {}
            other => panic!("expected Tolerance(Overflow), got {other:?}"),
        }
    }

    #[test]
    fn orient3d_tolerance_overflow_is_error() {
        let local_ctx = ToleranceContext::try_new(1e-9, f64::MAX, 1e-7, 1e-10).unwrap();
        let a = Point3::try_new(0.0, 0.0, 0.0).unwrap();
        let b = Point3::try_new(2.0, 0.0, 0.0).unwrap();
        let c = Point3::try_new(0.0, 2.0, 0.0).unwrap();
        let d = Point3::try_new(0.0, 0.0, 2.0).unwrap();
        match orient3d(a, b, c, d, local_ctx) {
            Err(PredicateError::Tolerance(ToleranceError::Overflow)) => {}
            other => panic!("expected Tolerance(Overflow), got {other:?}"),
        }
    }

    #[test]
    fn orient3d_regression_cc1_embedded() {
        let eps = 2.0_f64.powi(-54);
        let local_ctx = ToleranceContext::try_new(f64::from_bits(1), 0.0, 1e-7, 1e-10).unwrap();
        let a = Point3::try_new(1.0, 1.0, 0.0).unwrap();
        let b = Point3::try_new(0.0, -eps, 0.0).unwrap();
        let c = Point3::try_new(-eps, 0.0, 0.0).unwrap();
        let d = Point3::try_new(0.0, 0.0, 1.0).unwrap();
        assert_eq!(
            orient3d(a, b, c, d, local_ctx).unwrap(),
            OrientationSign::Negative
        );
    }

    #[test]
    fn orient2d_exact_threshold_just_below() {
        let h = 1.910_076_881_312_769_7e-5_f64;
        let abs_tol = h * (1.0 + f64::EPSILON);
        let local_ctx = ToleranceContext::try_new(abs_tol, 0.0, 1e-7, 1e-10).unwrap();
        let a = Point2::try_new(-0.5, 0.0).unwrap();
        let b = Point2::try_new(0.5, 0.0).unwrap();
        let c = Point2::try_new(0.0, h).unwrap();
        assert_eq!(
            orient2d(a, b, c, local_ctx).unwrap(),
            OrientationSign::Uncertain
        );
    }

    #[test]
    fn orient2d_exact_threshold_just_above() {
        let h = 1.910_076_881_312_769_7e-5_f64;
        let abs_tol = h * (1.0 - f64::EPSILON);
        let local_ctx = ToleranceContext::try_new(abs_tol, 0.0, 1e-7, 1e-10).unwrap();
        let a = Point2::try_new(-0.5, 0.0).unwrap();
        let b = Point2::try_new(0.5, 0.0).unwrap();
        let c = Point2::try_new(0.0, h).unwrap();
        assert_eq!(
            orient2d(a, b, c, local_ctx).unwrap(),
            OrientationSign::Positive
        );
    }

    /// Regression: with h = 2^-27 the largest exact edge-squared is
    /// 1 + 2^-54 (edge BC).  `rel_tol² × (1 + 2^-54) > MAX²` so overflow
    /// must be returned.  An uncertified fast path would compute
    /// `fast_max_edge ≈ 1.0` (the subnormal correction rounds away), call
    /// `effective_length(1.0) = MAX` (no f64 overflow), and incorrectly
    /// short-circuit to `Uncertain`, bypassing the overflow error.
    #[test]
    fn orient2d_overflow_not_bypassed_by_fast_path() {
        let h = 2.0_f64.powi(-27);
        // abs_tol=1 (> h, so altitude is within absolute band), rel_tol=MAX
        let local_ctx = ToleranceContext::try_new(1.0, f64::MAX, 1e-7, 1e-10).unwrap();
        let a = Point2::try_new(0.0, 0.0).unwrap();
        let b = Point2::try_new(1.0, 0.0).unwrap();
        let c = Point2::try_new(0.0, h).unwrap();
        // Exact max_edge_sq = |BC|² = 1 + h² = 1 + 2^-54.
        // MAX² × (1 + 2^-54) > MAX² → overflow.
        match orient2d(a, b, c, local_ctx) {
            Err(PredicateError::Tolerance(ToleranceError::Overflow)) => {}
            other => panic!("expected Tolerance(Overflow), got {other:?}"),
        }
    }

    /// 3-D analogue of `orient2d_overflow_not_bypassed_by_fast_path`.
    /// h = 2^-27; max exact edge-squared is |BC|² = 2 so MAX² × 2 > MAX².
    #[test]
    fn orient3d_overflow_not_bypassed_by_fast_path() {
        let height = 2.0_f64.powi(-27);
        let local_ctx = ToleranceContext::try_new(1.0, f64::MAX, 1e-7, 1e-10).unwrap();
        let a = Point3::try_new(0.0, 0.0, 0.0).unwrap();
        let b = Point3::try_new(1.0, 0.0, 0.0).unwrap();
        let c = Point3::try_new(0.0, 1.0, 0.0).unwrap();
        let d = Point3::try_new(0.0, 0.0, height).unwrap();
        // max exact edge-squared = 2 (edge BC); MAX² × 2 > MAX² → overflow.
        match orient3d(a, b, c, d, local_ctx) {
            Err(PredicateError::Tolerance(ToleranceError::Overflow)) => {}
            other => panic!("expected Tolerance(Overflow), got {other:?}"),
        }
    }
}
