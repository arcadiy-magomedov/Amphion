//! Analytic right circular cone surface.
//!
//! # Parameterization
//!
//! ```text
//! p(u, v) = apex + v·axis + v·tan(α)·(cos(u)·x_axis + sin(u)·y_axis)
//! ```
//!
//! where `α = half_angle`, `y_axis = axis × x_axis`.  `axis`, `x_axis`, and
//! `y_axis` form a right-handed orthonormal frame.  The parameter `v` is the
//! signed axial distance from the apex; both nappes (`v > 0` and `v < 0`) are
//! included.
//!
//! - U domain: `[0, 2π)` with period `2π` (angular)
//! - V domain: `(−∞, +∞)` (axial signed height from apex)
//!
//! Derivatives:
//! ```text
//! ∂p/∂u  =  v·tan(α)·(−sin u·x_axis + cos u·y_axis)
//! ∂p/∂v  =  axis + tan(α)·(cos u·x_axis + sin u·y_axis)
//! ∂²p/∂u²  =  −v·tan(α)·(cos u·x_axis + sin u·y_axis)
//! ∂²p/∂u∂v  =  tan(α)·(−sin u·x_axis + cos u·y_axis)
//! ∂²p/∂v²   =  0
//! ```
//!
//! The apex `v = 0` is a conical singularity: `∂p/∂u = 0` there.  Requesting
//! first- or second-order derivatives at `v = 0` returns
//! [`GeometryError::Singular`]; position evaluation at the apex is valid.
//!
//! Projection: decompose `q − apex` into axial (`h`) and radial (`r`,
//! non-negative) components, then project the point `(h, r)` onto the
//! nearest cone nappe(s) in the axial-radial half-plane.  When `h = 0` and
//! `r > 0` the two nappes are equidistant and both projections are returned.
//! Returns [`GeometryError::Singular`] when the radial component is exactly
//! zero (point on the cone axis).

#![allow(
    clippy::many_single_char_names,
    clippy::missing_panics_doc,
    clippy::similar_names
)]

use std::f64::consts::TAU;

use amphion_foundation::{Point3, ToleranceContext, Vector3};
use serde::{Deserialize, Serialize};

use crate::traits::SurfaceEvaluator;
use crate::{
    DerivativeOrder, DistanceBound, GeometryError, ParameterRange, ParameterValue, SurfaceDomain,
    SurfaceEvaluation, SurfaceKind, SurfaceProjection,
};

use super::{
    ConstructionError,
    helpers::{
        add3, all_finite3, angle_to_full_turn, cross3, dot3, in_range, mag3, normalize3, scale3,
        sub3,
    },
};

fn angular_range() -> ParameterRange {
    ParameterRange::try_new(Some(0.0), Some(TAU), Some(TAU))
        .expect("angular [0, 2π) is always valid")
}

fn unbounded_range() -> ParameterRange {
    ParameterRange::try_new(None, None, None).expect("unbounded range is always valid")
}

#[derive(Serialize, Deserialize)]
struct ConeRepr {
    apex: Point3,
    axis: Vector3,
    half_angle: f64,
    x_axis: Vector3,
}

/// A right circular cone surface, including both nappes.
///
/// Parameterization:
/// ```text
/// p(u, v) = apex + v·axis + v·tan(α)·(cos(u)·x_axis + sin(u)·y_axis)
/// ```
/// U ∈ `[0, 2π)` (periodic), V ∈ `(−∞, +∞)`.  `α = half_angle` ∈ `(0, π/2)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ConeRepr", into = "ConeRepr")]
pub struct Cone {
    apex: Point3,
    axis: Vector3,
    half_angle: f64,
    x_axis: Vector3,
}

