//! Evaluated points, derivatives, and inverse mappings.

use amphion_foundation::{Point2, Point3, Vector2, Vector3};
use serde::{Deserialize, Serialize};

/// Error returned for an invalid projection parameter or error bound.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ProjectionValueError;

impl core::fmt::Display for ProjectionValueError {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("projection parameters must be finite and distance bounds non-negative")
    }
}

impl core::error::Error for ProjectionValueError {}

/// A finite curve or surface parameter.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct ParameterValue(f64);

impl ParameterValue {
    /// Creates a finite parameter.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectionValueError`] when `value` is NaN or infinite.
    pub fn try_new(value: f64) -> Result<Self, ProjectionValueError> {
        Self::try_from(value)
    }

    /// Returns the scalar parameter.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for ParameterValue {
    type Error = ProjectionValueError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_finite() {
            Ok(Self(value))
        } else {
            Err(ProjectionValueError)
        }
    }
}

impl From<ParameterValue> for f64 {
    fn from(value: ParameterValue) -> Self {
        value.0
    }
}

/// A finite, non-negative upper bound on model-space distance.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct DistanceBound(f64);

impl DistanceBound {
    /// Creates a certified distance bound.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectionValueError`] when `value` is negative, NaN, or
    /// infinite.
    pub fn try_new(value: f64) -> Result<Self, ProjectionValueError> {
        Self::try_from(value)
    }

    /// Returns the bound in model-space metres.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for DistanceBound {
    type Error = ProjectionValueError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_finite() && value >= 0.0 {
            Ok(Self(value))
        } else {
            Err(ProjectionValueError)
        }
    }
}

impl From<DistanceBound> for f64 {
    fn from(value: DistanceBound) -> Self {
        value.0
    }
}

/// Highest derivative requested from an evaluator.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum DerivativeOrder {
    /// Position only.
    Position,
    /// Position and first derivative.
    First,
    /// Position, first derivative, and second derivative.
    Second,
}

/// A two-dimensional curve evaluation.
///
/// Evaluated position and derivatives with certified error bounds.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveEvaluation2 {
    /// Evaluated position in 2-D parameter space.
    pub position: Point2,
    /// First derivative, if requested.
    pub first: Option<Vector2>,
    /// Second derivative, if requested.
    pub second: Option<Vector2>,
    /// Certified upper bound on `‖position − true p(t)‖` in metres.
    /// Accounts for floating-point arithmetic error in the evaluation
    /// formula and stored-frame deviation. Zero only for exactly
    /// representable cases.
    pub position_error_bound: DistanceBound,
    /// Certified upper bound on `‖first − true p′(t)‖`, or `None` when
    /// `first` is `None`.
    pub first_error_bound: Option<DistanceBound>,
    /// Certified upper bound on `‖second − true p″(t)‖`, or `None` when
    /// `second` is `None`.
    pub second_error_bound: Option<DistanceBound>,
}

/// A three-dimensional curve evaluation.
///
/// Evaluated position and derivatives with certified error bounds.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveEvaluation3 {
    /// Evaluated point.
    pub position: Point3,
    /// First derivative when requested.
    pub first: Option<Vector3>,
    /// Second derivative when requested.
    pub second: Option<Vector3>,
    /// Certified upper bound on `‖position − true p(t)‖` in metres.
    pub position_error_bound: DistanceBound,
    /// Certified upper bound on `‖first − true p′(t)‖`, or `None` when
    /// `first` is `None`.
    pub first_error_bound: Option<DistanceBound>,
    /// Certified upper bound on `‖second − true p″(t)‖`, or `None` when
    /// `second` is `None`.
    pub second_error_bound: Option<DistanceBound>,
}

/// A surface evaluation through second order.
///
/// Evaluated position and derivatives with certified error bounds.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SurfaceEvaluation {
    /// Evaluated point.
    pub position: Point3,
    /// First partial derivative with respect to U.
    pub du: Option<Vector3>,
    /// First partial derivative with respect to V.
    pub dv: Option<Vector3>,
    /// Second partial derivative with respect to U.
    pub duu: Option<Vector3>,
    /// Mixed second partial derivative.
    pub duv: Option<Vector3>,
    /// Second partial derivative with respect to V.
    pub dvv: Option<Vector3>,
    /// Certified upper bound on `‖position − true p(u,v)‖` in metres.
    pub position_error_bound: DistanceBound,
    /// Certified upper bound on `‖du − true ∂p/∂u‖`, or `None` when `du` is
    /// `None`.
    pub first_u_error_bound: Option<DistanceBound>,
    /// Certified upper bound on `‖dv − true ∂p/∂v‖`, or `None` when `dv` is
    /// `None`.
    pub first_v_error_bound: Option<DistanceBound>,
    /// Certified upper bound on `‖duu − true ∂²p/∂u²‖`, or `None` when
    /// `duu` is `None`.
    pub second_uu_error_bound: Option<DistanceBound>,
    /// Certified upper bound on `‖duv − true ∂²p/∂u∂v‖`, or `None` when
    /// `duv` is `None`.
    pub second_uv_error_bound: Option<DistanceBound>,
    /// Certified upper bound on `‖dvv − true ∂²p/∂v²‖`, or `None` when
    /// `dvv` is `None`.
    pub second_vv_error_bound: Option<DistanceBound>,
}

/// One inverse mapping from model space to a two-dimensional curve.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveProjection2 {
    /// Curve parameter.
    pub parameter: ParameterValue,
    /// Evaluated point on the curve.
    pub point: Point2,
    /// Certified upper bound on projection distance.
    pub distance_bound: DistanceBound,
}

/// One inverse mapping from model space to a three-dimensional curve.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveProjection3 {
    /// Curve parameter.
    pub parameter: ParameterValue,
    /// Evaluated point on the curve.
    pub point: Point3,
    /// Certified upper bound on projection distance.
    pub distance_bound: DistanceBound,
}

/// One inverse mapping from model space to a surface.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SurfaceProjection {
    /// Surface U parameter.
    pub u: ParameterValue,
    /// Surface V parameter.
    pub v: ParameterValue,
    /// Evaluated point on the surface.
    pub point: Point3,
    /// Certified upper bound on projection distance.
    pub distance_bound: DistanceBound,
}
