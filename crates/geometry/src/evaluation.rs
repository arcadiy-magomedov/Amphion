//! Evaluated points, derivatives, and inverse mappings.

#![allow(clippy::missing_panics_doc)]

use amphion_foundation::{Point2, Point3, ToleranceContext, Vector2, Vector3};
use serde::{Deserialize, Serialize};

/// A non-negative certified upper bound on a periodic (angular) parameter
/// error, in radians.
///
/// A bound of `δ` means the true nearest parameter is within `δ` radians
/// of the reported parameter **in the periodic sense** — values near the
/// seam (`u ≈ 0` or `u ≈ 2π`) are equated. This is not a linear distance
/// across `[0, 2π)` but a circular distance.
///
/// Used for circle (`θ`), cylinder (`u`), and cone (`u`) parameters.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct AngularParameterBound(f64);

impl AngularParameterBound {
    /// Creates an angular parameter error bound.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectionValueError`] when `value` is negative, NaN, or
    /// infinite.
    pub fn try_new(value: f64) -> Result<Self, ProjectionValueError> {
        Self::try_from(value)
    }

    /// Returns the bound in radians.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for AngularParameterBound {
    type Error = ProjectionValueError;
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_finite() && value >= 0.0 {
            Ok(Self(value))
        } else {
            Err(ProjectionValueError)
        }
    }
}

impl From<AngularParameterBound> for f64 {
    fn from(value: AngularParameterBound) -> Self {
        value.0
    }
}

/// A non-negative certified upper bound on a non-periodic (linear) parameter
/// error.
///
/// Used for line (`t`), plane (`u`, `v`), cylinder (`v`), and cone (`v`)
/// parameters.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct LinearParameterBound(f64);

impl LinearParameterBound {
    /// Creates a linear parameter error bound.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectionValueError`] when `value` is negative, NaN, or
    /// infinite.
    pub fn try_new(value: f64) -> Result<Self, ProjectionValueError> {
        Self::try_from(value)
    }

    /// Returns the bound.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for LinearParameterBound {
    type Error = ProjectionValueError;
    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_finite() && value >= 0.0 {
            Ok(Self(value))
        } else {
            Err(ProjectionValueError)
        }
    }
}

impl From<LinearParameterBound> for f64 {
    fn from(value: LinearParameterBound) -> Self {
        value.0
    }
}

/// A dimensionally typed certified upper bound on a curve or surface
/// parameter error.
///
/// The `Angular` variant carries a periodic (circular) bound in radians;
/// the `Linear` variant carries a non-periodic bound. Both enforce that the
/// inner value is non-negative and finite.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum ParameterErrorBound {
    /// A periodic angular bound in radians (circles, cylinder/cone u).
    Angular(AngularParameterBound),
    /// A non-periodic linear bound (lines, plane u/v, cylinder/cone v).
    Linear(LinearParameterBound),
}

impl ParameterErrorBound {
    /// Returns the bound as a raw `f64` (non-negative).
    #[must_use]
    pub fn get(self) -> f64 {
        match self {
            Self::Angular(b) => b.get(),
            Self::Linear(b) => b.get(),
        }
    }
}

