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

use std::f64::consts::{PI, TAU};

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
        ILL_COND_THRESH, add3, all_finite3, angle_to_full_turn, certified_distance_bound3,
        certify_h_sign, cross3, dot3, in_range, mag3, normalize3, scale3, sub3,
        validate_orthogonal3, validate_unit3,
    },
};

fn angular_range() -> ParameterRange {
    ParameterRange::try_new(Some(0.0), Some(TAU), Some(TAU))
        .unwrap_or_else(|_| unreachable!("angular [0, 2π) domain is always valid"))
}

fn unbounded_range() -> ParameterRange {
    ParameterRange::try_new(None, None, None)
        .unwrap_or_else(|_| unreachable!("unbounded domain is always valid"))
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
    /// - [`ConstructionError::IllConditionedAxes`] — `x_axis` nearly parallel
    ///   to `axis`
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
        let perp_mag = mag3(x_perp);
        if perp_mag == 0.0 {
            return Err(ConstructionError::DependentAxes);
        }
        if perp_mag < ILL_COND_THRESH {
            return Err(ConstructionError::IllConditionedAxes);
        }
        let x_unit = normalize3(x_perp).ok_or(ConstructionError::DependentAxes)?;
        Ok(Self {
            apex: Point3::try_new(ap[0], ap[1], ap[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            axis: Vector3::try_new(a_unit[0], a_unit[1], a_unit[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            half_angle,
            x_axis: Vector3::try_new(x_unit[0], x_unit[1], x_unit[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
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
        Vector3::try_new(y[0], y[1], y[2]).unwrap_or_else(|_| {
            unreachable!("cross product of stored orthonormal pair is always a unit vector")
        })
    }
}

impl TryFrom<ConeRepr> for Cone {
    type Error = ConstructionError;
    fn try_from(repr: ConeRepr) -> Result<Self, Self::Error> {
        let apex = repr.apex.into_array();
        let axis = repr.axis.into_array();
        let x_axis = repr.x_axis.into_array();
        if !all_finite3(apex)
            || !all_finite3(axis)
            || !repr.half_angle.is_finite()
            || !all_finite3(x_axis)
        {
            return Err(ConstructionError::NonFiniteInput);
        }
        if repr.half_angle <= 0.0 || repr.half_angle >= std::f64::consts::FRAC_PI_2 {
            return Err(ConstructionError::InvalidHalfAngle);
        }
        validate_unit3(axis)?;
        validate_unit3(x_axis)?;
        validate_orthogonal3(axis, x_axis)?;
        Ok(Self {
            apex: repr.apex,
            axis: repr.axis,
            half_angle: repr.half_angle,
            x_axis: repr.x_axis,
        })
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
        tolerance: &ToleranceContext,
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
        let d_mag = mag3(d);
        let radial_vec = sub3(d, scale3(a, h)); // radial component vector
        let r = mag3(radial_vec);

        let eff_tol = tolerance
            .effective_length(d_mag)
            .unwrap_or_else(|_| tolerance.absolute_length());
        if r < eff_tol {
            return Err(GeometryError::Singular);
        }

        // Angular parameter from the radial direction.
        let u_val = angle_to_full_turn(dot3(radial_vec, y).atan2(dot3(radial_vec, x)));

        // Project (h, r) in the axial-radial plane onto each nappe.
        // Nappe 1 (v > 0): unit direction (cos α, sin α)
        // Nappe 2 (v < 0): unit direction (−cos α, sin α)
        let t1 = h * cos_a + r * sin_a; // projection parameter on nappe 1
        let t2 = -h * cos_a + r * sin_a; // projection parameter on nappe 2

        // Build a candidate on one nappe without touching `output` yet.
        let project_nappe = |t: f64, sign: f64| -> Result<SurfaceProjection, GeometryError> {
            let v_val = sign * t * cos_a;
            let u_proj = if sign.is_sign_positive() {
                u_val
            } else {
                angle_to_full_turn(u_val + PI)
            };
            let (cos_u, sin_u) = (u_proj.cos(), u_proj.sin());
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
            let dist = certified_distance_bound3(ap, q, proj_arr, tolerance)?;
            Ok(SurfaceProjection {
                u: ParameterValue::try_new(u_proj).map_err(|_| GeometryError::Uncertified {
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
            })
        };

        // Construct all candidates on the stack, validate both before any push.
        let candidates: [Option<SurfaceProjection>; 2] = match (t1 > 0.0, t2 > 0.0) {
            (true, true) => {
                let cand1 = project_nappe(t1, 1.0)?;
                let cand2 = project_nappe(t2, -1.0)?;
                // Use certify_h_sign to decide which nappe is geometrically closer.
                // `h` is the axial component of `d = q − apex`.
                // Positive h → nappe 1 is closer; negative → nappe 2; uncertain → both.
                match certify_h_sign(h, d_mag) {
                    Some(core::cmp::Ordering::Greater) => [Some(cand1), None],
                    Some(core::cmp::Ordering::Less) => [None, Some(cand2)],
                    Some(core::cmp::Ordering::Equal) | None => {
                        // Equatorial plane or numerically ambiguous: return both.
                        // certify_h_sign returning None means the sign cannot be
                        // certified; rather than returning Uncertified (which would
                        // suppress valid projections), we conservatively include both.
                        [Some(cand1), Some(cand2)]
                    }
                }
            }
            (true, false) => [Some(project_nappe(t1, 1.0)?), None],
            (false, true) => [None, Some(project_nappe(t2, -1.0)?)],
            (false, false) => return Err(GeometryError::Singular),
        };

        // Collect valid candidates onto a fixed-size stack, sort, then push atomically.
        let mut sorted: [Option<SurfaceProjection>; 2] = [None, None];
        let mut count = 0;
        for c in candidates.into_iter().flatten() {
            sorted[count] = Some(c);
            count += 1;
        }
        sorted[..count].sort_by(|a_opt, b_opt| {
            let a_c = a_opt.as_ref().unwrap();
            let b_c = b_opt.as_ref().unwrap();
            a_c.u
                .get()
                .total_cmp(&b_c.u.get())
                .then(a_c.v.get().total_cmp(&b_c.v.get()))
        });
        for opt in &sorted[..count] {
            output.push(*opt.as_ref().unwrap());
        }
        Ok(())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, PI, TAU};

    use amphion_foundation::{Point3, ToleranceContext, Vector3};
    use serde_json::json;

    use crate::analytic::helpers::ILL_COND_THRESH;
    use crate::traits::SurfaceEvaluator;
    use crate::{DerivativeOrder, GeometryError};

    use super::{Cone, ConeRepr, ConstructionError};

    fn tol() -> ToleranceContext {
        ToleranceContext::try_new(1e-9, 1e-8, 1e-10, 1e-12).unwrap()
    }

    fn dist3(a: Point3, b: Point3) -> f64 {
        let [ax, ay, az] = a.into_array();
        let [bx, by, bz] = b.into_array();
        (ax - bx).hypot((ay - by).hypot(az - bz))
    }

    fn angle_error(actual: f64, expected: f64) -> f64 {
        let tau = TAU;
        let delta = (actual - expected).rem_euclid(tau);
        delta.min(tau - delta)
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
        // half_angle is stored exactly as passed; FRAC_PI_4 round-trips bit-for-bit.
        assert_eq!(std_cone().half_angle().to_bits(), FRAC_PI_4.to_bits());
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
    fn cone_construction_rejects_ill_conditioned_axes() {
        let err = Cone::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            FRAC_PI_4,
            Vector3::try_new(ILL_COND_THRESH / 2.0, 0.0, 1.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::IllConditionedAxes);
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
            let q = c
                .evaluate(u0, v0, DerivativeOrder::Position)
                .unwrap()
                .position;
            let proj = c.project(q, &tol()).unwrap().remove(0);
            assert!(angle_error(proj.u.get(), u0) < 1e-10, "u={u0}");
            assert!((proj.v.get() - v0).abs() < 1e-10, "v={v0}");
            assert!(dist3(q, proj.point) <= proj.distance_bound.get());
        }
    }

    #[test]
    fn cone_projection_round_trip_negative_nappe() {
        let c = std_cone();
        for u0 in [0.0_f64, FRAC_PI_2, PI, 3.0 * PI / 2.0] {
            for v0 in [-0.5_f64, -1.0, -3.0] {
                let q = c
                    .evaluate(u0, v0, DerivativeOrder::Position)
                    .unwrap()
                    .position;
                let proj = c
                    .project(q, &tol())
                    .unwrap()
                    .into_iter()
                    .find(|candidate| (candidate.v.get() - v0).abs() < 1e-10)
                    .unwrap();
                assert!(angle_error(proj.u.get(), u0) < 1e-10, "u={u0}");
                assert!((proj.v.get() - v0).abs() < 1e-10, "v={v0}");
                let eval = c
                    .evaluate(proj.u.get(), proj.v.get(), DerivativeOrder::Position)
                    .unwrap();
                assert!(dist3(eval.position, q) < 1e-11, "u={u0} v={v0}");
                assert!(proj.distance_bound.get() < 1e-10);
            }
        }
    }

    #[test]
    fn cone_projection_equatorial_returns_correct_two_points() {
        let c = std_cone();
        let q = Point3::try_new(1.0, 0.0, 0.0).unwrap();
        let projs = c.project(q, &tol()).unwrap();
        assert_eq!(projs.len(), 2);
        let sin_a = FRAC_PI_4.sin();
        let cos_a = FRAC_PI_4.cos();
        let expected = [
            [sin_a * sin_a, 0.0, sin_a * cos_a],
            [sin_a * sin_a, 0.0, -sin_a * cos_a],
        ];
        for (proj, expected_point) in projs.iter().zip(expected) {
            let actual = proj.point.into_array();
            for i in 0..3 {
                assert!((actual[i] - expected_point[i]).abs() < 1e-12);
            }
            assert!(dist3(q, proj.point) <= proj.distance_bound.get());
        }
        assert!(projs[0].u.get() < projs[1].u.get());
        assert!((projs[0].distance_bound.get() - projs[1].distance_bound.get()).abs() < 1e-9);
    }

    #[test]
    fn cone_projection_singular_on_axis() {
        let c = std_cone();
        let q = Point3::try_new(0.0, 0.0, 5.0).unwrap();
        assert_eq!(c.project(q, &tol()), Err(GeometryError::Singular));
        let near_axis = Point3::try_new(0.5e-9, 0.0, 5.0).unwrap();
        assert_eq!(c.project(near_axis, &tol()), Err(GeometryError::Singular));
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
            Vector3::try_new(
                1.0 / 3.0_f64.sqrt(),
                1.0 / 3.0_f64.sqrt(),
                1.0 / 3.0_f64.sqrt(),
            )
            .unwrap(),
            0.5,
            Vector3::try_new(1.0 / 2.0_f64.sqrt(), -1.0 / 2.0_f64.sqrt(), 0.0).unwrap(),
        )
        .unwrap();
        let json = serde_json::to_string(&c).unwrap();
        let decoded: Cone = serde_json::from_str(&json).unwrap();
        assert_eq!(c, decoded);
    }

    #[test]
    fn cone_projection_round_trip_negative_nappe_in_rotated_frame() {
        let c = Cone::try_new(
            Point3::try_new(1.0, -2.0, 0.5).unwrap(),
            Vector3::try_new(1.0, 1.0, 1.0).unwrap(),
            0.5,
            Vector3::try_new(1.0, -1.0, 0.0).unwrap(),
        )
        .unwrap();
        let u0 = PI / 3.0;
        let v0 = -2.0;
        let q = c
            .evaluate(u0, v0, DerivativeOrder::Position)
            .unwrap()
            .position;
        let proj = c
            .project(q, &tol())
            .unwrap()
            .into_iter()
            .find(|candidate| (candidate.v.get() - v0).abs() < 1e-10)
            .unwrap();
        assert!(angle_error(proj.u.get(), u0) < 1e-10);
        assert!((proj.v.get() - v0).abs() < 1e-10);
        assert!(dist3(q, proj.point) <= proj.distance_bound.get());
    }

    #[test]
    fn cone_distance_bounds_certify_actual_distance_at_extreme_scales() {
        let tiny_tol = ToleranceContext::try_new(1e-15, 1e-8, 1e-10, 1e-12).unwrap();
        let unit = std_cone();
        let tiny = Cone::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            FRAC_PI_4,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        for (cone, query, tolerance) in [
            (
                unit.clone(),
                unit.evaluate(0.25, 2.0, DerivativeOrder::Position)
                    .unwrap()
                    .position,
                tol(),
            ),
            (unit.clone(), Point3::try_new(2.0, 0.0, 3.0).unwrap(), tol()),
            (
                unit.clone(),
                Point3::try_new(1.0e12, 1.0, 1.0e12).unwrap(),
                tol(),
            ),
            (
                tiny.clone(),
                Point3::try_new(2.0e-12, 0.0, 1.0e-12).unwrap(),
                tiny_tol,
            ),
            (
                unit.clone(),
                Point3::try_new(1.0, 1.0e-12, 1.0 + 1.0e-12).unwrap(),
                tol(),
            ),
        ] {
            let projections = cone.project(query, &tolerance).unwrap();
            for projection in projections {
                let actual = dist3(query, projection.point);
                assert!(actual <= projection.distance_bound.get(), "{query:?}");
                assert!(projection.distance_bound.get() >= 0.0);
                assert!(projection.u.get() < TAU);
            }
        }
    }

    #[test]
    fn cone_projection_near_equator_returns_two_when_distances_tie() {
        let c = std_cone();
        // Exactly on the equatorial plane: h = 0, certify_h_sign = Equal → both nappes.
        let q_exact = Point3::try_new(1.0, 0.0, 0.0).unwrap();
        let projs_exact = c.project(q_exact, &tol()).unwrap();
        assert_eq!(
            projs_exact.len(),
            2,
            "equatorial h=0 must return both nappes"
        );

        // Below FP threshold: h = 1e-300 < 8ε·d_mag ≈ 1.8e-15 → certify_h_sign = None → both.
        let q_tiny = Point3::try_new(1.0, 0.0, 1e-300).unwrap();
        let projs_tiny = c.project(q_tiny, &tol()).unwrap();
        assert_eq!(
            projs_tiny.len(),
            2,
            "sub-threshold h should return both nappes"
        );

        // Certified positive h: h = 1e-12 >> 8ε·d_mag ≈ 1.8e-15 → only nappe 1 returned.
        let q_cert = Point3::try_new(1.0, 0.0, 1.0e-12).unwrap();
        let projs_cert = c.project(q_cert, &tol()).unwrap();
        assert_eq!(
            projs_cert.len(),
            1,
            "certified h > 0 should return only nappe 1"
        );
        assert!(projs_cert[0].v.get() > 0.0, "nappe 1 has positive v");
    }

    #[test]
    fn cone_projection_handles_apex_adjacent_queries() {
        let c = std_cone();
        let q = Point3::try_new(1.0e-6, 0.0, 1.0e-9).unwrap();
        let projs = c.project(q, &tol()).unwrap();
        assert!(!projs.is_empty());
        for projection in projs {
            assert!(projection.distance_bound.get() >= 0.0);
        }
    }

    #[test]
    fn cone_project_into_clears_output_on_error() {
        let c = std_cone();
        let mut output = c
            .project(Point3::try_new(1.0, 0.0, 1.0).unwrap(), &tol())
            .unwrap();
        let err = c.project_into(Point3::try_new(0.0, 0.0, 5.0).unwrap(), &tol(), &mut output);
        assert_eq!(err, Err(GeometryError::Singular));
        assert!(output.is_empty());
    }

    #[test]
    fn cone_projection_seam_stays_in_range() {
        let c = std_cone();
        let eps = 1e-12_f64;
        for u in [TAU - eps, eps] {
            let q = c
                .evaluate(u, 2.0, DerivativeOrder::Position)
                .unwrap()
                .position;
            let projection = c.project(q, &tol()).unwrap().remove(0);
            assert!(projection.u.get() >= 0.0);
            assert!(projection.u.get() < TAU);
        }
    }

    #[test]
    fn cone_serde_rejects_bad_axis_angle_and_orthogonality() {
        let bad_axis: ConeRepr = serde_json::from_value(json!({
            "apex": [1.0, 2.0, 3.0],
            "axis": [2.0, 0.0, 0.0],
            "half_angle": 0.5,
            "x_axis": [0.0, 1.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Cone::try_from(bad_axis),
            Err(ConstructionError::DegenerateAxis)
        );

        let bad_frame: ConeRepr = serde_json::from_value(json!({
            "apex": [1.0, 2.0, 3.0],
            "axis": [1.0, 0.0, 0.0],
            "half_angle": 0.5,
            "x_axis": [1.0, 0.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Cone::try_from(bad_frame),
            Err(ConstructionError::DependentAxes)
        );

        let bad_angle: ConeRepr = serde_json::from_value(json!({
            "apex": [1.0, 2.0, 3.0],
            "axis": [0.0, 0.0, 1.0],
            "half_angle": 0.0,
            "x_axis": [1.0, 0.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Cone::try_from(bad_angle),
            Err(ConstructionError::InvalidHalfAngle)
        );
    }

    #[test]
    fn cone_serde_rejects_nan_and_inf_fields() {
        assert!(serde_json::from_str::<Cone>(
            r#"{"apex":[NaN,0.0,0.0],"axis":[0.0,0.0,1.0],"half_angle":0.5,"x_axis":[1.0,0.0,0.0]}"#
        )
        .is_err());
        assert!(serde_json::from_str::<Cone>(
            r#"{"apex":[0.0,0.0,0.0],"axis":[Infinity,0.0,1.0],"half_angle":0.5,"x_axis":[1.0,0.0,0.0]}"#
        )
        .is_err());
    }
}
