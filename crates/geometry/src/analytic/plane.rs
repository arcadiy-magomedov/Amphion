//! Analytic planar surface.
//!
//! # Parameterization
//!
//! ```text
//! p(u, v) = origin + u·u_axis + v·v_axis
//! ```
//!
//! `u_axis` and `v_axis` form a right-handed orthonormal frame together with
//! the surface normal `n = u_axis × v_axis`.  The UV domain is unbounded in
//! both directions.
//!
//! Derivatives:
//! - `∂p/∂u = u_axis`,  `∂p/∂v = v_axis`
//! - All second-order partials are zero.
//!
//! Projection: `u = (q − origin)·u_axis`,  `v = (q − origin)·v_axis`.  The
//! distance bound equals the perpendicular distance from `q` to the plane:
//! `|(q − origin)·normal|`.

#![allow(
    clippy::many_single_char_names,
    clippy::missing_panics_doc,
    clippy::similar_names
)]

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
        ILL_COND_THRESH, add3, all_finite3, cross3, dot3, mag3, normalize3, scale3, sub3,
        validate_orthogonal3, validate_unit3, widened_distance_bound3,
    },
};

fn unbounded_range() -> ParameterRange {
    ParameterRange::try_new(None, None, None).expect("None/None/None is always valid")
}

#[derive(Serialize, Deserialize)]
struct PlaneRepr {
    origin: Point3,
    u_axis: Vector3,
    v_axis: Vector3,
}

/// An infinite analytic plane surface.
///
/// Parameterization: `p(u, v) = origin + u·u_axis + v·v_axis` over
/// `(−∞, +∞) × (−∞, +∞)`.  `u_axis` and `v_axis` are orthonormal.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "PlaneRepr", into = "PlaneRepr")]
pub struct Plane {
    origin: Point3,
    u_axis: Vector3,
    v_axis: Vector3,
}

impl Plane {
    /// Constructs a plane.
    ///
    /// `u_axis` is normalized internally.  `v_axis` is orthogonalized against
    /// `u_axis` (Gram-Schmidt) and then normalized.
    ///
    /// # Errors
    ///
    /// - [`ConstructionError::NonFiniteInput`] — any NaN/Inf component
    /// - [`ConstructionError::DegenerateAxis`] — zero-length `u_axis` or `v_axis`
    /// - [`ConstructionError::DependentAxes`] — `u_axis` parallel to `v_axis`
    /// - [`ConstructionError::IllConditionedAxes`] — `v_axis` nearly parallel
    ///   to `u_axis`
    pub fn try_new(
        origin: Point3,
        u_axis: Vector3,
        v_axis: Vector3,
    ) -> Result<Self, ConstructionError> {
        let o = origin.into_array();
        let u = u_axis.into_array();
        let v = v_axis.into_array();
        if !all_finite3(o) || !all_finite3(u) || !all_finite3(v) {
            return Err(ConstructionError::NonFiniteInput);
        }
        let u_unit = normalize3(u).ok_or(ConstructionError::DegenerateAxis)?;
        let v_norm = normalize3(v).ok_or(ConstructionError::DegenerateAxis)?;
        // Orthogonalize v against u.
        let dot_vu = dot3(v_norm, u_unit);
        let v_perp = sub3(v_norm, scale3(u_unit, dot_vu));
        let perp_mag = mag3(v_perp);
        if perp_mag == 0.0 {
            return Err(ConstructionError::DependentAxes);
        }
        if perp_mag < ILL_COND_THRESH {
            return Err(ConstructionError::IllConditionedAxes);
        }
        let v_unit = normalize3(v_perp).ok_or(ConstructionError::DependentAxes)?;
        Ok(Self {
            // unreachable: validated above
            origin: Point3::try_new(o[0], o[1], o[2]).expect("origin validated finite"),
            // unreachable: validated above
            u_axis: Vector3::try_new(u_unit[0], u_unit[1], u_unit[2])
                .expect("unit u_axis is finite"),
            // unreachable: validated above
            v_axis: Vector3::try_new(v_unit[0], v_unit[1], v_unit[2])
                .expect("unit v_axis is finite"),
        })
    }

    /// Returns the plane origin.
    #[must_use]
    pub fn origin(&self) -> Point3 {
        self.origin
    }

    /// Returns the unit U direction.
    #[must_use]
    pub fn u_axis(&self) -> Vector3 {
        self.u_axis
    }

    /// Returns the unit V direction (orthogonal to `u_axis`).
    #[must_use]
    pub fn v_axis(&self) -> Vector3 {
        self.v_axis
    }