/// Declares a validated, non-negative, **finite** derivative-limit newtype.
///
/// Construction goes through [`Self::try_new`], which rejects NaN, negative
/// values, **and** `±∞`. To express "no effective limit" use [`f64::MAX`],
/// which is finite. Rejecting `±∞` (in addition to NaN) closes two bypass
/// holes of a bare `f64` field: a NaN limit would silently disable the check
/// because `bound > NaN` is always false, and a `+∞` limit is not a
/// meaningful certified error bound.
macro_rules! derivative_limit_newtype {
    ($(#[$meta:meta])* $name:ident, $unit:literal) => {
        $(#[$meta])*
        ///
        #[doc = concat!("Units: ", $unit, ".")]
        #[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
        #[serde(try_from = "f64", into = "f64")]
        pub struct $name(f64);

        impl $name {
            #[doc = concat!("Creates a validated `", stringify!($name), "`.")]
            ///
            /// # Errors
            ///
            /// Returns [`ProjectionValueError`] when `value` is NaN, negative,
            /// or infinite. Use [`f64::MAX`] to mean "no effective limit".
            pub fn try_new(value: f64) -> Result<Self, ProjectionValueError> {
                Self::try_from(value)
            }

            /// Returns the limit.
            #[must_use]
            pub const fn get(self) -> f64 {
                self.0
            }
        }

        impl TryFrom<f64> for $name {
            type Error = ProjectionValueError;
            fn try_from(value: f64) -> Result<Self, Self::Error> {
                // Accept only finite, non-negative values: this rejects NaN,
                // negatives, and ±∞. `f64::MAX` is finite and means "no
                // effective limit".
                if value.is_finite() && value >= 0.0 {
                    Ok(Self(value))
                } else {
                    Err(ProjectionValueError)
                }
            }
        }

        impl From<$name> for f64 {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

derivative_limit_newtype!(
    /// Limit on the certified error bound of a curve's first derivative.
    CurveFirstDerivativeLimit,
    "metres per radian (circles) or metres per unit parameter (lines)"
);
derivative_limit_newtype!(
    /// Limit on the certified error bound of a curve's second derivative.
    CurveSecondDerivativeLimit,
    "metres per radian² or metres per unit parameter²"
);
derivative_limit_newtype!(
    /// Limit on the certified error bound of a surface `∂p/∂u`.
    SurfaceDuLimit,
    "metres per radian (u periodic) or metres per unit parameter"
);
derivative_limit_newtype!(
    /// Limit on the certified error bound of a surface `∂p/∂v`.
    SurfaceDvLimit,
    "metres per unit parameter"
);
derivative_limit_newtype!(
    /// Limit on the certified error bound of a surface `∂²p/∂u²`.
    SurfaceDuuLimit,
    "metres per radian² or metres per unit parameter²"
);
derivative_limit_newtype!(
    /// Limit on the certified error bound of a surface `∂²p/∂u∂v`.
    SurfaceDuvLimit,
    "metres per (radian·unit) or metres per unit parameter²"
);
derivative_limit_newtype!(
    /// Limit on the certified error bound of a surface `∂²p/∂v²`.
    SurfaceDvvLimit,
    "metres per unit parameter²"
);

/// Mandatory derivative limits for curve evaluation (circle/line).
///
/// Every slot always carries a finite, non-negative limit; there is no
/// `Option<...>` and no `Default`. Use [`CurveDerivativeLimits::unlimited()`]
/// for no effective limit (each slot set to [`f64::MAX`]).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CurveDerivativeLimits {
    /// Certified error bound limit for the first derivative.
    pub first: CurveFirstDerivativeLimit,
    /// Certified error bound limit for the second derivative.
    pub second: CurveSecondDerivativeLimit,
}

impl CurveDerivativeLimits {
    /// Creates curve derivative limits with explicit per-slot limits.
    #[must_use]
    pub fn new(first: CurveFirstDerivativeLimit, second: CurveSecondDerivativeLimit) -> Self {
        Self { first, second }
    }

    /// No effective limit on any derivative slot (uses [`f64::MAX`]).
    #[must_use]
    pub fn unlimited() -> Self {
        Self {
            first: CurveFirstDerivativeLimit::try_new(f64::MAX).expect("MAX is finite nonneg"),
            second: CurveSecondDerivativeLimit::try_new(f64::MAX).expect("MAX is finite nonneg"),
        }
    }
}

/// Mandatory derivative limits for surface evaluation (plane/cylinder/cone).
///
/// Every slot always carries a finite, non-negative limit; there is no
/// `Option<...>` and no `Default`. Use [`SurfaceDerivativeLimits::unlimited()`]
/// for no effective limit (each slot set to [`f64::MAX`]).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SurfaceDerivativeLimits {
    /// Certified error bound limit for `∂p/∂u`.
    pub du: SurfaceDuLimit,
    /// Certified error bound limit for `∂p/∂v`.
    pub dv: SurfaceDvLimit,
    /// Certified error bound limit for `∂²p/∂u²`.
    pub duu: SurfaceDuuLimit,
    /// Certified error bound limit for `∂²p/∂u∂v`.
    pub duv: SurfaceDuvLimit,
    /// Certified error bound limit for `∂²p/∂v²`.
    pub dvv: SurfaceDvvLimit,
}

impl SurfaceDerivativeLimits {
    /// Creates surface derivative limits with explicit per-slot limits.
    #[must_use]
    pub fn new(
        du: SurfaceDuLimit,
        dv: SurfaceDvLimit,
        duu: SurfaceDuuLimit,
        duv: SurfaceDuvLimit,
        dvv: SurfaceDvvLimit,
    ) -> Self {
        Self {
            du,
            dv,
            duu,
            duv,
            dvv,
        }
    }

    /// No effective limit on any derivative slot (uses [`f64::MAX`]).
    #[must_use]
    pub fn unlimited() -> Self {
        Self {
            du: SurfaceDuLimit::try_new(f64::MAX).expect("MAX is finite nonneg"),
            dv: SurfaceDvLimit::try_new(f64::MAX).expect("MAX is finite nonneg"),
            duu: SurfaceDuuLimit::try_new(f64::MAX).expect("MAX is finite nonneg"),
            duv: SurfaceDuvLimit::try_new(f64::MAX).expect("MAX is finite nonneg"),
            dvv: SurfaceDvvLimit::try_new(f64::MAX).expect("MAX is finite nonneg"),
        }
    }
}

/// Mandatory per-slot derivative error bound limits.
///
/// Contains both curve and surface limit groups; use
/// [`DerivativeLimits::unlimited()`] for no effective limits. There is no
/// `Default` and no `Option<...>` — every slot always carries a finite,
/// non-negative limit; [`f64::MAX`] means "no effective limit".
///
/// ## Slot semantics
/// - Curve primitives (Line2/3, Circle2/3) use only the `curve` limits.
/// - Surface primitives (Plane, Cylinder, Cone) use only the `surface` limits.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DerivativeLimits {
    /// Limits applied to curve derivative slots.
    pub curve: CurveDerivativeLimits,
    /// Limits applied to surface derivative slots.
    pub surface: SurfaceDerivativeLimits,
}

impl DerivativeLimits {
    /// Creates combined derivative limits from a curve and surface group.
    #[must_use]
    pub fn new(curve: CurveDerivativeLimits, surface: SurfaceDerivativeLimits) -> Self {
        Self { curve, surface }
    }

    /// No effective limit on any derivative slot (uses [`f64::MAX`]).
    #[must_use]
    pub fn unlimited() -> Self {
        Self {
            curve: CurveDerivativeLimits::unlimited(),
            surface: SurfaceDerivativeLimits::unlimited(),
        }
    }
}

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

/// A finite, non-negative certified upper bound on position error, in
/// model-space metres.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct PositionBound(f64);

impl PositionBound {
    /// Creates a certified position-error bound.
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

impl TryFrom<f64> for PositionBound {
    type Error = ProjectionValueError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_finite() && value >= 0.0 {
            Ok(Self(value))
        } else {
            Err(ProjectionValueError)
        }
    }
}

impl From<PositionBound> for f64 {
    fn from(value: PositionBound) -> Self {
        value.0
    }
}

/// A finite, non-negative certified upper bound on first-derivative error.
///
/// Units are model-space metres per radian for angular parameterizations
/// (circle, cylinder, cone) and metres per unit parameter for linear
/// parameterizations (line, plane).
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct FirstDerivativeBound(f64);

impl FirstDerivativeBound {
    /// Creates a certified first-derivative error bound.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectionValueError`] when `value` is negative, NaN, or
    /// infinite.
    pub fn try_new(value: f64) -> Result<Self, ProjectionValueError> {
        Self::try_from(value)
    }

    /// Returns the bound.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for FirstDerivativeBound {
    type Error = ProjectionValueError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_finite() && value >= 0.0 {
            Ok(Self(value))
        } else {
            Err(ProjectionValueError)
        }
    }
}

impl From<FirstDerivativeBound> for f64 {
    fn from(value: FirstDerivativeBound) -> Self {
        value.0
    }
}

/// A finite, non-negative certified upper bound on second-derivative error.
///
/// Units are model-space metres per radian² for angular parameterizations
/// (circle, cylinder, cone) and metres per unit parameter² for linear
/// parameterizations (line, plane).
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct SecondDerivativeBound(f64);

impl SecondDerivativeBound {
    /// Creates a certified second-derivative error bound.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectionValueError`] when `value` is negative, NaN, or
    /// infinite.
    pub fn try_new(value: f64) -> Result<Self, ProjectionValueError> {
        Self::try_from(value)
    }

    /// Returns the bound.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for SecondDerivativeBound {
    type Error = ProjectionValueError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if value.is_finite() && value >= 0.0 {
            Ok(Self(value))
        } else {
            Err(ProjectionValueError)
        }
    }
}

