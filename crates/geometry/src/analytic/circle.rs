//! Analytic circular curves.
//!
//! # Parameterization
//!
//! **`Circle2`** and **`Circle3`** use the trigonometric parameterization:
//!
//! ```text
//! p(θ) = center + r·cos(θ)·x_axis + r·sin(θ)·y_axis
//! ```
//!
//! where `r` is the radius, `x_axis` is the stored unit reference direction,
//! and `y_axis` is its 90° CCW rotation (2-D) or `normal × x_axis` (3-D).
//!
//! The parameter domain is `[0, 2π)` with period `2π`.
//!
//! Derivatives:
//! - `p′(θ) = r·(−sin θ·x_axis + cos θ·y_axis)`
//! - `p″(θ) = −r·(cos θ·x_axis + sin θ·y_axis) = −(p(θ) − center)`
//!
//! Projection: the query point is projected onto the circle's plane (3-D) or
//! interpreted directly (2-D), then `θ = atan2(Δ·y_axis, Δ·x_axis)` mapped to
//! `[0, 2π)`.  Returns [`GeometryError::Singular`] when the in-plane
//! displacement from the center is exactly zero.

#![allow(
    clippy::many_single_char_names,
    clippy::missing_panics_doc,
    clippy::similar_names
)]

use std::f64::consts::TAU;

use amphion_foundation::{Point2, Point3, ToleranceContext, Transform3, Vector2, Vector3};
use serde::{Deserialize, Serialize};

use crate::traits::{Curve2Evaluator, Curve3Evaluator};
use crate::{
    CurveEvaluation2, CurveEvaluation3, CurveKind, CurveProjection2, CurveProjection3,
    DerivativeOrder, GeometryError, ParameterRange,
};

use super::{
    ConstructionError, TransformError,
    helpers::{
        ILL_COND_THRESH, all_finite2, all_finite3, cross3, dot3, in_range, mag3, normalize2,
        normalize3, perp2, scale3, sub3, validate_orthogonal3, validate_unit2, validate_unit3,
    },
    transform::{apply_to_point, apply_to_vector, similarity_scale},
};

fn circle_domain() -> ParameterRange {
    // (0.0, TAU, TAU) is a compile-time constant with lo < hi; this is not
    // an input-dependent path, so a static-invariant `expect` is acceptable
    // here (see CONTRACTS.md).
    ParameterRange::try_new(Some(0.0), Some(TAU), Some(TAU))
        .expect("circle domain [0, 2π) is always valid")
}

// ─── Circle2 ─────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct Circle2Repr {
    center: Point2,
    radius: f64,
    x_axis: Vector2,
}

/// A circular arc in two-dimensional parameter space.
///
/// Parameterization: `p(θ) = center + r·cos θ·x + r·sin θ·y`\
/// where `y = perp(x_axis)` (90° CCW), `r = radius`, domain `[0, 2π)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Circle2Repr", into = "Circle2Repr")]
pub struct Circle2 {
    center: Point2,
    radius: f64,
    x_axis: Vector2,
    y_axis: Vector2,
}

impl Circle2 {
    /// Constructs a circle.
    ///
    /// `x_axis` is normalized internally and defines the `θ = 0` direction.
    ///
    /// # Errors
    ///
    /// - [`ConstructionError::NonFiniteInput`] — any NaN/Inf input
    /// - [`ConstructionError::DegenerateAxis`] — zero-length `x_axis`
    /// - [`ConstructionError::NotPositive`] — `radius <= 0`
    pub fn try_new(
        center: Point2,
        radius: f64,
        x_axis: Vector2,
    ) -> Result<Self, ConstructionError> {
        let c = center.into_array();
        let x = x_axis.into_array();
        if !all_finite2(c) || !radius.is_finite() || !all_finite2(x) {
            return Err(ConstructionError::NonFiniteInput);
        }
        if radius <= 0.0 {
            return Err(ConstructionError::NotPositive);
        }
        let x_unit = normalize2(x).ok_or(ConstructionError::DegenerateAxis)?;
        let y_arr = perp2(x_unit);
        Ok(Self {
            center: Point2::try_new(c[0], c[1]).map_err(|_| ConstructionError::NonFiniteInput)?,
            radius,
            x_axis: Vector2::try_new(x_unit[0], x_unit[1])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            y_axis: Vector2::try_new(y_arr[0], y_arr[1])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
        })
    }

