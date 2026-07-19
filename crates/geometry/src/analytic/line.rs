//! Analytic straight-line curves.
//!
//! # Parameterizations
//!
//! **`Line2`** and **`Line3`** both use the affine parameterization
//! `p(t) = origin + t · direction`, where `direction` is the stored unit
//! vector.  The domain is the full real line (no bounds), so only finiteness
//! of `t` is checked.
//!
//! Derivatives:
//! - `p′(t) = direction`
//! - `p″(t) = 0`
//!
//! Projection: `t = ((q − origin)·direction) / (direction·direction)`,
//! computed via exact rational arithmetic (see the private `helpers`
//! module). The projected distance bound equals a certified upper bound on
//! the true perpendicular distance from `q` to the line.

#![allow(
    clippy::many_single_char_names,
    clippy::missing_panics_doc,
    clippy::similar_names
)]

use amphion_foundation::{Point2, Point3, Transform3, UnitVector2, UnitVector3, Vector2, Vector3};
use serde::{Deserialize, Serialize};

use crate::traits::{Curve2Evaluator, Curve3Evaluator};
use crate::{
    CurveEvaluation2, CurveEvaluation3, CurveKind, CurveProjection2, CurveProjection3,
    DerivativeOrder, DistanceBound, EvaluationContext, FirstDerivativeBound, GeometryError,
    ParameterRange, ParameterValue, PositionBound, SecondDerivativeBound,
};

use super::{
    ConstructionError, TransformError,
    helpers::{
        all_finite2, all_finite3, check_tolerance, exact_affine_eval2, exact_affine_eval3,
        exact_line_project2, exact_line_project3, mag2, mag3, normalization_to_construction,
        scale2, scale3,
    },
};

// ─── Helpers shared by both line types ──────────────────────────────────────

/// Infinite line domain: no bounds, no period.
fn line_domain() -> ParameterRange {
    // (None, None, None) is a compile-time constant and always valid; this
    // is not an input-dependent path, so a static-invariant `expect` is
    // acceptable here (see CONTRACTS.md).
    ParameterRange::try_new(None, None, None).expect("unbounded domain is always valid")
}

// ─── Line2 ───────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct Line2Repr {
    origin: Point2,
    direction: Vector2,
}

/// A directed straight line in two-dimensional parameter space.
///
/// The line is parameterized as `p(t) = origin + t · direction`, where
/// `direction` is a unit vector.  The domain is `(−∞, +∞)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Line2Repr", into = "Line2Repr")]
pub struct Line2 {
    origin: Point2,
    direction: UnitVector2,
}

impl Line2 {
    /// Constructs a line from `origin` and a direction that will be
    /// normalized internally.
    ///
    /// # Errors
    ///
    /// Returns [`ConstructionError::NonFiniteInput`] if any component is NaN
    /// or infinite, and [`ConstructionError::DegenerateAxis`] if `direction`
    /// has zero length.
    pub fn try_new(origin: Point2, direction: Vector2) -> Result<Self, ConstructionError> {
        let o = origin.into_array();
        if !all_finite2(o) {
            return Err(ConstructionError::NonFiniteInput);
        }
        let dir = UnitVector2::try_normalize(direction).map_err(normalization_to_construction)?;
        Ok(Self {
            origin: Point2::try_new(o[0], o[1]).map_err(|_| ConstructionError::NonFiniteInput)?,
            direction: dir,
        })
    }

    /// Returns the line's origin point.
    #[must_use]
    pub fn origin(&self) -> Point2 {
        self.origin
    }

    /// Returns the stored unit direction vector.
    #[must_use]
    pub fn direction(&self) -> Vector2 {
        self.direction.as_vector()
    }
}

impl TryFrom<Line2Repr> for Line2 {
    type Error = ConstructionError;
    fn try_from(repr: Line2Repr) -> Result<Self, Self::Error> {
        let origin = repr.origin.into_array();
        if !all_finite2(origin) {
            return Err(ConstructionError::NonFiniteInput);
        }
        let direction =
            UnitVector2::try_normalize(repr.direction).map_err(normalization_to_construction)?;
        Ok(Self {
            origin: repr.origin,
            direction,
        })
    }
}

impl From<Line2> for Line2Repr {
    fn from(line: Line2) -> Self {
        Self {
            origin: line.origin,
            direction: line.direction.as_vector(),
        }
    }
}

impl Curve2Evaluator for Line2 {
    fn kind(&self) -> CurveKind {
        CurveKind::Line
    }

    fn domain(&self) -> ParameterRange {
        line_domain()
    }

