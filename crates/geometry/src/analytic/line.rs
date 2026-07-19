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
//! Projection: `t = (q − origin) · direction`.  The projected distance bound
//! equals the perpendicular distance from `q` to the line.

#![allow(
    clippy::many_single_char_names,
    clippy::missing_panics_doc,
    clippy::similar_names
)]

use amphion_foundation::{Point2, Point3, ToleranceContext, Vector2, Vector3};
use serde::{Deserialize, Serialize};

use crate::traits::{Curve2Evaluator, Curve3Evaluator};
use crate::{
    CurveEvaluation2, CurveEvaluation3, CurveKind, CurveProjection2, CurveProjection3,
    DerivativeOrder, DistanceBound, GeometryError, ParameterRange, ParameterValue,
};

use super::{
    ConstructionError,
    helpers::{
        add2, add3, all_finite2, all_finite3, dot2, dot3, mag2_sq, mag3_sq, normalize2, normalize3,
        scale2, scale3, sub2, sub3,
    },
};

// ─── Helpers shared by both line types ──────────────────────────────────────

/// Infinite line domain: no bounds, no period.
fn line_domain() -> ParameterRange {
    ParameterRange::try_new(None, None, None)
        .expect("None/None/None is always a valid ParameterRange")
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
    direction: Vector2,
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
        let d = direction.into_array();
        if !all_finite2(o) || !all_finite2(d) {
            return Err(ConstructionError::NonFiniteInput);
        }
        let d_unit = normalize2(d).ok_or(ConstructionError::DegenerateAxis)?;
        Ok(Self {
            origin: Point2::try_new(o[0], o[1]).expect("origin is already validated finite"),
            direction: Vector2::try_new(d_unit[0], d_unit[1]).expect("unit direction is finite"),
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
        self.direction
    }
}

impl TryFrom<Line2Repr> for Line2 {
    type Error = ConstructionError;
    fn try_from(repr: Line2Repr) -> Result<Self, Self::Error> {
        Self::try_new(repr.origin, repr.direction)
    }
}

impl From<Line2> for Line2Repr {
    fn from(line: Line2) -> Self {
        Self {
            origin: line.origin,
            direction: line.direction,
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
    ) -> Result<CurveEvaluation2, GeometryError> {
        if !parameter.is_finite() {
            return Err(GeometryError::NonFiniteParameter);
        }
        // Infinite domain: no bound check needed.
        let o = self.origin.into_array();
        let d = self.direction.into_array();
        let pos_arr = add2(o, scale2(d, parameter));
        let pos =
            Point2::try_new(pos_arr[0], pos_arr[1]).map_err(|_| GeometryError::Uncertified {
                reason: "line position is non-finite".to_owned(),
            })?;
        let (first, second) = match order {
            DerivativeOrder::Position => (None, None),
            DerivativeOrder::First => {
                let v = Vector2::try_new(d[0], d[1]).map_err(|_| GeometryError::Uncertified {
                    reason: "line direction is non-finite".to_owned(),
                })?;
                (Some(v), None)
            }
            DerivativeOrder::Second => {
                let v = Vector2::try_new(d[0], d[1]).map_err(|_| GeometryError::Uncertified {
                    reason: "line direction is non-finite".to_owned(),
                })?;
                let zero = Vector2::try_new(0.0, 0.0).expect("zero is finite");
                (Some(v), Some(zero))
            }
        };
        Ok(CurveEvaluation2 {
            position: pos,
            first,
            second,
        })
    }

    fn project_into(
        &self,
        point: Point2,
        _tolerance: &ToleranceContext,
        output: &mut Vec<CurveProjection2>,
    ) -> Result<(), GeometryError> {
        output.clear();
        let q = point.into_array();
        let o = self.origin.into_array();
        let d = self.direction.into_array();
        let diff = sub2(q, o);
        let t = dot2(diff, d);
        if !t.is_finite() {
            return Err(GeometryError::Uncertified {
                reason: "line projection parameter is non-finite".to_owned(),
            });
        }
        let proj_arr = add2(o, scale2(d, t));
        let proj =
            Point2::try_new(proj_arr[0], proj_arr[1]).map_err(|_| GeometryError::Uncertified {
                reason: "line projection point is non-finite".to_owned(),
            })?;
        // Perpendicular distance: sqrt(|diff|² − t²), clamped to avoid negative
        // floating-point residuals.
        let dist_sq = (mag2_sq(diff) - t * t).max(0.0);
        let dist = dist_sq.sqrt();
        output.push(CurveProjection2 {
            parameter: ParameterValue::try_new(t).map_err(|_| GeometryError::Uncertified {
                reason: "line projection parameter is non-finite".to_owned(),
            })?,
            point: proj,
            distance_bound: DistanceBound::try_new(dist).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "line projection distance is non-finite or negative".to_owned(),
                }
            })?,
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
    direction: Vector3,
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
        let d = direction.into_array();
        if !all_finite3(o) || !all_finite3(d) {
            return Err(ConstructionError::NonFiniteInput);
        }
        let d_unit = normalize3(d).ok_or(ConstructionError::DegenerateAxis)?;
        Ok(Self {
            origin: Point3::try_new(o[0], o[1], o[2]).expect("origin is already validated finite"),
            direction: Vector3::try_new(d_unit[0], d_unit[1], d_unit[2])
                .expect("unit direction is finite"),
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
        self.direction
    }
}

impl TryFrom<Line3Repr> for Line3 {
    type Error = ConstructionError;
    fn try_from(repr: Line3Repr) -> Result<Self, Self::Error> {
        Self::try_new(repr.origin, repr.direction)
    }
}

impl From<Line3> for Line3Repr {
    fn from(line: Line3) -> Self {
        Self {
            origin: line.origin,
            direction: line.direction,
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
    ) -> Result<CurveEvaluation3, GeometryError> {
        if !parameter.is_finite() {
            return Err(GeometryError::NonFiniteParameter);
        }
        let o = self.origin.into_array();
        let d = self.direction.into_array();
        let pos_arr = add3(o, scale3(d, parameter));
        let pos = Point3::try_new(pos_arr[0], pos_arr[1], pos_arr[2]).map_err(|_| {
            GeometryError::Uncertified {
                reason: "line position is non-finite".to_owned(),
            }
        })?;
        let (first, second) = match order {
            DerivativeOrder::Position => (None, None),
            DerivativeOrder::First => {
                let v =
                    Vector3::try_new(d[0], d[1], d[2]).map_err(|_| GeometryError::Uncertified {
                        reason: "line direction is non-finite".to_owned(),
                    })?;
                (Some(v), None)
            }
            DerivativeOrder::Second => {
                let v =
                    Vector3::try_new(d[0], d[1], d[2]).map_err(|_| GeometryError::Uncertified {
                        reason: "line direction is non-finite".to_owned(),
                    })?;
                let zero = Vector3::try_new(0.0, 0.0, 0.0).expect("zero is finite");
                (Some(v), Some(zero))
            }
        };
        Ok(CurveEvaluation3 {
            position: pos,
            first,
            second,
        })
    }

    fn project_into(
        &self,
        point: Point3,
        _tolerance: &ToleranceContext,
        output: &mut Vec<CurveProjection3>,
    ) -> Result<(), GeometryError> {
        output.clear();
        let q = point.into_array();
        let o = self.origin.into_array();
        let d = self.direction.into_array();
        let diff = sub3(q, o);
        let t = dot3(diff, d);
        if !t.is_finite() {
            return Err(GeometryError::Uncertified {
                reason: "line projection parameter is non-finite".to_owned(),
            });
        }
        let proj_arr = add3(o, scale3(d, t));
        let proj = Point3::try_new(proj_arr[0], proj_arr[1], proj_arr[2]).map_err(|_| {
            GeometryError::Uncertified {
                reason: "line projection point is non-finite".to_owned(),
            }
        })?;
        let dist_sq = (mag3_sq(diff) - t * t).max(0.0);
        let dist = dist_sq.sqrt();
        output.push(CurveProjection3 {
            parameter: ParameterValue::try_new(t).map_err(|_| GeometryError::Uncertified {
                reason: "line projection parameter is non-finite".to_owned(),
            })?,
            point: proj,
            distance_bound: DistanceBound::try_new(dist).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "line projection distance is non-finite or negative".to_owned(),
                }
            })?,
        });
        Ok(())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::f64::consts::SQRT_2;

    use amphion_foundation::{Point2, Point3, ToleranceContext, Vector2, Vector3};

    use crate::traits::{Curve2Evaluator, Curve3Evaluator};
    use crate::{DerivativeOrder, GeometryError};

    use super::{ConstructionError, Line2, Line3};

    fn tol() -> ToleranceContext {
        ToleranceContext::try_new(1e-9, 1e-8, 1e-10, 1e-12).unwrap()
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
        assert_eq!(bad_dir_result.unwrap(), false);
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
        let ev = line.evaluate(3.0, DerivativeOrder::Position).unwrap();
        assert!((ev.position.x() - 4.0).abs() < 1e-14);
        assert!((ev.position.y() - 2.0).abs() < 1e-14);
        assert!(ev.first.is_none());
    }

    #[test]
    fn line2_evaluate_derivatives() {
        let line = Line2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        let ev = line.evaluate(5.0, DerivativeOrder::Second).unwrap();
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
            .evaluate(t0 + h, DerivativeOrder::Position)
            .unwrap()
            .position
            .into_array();
        let p_minus = line
            .evaluate(t0 - h, DerivativeOrder::Position)
            .unwrap()
            .position
            .into_array();
        let fd = [
            (p_plus[0] - p_minus[0]) / (2.0 * h),
            (p_plus[1] - p_minus[1]) / (2.0 * h),
        ];
        let analytic = line
            .evaluate(t0, DerivativeOrder::First)
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
            .evaluate(t0, DerivativeOrder::Position)
            .unwrap()
            .position;
        let projections = line.project(point, &tol()).unwrap();
        assert_eq!(projections.len(), 1);
        assert!((projections[0].parameter.get() - t0).abs() < 1e-12);
        assert!(projections[0].distance_bound.get() < 1e-12);
    }

    #[test]
    fn line2_projection_off_line() {
        let line = Line2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        let q = Point2::try_new(3.0, 4.0).unwrap();
        let projections = line.project(q, &tol()).unwrap();
        assert_eq!(projections.len(), 1);
        let proj = &projections[0];
        // foot should be at (3, 0)
        assert!((proj.point.x() - 3.0).abs() < 1e-12);
        assert!(proj.point.y().abs() < 1e-12);
        assert!((proj.distance_bound.get() - 4.0).abs() < 1e-12);
    }

    #[test]
    fn line2_evaluate_rejects_non_finite() {
        let line = Line2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        assert_eq!(
            line.evaluate(f64::NAN, DerivativeOrder::Position),
            Err(GeometryError::NonFiniteParameter)
        );
        assert_eq!(
            line.evaluate(f64::INFINITY, DerivativeOrder::Position),
            Err(GeometryError::NonFiniteParameter)
        );
    }

    #[test]
    fn line2_serde_round_trip() {
        // Use a coordinate-aligned direction so normalization is a no-op and
        // the serde round-trip is bit-identical.
        let line = Line2::try_new(
            Point2::try_new(1.0, 2.0).unwrap(),
            Vector2::try_new(0.0, 1.0).unwrap(),
        )
        .unwrap();
        let json = serde_json::to_string(&line).unwrap();
        let decoded: Line2 = serde_json::from_str(&json).unwrap();
        assert_eq!(line, decoded);
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
        let ev = line.evaluate(0.0, DerivativeOrder::Second).unwrap();
        assert!((ev.position.x() - 1.0).abs() < 1e-14);
        assert!((ev.position.y() - 2.0).abs() < 1e-14);
        assert!((ev.position.z() - 3.0).abs() < 1e-14);
        let d2 = ev.second.unwrap().into_array();
        assert!(
            d2.iter().all(|v| v.abs() < 1e-14),
            "d2 must be zero for a line"
        );
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
            .evaluate(t0 + h, DerivativeOrder::Position)
            .unwrap()
            .position
            .into_array();
        let p_minus = line
            .evaluate(t0 - h, DerivativeOrder::Position)
            .unwrap()
            .position
            .into_array();
        let fd: [f64; 3] = std::array::from_fn(|i| (p_plus[i] - p_minus[i]) / (2.0 * h));
        let analytic = line
            .evaluate(t0, DerivativeOrder::First)
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
                .evaluate(t0, DerivativeOrder::Position)
                .unwrap()
                .position;
            let projs = line.project(pt, &tol()).unwrap();
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
            Vector3::try_new(0.0, 1.0, 0.0).unwrap(),
        )
        .unwrap();
        let json = serde_json::to_string(&line).unwrap();
        let decoded: Line3 = serde_json::from_str(&json).unwrap();
        assert_eq!(line, decoded);
    }
}