    /// Returns the outward unit normal `u_axis × v_axis`.
    #[must_use]
    pub fn normal(&self) -> Vector3 {
        let u = self.u_axis.into_array();
        let v = self.v_axis.into_array();
        let n = cross3(u, v);
        Vector3::try_new(n[0], n[1], n[2]).expect("cross product of orthonormal pair is finite")
    }
}

impl TryFrom<PlaneRepr> for Plane {
    type Error = ConstructionError;
    fn try_from(repr: PlaneRepr) -> Result<Self, Self::Error> {
        let origin = repr.origin.into_array();
        let u_axis = repr.u_axis.into_array();
        let v_axis = repr.v_axis.into_array();
        if !all_finite3(origin) || !all_finite3(u_axis) || !all_finite3(v_axis) {
            return Err(ConstructionError::NonFiniteInput);
        }
        validate_unit3(u_axis)?;
        validate_unit3(v_axis)?;
        validate_orthogonal3(u_axis, v_axis)?;
        Ok(Self {
            origin: repr.origin,
            u_axis: repr.u_axis,
            v_axis: repr.v_axis,
        })
    }
}

impl From<Plane> for PlaneRepr {
    fn from(p: Plane) -> Self {
        Self {
            origin: p.origin,
            u_axis: p.u_axis,
            v_axis: p.v_axis,
        }
    }
}

impl SurfaceEvaluator for Plane {
    fn kind(&self) -> SurfaceKind {
        SurfaceKind::Plane
    }

    fn domain(&self) -> SurfaceDomain {
        SurfaceDomain::new(unbounded_range(), unbounded_range())
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
        let o = self.origin.into_array();
        let u_ax = self.u_axis.into_array();
        let v_ax = self.v_axis.into_array();
        let pos_arr = add3(o, add3(scale3(u_ax, u), scale3(v_ax, v)));
        let pos = Point3::try_new(pos_arr[0], pos_arr[1], pos_arr[2]).map_err(|_| {
            GeometryError::Uncertified {
                reason: "plane position is non-finite".to_owned(),
            }
        })?;
        let (du, dv, duu, duv, dvv) = match order {
            DerivativeOrder::Position => (None, None, None, None, None),
            DerivativeOrder::First => {
                let du = Vector3::try_new(u_ax[0], u_ax[1], u_ax[2]).map_err(|_| {
                    GeometryError::Uncertified {
                        reason: "u_axis non-finite".to_owned(),
                    }
                })?;
                let dv = Vector3::try_new(v_ax[0], v_ax[1], v_ax[2]).map_err(|_| {
                    GeometryError::Uncertified {
                        reason: "v_axis non-finite".to_owned(),
                    }
                })?;
                (Some(du), Some(dv), None, None, None)
            }
            DerivativeOrder::Second => {
                let du = Vector3::try_new(u_ax[0], u_ax[1], u_ax[2]).map_err(|_| {
                    GeometryError::Uncertified {
                        reason: "u_axis non-finite".to_owned(),
                    }
                })?;
                let dv = Vector3::try_new(v_ax[0], v_ax[1], v_ax[2]).map_err(|_| {
                    GeometryError::Uncertified {
                        reason: "v_axis non-finite".to_owned(),
                    }
                })?;
                let zero = Vector3::try_new(0.0, 0.0, 0.0).expect("zero is finite");
                (Some(du), Some(dv), Some(zero), Some(zero), Some(zero))
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
        let o = self.origin.into_array();
        let u_ax = self.u_axis.into_array();
        let v_ax = self.v_axis.into_array();
        let diff = sub3(q, o);
        let u_val = dot3(diff, u_ax);
        let v_val = dot3(diff, v_ax);
        if !u_val.is_finite() || !v_val.is_finite() {
            return Err(GeometryError::Uncertified {
                reason: "plane projection parameter is non-finite".to_owned(),
            });
        }
        let proj_arr = add3(o, add3(scale3(u_ax, u_val), scale3(v_ax, v_val)));
        let proj = Point3::try_new(proj_arr[0], proj_arr[1], proj_arr[2]).map_err(|_| {
            GeometryError::Uncertified {
                reason: "plane projection point is non-finite".to_owned(),
            }
        })?;
        let local = SurfaceProjection {
            u: ParameterValue::try_new(u_val).map_err(|_| GeometryError::Uncertified {
                reason: "plane projection u is non-finite".to_owned(),
            })?,
            v: ParameterValue::try_new(v_val).map_err(|_| GeometryError::Uncertified {
                reason: "plane projection v is non-finite".to_owned(),
            })?,
            point: proj,
            distance_bound: DistanceBound::try_new(widened_distance_bound3(q, proj_arr)).map_err(
                |_| GeometryError::Uncertified {
                    reason: "plane projection distance is non-finite or negative".to_owned(),
                },
            )?,
        };
        output.push(local);
        Ok(())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use amphion_foundation::{Point3, ToleranceContext, Vector3};
    use serde_json::json;

    use crate::traits::SurfaceEvaluator;
    use crate::{DerivativeOrder, GeometryError};

    use super::{ConstructionError, Plane, PlaneRepr};
    use crate::analytic::helpers::ILL_COND_THRESH;

    fn tol() -> ToleranceContext {
        ToleranceContext::try_new(1e-9, 1e-8, 1e-10, 1e-12).unwrap()
    }

    fn dist3(a: Point3, b: Point3) -> f64 {
        let [ax, ay, az] = a.into_array();
        let [bx, by, bz] = b.into_array();
        (ax - bx).hypot((ay - by).hypot(az - bz))
    }

    fn xy_plane() -> Plane {
        Plane::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 1.0, 0.0).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn plane_construction_valid() {
        assert!(
            xy_plane()
                .normal()
                .into_array()
                .iter()
                .zip([0.0, 0.0, 1.0])
                .all(|(a, b)| (a - b).abs() < 1e-14)
        );
    }

    #[test]
    fn plane_construction_orthogonalizes_v_axis() {
        // Supply a v_axis that is not exactly perpendicular to u_axis.
        let p = Plane::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.5, 1.0, 0.0).unwrap(), // not perpendicular
        )
        .unwrap();
        let u = p.u_axis().into_array();
        let v = p.v_axis().into_array();
        let dot = u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
        assert!(
            dot.abs() < 1e-14,
            "u·v should be 0 after orthogonalization, got {dot}"
        );
    }

    #[test]
    fn plane_construction_rejects_dependent_axes() {
        let err = Plane::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(), // parallel
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::DependentAxes);
    }

