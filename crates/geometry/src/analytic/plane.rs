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
        ILL_COND_THRESH, add3, all_finite3, arithmetic_eval_bound, arithmetic_proj_bound3, cross3,
        dot3, mag3, normalize3, scale3, sub3, validate_orthogonal3, validate_unit3,
    },
    transform::{apply_to_point, apply_to_vector},
};

fn unbounded_range() -> ParameterRange {
    // (None, None, None) is a compile-time constant and always valid; this
    // is not an input-dependent path, so a static-invariant `expect` is
    // acceptable here (see CONTRACTS.md).
    ParameterRange::try_new(None, None, None).expect("unbounded domain is always valid")
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
    normal: Vector3,
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
        let n_arr = cross3(u_unit, v_unit);
        Ok(Self {
            origin: Point3::try_new(o[0], o[1], o[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            u_axis: Vector3::try_new(u_unit[0], u_unit[1], u_unit[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            v_axis: Vector3::try_new(v_unit[0], v_unit[1], v_unit[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            normal: Vector3::try_new(n_arr[0], n_arr[1], n_arr[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
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
        self.normal
    }

    /// Applies an affine `transform` to this plane, returning a new plane.
    ///
    /// Any non-degenerate affine transform is accepted: the affine image of
    /// a plane is a plane as long as the transformed spanning vectors stay
    /// independent. `try_new` re-orthonormalizes the transformed axes, so a
    /// non-similarity (shearing) transform is accepted but its effect on
    /// `v_axis` is Gram-Schmidt-corrected against the transformed `u_axis`.
    ///
    /// # Errors
    ///
    /// - [`TransformError::NonFiniteResult`] — the transformed origin or
    ///   axes contain a non-finite component
    /// - [`TransformError::DegenerateResult`] — the transformed axes become
    ///   zero-length, dependent, or ill-conditioned
    pub fn try_transform(&self, transform: &Transform3) -> Result<Self, TransformError> {
        let m = transform.into_row_major();
        let o =
            apply_to_point(m, self.origin.into_array()).ok_or(TransformError::NonFiniteResult)?;
        let u =
            apply_to_vector(m, self.u_axis.into_array()).ok_or(TransformError::NonFiniteResult)?;
        let v =
            apply_to_vector(m, self.v_axis.into_array()).ok_or(TransformError::NonFiniteResult)?;
        Self::try_new(
            Point3::try_new(o[0], o[1], o[2]).map_err(|_| TransformError::NonFiniteResult)?,
            Vector3::try_new(u[0], u[1], u[2]).map_err(|_| TransformError::NonFiniteResult)?,
            Vector3::try_new(v[0], v[1], v[2]).map_err(|_| TransformError::NonFiniteResult)?,
        )
        .map_err(|_| TransformError::DegenerateResult)
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
        let n_arr = cross3(u_axis, v_axis);
        Ok(Self {
            origin: repr.origin,
            u_axis: repr.u_axis,
            v_axis: repr.v_axis,
            normal: Vector3::try_new(n_arr[0], n_arr[1], n_arr[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
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
        tolerance: &ToleranceContext,
    ) -> Result<SurfaceEvaluation, GeometryError> {
        if !u.is_finite() || !v.is_finite() {
            return Err(GeometryError::NonFiniteParameter);
        }
        let o = self.origin.into_array();
        let u_ax = self.u_axis.into_array();
        let v_ax = self.v_axis.into_array();
        let u_term = scale3(u_ax, u);
        let v_term = scale3(v_ax, v);
        let pos_arr = add3(o, add3(u_term, v_term));
        let pos = Point3::try_new(pos_arr[0], pos_arr[1], pos_arr[2]).map_err(|_| {
            GeometryError::Uncertified {
                reason: "plane position is non-finite".to_owned(),
            }
        })?;

        // Plane evaluation is arithmetic-only (no trig); the Higham bound
        // applies.
        let eval_scale = mag3(o) + mag3(u_term) + mag3(v_term);
        let position_error_bound = arithmetic_eval_bound(eval_scale)?;
        let eff_tol =
            tolerance
                .effective_length(eval_scale)
                .map_err(|_| GeometryError::Uncertified {
                    reason: "world scale is invalid for tolerance computation".to_owned(),
                })?;
        if position_error_bound.get() > eff_tol {
            return Err(GeometryError::Uncertified {
                reason: "position error bound exceeds requested tolerance at this scale".to_owned(),
            });
        }

        let to_vec3 = |arr: [f64; 3], what: &'static str| {
            Vector3::try_new(arr[0], arr[1], arr[2]).map_err(|_| GeometryError::Uncertified {
                reason: format!("{what} non-finite"),
            })
        };
        let axis_error_bound = arithmetic_eval_bound(1.0)?;
        let zero_error_bound =
            DistanceBound::try_new(0.0).map_err(|_| GeometryError::Uncertified {
                reason: "zero distance bound construction failed unexpectedly".to_owned(),
            })?;

        let (du, dv, duu, duv, dvv, first_u_eb, first_v_eb, duu_eb, duv_eb, dvv_eb) = match order {
            DerivativeOrder::Position => {
                (None, None, None, None, None, None, None, None, None, None)
            }
            DerivativeOrder::First => {
                let du = to_vec3(u_ax, "u_axis")?;
                let dv = to_vec3(v_ax, "v_axis")?;
                (
                    Some(du),
                    Some(dv),
                    None,
                    None,
                    None,
                    Some(axis_error_bound),
                    Some(axis_error_bound),
                    None,
                    None,
                    None,
                )
            }
            DerivativeOrder::Second => {
                let du = to_vec3(u_ax, "u_axis")?;
                let dv = to_vec3(v_ax, "v_axis")?;
                let zero = to_vec3([0.0, 0.0, 0.0], "zero vector")?;
                (
                    Some(du),
                    Some(dv),
                    Some(zero),
                    Some(zero),
                    Some(zero),
                    Some(axis_error_bound),
                    Some(axis_error_bound),
                    Some(zero_error_bound),
                    Some(zero_error_bound),
                    Some(zero_error_bound),
                )
            }
        };
        Ok(SurfaceEvaluation {
            position: pos,
            du,
            dv,
            duu,
            duv,
            dvv,
            position_error_bound,
            first_u_error_bound: first_u_eb,
            first_v_error_bound: first_v_eb,
            second_uu_error_bound: duu_eb,
            second_uv_error_bound: duv_eb,
            second_vv_error_bound: dvv_eb,
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
            distance_bound: DistanceBound::try_new(arithmetic_proj_bound3(q, proj_arr, tolerance)?)
                .map_err(|_| GeometryError::Uncertified {
                    reason: "plane projection distance is non-finite or negative".to_owned(),
                })?,
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
        let ev = p
            .evaluate(3.0, 4.0, DerivativeOrder::Position, &tol())
            .unwrap();
        assert!((ev.position.x() - 3.0).abs() < 1e-14);
        assert!((ev.position.y() - 4.0).abs() < 1e-14);
        assert!((ev.position.z()).abs() < 1e-14);
        assert!(ev.position_error_bound.get() >= 0.0);
    }

    #[test]
    fn plane_evaluate_derivatives() {
        let p = xy_plane();
        let ev = p
            .evaluate(1.0, 2.0, DerivativeOrder::Second, &tol())
            .unwrap();
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
        assert!(ev.position_error_bound.get() >= 0.0);
        assert!(ev.first_u_error_bound.unwrap().get() >= 0.0);
        assert!(ev.first_v_error_bound.unwrap().get() >= 0.0);
        assert!(ev.second_uu_error_bound.unwrap().get() >= 0.0);
        assert!(ev.second_uv_error_bound.unwrap().get() >= 0.0);
        assert!(ev.second_vv_error_bound.unwrap().get() >= 0.0);
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
            .evaluate(u0 + h, v0, DerivativeOrder::Position, &tol())
            .unwrap()
            .position
            .into_array();
        let p_u_minus = p
            .evaluate(u0 - h, v0, DerivativeOrder::Position, &tol())
            .unwrap()
            .position
            .into_array();
        let fd_u: [f64; 3] = std::array::from_fn(|i| (p_u_plus[i] - p_u_minus[i]) / (2.0 * h));
        let analytic_du = p
            .evaluate(u0, v0, DerivativeOrder::First, &tol())
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
                .evaluate(u0, v0, DerivativeOrder::Position, &tol())
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
                .evaluate(3.0, 4.0, DerivativeOrder::Position, &tol())
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
            p.evaluate(f64::NAN, 0.0, DerivativeOrder::Position, &tol()),
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

    #[test]
    fn plane_try_transform_identity_is_noop() {
        let p = xy_plane();
        let out = p
            .try_transform(&amphion_foundation::Transform3::IDENTITY)
            .unwrap();
        assert_eq!(out, p);
    }

    #[test]
    fn plane_try_transform_scale_rotation_translation() {
        // Rotation by 90° about Z, uniform scale 2, plus translation.
        let m = [
            0.0, -2.0, 0.0, 5.0, //
            2.0, 0.0, 0.0, -3.0, //
            0.0, 0.0, 2.0, 7.0,
        ];
        let t = amphion_foundation::Transform3::try_from_row_major(m).unwrap();
        let p = xy_plane();
        let out = p.try_transform(&t).unwrap();
        let [ox, oy, oz] = out.origin().into_array();
        assert!((ox - 5.0).abs() < 1e-9);
        assert!((oy - (-3.0)).abs() < 1e-9);
        assert!((oz - 7.0).abs() < 1e-9);
        // u_axis (1,0,0) -> (0,2,0) normalized -> (0,1,0)
        let [ux, uy, uz] = out.u_axis().into_array();
        assert!((ux - 0.0).abs() < 1e-9);
        assert!((uy - 1.0).abs() < 1e-9);
        assert!((uz - 0.0).abs() < 1e-9);
        // normal should still be a unit vector.
        let n = out.normal().into_array();
        let mag = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        assert!((mag - 1.0).abs() < 1e-9);
    }

    #[test]
    fn plane_try_transform_rejects_degenerate_result() {
        // A rank-deficient linear part collapses u_axis and v_axis onto the
        // same line, which cannot form a plane.
        let m = [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 0.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0,
        ];
        let t = amphion_foundation::Transform3::try_from_row_major(m).unwrap();
        let p = xy_plane();
        assert_eq!(
            p.try_transform(&t),
            Err(super::TransformError::DegenerateResult)
        );
    }
}
