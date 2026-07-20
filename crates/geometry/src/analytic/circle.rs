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
//! where `r` is the radius and the frame directions are the mathematical
//! ideal frame derived from the frozen stored seed bits: in 2-D,
//! `x = normalize(x_seed), y = perp(x)`; in 3-D, `z = normalize(normal_seed)`,
//! `x = normalize(x_seed - normal_seed*(x_seed·normal_seed)/||normal_seed||²)`,
//! and `y = z × x`.
//!
//! The evaluator domain is `[0, 2π]` with period `2π`; the upper endpoint is
//! accepted as a seam alias and evaluated canonically at `0`. Projection
//! parameters remain canonical in `[0, 2π)`.
//!
//! Derivatives:
//! - `p′(θ) = r·(−sin θ·x_axis + cos θ·y_axis)`
//! - `p″(θ) = −r·(cos θ·x_axis + sin θ·y_axis) = −(p(θ) − center)`
//!
//! Projection: the query point is projected onto the circle's plane (3-D) or
//! interpreted directly (2-D), then its ideal-frame angle is recovered with
//! certified exact-rational `atan2` arguments and mapped to `[0, 2π)`.
//! Returns [`GeometryError::Singular`] when the in-plane displacement from the
//! center is exactly zero.

#![allow(
    clippy::many_single_char_names,
    clippy::missing_panics_doc,
    clippy::similar_names
)]

use amphion_foundation::{
    Point2, Point3, SchemaVersion, Transform3, UnitVector2, UnitVector3, Vector2, Vector3,
};
use serde::{Deserialize, Serialize};

use crate::traits::{Curve2Evaluator, Curve3Evaluator};
use crate::{
    AngularParameterBound, CurveEvaluation2, CurveEvaluation3, CurveKind, CurveProjection2,
    CurveProjection3, DerivativeOrder, DistanceBound, EvaluationContext, FirstDerivativeBound,
    GeometryError, ParameterErrorBound, ParameterRange, ParameterValue, PositionBound,
    SecondDerivativeBound,
};

use super::{
    ConstructionError, TransformError,
    helpers::{
        ILL_COND_THRESH, all_finite2, all_finite3, canonicalize_periodic_endpoint,
        check_angular_tolerance, check_derivative_limit, check_parametric_tolerance,
        check_tolerance, dot3, exact_circle_eval2, exact_circle_eval3, exact_circle_project2,
        exact_circle_project3, in_range, mag3, normalization_to_construction, normalized_cross3,
        perp2, scale3, sub3,
    },
    transform::{
        exact_scaled_length, exact_transform_point, exact_transform_vector, is_identity_transform,
        similarity_scale,
    },
};

fn circle_domain() -> ParameterRange {
    ParameterRange::full_angular_period()
}

// ─── Circle2 ─────────────────────────────────────────────────────────────────

/// Current serialized schema version for this module's primitives.
const SCHEMA_VERSION: SchemaVersion = SchemaVersion::new(1, 0);

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Circle2Repr {
    version: SchemaVersion,
    center: Point2,
    radius: f64,
    x_axis: UnitVector2,
}