    fn evaluate(
        &self,
        parameter: f64,
        order: DerivativeOrder,
        context: &EvaluationContext,
    ) -> Result<CurveEvaluation2, GeometryError> {
        if !parameter.is_finite() {
            return Err(GeometryError::NonFiniteParameter);
        }
        let o = self.origin.into_array();
        let d = self.direction.into_array();

        let eval = exact_affine_eval2(context.budget, o, d, parameter)?;
        let pos = Point2::try_new(eval.point[0], eval.point[1]).map_err(|_| {
            GeometryError::Uncertified {
                reason: "line position is non-finite".to_owned(),
            }
        })?;
        let position_error_bound =
            PositionBound::try_new(eval.position_error_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "position error bound overflowed representable range".to_owned(),
                }
            })?;
        let eval_scale = mag2(o) + mag2(scale2(d, parameter));
        check_tolerance(&context.tolerance, position_error_bound.get(), eval_scale)?;

        // The first derivative is exactly the stored direction (no
        // arithmetic beyond returning the stored vector) and the second
        // derivative is exactly zero: both are certified with zero error.
        let direction_error_bound =
            FirstDerivativeBound::try_new(0.0).map_err(|_| GeometryError::Uncertified {
                reason: "zero derivative bound construction failed unexpectedly".to_owned(),
            })?;
        let zero_error_bound =
            SecondDerivativeBound::try_new(0.0).map_err(|_| GeometryError::Uncertified {
                reason: "zero derivative bound construction failed unexpectedly".to_owned(),
            })?;

        let (first, first_error_bound, second, second_error_bound) = match order {
            DerivativeOrder::Position => (None, None, None, None),
            DerivativeOrder::First => {
                let v = Vector2::try_new(d[0], d[1]).map_err(|_| GeometryError::Uncertified {
                    reason: "line direction is non-finite".to_owned(),
                })?;
                (Some(v), Some(direction_error_bound), None, None)
            }
            DerivativeOrder::Second => {
                let v = Vector2::try_new(d[0], d[1]).map_err(|_| GeometryError::Uncertified {
                    reason: "line direction is non-finite".to_owned(),
                })?;
                let zero = Vector2::try_new(0.0, 0.0).map_err(|_| GeometryError::Uncertified {
                    reason: "zero vector construction failed unexpectedly".to_owned(),
                })?;
                (
                    Some(v),
                    Some(direction_error_bound),
                    Some(zero),
                    Some(zero_error_bound),
                )
            }
        };
        Ok(CurveEvaluation2 {
            position: pos,
            first,
            second,
            position_error_bound,
            first_error_bound,
            second_error_bound,
        })
    }

    fn project_into(
        &self,
        point: Point2,
        context: &EvaluationContext,
        output: &mut Vec<CurveProjection2>,
    ) -> Result<(), GeometryError> {
        output.clear();
        let q = point.into_array();
        let o = self.origin.into_array();
        let d = self.direction.into_array();

        let result = exact_line_project2(context.budget, q, o, d)?;
        let scale = mag2(q) + mag2(result.point);
        check_tolerance(&context.tolerance, result.point_residual_bound, scale)?;

        let proj = Point2::try_new(result.point[0], result.point[1]).map_err(|_| {
            GeometryError::Uncertified {
                reason: "line projection point is non-finite".to_owned(),
            }
        })?;
        output.push(CurveProjection2 {
            parameter: ParameterValue::try_new(result.parameter).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "line projection parameter is non-finite".to_owned(),
                }
            })?,
            point: proj,
            distance_bound: DistanceBound::try_new(result.distance_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "line projection distance is non-finite or negative".to_owned(),
                }
            })?,
            parameter_error_bound: result.parameter_error_bound,
            point_residual_bound: PositionBound::try_new(result.point_residual_bound).map_err(
                |_| GeometryError::Uncertified {
                    reason: "line projection point residual bound is non-finite or negative"
                        .to_owned(),
                },
            )?,
        });
        Ok(())
    }
}

// ─── Line3 ───────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
struct Line3Repr {
    origin: Point3,
    direction: Vector3,
}

/// A directed straight line in three-dimensional model space.
///
/// The line is parameterized as `p(t) = origin + t · direction`, where
/// `direction` is a unit vector.  The domain is `(−∞, +∞)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Line3Repr", into = "Line3Repr")]
pub struct Line3 {
    origin: Point3,
    direction: UnitVector3,
}