    /// Returns the center point.
    #[must_use]
    pub fn center(&self) -> Point2 {
        self.center
    }

    /// Returns the radius.
    #[must_use]
    pub fn radius(&self) -> f64 {
        self.radius
    }

    /// Returns the unit reference direction for `θ = 0`.
    #[must_use]
    pub fn x_axis(&self) -> Vector2 {
        self.x_axis
    }

    /// Returns the unit y-axis (90° CCW rotation of `x_axis`).
    #[must_use]
    pub fn y_axis(&self) -> Vector2 {
        self.y_axis
    }
}

impl TryFrom<Circle2Repr> for Circle2 {
    type Error = ConstructionError;
    fn try_from(repr: Circle2Repr) -> Result<Self, Self::Error> {
        let center = repr.center.into_array();
        let x_axis = repr.x_axis.into_array();
        if !all_finite2(center) || !repr.radius.is_finite() || !all_finite2(x_axis) {
            return Err(ConstructionError::NonFiniteInput);
        }
        if repr.radius <= 0.0 {
            return Err(ConstructionError::NotPositive);
        }
        validate_unit2(x_axis)?;
        let y_arr = perp2(x_axis);
        Ok(Self {
            center: repr.center,
            radius: repr.radius,
            x_axis: repr.x_axis,
            y_axis: Vector2::try_new(y_arr[0], y_arr[1])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
        })
    }
}

impl From<Circle2> for Circle2Repr {
    fn from(c: Circle2) -> Self {
        Self {
            center: c.center,
            radius: c.radius,
            x_axis: c.x_axis,
        }
    }
}

impl Curve2Evaluator for Circle2 {
    fn kind(&self) -> CurveKind {
        CurveKind::Circle
    }

    fn domain(&self) -> ParameterRange {
        circle_domain()
    }

    fn evaluate(
        &self,
        parameter: f64,
        _order: DerivativeOrder,
        _tolerance: &ToleranceContext,
    ) -> Result<CurveEvaluation2, GeometryError> {
        if !parameter.is_finite() {
            return Err(GeometryError::NonFiniteParameter);
        }
        if !in_range(parameter, self.domain()) {
            return Err(GeometryError::OutsideDomain);
        }
        // p(θ) = center + r·cos(θ)·x_axis + r·sin(θ)·y_axis requires `cos`
        // and `sin`. No pure-Rust, WASM-compatible, formally-proved
        // correctly-rounded implementation of these functions currently
        // exists (see the `analytic::helpers` module docs for the survey of
        // candidates), so no certified error bound can be produced.
        Err(GeometryError::Uncertified {
            reason: "circle evaluation requires certified sin/cos; no formally-proved \
                     WASM-compatible implementation is available. libm (MIT, WASM) gives \
                     ~1-2 ULP empirically but is not formally proved. core-math (MIT, 0.5 ULP) \
                     requires C FFI incompatible with WASM. IEEE 754-2019 §9.2 recommends but \
                     does not require correctly-rounded transcendentals."
                .to_owned(),
        })
    }

    fn project_into(
        &self,
        _point: Point2,
        _tolerance: &ToleranceContext,
        output: &mut Vec<CurveProjection2>,
    ) -> Result<(), GeometryError> {
        output.clear();
        // θ = atan2(Δ·y_axis, Δ·x_axis) is an uncertified std transcendental;
        // the sin(θ)/cos(θ) reconstruction of the projected point is also
        // uncertified. See the `analytic::helpers` module docs.
        Err(GeometryError::Uncertified {
            reason: "circle projection requires certified atan2/sin/cos; pending certified \
                     trig integration. See: libm crate (empirical accuracy only), core-math \
                     (0.5 ULP, not WASM-compatible)."
                .to_owned(),
        })
    }
}

// ─── Circle3 ─────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct Circle3Repr {
    center: Point3,
    radius: f64,
    normal: Vector3,
    x_axis: Vector3,
}

/// A circular arc in three-dimensional model space.
///
/// Parameterization:
/// ```text
/// p(θ) = center + r·cos θ·x_axis + r·sin θ·y_axis
/// ```
/// where `y_axis = normal × x_axis`, `r = radius`, domain `[0, 2π)`.
/// `normal`, `x_axis`, and `y_axis` form a right-handed orthonormal frame.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Circle3Repr", into = "Circle3Repr")]
pub struct Circle3 {
    center: Point3,
    radius: f64,
    normal: Vector3,
    x_axis: Vector3,
    y_axis: Vector3,
}

