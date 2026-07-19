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
        add2, add3, all_finite2, all_finite3, angle_to_full_turn, cross3, dot2, dot3, in_range,
        mag2, mag2_sq, mag3, normalize2, normalize3, perp2, scale2, scale3, sub2, sub3,
    },
};

fn circle_domain() -> ParameterRange {
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
        Ok(Self {
            center: Point2::try_new(c[0], c[1]).expect("center validated finite"),
            radius,
            x_axis: Vector2::try_new(x_unit[0], x_unit[1]).expect("unit x_axis is finite"),
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
        let x = self.x_axis.into_array();
        let y = perp2(x);
        Vector2::try_new(y[0], y[1]).expect("perp of unit vector is unit")
    }
}

impl TryFrom<Circle2Repr> for Circle2 {
    type Error = ConstructionError;
    fn try_from(repr: Circle2Repr) -> Result<Self, Self::Error> {
        Self::try_new(repr.center, repr.radius, repr.x_axis)
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
        order: DerivativeOrder,
    ) -> Result<CurveEvaluation2, GeometryError> {
        if !parameter.is_finite() {
            return Err(GeometryError::NonFiniteParameter);
        }
        if !in_range(parameter, self.domain()) {
            return Err(GeometryError::OutsideDomain);
        }
        let c = self.center.into_array();
        let x = self.x_axis.into_array();
        let y = perp2(x);
        let r = self.radius;
        let (cos_t, sin_t) = (parameter.cos(), parameter.sin());

        let pos_arr = add2(c, add2(scale2(x, r * cos_t), scale2(y, r * sin_t)));
        let pos =
            Point2::try_new(pos_arr[0], pos_arr[1]).map_err(|_| GeometryError::Uncertified {
                reason: "circle position is non-finite".to_owned(),
            })?;

        let first = if matches!(order, DerivativeOrder::First | DerivativeOrder::Second) {
            let d1_arr = add2(scale2(x, -r * sin_t), scale2(y, r * cos_t));
            Some(Vector2::try_new(d1_arr[0], d1_arr[1]).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "circle first derivative is non-finite".to_owned(),
                }
            })?)
        } else {
            None
        };

        let second = if order == DerivativeOrder::Second {
            // p″ = −r·(cos θ·x + sin θ·y) = −(p − center)
            let d2_arr = add2(scale2(x, -r * cos_t), scale2(y, -r * sin_t));
            Some(Vector2::try_new(d2_arr[0], d2_arr[1]).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "circle second derivative is non-finite".to_owned(),
                }
            })?)
        } else {
            None
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
        let c = self.center.into_array();
        let x = self.x_axis.into_array();
        let y = perp2(x);
        let r = self.radius;
        let diff = sub2(q, c);
        // A point at exactly the center maps to all circle points simultaneously.
        if mag2_sq(diff) == 0.0 {
            return Err(GeometryError::Singular);
        }
        let diff_len = mag2(diff);
        let theta = angle_to_full_turn(dot2(diff, y).atan2(dot2(diff, x)));
        let (cos_t, sin_t) = (theta.cos(), theta.sin());
        let proj_arr = add2(c, add2(scale2(x, r * cos_t), scale2(y, r * sin_t)));
        let proj =
            Point2::try_new(proj_arr[0], proj_arr[1]).map_err(|_| GeometryError::Uncertified {
                reason: "circle projection point is non-finite".to_owned(),
            })?;
        let dist = (diff_len - r).abs();
        output.push(CurveProjection2 {
            parameter: ParameterValue::try_new(theta).map_err(|_| GeometryError::Uncertified {
                reason: "circle projection parameter is non-finite".to_owned(),
            })?,
            point: proj,
            distance_bound: DistanceBound::try_new(dist).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "circle projection distance is non-finite or negative".to_owned(),
                }
            })?,
        });
        Ok(())
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
        let x_unit = normalize3(x_perp).ok_or(ConstructionError::DependentAxes)?;
        Ok(Self {
            center: Point3::try_new(c[0], c[1], c[2]).expect("center validated finite"),
            radius,
            normal: Vector3::try_new(n_unit[0], n_unit[1], n_unit[2])
                .expect("normal unit vector is finite"),
            x_axis: Vector3::try_new(x_unit[0], x_unit[1], x_unit[2])
                .expect("x_axis unit vector is finite"),
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
        let n = self.normal.into_array();
        let x = self.x_axis.into_array();
        let y = cross3(n, x);
        Vector3::try_new(y[0], y[1], y[2]).expect("cross product of orthonormal pair is unit")
    }
}

