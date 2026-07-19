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
//! The apex `v = 0` is a conical singularity: `∂p/∂u = 0` there.  Requesting
//! first- or second-order derivatives at `v = 0` returns
//! [`GeometryError::Singular`] (this is a geometric fact, independent of
//! trig certification).
//!
//! Derivatives:
//! ```text
//! ∂p/∂u    =  v·tan(α)·(−sin u·x_axis + cos u·y_axis)
//! ∂p/∂v    =  axis + tan(α)·(cos u·x_axis + sin u·y_axis)
//! ∂²p/∂u²  =  −v·tan(α)·(cos u·x_axis + sin u·y_axis)
//! ∂²p/∂u∂v =  tan(α)·(−sin u·x_axis + cos u·y_axis)
//! ∂²p/∂v²  =  0
//! ```
//!
//! Projection: the correct nappe (sign of the reported `v`) is `s = sign(h)`
//! where `h = (q − apex)·axis`; on the equatorial plane (`h = 0`) both
//! nappes are equidistant and both certified solutions are returned.
//! Returns [`GeometryError::Singular`] when the query lies exactly on the
//! cone's axis (no unique nearest point or azimuthal angle).

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
        ILL_COND_THRESH, UNIT_VECTOR_TOL, all_finite3, check_angular_tolerance, check_tolerance,
        dot3, exact_cone_eval, exact_cone_project, in_range, mag3, normalization_to_construction,
        scale3, sub3,
    },
    transform::similarity_scale,
};

fn angular_range() -> ParameterRange {
    match ParameterRange::try_new(Some(0.0), Some(TAU), Some(TAU)) {
        Ok(range) => range,
        Err(error) => panic!("cone angular domain is a static invariant: {error:?}"),
    }
}

fn unbounded_range() -> ParameterRange {
    match ParameterRange::try_new(None, None, None) {
        Ok(range) => range,
        Err(error) => panic!("cone v-domain is a static invariant: {error:?}"),
    }
}

