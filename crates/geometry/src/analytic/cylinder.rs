//! Analytic right circular cylinder surface.
//!
//! # Parameterization
//!
//! ```text
//! p(u, v) = axis_origin + v·axis_dir + r·cos(u)·x_axis + r·sin(u)·y_axis
//! ```
//!
//! where `y_axis = axis_dir × x_axis`.  `axis_dir`, `x_axis`, and `y_axis`
//! form a right-handed orthonormal frame.
//!
//! - U domain: `[0, 2π)` with period `2π` (angular, CCW around `axis_dir`)
//! - V domain: `(−∞, +∞)` (axial)
//!
//! Derivatives:
//! ```text
//! ∂p/∂u  =  r·(−sin u·x_axis + cos u·y_axis)
//! ∂p/∂v  =  axis_dir
//! ∂²p/∂u²  =  −r·(cos u·x_axis + sin u·y_axis)
//! ∂²p/∂u∂v  =  0
//! ∂²p/∂v²   =  0
//! ```
//!
//! Projection: decompose `q − axis_origin` into axial and radial components;
//! `v` is the axial component and `u` is the angle of the radial direction.
//! Returns [`GeometryError::Singular`] when the radial component is exactly
//! zero (point on the cylinder axis).

#![allow(
    clippy::many_single_char_names,
    clippy::missing_panics_doc,
    clippy::similar_names
)]

use std::f64::consts::TAU;

use amphion_foundation::{Point3, ToleranceContext, Transform3, Vector3};
use serde::{Deserialize, Serialize};

use crate::traits::SurfaceEvaluator;
use crate::{
    DerivativeOrder, DistanceBound, GeometryError, ParameterRange, ParameterValue, SurfaceDomain,
    SurfaceEvaluation, SurfaceKind, SurfaceProjection,
};

use super::{
    ConstructionError, TransformError,
    helpers::{
        ILL_COND_THRESH, add3, all_finite3, angle_to_full_turn, cross3, dot3, in_range, mag3,
        normalize3, projection_distance_bound3, scale3, sub3, validate_orthogonal3, validate_unit3,
    },
    transform::{apply_to_point, apply_to_vector, similarity_scale},
};

fn angular_range() -> ParameterRange {
    // (0.0, TAU, TAU) is a compile-time constant with lo < hi; this is not
    // an input-dependent path, so a static-invariant `expect` is acceptable
    // here (see CONTRACTS.md).
    ParameterRange::try_new(Some(0.0), Some(TAU), Some(TAU))
        .expect("angular [0, 2π) domain is always valid")
}

fn unbounded_range() -> ParameterRange {
    // (None, None, None) is a compile-time constant and always valid; this
    // is not an input-dependent path, so a static-invariant `expect` is
    // acceptable here (see CONTRACTS.md).
    ParameterRange::try_new(None, None, None).expect("unbounded domain is always valid")
}

#[derive(Serialize, Deserialize)]
struct CylinderRepr {
    axis_origin: Point3,
    axis_dir: Vector3,
    radius: f64,
    x_axis: Vector3,
}

/// A right circular cylinder surface.
///
/// Parameterization:
/// ```text
/// p(u, v) = axis_origin + v·axis_dir + r·cos(u)·x_axis + r·sin(u)·y_axis
/// ```
/// U ∈ `[0, 2π)` (periodic), V ∈ `(−∞, +∞)` (axial).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "CylinderRepr", into = "CylinderRepr")]
pub struct Cylinder {
    axis_origin: Point3,
    axis_dir: Vector3,
    radius: f64,
    x_axis: Vector3,
    y_axis: Vector3,
}

