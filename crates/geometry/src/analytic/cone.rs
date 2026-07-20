//! Analytic right circular cone surface.
//!
//! # Parameterization
//!
//! ```text
//! p(u, v) = apex + v·axis + v·tan(α)·(cos(u)·x_axis + sin(u)·y_axis)
//! ```
//!
//! where `α = half_angle` and the displayed frame is the mathematical ideal
//! Gram–Schmidt frame derived from frozen `axis`/`x_axis` seed bits. The
//! parameter `v` is the signed ideal-axis distance from the apex; both nappes
//! (`v > 0` and `v < 0`) are included.
//!
//! - U domain: `[0, 2π]` with period `2π`; `2π` is evaluated as the `0` seam
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
//! Projection: both meridian rays and the apex are constructed and compared by
//! certified squared-distance intervals. On a proved equatorial tie (`h = 0`)
//! both ray solutions are returned in deterministic positive-then-negative
//! order.
//! Returns [`GeometryError::Singular`] when the query lies exactly on the
//! cone's axis away from the apex (no unique nearest point or azimuthal
//! angle). The apex itself returns the canonical `(u, v) = (0, 0)` projection.

#![allow(
    clippy::many_single_char_names,
    clippy::missing_panics_doc,
    clippy::similar_names
)]

use amphion_foundation::{Point3, SchemaVersion, Transform3, UnitVector3, Vector3};
use serde::{Deserialize, Serialize};

use crate::traits::SurfaceEvaluator;
use crate::{
    AngularParameterBound, DerivativeOrder, DistanceBound, EvaluationContext, FirstDerivativeBound,
    GeometryError, LinearParameterBound, ParameterErrorBound, ParameterRange, ParameterValue,
    PositionBound, SecondDerivativeBound, SurfaceDomain, SurfaceEvaluation, SurfaceKind,
    SurfaceProjection,
};

use super::{
    ConstructionError, TransformError,
    helpers::{
        ILL_COND_THRESH, all_finite3, canonicalize_periodic_endpoint, certified_tan_upper,
        check_angular_tolerance, check_derivative_limit, check_parametric_tolerance,
        check_tolerance, dot3, exact_cone_eval, exact_cone_project, in_range, mag3,
        normalization_to_construction, normalized_cross3, scale3, sub3,
    },
    transform::{
        exact_transform_point, exact_transform_vector, is_identity_transform, similarity_scale,
    },
};

fn angular_range() -> ParameterRange {
    ParameterRange::full_angular_period()
}

fn unbounded_range() -> ParameterRange {
    ParameterRange::unbounded()
}

