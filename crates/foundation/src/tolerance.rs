//! Explicit, scale-aware comparison tolerances.

use core::error::Error;
use core::fmt;

use serde::{Deserialize, Serialize};

/// Invalid tolerance input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ToleranceError {
    /// A tolerance component was NaN or infinite.
    NonFinite,
    /// An absolute, angular, or parametric tolerance was not positive.
    NotPositive,
    /// A relative tolerance was negative.
    NegativeRelative,
    /// A scale supplied for an effective tolerance was invalid.
    InvalidScale,
    /// Arithmetic overflowed while evaluating a tolerance comparison.
    Overflow,
}

impl fmt::Display for ToleranceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NonFinite => "tolerance components must be finite",
            Self::NotPositive => {
                "absolute length, angular, and parametric tolerances must be positive"
            }
            Self::NegativeRelative => "relative length tolerance must not be negative",
            Self::InvalidScale => "comparison scale must be finite and non-negative",
            Self::Overflow => {
                "tolerance comparison overflowed; coordinates may be too large for the supplied tolerance"
            }
        };
        formatter.write_str(message)
    }
}

impl Error for ToleranceError {}

/// The result of classifying a scalar relative to a tolerance band.
///
/// Used by the `classify_*` methods on [`ToleranceContext`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Classification {
    /// The value is strictly below the negative tolerance threshold.
    BelowTolerance,
    /// The value's absolute magnitude is within the tolerance band.
    WithinTolerance,
    /// The value is strictly above the positive tolerance threshold.
    AboveTolerance,
}

/// A positive finite model-space tolerance in metres.
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd, Serialize, Deserialize)]
#[serde(try_from = "f64", into = "f64")]
pub struct LengthTolerance(f64);

impl LengthTolerance {
    /// Creates a certified entity tolerance.
    ///
    /// # Errors
    ///
    /// Returns [`ToleranceError`] unless `value` is positive and finite.
    pub fn try_new(value: f64) -> Result<Self, ToleranceError> {
        Self::try_from(value)
    }

    /// Returns the tolerance in model-space metres.
    #[must_use]
    pub const fn get(self) -> f64 {
        self.0
    }
}

impl TryFrom<f64> for LengthTolerance {
    type Error = ToleranceError;

    fn try_from(value: f64) -> Result<Self, Self::Error> {
        if !value.is_finite() {
            return Err(ToleranceError::NonFinite);
        }
        if value <= 0.0 {
            return Err(ToleranceError::NotPositive);
        }
        Ok(Self(value))
    }
}