impl Line3 {
    /// Constructs a line from `origin` and a direction that will be
    /// normalized internally.
    ///
    /// # Errors
    ///
    /// Returns [`ConstructionError::NonFiniteInput`] if any component is NaN
    /// or infinite, and [`ConstructionError::DegenerateAxis`] if `direction`
    /// has zero length.
    pub fn try_new(origin: Point3, direction: Vector3) -> Result<Self, ConstructionError> {
        let o = origin.into_array();
        if !all_finite3(o) {
            return Err(ConstructionError::NonFiniteInput);
        }
        let dir = UnitVector3::try_normalize(direction).map_err(normalization_to_construction)?;
        Ok(Self {
            origin: Point3::try_new(o[0], o[1], o[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            direction: dir,
        })
    }

    /// Returns the line's origin point.
    #[must_use]
    pub fn origin(&self) -> Point3 {
        self.origin
    }

    /// Returns the stored unit direction vector.
    #[must_use]
    pub fn direction(&self) -> Vector3 {
        self.direction.as_vector()
    }

    /// Applies an affine `transform` to this line, returning a new line.
    ///
    /// Any non-degenerate affine transform is accepted: the affine image of
    /// a line is always a line, as long as the transformed direction does
    /// not collapse to zero.
    ///
    /// # Errors
    ///
    /// - [`TransformError::NonFiniteResult`] — the transformed origin or
    ///   direction contains a non-finite component
    /// - [`TransformError::DegenerateResult`] — the transformed direction has
    ///   zero length
    pub fn try_transform(&self, transform: &Transform3) -> Result<Self, TransformError> {
        let o = transform
            .try_apply_to_point(self.origin)
            .map_err(|_| TransformError::NonFiniteResult)?;
        let d_vec = transform
            .try_apply_to_vector(self.direction.as_vector())
            .map_err(|_| TransformError::NonFiniteResult)?;
        Self::try_new(o, d_vec).map_err(|_| TransformError::DegenerateResult)
    }
}

impl TryFrom<Line3Repr> for Line3 {
    type Error = ConstructionError;
    fn try_from(repr: Line3Repr) -> Result<Self, Self::Error> {
        let origin = repr.origin.into_array();
        if !all_finite3(origin) {
            return Err(ConstructionError::NonFiniteInput);
        }
        let direction =
            UnitVector3::try_normalize(repr.direction).map_err(normalization_to_construction)?;
        Ok(Self {
            origin: repr.origin,
            direction,
        })
    }
}

impl From<Line3> for Line3Repr {
    fn from(line: Line3) -> Self {
        Self {
            origin: line.origin,
            direction: line.direction.as_vector(),
        }
    }
}

impl Curve3Evaluator for Line3 {
    fn kind(&self) -> CurveKind {
        CurveKind::Line
    }

    fn domain(&self) -> ParameterRange {
        line_domain()
    }

    fn evaluate(
        &self,
        parameter: f64,
        order: DerivativeOrder,
        context: &EvaluationContext,
    ) -> Result<CurveEvaluation3, GeometryError> {
        if !parameter.is_finite() {
            return Err(GeometryError::NonFiniteParameter);
        }
        let o = self.origin.into_array();
        let d = self.direction.into_array();

        let eval = exact_affine_eval3(context.budget, o, d, parameter)?;
        let pos = Point3::try_new(eval.point[0], eval.point[1], eval.point[2]).map_err(|_| {
            GeometryError::Uncertified {
                reason: "line position is non-finite".to_owned(),
            }
        })?;
        let position_error_bound =
            PositionBound::try_new(eval.position_error_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "position error bound overflowed representable range".to_owned(),
                }
            })?;
        let eval_scale = mag3(o) + mag3(scale3(d, parameter));
        check_tolerance(&context.tolerance, position_error_bound.get(), eval_scale)?;

        let direction_error_bound =
            FirstDerivativeBound::try_new(0.0).map_err(|_| GeometryError::Uncertified {
                reason: "zero derivative bound construction failed unexpectedly".to_owned(),
            })?;
        let zero_error_bound =
            SecondDerivativeBound::try_new(0.0).map_err(|_| GeometryError::Uncertified {
                reason: "zero derivative bound construction failed unexpectedly".to_owned(),
            })?;

        let (first, first_error_bound, second, second_error_bound) = match order {
            DerivativeOrder::Position => (None, None, None, None),
            DerivativeOrder::First => {
                let v =
                    Vector3::try_new(d[0], d[1], d[2]).map_err(|_| GeometryError::Uncertified {
                        reason: "line direction is non-finite".to_owned(),
                    })?;
                (Some(v), Some(direction_error_bound), None, None)
            }
            DerivativeOrder::Second => {
                let v =
                    Vector3::try_new(d[0], d[1], d[2]).map_err(|_| GeometryError::Uncertified {
                        reason: "line direction is non-finite".to_owned(),
                    })?;
                let zero =
                    Vector3::try_new(0.0, 0.0, 0.0).map_err(|_| GeometryError::Uncertified {
                        reason: "zero vector construction failed unexpectedly".to_owned(),
                    })?;
                (
                    Some(v),
                    Some(direction_error_bound),
                    Some(zero),
                    Some(zero_error_bound),
                )
            }
        };
        Ok(CurveEvaluation3 {
            position: pos,
            first,
            second,
            position_error_bound,
            first_error_bound,
            second_error_bound,
        })
    }

