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
        };
        formatter.write_str(message)
    }
}

impl Error for ToleranceError {}

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
        Ok(self.absolute_length().max(self.relative_length() * scale))
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
    use super::{LengthTolerance, ToleranceContext};

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
}