impl From<LengthTolerance> for f64 {
    fn from(value: LengthTolerance) -> Self {
        value.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct ToleranceRepr {
    absolute_length: f64,
    relative_length: f64,
    angular: f64,
    parametric: f64,
}

/// All tolerances required by a geometry operation.
///
/// There is deliberately no `Default` implementation. A document, import, or
/// caller must choose a context explicitly and pass it through every
/// comparison-based operation.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ToleranceRepr", into = "ToleranceRepr")]
pub struct ToleranceContext(ToleranceRepr);

impl ToleranceContext {
    /// Creates a validated tolerance context.
    ///
    /// # Errors
    ///
    /// Returns [`ToleranceError`] for non-finite components, non-positive
    /// absolute/angular/parametric tolerances, or a negative relative
    /// tolerance.
    pub fn try_new(
        absolute_length: f64,
        relative_length: f64,
        angular: f64,
        parametric: f64,
    ) -> Result<Self, ToleranceError> {
        Self::try_from(ToleranceRepr {
            absolute_length,
            relative_length,
            angular,
            parametric,
        })
    }

    /// Returns the absolute length tolerance in model-space metres.
    #[must_use]
    pub const fn absolute_length(self) -> f64 {
        self.0.absolute_length
    }

    /// Returns the dimensionless relative length tolerance.
    #[must_use]
    pub const fn relative_length(self) -> f64 {
        self.0.relative_length
    }

    /// Returns the angular tolerance in radians.
    #[must_use]
    pub const fn angular(self) -> f64 {
        self.0.angular
    }

    /// Returns the tolerance used in curve and surface parameter spaces.
    #[must_use]
    pub const fn parametric(self) -> f64 {
        self.0.parametric
    }

    /// Computes `max(absolute, relative * scale)` for a model-space scale.
    ///
    /// # Errors
    ///
    /// Returns [`ToleranceError::InvalidScale`] when `scale` is NaN, infinite,
    /// or negative.
    pub fn effective_length(self, scale: f64) -> Result<f64, ToleranceError> {
        if !scale.is_finite() || scale < 0.0 {
            return Err(ToleranceError::InvalidScale);
        }
        let rel = self.relative_length() * scale;
        if !rel.is_finite() {
            return Err(ToleranceError::Overflow);
        }
        Ok(self.absolute_length().max(rel))
    }

    /// Classifies `value` relative to the scale-aware length tolerance.
    ///
    /// Returns:
    /// - [`Classification::BelowTolerance`] when `value < -tol`,
    /// - [`Classification::WithinTolerance`] when `|value| <= tol`,
    /// - [`Classification::AboveTolerance`] when `value > tol`.
    ///
    /// # Errors
    ///
    /// Propagates [`ToleranceError`] from [`effective_length`][Self::effective_length].
    pub fn classify_length(self, value: f64, scale: f64) -> Result<Classification, ToleranceError> {
        if !value.is_finite() {
            return Err(ToleranceError::NonFinite);
        }
        let tol = self.effective_length(scale)?;
        if value > tol {
            Ok(Classification::AboveTolerance)
        } else if value < -tol {
            Ok(Classification::BelowTolerance)
        } else {
            Ok(Classification::WithinTolerance)
        }
    }

    /// Returns `true` when `|a - b|` is within the scale-aware length
    /// tolerance.
    ///
    /// # Errors
    ///
    /// Propagates [`ToleranceError`] from [`effective_length`][Self::effective_length].
    pub fn within_length(self, a: f64, b: f64, scale: f64) -> Result<bool, ToleranceError> {
        if !a.is_finite() || !b.is_finite() {
            return Err(ToleranceError::NonFinite);
        }
        let tol = self.effective_length(scale)?;
        let diff = a - b;
        if !diff.is_finite() {
            return Err(ToleranceError::Overflow);
        }
        Ok(diff.abs() <= tol)
    }

    /// Classifies `value` relative to the angular tolerance.
    ///
    /// Inputs must be finite angles in radians.
    ///
    /// # Errors
    ///
    /// Returns [`ToleranceError::NonFinite`] when `value` is not finite.
    pub fn classify_angle(self, value: f64) -> Result<Classification, ToleranceError> {
        if !value.is_finite() {
            return Err(ToleranceError::NonFinite);
        }
        let tol = self.angular();
        if value > tol {
            Ok(Classification::AboveTolerance)
        } else if value < -tol {
            Ok(Classification::BelowTolerance)
        } else {
            Ok(Classification::WithinTolerance)
        }
    }

    /// Returns `true` when `|a - b|` is within the angular tolerance.
    ///
    /// # Errors
    ///
    /// Returns [`ToleranceError::NonFinite`] when either angle is not finite.
    pub fn within_angle(self, a: f64, b: f64) -> Result<bool, ToleranceError> {
        if !a.is_finite() || !b.is_finite() {
            return Err(ToleranceError::NonFinite);
        }
        let diff = a - b;
        if !diff.is_finite() {
            return Err(ToleranceError::Overflow);
        }
        Ok(diff.abs() <= self.angular())
    }

    /// Classifies `value` relative to the parametric tolerance.
    ///
    /// # Errors
    ///
    /// Returns [`ToleranceError::NonFinite`] when `value` is not finite.
    pub fn classify_parametric(self, value: f64) -> Result<Classification, ToleranceError> {
        if !value.is_finite() {
            return Err(ToleranceError::NonFinite);
        }
        let tol = self.parametric();
        if value > tol {
            Ok(Classification::AboveTolerance)
        } else if value < -tol {
            Ok(Classification::BelowTolerance)
        } else {
            Ok(Classification::WithinTolerance)
        }
    }
}

impl TryFrom<ToleranceRepr> for ToleranceContext {
    type Error = ToleranceError;

    fn try_from(value: ToleranceRepr) -> Result<Self, Self::Error> {
        let values = [
            value.absolute_length,
            value.relative_length,
            value.angular,
            value.parametric,
        ];
        if values.iter().any(|component| !component.is_finite()) {
            return Err(ToleranceError::NonFinite);
        }
        if value.absolute_length <= 0.0 || value.angular <= 0.0 || value.parametric <= 0.0 {
            return Err(ToleranceError::NotPositive);
        }
        if value.relative_length < 0.0 {
            return Err(ToleranceError::NegativeRelative);
        }
        Ok(Self(value))
    }
}

impl From<ToleranceContext> for ToleranceRepr {
    fn from(value: ToleranceContext) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::{Classification, LengthTolerance, ToleranceContext, ToleranceError};

    fn ctx() -> ToleranceContext {
        ToleranceContext::try_new(1.0e-9, 1.0e-8, 1.0e-7, 1.0e-10).unwrap()
    }

    #[test]
    fn context_is_explicit_and_scale_aware() {
        let context = match ToleranceContext::try_new(1.0e-9, 1.0e-8, 1.0e-10, 1.0e-12) {
            Ok(context) => context,
            Err(error) => panic!("unexpected tolerance error: {error}"),
        };
        assert_eq!(context.effective_length(1.0), Ok(1.0e-8));
        assert_eq!(context.effective_length(0.001), Ok(1.0e-9));
    }

    #[test]
    fn invalid_contexts_and_scales_are_rejected() {
        assert!(ToleranceContext::try_new(0.0, 0.0, 1.0, 1.0).is_err());
        assert!(ToleranceContext::try_new(1.0, -1.0, 1.0, 1.0).is_err());
        let context = match ToleranceContext::try_new(1.0, 0.0, 1.0, 1.0) {
            Ok(context) => context,
            Err(error) => panic!("unexpected tolerance error: {error}"),
        };
        assert!(context.effective_length(f64::NAN).is_err());
    }

    #[test]
    fn entity_tolerances_are_positive_and_finite() {
        assert!(LengthTolerance::try_new(1.0e-9).is_ok());
        assert!(LengthTolerance::try_new(0.0).is_err());
        assert!(LengthTolerance::try_new(f64::INFINITY).is_err());
    }

    #[test]
    fn classify_length_above_tolerance() {
        let c = ctx();
        // 1.0 >> 1e-9 absolute
        assert_eq!(
            c.classify_length(1.0, 1.0).unwrap(),
            Classification::AboveTolerance
        );
    }

    #[test]
    fn classify_length_below_tolerance() {
        let c = ctx();
        assert_eq!(
            c.classify_length(-1.0, 1.0).unwrap(),
            Classification::BelowTolerance
        );
    }

    #[test]
    fn classify_length_within_tolerance() {
        let c = ctx();
        // 1e-10 < 1e-9 absolute tol at scale 1.0 → within
        assert_eq!(
            c.classify_length(1.0e-10, 1.0).unwrap(),
            Classification::WithinTolerance
        );
    }

    #[test]
    fn classify_length_scale_aware() {
        // At scale 1000, effective = max(1e-9, 1e-8 * 1000) = 1e-5
        let c = ctx();
        // 1e-6 < 1e-5 → within
        assert_eq!(
            c.classify_length(1.0e-6, 1000.0).unwrap(),
            Classification::WithinTolerance
        );
        // 1e-4 > 1e-5 → above
        assert_eq!(
            c.classify_length(1.0e-4, 1000.0).unwrap(),
            Classification::AboveTolerance
        );
    }

    #[test]
    fn within_length_returns_bool() {
        let c = ctx();
        assert!(c.within_length(1.0, 1.0 + 1.0e-10, 1.0).unwrap());
        assert!(!c.within_length(1.0, 2.0, 1.0).unwrap());
    }

    #[test]
    fn classify_angle_is_not_scale_aware() {
        // Angular tolerance is fixed at 1e-7 rad.
        let c = ctx();
        assert_eq!(
            c.classify_angle(1.0e-8).unwrap(),
            Classification::WithinTolerance
        );
        assert_eq!(
            c.classify_angle(1.0e-6).unwrap(),
            Classification::AboveTolerance
        );
    }

    #[test]
    fn classify_rejects_non_finite() {
        let c = ctx();
        assert!(c.classify_length(f64::NAN, 1.0).is_err());
        assert!(c.classify_length(f64::INFINITY, 1.0).is_err());
        assert!(c.classify_angle(f64::NAN).is_err());
    }

    #[test]
    fn within_length_rejects_non_finite() {
        let c = ctx();
        assert!(c.within_length(f64::NAN, 0.0, 1.0).is_err());
        assert!(c.within_length(0.0, f64::INFINITY, 1.0).is_err());
    }

    #[test]
    fn serde_round_trip_for_context() {
        let c = ctx();
        let json = serde_json::to_string(&c).unwrap();
        let c2: ToleranceContext = serde_json::from_str(&json).unwrap();
        assert_eq!(c, c2);
    }

    #[test]
    fn serde_rejects_non_positive_absolute() {
        let bad =
            r#"{"absolute_length":0.0,"relative_length":0.0,"angular":1e-7,"parametric":1e-10}"#;
        assert!(serde_json::from_str::<ToleranceContext>(bad).is_err());
    }

    #[test]
    fn serde_rejects_negative_relative() {
        let bad =
            r#"{"absolute_length":1e-9,"relative_length":-1.0,"angular":1e-7,"parametric":1e-10}"#;
        assert!(serde_json::from_str::<ToleranceContext>(bad).is_err());
    }

    #[test]
    fn length_tolerance_serde_round_trip() {
        let lt = LengthTolerance::try_new(1.0e-6).unwrap();
        let json = serde_json::to_string(&lt).unwrap();
        let lt2: LengthTolerance = serde_json::from_str(&json).unwrap();
        assert_eq!(lt, lt2);
    }

    #[test]
    fn length_tolerance_serde_rejects_zero_and_negative() {
        assert!(serde_json::from_str::<LengthTolerance>("0.0").is_err());
        assert!(serde_json::from_str::<LengthTolerance>("-1.0").is_err());
    }

    #[test]
    fn effective_length_is_deterministic() {
        let c = ctx();
        let t1 = c.effective_length(5.0).unwrap();
        let t2 = c.effective_length(5.0).unwrap();
        // Bit-identical result for the same inputs.
        assert_eq!(
            t1.to_bits(),
            t2.to_bits(),
            "effective_length must be deterministic"
        );
    }

    #[test]
    fn effective_length_overflow_is_rejected() {
        let c = ToleranceContext::try_new(1.0e-9, 2.0, 1.0e-7, 1.0e-10).unwrap();
        assert_eq!(c.effective_length(f64::MAX), Err(ToleranceError::Overflow));
    }

    #[test]
    fn within_length_overflow_is_rejected() {
        let c = ctx();
        assert_eq!(
            c.within_length(f64::MAX, -f64::MAX, 1.0),
            Err(ToleranceError::Overflow)
        );
    }

    #[test]
    fn within_angle_overflow_is_rejected() {
        let c = ctx();
        assert_eq!(
            c.within_angle(f64::MAX, -f64::MAX),
            Err(ToleranceError::Overflow)
        );
    }
}