    #[test]
    fn plane_construction_rejects_ill_conditioned_axes() {
        let err = Plane::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(ILL_COND_THRESH / 2.0, 0.0, 1.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::IllConditionedAxes);
    }

    #[test]
    fn plane_construction_rejects_zero_axis() {
        let err = Plane::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 1.0, 0.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::DegenerateAxis);
    }

    #[test]
    fn plane_evaluate_position() {
        let p = xy_plane();
        let ev = p.evaluate(3.0, 4.0, DerivativeOrder::Position).unwrap();
        assert!((ev.position.x() - 3.0).abs() < 1e-14);
        assert!((ev.position.y() - 4.0).abs() < 1e-14);
        assert!((ev.position.z()).abs() < 1e-14);
    }

    #[test]
    fn plane_evaluate_derivatives() {
        let p = xy_plane();
        let ev = p.evaluate(1.0, 2.0, DerivativeOrder::Second).unwrap();
        let du = ev.du.unwrap().into_array();
        let dv = ev.dv.unwrap().into_array();
        let duu = ev.duu.unwrap().into_array();
        let duv = ev.duv.unwrap().into_array();
        let dvv = ev.dvv.unwrap().into_array();
        assert!((du[0] - 1.0).abs() < 1e-14 && du[1].abs() < 1e-14 && du[2].abs() < 1e-14);
        assert!(dv[0].abs() < 1e-14 && (dv[1] - 1.0).abs() < 1e-14 && dv[2].abs() < 1e-14);
        assert!(duu.iter().all(|v| v.abs() < 1e-14), "duu must be zero");
        assert!(duv.iter().all(|v| v.abs() < 1e-14), "duv must be zero");
        assert!(dvv.iter().all(|v| v.abs() < 1e-14), "dvv must be zero");
    }

    #[test]
    fn plane_evaluate_fd_check() {
        let p = Plane::try_new(
            Point3::try_new(1.0, 2.0, 3.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 1.0, 0.0).unwrap(),
        )
        .unwrap();
        let h = 1e-7_f64;
        let (u0, v0) = (1.5, -2.0);
        let p_u_plus = p
            .evaluate(u0 + h, v0, DerivativeOrder::Position)
            .unwrap()
            .position
            .into_array();
        let p_u_minus = p
            .evaluate(u0 - h, v0, DerivativeOrder::Position)
            .unwrap()
            .position
            .into_array();
        let fd_u: [f64; 3] = std::array::from_fn(|i| (p_u_plus[i] - p_u_minus[i]) / (2.0 * h));
        let analytic_du = p
            .evaluate(u0, v0, DerivativeOrder::First)
            .unwrap()
            .du
            .unwrap()
            .into_array();
        for i in 0..3 {
            assert!((fd_u[i] - analytic_du[i]).abs() < 1e-6, "du component {i}");
        }
    }

    #[test]
    fn plane_projection_round_trip() {
        let p = Plane::try_new(
            Point3::try_new(1.0, 2.0, 3.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 1.0, 0.0).unwrap(),
        )
        .unwrap();
        for (u0, v0) in [(0.0, 0.0), (5.0, -3.0), (-100.0, 200.0)] {
            let pt = p
                .evaluate(u0, v0, DerivativeOrder::Position)
                .unwrap()
                .position;
            let projs = p.project(pt, &tol()).unwrap();
            assert_eq!(projs.len(), 1);
            assert!((projs[0].u.get() - u0).abs() < 1e-11, "u round-trip");
            assert!((projs[0].v.get() - v0).abs() < 1e-11, "v round-trip");
            assert!(projs[0].distance_bound.get() < 1e-11);
        }
    }

    #[test]
    fn plane_projection_off_plane_distance() {
        let p = xy_plane();
        // Point 5 units above the XY plane.
        let q = Point3::try_new(3.0, 4.0, 5.0).unwrap();
        let projs = p.project(q, &tol()).unwrap();
        assert_eq!(projs.len(), 1);
        assert!((projs[0].u.get() - 3.0).abs() < 1e-12);
        assert!((projs[0].v.get() - 4.0).abs() < 1e-12);
        assert!(5.0 <= projs[0].distance_bound.get());
    }

    #[test]
    fn plane_distance_bounds_certify_actual_distance_at_extreme_scales() {
        let plane = xy_plane();
        for query in [
            plane
                .evaluate(3.0, 4.0, DerivativeOrder::Position)
                .unwrap()
                .position,
            Point3::try_new(3.0, 4.0, 5.0).unwrap(),
            Point3::try_new(1.0e12, -1.0e12, 7.0).unwrap(),
            Point3::try_new(1.0e-12, -2.0e-12, 3.0e-12).unwrap(),
            Point3::try_new(10.0, 11.0, 1.0e-12).unwrap(),
        ] {
            let projection = plane.project(query, &tol()).unwrap().remove(0);
            let actual = dist3(query, projection.point);
            assert!(actual <= projection.distance_bound.get(), "{query:?}");
            assert!(projection.distance_bound.get() >= 0.0);
        }
    }

    #[test]
    fn plane_evaluate_rejects_non_finite() {
        let p = xy_plane();
        assert_eq!(
            p.evaluate(f64::NAN, 0.0, DerivativeOrder::Position),
            Err(GeometryError::NonFiniteParameter)
        );
    }

    #[test]
    fn plane_serde_round_trip() {
        let p = Plane::try_new(
            Point3::try_new(1.0, 2.0, 3.0).unwrap(),
            Vector3::try_new(
                1.0 / 3.0_f64.sqrt(),
                1.0 / 3.0_f64.sqrt(),
                1.0 / 3.0_f64.sqrt(),
            )
            .unwrap(),
            Vector3::try_new(1.0 / 2.0_f64.sqrt(), -1.0 / 2.0_f64.sqrt(), 0.0).unwrap(),
        )
        .unwrap();
        let json = serde_json::to_string(&p).unwrap();
        let decoded: Plane = serde_json::from_str(&json).unwrap();
        assert_eq!(p, decoded);
    }

    #[test]
    fn plane_serde_rejects_non_unit_axes() {
        let repr: PlaneRepr = serde_json::from_value(json!({
            "origin": [1.0, 2.0, 3.0],
            "u_axis": [2.0, 0.0, 0.0],
            "v_axis": [0.0, 1.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Plane::try_from(repr),
            Err(ConstructionError::DegenerateAxis)
        );
    }

    #[test]
    fn plane_serde_rejects_non_orthogonal_axes() {
        let repr: PlaneRepr = serde_json::from_value(json!({
            "origin": [1.0, 2.0, 3.0],
            "u_axis": [1.0, 0.0, 0.0],
            "v_axis": [1.0, 0.0, 0.0]
        }))
        .unwrap();
        assert_eq!(Plane::try_from(repr), Err(ConstructionError::DependentAxes));
    }

    #[test]
    fn plane_serde_rejects_nan_and_inf_fields() {
        assert!(
            serde_json::from_str::<Plane>(
                r#"{"origin":[NaN,0.0,0.0],"u_axis":[1.0,0.0,0.0],"v_axis":[0.0,1.0,0.0]}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<Plane>(
                r#"{"origin":[0.0,0.0,0.0],"u_axis":[Infinity,0.0,0.0],"v_axis":[0.0,1.0,0.0]}"#
            )
            .is_err()
        );
    }
}