impl Cone {
    /// Constructs a cone.
    ///
    /// `axis` and `x_axis` are normalized internally.  `x_axis` is
    /// orthogonalized against `axis` (Gram-Schmidt).
    ///
    /// # Errors
    ///
    /// - [`ConstructionError::NonFiniteInput`] — any NaN/Inf input
    /// - [`ConstructionError::DegenerateAxis`] — zero-length `axis` or `x_axis`
    /// - [`ConstructionError::InvalidHalfAngle`] — `half_angle ∉ (0, π/2)`
    /// - [`ConstructionError::DependentAxes`] — `x_axis` parallel to `axis`
    pub fn try_new(
        apex: Point3,
        axis: Vector3,
        half_angle: f64,
        x_axis: Vector3,
    ) -> Result<Self, ConstructionError> {
        let ap = apex.into_array();
        let a = axis.into_array();
        let x = x_axis.into_array();
        if !all_finite3(ap) || !all_finite3(a) || !half_angle.is_finite() || !all_finite3(x) {
            return Err(ConstructionError::NonFiniteInput);
        }
        if half_angle <= 0.0 || half_angle >= std::f64::consts::FRAC_PI_2 {
            return Err(ConstructionError::InvalidHalfAngle);
        }
        let a_unit = normalize3(a).ok_or(ConstructionError::DegenerateAxis)?;
        let x_norm = normalize3(x).ok_or(ConstructionError::DegenerateAxis)?;
        // Orthogonalize x against axis.
        let dot_xa = dot3(x_norm, a_unit);
        let x_perp = sub3(x_norm, scale3(a_unit, dot_xa));
        let x_unit = normalize3(x_perp).ok_or(ConstructionError::DependentAxes)?;
        Ok(Self {
            apex: Point3::try_new(ap[0], ap[1], ap[2]).expect("apex validated finite"),
            axis: Vector3::try_new(a_unit[0], a_unit[1], a_unit[2]).expect("unit axis is finite"),
            half_angle,
            x_axis: Vector3::try_new(x_unit[0], x_unit[1], x_unit[2])
                .expect("unit x_axis is finite"),
        })
    }

    /// Returns the apex point.
    #[must_use]
    pub fn apex(&self) -> Point3 {
        self.apex
    }

    /// Returns the unit axis direction.
    #[must_use]
    pub fn axis(&self) -> Vector3 {
        self.axis
    }

    /// Returns the half-angle in radians (strictly between 0 and π/2).
    #[must_use]
    pub fn half_angle(&self) -> f64 {
        self.half_angle
    }

    /// Returns the unit reference direction for `u = 0`.
    #[must_use]
    pub fn x_axis(&self) -> Vector3 {
        self.x_axis
    }

    /// Returns the unit y-axis: `axis × x_axis`.
    #[must_use]
    pub fn y_axis(&self) -> Vector3 {
        let a = self.axis.into_array();
        let x = self.x_axis.into_array();
        let y = cross3(a, x);
        Vector3::try_new(y[0], y[1], y[2]).expect("cross product of orthonormal pair is finite")
    }
}

impl TryFrom<ConeRepr> for Cone {
    type Error = ConstructionError;
    fn try_from(repr: ConeRepr) -> Result<Self, Self::Error> {
        Self::try_new(repr.apex, repr.axis, repr.half_angle, repr.x_axis)
    }
}

impl From<Cone> for ConeRepr {
    fn from(c: Cone) -> Self {
        Self {
            apex: c.apex,
            axis: c.axis,
            half_angle: c.half_angle,
            x_axis: c.x_axis,
        }
    }
}

impl SurfaceEvaluator for Cone {
    fn kind(&self) -> SurfaceKind {
        SurfaceKind::Cone
    }

    fn domain(&self) -> SurfaceDomain {
        SurfaceDomain::new(angular_range(), unbounded_range())
    }