impl From<SecondDerivativeBound> for f64 {
    fn from(value: SecondDerivativeBound) -> Self {
        value.0
    }
}

/// Budget that caps certified rational-arithmetic computations (series
/// truncation, intermediate `BigRational` bit-width).
///
/// Exhausting any limit causes the evaluator to return
/// [`crate::GeometryError::Uncertified`] rather than continue with an
/// uncertified result.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct CertificationBudget {
    /// Maximum terms in any Taylor or alternating series.
    pub series_terms: u32,
    /// Maximum bits in any intermediate `BigRational` numerator or
    /// denominator. Prevents unbounded memory growth on adversarial input.
    pub rational_bits: u32,
}

impl Default for CertificationBudget {
    fn default() -> Self {
        Self {
            series_terms: 200,
            rational_bits: 1 << 20,
        }
    }
}

/// Combined context for evaluation and projection: tolerance limits plus a
/// certified rational-arithmetic computation budget and mandatory per-slot
/// derivative limits.
///
/// Fields are private and can only be set through
/// [`EvaluationContext::try_new`], [`EvaluationContext::unlimited`], and
/// [`EvaluationContext::with_budget`], each of which routes through validated
/// newtypes. This prevents the struct-literal bypass where a NaN budget or
/// derivative limit could be set without validation. Deserialization goes
/// through the same path via a private validating surrogate
/// (`EvaluationContextRepr`).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "EvaluationContextRepr", into = "EvaluationContextRepr")]
pub struct EvaluationContext {
    tolerance: ToleranceContext,
    budget: CertificationBudget,
    derivative_limits: DerivativeLimits,
}