/// Current serialized schema version for this module's primitive.
const SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0);

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConeRepr {
    version: SchemaVersion,
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
/// U ∈ `[0, 2π]` (periodic, with `2π` an alias of `0`),
/// V ∈ `(−∞, +∞)`. `α = half_angle` ∈ `(0, π/2)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ConeRepr", into = "ConeRepr")]
pub struct Cone {
    apex: Point3,
    axis: Vector3,
    half_angle: f64,
    x_axis: Vector3,
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
            axis: a_unit.as_vector(),
            half_angle,
            x_axis: x_unit.as_vector(),
            y_axis,
        })
    }

    /// Returns the apex point.
    #[must_use]
    pub fn apex(&self) -> Point3 {
        self.apex
    }

    /// Returns the frozen finite axis seed used to derive the ideal unit axis.
    #[must_use]
    pub fn axis(&self) -> Vector3 {
        self.axis
    }

    /// Returns the half-angle in radians (strictly between 0 and π/2).
    #[must_use]
    pub fn half_angle(&self) -> f64 {
        self.half_angle
    }

    /// Returns the frozen finite X seed used to derive the ideal `u = 0`
    /// direction.
    #[must_use]
    pub fn x_axis(&self) -> Vector3 {
        self.x_axis
    }

    /// Returns the stored finite Y approximation. Evaluation derives the ideal
    /// Y direction from the frozen axis and X seeds.
    #[must_use]
    pub fn y_axis(&self) -> Vector3 {
        self.y_axis.as_vector()
    }

    /// Applies a similarity `transform` (rigid motion plus uniform scale, no
    /// reflection) to this cone, returning a new cone with the same
    /// `half_angle` (an angle is invariant under a similarity transform).
    ///
    /// A general affine transform does not map a circular cone to a
    /// circular cone, so only transforms whose finite matrix entries satisfy
    /// the similarity identities exactly are accepted.
    ///
    /// # Errors
    ///
    /// - [`TransformError::NotSimilarity`] — the transform's linear part is
    ///   not an exact uniform-scale rotation over its stored matrix entries
    /// - [`TransformError::UnrepresentableScale`] — the exact uniform scale
    ///   cannot be represented by the current transform API
    /// - [`TransformError::UnrepresentableResult`] — an exact transformed apex
    ///   or seed component is not representable as `f64`
    /// - [`TransformError::DegenerateResult`] — the transformed frozen seeds
    ///   become zero or dependent
    pub fn try_transform(&self, transform: &Transform3) -> Result<Self, TransformError> {
        if is_identity_transform(transform) {
            return Ok(self.clone());
        }
        // The scale factor itself is irrelevant to a cone (half_angle and
        // the apex-relative direction fully determine its shape); only its
        // existence certifies that `transform` is a similarity.
        similarity_scale(transform)?;
        let new_apex = exact_transform_point(transform, self.apex)?;
        let new_axis_vec = exact_transform_vector(transform, self.axis)?;
        let new_x_vec = exact_transform_vector(transform, self.x_axis)?;
        Self::try_from(ConeRepr {
            version: SCHEMA_VERSION,
            apex: new_apex,
            axis: new_axis_vec,
            half_angle: self.half_angle,
            x_axis: new_x_vec,
        })
        .map_err(|_| TransformError::DegenerateResult)
    }
}

impl TryFrom<ConeRepr> for Cone {
    type Error = ConstructionError;
    fn try_from(repr: ConeRepr) -> Result<Self, Self::Error> {
        if repr.version != SCHEMA_VERSION {
            return Err(ConstructionError::UnsupportedSchemaVersion {
                found: repr.version,
                supported: SCHEMA_VERSION,
            });
        }
        let apex = repr.apex.into_array();
        if !all_finite3(apex) || !repr.half_angle.is_finite() {
            return Err(ConstructionError::NonFiniteInput);
        }
        if repr.half_angle <= 0.0 || repr.half_angle >= std::f64::consts::FRAC_PI_2 {
            return Err(ConstructionError::InvalidHalfAngle);
        }
        let axis = repr.axis;
        let x_axis = repr.x_axis;
        let y_axis = normalized_cross3(axis, x_axis)?;
        Ok(Self {
            apex: repr.apex,
            axis,
            half_angle: repr.half_angle,
            x_axis,
            y_axis,
        })
    }
}