#[derive(Serialize, Deserialize)]
struct ConeRepr {
    apex: Point3,
    axis: UnitVector3,
    half_angle: f64,
    x_axis: UnitVector3,
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
    axis: UnitVector3,
    half_angle: f64,
    x_axis: UnitVector3,
    y_axis: UnitVector3,
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
        if !all_finite3(ap) || !half_angle.is_finite() {
            return Err(ConstructionError::NonFiniteInput);
        }
        if half_angle <= 0.0 || half_angle >= std::f64::consts::FRAC_PI_2 {
            return Err(ConstructionError::InvalidHalfAngle);
        }
        let a_unit = UnitVector3::try_normalize(axis).map_err(normalization_to_construction)?;
        let x_norm = UnitVector3::try_normalize(x_axis).map_err(normalization_to_construction)?;
        // Orthogonalize x against axis.
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
            apex: Point3::try_new(ap[0], ap[1], ap[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            axis: a_unit,
            half_angle,
            x_axis: x_unit,
            y_axis,
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
        self.axis.as_vector()
    }

    /// Returns the half-angle in radians (strictly between 0 and π/2).
    #[must_use]
    pub fn half_angle(&self) -> f64 {
        self.half_angle
    }

    /// Returns the unit reference direction for `u = 0`.
    #[must_use]
    pub fn x_axis(&self) -> Vector3 {
        self.x_axis.as_vector()
    }

    /// Returns the unit y-axis: `axis × x_axis`.
    #[must_use]
    pub fn y_axis(&self) -> Vector3 {
        self.y_axis.as_vector()
    }

    /// Applies a similarity `transform` (rigid motion plus uniform scale, no
    /// reflection) to this cone, returning a new cone with the same
    /// `half_angle` (an angle is invariant under a similarity transform).
    ///
    /// A general affine transform does not map a circular cone to a
    /// circular cone, so only similarity transforms are accepted; see the
    /// `transform` module documentation for the (provisional, heuristic)
    /// similarity test.
    ///
    /// # Errors
    ///
    /// - [`TransformError::NotSimilarity`] — the transform's linear part is
    ///   not (within tolerance) a uniform-scale rotation
    /// - [`TransformError::NonFiniteResult`] — the transformed apex or axes
    ///   contain a non-finite component
    /// - [`TransformError::DegenerateResult`] — the transformed axes fail
    ///   cone construction
    pub fn try_transform(&self, transform: &Transform3) -> Result<Self, TransformError> {
        // The scale factor itself is irrelevant to a cone (half_angle and
        // the apex-relative direction fully determine its shape); only its
        // existence certifies that `transform` is a similarity.
        similarity_scale(transform).ok_or(TransformError::NotSimilarity)?;
        let new_apex = transform
            .try_apply_to_point(self.apex)
            .map_err(|_| TransformError::NonFiniteResult)?;
        let new_axis_vec = transform
            .try_apply_to_vector(self.axis.as_vector())
            .map_err(|_| TransformError::NonFiniteResult)?;
        let new_x_vec = transform
            .try_apply_to_vector(self.x_axis.as_vector())
            .map_err(|_| TransformError::NonFiniteResult)?;
        Self::try_new(new_apex, new_axis_vec, self.half_angle, new_x_vec)
            .map_err(|_| TransformError::DegenerateResult)
    }
}

impl TryFrom<ConeRepr> for Cone {
    type Error = ConstructionError;
    fn try_from(repr: ConeRepr) -> Result<Self, Self::Error> {
        let apex = repr.apex.into_array();
        if !all_finite3(apex) || !repr.half_angle.is_finite() {
            return Err(ConstructionError::NonFiniteInput);
        }
        if repr.half_angle <= 0.0 || repr.half_angle >= std::f64::consts::FRAC_PI_2 {
            return Err(ConstructionError::InvalidHalfAngle);
        }
        let a_unit = repr.axis;
        let x_unit = repr.x_axis;
        if a_unit.dot(x_unit).abs() > UNIT_VECTOR_TOL {
            return Err(ConstructionError::DependentAxes);
        }
        let y_axis = UnitVector3::try_normalize(a_unit.cross(x_unit))
            .map_err(|_| ConstructionError::DependentAxes)?;
        Ok(Self {
            apex: repr.apex,
            axis: a_unit,
            half_angle: repr.half_angle,
            x_axis: x_unit,
            y_axis,
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
        // Apex singularity: ∂p/∂u = 0 at v = 0. This is a genuine geometric
        // singularity independent of trig certification (it holds regardless
        // of how accurately sin/cos/tan are computed), so it is checked
        // before calling the certified evaluator below.
        if v == 0.0 && order != DerivativeOrder::Position {
            return Err(GeometryError::Singular);
        }

        let ap = self.apex.into_array();
        let ax = self.axis.into_array();
        let xa = self.x_axis.into_array();
        let ya = self.y_axis.into_array();

        let eval = exact_cone_eval(context.budget, ap, ax, self.half_angle, xa, ya, u, v)?;
        let pos = Point3::try_new(eval.point[0], eval.point[1], eval.point[2]).map_err(|_| {
            GeometryError::Uncertified {
                reason: "cone position is non-finite".to_owned(),
            }
        })?;
        let position_error_bound =
            PositionBound::try_new(eval.position_error_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "position error bound overflowed representable range".to_owned(),
                }
            })?;
        let eval_scale = mag3(ap) + v.abs() * (1.0 + self.half_angle.tan().abs());
        check_tolerance(&context.tolerance, position_error_bound.get(), eval_scale)?;

        let du_error_bound = FirstDerivativeBound::try_new(eval.du_error_bound).map_err(|_| {
            GeometryError::Uncertified {
                reason: "first derivative error bound overflowed representable range".to_owned(),
            }
        })?;
        let dv_error_bound = FirstDerivativeBound::try_new(eval.dv_error_bound).map_err(|_| {
            GeometryError::Uncertified {
                reason: "first derivative error bound overflowed representable range".to_owned(),
            }
        })?;
        let duu_error_bound =
            SecondDerivativeBound::try_new(eval.duu_error_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "second derivative error bound overflowed representable range"
                        .to_owned(),
                }
            })?;
        let duv_error_bound =
            SecondDerivativeBound::try_new(eval.duv_error_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "second derivative error bound overflowed representable range"
                        .to_owned(),
                }
            })?;
        // ∂²p/∂v² = 0 exactly (the parameterization is affine in v along a
        // fixed ray direction).
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
                let du = to_vec3(eval.du, "cone first u-derivative")?;
                let dv = to_vec3(eval.dv, "cone first v-derivative")?;
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
                let du = to_vec3(eval.du, "cone first u-derivative")?;
                let dv = to_vec3(eval.dv, "cone first v-derivative")?;
                let duu = to_vec3(eval.duu, "cone second u-derivative")?;
                let duv = to_vec3(eval.duv, "cone mixed second derivative")?;
                let zero = to_vec3([0.0, 0.0, 0.0], "zero vector")?;
                (
                    Some(du),
                    Some(dv),
                    Some(duu),
                    Some(duv),
                    Some(zero),
                    Some(du_error_bound),
                    Some(dv_error_bound),
                    Some(duu_error_bound),
                    Some(duv_error_bound),
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
        let ap = self.apex.into_array();
        let ax = self.axis.into_array();
        let xa = self.x_axis.into_array();
        let ya = self.y_axis.into_array();

        let result = exact_cone_project(context.budget, q, ap, ax, self.half_angle, xa, ya)?;
        let mut certified = Vec::new();
        for projection in [Some(result.primary), result.secondary]
            .into_iter()
            .flatten()
        {
            let scale = mag3(q) + mag3(projection.point);
            check_tolerance(&context.tolerance, projection.point_residual_bound, scale)?;
            check_angular_tolerance(&context.tolerance, projection.u_error_bound)?;
            check_tolerance(&context.tolerance, projection.v_error_bound, 1.0)?;

            let proj = Point3::try_new(
                projection.point[0],
                projection.point[1],
                projection.point[2],
            )
            .map_err(|_| GeometryError::Uncertified {
                reason: "cone projection point is non-finite".to_owned(),
            })?;
            certified.push(SurfaceProjection {
                u: ParameterValue::try_new(projection.u).map_err(|_| {
                    GeometryError::Uncertified {
                        reason: "cone projection u is non-finite".to_owned(),
                    }
                })?,
                v: ParameterValue::try_new(projection.v).map_err(|_| {
                    GeometryError::Uncertified {
                        reason: "cone projection v is non-finite".to_owned(),
                    }
                })?,
                point: proj,
                distance_bound: DistanceBound::try_new(projection.distance_bound).map_err(
                    |_| GeometryError::Uncertified {
                        reason: "cone projection distance is non-finite or negative".to_owned(),
                    },
                )?,
                parameter_error_bound: projection.parameter_error_bound,
                point_residual_bound: PositionBound::try_new(projection.point_residual_bound)
                    .map_err(|_| GeometryError::Uncertified {
                        reason: "cone projection point residual bound is non-finite or negative"
                            .to_owned(),
                    })?,
            });
        }
        output.extend(certified);
        Ok(())
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    #![allow(clippy::float_cmp)]

    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, TAU};

    use amphion_foundation::{Point3, ToleranceContext, Vector3};
    use serde_json::json;

    use crate::analytic::helpers::ILL_COND_THRESH;
    use crate::traits::SurfaceEvaluator;
    use crate::{DerivativeOrder, EvaluationContext, GeometryError};

    use super::{Cone, ConeRepr, ConstructionError};

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
    fn cone_evaluate_apex_derivative_singular() {
        // The apex ∂p/∂u = 0 singularity is a geometric fact independent of
        // trig certification, so it is still reported (and takes priority
        // over any certified computation).
        let c = std_cone();
        assert_eq!(
            c.evaluate(0.0, 0.0, DerivativeOrder::First, &ctx()),
            Err(GeometryError::Singular)
        );
        assert_eq!(
            c.evaluate(0.0, 0.0, DerivativeOrder::Second, &ctx()),
            Err(GeometryError::Singular)
        );
    }

    #[test]
    fn cone_evaluate_matches_known_values() {
        // std_cone: apex=(0,0,0), axis=(0,0,1), half_angle=π/4
        // (tan(π/4)=1), x_axis=(1,0,0), y_axis=(0,1,0). At (u,v)=(0,0):
        // p=apex=(0,0,0). At (u,v)=(0,5): p=(5,0,5), du=(0,5,0),
        // dv=(1,0,1), duu=(-5,0,0), duv=(0,1,0), dvv=(0,0,0).
        let c = std_cone();
        let eval = c
            .evaluate(0.0, 0.0, DerivativeOrder::Position, &ctx())
            .unwrap();
        let [px, py, pz] = eval.position.into_array();
        assert!(px.abs() < 1e-9 && py.abs() < 1e-9 && pz.abs() < 1e-9);

        let eval = c
            .evaluate(0.0, 5.0, DerivativeOrder::Second, &ctx())
            .unwrap();
        let [px, py, pz] = eval.position.into_array();
        assert!((px - 5.0).abs() < 1e-9, "px={px}");
        assert!(py.abs() < 1e-9, "py={py}");
        assert!((pz - 5.0).abs() < 1e-9, "pz={pz}");

        let [dux, duy, duz] = eval.du.unwrap().into_array();
        assert!(dux.abs() < 1e-9, "dux={dux}");
        assert!((duy - 5.0).abs() < 1e-9, "duy={duy}");
        assert!(duz.abs() < 1e-9, "duz={duz}");

        let [dvx, dvy, dvz] = eval.dv.unwrap().into_array();
        assert!((dvx - 1.0).abs() < 1e-9, "dvx={dvx}");
        assert!(dvy.abs() < 1e-9, "dvy={dvy}");
        assert!((dvz - 1.0).abs() < 1e-9, "dvz={dvz}");

        let [duux, duuy, duuz] = eval.duu.unwrap().into_array();
        assert!((duux - (-5.0)).abs() < 1e-9, "duux={duux}");
        assert!(duuy.abs() < 1e-9 && duuz.abs() < 1e-9);

        let [duvx, duvy, duvz] = eval.duv.unwrap().into_array();
        assert!(duvx.abs() < 1e-9, "duvx={duvx}");
        assert!((duvy - 1.0).abs() < 1e-9, "duvy={duvy}");
        assert!(duvz.abs() < 1e-9, "duvz={duvz}");

        let [dvvx, dvvy, dvvz] = eval.dvv.unwrap().into_array();
        assert!(dvvx.abs() < 1e-9 && dvvy.abs() < 1e-9 && dvvz.abs() < 1e-9);
    }

    #[test]
    fn cone_evaluate_rejects_out_of_domain() {
        let c = std_cone();
        assert_eq!(
            c.evaluate(-0.001, 1.0, DerivativeOrder::Position, &ctx()),
            Err(GeometryError::OutsideDomain)
        );
        assert_eq!(
            c.evaluate(TAU, 1.0, DerivativeOrder::Position, &ctx()),
            Err(GeometryError::OutsideDomain)
        );
    }

    #[test]
    fn cone_evaluate_rejects_non_finite() {
        let c = std_cone();
        assert_eq!(
            c.evaluate(f64::NAN, 1.0, DerivativeOrder::Position, &ctx()),
            Err(GeometryError::NonFiniteParameter)
        );
        assert_eq!(
            c.evaluate(0.0, f64::INFINITY, DerivativeOrder::Position, &ctx()),
            Err(GeometryError::NonFiniteParameter)
        );
    }

    #[test]
    fn cone_project_matches_known_values() {
        // std_cone: apex=(0,0,0), axis=(0,0,1), half_angle=π/4 (tan=1),
        // x_axis=(1,0,0), y_axis=(0,1,0). q=(2,0,1): h=1, in-plane offset
        // (2,0) ⇒ radial=2. t* = cos(π/4) + 2·sin(π/4) = 3√2/2,
        // v = t*·cos(π/4) = 1.5, rho = t*·sin(π/4) = 1.5 ⇒ nearest
        // point=(1.5,0,1.5), sq_dist = h² + radial² − t*² = 1+4−4.5 = 0.5.
        let c = std_cone();
        let q = Point3::try_new(2.0, 0.0, 1.0).unwrap();
        let projs = c.project(q, &ctx()).unwrap();
        assert_eq!(projs.len(), 1);
        let p = &projs[0];
        let [px, py, pz] = p.point.into_array();
        assert!((px - 1.5).abs() < 1e-9, "px={px}");
        assert!(py.abs() < 1e-9, "py={py}");
        assert!((pz - 1.5).abs() < 1e-9, "pz={pz}");
        let expected_dist = 0.5_f64.sqrt();
        assert!((p.distance_bound.get() - expected_dist).abs() < 1e-9);
        assert!(p.u.get().abs() < 1e-9, "u={}", p.u.get());
        assert!((p.v.get() - 1.5).abs() < 1e-9, "v={}", p.v.get());
        let actual = dist3(q, p.point);
        assert!(actual <= p.distance_bound.get());
    }

    #[test]
    fn cone_project_equatorial_returns_two_solutions() {
        let c = std_cone();
        let q = Point3::try_new(2.0, 0.0, 0.0).unwrap();
        let projs = c.project(q, &ctx()).unwrap();
        assert_eq!(projs.len(), 2);
        assert!(projs[0].v.get() > 0.0);
        assert!(projs[1].v.get() < 0.0);
    }

    #[test]
    fn cone_project_into_clears_output_on_error() {
        // Querying exactly on the cone axis is singular: the in-plane
        // offset is zero, so there is no unique nearest point / azimuthal
        // angle.
        let c = std_cone();
        let mut output = vec![];
        let err = c.project_into(Point3::try_new(0.0, 0.0, 5.0).unwrap(), &ctx(), &mut output);
        assert_eq!(err.unwrap_err(), GeometryError::Singular);
        assert!(output.is_empty());
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
        assert_eq!(c.axis().into_array(), decoded.axis().into_array());
        assert_eq!(c.x_axis().into_array(), decoded.x_axis().into_array());
    }

    #[test]
    fn cone_serde_rejects_non_unit_axis_and_bad_angle() {
        assert!(
            serde_json::from_str::<Cone>(
                r#"{"apex":[1.0,2.0,3.0],"axis":[2.0,0.0,0.0],"half_angle":0.5,"x_axis":[0.0,1.0,0.0]}"#
            )
            .is_err()
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

    #[test]
    fn cone_try_transform_identity_is_noop() {
        let c = std_cone();
        let out = c
            .try_transform(&amphion_foundation::Transform3::IDENTITY)
            .unwrap();
        assert_eq!(out, c);
    }

    #[test]
    fn cone_try_transform_similarity_preserves_half_angle() {
        // Rotation by 90° about Z, uniform scale 2, plus translation.
        let m = [
            0.0, -2.0, 0.0, 5.0, //
            2.0, 0.0, 0.0, -3.0, //
            0.0, 0.0, 2.0, 7.0,
        ];
        let t = amphion_foundation::Transform3::try_from_row_major(m).unwrap();
        let c = std_cone();
        let out = c.try_transform(&t).unwrap();
        assert!((out.half_angle() - FRAC_PI_4).abs() < 1e-9);
        let [ax, ay, az] = out.apex().into_array();
        assert!((ax - 5.0).abs() < 1e-9);
        assert!((ay - (-3.0)).abs() < 1e-9);
        assert!((az - 7.0).abs() < 1e-9);
    }

    #[test]
    fn cone_try_transform_rejects_non_similarity() {
        let m = [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 2.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0,
        ];
        let t = amphion_foundation::Transform3::try_from_row_major(m).unwrap();
        let c = std_cone();
        assert_eq!(
            c.try_transform(&t),
            Err(super::TransformError::NotSimilarity)
        );
    }
}