/// Serde surrogate for [`EvaluationContext`]. Deserialization runs through
/// [`EvaluationContext::try_from`], which validates the budget. An omitted
/// `derivative_limits` field defaults to [`DerivativeLimits::unlimited()`].
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct EvaluationContextRepr {
    tolerance: ToleranceContext,
    #[serde(default)]
    budget: CertificationBudget,
    #[serde(default = "DerivativeLimits::unlimited")]
    derivative_limits: DerivativeLimits,
}

impl EvaluationContext {
    /// Creates a context with explicit tolerance, budget, and derivative
    /// limits.
    ///
    /// # Errors
    ///
    /// Returns [`ProjectionValueError`] when the certification `budget` is
    /// degenerate (either `series_terms` or `rational_bits` is zero); such a
    /// budget can never certify any computation. All other combinations are
    /// accepted.
    pub fn try_new(
        tolerance: ToleranceContext,
        budget: CertificationBudget,
        limits: DerivativeLimits,
    ) -> Result<Self, ProjectionValueError> {
        if budget.series_terms == 0 || budget.rational_bits == 0 {
            return Err(ProjectionValueError);
        }
        Ok(Self {
            tolerance,
            budget,
            derivative_limits: limits,
        })
    }

    /// Convenience: explicit tolerance with the default certification budget
    /// and unlimited derivative limits ([`DerivativeLimits::unlimited()`]).
    #[must_use]
    pub fn unlimited(tolerance: ToleranceContext) -> Self {
        Self {
            tolerance,
            budget: CertificationBudget::default(),
            derivative_limits: DerivativeLimits::unlimited(),
        }
    }

    /// Returns a copy of this context with the given certification budget.
    #[must_use]
    pub fn with_budget(mut self, budget: CertificationBudget) -> Self {
        self.budget = budget;
        self
    }

    /// Returns the modeling tolerance context.
    #[must_use]
    pub fn tolerance(&self) -> ToleranceContext {
        self.tolerance
    }

    /// Returns the certification budget.
    #[must_use]
    pub fn budget(&self) -> CertificationBudget {
        self.budget
    }

    /// Returns the mandatory per-slot derivative limits.
    #[must_use]
    pub fn derivative_limits(&self) -> DerivativeLimits {
        self.derivative_limits
    }
}

impl TryFrom<EvaluationContextRepr> for EvaluationContext {
    type Error = ProjectionValueError;

    fn try_from(repr: EvaluationContextRepr) -> Result<Self, Self::Error> {
        Self::try_new(repr.tolerance, repr.budget, repr.derivative_limits)
    }
}