/// A circular arc in two-dimensional parameter space.
///
/// Parameterization: `p(θ) = center + r·cos θ·x + r·sin θ·y`\
/// where `y = perp(x_axis)` (90° CCW), `r = radius`, domain `[0, 2π]`
/// with `2π` an alias of `0`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Circle2Repr", into = "Circle2Repr")]
pub struct Circle2 {
    center: Point2,
    radius: f64,
    x_axis: UnitVector2,
    y_axis: UnitVector2,
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
        if !all_finite2(c) || !radius.is_finite() {
            return Err(ConstructionError::NonFiniteInput);
        }
        if radius <= 0.0 {
            return Err(ConstructionError::NotPositive);
        }
        let x_unit = UnitVector2::try_normalize(x_axis).map_err(normalization_to_construction)?;
        let y_arr = perp2(x_unit.into_array());
        let y_unit = UnitVector2::try_normalize(
            Vector2::try_new(y_arr[0], y_arr[1]).map_err(|_| ConstructionError::NonFiniteInput)?,
        )
        .map_err(normalization_to_construction)?;
        Ok(Self {
            center: Point2::try_new(c[0], c[1]).map_err(|_| ConstructionError::NonFiniteInput)?,
            radius,
            x_axis: x_unit,
            y_axis: y_unit,
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

    /// Returns the frozen finite X seed used to derive the ideal `θ = 0`
    /// direction.
    #[must_use]
    pub fn x_axis(&self) -> Vector2 {
        self.x_axis.as_vector()
    }

    /// Returns the stored finite Y approximation. Evaluation derives the ideal
    /// Y direction from the frozen X seed.
    #[must_use]
    pub fn y_axis(&self) -> Vector2 {
        self.y_axis.as_vector()
    }
}

impl TryFrom<Circle2Repr> for Circle2 {
    type Error = ConstructionError;
    fn try_from(repr: Circle2Repr) -> Result<Self, Self::Error> {
        if repr.version != SCHEMA_VERSION {
            return Err(ConstructionError::UnsupportedSchemaVersion {
                found: repr.version,
                supported: SCHEMA_VERSION,
            });
        }
        let center = repr.center.into_array();
        if !all_finite2(center) || !repr.radius.is_finite() {
            return Err(ConstructionError::NonFiniteInput);
        }
        if repr.radius <= 0.0 {
            return Err(ConstructionError::NotPositive);
        }
        let x_unit = repr.x_axis;
        let y_arr = perp2(x_unit.into_array());
        let y_unit = UnitVector2::try_normalize(
            Vector2::try_new(y_arr[0], y_arr[1]).map_err(|_| ConstructionError::NonFiniteInput)?,
        )
        .map_err(normalization_to_construction)?;
        Ok(Self {
            center: repr.center,
            radius: repr.radius,
            x_axis: x_unit,
            y_axis: y_unit,
        })
    }
}

impl From<Circle2> for Circle2Repr {
    fn from(c: Circle2) -> Self {
        Self {
            version: SCHEMA_VERSION,
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
        context: &EvaluationContext,
    ) -> Result<CurveEvaluation2, GeometryError> {
        if !parameter.is_finite() {
            return Err(GeometryError::NonFiniteParameter);
        }
        let domain = self.domain();
        if !in_range(parameter, domain) {
            return Err(GeometryError::OutsideDomain);
        }
        let parameter = canonicalize_periodic_endpoint(parameter, domain);
        let c = self.center.into_array();
        let x_ax = self.x_axis.into_array();

        let eval = exact_circle_eval2(context.budget(), c, self.radius, x_ax, parameter)?;
        let pos = Point2::try_new(eval.point[0], eval.point[1]).map_err(|_| {
            GeometryError::Uncertified {
                reason: "circle position is non-finite".to_owned(),
            }
        })?;
        let position_error_bound =
            PositionBound::try_new(eval.position_error_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "position error bound overflowed representable range".to_owned(),
                }
            })?;
        check_parametric_tolerance(&context.tolerance(), position_error_bound.get())?;

        let first_error_bound =
            FirstDerivativeBound::try_new(eval.first_error_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "first derivative error bound overflowed representable range"
                        .to_owned(),
                }
            })?;
        let second_error_bound =
            SecondDerivativeBound::try_new(eval.second_error_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "second derivative error bound overflowed representable range"
                        .to_owned(),
                }
            })?;

        let (first, first_eb, second, second_eb) = match order {
            DerivativeOrder::Position => (None, None, None, None),
            DerivativeOrder::First => {
                check_derivative_limit(
                    first_error_bound.get(),
                    context.derivative_limits().curve().first().get(),
                )?;
                let v = Vector2::try_new(eval.first[0], eval.first[1]).map_err(|_| {
                    GeometryError::Uncertified {
                        reason: "circle first derivative is non-finite".to_owned(),
                    }
                })?;
                (Some(v), Some(first_error_bound), None, None)
            }
            DerivativeOrder::Second => {
                check_derivative_limit(
                    first_error_bound.get(),
                    context.derivative_limits().curve().first().get(),
                )?;
                check_derivative_limit(
                    second_error_bound.get(),
                    context.derivative_limits().curve().second().get(),
                )?;
                let v = Vector2::try_new(eval.first[0], eval.first[1]).map_err(|_| {
                    GeometryError::Uncertified {
                        reason: "circle first derivative is non-finite".to_owned(),
                    }
                })?;
                let v2 = Vector2::try_new(eval.second[0], eval.second[1]).map_err(|_| {
                    GeometryError::Uncertified {
                        reason: "circle second derivative is non-finite".to_owned(),
                    }
                })?;
                (
                    Some(v),
                    Some(first_error_bound),
                    Some(v2),
                    Some(second_error_bound),
                )
            }
        };
        Ok(CurveEvaluation2 {
            position: pos,
            first,
            second,
            position_error_bound,
            first_error_bound: first_eb,
            second_error_bound: second_eb,
        })
    }

    fn project_into(
        &self,
        point: Point2,
        context: &EvaluationContext,
        output: &mut Vec<CurveProjection2>,
    ) -> Result<(), GeometryError> {
        output.clear();
        let q = point.into_array();
        let c = self.center.into_array();
        let x_ax = self.x_axis.into_array();

        let result = exact_circle_project2(context.budget(), q, c, self.radius, x_ax)?;
        check_parametric_tolerance(&context.tolerance(), result.point_residual_bound)?;
        check_angular_tolerance(&context.tolerance(), result.parameter_error_bound)?;

        let proj = Point2::try_new(result.point[0], result.point[1]).map_err(|_| {
            GeometryError::Uncertified {
                reason: "circle projection point is non-finite".to_owned(),
            }
        })?;
        let ang_bound =
            AngularParameterBound::try_new(result.parameter_error_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "circle2 angular parameter bound is invalid".to_owned(),
                }
            })?;
        output.push(CurveProjection2 {
            parameter: ParameterValue::try_new(result.parameter).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "circle projection parameter is non-finite".to_owned(),
                }
            })?,
            point: proj,
            distance_bound: DistanceBound::try_new(result.distance_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "circle projection distance is non-finite or negative".to_owned(),
                }
            })?,
            parameter_error_bound: ParameterErrorBound::Angular(ang_bound),
            point_residual_bound: PositionBound::try_new(result.point_residual_bound).map_err(
                |_| GeometryError::Uncertified {
                    reason: "circle projection point residual bound is non-finite or negative"
                        .to_owned(),
                },
            )?,
        });
        Ok(())
    }
}

