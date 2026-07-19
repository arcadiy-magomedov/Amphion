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

use amphion_foundation::{Point3, Transform3, UnitVector3, Vector3};
use serde::{Deserialize, Serialize};

use crate::traits::SurfaceEvaluator;
use crate::{
    DerivativeOrder, DistanceBound, EvaluationContext, FirstDerivativeBound, GeometryError,
    ParameterRange, ParameterValue, PositionBound, SecondDerivativeBound, SurfaceDomain,
    SurfaceEvaluation, SurfaceKind, SurfaceProjection,
};

use super::{
    ConstructionError, TransformError,
    helpers::{
        ILL_COND_THRESH, UNIT_VECTOR_TOL, all_finite3, check_tolerance, dot3, exact_cylinder_eval,
        exact_cylinder_project, in_range, mag3, normalization_to_construction, scale3, sub3,
    },
    transform::similarity_scale,
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
    axis_dir: UnitVector3,
    radius: f64,
    x_axis: UnitVector3,
    y_axis: UnitVector3,
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
        if !all_finite3(o) || !radius.is_finite() {
            return Err(ConstructionError::NonFiniteInput);
        }
        if radius <= 0.0 {
            return Err(ConstructionError::NotPositive);
        }
        let a_unit = UnitVector3::try_normalize(axis_dir).map_err(normalization_to_construction)?;
        let x_norm = UnitVector3::try_normalize(x_axis).map_err(normalization_to_construction)?;
        // Orthogonalize x against axis_dir.
        let dot_xa = dot3(x_norm.into_array(), a_unit.into_array());
        let x_perp = sub3(x_norm.into_array(), scale3(a_unit.into_array(), dot_xa));
        let perp_mag = mag3(x_perp);
        if perp_mag == 0.0 {
            return Err(ConstructionError::DependentAxes);
        }
        if perp_mag < ILL_COND_THRESH {
            return Err(ConstructionError::IllConditionedAxes);
        }
        let x_unit = UnitVector3::try_normalize(
            Vector3::try_new(x_perp[0], x_perp[1], x_perp[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
        )
        .map_err(|_| ConstructionError::DependentAxes)?;
        let y_axis = UnitVector3::try_normalize(a_unit.cross(x_unit))
            .map_err(|_| ConstructionError::DependentAxes)?;
        Ok(Self {
            axis_origin: Point3::try_new(o[0], o[1], o[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            axis_dir: a_unit,
            radius,
            x_axis: x_unit,
            y_axis,
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
        self.axis_dir.as_vector()
    }

    /// Returns the radius.
    #[must_use]
    pub fn radius(&self) -> f64 {
        self.radius
    }

    /// Returns the unit reference direction for `u = 0`.
    #[must_use]
    pub fn x_axis(&self) -> Vector3 {
        self.x_axis.as_vector()
    }

    /// Returns the unit y-axis: `axis_dir × x_axis`.
    #[must_use]
    pub fn y_axis(&self) -> Vector3 {
        self.y_axis.as_vector()
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
        let scale = similarity_scale(transform).ok_or(TransformError::NotSimilarity)?;
        let new_origin = transform
            .try_apply_to_point(self.axis_origin)
            .map_err(|_| TransformError::NonFiniteResult)?;
        let new_axis_vec = transform
            .try_apply_to_vector(self.axis_dir.as_vector())
            .map_err(|_| TransformError::NonFiniteResult)?;
        let new_x_vec = transform
            .try_apply_to_vector(self.x_axis.as_vector())
            .map_err(|_| TransformError::NonFiniteResult)?;
        let new_radius = self.radius * scale;
        Self::try_new(new_origin, new_axis_vec, new_radius, new_x_vec)
            .map_err(|_| TransformError::DegenerateResult)
    }
}

impl TryFrom<CylinderRepr> for Cylinder {
    type Error = ConstructionError;
    fn try_from(repr: CylinderRepr) -> Result<Self, Self::Error> {
        let axis_origin = repr.axis_origin.into_array();
        if !all_finite3(axis_origin) || !repr.radius.is_finite() {
            return Err(ConstructionError::NonFiniteInput);
        }
        if repr.radius <= 0.0 {
            return Err(ConstructionError::NotPositive);
        }
        let a_unit =
            UnitVector3::try_normalize(repr.axis_dir).map_err(normalization_to_construction)?;
        let x_unit =
            UnitVector3::try_normalize(repr.x_axis).map_err(normalization_to_construction)?;
        if a_unit.dot(x_unit).abs() > UNIT_VECTOR_TOL {
            return Err(ConstructionError::DependentAxes);
        }
        let y_axis = UnitVector3::try_normalize(a_unit.cross(x_unit))
            .map_err(|_| ConstructionError::DependentAxes)?;
        Ok(Self {
            axis_origin: repr.axis_origin,
            axis_dir: a_unit,
            radius: repr.radius,
            x_axis: x_unit,
            y_axis,
        })
    }
}

impl From<Cylinder> for CylinderRepr {
    fn from(c: Cylinder) -> Self {
        Self {
            axis_origin: c.axis_origin,
            axis_dir: c.axis_dir.as_vector(),
            radius: c.radius,
            x_axis: c.x_axis.as_vector(),
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

    // Long due to certifying position plus first/second derivative bounds
    // (each independently wrapped in its own `GeometryError`-mapped
    // constructor) across every `DerivativeOrder`, not accidental
    // complexity.
    #[allow(clippy::too_many_lines)]
    fn evaluate(
        &self,
        u: f64,
        v: f64,
        order: DerivativeOrder,
        context: &EvaluationContext,
    ) -> Result<SurfaceEvaluation, GeometryError> {
        if !u.is_finite() || !v.is_finite() {
            return Err(GeometryError::NonFiniteParameter);
        }
        if !in_range(u, self.domain().u()) {
            return Err(GeometryError::OutsideDomain);
        }
        let o = self.axis_origin.into_array();
        let ad = self.axis_dir.into_array();
        let x_ax = self.x_axis.into_array();
        let y_ax = self.y_axis.into_array();

        let eval = exact_cylinder_eval(context.budget, o, ad, self.radius, x_ax, y_ax, u, v)?;
        let pos = Point3::try_new(eval.point[0], eval.point[1], eval.point[2]).map_err(|_| {
            GeometryError::Uncertified {
                reason: "cylinder position is non-finite".to_owned(),
            }
        })?;
        let position_error_bound =
            PositionBound::try_new(eval.position_error_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "position error bound overflowed representable range".to_owned(),
                }
            })?;
        let eval_scale = mag3(o) + self.radius.abs() + v.abs();
        check_tolerance(&context.tolerance, position_error_bound.get(), eval_scale)?;

        let du_error_bound = FirstDerivativeBound::try_new(eval.du_error_bound).map_err(|_| {
            GeometryError::Uncertified {
                reason: "first derivative error bound overflowed representable range".to_owned(),
            }
        })?;
        // ∂p/∂v = axis_dir is stored verbatim (no arithmetic), so it is
        // exact; ∂²p/∂u∂v = ∂²p/∂v² = 0 exactly.
        let dv_error_bound =
            FirstDerivativeBound::try_new(0.0).map_err(|_| GeometryError::Uncertified {
                reason: "zero derivative bound construction failed unexpectedly".to_owned(),
            })?;
        let duu_error_bound =
            SecondDerivativeBound::try_new(eval.duu_error_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "second derivative error bound overflowed representable range"
                        .to_owned(),
                }
            })?;
        let zero_second_bound =
            SecondDerivativeBound::try_new(0.0).map_err(|_| GeometryError::Uncertified {
                reason: "zero derivative bound construction failed unexpectedly".to_owned(),
            })?;

        let to_vec3 = |arr: [f64; 3], what: &'static str| {
            Vector3::try_new(arr[0], arr[1], arr[2]).map_err(|_| GeometryError::Uncertified {
                reason: format!("{what} non-finite"),
            })
        };

        let (du, dv, duu, duv, dvv, first_u_eb, first_v_eb, duu_eb, duv_eb, dvv_eb) = match order {
            DerivativeOrder::Position => {
                (None, None, None, None, None, None, None, None, None, None)
            }
            DerivativeOrder::First => {
                let du = to_vec3(eval.du, "cylinder first u-derivative")?;
                let dv = to_vec3(ad, "axis_dir")?;
                (
                    Some(du),
                    Some(dv),
                    None,
                    None,
                    None,
                    Some(du_error_bound),
                    Some(dv_error_bound),
                    None,
                    None,
                    None,
                )
            }
            DerivativeOrder::Second => {
                let du = to_vec3(eval.du, "cylinder first u-derivative")?;
                let dv = to_vec3(ad, "axis_dir")?;
                let duu = to_vec3(eval.duu, "cylinder second u-derivative")?;
                let zero = to_vec3([0.0, 0.0, 0.0], "zero vector")?;
                (
                    Some(du),
                    Some(dv),
                    Some(duu),
                    Some(zero),
                    Some(zero),
                    Some(du_error_bound),
                    Some(dv_error_bound),
                    Some(duu_error_bound),
                    Some(zero_second_bound),
                    Some(zero_second_bound),
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
        context: &EvaluationContext,
        output: &mut Vec<SurfaceProjection>,
    ) -> Result<(), GeometryError> {
        output.clear();
        let q = point.into_array();
        let o = self.axis_origin.into_array();
        let ad = self.axis_dir.into_array();
        let x_ax = self.x_axis.into_array();
        let y_ax = self.y_axis.into_array();

        let result = exact_cylinder_project(context.budget, q, o, ad, self.radius, x_ax, y_ax)?;
        let scale = mag3(q) + mag3(result.point);
        check_tolerance(&context.tolerance, result.point_residual_bound, scale)?;

        let proj =
            Point3::try_new(result.point[0], result.point[1], result.point[2]).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "cylinder projection point is non-finite".to_owned(),
                }
            })?;
        output.push(SurfaceProjection {
            u: ParameterValue::try_new(result.u).map_err(|_| GeometryError::Uncertified {
                reason: "cylinder projection u is non-finite".to_owned(),
            })?,
            v: ParameterValue::try_new(result.v).map_err(|_| GeometryError::Uncertified {
                reason: "cylinder projection v is non-finite".to_owned(),
            })?,
            point: proj,
            distance_bound: DistanceBound::try_new(result.distance_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "cylinder projection distance is non-finite or negative".to_owned(),
                }
            })?,
            parameter_error_bound: result.parameter_error_bound,
            point_residual_bound: PositionBound::try_new(result.point_residual_bound).map_err(
                |_| GeometryError::Uncertified {
                    reason: "cylinder projection point residual bound is non-finite or negative"
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
    use amphion_foundation::{Point3, ToleranceContext, Vector3};
    use serde_json::json;

    use crate::analytic::helpers::ILL_COND_THRESH;
    use crate::traits::SurfaceEvaluator;
    use crate::{DerivativeOrder, EvaluationContext, GeometryError};

    use super::{ConstructionError, Cylinder, CylinderRepr};

    fn tol() -> ToleranceContext {
        ToleranceContext::try_new(1e-9, 1e-8, 1e-10, 1e-12).unwrap()
    }

    fn ctx() -> EvaluationContext {
        EvaluationContext::new(tol())
    }

    fn dist3(a: Point3, b: Point3) -> f64 {
        let [ax, ay, az] = a.into_array();
        let [bx, by, bz] = b.into_array();
        ((ax - bx).powi(2) + (ay - by).powi(2) + (az - bz).powi(2)).sqrt()
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
    fn cylinder_evaluate_matches_known_values() {
        // unit_cylinder: axis_origin=(0,0,0), axis_dir=(0,0,1), radius=1,
        // x_axis=(1,0,0), y_axis=(0,1,0). At (u,v)=(0,0): p=(1,0,0). At
        // (u,v)=(0,5): p=(1,0,5), du=(0,1,0), dv=(0,0,1), duu=(-1,0,0).
        let c = unit_cylinder();
        let eval = c
            .evaluate(0.0, 0.0, DerivativeOrder::Position, &ctx())
            .unwrap();
        let [px, py, pz] = eval.position.into_array();
        assert!((px - 1.0).abs() < 1e-9, "px={px}");
        assert!(py.abs() < 1e-9, "py={py}");
        assert!(pz.abs() < 1e-9, "pz={pz}");

        let eval = c
            .evaluate(0.0, 5.0, DerivativeOrder::Second, &ctx())
            .unwrap();
        let [px, py, pz] = eval.position.into_array();
        assert!((px - 1.0).abs() < 1e-9, "px={px}");
        assert!(py.abs() < 1e-9, "py={py}");
        assert!((pz - 5.0).abs() < 1e-9, "pz={pz}");
        let [dux, duy, duz] = eval.du.unwrap().into_array();
        assert!(dux.abs() < 1e-9, "dux={dux}");
        assert!((duy - 1.0).abs() < 1e-9, "duy={duy}");
        assert!(duz.abs() < 1e-9, "duz={duz}");
        let [dvx, dvy, dvz] = eval.dv.unwrap().into_array();
        assert!(dvx.abs() < 1e-9 && dvy.abs() < 1e-9 && (dvz - 1.0).abs() < 1e-9);
        let [duux, duuy, duuz] = eval.duu.unwrap().into_array();
        assert!((duux - (-1.0)).abs() < 1e-9, "duux={duux}");
        assert!(duuy.abs() < 1e-9 && duuz.abs() < 1e-9);
    }

    #[test]
    fn cylinder_evaluate_rejects_out_of_domain() {
        let c = unit_cylinder();
        assert_eq!(
            c.evaluate(-0.001, 0.0, DerivativeOrder::Position, &ctx()),
            Err(GeometryError::OutsideDomain)
        );
        assert_eq!(
            c.evaluate(
                std::f64::consts::TAU,
                0.0,
                DerivativeOrder::Position,
                &ctx()
            ),
            Err(GeometryError::OutsideDomain)
        );
    }

    #[test]
    fn cylinder_evaluate_rejects_non_finite() {
        let c = unit_cylinder();
        assert_eq!(
            c.evaluate(f64::NAN, 0.0, DerivativeOrder::Position, &ctx()),
            Err(GeometryError::NonFiniteParameter)
        );
        assert_eq!(
            c.evaluate(0.0, f64::INFINITY, DerivativeOrder::Position, &ctx()),
            Err(GeometryError::NonFiniteParameter)
        );
    }

    #[test]
    fn cylinder_project_matches_known_values() {
        // axis_origin=(0,0,0), axis_dir=(0,0,1), radius=3, x_axis=(1,0,0).
        // q=(2,0,3): v=3, in-plane offset (2,0) ⇒ nearest point=(3,0,3),
        // distance=|2-3|=1, u=atan2(0,2)=0.
        let c = Cylinder::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            3.0,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let q = Point3::try_new(2.0, 0.0, 3.0).unwrap();
        let projs = c.project(q, &ctx()).unwrap();
        assert_eq!(projs.len(), 1);
        let p = &projs[0];
        let [px, py, pz] = p.point.into_array();
        assert!((px - 3.0).abs() < 1e-9, "px={px}");
        assert!(py.abs() < 1e-9, "py={py}");
        assert!((pz - 3.0).abs() < 1e-9, "pz={pz}");
        assert!((p.distance_bound.get() - 1.0).abs() < 1e-9);
        assert!(p.u.get().abs() < 1e-9);
        assert!((p.v.get() - 3.0).abs() < 1e-9);
        let actual = dist3(q, p.point);
        assert!(actual <= p.distance_bound.get());
    }

    #[test]
    fn cylinder_project_into_clears_output_on_error() {
        // Querying exactly on the cylinder axis is singular: the in-plane
        // offset is zero, so there is no unique nearest point / angle.
        let c = unit_cylinder();
        let mut output = vec![];
        let err = c.project_into(Point3::try_new(0.0, 0.0, 0.0).unwrap(), &ctx(), &mut output);
        assert_eq!(err.unwrap_err(), GeometryError::Singular);
        assert!(output.is_empty());
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
    fn cylinder_serde_normalizes_axis_and_rejects_dependent_axes_and_bad_radius() {
        // Foundation's UnitVector3::try_normalize is lenient: a non-unit but
        // finite, non-zero axis_dir is silently renormalized rather than
        // rejected.
        let normalized_axis: CylinderRepr = serde_json::from_value(json!({
            "axis_origin": [1.0, 2.0, 3.0],
            "axis_dir": [2.0, 0.0, 0.0],
            "radius": 2.5,
            "x_axis": [0.0, 1.0, 0.0]
        }))
        .unwrap();
        let cylinder = Cylinder::try_from(normalized_axis).unwrap();
        assert_eq!(
            cylinder.axis_dir(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap()
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