impl Circle3 {
    /// Constructs a circle.
    ///
    /// `normal` and `x_axis` are normalized internally.  `x_axis` is then
    /// orthogonalized against `normal` (Gram-Schmidt), so callers need only
    /// supply an approximately perpendicular vector.
    ///
    /// # Errors
    ///
    /// - [`ConstructionError::NonFiniteInput`] — any NaN/Inf input
    /// - [`ConstructionError::DegenerateAxis`] — zero-length `normal` or `x_axis`
    /// - [`ConstructionError::NotPositive`] — `radius <= 0`
    /// - [`ConstructionError::DependentAxes`] — `x_axis` parallel to `normal`
    /// - [`ConstructionError::IllConditionedAxes`] — `x_axis` nearly parallel
    ///   to `normal`
    pub fn try_new(
        center: Point3,
        radius: f64,
        normal: Vector3,
        x_axis: Vector3,
    ) -> Result<Self, ConstructionError> {
        let c = center.into_array();
        let n = normal.into_array();
        let x = x_axis.into_array();
        if !all_finite3(c) || !radius.is_finite() || !all_finite3(n) || !all_finite3(x) {
            return Err(ConstructionError::NonFiniteInput);
        }
        if radius <= 0.0 {
            return Err(ConstructionError::NotPositive);
        }
        let n_unit = normalize3(n).ok_or(ConstructionError::DegenerateAxis)?;
        let x_norm = normalize3(x).ok_or(ConstructionError::DegenerateAxis)?;
        // Orthogonalize x against n.
        let dot_xn = dot3(x_norm, n_unit);
        let x_perp = sub3(x_norm, scale3(n_unit, dot_xn));
        let perp_mag = mag3(x_perp);
        if perp_mag == 0.0 {
            return Err(ConstructionError::DependentAxes);
        }
        if perp_mag < ILL_COND_THRESH {
            return Err(ConstructionError::IllConditionedAxes);
        }
        let x_unit = normalize3(x_perp).ok_or(ConstructionError::DependentAxes)?;
        let y_arr = cross3(n_unit, x_unit);
        Ok(Self {
            center: Point3::try_new(c[0], c[1], c[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            radius,
            normal: Vector3::try_new(n_unit[0], n_unit[1], n_unit[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            x_axis: Vector3::try_new(x_unit[0], x_unit[1], x_unit[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            y_axis: Vector3::try_new(y_arr[0], y_arr[1], y_arr[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
        })
    }

    /// Returns the center point.
    #[must_use]
    pub fn center(&self) -> Point3 {
        self.center
    }

    /// Returns the radius.
    #[must_use]
    pub fn radius(&self) -> f64 {
        self.radius
    }

    /// Returns the unit normal.
    #[must_use]
    pub fn normal(&self) -> Vector3 {
        self.normal
    }

    /// Returns the unit reference direction for `θ = 0`.
    #[must_use]
    pub fn x_axis(&self) -> Vector3 {
        self.x_axis
    }

    /// Returns the unit y-axis: `normal × x_axis`.
    #[must_use]
    pub fn y_axis(&self) -> Vector3 {
        self.y_axis
    }

    /// Applies a similarity `transform` (rigid motion plus uniform scale, no
    /// reflection) to this circle, returning a new circle whose radius is
    /// scaled accordingly.
    ///
    /// A general affine transform does not map a circle to a circle, so
    /// only similarity transforms are accepted; see the `transform` module
    /// documentation for the (provisional, heuristic) similarity test.
    ///
    /// # Errors
    ///
    /// - [`TransformError::NotSimilarity`] — the transform's linear part is
    ///   not (within tolerance) a uniform-scale rotation
    /// - [`TransformError::NonFiniteResult`] — the transformed center or
    ///   axes contain a non-finite component
    /// - [`TransformError::DegenerateResult`] — the transformed axes or
    ///   scaled radius fail circle construction
    pub fn try_transform(&self, transform: &Transform3) -> Result<Self, TransformError> {
        let m = transform.into_row_major();
        let scale = similarity_scale(m).ok_or(TransformError::NotSimilarity)?;
        let c =
            apply_to_point(m, self.center.into_array()).ok_or(TransformError::NonFiniteResult)?;
        let n =
            apply_to_vector(m, self.normal.into_array()).ok_or(TransformError::NonFiniteResult)?;
        let x =
            apply_to_vector(m, self.x_axis.into_array()).ok_or(TransformError::NonFiniteResult)?;
        let new_radius = self.radius * scale;
        Self::try_new(
            Point3::try_new(c[0], c[1], c[2]).map_err(|_| TransformError::NonFiniteResult)?,
            new_radius,
            Vector3::try_new(n[0], n[1], n[2]).map_err(|_| TransformError::NonFiniteResult)?,
            Vector3::try_new(x[0], x[1], x[2]).map_err(|_| TransformError::NonFiniteResult)?,
        )
        .map_err(|_| TransformError::DegenerateResult)
    }
}

impl TryFrom<Circle3Repr> for Circle3 {
    type Error = ConstructionError;
    fn try_from(repr: Circle3Repr) -> Result<Self, Self::Error> {
        let center = repr.center.into_array();
        let normal = repr.normal.into_array();
        let x_axis = repr.x_axis.into_array();
        if !all_finite3(center)
            || !repr.radius.is_finite()
            || !all_finite3(normal)
            || !all_finite3(x_axis)
        {
            return Err(ConstructionError::NonFiniteInput);
        }
        if repr.radius <= 0.0 {
            return Err(ConstructionError::NotPositive);
        }
        validate_unit3(normal)?;
        validate_unit3(x_axis)?;
        validate_orthogonal3(normal, x_axis)?;
        let y_arr = cross3(normal, x_axis);
        Ok(Self {
            center: repr.center,
            radius: repr.radius,
            normal: repr.normal,
            x_axis: repr.x_axis,
            y_axis: Vector3::try_new(y_arr[0], y_arr[1], y_arr[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
        })
    }
}

impl From<Circle3> for Circle3Repr {
    fn from(c: Circle3) -> Self {
        Self {
            center: c.center,
            radius: c.radius,
            normal: c.normal,
            x_axis: c.x_axis,
        }
    }
}

impl Curve3Evaluator for Circle3 {
    fn kind(&self) -> CurveKind {
        CurveKind::Circle
    }

    fn domain(&self) -> ParameterRange {
        circle_domain()
    }

    fn evaluate(
        &self,
        parameter: f64,
        _order: DerivativeOrder,
        _tolerance: &ToleranceContext,
    ) -> Result<CurveEvaluation3, GeometryError> {
        if !parameter.is_finite() {
            return Err(GeometryError::NonFiniteParameter);
        }
        if !in_range(parameter, self.domain()) {
            return Err(GeometryError::OutsideDomain);
        }
        // p(θ) = center + r·cos(θ)·x_axis + r·sin(θ)·y_axis requires `cos`
        // and `sin`. No pure-Rust, WASM-compatible, formally-proved
        // correctly-rounded implementation of these functions currently
        // exists (see the `analytic::helpers` module docs for the survey of
        // candidates), so no certified error bound can be produced.
        Err(GeometryError::Uncertified {
            reason: "circle evaluation requires certified sin/cos; no formally-proved \
                     WASM-compatible implementation is available. libm (MIT, WASM) gives \
                     ~1-2 ULP empirically but is not formally proved. core-math (MIT, 0.5 ULP) \
                     requires C FFI incompatible with WASM. IEEE 754-2019 §9.2 recommends but \
                     does not require correctly-rounded transcendentals."
                .to_owned(),
        })
    }

    fn project_into(
        &self,
        _point: Point3,
        _tolerance: &ToleranceContext,
        output: &mut Vec<CurveProjection3>,
    ) -> Result<(), GeometryError> {
        output.clear();
        // θ = atan2(...) is an uncertified std transcendental; the
        // sin(θ)/cos(θ) reconstruction of the projected point is also
        // uncertified. See the `analytic::helpers` module docs.
        Err(GeometryError::Uncertified {
            reason: "circle projection requires certified atan2/sin/cos; pending certified \
                     trig integration. See: libm crate (empirical accuracy only), core-math \
                     (0.5 ULP, not WASM-compatible)."
                .to_owned(),
        })
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, TAU};

    use amphion_foundation::{Point2, Point3, ToleranceContext, Vector2, Vector3};
    use serde_json::json;

    use crate::analytic::helpers::ILL_COND_THRESH;
    use crate::traits::{Curve2Evaluator, Curve3Evaluator};
    use crate::{DerivativeOrder, GeometryError};

    use super::{Circle2, Circle2Repr, Circle3, Circle3Repr, ConstructionError};

    fn tol() -> ToleranceContext {
        ToleranceContext::try_new(1e-9, 1e-8, 1e-10, 1e-12).unwrap()
    }

    fn dist2(a: Point2, b: Point2) -> f64 {
        let [ax, ay] = a.into_array();
        let [bx, by] = b.into_array();
        (ax - bx).hypot(ay - by)
    }

    fn assert_uncertified(err: &GeometryError) {
        match err {
            GeometryError::Uncertified { reason } => {
                assert!(!reason.is_empty(), "reason string must not be empty");
            }
            other => panic!("expected Uncertified, got {other:?}"),
        }
    }

    // ── Circle2 ──────────────────────────────────────────────────────────────

    #[test]
    fn circle2_construction_valid() {
        let c = Circle2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            1.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        );
        assert!(c.is_ok());
    }

    #[test]
    fn circle2_construction_rejects_zero_radius() {
        let err = Circle2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            0.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::NotPositive);
    }

    #[test]
    fn circle2_construction_rejects_negative_radius() {
        let err = Circle2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            -1.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::NotPositive);
    }

    #[test]
    fn circle2_construction_rejects_zero_axis() {
        let err = Circle2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            1.0,
            Vector2::try_new(0.0, 0.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::DegenerateAxis);
    }

    #[test]
    fn circle2_evaluate_returns_uncertified_pending_trig() {
        // No pure-Rust, WASM-compatible, formally-proved correctly-rounded
        // sin/cos implementation exists; evaluate() must be honest about
        // this rather than assert an unproven bound.
        let c = Circle2::try_new(
            Point2::try_new(1.0, 2.0).unwrap(),
            3.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        let err = c
            .evaluate(0.0, DerivativeOrder::Position, &tol())
            .unwrap_err();
        assert_uncertified(&err);
        let err = c
            .evaluate(FRAC_PI_2, DerivativeOrder::Second, &tol())
            .unwrap_err();
        assert_uncertified(&err);
    }

    #[test]
    fn circle2_evaluate_rejects_out_of_domain() {
        let c = Circle2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            1.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        assert_eq!(
            c.evaluate(-0.001, DerivativeOrder::Position, &tol()),
            Err(GeometryError::OutsideDomain)
        );
        assert_eq!(
            c.evaluate(TAU, DerivativeOrder::Position, &tol()),
            Err(GeometryError::OutsideDomain)
        );
    }

    #[test]
    fn circle2_evaluate_rejects_non_finite_before_uncertified() {
        let c = Circle2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            1.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        assert_eq!(
            c.evaluate(f64::NAN, DerivativeOrder::Position, &tol()),
            Err(GeometryError::NonFiniteParameter)
        );
    }

    #[test]
    fn circle2_project_returns_uncertified_pending_trig() {
        // θ = atan2(...) and its sin/cos reconstruction are uncertified std
        // transcendentals; project_into must report Uncertified rather than
        // a bound it cannot support.
        let c = Circle2::try_new(
            Point2::try_new(2.0, 3.0).unwrap(),
            4.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        let q = Point2::try_new(10.0, 3.0).unwrap();
        let err = c.project(q, &tol()).unwrap_err();
        assert_uncertified(&err);
    }

    #[test]
    fn circle2_project_into_clears_output_on_error() {
        let c = Circle2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            1.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        let mut output = vec![];
        let err = c.project_into(Point2::try_new(0.5, 0.0).unwrap(), &tol(), &mut output);
        assert_uncertified(&err.unwrap_err());
        assert!(output.is_empty());
    }

    #[test]
    fn circle2_serde_round_trip() {
        let c = Circle2::try_new(
            Point2::try_new(1.0, -2.0).unwrap(),
            3.5,
            Vector2::try_new(1.0, 1.0).unwrap(),
        )
        .unwrap();
        let json = serde_json::to_string(&c).unwrap();
        let decoded: Circle2 = serde_json::from_str(&json).unwrap();
        assert_eq!(c, decoded);
    }

    #[test]
    fn circle2_distance_bound_large_radius_is_valid_upper_bound_or_uncertified() {
        // Concrete counterexample from review: Circle2(radius=2^53), query=(1,1),
        // tol.abs=1. The old local-scale (`|query - center|`) formula certified a
        // bound (≈9007199254740990.0) that was numerically *below* the true
        // Euclidean distance (≈9007199254740990.586), violating the DistanceBound
        // contract. Circle projection is now unconditionally `Uncertified`
        // (pending certified trig), so this must always be reported as
        // Uncertified — and if any bound is ever returned in the future, it
        // must be a genuine upper bound.
        let tol_1m = ToleranceContext::try_new(1.0, 0.0, 1e-10, 1e-12).unwrap();
        let radius = f64::powi(2.0, 53); // 2^53
        let c = Circle2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            radius,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        let q = Point2::try_new(1.0, 1.0).unwrap();
        let result = c.project(q, &tol_1m);
        match result {
            Err(GeometryError::Uncertified { .. }) => {
                // Correctly identified as uncertifiable at this scale/tolerance.
            }
            Ok(projs) => {
                // If a bound is returned, it MUST be a valid upper bound.
                for p in &projs {
                    let actual = dist2(q, p.point);
                    assert!(
                        actual <= p.distance_bound.get(),
                        "distance_bound={} < actual_distance={actual}",
                        p.distance_bound.get()
                    );
                }
            }
            Err(other) => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn circle2_serde_rejects_bad_radius_or_axis() {
        let bad_axis: Circle2Repr = serde_json::from_value(json!({
            "center": [1.0, 2.0],
            "radius": 3.0,
            "x_axis": [2.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Circle2::try_from(bad_axis),
            Err(ConstructionError::DegenerateAxis)
        );

        let bad_radius: Circle2Repr = serde_json::from_value(json!({
            "center": [1.0, 2.0],
            "radius": 0.0,
            "x_axis": [1.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Circle2::try_from(bad_radius),
            Err(ConstructionError::NotPositive)
        );
    }

    #[test]
    fn circle2_serde_rejects_nan_and_inf_fields() {
        assert!(
            serde_json::from_str::<Circle2>(
                r#"{"center":[NaN,0.0],"radius":1.0,"x_axis":[1.0,0.0]}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<Circle2>(
                r#"{"center":[0.0,0.0],"radius":1.0,"x_axis":[Infinity,0.0]}"#
            )
            .is_err()
        );
    }

    // ── Circle3 ──────────────────────────────────────────────────────────────

    #[test]
    fn circle3_construction_valid() {
        let c = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            1.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        );
        assert!(c.is_ok());
    }

    #[test]
    fn circle3_construction_rejects_parallel_axes() {
        // x_axis parallel to normal → DependentAxes
        let err = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            1.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::DependentAxes);
    }

    #[test]
    fn circle3_construction_rejects_ill_conditioned_axes() {
        let err = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            1.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(ILL_COND_THRESH / 2.0, 0.0, 1.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::IllConditionedAxes);
    }

    #[test]
    fn circle3_construction_rejects_zero_radius() {
        let err = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            0.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::NotPositive);
    }

    #[test]
    fn circle3_y_axis_is_right_handed() {
        // With normal = +Z and x_axis = +X, y_axis = Z × X = (0,0,1)×(1,0,0) = (0,1,0)
        let c = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            1.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let y = c.y_axis().into_array();
        assert!((y[0]).abs() < 1e-14 && (y[1] - 1.0).abs() < 1e-14 && y[2].abs() < 1e-14);
    }

    #[test]
    fn circle3_evaluate_returns_uncertified_pending_trig() {
        let c = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 1.0).unwrap(),
            2.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let err = c
            .evaluate(0.0, DerivativeOrder::Position, &tol())
            .unwrap_err();
        assert_uncertified(&err);
        let err = c
            .evaluate(FRAC_PI_2, DerivativeOrder::Second, &tol())
            .unwrap_err();
        assert_uncertified(&err);
    }

    #[test]
    fn circle3_evaluate_rejects_out_of_domain() {
        let c = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            1.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        assert_eq!(
            c.evaluate(-0.001, DerivativeOrder::Position, &tol()),
            Err(GeometryError::OutsideDomain)
        );
        assert_eq!(
            c.evaluate(TAU, DerivativeOrder::Position, &tol()),
            Err(GeometryError::OutsideDomain)
        );
    }

    #[test]
    fn circle3_project_returns_uncertified_pending_trig() {
        let c = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            5.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let q = Point3::try_new(1.0, 0.0, 5.0).unwrap();
        let err = c.project(q, &tol()).unwrap_err();
        assert_uncertified(&err);
    }

    #[test]
    fn circle3_project_into_clears_output_on_error() {
        let c = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            1.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let mut output = vec![];
        let err = c.project_into(Point3::try_new(1.0, 0.0, 0.0).unwrap(), &tol(), &mut output);
        assert_uncertified(&err.unwrap_err());
        assert!(output.is_empty());
    }

    #[test]
    fn circle3_serde_round_trip() {
        let c = Circle3::try_new(
            Point3::try_new(1.0, 2.0, 3.0).unwrap(),
            4.0,
            Vector3::try_new(
                1.0 / 3.0_f64.sqrt(),
                1.0 / 3.0_f64.sqrt(),
                1.0 / 3.0_f64.sqrt(),
            )
            .unwrap(),
            Vector3::try_new(1.0 / 2.0_f64.sqrt(), -1.0 / 2.0_f64.sqrt(), 0.0).unwrap(),
        )
        .unwrap();
        let json = serde_json::to_string(&c).unwrap();
        let decoded: Circle3 = serde_json::from_str(&json).unwrap();
        assert_eq!(c, decoded);
    }

    #[test]
    fn circle3_serde_rejects_bad_axis_radius_and_orthogonality() {
        let bad_axis: Circle3Repr = serde_json::from_value(json!({
            "center": [1.0, 2.0, 3.0],
            "radius": 4.0,
            "normal": [2.0, 0.0, 0.0],
            "x_axis": [0.0, 1.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Circle3::try_from(bad_axis),
            Err(ConstructionError::DegenerateAxis)
        );

        let bad_frame: Circle3Repr = serde_json::from_value(json!({
            "center": [1.0, 2.0, 3.0],
            "radius": 4.0,
            "normal": [1.0, 0.0, 0.0],
            "x_axis": [1.0, 0.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Circle3::try_from(bad_frame),
            Err(ConstructionError::DependentAxes)
        );

        let bad_radius: Circle3Repr = serde_json::from_value(json!({
            "center": [1.0, 2.0, 3.0],
            "radius": 0.0,
            "normal": [0.0, 0.0, 1.0],
            "x_axis": [1.0, 0.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Circle3::try_from(bad_radius),
            Err(ConstructionError::NotPositive)
        );
    }

    #[test]
    fn circle3_serde_rejects_nan_and_inf_fields() {
        assert!(serde_json::from_str::<Circle3>(
            r#"{"center":[NaN,0.0,0.0],"radius":1.0,"normal":[0.0,0.0,1.0],"x_axis":[1.0,0.0,0.0]}"#
        )
        .is_err());
        assert!(serde_json::from_str::<Circle3>(
            r#"{"center":[0.0,0.0,0.0],"radius":1.0,"normal":[Infinity,0.0,1.0],"x_axis":[1.0,0.0,0.0]}"#
        )
        .is_err());
    }

    #[test]
    fn circle3_try_transform_identity_is_noop() {
        let c = Circle3::try_new(
            Point3::try_new(1.0, 2.0, 3.0).unwrap(),
            2.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let out = c
            .try_transform(&amphion_foundation::Transform3::IDENTITY)
            .unwrap();
        assert_eq!(out, c);
    }

    #[test]
    fn circle3_try_transform_similarity_scales_radius() {
        // Rotation by 90° about Z, uniform scale 2, plus translation.
        let m = [
            0.0, -2.0, 0.0, 5.0, //
            2.0, 0.0, 0.0, -3.0, //
            0.0, 0.0, 2.0, 7.0,
        ];
        let t = amphion_foundation::Transform3::try_from_row_major(m).unwrap();
        let c = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            3.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let out = c.try_transform(&t).unwrap();
        assert!((out.radius() - 6.0).abs() < 1e-9);
        let [cx, cy, cz] = out.center().into_array();
        assert!((cx - 5.0).abs() < 1e-9);
        assert!((cy - (-3.0)).abs() < 1e-9);
        assert!((cz - 7.0).abs() < 1e-9);
    }

    #[test]
    fn circle3_try_transform_rejects_non_similarity() {
        // Non-uniform scale (shear-free but anisotropic) is not a
        // similarity: it distorts a circle into an ellipse.
        let m = [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 2.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0,
        ];
        let t = amphion_foundation::Transform3::try_from_row_major(m).unwrap();
        let c = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            1.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        assert_eq!(
            c.try_transform(&t),
            Err(super::TransformError::NotSimilarity)
        );
    }
}