// ─── Circle3 ─────────────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Circle3Repr {
    version: SchemaVersion,
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
/// where `y_axis = normal × x_axis`, `r = radius`, domain `[0, 2π]`
/// with `2π` an alias of `0`.
///
/// [`Circle3::try_new`] Gram-Schmidt-orthogonalizes the supplied vectors. Both
/// constructor and deserialization paths retain finite seed bits from which
/// evaluation and projection derive the mathematical ideal right-handed
/// orthonormal frame; deserialization keeps its stored `normal`/`x_axis`
/// seeds **as-is**.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Circle3Repr", into = "Circle3Repr")]
pub struct Circle3 {
    center: Point3,
    radius: f64,
    normal: Vector3,
    x_axis: Vector3,
    y_axis: UnitVector3,
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
    /// - [`ConstructionError::IllConditionedAxes`] — `x_axis` nearly parallel
    ///   to `normal`
    pub fn try_new(
        center: Point3,
        radius: f64,
        normal: Vector3,
        x_axis: Vector3,
    ) -> Result<Self, ConstructionError> {
        let c = center.into_array();
        if !all_finite3(c) || !radius.is_finite() {
            return Err(ConstructionError::NonFiniteInput);
        }
        if radius <= 0.0 {
            return Err(ConstructionError::NotPositive);
        }
        let n_unit = UnitVector3::try_normalize(normal).map_err(normalization_to_construction)?;
        let x_norm = UnitVector3::try_normalize(x_axis).map_err(normalization_to_construction)?;
        // Orthogonalize x against n.
        let dot_xn = dot3(x_norm.into_array(), n_unit.into_array());
        let x_perp = sub3(x_norm.into_array(), scale3(n_unit.into_array(), dot_xn));
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
        let y_unit = UnitVector3::try_normalize(n_unit.cross(x_unit))
            .map_err(|_| ConstructionError::DependentAxes)?;
        Ok(Self {
            center: Point3::try_new(c[0], c[1], c[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            radius,
            normal: n_unit.as_vector(),
            x_axis: x_unit.as_vector(),
            y_axis: y_unit,
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

    /// Returns the frozen finite normal seed used to derive the ideal plane
    /// normal.
    #[must_use]
    pub fn normal(&self) -> Vector3 {
        self.normal
    }

    /// Returns the frozen finite X seed used to derive the ideal `θ = 0`
    /// direction.
    #[must_use]
    pub fn x_axis(&self) -> Vector3 {
        self.x_axis
    }

    /// Returns the stored finite Y approximation. Evaluation derives the ideal
    /// Y direction from the frozen normal and X seeds.
    #[must_use]
    pub fn y_axis(&self) -> Vector3 {
        self.y_axis.as_vector()
    }

    /// Applies a similarity `transform` (rigid motion plus uniform scale, no
    /// reflection) to this circle, returning a new circle whose radius is
    /// scaled accordingly.
    ///
    /// A general affine transform does not map a circle to a circle, so
    /// only transforms whose finite matrix entries satisfy the similarity
    /// identities exactly are accepted.
    ///
    /// # Errors
    ///
    /// - [`TransformError::NotSimilarity`] — the transform's linear part is
    ///   not an exact uniform-scale rotation over its stored matrix entries
    /// - [`TransformError::UnrepresentableScale`] — the exact uniform scale or
    ///   scaled circle radius cannot be represented as `f64`
    /// - [`TransformError::UnrepresentableResult`] — an exact transformed
    ///   center or seed component is not representable as `f64`
    /// - [`TransformError::DegenerateResult`] — the transformed frozen seeds
    ///   become zero or dependent
    pub fn try_transform(&self, transform: &Transform3) -> Result<Self, TransformError> {
        if is_identity_transform(transform) {
            return Ok(self.clone());
        }
        let scale = similarity_scale(transform)?;
        let new_center = exact_transform_point(transform, self.center)?;
        let new_normal_vec = exact_transform_vector(transform, self.normal)?;
        let new_x_axis_vec = exact_transform_vector(transform, self.x_axis)?;
        let new_radius = exact_scaled_length(self.radius, scale)?;
        Self::try_from(Circle3Repr {
            version: SCHEMA_VERSION,
            center: new_center,
            radius: new_radius,
            normal: new_normal_vec,
            x_axis: new_x_axis_vec,
        })
        .map_err(|_| TransformError::DegenerateResult)
    }
}

impl TryFrom<Circle3Repr> for Circle3 {
    type Error = ConstructionError;
    fn try_from(repr: Circle3Repr) -> Result<Self, Self::Error> {
        if repr.version != SCHEMA_VERSION {
            return Err(ConstructionError::UnsupportedSchemaVersion {
                found: repr.version,
                supported: SCHEMA_VERSION,
            });
        }
        let center = repr.center.into_array();
        if !all_finite3(center) || !repr.radius.is_finite() {
            return Err(ConstructionError::NonFiniteInput);
        }
        if repr.radius <= 0.0 {
            return Err(ConstructionError::NotPositive);
        }
        let normal = repr.normal;
        let x_axis = repr.x_axis;
        let y_unit = normalized_cross3(normal, x_axis)?;
        Ok(Self {
            center: repr.center,
            radius: repr.radius,
            normal,
            x_axis,
            y_axis: y_unit,
        })
    }
}

impl From<Circle3> for Circle3Repr {
    fn from(c: Circle3) -> Self {
        Self {
            version: SCHEMA_VERSION,
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
        context: &EvaluationContext,
    ) -> Result<CurveEvaluation3, GeometryError> {
        if !parameter.is_finite() {
            return Err(GeometryError::NonFiniteParameter);
        }
        let domain = self.domain();
        if !in_range(parameter, domain) {
            return Err(GeometryError::OutsideDomain);
        }
        let parameter = canonicalize_periodic_endpoint(parameter, domain);
        let c = self.center.into_array();
        let n_ax = self.normal.into_array();
        let x_ax = self.x_axis.into_array();

        let eval = exact_circle_eval3(context.budget(), c, self.radius, n_ax, x_ax, parameter)?;
        let pos = Point3::try_new(eval.point[0], eval.point[1], eval.point[2]).map_err(|_| {
            GeometryError::Uncertified {
                reason: "circle position is non-finite".to_owned(),
            }
        })?;
        let position_error_bound =
            PositionBound::try_new(eval.position_error_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "position error bound overflowed representable range".to_owned(),
                }
            })?;
        let eval_scale = mag3(c) + self.radius.abs();
        check_tolerance(&context.tolerance(), position_error_bound.get(), eval_scale)?;

        let first_error_bound =
            FirstDerivativeBound::try_new(eval.first_error_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "first derivative error bound overflowed representable range"
                        .to_owned(),
                }
            })?;
        let second_error_bound =
            SecondDerivativeBound::try_new(eval.second_error_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "second derivative error bound overflowed representable range"
                        .to_owned(),
                }
            })?;

        let (first, first_eb, second, second_eb) = match order {
            DerivativeOrder::Position => (None, None, None, None),
            DerivativeOrder::First => {
                check_derivative_limit(
                    first_error_bound.get(),
                    context.derivative_limits().curve().first().get(),
                )?;
                let v = Vector3::try_new(eval.first[0], eval.first[1], eval.first[2]).map_err(
                    |_| GeometryError::Uncertified {
                        reason: "circle first derivative is non-finite".to_owned(),
                    },
                )?;
                (Some(v), Some(first_error_bound), None, None)
            }
            DerivativeOrder::Second => {
                check_derivative_limit(
                    first_error_bound.get(),
                    context.derivative_limits().curve().first().get(),
                )?;
                check_derivative_limit(
                    second_error_bound.get(),
                    context.derivative_limits().curve().second().get(),
                )?;
                let v = Vector3::try_new(eval.first[0], eval.first[1], eval.first[2]).map_err(
                    |_| GeometryError::Uncertified {
                        reason: "circle first derivative is non-finite".to_owned(),
                    },
                )?;
                let v2 = Vector3::try_new(eval.second[0], eval.second[1], eval.second[2]).map_err(
                    |_| GeometryError::Uncertified {
                        reason: "circle second derivative is non-finite".to_owned(),
                    },
                )?;
                (
                    Some(v),
                    Some(first_error_bound),
                    Some(v2),
                    Some(second_error_bound),
                )
            }
        };
        Ok(CurveEvaluation3 {
            position: pos,
            first,
            second,
            position_error_bound,
            first_error_bound: first_eb,
            second_error_bound: second_eb,
        })
    }