    fn evaluate(
        &self,
        u: f64,
        v: f64,
        order: DerivativeOrder,
    ) -> Result<SurfaceEvaluation, GeometryError> {
        if !u.is_finite() || !v.is_finite() {
            return Err(GeometryError::NonFiniteParameter);
        }
        if !in_range(u, self.domain().u()) {
            return Err(GeometryError::OutsideDomain);
        }
        // Apex singularity: ∂p/∂u = 0 at v = 0.
        if v == 0.0 && order != DerivativeOrder::Position {
            return Err(GeometryError::Singular);
        }
        let ap = self.apex.into_array();
        let a = self.axis.into_array();
        let x = self.x_axis.into_array();
        let y = cross3(a, x);
        let tan_a = self.half_angle.tan();
        let (cos_u, sin_u) = (u.cos(), u.sin());

        // p = apex + v·a + v·tan(α)·(cos u·x + sin u·y)
        let radial_dir = add3(scale3(x, cos_u), scale3(y, sin_u));
        let pos_arr = add3(ap, add3(scale3(a, v), scale3(radial_dir, v * tan_a)));
        let pos = Point3::try_new(pos_arr[0], pos_arr[1], pos_arr[2]).map_err(|_| {
            GeometryError::Uncertified {
                reason: "cone position is non-finite".to_owned(),
            }
        })?;

        let (du, dv, duu, duv, dvv) = match order {
            DerivativeOrder::Position => (None, None, None, None, None),
            DerivativeOrder::First => {
                // ∂p/∂u = v·tan(α)·(−sin u·x + cos u·y)
                let tang_dir = add3(scale3(x, -sin_u), scale3(y, cos_u));
                let du_arr = scale3(tang_dir, v * tan_a);
                let du = Vector3::try_new(du_arr[0], du_arr[1], du_arr[2]).map_err(|_| {
                    GeometryError::Uncertified {
                        reason: "cone du non-finite".to_owned(),
                    }
                })?;
                // ∂p/∂v = a + tan(α)·(cos u·x + sin u·y)
                let dv_arr = add3(a, scale3(radial_dir, tan_a));
                let dv = Vector3::try_new(dv_arr[0], dv_arr[1], dv_arr[2]).map_err(|_| {
                    GeometryError::Uncertified {
                        reason: "cone dv non-finite".to_owned(),
                    }
                })?;
                (Some(du), Some(dv), None, None, None)
            }
            DerivativeOrder::Second => {
                let tang_dir = add3(scale3(x, -sin_u), scale3(y, cos_u));
                let du_arr = scale3(tang_dir, v * tan_a);
                let du = Vector3::try_new(du_arr[0], du_arr[1], du_arr[2]).map_err(|_| {
                    GeometryError::Uncertified {
                        reason: "cone du non-finite".to_owned(),
                    }
                })?;
                let dv_arr = add3(a, scale3(radial_dir, tan_a));
                let dv = Vector3::try_new(dv_arr[0], dv_arr[1], dv_arr[2]).map_err(|_| {
                    GeometryError::Uncertified {
                        reason: "cone dv non-finite".to_owned(),
                    }
                })?;
                // ∂²p/∂u² = −v·tan(α)·(cos u·x + sin u·y)
                let duu_arr = scale3(radial_dir, -v * tan_a);
                let duu = Vector3::try_new(duu_arr[0], duu_arr[1], duu_arr[2]).map_err(|_| {
                    GeometryError::Uncertified {
                        reason: "cone duu non-finite".to_owned(),
                    }
                })?;
                // ∂²p/∂u∂v = tan(α)·(−sin u·x + cos u·y)
                let duv_arr = scale3(tang_dir, tan_a);
                let duv = Vector3::try_new(duv_arr[0], duv_arr[1], duv_arr[2]).map_err(|_| {
                    GeometryError::Uncertified {
                        reason: "cone duv non-finite".to_owned(),
                    }
                })?;
                let zero = Vector3::try_new(0.0, 0.0, 0.0).expect("zero is finite");
                (Some(du), Some(dv), Some(duu), Some(duv), Some(zero))
            }
        };
        Ok(SurfaceEvaluation {
            position: pos,
            du,
            dv,
            duu,
            duv,
            dvv,
        })
    }