impl From<Cone> for ConeRepr {
    fn from(c: Cone) -> Self {
        Self {
            version: SCHEMA_VERSION,
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
        let u_range = self.domain().u();
        if !in_range(u, u_range) {
            return Err(GeometryError::OutsideDomain);
        }
        let u = canonicalize_periodic_endpoint(u, u_range);
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

        let eval = exact_cone_eval(context.budget(), ap, ax, self.half_angle, xa, u, v)?;
        let pos = Point3::try_new(eval.point[0], eval.point[1], eval.point[2]).map_err(|_| {
            GeometryError::Uncertified {
                reason: "cone position is non-finite".to_owned(),
            }
        })?;
        // Certified tan(half_angle) upper bound. If the certified trig backend
        // exhausts its budget, we must return Uncertified rather than silently
        // substitute an uncertified host tan. Needed both for the ideal-frame
        // radial coefficient and the eval scale.
        let tan_upper =
            certified_tan_upper(self.half_angle, context.budget()).ok_or_else(|| {
                GeometryError::Uncertified {
                    reason: "cone half-angle tangent could not be certified within budget"
                        .to_owned(),
                }
            })?;
        let position_error_bound =
            PositionBound::try_new(eval.position_error_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "position error bound overflowed representable range".to_owned(),
                }
            })?;
        let eval_scale = mag3(ap) + v.abs() * (1.0 + tan_upper);
        check_tolerance(&context.tolerance(), position_error_bound.get(), eval_scale)?;

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
                check_derivative_limit(
                    du_error_bound.get(),
                    context.derivative_limits().surface().du().get(),
                )?;
                check_derivative_limit(
                    dv_error_bound.get(),
                    context.derivative_limits().surface().dv().get(),
                )?;
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
                check_derivative_limit(
                    du_error_bound.get(),
                    context.derivative_limits().surface().du().get(),
                )?;
                check_derivative_limit(
                    dv_error_bound.get(),
                    context.derivative_limits().surface().dv().get(),
                )?;
                check_derivative_limit(
                    duu_error_bound.get(),
                    context.derivative_limits().surface().duu().get(),
                )?;
                check_derivative_limit(
                    duv_error_bound.get(),
                    context.derivative_limits().surface().duv().get(),
                )?;
                check_derivative_limit(
                    zero_second_bound.get(),
                    context.derivative_limits().surface().dvv().get(),
                )?;
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