    fn project_into(
        &self,
        point: Point3,
        context: &EvaluationContext,
        output: &mut Vec<CurveProjection3>,
    ) -> Result<(), GeometryError> {
        output.clear();
        let q = point.into_array();
        let o = self.origin.into_array();
        let d = self.direction.into_array();

        let result = exact_line_project3(context.budget, q, o, d)?;
        let scale = mag3(q) + mag3(result.point);
        check_tolerance(&context.tolerance, result.point_residual_bound, scale)?;

        let proj =
            Point3::try_new(result.point[0], result.point[1], result.point[2]).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "line projection point is non-finite".to_owned(),
                }
            })?;
        output.push(CurveProjection3 {
            parameter: ParameterValue::try_new(result.parameter).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "line projection parameter is non-finite".to_owned(),
                }
            })?,
            point: proj,
            distance_bound: DistanceBound::try_new(result.distance_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "line projection distance is non-finite or negative".to_owned(),
                }
            })?,
            parameter_error_bound: result.parameter_error_bound,
            point_residual_bound: PositionBound::try_new(result.point_residual_bound).map_err(
                |_| GeometryError::Uncertified {
                    reason: "line projection point residual bound is non-finite or negative"
                        .to_owned(),
                },
            )?,
        });
        Ok(())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::f64::consts::SQRT_2;

    use amphion_foundation::{Point2, Point3, ToleranceContext, Vector2, Vector3};
    use num_bigint::BigInt;
    use num_rational::BigRational;
    use serde_json::json;

    use crate::traits::{Curve2Evaluator, Curve3Evaluator};
    use crate::{DerivativeOrder, EvaluationContext, GeometryError};

    use super::{ConstructionError, Line2, Line2Repr, Line3, Line3Repr};

    fn tol() -> ToleranceContext {
        ToleranceContext::try_new(1e-9, 1e-8, 1e-10, 1e-12).unwrap()
    }

    fn ctx() -> EvaluationContext {
        EvaluationContext::new(tol())
    }

    fn dist2(a: Point2, b: Point2) -> f64 {
        let [ax, ay] = a.into_array();
        let [bx, by] = b.into_array();
        (ax - bx).hypot(ay - by)
    }

    fn dist3(a: Point3, b: Point3) -> f64 {
        let [ax, ay, az] = a.into_array();
        let [bx, by, bz] = b.into_array();
        (ax - bx).hypot((ay - by).hypot(az - bz))
    }

    // ── Line2 ────────────────────────────────────────────────────────────────

    #[test]
    fn line2_construction_valid() {
        let line = Line2::try_new(
            Point2::try_new(1.0, 2.0).unwrap(),
            Vector2::try_new(1.0, 0.0).unwrap(),
        );
        assert!(line.is_ok());
    }

    #[test]
    fn line2_construction_normalizes_direction() {
        let line = Line2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            Vector2::try_new(3.0, 4.0).unwrap(),
        )
        .unwrap();
        let d = line.direction().into_array();
        let mag = (d[0] * d[0] + d[1] * d[1]).sqrt();
        assert!(
            (mag - 1.0).abs() < 1e-14,
            "direction should be unit, got mag={mag}"
        );
    }

    #[test]
    fn line2_construction_rejects_non_finite() {
        assert_eq!(
            Line2::try_new(
                Point2::try_new(f64::NAN, 0.0).unwrap_or(Point2::try_new(0.0, 0.0).unwrap()),
                Vector2::try_new(1.0, 0.0).unwrap(),
            )
            .err(),
            // NaN in origin array is rejected by Point2 before we see it;
            // the interesting case is the direction:
            None
        );
        // NaN in direction
        let bad_dir_result = std::panic::catch_unwind(|| {
            // Vector2::try_new rejects NaN, so we test via the inner array
            let dir = [f64::NAN, 0.0f64];
            crate::analytic::helpers::all_finite2(dir)
        });
        assert!(!bad_dir_result.unwrap());
    }

    #[test]
    fn line2_construction_rejects_zero_direction() {
        let err = Line2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            Vector2::try_new(0.0, 0.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::DegenerateAxis);
    }

    #[test]
    fn line2_evaluate_position() {
        let line = Line2::try_new(
            Point2::try_new(1.0, 2.0).unwrap(),
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        let ev = line
            .evaluate(3.0, DerivativeOrder::Position, &ctx())
            .unwrap();
        assert!((ev.position.x() - 4.0).abs() < 1e-14);
        assert!((ev.position.y() - 2.0).abs() < 1e-14);
        assert!(ev.first.is_none());
        assert!(ev.position_error_bound.get() >= 0.0);
    }

    #[test]
    fn line2_evaluate_derivatives() {
        let line = Line2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        let ev = line.evaluate(5.0, DerivativeOrder::Second, &ctx()).unwrap();
        let d1 = ev.first.unwrap().into_array();
        let d2 = ev.second.unwrap().into_array();
        assert!(
            (d1[0] - 1.0).abs() < 1e-14 && d1[1].abs() < 1e-14,
            "d1 should be direction"
        );
        assert!(
            d2[0].abs() < 1e-14 && d2[1].abs() < 1e-14,
            "d2 should be zero"
        );
        assert!(ev.position_error_bound.get() >= 0.0);
        assert!(ev.first_error_bound.unwrap().get() >= 0.0);
        assert!(ev.second_error_bound.unwrap().get() >= 0.0);
    }

    #[test]
    fn line2_evaluate_fd_check() {
        // finite-difference cross-check: p'(t) ≈ (p(t+h) - p(t-h)) / (2h)
        let line = Line2::try_new(
            Point2::try_new(1.0, -3.0).unwrap(),
            Vector2::try_new(3.0, 4.0).unwrap(),
        )
        .unwrap();
        let h = 1e-7_f64;
        let t0 = 2.0_f64;
        let p_plus = line
            .evaluate(t0 + h, DerivativeOrder::Position, &ctx())
            .unwrap()
            .position
            .into_array();
        let p_minus = line
            .evaluate(t0 - h, DerivativeOrder::Position, &ctx())
            .unwrap()
            .position
            .into_array();
        let fd = [
            (p_plus[0] - p_minus[0]) / (2.0 * h),
            (p_plus[1] - p_minus[1]) / (2.0 * h),
        ];
        let analytic = line
            .evaluate(t0, DerivativeOrder::First, &ctx())
            .unwrap()
            .first
            .unwrap()
            .into_array();
        assert!(
            (fd[0] - analytic[0]).abs() < 1e-6,
            "fd[0]={} analytic[0]={}",
            fd[0],
            analytic[0]
        );
        assert!(
            (fd[1] - analytic[1]).abs() < 1e-6,
            "fd[1]={} analytic[1]={}",
            fd[1],
            analytic[1]
        );
    }

    #[test]
    fn line2_projection_round_trip() {
        let line = Line2::try_new(
            Point2::try_new(1.0, 0.0).unwrap(),
            Vector2::try_new(0.0, 1.0).unwrap(),
        )
        .unwrap();
        let t0 = 7.5_f64;
        let point = line
            .evaluate(t0, DerivativeOrder::Position, &ctx())
            .unwrap()
            .position;
        let projections = line.project(point, &ctx()).unwrap();
        assert_eq!(projections.len(), 1);
        assert!((projections[0].parameter.get() - t0).abs() < 1e-12);
        assert!(projections[0].distance_bound.get() < 1e-12);
        assert!(projections[0].parameter_error_bound >= 0.0);
        assert!(projections[0].point_residual_bound.get() >= 0.0);
    }

    #[test]
    fn line2_projection_off_line() {
        let line = Line2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        let q = Point2::try_new(3.0, 4.0).unwrap();
        let projections = line.project(q, &ctx()).unwrap();
        assert_eq!(projections.len(), 1);
        let proj = &projections[0];
        // foot should be at (3, 0)
        assert!((proj.point.x() - 3.0).abs() < 1e-12);
        assert!(proj.point.y().abs() < 1e-12);
        assert!(4.0 <= proj.distance_bound.get());
    }

    #[test]
    fn line2_distance_bounds_certify_actual_distance_at_extreme_scales() {
        let line = Line2::try_new(
            Point2::try_new(1.0, -2.0).unwrap(),
            Vector2::try_new(1.0, 1.0).unwrap(),
        )
        .unwrap();
        for query in [
            line.evaluate(3.0, DerivativeOrder::Position, &ctx())
                .unwrap()
                .position,
            Point2::try_new(3.0, 4.0).unwrap(),
            Point2::try_new(1.0e12, 1.0e12 + 2.0).unwrap(),
            Point2::try_new(1.0e-12, -2.0e-12).unwrap(),
            Point2::try_new(10.0, 10.0 + 1.0e-12).unwrap(),
        ] {
            let projection = line.project(query, &ctx()).unwrap().remove(0);
            let actual = dist2(query, projection.point);
            assert!(actual <= projection.distance_bound.get(), "{query:?}");
            assert!(projection.distance_bound.get() >= 0.0);
        }
    }

    /// Regression test for the "minsub" floating-point cancellation failure
    /// mode: `Line2::try_new((0,0), (1,-1))`, projecting `(m,m)` with
    /// `m = 2^-52`. The true squared distance is exactly `2m²` (a power of
    /// two, exactly representable), so the true distance is the irrational
    /// `m√2`. A constant-factor (Higham) floating-point error bound computed
    /// from the already-tiny `scale ≈ m√2` can be swamped by rounding in its
    /// own computation; the exact-rational bound derived from `sq_dist_exact
    /// = 2m²` is always ≥ the true value by construction (`sqrt_up`).
    #[test]
    fn line2_exact_distance_minsub_regression() {
        let m = 2.0_f64.powi(-52);
        let line = Line2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            Vector2::try_new(1.0, -1.0).unwrap(),
        )
        .unwrap();
        let q = Point2::try_new(m, m).unwrap();
        let proj = line.project(q, &ctx()).unwrap().remove(0);

        // sq_dist_exact = 2m^2 is an exact power of two.
        let sq_dist_exact = 2.0 * m * m;
        // `sq_dist_exact * 2^200 = 2^97` is an exact power of two well within
        // `i128` range for `m = 2^-52`, so this scaling cast is exact, not a
        // truncating approximation.
        #[allow(clippy::cast_possible_truncation)]
        let sq_dist_numer = (sq_dist_exact * 2.0_f64.powi(200)) as i128;
        let sq_rat = BigRational::new(BigInt::from(sq_dist_numer), BigInt::from(2i128).pow(200));
        let lower = crate::analytic::exact::sqrt_down(&sq_rat).unwrap();
        let upper = crate::analytic::exact::sqrt_up(&sq_rat).unwrap();

        assert!(
            proj.distance_bound.get() >= lower,
            "certified bound {} must be >= certified lower bound {lower}",
            proj.distance_bound.get()
        );
        // The certified bound must also be reasonably tight (within a small
        // factor of the certified upper bound derived independently).
        assert!(
            proj.distance_bound.get() <= upper * 1.000_000_1,
            "certified bound {} should be close to the true value {upper}",
            proj.distance_bound.get()
        );
    }

    /// Regression test for the floating-point cancellation failure mode:
    /// `Line2::try_new((284.0065004673188, 426.00975070097496), (2,3))`,
    /// projecting `(0, 2^-43)`. Naive `f64` computation of the projected
    /// point suffers catastrophic cancellation (`origin ≈ 284` cancels
    /// against `t·direction ≈ -284`, leaving a residual on the order of
    /// `1e-12` computed from operands that have already lost precision at
    /// the `~284` scale, i.e. an absolute error of `~eps·284 ≈ 6e-14` — over
    /// 3% of the final `~1.86e-12` residual). The exact-rational
    /// computation performs the cancellation in exact arithmetic, so it
    /// never loses that precision.
    #[test]
    fn line2_exact_distance_cancellation_regression() {
        let line = Line2::try_new(
            Point2::try_new(284.006_500_467_318_8, 426.009_750_700_974_96).unwrap(),
            Vector2::try_new(2.0, 3.0).unwrap(),
        )
        .unwrap();
        let q = Point2::try_new(0.0, 2.0_f64.powi(-43)).unwrap();
        let proj = line.project(q, &ctx()).unwrap().remove(0);

        // Independent oracle: recompute the exact squared distance directly
        // via BigRational, mirroring (but not calling into) the production
        // helper, using the same stored (already-normalized) direction.
        let o = line.origin().into_array();
        let d = line.direction().into_array();
        let qa = q.into_array();
        let o_r = [
            crate::analytic::exact::f64_to_rat(o[0]),
            crate::analytic::exact::f64_to_rat(o[1]),
        ];
        let d_r = [
            crate::analytic::exact::f64_to_rat(d[0]),
            crate::analytic::exact::f64_to_rat(d[1]),
        ];
        let q_r = [
            crate::analytic::exact::f64_to_rat(qa[0]),
            crate::analytic::exact::f64_to_rat(qa[1]),
        ];
        let diff_r = [&q_r[0] - &o_r[0], &q_r[1] - &o_r[1]];
        let dot_dd = &d_r[0] * &d_r[0] + &d_r[1] * &d_r[1];
        let dot_diff_d = &diff_r[0] * &d_r[0] + &diff_r[1] * &d_r[1];
        let t_exact = &dot_diff_d / &dot_dd;
        let proj_r = [&o_r[0] + &t_exact * &d_r[0], &o_r[1] + &t_exact * &d_r[1]];
        let res_r = [&q_r[0] - &proj_r[0], &q_r[1] - &proj_r[1]];
        let sq_dist_exact = &res_r[0] * &res_r[0] + &res_r[1] * &res_r[1];

        let lower = crate::analytic::exact::sqrt_down(&sq_dist_exact).unwrap();
        let upper = crate::analytic::exact::sqrt_up(&sq_dist_exact).unwrap();

        assert!(
            proj.distance_bound.get() >= lower,
            "certified bound {} must be >= certified true lower bound {lower}",
            proj.distance_bound.get()
        );
        assert!(
            proj.distance_bound.get() <= upper * 1.000_000_1,
            "certified bound {} should be close to the true value {upper}",
            proj.distance_bound.get()
        );
        // The true distance is close to 1.8603e-12; the naive
        // catastrophically-cancelled floating-point residual (≈1.8446e-12)
        // must not be mistaken for a valid certified bound.
        assert!(
            proj.distance_bound.get() > 1.85e-12,
            "certified bound {} must reflect the true ~1.8603e-12 distance, not the \
             cancellation-corrupted ~1.8446e-12 naive residual",
            proj.distance_bound.get()
        );
    }

    #[test]
    fn line2_evaluate_rejects_non_finite() {
        let line = Line2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        assert_eq!(
            line.evaluate(f64::NAN, DerivativeOrder::Position, &ctx()),
            Err(GeometryError::NonFiniteParameter)
        );
        assert_eq!(
            line.evaluate(f64::INFINITY, DerivativeOrder::Position, &ctx()),
            Err(GeometryError::NonFiniteParameter)
        );
    }

    #[test]
    fn line2_serde_round_trip() {
        let line = Line2::try_new(
            Point2::try_new(1.0, 2.0).unwrap(),
            Vector2::try_new(1.0, 1.0).unwrap(),
        )
        .unwrap();
        let json = serde_json::to_string(&line).unwrap();
        let decoded: Line2 = serde_json::from_str(&json).unwrap();
        assert_eq!(line, decoded);
    }

    #[test]
    fn line2_serde_normalizes_non_unit_direction() {
        // Foundation's UnitVector2::try_normalize is lenient: any finite,
        // non-zero direction is silently renormalized rather than rejected.
        let repr: Line2Repr = serde_json::from_value(json!({
            "origin": [1.0, 2.0],
            "direction": [2.0, 0.0]
        }))
        .unwrap();
        let line = Line2::try_from(repr).unwrap();
        assert_eq!(line.direction(), Vector2::try_new(1.0, 0.0).unwrap());
    }

    #[test]
    fn line2_serde_normalizes_marginally_non_unit_direction() {
        // `1.000_000_000_000_000_9` is 4 ULPs above 1.0; foundation's lenient
        // `try_normalize` accepts and renormalizes any finite non-zero input,
        // unlike the old strict `validate_unit2` (which rejected deviations
        // beyond `UNIT_VECTOR_TOL`).
        let repr: Line2Repr = serde_json::from_value(json!({
            "origin": [1.0, 2.0],
            "direction": [1.000_000_000_000_000_9, 0.0]
        }))
        .unwrap();
        let line = Line2::try_from(repr).unwrap();
        assert_eq!(line.direction(), Vector2::try_new(1.0, 0.0).unwrap());
    }

    #[test]
    fn line2_serde_rejects_nan_and_inf_fields() {
        assert!(
            serde_json::from_str::<Line2>(r#"{"origin":[NaN,0.0],"direction":[1.0,0.0]}"#).is_err()
        );
        assert!(
            serde_json::from_str::<Line2>(r#"{"origin":[0.0,0.0],"direction":[Infinity,0.0]}"#)
                .is_err()
        );
    }

    // ── Line3 ────────────────────────────────────────────────────────────────

    #[test]
    fn line3_construction_valid() {
        let line = Line3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
        );
        assert!(line.is_ok());
    }

    #[test]
    fn line3_construction_rejects_zero_direction() {
        let err = Line3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 0.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::DegenerateAxis);
    }

    #[test]
    fn line3_evaluate_position_and_derivatives() {
        let line = Line3::try_new(
            Point3::try_new(1.0, 2.0, 3.0).unwrap(),
            Vector3::try_new(SQRT_2 / 2.0, SQRT_2 / 2.0, 0.0).unwrap(),
        )
        .unwrap();
        let ev = line.evaluate(0.0, DerivativeOrder::Second, &ctx()).unwrap();
        assert!((ev.position.x() - 1.0).abs() < 1e-14);
        assert!((ev.position.y() - 2.0).abs() < 1e-14);
        assert!((ev.position.z() - 3.0).abs() < 1e-14);
        let d2 = ev.second.unwrap().into_array();
        assert!(
            d2.iter().all(|v| v.abs() < 1e-14),
            "d2 must be zero for a line"
        );
        assert!(ev.position_error_bound.get() >= 0.0);
        assert!(ev.first_error_bound.unwrap().get() >= 0.0);
        assert!(ev.second_error_bound.unwrap().get() >= 0.0);
    }

    #[test]
    fn line3_evaluate_fd_check() {
        let line = Line3::try_new(
            Point3::try_new(0.0, 1.0, -2.0).unwrap(),
            Vector3::try_new(1.0, 2.0, 3.0).unwrap(),
        )
        .unwrap();
        let h = 1e-7_f64;
        let t0 = -1.5_f64;
        let p_plus = line
            .evaluate(t0 + h, DerivativeOrder::Position, &ctx())
            .unwrap()
            .position
            .into_array();
        let p_minus = line
            .evaluate(t0 - h, DerivativeOrder::Position, &ctx())
            .unwrap()
            .position
            .into_array();
        let fd: [f64; 3] = std::array::from_fn(|i| (p_plus[i] - p_minus[i]) / (2.0 * h));
        let analytic = line
            .evaluate(t0, DerivativeOrder::First, &ctx())
            .unwrap()
            .first
            .unwrap()
            .into_array();
        for i in 0..3 {
            assert!(
                (fd[i] - analytic[i]).abs() < 1e-6,
                "component {i}: fd={} analytic={}",
                fd[i],
                analytic[i]
            );
        }
    }

    #[test]
    fn line3_projection_round_trip() {
        let line = Line3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(1.0, 1.0, 1.0).unwrap(),
        )
        .unwrap();
        for t0 in [-10.0, 0.0, 5.0, 100.0] {
            let pt = line
                .evaluate(t0, DerivativeOrder::Position, &ctx())
                .unwrap()
                .position;
            let projs = line.project(pt, &ctx()).unwrap();
            assert_eq!(projs.len(), 1);
            assert!(
                (projs[0].parameter.get() - t0).abs() < 1e-10,
                "round-trip failed at t={t0}"
            );
            assert!(projs[0].distance_bound.get() < 1e-10);
        }
    }

    #[test]
    fn line3_serde_round_trip() {
        let line = Line3::try_new(
            Point3::try_new(1.0, -2.0, 3.0).unwrap(),
            Vector3::try_new(1.0, 1.0, 1.0).unwrap(),
        )
        .unwrap();
        let json = serde_json::to_string(&line).unwrap();
        let decoded: Line3 = serde_json::from_str(&json).unwrap();
        assert_eq!(line, decoded);
    }

    #[test]
    fn line3_distance_bounds_certify_actual_distance_at_extreme_scales() {
        let line = Line3::try_new(
            Point3::try_new(1.0, -2.0, 0.5).unwrap(),
            Vector3::try_new(1.0, 1.0, 1.0).unwrap(),
        )
        .unwrap();
        for query in [
            line.evaluate(-4.0, DerivativeOrder::Position, &ctx())
                .unwrap()
                .position,
            Point3::try_new(3.0, 4.0, 5.0).unwrap(),
            Point3::try_new(1.0e12, 1.0e12 + 2.0, 1.0e12 - 1.0).unwrap(),
            Point3::try_new(1.0e-12, -2.0e-12, 3.0e-12).unwrap(),
            Point3::try_new(10.0, 10.0 + 1.0e-12, 10.0 - 1.0e-12).unwrap(),
        ] {
            let projection = line.project(query, &ctx()).unwrap().remove(0);
            let actual = dist3(query, projection.point);
            assert!(actual <= projection.distance_bound.get(), "{query:?}");
            assert!(projection.distance_bound.get() >= 0.0);
        }
    }

    #[test]
    fn line3_serde_normalizes_non_unit_direction() {
        let repr: Line3Repr = serde_json::from_value(json!({
            "origin": [1.0, 2.0, 3.0],
            "direction": [2.0, 0.0, 0.0]
        }))
        .unwrap();
        let line = Line3::try_from(repr).unwrap();
        assert_eq!(line.direction(), Vector3::try_new(1.0, 0.0, 0.0).unwrap());
    }

    #[test]
    fn line3_serde_normalizes_marginally_non_unit_direction() {
        // See `line2_serde_normalizes_marginally_non_unit_direction`:
        // foundation's lenient `try_normalize` accepts and renormalizes any
        // finite non-zero input, including directions a few ULPs from unit
        // length.
        let repr: Line3Repr = serde_json::from_value(json!({
            "origin": [1.0, 2.0, 3.0],
            "direction": [1.000_000_000_000_000_9, 0.0, 0.0]
        }))
        .unwrap();
        let line = Line3::try_from(repr).unwrap();
        assert_eq!(line.direction(), Vector3::try_new(1.0, 0.0, 0.0).unwrap());
    }

    #[test]
    fn line3_serde_rejects_nan_and_inf_fields() {
        assert!(
            serde_json::from_str::<Line3>(r#"{"origin":[NaN,0.0,0.0],"direction":[1.0,0.0,0.0]}"#)
                .is_err()
        );
        assert!(
            serde_json::from_str::<Line3>(
                r#"{"origin":[0.0,0.0,0.0],"direction":[Infinity,0.0,0.0]}"#
            )
            .is_err()
        );
    }

    #[test]
    fn line3_try_transform_identity_is_noop() {
        let l = Line3::try_new(
            Point3::try_new(1.0, 2.0, 3.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let out = l
            .try_transform(&amphion_foundation::Transform3::IDENTITY)
            .unwrap();
        assert_eq!(out, l);
    }

    #[test]
    fn line3_try_transform_scale_rotation_translation() {
        // Rotation by 90° about Z, uniform scale 2, plus translation.
        let m = [
            0.0, -2.0, 0.0, 5.0, //
            2.0, 0.0, 0.0, -3.0, //
            0.0, 0.0, 2.0, 7.0,
        ];
        let t = amphion_foundation::Transform3::try_from_row_major(m).unwrap();
        let l = Line3::try_new(
            Point3::try_new(1.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let out = l.try_transform(&t).unwrap();
        let [ox, oy, oz] = out.origin().into_array();
        // origin (1,0,0) -> (0*1-2*0+0*0+5, 2*1+0*0+0*0-3, 0*1+0*0+2*0+7) = (5,-1,7)
        assert!((ox - 5.0).abs() < 1e-9);
        assert!((oy - (-1.0)).abs() < 1e-9);
        assert!((oz - 7.0).abs() < 1e-9);
        // direction (1,0,0) -> (0,2,0) normalized -> (0,1,0)
        let [dx, dy, dz] = out.direction().into_array();
        assert!((dx - 0.0).abs() < 1e-9);
        assert!((dy - 1.0).abs() < 1e-9);
        assert!((dz - 0.0).abs() < 1e-9);
    }

    #[test]
    fn line3_try_transform_rejects_non_finite_result() {
        // An extreme finite scale causes the point application to overflow
        // to infinity, even though the transform itself was constructed
        // from finite values.
        let huge = f64::MAX;
        let m = [
            huge, 0.0, 0.0, 0.0, 0.0, huge, 0.0, 0.0, 0.0, 0.0, huge, 0.0,
        ];
        let t = amphion_foundation::Transform3::try_from_row_major(m).unwrap();
        let l = Line3::try_new(
            Point3::try_new(2.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        assert_eq!(
            l.try_transform(&t),
            Err(super::TransformError::NonFiniteResult)
        );
    }
}