    fn project_into(
        &self,
        point: Point3,
        _tolerance: &ToleranceContext,
        output: &mut Vec<SurfaceProjection>,
    ) -> Result<(), GeometryError> {
        output.clear();
        let q = point.into_array();
        let ap = self.apex.into_array();
        let a = self.axis.into_array();
        let x = self.x_axis.into_array();
        let y = cross3(a, x);
        let alpha = self.half_angle;
        let (cos_a, sin_a) = (alpha.cos(), alpha.sin());

        let d = sub3(q, ap);
        let h = dot3(d, a); // axial component
        let radial_vec = sub3(d, scale3(a, h)); // radial component vector
        let r = mag3(radial_vec);

        if r == 0.0 {
            // Point is on the cone axis; every u is equidistant.
            return Err(GeometryError::Singular);
        }

        // Angular parameter from the radial direction.
        let u_val = angle_to_full_turn(dot3(radial_vec, y).atan2(dot3(radial_vec, x)));

        // Project (h, r) in the axial-radial plane onto each nappe.
        // Nappe 1 (v > 0): unit direction (cos α, sin α)
        // Nappe 2 (v < 0): unit direction (−cos α, sin α)
        let t1 = h * cos_a + r * sin_a; // projection parameter on nappe 1
        let t2 = -h * cos_a + r * sin_a; // projection parameter on nappe 2

        let mut push_nappe = |t: f64, sign: f64| -> Result<(), GeometryError> {
            // v = sign * t * cos_a  (sign distinguishes nappes)
            let v_val = sign * t * cos_a;
            let (cos_u, sin_u) = (u_val.cos(), u_val.sin());
            let radial_dir = add3(scale3(x, cos_u), scale3(y, sin_u));
            let proj_arr = add3(
                ap,
                add3(scale3(a, v_val), scale3(radial_dir, v_val * alpha.tan())),
            );
            let proj = Point3::try_new(proj_arr[0], proj_arr[1], proj_arr[2]).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "cone projection point is non-finite".to_owned(),
                }
            })?;
            // Numerically stable perpendicular distance in the axial-radial half-plane.
            // For nappe 1 (sign=+1): dist = |h·sin α − r·cos α|
            // For nappe 2 (sign=−1): dist = |h·sin α + r·cos α|
            // Both collapse to zero for points already on the respective nappe,
            // avoiding the catastrophic cancellation in sqrt(|d|²−t²).
            let dist = (h * sin_a - sign * r * cos_a).abs();
            output.push(SurfaceProjection {
                u: ParameterValue::try_new(u_val).map_err(|_| GeometryError::Uncertified {
                    reason: "cone u is non-finite".to_owned(),
                })?,
                v: ParameterValue::try_new(v_val).map_err(|_| GeometryError::Uncertified {
                    reason: "cone v is non-finite".to_owned(),
                })?,
                point: proj,
                distance_bound: DistanceBound::try_new(dist).map_err(|_| {
                    GeometryError::Uncertified {
                        reason: "cone distance is non-finite or negative".to_owned(),
                    }
                })?,
            });
            Ok(())
        };

        match (t1 > 0.0, t2 > 0.0) {
            (true, true) => {
                // t1 == t2 iff h == 0: equatorial point, both nappes equidistant.
                if t1 >= t2 {
                    push_nappe(t1, 1.0)?;
                }
                if t2 >= t1 {
                    push_nappe(t2, -1.0)?;
                }
            }
            (true, false) => push_nappe(t1, 1.0)?,
            (false, true) => push_nappe(t2, -1.0)?,
            (false, false) => {
                // Both projections land at the apex (impossible for r > 0).
                return Err(GeometryError::Singular);
            }
        }
        Ok(())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

    use amphion_foundation::{Point3, ToleranceContext, Vector3};

    use crate::traits::SurfaceEvaluator;
    use crate::{DerivativeOrder, GeometryError};

    use super::{Cone, ConstructionError};

    fn tol() -> ToleranceContext {
        ToleranceContext::try_new(1e-9, 1e-8, 1e-10, 1e-12).unwrap()
    }

    fn std_cone() -> Cone {
        Cone::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            FRAC_PI_4,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn cone_construction_valid() {
        assert_eq!(std_cone().half_angle(), FRAC_PI_4);
    }

    #[test]
    fn cone_construction_rejects_zero_half_angle() {
        let err = Cone::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            0.0,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::InvalidHalfAngle);
    }

    #[test]
    fn cone_construction_rejects_half_angle_equals_pi_over_2() {
        let err = Cone::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            FRAC_PI_2,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::InvalidHalfAngle);
    }

    #[test]
    fn cone_construction_rejects_degenerate_axis() {
        let err = Cone::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 0.0).unwrap(),
            FRAC_PI_4,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::DegenerateAxis);
    }

    #[test]
    fn cone_construction_rejects_dependent_axes() {
        let err = Cone::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            FRAC_PI_4,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::DependentAxes);
    }

    #[test]
    fn cone_evaluate_apex_position() {
        // At v=0, position is the apex regardless of u.
        let c = std_cone();
        for u in [0.0_f64, 1.0, 2.0, PI, 5.0] {
            let ev = c.evaluate(u, 0.0, DerivativeOrder::Position).unwrap();
            let p = ev.position.into_array();
            assert!(
                p.iter().all(|v| v.abs() < 1e-14),
                "u={u} apex not at origin, got {p:?}"
            );
        }
    }

    #[test]
    fn cone_evaluate_apex_derivative_singular() {
        let c = std_cone();
        // First and second order at the apex are singular.
        assert_eq!(
            c.evaluate(0.0, 0.0, DerivativeOrder::First),
            Err(GeometryError::Singular)
        );
        assert_eq!(
            c.evaluate(0.0, 0.0, DerivativeOrder::Second),
            Err(GeometryError::Singular)
        );
    }

    #[test]
    fn cone_evaluate_position_at_known_point() {
        // For α = π/4, tan(α) = 1. At u=0, v=1: p = (0,0,0) + (0,0,1) + (1,0,0) = (1,0,1).
        let c = std_cone();
        let ev = c.evaluate(0.0, 1.0, DerivativeOrder::Position).unwrap();
        assert!((ev.position.x() - 1.0).abs() < 1e-13);
        assert!((ev.position.y()).abs() < 1e-13);
        assert!((ev.position.z() - 1.0).abs() < 1e-13);
    }

    #[test]
    fn cone_evaluate_derivatives_at_known_angle() {
        // α = π/4, u=0, v=1:
        // du = v·tan(α)·(−sin0·x + cos0·y) = 1·1·(0,0,0)+(0,1,0) = (0,1,0)
        // dv = a + tan(α)·(cos0·x + sin0·y) = (0,0,1)+(1,0,0) = (1,0,1)
        let c = std_cone();
        let ev = c.evaluate(0.0, 1.0, DerivativeOrder::Second).unwrap();
        let du = ev.du.unwrap().into_array();
        let dv = ev.dv.unwrap().into_array();
        assert!(
            du[0].abs() < 1e-13 && (du[1] - 1.0).abs() < 1e-13 && du[2].abs() < 1e-13,
            "du={du:?}"
        );
        assert!(
            (dv[0] - 1.0).abs() < 1e-13 && dv[1].abs() < 1e-13 && (dv[2] - 1.0).abs() < 1e-13,
            "dv={dv:?}"
        );
    }

    #[test]
    fn cone_evaluate_fd_check() {
        let c = std_cone();
        let h = 1e-7_f64;
        for u in [0.3_f64, 1.5, 4.0] {
            let p_plus = c
                .evaluate(u + h, 2.0, DerivativeOrder::Position)
                .unwrap()
                .position
                .into_array();
            let p_minus = c
                .evaluate(u - h, 2.0, DerivativeOrder::Position)
                .unwrap()
                .position
                .into_array();
            let fd_du: [f64; 3] = std::array::from_fn(|i| (p_plus[i] - p_minus[i]) / (2.0 * h));
            let analytic_du = c
                .evaluate(u, 2.0, DerivativeOrder::First)
                .unwrap()
                .du
                .unwrap()
                .into_array();
            for i in 0..3 {
                assert!(
                    (fd_du[i] - analytic_du[i]).abs() < 1e-5,
                    "u={u} component {i}: fd={} analytic={}",
                    fd_du[i],
                    analytic_du[i]
                );
            }
        }
    }

    #[test]
    fn cone_evaluate_fd_dv_check() {
        let c = std_cone();
        let h = 1e-7_f64;
        for v in [0.5_f64, 1.0, 3.0] {
            let p_plus = c
                .evaluate(1.0, v + h, DerivativeOrder::Position)
                .unwrap()
                .position
                .into_array();
            let p_minus = c
                .evaluate(1.0, v - h, DerivativeOrder::Position)
                .unwrap()
                .position
                .into_array();
            let fd_dv: [f64; 3] = std::array::from_fn(|i| (p_plus[i] - p_minus[i]) / (2.0 * h));
            let analytic_dv = c
                .evaluate(1.0, v, DerivativeOrder::First)
                .unwrap()
                .dv
                .unwrap()
                .into_array();
            for i in 0..3 {
                assert!(
                    (fd_dv[i] - analytic_dv[i]).abs() < 1e-5,
                    "v={v} component {i}"
                );
            }
        }
    }

    #[test]
    fn cone_second_derivative_duu_identity() {
        // duu = −v·tan(α)·(cos u·x + sin u·y) = −du evaluated at the radial part.
        // Another identity: duu = −(p − apex − v·axis) / v^2 * v  ... simplifies to −radial_part.
        // Let's just FD-check duu.
        let c = std_cone();
        let h = 1e-5_f64;
        for u in [0.5_f64, 2.0, 5.0] {
            let du_plus = c
                .evaluate(u + h, 1.0, DerivativeOrder::First)
                .unwrap()
                .du
                .unwrap()
                .into_array();
            let du_minus = c
                .evaluate(u - h, 1.0, DerivativeOrder::First)
                .unwrap()
                .du
                .unwrap()
                .into_array();
            let fd_duu: [f64; 3] = std::array::from_fn(|i| (du_plus[i] - du_minus[i]) / (2.0 * h));
            let analytic_duu = c
                .evaluate(u, 1.0, DerivativeOrder::Second)
                .unwrap()
                .duu
                .unwrap()
                .into_array();
            for i in 0..3 {
                assert!(
                    (fd_duu[i] - analytic_duu[i]).abs() < 1e-4,
                    "u={u} duu component {i}: fd={} analytic={}",
                    fd_duu[i],
                    analytic_duu[i]
                );
            }
        }
    }

    #[test]
    fn cone_periodic_equivalence() {
        let c = std_cone();
        let eps = 1e-12_f64;
        let p0 = c
            .evaluate(0.0, 2.0, DerivativeOrder::Position)
            .unwrap()
            .position
            .into_array();
        let p_near = c
            .evaluate(TAU - eps, 2.0, DerivativeOrder::Position)
            .unwrap()
            .position
            .into_array();
        let d = (0..3)
            .map(|i| (p0[i] - p_near[i]).powi(2))
            .sum::<f64>()
            .sqrt();
        assert!(d < 1e-10, "seam continuity distance={d}");
    }

    #[test]
    fn cone_projection_round_trip_nappe1() {
        let c = std_cone();
        for (u0, v0) in [(0.0_f64, 1.0), (1.0, 2.0), (PI, 0.5), (5.0, 3.0)] {
            let pt = c
                .evaluate(u0, v0, DerivativeOrder::Position)
                .unwrap()
                .position;
            let projs = c.project(pt, &tol()).unwrap();
            assert!(!projs.is_empty(), "u={u0} v={v0}");
            // Find the projection with v closest to v0.
            let best = projs
                .iter()
                .min_by(|a, b| {
                    (a.v.get() - v0)
                        .abs()
                        .partial_cmp(&(b.v.get() - v0).abs())
                        .unwrap()
                })
                .unwrap();
            assert!((best.u.get() - u0).abs() < 1e-10, "u round-trip u={u0}");
            assert!((best.v.get() - v0).abs() < 1e-10, "v round-trip v={v0}");
            assert!(best.distance_bound.get() < 1e-10);
        }
    }

    #[test]
    fn cone_projection_equatorial_returns_two() {
        // Point in the equatorial plane (h = 0) projects to both nappes equally.
        let c = std_cone();
        // p at u=0, v=1 is (1, 0, 1). Radial at h=0, r=1:
        // project (0, 1) to nappe1 t1 = 0*cos(π/4) + 1*sin(π/4) = 1/√2 > 0
        // t2 = 0*cos(π/4) + 1*sin(π/4) = 1/√2 > 0 and t1==t2
        let q = Point3::try_new(1.0, 0.0, 0.0).unwrap(); // h=0, r=1
        let projs = c.project(q, &tol()).unwrap();
        assert_eq!(projs.len(), 2, "equatorial point should give 2 projections");
        // Both should have same distance bound.
        let d0 = projs[0].distance_bound.get();
        let d1 = projs[1].distance_bound.get();
        assert!(
            (d0 - d1).abs() < 1e-12,
            "both projections should be equidistant"
        );
    }

    #[test]
    fn cone_projection_singular_on_axis() {
        let c = std_cone();
        let q = Point3::try_new(0.0, 0.0, 5.0).unwrap();
        assert_eq!(c.project(q, &tol()), Err(GeometryError::Singular));
    }

    #[test]
    fn cone_evaluate_rejects_out_of_domain() {
        let c = std_cone();
        assert_eq!(
            c.evaluate(-0.001, 1.0, DerivativeOrder::Position),
            Err(GeometryError::OutsideDomain)
        );
        assert_eq!(
            c.evaluate(TAU, 1.0, DerivativeOrder::Position),
            Err(GeometryError::OutsideDomain)
        );
    }

    #[test]
    fn cone_evaluate_rejects_non_finite() {
        let c = std_cone();
        assert_eq!(
            c.evaluate(f64::NAN, 1.0, DerivativeOrder::Position),
            Err(GeometryError::NonFiniteParameter)
        );
        assert_eq!(
            c.evaluate(0.0, f64::INFINITY, DerivativeOrder::Position),
            Err(GeometryError::NonFiniteParameter)
        );
    }

    #[test]
    fn cone_serde_round_trip() {
        let c = Cone::try_new(
            Point3::try_new(1.0, 2.0, 3.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            0.5,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let json = serde_json::to_string(&c).unwrap();
        let decoded: Cone = serde_json::from_str(&json).unwrap();
        assert_eq!(c, decoded);
    }
}