        let result = exact_cone_project(context.budget(), q, ap, ax, self.half_angle, xa)?;
        let mut certified = Vec::new();
        for projection in [Some(result.primary), result.secondary]
            .into_iter()
            .flatten()
        {
            let scale = mag3(q) + mag3(projection.point);
            check_tolerance(&context.tolerance(), projection.point_residual_bound, scale)?;
            check_angular_tolerance(&context.tolerance(), projection.u_error_bound)?;
            check_parametric_tolerance(&context.tolerance(), projection.v_error_bound)?;

            let proj = Point3::try_new(
                projection.point[0],
                projection.point[1],
                projection.point[2],
            )
            .map_err(|_| GeometryError::Uncertified {
                reason: "cone projection point is non-finite".to_owned(),
            })?;
            let ang_u = AngularParameterBound::try_new(projection.u_error_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "cone u angular bound is invalid".to_owned(),
                }
            })?;
            let lin_v = LinearParameterBound::try_new(projection.v_error_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "cone v linear bound is invalid".to_owned(),
                }
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
                u_error_bound: ParameterErrorBound::Angular(ang_u),
                v_error_bound: lin_v,
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
        EvaluationContext::unlimited(tol())
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

    /// Correction 10-E: when the certification budget is too small for the
    /// certified tangent, cone evaluation returns `Uncertified` rather than
    /// silently substituting an uncertified host `tan`.
    #[test]
    fn cone_eval_uncertified_on_starved_tan_budget() {
        use crate::CertificationBudget;
        let starved = EvaluationContext::unlimited(tol())
            .with_budget(CertificationBudget::try_new(1, 1 << 20).unwrap());
        let result = std_cone().evaluate(0.5, 1.0, DerivativeOrder::Position, &starved);
        assert!(
            matches!(result, Err(GeometryError::Uncertified { .. })),
            "starved tan budget must yield Uncertified, got {result:?}"
        );
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
            c.evaluate(TAU.next_up(), 1.0, DerivativeOrder::Position, &ctx()),
            Err(GeometryError::OutsideDomain)
        );
    }

    #[test]
    fn cone_evaluate_canonicalizes_periodic_endpoint() {
        let cone = std_cone();
        assert_eq!(
            cone.evaluate(0.0, 1.0, DerivativeOrder::Second, &ctx()),
            cone.evaluate(TAU, 1.0, DerivativeOrder::Second, &ctx())
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
    fn cone_serde_preserves_scaled_seeds_and_rejects_bad_angle() {
        let scaled: Cone = serde_json::from_str(
            r#"{"version":{"major":1,"minor":0},"apex":[1.0,2.0,3.0],"axis":[2.0,0.0,0.0],"half_angle":0.5,"x_axis":[0.0,1.0,0.0]}"#,
        )
        .unwrap();
        assert_eq!(scaled.axis().into_array(), [2.0, 0.0, 0.0]);

        let bad_frame: ConeRepr = serde_json::from_value(json!({
            "version": {"major": 1, "minor": 0},
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
            "version": {"major": 1, "minor": 0},
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
            r#"{"version":{"major":1,"minor":0},"apex":[NaN,0.0,0.0],"axis":[0.0,0.0,1.0],"half_angle":0.5,"x_axis":[1.0,0.0,0.0]}"#
        )
        .is_err());
        assert!(serde_json::from_str::<Cone>(
            r#"{"version":{"major":1,"minor":0},"apex":[0.0,0.0,0.0],"axis":[Infinity,0.0,1.0],"half_angle":0.5,"x_axis":[1.0,0.0,0.0]}"#
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
    fn cone_identity_transform_preserves_deserialized_frozen_seeds() {
        let cone: Cone = serde_json::from_value(json!({
            "version": {"major": 1, "minor": 0},
            "apex": [0.0, 0.0, 0.0],
            "axis": [0.0, 0.0, 1.0],
            "half_angle": FRAC_PI_4,
            "x_axis": [0.099_833_416_646_828_15, 0.0, 0.995_004_165_278_025_8]
        }))
        .unwrap();
        assert_eq!(
            cone.try_transform(&amphion_foundation::Transform3::IDENTITY)
                .unwrap(),
            cone
        );
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
        assert_eq!(out.axis().into_array(), [0.0, 0.0, 2.0]);
        assert_eq!(out.x_axis().into_array(), [0.0, 2.0, 0.0]);
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

    // ─── Blocker regression tests ────────────────────────────────────────────

    /// Blocker 5: q=(2,0,0), alpha=pi/4 → exactly two ordered solutions.
    /// Verified via exact h=0 axial coordinate.
    #[test]
    fn cone_project_equatorial_two_solutions() {
        let c = std_cone();
        // q=(2,0,0) is on the equatorial plane (h=0 for apex at origin with
        // axis=(0,0,1)).  Both nappes are equidistant.
        let q = Point3::try_new(2.0, 0.0, 0.0).unwrap();
        let projs = c.project(q, &ctx()).unwrap();
        assert_eq!(
            projs.len(),
            2,
            "expected 2 solutions for equatorial query, got {}",
            projs.len()
        );
        // Both u values must be in [0, TAU).
        for p in &projs {
            let u = p.u.get();
            assert!((0.0..TAU).contains(&u), "u={u} out of [0,TAU)");
        }
        // Both distance bounds must be positive.
        for p in &projs {
            assert!(p.distance_bound.get() >= 0.0);
        }
    }

    /// Blocker 5: h-neighbor (h ≠ 0) selects only one solution.
    #[test]
    fn cone_project_h_neighbor_one_solution() {
        let c = std_cone();
        // Slightly above equatorial plane → positive nappe only.
        let q_above = Point3::try_new(2.0, 0.0, f64::EPSILON).unwrap();
        let projs_above = c.project(q_above, &ctx()).unwrap();
        assert_eq!(projs_above.len(), 1, "h>0 neighbor should give 1 solution");
        let v_above = projs_above[0].v.get();
        assert!(v_above > 0.0, "positive nappe: v={v_above} must be > 0");

        // Slightly below equatorial plane → negative nappe only.
        let q_below = Point3::try_new(2.0, 0.0, -f64::EPSILON).unwrap();
        let projs_below = c.project(q_below, &ctx()).unwrap();
        assert_eq!(projs_below.len(), 1, "h<0 neighbor should give 1 solution");
        let v_below = projs_below[0].v.get();
        assert!(v_below < 0.0, "negative nappe: v={v_below} must be < 0");
    }

    /// Correction 10-G: mismatched `SchemaVersion` major must be rejected.
    #[test]
    fn cone_serde_version_rejection() {
        let invalid = serde_json::json!({
            "version": {"major": 99, "minor": 0},
            "apex": [0.0, 0.0, 0.0],
            "axis": [0.0, 0.0, 1.0],
            "half_angle": FRAC_PI_4,
            "x_axis": [1.0, 0.0, 0.0]
        });
        let result: Result<Cone, _> = serde_json::from_value(invalid);
        assert!(result.is_err(), "major=99 must be rejected");

        let missing = serde_json::json!({
            "apex": [0.0, 0.0, 0.0],
            "axis": [0.0, 0.0, 1.0],
            "half_angle": FRAC_PI_4,
            "x_axis": [1.0, 0.0, 0.0]
        });
        assert!(
            serde_json::from_value::<Cone>(missing).is_err(),
            "missing version must be rejected"
        );

        // Item 6: exact-match — a different minor is now rejected too.
        let wrong_minor = serde_json::json!({
            "version": {"major": 1, "minor": 7},
            "apex": [0.0, 0.0, 0.0],
            "axis": [0.0, 0.0, 1.0],
            "half_angle": FRAC_PI_4,
            "x_axis": [1.0, 0.0, 0.0]
        });
        assert!(
            serde_json::from_value::<Cone>(wrong_minor).is_err(),
            "major=1 minor=7 must be rejected under exact-match"
        );
    }

    /// Blocker 4: Serialization round-trip must be byte-identical for version=1.
    #[test]
    fn cone_serde_round_trip_byte_identical() {
        let c = std_cone();
        let json = serde_json::to_string(&c).unwrap();
        let c2: Cone = serde_json::from_str(&json).unwrap();
        assert_eq!(c, c2);
        let json2 = serde_json::to_string(&c2).unwrap();
        assert_eq!(json, json2, "re-serialization must be byte-identical");
    }

    fn skew_serialized_cone() -> Cone {
        serde_json::from_value(json!({
            "version": {"major": 1, "minor": 0},
            "apex": [0.0, 0.0, 0.0],
            "axis": [0.0, 0.0, 1.0],
            "half_angle": FRAC_PI_4,
            "x_axis": [0.099_833_416_646_828_15, 0.0, 0.995_004_165_278_025_8]
        }))
        .unwrap()
    }

    fn permissive_ctx() -> EvaluationContext {
        EvaluationContext::unlimited(ToleranceContext::try_new(10.0, 0.0, 1.0, 1.0).unwrap())
    }

    #[test]
    fn cone_skew_serialized_frame_evaluates_the_ideal_frame_with_bounds() {
        let tangent = FRAC_PI_4.tan();
        let evaluation = skew_serialized_cone()
            .evaluate(0.0, 2.0, DerivativeOrder::Second, &permissive_ctx())
            .unwrap();
        let norm = |actual: [f64; 3], ideal: [f64; 3]| {
            ((actual[0] - ideal[0]).powi(2)
                + (actual[1] - ideal[1]).powi(2)
                + (actual[2] - ideal[2]).powi(2))
            .sqrt()
        };
        assert!(
            norm(evaluation.position.into_array(), [2.0 * tangent, 0.0, 2.0])
                <= evaluation.position_error_bound.get()
        );
        assert!(
            norm(
                evaluation.du.unwrap().into_array(),
                [0.0, 2.0 * tangent, 0.0]
            ) <= evaluation.first_u_error_bound.unwrap().get()
        );
        assert!(
            norm(evaluation.dv.unwrap().into_array(), [tangent, 0.0, 1.0])
                <= evaluation.first_v_error_bound.unwrap().get()
        );
        assert!(
            norm(
                evaluation.duu.unwrap().into_array(),
                [-2.0 * tangent, 0.0, 0.0],
            ) <= evaluation.second_uu_error_bound.unwrap().get()
        );
        assert!(
            norm(evaluation.duv.unwrap().into_array(), [0.0, tangent, 0.0])
                <= evaluation.second_uv_error_bound.unwrap().get()
        );
    }

    #[test]
    fn cone_serialized_axis_seed_certifies_ideal_axial_contribution() {
        let cone: Cone = serde_json::from_value(json!({
            "version": {"major": 1, "minor": 0},
            "apex": [0.0, 0.0, 0.0],
            "axis": [0.6, 0.0, 0.8],
            "half_angle": FRAC_PI_4,
            "x_axis": [0.0, 1.0, 0.0]
        }))
        .unwrap();
        let evaluation = cone
            .evaluate(0.0, 2.0, DerivativeOrder::First, &permissive_ctx())
            .unwrap();
        let norm = (0.6_f64.powi(2) + 0.8_f64.powi(2)).sqrt();
        let tangent = FRAC_PI_4.tan();
        let ideal_dv = [0.6 / norm, tangent, 0.8 / norm];
        let dv = evaluation.dv.unwrap().into_array();
        let dv_error = evaluation.first_v_error_bound.unwrap().get();
        let difference = ((dv[0] - ideal_dv[0]).powi(2)
            + (dv[1] - ideal_dv[1]).powi(2)
            + (dv[2] - ideal_dv[2]).powi(2))
        .sqrt();
        assert!(difference <= dv_error);
        assert!(dv_error > 0.0, "z_ideal enclosure must not be zero-width");
    }

    #[test]
    fn cone_skew_serialized_projection_uses_ideal_cone() {
        let tangent = FRAC_PI_4.tan();
        // Both meridian-ray feet are positive for this query; the result must
        // therefore be selected by certified distance comparison, not sign(h).
        let expected_v = (2.0 * tangent + 1.0) / (1.0 + tangent * tangent);
        let expected = [expected_v * tangent, 0.0, expected_v];
        let query = Point3::try_new(2.0, 0.0, 1.0).unwrap();
        let projections = skew_serialized_cone()
            .project(query, &permissive_ctx())
            .unwrap();
        assert_eq!(projections.len(), 1);
        let projection = &projections[0];
        let point = projection.point.into_array();
        assert!((point[0] - expected[0]).abs() < 1e-12, "{point:?}");
        assert!(point[1].abs() < 1e-12, "{point:?}");
        assert!((point[2] - expected[2]).abs() < 1e-12, "{point:?}");
        assert!(
            ((point[0] - expected[0]).powi(2)
                + (point[1] - expected[1]).powi(2)
                + (point[2] - expected[2]).powi(2))
            .sqrt()
                <= projection.point_residual_bound.get()
        );
        assert!(projection.v.get() > 0.0);
    }

    #[test]
    fn cone_exact_equatorial_tie_orders_positive_then_negative_after_comparison() {
        let projections = std_cone()
            .project(Point3::try_new(2.0, 0.0, 0.0).unwrap(), &permissive_ctx())
            .unwrap();
        assert_eq!(projections.len(), 2);
        assert!(projections[0].v.get() > 0.0);
        assert!(projections[1].v.get() < 0.0);
        assert!(
            (projections[0].distance_bound.get() - projections[1].distance_bound.get()).abs()
                < 1e-12
        );
    }

    #[test]
    fn cone_negative_nappe_flips_angular_coordinates_before_atan2() {
        let projections = std_cone()
            .project(Point3::try_new(2.0, 0.0, -1.0).unwrap(), &permissive_ctx())
            .unwrap();
        assert_eq!(projections.len(), 1);
        let projection = &projections[0];
        assert!(projection.v.get() < 0.0);
        assert!((projection.u.get() - std::f64::consts::PI).abs() < 1e-12);
        let point = projection.point.into_array();
        assert!((point[0] - 1.5).abs() < 1e-12);
        assert!(point[1].abs() < 1e-12);
        assert!((point[2] + 1.5).abs() < 1e-12);
    }

    #[test]
    fn cone_apex_candidate_is_canonical_and_unique() {
        let projections = std_cone()
            .project(Point3::try_new(0.0, 0.0, 0.0).unwrap(), &permissive_ctx())
            .unwrap();
        assert_eq!(projections.len(), 1);
        assert_eq!(projections[0].u.get(), 0.0);
        assert_eq!(projections[0].v.get(), 0.0);
        assert_eq!(projections[0].point.into_array(), [0.0, 0.0, 0.0]);
        assert_eq!(projections[0].distance_bound.get(), 0.0);
    }
}