impl From<EvaluationContext> for EvaluationContextRepr {
    fn from(context: EvaluationContext) -> Self {
        Self {
            tolerance: context.tolerance,
            budget: context.budget,
            derivative_limits: context.derivative_limits,
        }
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
    pub position_error_bound: PositionBound,
    /// Certified upper bound on `‖first − true p′(t)‖`, or `None` when
    /// `first` is `None`.
    pub first_error_bound: Option<FirstDerivativeBound>,
    /// Certified upper bound on `‖second − true p″(t)‖`, or `None` when
    /// `second` is `None`.
    pub second_error_bound: Option<SecondDerivativeBound>,
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
    pub position_error_bound: PositionBound,
    /// Certified upper bound on `‖first − true p′(t)‖`, or `None` when
    /// `first` is `None`.
    pub first_error_bound: Option<FirstDerivativeBound>,
    /// Certified upper bound on `‖second − true p″(t)‖`, or `None` when
    /// `second` is `None`.
    pub second_error_bound: Option<SecondDerivativeBound>,
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
    pub position_error_bound: PositionBound,
    /// Certified upper bound on `‖du − true ∂p/∂u‖`, or `None` when `du` is
    /// `None`.
    pub first_u_error_bound: Option<FirstDerivativeBound>,
    /// Certified upper bound on `‖dv − true ∂p/∂v‖`, or `None` when `dv` is
    /// `None`.
    pub first_v_error_bound: Option<FirstDerivativeBound>,
    /// Certified upper bound on `‖duu − true ∂²p/∂u²‖`, or `None` when
    /// `duu` is `None`.
    pub second_uu_error_bound: Option<SecondDerivativeBound>,
    /// Certified upper bound on `‖duv − true ∂²p/∂u∂v‖`, or `None` when
    /// `duv` is `None`.
    pub second_uv_error_bound: Option<SecondDerivativeBound>,
    /// Certified upper bound on `‖dvv − true ∂²p/∂v²‖`, or `None` when
    /// `dvv` is `None`.
    pub second_vv_error_bound: Option<SecondDerivativeBound>,
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
    /// Dimensionally typed certified upper bound on the parameter error.
    /// `Angular` for circles, `Linear` for lines.
    pub parameter_error_bound: ParameterErrorBound,
    /// Certified upper bound on `‖point − true_nearest_point_on_primitive‖`
    /// in metres.
    pub point_residual_bound: PositionBound,
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
    /// Dimensionally typed certified upper bound on the parameter error.
    /// `Angular` for circles, `Linear` for lines.
    pub parameter_error_bound: ParameterErrorBound,
    /// Certified upper bound on `‖point − true_nearest_point_on_primitive‖`
    /// in metres.
    pub point_residual_bound: PositionBound,
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
    /// Dimensionally typed certified upper bound on the U parameter error.
    /// `Angular` for cylinder/cone (periodic u ∈ `[0, 2π)`);
    /// `Linear` for plane (non-periodic).
    pub u_error_bound: ParameterErrorBound,
    /// Certified upper bound on the V parameter error (always linear/non-periodic).
    pub v_error_bound: LinearParameterBound,
    /// Certified upper bound on `‖point − true_nearest_point_on_primitive‖`
    /// in metres.
    pub point_residual_bound: PositionBound,
}

#[cfg(test)]
mod tests {
    use super::{
        CertificationBudget, CurveDerivativeLimits, CurveFirstDerivativeLimit,
        CurveSecondDerivativeLimit, DerivativeLimits, EvaluationContext, SurfaceDerivativeLimits,
        SurfaceDuLimit,
    };
    use amphion_foundation::ToleranceContext;

    fn tol() -> ToleranceContext {
        ToleranceContext::try_new(1e-9, 1e-12, 1e-9, 1e-9).unwrap()
    }

    #[test]
    fn derivative_limit_rejects_nan() {
        assert!(CurveFirstDerivativeLimit::try_new(f64::NAN).is_err());
    }

    #[test]
    fn derivative_limit_rejects_negative() {
        assert!(CurveFirstDerivativeLimit::try_new(-1.0).is_err());
    }

    #[test]
    fn derivative_limit_rejects_inf() {
        assert!(CurveFirstDerivativeLimit::try_new(f64::INFINITY).is_err());
        assert!(SurfaceDuLimit::try_new(f64::INFINITY).is_err());
    }

    #[test]
    fn derivative_limit_accepts_max() {
        let limit = CurveFirstDerivativeLimit::try_new(f64::MAX).expect("MAX accepted");
        assert_eq!(limit.get().to_bits(), f64::MAX.to_bits());
    }