    fn project_into(
        &self,
        point: Point3,
        context: &EvaluationContext,
        output: &mut Vec<CurveProjection3>,
    ) -> Result<(), GeometryError> {
        output.clear();
        let q = point.into_array();
        let c = self.center.into_array();
        let n_ax = self.normal.into_array();
        let x_ax = self.x_axis.into_array();

        let result = exact_circle_project3(context.budget(), q, c, self.radius, n_ax, x_ax)?;
        let scale = mag3(q) + mag3(result.point);
        check_tolerance(&context.tolerance(), result.point_residual_bound, scale)?;
        check_angular_tolerance(&context.tolerance(), result.parameter_error_bound)?;

        let proj =
            Point3::try_new(result.point[0], result.point[1], result.point[2]).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "circle projection point is non-finite".to_owned(),
                }
            })?;
        let ang_bound =
            AngularParameterBound::try_new(result.parameter_error_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "circle3 angular parameter bound is invalid".to_owned(),
                }
            })?;
        output.push(CurveProjection3 {
            parameter: ParameterValue::try_new(result.parameter).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "circle projection parameter is non-finite".to_owned(),
                }
            })?,
            point: proj,
            distance_bound: DistanceBound::try_new(result.distance_bound).map_err(|_| {
                GeometryError::Uncertified {
                    reason: "circle projection distance is non-finite or negative".to_owned(),
                }
            })?,
            parameter_error_bound: ParameterErrorBound::Angular(ang_bound),
            point_residual_bound: PositionBound::try_new(result.point_residual_bound).map_err(
                |_| GeometryError::Uncertified {
                    reason: "circle projection point residual bound is non-finite or negative"
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
    #![allow(clippy::float_cmp)]

    use std::f64::consts::{FRAC_PI_2, TAU};

    use amphion_foundation::{Point2, Point3, SchemaVersion, ToleranceContext, Vector2, Vector3};
    use serde_json::json;

    use crate::analytic::helpers::ILL_COND_THRESH;
    use crate::traits::{Curve2Evaluator, Curve3Evaluator};
    use crate::{
        CertificationBudget, CurveDerivativeLimits, CurveFirstDerivativeLimit,
        CurveSecondDerivativeLimit, DerivativeLimits, DerivativeOrder, EvaluationContext,
        GeometryError, SurfaceDerivativeLimits,
    };

    use super::{Circle2, Circle2Repr, Circle3, Circle3Repr, ConstructionError};

    fn tol() -> ToleranceContext {
        ToleranceContext::try_new(1e-9, 1e-8, 1e-10, 1e-12).unwrap()
    }

    fn ctx() -> EvaluationContext {
        EvaluationContext::unlimited(tol())
    }

    fn dist2(a: Point2, b: Point2) -> f64 {
        let [ax, ay] = a.into_array();
        let [bx, by] = b.into_array();
        (ax - bx).hypot(ay - by)
    }

    fn dist3(a: Point3, b: Point3) -> f64 {
        let [ax, ay, az] = a.into_array();
        let [bx, by, bz] = b.into_array();
        ((ax - bx).powi(2) + (ay - by).powi(2) + (az - bz).powi(2)).sqrt()
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
    fn circle2_evaluate_matches_known_values() {
        // center=(1,2), radius=3, x_axis=(1,0) ⇒ y_axis=(0,1) (perp2 rotates
        // +90°). At θ=0: p=(4,2). At θ=π/2: p=(1,5), p′=(-3,0), p″=(0,-3).
        let c = Circle2::try_new(
            Point2::try_new(1.0, 2.0).unwrap(),
            3.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        let eval = c.evaluate(0.0, DerivativeOrder::Position, &ctx()).unwrap();
        let [px, py] = eval.position.into_array();
        assert!((px - 4.0).abs() < 1e-9, "px={px}");
        assert!((py - 2.0).abs() < 1e-9, "py={py}");
        assert!(eval.position_error_bound.get() < 1e-9);

        let eval = c
            .evaluate(FRAC_PI_2, DerivativeOrder::Second, &ctx())
            .unwrap();
        let [px, py] = eval.position.into_array();
        assert!((px - 1.0).abs() < 1e-9, "px={px}");
        assert!((py - 5.0).abs() < 1e-9, "py={py}");
        let [dx, dy] = eval.first.unwrap().into_array();
        assert!((dx - (-3.0)).abs() < 1e-9, "dx={dx}");
        assert!(dy.abs() < 1e-9, "dy={dy}");
        let [ddx, ddy] = eval.second.unwrap().into_array();
        assert!(ddx.abs() < 1e-9, "ddx={ddx}");
        assert!((ddy - (-3.0)).abs() < 1e-9, "ddy={ddy}");
    }

    /// Curve2 position and projection-point certificates use UV tolerance even
    /// when the model-space length tolerance is deliberately permissive.
    #[test]
    fn circle2_position_certificates_use_parametric_tolerance() {
        let center_x = 2.0_f64.powi(53);
        let circle = Circle2::try_new(
            Point2::try_new(center_x, 0.0).unwrap(),
            1.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        let context = EvaluationContext::unlimited(
            ToleranceContext::try_new(1.0e6, 0.0, 1e-10, 0.5).unwrap(),
        );

        let evaluation = circle.evaluate(0.0, DerivativeOrder::Position, &context);
        assert!(
            matches!(evaluation, Err(GeometryError::Uncertified { .. })),
            "unrepresentable UV point must exceed parametric tolerance: {evaluation:?}"
        );

        let query = Point2::try_new(center_x + 2.0, 0.0).unwrap();
        let projection = circle.project(query, &context);
        assert!(
            matches!(projection, Err(GeometryError::Uncertified { .. })),
            "projection UV residual must exceed parametric tolerance: {projection:?}"
        );
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
            c.evaluate(-0.001, DerivativeOrder::Position, &ctx()),
            Err(GeometryError::OutsideDomain)
        );
        assert_eq!(
            c.evaluate(TAU.next_up(), DerivativeOrder::Position, &ctx()),
            Err(GeometryError::OutsideDomain)
        );
    }

    #[test]
    fn circle2_evaluate_canonicalizes_periodic_endpoint() {
        let circle = Circle2::try_new(
            Point2::try_new(1.0, 2.0).unwrap(),
            3.0,
            Vector2::try_new(1.0, 1.0).unwrap(),
        )
        .unwrap();
        assert_eq!(
            circle.evaluate(0.0, DerivativeOrder::Second, &ctx()),
            circle.evaluate(TAU, DerivativeOrder::Second, &ctx())
        );
    }

    #[test]
    fn circle2_evaluate_rejects_non_finite_before_uncertified() {
        let c = Circle2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            1.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        assert_eq!(
            c.evaluate(f64::NAN, DerivativeOrder::Position, &ctx()),
            Err(GeometryError::NonFiniteParameter)
        );
    }

    #[test]
    fn circle2_project_matches_known_values() {
        // center=(2,3), radius=4, x_axis=(1,0). q=(10,3) lies on the +x_axis
        // ray from the center, so the nearest point is center+radius·x_axis
        // = (6,3), distance = |10-2|-4 = 4, parameter = atan2(0, 8) = 0.
        let c = Circle2::try_new(
            Point2::try_new(2.0, 3.0).unwrap(),
            4.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        let q = Point2::try_new(10.0, 3.0).unwrap();
        let projs = c.project(q, &ctx()).unwrap();
        assert_eq!(projs.len(), 1);
        let p = &projs[0];
        let [px, py] = p.point.into_array();
        assert!((px - 6.0).abs() < 1e-9, "px={px}");
        assert!((py - 3.0).abs() < 1e-9, "py={py}");
        assert!((p.distance_bound.get() - 4.0).abs() < 1e-9);
        assert!(p.parameter.get().abs() < 1e-9);
        let actual = dist2(q, p.point);
        assert!(actual <= p.distance_bound.get());
    }

    #[test]
    fn circle2_project_into_clears_output_on_error() {
        // Querying exactly at the center is singular: the in-plane offset is
        // zero, so there is no unique nearest point / well-defined angle.
        let c = Circle2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            1.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        let mut output = vec![];
        let err = c.project_into(Point2::try_new(0.0, 0.0).unwrap(), &ctx(), &mut output);
        assert_eq!(err.unwrap_err(), GeometryError::Singular);
        assert!(output.is_empty());
    }

    #[test]
    fn circle2_serde_round_trip() {
        let c = Circle2::try_new(
            Point2::try_new(1.0, -2.0).unwrap(),
            3.5,
            Vector2::try_new(1.0, 1.0).unwrap(),
        )
        .unwrap();
        let json = serde_json::to_string(&c).unwrap();
        let decoded: Circle2 = serde_json::from_str(&json).unwrap();
        assert_eq!(c, decoded);
    }

    #[test]
    fn circle2_serde_bitexact_roundtrip() {
        let c = Circle2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            1.0,
            Vector2::try_new(0.6, 0.8).unwrap(),
        )
        .unwrap();
        let json = serde_json::to_string(&c).unwrap();
        let decoded: Circle2 = serde_json::from_str(&json).unwrap();
        assert_eq!(c.x_axis().into_array(), decoded.x_axis().into_array());
    }

    #[test]
    fn circle2_distance_bound_large_radius_is_valid_upper_bound_or_uncertified() {
        let tol_1m = ToleranceContext::try_new(1.0, 0.0, 1e-10, 1e-12).unwrap();
        let radius = f64::powi(2.0, 53);
        let c = Circle2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            radius,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        let q = Point2::try_new(1.0, 1.0).unwrap();
        let result = c.project(q, &EvaluationContext::unlimited(tol_1m));
        match result {
            Err(GeometryError::Uncertified { .. }) => {}
            Ok(projs) => {
                for p in &projs {
                    let actual = dist2(q, p.point);
                    assert!(actual <= p.distance_bound.get());
                }
            }
            Err(other) => panic!("unexpected error: {other:?}"),
        }
    }

    #[test]
    fn circle2_serde_rejects_non_unit_axis() {
        assert!(
            serde_json::from_str::<Circle2>(
                r#"{"version":{"major":1,"minor":0},"center":[0.0,0.0],"radius":1.0,"x_axis":[2.0,0.0]}"#
            )
            .is_err()
        );

        let bad_radius: Circle2Repr = serde_json::from_value(json!({
            "version": {"major": 1, "minor": 0},
            "center": [1.0, 2.0],
            "radius": 0.0,
            "x_axis": [1.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Circle2::try_from(bad_radius),
            Err(ConstructionError::NotPositive)
        );
    }

    #[test]
    fn circle2_serde_rejects_nan_and_inf_fields() {
        assert!(
            serde_json::from_str::<Circle2>(
                r#"{"version":{"major":1,"minor":0},"center":[NaN,0.0],"radius":1.0,"x_axis":[1.0,0.0]}"#
            )
            .is_err()
        );
        assert!(
            serde_json::from_str::<Circle2>(
                r#"{"version":{"major":1,"minor":0},"center":[0.0,0.0],"radius":1.0,"x_axis":[Infinity,0.0]}"#
            )
            .is_err()
        );
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
    fn circle3_construction_rejects_ill_conditioned_axes() {
        let err = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            1.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(ILL_COND_THRESH / 2.0, 0.0, 1.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::IllConditionedAxes);
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
    fn circle3_evaluate_matches_known_values() {
        // center=(0,0,1), radius=2, normal=(0,0,1), x_axis=(1,0,0) ⇒
        // y_axis = normal × x_axis = (0,1,0). At θ=0: p=(2,0,1). At θ=π/2:
        // p=(0,2,1), p′=(-2,0,0), p″=(0,-2,0).
        let c = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 1.0).unwrap(),
            2.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let eval = c.evaluate(0.0, DerivativeOrder::Position, &ctx()).unwrap();
        let [px, py, pz] = eval.position.into_array();
        assert!((px - 2.0).abs() < 1e-9, "px={px}");
        assert!(py.abs() < 1e-9, "py={py}");
        assert!((pz - 1.0).abs() < 1e-9, "pz={pz}");

        let eval = c
            .evaluate(FRAC_PI_2, DerivativeOrder::Second, &ctx())
            .unwrap();
        let [px, py, pz] = eval.position.into_array();
        assert!(px.abs() < 1e-9, "px={px}");
        assert!((py - 2.0).abs() < 1e-9, "py={py}");
        assert!((pz - 1.0).abs() < 1e-9, "pz={pz}");
        let [dx, dy, dz] = eval.first.unwrap().into_array();
        assert!((dx - (-2.0)).abs() < 1e-9, "dx={dx}");
        assert!(dy.abs() < 1e-9, "dy={dy}");
        assert!(dz.abs() < 1e-9, "dz={dz}");
        let [ddx, ddy, ddz] = eval.second.unwrap().into_array();
        assert!(ddx.abs() < 1e-9, "ddx={ddx}");
        assert!((ddy - (-2.0)).abs() < 1e-9, "ddy={ddy}");
        assert!(ddz.abs() < 1e-9, "ddz={ddz}");
    }

    #[test]
    fn circle3_evaluate_rejects_out_of_domain() {
        let c = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            1.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        assert_eq!(
            c.evaluate(-0.001, DerivativeOrder::Position, &ctx()),
            Err(GeometryError::OutsideDomain)
        );
        assert_eq!(
            c.evaluate(TAU.next_up(), DerivativeOrder::Position, &ctx()),
            Err(GeometryError::OutsideDomain)
        );
    }

    #[test]
    fn circle3_evaluate_canonicalizes_periodic_endpoint() {
        let circle = Circle3::try_new(
            Point3::try_new(1.0, 2.0, 3.0).unwrap(),
            4.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 1.0, 0.0).unwrap(),
        )
        .unwrap();
        assert_eq!(
            circle.evaluate(0.0, DerivativeOrder::Second, &ctx()),
            circle.evaluate(TAU, DerivativeOrder::Second, &ctx())
        );
    }

    #[test]
    fn circle3_project_matches_known_values() {
        // center=(0,0,0), radius=5, normal=(0,0,1), x_axis=(1,0,0),
        // y_axis=(0,1,0). q=(1,0,5): in-plane offset (1,0), out-of-plane 5.
        // Nearest point = (5,0,0), distance = sqrt((1-5)^2 + 5^2) = sqrt(41).
        let c = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            5.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let q = Point3::try_new(1.0, 0.0, 5.0).unwrap();
        let projs = c.project(q, &ctx()).unwrap();
        assert_eq!(projs.len(), 1);
        let p = &projs[0];
        let [px, py, pz] = p.point.into_array();
        assert!((px - 5.0).abs() < 1e-9, "px={px}");
        assert!(py.abs() < 1e-9, "py={py}");
        assert!(pz.abs() < 1e-9, "pz={pz}");
        let expected_dist = 41.0_f64.sqrt();
        assert!((p.distance_bound.get() - expected_dist).abs() < 1e-9);
        assert!(p.parameter.get().abs() < 1e-9);
        let actual = dist3(q, p.point);
        assert!(actual <= p.distance_bound.get());
    }

    #[test]
    fn circle3_project_into_clears_output_on_error() {
        // Querying exactly at the center is singular: the in-plane offset is
        // zero, so there is no unique nearest point / well-defined angle.
        let c = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            1.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let mut output = vec![];
        let err = c.project_into(Point3::try_new(0.0, 0.0, 0.0).unwrap(), &ctx(), &mut output);
        assert_eq!(err.unwrap_err(), GeometryError::Singular);
        assert!(output.is_empty());
    }

    #[test]
    fn circle3_serde_round_trip() {
        let c = Circle3::try_new(
            Point3::try_new(1.0, 2.0, 3.0).unwrap(),
            4.0,
            Vector3::try_new(
                1.0 / 3.0_f64.sqrt(),
                1.0 / 3.0_f64.sqrt(),
                1.0 / 3.0_f64.sqrt(),
            )
            .unwrap(),
            Vector3::try_new(1.0 / 2.0_f64.sqrt(), -1.0 / 2.0_f64.sqrt(), 0.0).unwrap(),
        )
        .unwrap();
        let json = serde_json::to_string(&c).unwrap();
        let decoded: Circle3 = serde_json::from_str(&json).unwrap();
        assert_eq!(c, decoded);
        assert_eq!(c.normal().into_array(), decoded.normal().into_array());
        assert_eq!(c.x_axis().into_array(), decoded.x_axis().into_array());
    }

    #[test]
    fn circle3_serde_preserves_scaled_seeds_and_rejects_bad_radius() {
        let scaled: Circle3 = serde_json::from_str(
            r#"{"version":{"major":1,"minor":0},"center":[1.0,2.0,3.0],"radius":4.0,"normal":[2.0,0.0,0.0],"x_axis":[0.0,1.0,0.0]}"#,
        )
        .unwrap();
        assert_eq!(scaled.normal().into_array(), [2.0, 0.0, 0.0]);

        let bad_frame: Circle3Repr = serde_json::from_value(json!({
            "version": {"major": 1, "minor": 0},
            "center": [1.0, 2.0, 3.0],
            "radius": 4.0,
            "normal": [1.0, 0.0, 0.0],
            "x_axis": [1.0, 0.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Circle3::try_from(bad_frame),
            Err(ConstructionError::DependentAxes)
        );

        let bad_radius: Circle3Repr = serde_json::from_value(json!({
            "version": {"major": 1, "minor": 0},
            "center": [1.0, 2.0, 3.0],
            "radius": 0.0,
            "normal": [0.0, 0.0, 1.0],
            "x_axis": [1.0, 0.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Circle3::try_from(bad_radius),
            Err(ConstructionError::NotPositive)
        );
    }

    #[test]
    fn circle3_serde_rejects_nan_and_inf_fields() {
        assert!(serde_json::from_str::<Circle3>(
            r#"{"version":{"major":1,"minor":0},"center":[NaN,0.0,0.0],"radius":1.0,"normal":[0.0,0.0,1.0],"x_axis":[1.0,0.0,0.0]}"#
        )
        .is_err());
        assert!(serde_json::from_str::<Circle3>(
            r#"{"version":{"major":1,"minor":0},"center":[0.0,0.0,0.0],"radius":1.0,"normal":[Infinity,0.0,1.0],"x_axis":[1.0,0.0,0.0]}"#
        )
        .is_err());
    }

    #[test]
    fn circle3_try_transform_identity_is_noop() {
        let c = Circle3::try_new(
            Point3::try_new(1.0, 2.0, 3.0).unwrap(),
            2.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let out = c
            .try_transform(&amphion_foundation::Transform3::IDENTITY)
            .unwrap();
        assert_eq!(out, c);
    }

    #[test]
    fn circle3_identity_transform_preserves_deserialized_frozen_seeds() {
        let circle: Circle3 = serde_json::from_value(json!({
            "version": {"major": 1, "minor": 0},
            "center": [0.0, 0.0, 0.0],
            "radius": 2.0,
            "normal": [0.0, 0.0, 1.0],
            "x_axis": [0.099_833_416_646_828_15, 0.0, 0.995_004_165_278_025_8]
        }))
        .unwrap();
        assert_eq!(
            circle
                .try_transform(&amphion_foundation::Transform3::IDENTITY)
                .unwrap(),
            circle
        );
    }

    #[test]
    fn circle3_try_transform_similarity_scales_radius() {
        // Rotation by 90° about Z, uniform scale 2, plus translation.
        let m = [
            0.0, -2.0, 0.0, 5.0, //
            2.0, 0.0, 0.0, -3.0, //
            0.0, 0.0, 2.0, 7.0,
        ];
        let t = amphion_foundation::Transform3::try_from_row_major(m).unwrap();
        let c = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            3.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let out = c.try_transform(&t).unwrap();
        assert!((out.radius() - 6.0).abs() < 1e-9);
        let [cx, cy, cz] = out.center().into_array();
        assert!((cx - 5.0).abs() < 1e-9);
        assert!((cy - (-3.0)).abs() < 1e-9);
        assert!((cz - 7.0).abs() < 1e-9);
        assert_eq!(out.normal().into_array(), [0.0, 0.0, 2.0]);
        assert_eq!(out.x_axis().into_array(), [0.0, 2.0, 0.0]);
    }

    #[test]
    fn circle3_try_transform_rejects_non_similarity() {
        // Non-uniform scale (shear-free but anisotropic) is not a
        // similarity: it distorts a circle into an ellipse.
        let m = [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 2.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0,
        ];
        let t = amphion_foundation::Transform3::try_from_row_major(m).unwrap();
        let c = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            1.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        assert_eq!(
            c.try_transform(&t),
            Err(super::TransformError::NotSimilarity)
        );
    }

    #[test]
    fn circle3_try_transform_rejects_unrepresentable_exact_scale() {
        let q = 729_000_054_000_001.0;
        let transform = amphion_foundation::Transform3::try_from_row_major([
            -3.0 * q,
            4.0 * q,
            12.0 * q,
            0.0,
            12.0 * q,
            -3.0 * q,
            4.0 * q,
            0.0,
            4.0 * q,
            12.0 * q,
            -3.0 * q,
            0.0,
        ])
        .unwrap();
        let circle = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            1.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        assert_eq!(
            circle.try_transform(&transform),
            Err(super::TransformError::UnrepresentableScale)
        );
    }

    #[test]
    fn circle3_try_transform_rejects_unrepresentable_scaled_radius() {
        let scale = 0.1;
        let transform = amphion_foundation::Transform3::try_from_row_major([
            scale, 0.0, 0.0, 0.0, //
            0.0, scale, 0.0, 0.0, //
            0.0, 0.0, scale, 0.0,
        ])
        .unwrap();
        let circle = Circle3::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            scale,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        assert_eq!(
            circle.try_transform(&transform),
            Err(super::TransformError::UnrepresentableScale)
        );
    }

    // ─── Blocker regression tests ────────────────────────────────────────────

    /// Blocker 1 / Blocker 6: Unit circle q=(1,-minsub) must return u in
    /// `[0,TAU)`, never TAU.  The seam direction minsub = `f64::from_bits(1)` is
    /// the smallest positive subnormal (not `MIN_POSITIVE`).
    #[test]
    fn circle2_seam_minsub_u_in_range() {
        let circle = Circle2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            1.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        let minsub = f64::from_bits(1);
        let q = Point2::try_new(1.0, -minsub).unwrap();
        let projs = circle.project(q, &ctx()).unwrap();
        assert_eq!(projs.len(), 1);
        let u = projs[0].parameter.get();
        assert!(u >= 0.0, "u={u} must be >= 0");
        assert!(u < TAU, "u={u} must be < TAU, not TAU itself");
    }

    /// Blocker 6: Both seam sides return u in [0,TAU).
    #[test]
    fn circle2_seam_both_sides_u_in_range() {
        let circle = Circle2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            1.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        // Just above the seam (positive y side → u near 0)
        let q_pos = Point2::try_new(1.0, f64::from_bits(1)).unwrap();
        // Just below the seam (negative y side → u near TAU)
        let q_neg = Point2::try_new(1.0, -f64::from_bits(1)).unwrap();
        for q in [q_pos, q_neg] {
            let projs = circle.project(q, &ctx()).unwrap();
            assert_eq!(projs.len(), 1);
            let u = projs[0].parameter.get();
            assert!((0.0..TAU).contains(&u), "u={u} out of [0,TAU) for q={q:?}");
        }
    }

    /// Circle2 with approximate stored seed bits.
    /// Regression: axis bits 0x3fc52b6ffa8b3bf8, 0x3fc22f545e6f5468.
    /// The projection `distance_bound` must enclose the actual Euclidean
    /// distance from q to the returned point.
    #[test]
    fn circle2_ideal_projection_distance_bound() {
        // Raw axis bits for a non-exactly-unit stored x_axis.
        let xa_x = f64::from_bits(0x3fc5_2b6f_fa8b_3bf8);
        let xa_y = f64::from_bits(0x3fc2_2f54_5e6f_5468);
        // This axis has norm ≈ 1 - 1ulp; we construct via the public API which
        // normalizes it.
        let circle = Circle2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            1.0,
            Vector2::try_new(xa_x, xa_y).unwrap(),
        )
        .unwrap();
        // q = 2 * stored x_axis components (outside the circle)
        let stored_x = circle.x_axis().into_array();
        let q = Point2::try_new(2.0 * stored_x[0], 2.0 * stored_x[1]).unwrap();
        let projs = circle.project(q, &ctx()).unwrap();
        assert_eq!(projs.len(), 1);
        let actual_dist = {
            let [px, py] = projs[0].point.into_array();
            let [qx, qy] = q.into_array();
            (px - qx).hypot(py - qy)
        };
        assert!(
            projs[0].distance_bound.get() >= actual_dist,
            "distance_bound {} must enclose actual distance {}",
            projs[0].distance_bound.get(),
            actual_dist
        );
    }

    #[test]
    fn circle2_serialized_seed_evaluates_ideal_frame_with_bounds() {
        let circle: Circle2 = serde_json::from_value(json!({
            "version": {"major": 1, "minor": 0},
            "center": [1.0, -2.0],
            "radius": 3.0,
            "x_axis": [0.6, 0.8]
        }))
        .unwrap();
        let permissive =
            EvaluationContext::unlimited(ToleranceContext::try_new(10.0, 0.0, 1.0, 1.0).unwrap());
        let evaluation = circle
            .evaluate(0.0, DerivativeOrder::Second, &permissive)
            .unwrap();
        let norm = (0.6_f64.powi(2) + 0.8_f64.powi(2)).sqrt();
        let x = [0.6 / norm, 0.8 / norm];
        let y = [-x[1], x[0]];
        let norm2 =
            |actual: [f64; 2], ideal: [f64; 2]| (actual[0] - ideal[0]).hypot(actual[1] - ideal[1]);
        assert!(
            norm2(
                evaluation.position.into_array(),
                [1.0 + 3.0 * x[0], -2.0 + 3.0 * x[1]],
            ) <= evaluation.position_error_bound.get()
        );
        assert!(
            norm2(
                evaluation.first.unwrap().into_array(),
                [3.0 * y[0], 3.0 * y[1]]
            ) <= evaluation.first_error_bound.unwrap().get()
        );
        assert!(
            norm2(
                evaluation.second.unwrap().into_array(),
                [-3.0 * x[0], -3.0 * x[1]],
            ) <= evaluation.second_error_bound.unwrap().get()
        );
    }

    /// Blocker 2: Derivative limit check — a tight `first_or_du` limit must
    /// reject circle evaluation when the certified first derivative error
    /// bound exceeds the limit.
    #[test]
    fn circle2_derivative_limit_rejects_tight_limit() {
        let circle = Circle2::try_new(
            Point2::try_new(0.0, 0.0).unwrap(),
            1.0,
            Vector2::try_new(1.0, 0.0).unwrap(),
        )
        .unwrap();
        // A limit of 0.0 on the first derivative is always too tight.
        let limits = DerivativeLimits::new(
            CurveDerivativeLimits::new(
                CurveFirstDerivativeLimit::try_new(0.0).unwrap(),
                CurveSecondDerivativeLimit::try_new(f64::MAX).unwrap(),
            ),
            SurfaceDerivativeLimits::unlimited(),
        );
        let strict_ctx =
            EvaluationContext::try_new(tol(), CertificationBudget::default(), limits).unwrap();
        let result = circle.evaluate(0.5, DerivativeOrder::First, &strict_ctx);
        assert!(
            matches!(result, Err(GeometryError::Uncertified { .. })),
            "expected Uncertified for zero first-derivative limit, got {result:?}"
        );
    }

    /// Correction 10-G: a mismatched `SchemaVersion` major must be rejected.
    #[test]
    fn circle2_serde_version_rejection() {
        let invalid = serde_json::json!({
            "version": {"major": 99, "minor": 0},
            "center": [0.0, 0.0],
            "radius": 1.0,
            "x_axis": [1.0, 0.0]
        });
        let result: Result<Circle2, _> = serde_json::from_value(invalid);
        assert!(result.is_err(), "major=99 must be rejected");

        // A missing version field is also rejected (no serde default).
        let missing = serde_json::json!({
            "center": [0.0, 0.0],
            "radius": 1.0,
            "x_axis": [1.0, 0.0]
        });
        assert!(
            serde_json::from_value::<Circle2>(missing).is_err(),
            "missing version must be rejected"
        );

        // An unknown field is rejected (deny_unknown_fields).
        let unknown = serde_json::json!({
            "version": {"major": 1, "minor": 0},
            "center": [0.0, 0.0],
            "radius": 1.0,
            "x_axis": [1.0, 0.0],
            "bogus": 3
        });
        assert!(
            serde_json::from_value::<Circle2>(unknown).is_err(),
            "unknown field must be rejected"
        );
    }

    /// Item 6: the schema version must match **exactly**. The current `1.0` is
    /// accepted, but any other minor (previously accepted under a major-only
    /// check) is now rejected with [`ConstructionError::UnsupportedSchemaVersion`].
    #[test]
    fn circle2_serde_version_exact_match() {
        // Exact 1.0 is accepted.
        let valid = serde_json::json!({
            "version": {"major": 1, "minor": 0},
            "center": [0.0, 0.0],
            "radius": 1.0,
            "x_axis": [1.0, 0.0]
        });
        assert!(
            serde_json::from_value::<Circle2>(valid).is_ok(),
            "exact major=1 minor=0 should be accepted"
        );

        // A different minor is now rejected exactly, carrying the found and
        // supported versions.
        let wrong_minor: Circle2Repr = serde_json::from_value(serde_json::json!({
            "version": {"major": 1, "minor": 7},
            "center": [0.0, 0.0],
            "radius": 1.0,
            "x_axis": [1.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Circle2::try_from(wrong_minor),
            Err(ConstructionError::UnsupportedSchemaVersion {
                found: SchemaVersion::new(1, 7),
                supported: SchemaVersion::new(1, 0),
            }),
            "major=1 minor=7 must be rejected under exact-match"
        );
    }

    /// Item 6: `Circle3` also enforces exact schema-version matching.
    #[test]
    fn circle3_serde_version_exact_match() {
        let valid = serde_json::json!({
            "version": {"major": 1, "minor": 0},
            "center": [0.0, 0.0, 0.0],
            "radius": 1.0,
            "normal": [0.0, 0.0, 1.0],
            "x_axis": [1.0, 0.0, 0.0]
        });
        assert!(
            serde_json::from_value::<Circle3>(valid).is_ok(),
            "exact major=1 minor=0 should be accepted"
        );

        let wrong_minor: Circle3Repr = serde_json::from_value(serde_json::json!({
            "version": {"major": 1, "minor": 7},
            "center": [0.0, 0.0, 0.0],
            "radius": 1.0,
            "normal": [0.0, 0.0, 1.0],
            "x_axis": [1.0, 0.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Circle3::try_from(wrong_minor),
            Err(ConstructionError::UnsupportedSchemaVersion {
                found: SchemaVersion::new(1, 7),
                supported: SchemaVersion::new(1, 0),
            }),
            "major=1 minor=7 must be rejected under exact-match"
        );
    }

    #[test]
    fn circle3_skew_serialized_frame_evaluates_the_ideal_frame_with_bounds() {
        // This frozen seed cannot survive Circle3::try_new unchanged: its x
        // direction has an axial leak.
        let skew: Circle3 = serde_json::from_value(json!({
            "version": {"major": 1, "minor": 0},
            "center": [0.0, 0.0, 0.0],
            "radius": 2.0,
            "normal": [0.0, 0.0, 1.0],
            "x_axis": [0.099_833_416_646_828_15, 0.0, 0.995_004_165_278_025_8]
        }))
        .unwrap();
        let permissive =
            EvaluationContext::unlimited(ToleranceContext::try_new(10.0, 0.0, 1.0, 1.0).unwrap());
        let evaluation = skew
            .evaluate(0.0, DerivativeOrder::Second, &permissive)
            .unwrap();

        let position = evaluation.position.into_array();
        let first = evaluation.first.unwrap().into_array();
        let second = evaluation.second.unwrap().into_array();
        let norm = |actual: [f64; 3], ideal: [f64; 3]| {
            ((actual[0] - ideal[0]).powi(2)
                + (actual[1] - ideal[1]).powi(2)
                + (actual[2] - ideal[2]).powi(2))
            .sqrt()
        };
        assert!(norm(position, [2.0, 0.0, 0.0]) <= evaluation.position_error_bound.get());
        assert!(norm(first, [0.0, 2.0, 0.0]) <= evaluation.first_error_bound.unwrap().get());
        assert!(norm(second, [-2.0, 0.0, 0.0]) <= evaluation.second_error_bound.unwrap().get());
    }
}