impl TryFrom<Circle3Repr> for Circle3 {
    type Error = ConstructionError;
    fn try_from(repr: Circle3Repr) -> Result<Self, Self::Error> {
        Self::try_new(repr.center, repr.radius, repr.normal, repr.x_axis)
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
        order: DerivativeOrder,
    ) -> Result<CurveEvaluation3, GeometryError> {
        if !parameter.is_finite() {
            return Err(GeometryError::NonFiniteParameter);
        }
        if !in_range(parameter, self.domain()) {
            return Err(GeometryError::OutsideDomain);
        }
        let c = self.center.into_array();
        let x = self.x_axis.into_array();
        let y = cross3(self.normal.into_array(), x);
        let r = self.radius;
        let (cos_t, sin_t) = (parameter.cos(), parameter.sin());

        let pos_arr = add3(c, add3(scale3(x, r * cos_t), scale3(y, r * sin_t)));
        let pos = Point3::try_new(pos_arr[0], pos_arr[1], pos_arr[2]).map_err(|_| {
            GeometryError::Uncertified {
                reason: "circle position is non-finite".to_owned(),
            }
        })?;

        let first = if matches!(order, DerivativeOrder::First | DerivativeOrder::Second) {
            let d1 = add3(scale3(x, -r * sin_t), scale3(y, r * cos_t));
            Some(
                Vector3::try_new(d1[0], d1[1], d1[2]).map_err(|_| GeometryError::Uncertified {
                    reason: "circle first derivative is non-finite".to_owned(),
                })?,
            )
        } else {
            None
        };

        let second = if order == DerivativeOrder::Second {
            let d2 = add3(scale3(x, -r * cos_t), scale3(y, -r * sin_t));
            Some(
                Vector3::try_new(d2[0], d2[1], d2[2]).map_err(|_| GeometryError::Uncertified {
                    reason: "circle second derivative is non-finite".to_owned(),
                })?,
            )
        } else {
            None
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
        let c = self.center.into_array();
        let n = self.normal.into_array();
        let x = self.x_axis.into_array();
        let y = cross3(n, x);
        let r = self.radius;
        let diff = sub3(q, c);
        // In-plane component (project off the normal direction).
        let diff_in_plane = sub3(diff, scale3(n, dot3(diff, n)));
        if mag3(diff_in_plane) == 0.0 {
            // Point is on the circle's axis; every θ is equidistant.
            return Err(GeometryError::Singular);
        }
        let proj_x = dot3(diff_in_plane, x);
        let proj_y = dot3(diff_in_plane, y);
        let theta = angle_to_full_turn(proj_y.atan2(proj_x));
        let (cos_t, sin_t) = (theta.cos(), theta.sin());
        let proj_arr = add3(c, add3(scale3(x, r * cos_t), scale3(y, r * sin_t)));
        let proj = Point3::try_new(proj_arr[0], proj_arr[1], proj_arr[2]).map_err(|_| {
            GeometryError::Uncertified {
                reason: "circle projection point is non-finite".to_owned(),
            }
        })?;
        // Distance from q to proj.
        let delta = sub3(q, proj_arr);
        let dist = mag3(delta);
        output.push(CurveProjection3 {
            parameter: ParameterValue::try_new(theta).map_err(|_| GeometryError::Uncertified {
                reason: "circle projection parameter is non-finite".to_owned(),
            })?,
            point: proj,
            distance_bound: DistanceBound::try_new(dist).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "circle projection distance is non-finite or negative".to_owned(),
                }
            })?,
        });
        Ok(())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

    use amphion_foundation::{Point2, Point3, ToleranceContext, Vector2, Vector3};

    use crate::traits::{Curve2Evaluator, Curve3Evaluator};
    use crate::{DerivativeOrder, GeometryError};

    use super::{Circle2, Circle3, ConstructionError};

    fn tol() -> ToleranceContext {
        ToleranceContext::try_new(1e-9, 1e-8, 1e-10, 1e-12).unwrap()
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
    fn circle2_evaluate_position_at_cardinal_angles() {
        let c = Circle2::try_new(
            Point2::try_new(1.0, 2.0).unwrap(),
            3.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        let p0 = c.evaluate(0.0, DerivativeOrder::Position).unwrap().position;
        let p90 = c
            .evaluate(FRAC_PI_2, DerivativeOrder::Position)
            .unwrap()
            .position;
        let p180 = c.evaluate(PI, DerivativeOrder::Position).unwrap().position;
        assert!((p0.x() - 4.0).abs() < 1e-13 && (p0.y() - 2.0).abs() < 1e-13);
        assert!((p90.x() - 1.0).abs() < 1e-13 && (p90.y() - 5.0).abs() < 1e-13);
        assert!((p180.x() - (-2.0)).abs() < 1e-13 && (p180.y() - 2.0).abs() < 1e-13);
    }

    #[test]
    fn circle2_second_derivative_identity() {
        // p″(θ) = −(p(θ) − center)
        let c = Circle2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            2.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        for theta in [0.1, 1.0, 2.5, PI, 5.0] {
            let ev = c.evaluate(theta, DerivativeOrder::Second).unwrap();
            let p = ev.position.into_array();
            let d2 = ev.second.unwrap().into_array();
            // p″ should equal −p (center is origin here)
            for i in 0..2 {
                assert!(
                    (d2[i] + p[i]).abs() < 1e-12,
                    "d2[{i}]={} ≠ -p[{i}]={}",
                    d2[i],
                    -p[i]
                );
            }
        }
    }

    #[test]
    fn circle2_evaluate_fd_check() {
        let c = Circle2::try_new(
            Point2::try_new(1.0, -1.0).unwrap(),
            2.5,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        let h = 1e-7_f64;
        for theta in [0.3_f64, 1.5, 3.0, 5.0] {
            let p_plus = c
                .evaluate(theta + h, DerivativeOrder::Position)
                .unwrap()
                .position
                .into_array();
            let p_minus = c
                .evaluate(theta - h, DerivativeOrder::Position)
                .unwrap()
                .position
                .into_array();
            let fd = [
                (p_plus[0] - p_minus[0]) / (2.0 * h),
                (p_plus[1] - p_minus[1]) / (2.0 * h),
            ];
            let analytic = c
                .evaluate(theta, DerivativeOrder::First)
                .unwrap()
                .first
                .unwrap()
                .into_array();
            for i in 0..2 {
                assert!(
                    (fd[i] - analytic[i]).abs() < 1e-5,
                    "θ={theta} component {i}: fd={} analytic={}",
                    fd[i],
                    analytic[i]
                );
            }
        }
    }

    #[test]
    fn circle2_periodic_equivalence() {
        // p(θ) and p(θ) for θ near the seam should be handled by the domain.
        let c = Circle2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            1.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        // The seam: θ = 0 and θ = 2π - ε are distinct parameters.
        let p0 = c.evaluate(0.0, DerivativeOrder::Position).unwrap().position;
        // Evaluate just below the seam.
        let eps = 1e-12_f64;
        let p_near = c
            .evaluate(TAU - eps, DerivativeOrder::Position)
            .unwrap()
            .position;
        // They should be very close but not identical.
        let d = ((p0.x() - p_near.x()).powi(2) + (p0.y() - p_near.y()).powi(2)).sqrt();
        assert!(d < 1e-10, "seam continuity: distance={d}");
    }

    #[test]
    fn circle2_projection_round_trip() {
        let c = Circle2::try_new(
            Point2::try_new(2.0, 3.0).unwrap(),
            4.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        for theta in [
            0.0_f64,
            FRAC_PI_4,
            FRAC_PI_2,
            PI,
            3.0 * PI / 2.0,
            TAU - 0.001,
        ] {
            let pt = c
                .evaluate(theta, DerivativeOrder::Position)
                .unwrap()
                .position;
            let projs = c.project(pt, &tol()).unwrap();
            assert_eq!(projs.len(), 1);
            assert!(
                (projs[0].parameter.get() - theta).abs() < 1e-11,
                "round-trip θ={theta}"
            );
            assert!(projs[0].distance_bound.get() < 1e-11);
        }
    }

    #[test]
    fn circle2_projection_singular_at_center() {
        let c = Circle2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            1.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        let center = c.center();
        assert_eq!(c.project(center, &tol()), Err(GeometryError::Singular));
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
            c.evaluate(-0.001, DerivativeOrder::Position),
            Err(GeometryError::OutsideDomain)
        );
        assert_eq!(
            c.evaluate(TAU, DerivativeOrder::Position),
            Err(GeometryError::OutsideDomain)
        );
    }

    #[test]
    fn circle2_serde_round_trip() {
        let c = Circle2::try_new(
            Point2::try_new(1.0, -2.0).unwrap(),
            3.5,
            Vector2::try_new(0.0, 1.0).unwrap(),
        )
        .unwrap();
        let json = serde_json::to_string(&c).unwrap();
        let decoded: Circle2 = serde_json::from_str(&json).unwrap();
        assert_eq!(c, decoded);
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
    fn circle3_evaluate_position_at_cardinal_angles() {
        let c = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 1.0).unwrap(),
            2.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let p0 = c.evaluate(0.0, DerivativeOrder::Position).unwrap().position;
        let p90 = c
            .evaluate(FRAC_PI_2, DerivativeOrder::Position)
            .unwrap()
            .position;
        // θ=0: center + 2*x = (2, 0, 1)
        assert!((p0.x() - 2.0).abs() < 1e-13);
        assert!((p0.y()).abs() < 1e-13);
        assert!((p0.z() - 1.0).abs() < 1e-13);
        // θ=π/2: center + 2*y = (0, 2, 1)
        assert!((p90.x()).abs() < 1e-13);
        assert!((p90.y() - 2.0).abs() < 1e-13);
    }

    #[test]
    fn circle3_second_derivative_identity() {
        // p″(θ) = −(p(θ) − center)  for circle centered at origin
        let c = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            3.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        for theta in [0.2, 1.0, 2.0, 4.0, 5.5] {
            let ev = c.evaluate(theta, DerivativeOrder::Second).unwrap();
            let p = ev.position.into_array();
            let d2 = ev.second.unwrap().into_array();
            for i in 0..3 {
                assert!(
                    (d2[i] + p[i]).abs() < 1e-11,
                    "θ={theta} d2[{i}]+p[{i}]={}",
                    d2[i] + p[i]
                );
            }
        }
    }

    #[test]
    fn circle3_evaluate_fd_check() {
        let c = Circle3::try_new(
            Point3::try_new(1.0, 0.0, 0.0).unwrap(),
            1.5,
            Vector3::try_new(0.0, 1.0, 0.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let h = 1e-7_f64;
        for theta in [0.5_f64, 1.5, 3.0, 4.5] {
            let p_plus = c
                .evaluate(theta + h, DerivativeOrder::Position)
                .unwrap()
                .position
                .into_array();
            let p_minus = c
                .evaluate(theta - h, DerivativeOrder::Position)
                .unwrap()
                .position
                .into_array();
            let fd: [f64; 3] = std::array::from_fn(|i| (p_plus[i] - p_minus[i]) / (2.0 * h));
            let analytic = c
                .evaluate(theta, DerivativeOrder::First)
                .unwrap()
                .first
                .unwrap()
                .into_array();
            for i in 0..3 {
                assert!(
                    (fd[i] - analytic[i]).abs() < 1e-5,
                    "θ={theta} component {i}"
                );
            }
        }
    }

    #[test]
    fn circle3_projection_round_trip() {
        let c = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            5.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        for theta in [0.0_f64, 0.5, PI, 4.0, TAU - 0.001] {
            let pt = c
                .evaluate(theta, DerivativeOrder::Position)
                .unwrap()
                .position;
            let projs = c.project(pt, &tol()).unwrap();
            assert_eq!(projs.len(), 1, "θ={theta}");
            assert!(
                (projs[0].parameter.get() - theta).abs() < 1e-11,
                "round-trip θ={theta}"
            );
            assert!(projs[0].distance_bound.get() < 1e-11, "θ={theta}");
        }
    }

    #[test]
    fn circle3_projection_off_plane() {
        // Point above the circle's plane projects correctly.
        let c = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            1.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        // Point at (1, 0, 5): directly above θ=0
        let q = Point3::try_new(1.0, 0.0, 5.0).unwrap();
        let projs = c.project(q, &tol()).unwrap();
        assert_eq!(projs.len(), 1);
        assert!((projs[0].parameter.get()).abs() < 1e-11);
        // distance = sqrt((1-1)^2 + 0^2 + 5^2) = 5
        assert!((projs[0].distance_bound.get() - 5.0).abs() < 1e-11);
    }

    #[test]
    fn circle3_projection_singular_on_axis() {
        let c = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            1.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        // Point exactly on the Z-axis: all θ equidistant.
        let q = Point3::try_new(0.0, 0.0, 3.0).unwrap();
        assert_eq!(c.project(q, &tol()), Err(GeometryError::Singular));
    }

    #[test]
    fn circle3_serde_round_trip() {
        let c = Circle3::try_new(
            Point3::try_new(1.0, 2.0, 3.0).unwrap(),
            4.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let json = serde_json::to_string(&c).unwrap();
        let decoded: Circle3 = serde_json::from_str(&json).unwrap();
        assert_eq!(c, decoded);
    }
}