    #[test]
    fn evaluation_context_unlimited_works() {
        let ctx = EvaluationContext::unlimited(tol());
        assert_eq!(
            ctx.derivative_limits().curve.first.get().to_bits(),
            f64::MAX.to_bits()
        );
        assert_eq!(ctx.budget(), CertificationBudget::default());
    }

    #[test]
    fn evaluation_context_try_new_works() {
        let ctx = EvaluationContext::try_new(
            tol(),
            CertificationBudget::default(),
            DerivativeLimits::unlimited(),
        );
        assert!(ctx.is_ok());
    }

    #[test]
    fn evaluation_context_try_new_rejects_degenerate_budget() {
        let bad = CertificationBudget {
            series_terms: 0,
            rational_bits: 0,
        };
        assert!(EvaluationContext::try_new(tol(), bad, DerivativeLimits::unlimited()).is_err());
    }

    #[test]
    fn evaluation_context_with_budget_overrides() {
        let budget = CertificationBudget {
            series_terms: 10,
            rational_bits: 1 << 10,
        };
        let ctx = EvaluationContext::unlimited(tol()).with_budget(budget);
        assert_eq!(ctx.budget(), budget);
    }

    #[test]
    fn evaluation_context_serde_round_trip() {
        let limits = DerivativeLimits::new(
            CurveDerivativeLimits::new(
                CurveFirstDerivativeLimit::try_new(0.01).unwrap(),
                CurveSecondDerivativeLimit::try_new(0.02).unwrap(),
            ),
            SurfaceDerivativeLimits::unlimited(),
        );
        let ctx =
            EvaluationContext::try_new(tol(), CertificationBudget::default(), limits).unwrap();
        let json = serde_json::to_string(&ctx).unwrap();
        let back: EvaluationContext = serde_json::from_str(&json).unwrap();
        assert_eq!(ctx, back);
    }

    #[test]
    fn evaluation_context_serde_rejects_negative_limit() {
        // A negative derivative limit must be rejected by the validated
        // newtype deserializer (the NaN/Inf-bypass holes are closed the same
        // way).
        let limits = DerivativeLimits::new(
            CurveDerivativeLimits::new(
                CurveFirstDerivativeLimit::try_new(0.01).unwrap(),
                CurveSecondDerivativeLimit::try_new(0.02).unwrap(),
            ),
            SurfaceDerivativeLimits::unlimited(),
        );
        let ctx =
            EvaluationContext::try_new(tol(), CertificationBudget::default(), limits).unwrap();
        let json = serde_json::to_string(&ctx).unwrap();
        let mutated = json.replace("\"first\":0.01", "\"first\":-1.0");
        assert_ne!(mutated, json, "expected to mutate the limits field");
        let parsed: Result<EvaluationContext, _> = serde_json::from_str(&mutated);
        assert!(
            parsed.is_err(),
            "negative derivative limit must be rejected"
        );
    }

    #[test]
    fn evaluation_context_serde_rejects_inf_limit() {
        // `+∞` is no longer an accepted derivative limit.
        let limits = DerivativeLimits::new(
            CurveDerivativeLimits::new(
                CurveFirstDerivativeLimit::try_new(0.01).unwrap(),
                CurveSecondDerivativeLimit::try_new(0.02).unwrap(),
            ),
            SurfaceDerivativeLimits::unlimited(),
        );
        let ctx =
            EvaluationContext::try_new(tol(), CertificationBudget::default(), limits).unwrap();
        let json = serde_json::to_string(&ctx).unwrap();
        let mutated = json.replace("\"first\":0.01", "\"first\":1e400");
        assert_ne!(mutated, json, "expected to mutate the limits field");
        let parsed: Result<EvaluationContext, _> = serde_json::from_str(&mutated);
        assert!(
            parsed.is_err(),
            "infinite derivative limit must be rejected"
        );
    }

    #[test]
    fn evaluation_context_serde_rejects_zero_budget() {
        let ctx = EvaluationContext::unlimited(tol());
        let json = serde_json::to_string(&ctx).unwrap();
        let mutated = json.replace("\"series_terms\":200", "\"series_terms\":0");
        assert_ne!(mutated, json, "expected to mutate the budget field");
        let parsed: Result<EvaluationContext, _> = serde_json::from_str(&mutated);
        assert!(parsed.is_err(), "zero series_terms budget must be rejected");
    }
}