impl Cylinder {
    /// Constructs a cylinder.
    ///
    /// `axis_dir` and `x_axis` are normalized internally.  `x_axis` is
    /// orthogonalized against `axis_dir` (Gram-Schmidt).
    ///
    /// # Errors
    ///
    /// - [`ConstructionError::NonFiniteInput`] — any NaN/Inf input
    /// - [`ConstructionError::DegenerateAxis`] — zero-length axis or x-axis
    /// - [`ConstructionError::NotPositive`] — `radius <= 0`
    /// - [`ConstructionError::DependentAxes`] — `x_axis` parallel to `axis_dir`
    /// - [`ConstructionError::IllConditionedAxes`] — `x_axis` nearly parallel
    ///   to `axis_dir`
    pub fn try_new(
        axis_origin: Point3,
        axis_dir: Vector3,
        radius: f64,
        x_axis: Vector3,
    ) -> Result<Self, ConstructionError> {
        let o = axis_origin.into_array();
        let a = axis_dir.into_array();
        let x = x_axis.into_array();
        if !all_finite3(o) || !all_finite3(a) || !radius.is_finite() || !all_finite3(x) {
            return Err(ConstructionError::NonFiniteInput);
        }
        if radius <= 0.0 {
            return Err(ConstructionError::NotPositive);
        }
        let a_unit = normalize3(a).ok_or(ConstructionError::DegenerateAxis)?;
        let x_norm = normalize3(x).ok_or(ConstructionError::DegenerateAxis)?;
        // Orthogonalize x against axis_dir.
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
        let y_arr = cross3(a_unit, x_unit);
        Ok(Self {
            axis_origin: Point3::try_new(o[0], o[1], o[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            axis_dir: Vector3::try_new(a_unit[0], a_unit[1], a_unit[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            radius,
            x_axis: Vector3::try_new(x_unit[0], x_unit[1], x_unit[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            y_axis: Vector3::try_new(y_arr[0], y_arr[1], y_arr[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
        })
    }

    /// Returns the axis origin.
    #[must_use]
    pub fn axis_origin(&self) -> Point3 {
        self.axis_origin
    }

    /// Returns the unit axis direction.
    #[must_use]
    pub fn axis_dir(&self) -> Vector3 {
        self.axis_dir
    }

    /// Returns the radius.
    #[must_use]
    pub fn radius(&self) -> f64 {
        self.radius
    }

    /// Returns the unit reference direction for `u = 0`.
    #[must_use]
    pub fn x_axis(&self) -> Vector3 {
        self.x_axis
    }

    /// Returns the unit y-axis: `axis_dir × x_axis`.
    #[must_use]
    pub fn y_axis(&self) -> Vector3 {
        self.y_axis
    }

    /// Applies a similarity `transform` (rigid motion plus uniform scale, no
    /// reflection) to this cylinder, returning a new cylinder whose radius
    /// is scaled accordingly.
    ///
    /// A general affine transform does not map a circular cylinder to a
    /// circular cylinder, so only similarity transforms are accepted; see
    /// the `transform` module documentation for the (provisional, heuristic)
    /// similarity test.
    ///
    /// # Errors
    ///
    /// - [`TransformError::NotSimilarity`] — the transform's linear part is
    ///   not (within tolerance) a uniform-scale rotation
    /// - [`TransformError::NonFiniteResult`] — the transformed axis origin
    ///   or axes contain a non-finite component
    /// - [`TransformError::DegenerateResult`] — the transformed axes or
    ///   scaled radius fail cylinder construction
    pub fn try_transform(&self, transform: &Transform3) -> Result<Self, TransformError> {
        let m = transform.into_row_major();
        let scale = similarity_scale(m).ok_or(TransformError::NotSimilarity)?;
        let o = apply_to_point(m, self.axis_origin.into_array())
            .ok_or(TransformError::NonFiniteResult)?;
        let a = apply_to_vector(m, self.axis_dir.into_array())
            .ok_or(TransformError::NonFiniteResult)?;
        let x =
            apply_to_vector(m, self.x_axis.into_array()).ok_or(TransformError::NonFiniteResult)?;
        let new_radius = self.radius * scale;
        Self::try_new(
            Point3::try_new(o[0], o[1], o[2]).map_err(|_| TransformError::NonFiniteResult)?,
            Vector3::try_new(a[0], a[1], a[2]).map_err(|_| TransformError::NonFiniteResult)?,
            new_radius,
            Vector3::try_new(x[0], x[1], x[2]).map_err(|_| TransformError::NonFiniteResult)?,
        )
        .map_err(|_| TransformError::DegenerateResult)
    }
}

impl TryFrom<CylinderRepr> for Cylinder {
    type Error = ConstructionError;
    fn try_from(repr: CylinderRepr) -> Result<Self, Self::Error> {
        let axis_origin = repr.axis_origin.into_array();
        let axis_dir = repr.axis_dir.into_array();
        let x_axis = repr.x_axis.into_array();
        if !all_finite3(axis_origin)
            || !all_finite3(axis_dir)
            || !repr.radius.is_finite()
            || !all_finite3(x_axis)
        {
            return Err(ConstructionError::NonFiniteInput);
        }
        if repr.radius <= 0.0 {
            return Err(ConstructionError::NotPositive);
        }
        validate_unit3(axis_dir)?;
        validate_unit3(x_axis)?;
        validate_orthogonal3(axis_dir, x_axis)?;
        let y_arr = cross3(axis_dir, x_axis);
        Ok(Self {
            axis_origin: repr.axis_origin,
            axis_dir: repr.axis_dir,
            radius: repr.radius,
            x_axis: repr.x_axis,
            y_axis: Vector3::try_new(y_arr[0], y_arr[1], y_arr[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
        })
    }
}

impl From<Cylinder> for CylinderRepr {
    fn from(c: Cylinder) -> Self {
        Self {
            axis_origin: c.axis_origin,
            axis_dir: c.axis_dir,
            radius: c.radius,
            x_axis: c.x_axis,
        }
    }
}

impl SurfaceEvaluator for Cylinder {
    fn kind(&self) -> SurfaceKind {
        SurfaceKind::Cylinder
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
        // v is unbounded; finiteness already checked above.
        let o = self.axis_origin.into_array();
        let a = self.axis_dir.into_array();
        let x = self.x_axis.into_array();
        let y = self.y_axis.into_array();
        let r = self.radius;
        let (cos_u, sin_u) = (u.cos(), u.sin());

        // p = axis_origin + v·a + r·cos(u)·x + r·sin(u)·y
        let radial = add3(scale3(x, r * cos_u), scale3(y, r * sin_u));
        let pos_arr = add3(add3(o, scale3(a, v)), radial);
        let pos = Point3::try_new(pos_arr[0], pos_arr[1], pos_arr[2]).map_err(|_| {
            GeometryError::Uncertified {
                reason: "cylinder position is non-finite".to_owned(),
            }
        })?;

        let (du, dv, duu, duv, dvv) = match order {
            DerivativeOrder::Position => (None, None, None, None, None),
            DerivativeOrder::First => {
                let du_arr = add3(scale3(x, -r * sin_u), scale3(y, r * cos_u));
                let du = Vector3::try_new(du_arr[0], du_arr[1], du_arr[2]).map_err(|_| {
                    GeometryError::Uncertified {
                        reason: "cylinder du non-finite".to_owned(),
                    }
                })?;
                let dv =
                    Vector3::try_new(a[0], a[1], a[2]).map_err(|_| GeometryError::Uncertified {
                        reason: "cylinder dv non-finite".to_owned(),
                    })?;
                (Some(du), Some(dv), None, None, None)
            }
            DerivativeOrder::Second => {
                let du_arr = add3(scale3(x, -r * sin_u), scale3(y, r * cos_u));
                let du = Vector3::try_new(du_arr[0], du_arr[1], du_arr[2]).map_err(|_| {
                    GeometryError::Uncertified {
                        reason: "cylinder du non-finite".to_owned(),
                    }
                })?;
                let dv =
                    Vector3::try_new(a[0], a[1], a[2]).map_err(|_| GeometryError::Uncertified {
                        reason: "cylinder dv non-finite".to_owned(),
                    })?;
                let duu_arr = add3(scale3(x, -r * cos_u), scale3(y, -r * sin_u));
                let duu = Vector3::try_new(duu_arr[0], duu_arr[1], duu_arr[2]).map_err(|_| {
                    GeometryError::Uncertified {
                        reason: "cylinder duu non-finite".to_owned(),
                    }
                })?;
                let zero =
                    Vector3::try_new(0.0, 0.0, 0.0).map_err(|_| GeometryError::Uncertified {
                        reason: "zero vector construction failed unexpectedly".to_owned(),
                    })?;
                (Some(du), Some(dv), Some(duu), Some(zero), Some(zero))
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
        let o = self.axis_origin.into_array();
        let a = self.axis_dir.into_array();
        let x = self.x_axis.into_array();
        let y = self.y_axis.into_array();
        let r = self.radius;
        let diff = sub3(q, o);
        let v_val = dot3(diff, a);
        let radial_vec = sub3(diff, scale3(a, v_val));
        let radial_len = mag3(radial_vec);
        let eff_tol = tolerance
            .effective_length(r)
            .unwrap_or_else(|_| tolerance.absolute_length());
        if radial_len < eff_tol {
            return Err(GeometryError::Singular);
        }
        let u_val = angle_to_full_turn(dot3(radial_vec, y).atan2(dot3(radial_vec, x)));
        let (cos_u, sin_u) = (u_val.cos(), u_val.sin());
        let proj_arr = add3(
            add3(o, scale3(a, v_val)),
            add3(scale3(x, r * cos_u), scale3(y, r * sin_u)),
        );
        let proj = Point3::try_new(proj_arr[0], proj_arr[1], proj_arr[2]).map_err(|_| {
            GeometryError::Uncertified {
                reason: "cylinder projection point is non-finite".to_owned(),
            }
        })?;
        let local = SurfaceProjection {
            u: ParameterValue::try_new(u_val).map_err(|_| GeometryError::Uncertified {
                reason: "cylinder projection u is non-finite".to_owned(),
            })?,
            v: ParameterValue::try_new(v_val).map_err(|_| GeometryError::Uncertified {
                reason: "cylinder projection v is non-finite".to_owned(),
            })?,
            point: proj,
            distance_bound: DistanceBound::try_new(projection_distance_bound3(
                q, proj_arr, tolerance,
            )?)
            .map_err(|_| GeometryError::Uncertified {
                reason: "cylinder projection distance is non-finite or negative".to_owned(),
            })?,
        };
        output.push(local);
        Ok(())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, PI, TAU};

    use amphion_foundation::{Point3, ToleranceContext, Vector3};
    use serde_json::json;

    use crate::analytic::helpers::ILL_COND_THRESH;
    use crate::traits::SurfaceEvaluator;
    use crate::{DerivativeOrder, GeometryError};

    use super::{ConstructionError, Cylinder, CylinderRepr};

    fn tol() -> ToleranceContext {
        ToleranceContext::try_new(1e-9, 1e-8, 1e-10, 1e-12).unwrap()
    }

    fn dist3(a: Point3, b: Point3) -> f64 {
        let [ax, ay, az] = a.into_array();
        let [bx, by, bz] = b.into_array();
        (ax - bx).hypot((ay - by).hypot(az - bz))
    }

    fn unit_cylinder() -> Cylinder {
        Cylinder::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            1.0,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn cylinder_construction_valid() {
        assert!(unit_cylinder().radius() > 0.0);
    }

    #[test]
    fn cylinder_construction_rejects_zero_radius() {
        let err = Cylinder::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            0.0,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::NotPositive);
    }

    #[test]
    fn cylinder_construction_rejects_degenerate_axis() {
        let err = Cylinder::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 0.0).unwrap(),
            1.0,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::DegenerateAxis);
    }

    #[test]
    fn cylinder_construction_rejects_dependent_axes() {
        // x_axis parallel to axis_dir
        let err = Cylinder::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            1.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::DependentAxes);
    }

    #[test]
    fn cylinder_construction_rejects_ill_conditioned_axes() {
        let err = Cylinder::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            1.0,
            Vector3::try_new(ILL_COND_THRESH / 2.0, 0.0, 1.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::IllConditionedAxes);
    }

    #[test]
    fn cylinder_y_axis_right_handed() {
        // axis_dir=+Z, x_axis=+X → y_axis = Z×X = +Y
        let c = unit_cylinder();
        let y = c.y_axis().into_array();
        assert!((y[0]).abs() < 1e-14 && (y[1] - 1.0).abs() < 1e-14 && y[2].abs() < 1e-14);
    }

    #[test]
    fn cylinder_evaluate_position_at_cardinal_angles() {
        let c = unit_cylinder();
        let p0 = c
            .evaluate(0.0, 0.0, DerivativeOrder::Position)
            .unwrap()
            .position;
        let p90 = c
            .evaluate(FRAC_PI_2, 0.0, DerivativeOrder::Position)
            .unwrap()
            .position;
        let p_axial = c
            .evaluate(0.0, 5.0, DerivativeOrder::Position)
            .unwrap()
            .position;
        // u=0: (1, 0, 0)
        assert!((p0.x() - 1.0).abs() < 1e-13 && p0.y().abs() < 1e-13 && p0.z().abs() < 1e-13);
        // u=π/2: (0, 1, 0)
        assert!(p90.x().abs() < 1e-13 && (p90.y() - 1.0).abs() < 1e-13 && p90.z().abs() < 1e-13);
        // v=5: z=5
        assert!((p_axial.z() - 5.0).abs() < 1e-13);
    }

    #[test]
    fn cylinder_evaluate_derivatives_at_known_angle() {
        let c = unit_cylinder();
        let ev = c.evaluate(0.0, 0.0, DerivativeOrder::Second).unwrap();
        let du = ev.du.unwrap().into_array();
        let dv = ev.dv.unwrap().into_array();
        let duu = ev.duu.unwrap().into_array();
        // u=0: du = r*(−sin0·x + cos0·y) = (0,1,0)
        assert!(du[0].abs() < 1e-14 && (du[1] - 1.0).abs() < 1e-14 && du[2].abs() < 1e-14);
        // dv = axis_dir = (0,0,1)
        assert!(dv[0].abs() < 1e-14 && dv[1].abs() < 1e-14 && (dv[2] - 1.0).abs() < 1e-14);
        // duu = −r*(cos0·x + sin0·y) = (−1,0,0)
        assert!((duu[0] + 1.0).abs() < 1e-14 && duu[1].abs() < 1e-14 && duu[2].abs() < 1e-14);
    }

    #[test]
    fn cylinder_duu_equals_negative_radial_position() {
        // p″_uu = −r·(cos u·x + sin u·y) = −(p_radial_part)
        let c = Cylinder::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            3.0,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        for u in [0.1_f64, 1.0, 2.5, PI, 4.7] {
            let ev = c.evaluate(u, 1.0, DerivativeOrder::Second).unwrap();
            let pos = ev.position.into_array();
            let duu = ev.duu.unwrap().into_array();
            // The radial part of p is (pos[0], pos[1], 0) for axis_dir=Z.
            // duu should equal -radial_part
            assert!(
                (duu[0] + pos[0]).abs() < 1e-12,
                "u={u} duu[0]={} pos[0]={}",
                duu[0],
                pos[0]
            );
            assert!(
                (duu[1] + pos[1]).abs() < 1e-12,
                "u={u} duu[1]={} pos[1]={}",
                duu[1],
                pos[1]
            );
            assert!(duu[2].abs() < 1e-12, "duu[2] should be 0");
        }
    }

    #[test]
    fn cylinder_evaluate_fd_check() {
        let c = Cylinder::try_new(
            Point3::try_new(1.0, -2.0, 0.5).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            2.0,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let h = 1e-7_f64;
        for u in [0.5_f64, 2.0, 4.0] {
            let p_plus = c
                .evaluate(u + h, 1.0, DerivativeOrder::Position)
                .unwrap()
                .position
                .into_array();
            let p_minus = c
                .evaluate(u - h, 1.0, DerivativeOrder::Position)
                .unwrap()
                .position
                .into_array();
            let fd_du: [f64; 3] = std::array::from_fn(|i| (p_plus[i] - p_minus[i]) / (2.0 * h));
            let analytic_du = c
                .evaluate(u, 1.0, DerivativeOrder::First)
                .unwrap()
                .du
                .unwrap()
                .into_array();
            for i in 0..3 {
                assert!(
                    (fd_du[i] - analytic_du[i]).abs() < 1e-5,
                    "u={u} component {i}"
                );
            }
        }
    }

    #[test]
    fn cylinder_periodic_equivalence() {
        let c = unit_cylinder();
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
    fn cylinder_projection_round_trip() {
        let c = Cylinder::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            3.0,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        for (u0, v0) in [(0.0, 0.0), (FRAC_PI_2, 1.0), (PI, -5.0), (5.0, 10.0)] {
            let pt = c
                .evaluate(u0, v0, DerivativeOrder::Position)
                .unwrap()
                .position;
            let projs = c.project(pt, &tol()).unwrap();
            assert_eq!(projs.len(), 1, "u={u0} v={v0}");
            assert!((projs[0].u.get() - u0).abs() < 1e-11, "u round-trip u={u0}");
            assert!((projs[0].v.get() - v0).abs() < 1e-11, "v round-trip v={v0}");
            assert!(projs[0].distance_bound.get() < 1e-11);
        }
    }

    #[test]
    fn cylinder_projection_singular_on_axis() {
        let c = unit_cylinder();
        // Point on the Z-axis: all u equidistant.
        let q = Point3::try_new(0.0, 0.0, 3.0).unwrap();
        assert_eq!(c.project(q, &tol()), Err(GeometryError::Singular));
        let near_axis = Point3::try_new(0.5e-9, 0.0, 3.0).unwrap();
        assert_eq!(c.project(near_axis, &tol()), Err(GeometryError::Singular));
    }

    #[test]
    fn cylinder_evaluate_rejects_out_of_domain() {
        let c = unit_cylinder();
        assert_eq!(
            c.evaluate(-0.001, 0.0, DerivativeOrder::Position),
            Err(GeometryError::OutsideDomain)
        );
        assert_eq!(
            c.evaluate(TAU, 0.0, DerivativeOrder::Position),
            Err(GeometryError::OutsideDomain)
        );
    }

    #[test]
    fn cylinder_evaluate_rejects_non_finite() {
        let c = unit_cylinder();
        assert_eq!(
            c.evaluate(f64::NAN, 0.0, DerivativeOrder::Position),
            Err(GeometryError::NonFiniteParameter)
        );
        assert_eq!(
            c.evaluate(0.0, f64::INFINITY, DerivativeOrder::Position),
            Err(GeometryError::NonFiniteParameter)
        );
    }

    #[test]
    fn cylinder_serde_round_trip() {
        let c = Cylinder::try_new(
            Point3::try_new(1.0, 2.0, 3.0).unwrap(),
            Vector3::try_new(
                1.0 / 3.0_f64.sqrt(),
                1.0 / 3.0_f64.sqrt(),
                1.0 / 3.0_f64.sqrt(),
            )
            .unwrap(),
            2.5,
            Vector3::try_new(1.0 / 2.0_f64.sqrt(), -1.0 / 2.0_f64.sqrt(), 0.0).unwrap(),
        )
        .unwrap();
        let json = serde_json::to_string(&c).unwrap();
        let decoded: Cylinder = serde_json::from_str(&json).unwrap();
        assert_eq!(c, decoded);
    }

    #[test]
    fn cylinder_distance_bounds_certify_actual_distance_at_extreme_scales() {
        let tiny_tol = ToleranceContext::try_new(1e-15, 1e-8, 1e-10, 1e-12).unwrap();
        let unit = unit_cylinder();
        let tiny = Cylinder::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            1.0e-12,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        for (cylinder, query, tolerance) in [
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
                Point3::try_new(1.0e12, 1.0, 3.0).unwrap(),
                tol(),
            ),
            (
                tiny.clone(),
                Point3::try_new(2.0e-12, 0.0, 3.0e-12).unwrap(),
                tiny_tol,
            ),
            (
                unit.clone(),
                Point3::try_new(1.0, 1.0e-12, 2.0).unwrap(),
                tol(),
            ),
        ] {
            let projection = cylinder.project(query, &tolerance).unwrap().remove(0);
            let actual = dist3(query, projection.point);
            assert!(actual <= projection.distance_bound.get(), "{query:?}");
            assert!(projection.distance_bound.get() >= 0.0);
            assert!(projection.u.get() < TAU);
        }
    }

    #[test]
    fn cylinder_project_into_clears_output_on_error() {
        let c = unit_cylinder();
        let mut output = c
            .project(Point3::try_new(1.0, 0.0, 0.0).unwrap(), &tol())
            .unwrap();
        let err = c.project_into(Point3::try_new(0.0, 0.0, 3.0).unwrap(), &tol(), &mut output);
        assert_eq!(err, Err(GeometryError::Singular));
        assert!(output.is_empty());
    }

    #[test]
    fn cylinder_projection_seam_stays_in_range() {
        let c = unit_cylinder();
        let eps = 1e-12_f64;
        for u in [TAU - eps, eps] {
            let query = c
                .evaluate(u, 1.0, DerivativeOrder::Position)
                .unwrap()
                .position;
            let projection = c.project(query, &tol()).unwrap().remove(0);
            assert!(projection.u.get() >= 0.0);
            assert!(projection.u.get() < TAU);
        }
    }

    #[test]
    fn cylinder_serde_rejects_bad_axis_radius_and_orthogonality() {
        let bad_axis: CylinderRepr = serde_json::from_value(json!({
            "axis_origin": [1.0, 2.0, 3.0],
            "axis_dir": [2.0, 0.0, 0.0],
            "radius": 2.5,
            "x_axis": [0.0, 1.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Cylinder::try_from(bad_axis),
            Err(ConstructionError::DegenerateAxis)
        );

        let bad_frame: CylinderRepr = serde_json::from_value(json!({
            "axis_origin": [1.0, 2.0, 3.0],
            "axis_dir": [1.0, 0.0, 0.0],
            "radius": 2.5,
            "x_axis": [1.0, 0.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Cylinder::try_from(bad_frame),
            Err(ConstructionError::DependentAxes)
        );

        let bad_radius: CylinderRepr = serde_json::from_value(json!({
            "axis_origin": [1.0, 2.0, 3.0],
            "axis_dir": [0.0, 0.0, 1.0],
            "radius": 0.0,
            "x_axis": [1.0, 0.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Cylinder::try_from(bad_radius),
            Err(ConstructionError::NotPositive)
        );
    }

    #[test]
    fn cylinder_serde_rejects_nan_and_inf_fields() {
        assert!(serde_json::from_str::<Cylinder>(
            r#"{"axis_origin":[NaN,0.0,0.0],"axis_dir":[0.0,0.0,1.0],"radius":1.0,"x_axis":[1.0,0.0,0.0]}"#
        )
        .is_err());
        assert!(serde_json::from_str::<Cylinder>(
            r#"{"axis_origin":[0.0,0.0,0.0],"axis_dir":[Infinity,0.0,1.0],"radius":1.0,"x_axis":[1.0,0.0,0.0]}"#
        )
        .is_err());
    }

    #[test]
    fn cylinder_try_transform_identity_is_noop() {
        let c = unit_cylinder();
        let out = c
            .try_transform(&amphion_foundation::Transform3::IDENTITY)
            .unwrap();
        assert_eq!(out, c);
    }

    #[test]
    fn cylinder_try_transform_similarity_scales_radius() {
        // Rotation by 90° about Z, uniform scale 2, plus translation.
        let m = [
            0.0, -2.0, 0.0, 5.0, //
            2.0, 0.0, 0.0, -3.0, //
            0.0, 0.0, 2.0, 7.0,
        ];
        let t = amphion_foundation::Transform3::try_from_row_major(m).unwrap();
        let c = unit_cylinder();
        let out = c.try_transform(&t).unwrap();
        assert!((out.radius() - 2.0).abs() < 1e-9);
        let [ox, oy, oz] = out.axis_origin().into_array();
        assert!((ox - 5.0).abs() < 1e-9);
        assert!((oy - (-3.0)).abs() < 1e-9);
        assert!((oz - 7.0).abs() < 1e-9);
    }

    #[test]
    fn cylinder_try_transform_rejects_non_similarity() {
        let m = [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 2.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0,
        ];
        let t = amphion_foundation::Transform3::try_from_row_major(m).unwrap();
        let c = unit_cylinder();
        assert_eq!(
            c.try_transform(&t),
            Err(super::TransformError::NotSimilarity)
        );
    }
}
