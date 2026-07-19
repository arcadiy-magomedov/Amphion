//! Explicit curve and surface parameter domains.

use core::error::Error;
use core::fmt;

use serde::{Deserialize, Serialize};

/// Error returned for an invalid parameter range.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ParameterRangeError;

impl fmt::Display for ParameterRangeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(
            "parameter bounds must be finite and increasing, and periods must be positive",
        )
    }
}

impl Error for ParameterRangeError {}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct ParameterRangeRepr {
    lower: Option<f64>,
    upper: Option<f64>,
    period: Option<f64>,
}

/// A one-dimensional parameter range with optional infinite bounds and period.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ParameterRangeRepr", into = "ParameterRangeRepr")]
pub struct ParameterRange {
    lower: Option<f64>,
    upper: Option<f64>,
    period: Option<f64>,
}

impl ParameterRange {
    /// Creates a parameter range.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterRangeError`] for non-finite bounds, unordered finite
    /// bounds, or a non-positive/non-finite period.
    pub fn try_new(
        lower: Option<f64>,
        upper: Option<f64>,
        period: Option<f64>,
    ) -> Result<Self, ParameterRangeError> {
        let finite_span = match (lower, upper) {
            (Some(lower), Some(upper)) => Some(upper - lower),
            _ => None,
        };
        if lower.is_some_and(|value| !value.is_finite())
            || upper.is_some_and(|value| !value.is_finite())
            || period.is_some_and(|value| !value.is_finite() || value <= 0.0)
            || period.is_some() && (lower.is_none() || upper.is_none())
            || matches!((lower, upper), (Some(lower), Some(upper)) if lower >= upper)
            || matches!((finite_span, period), (Some(span), Some(period)) if span > period)
        {
            return Err(ParameterRangeError);
        }
        Ok(Self {
            lower,
            upper,
            period,
        })
    }

    /// Returns the inclusive lower bound, or no bound.
    #[must_use]
    pub const fn lower(self) -> Option<f64> {
        self.lower
    }

    /// Returns the inclusive upper bound, or no bound.
    #[must_use]
    pub const fn upper(self) -> Option<f64> {
        self.upper
    }

    /// Returns the fundamental period for periodic geometry.
    #[must_use]
    pub const fn period(self) -> Option<f64> {
        self.period
    }
}

impl TryFrom<ParameterRangeRepr> for ParameterRange {
    type Error = ParameterRangeError;

    fn try_from(value: ParameterRangeRepr) -> Result<Self, Self::Error> {
        Self::try_new(value.lower, value.upper, value.period)
    }
}

impl From<ParameterRange> for ParameterRangeRepr {
    fn from(value: ParameterRange) -> Self {
        Self {
            lower: value.lower,
            upper: value.upper,
            period: value.period,
        }
    }
}

/// A finite directed trimming interval on a canonical curve.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "[f64; 2]", into = "[f64; 2]")]
pub struct ParameterInterval {
    start: f64,
    end: f64,
}

impl ParameterInterval {
    /// Creates a finite, increasing trimming interval.
    ///
    /// For a closed edge on a periodic curve, start and end vertices may be
    /// identical while `end - start` equals one period.
    ///
    /// # Errors
    ///
    /// Returns [`ParameterRangeError`] when either endpoint is non-finite or
    /// `start >= end`.
    pub fn try_new(start: f64, end: f64) -> Result<Self, ParameterRangeError> {
        Self::try_from([start, end])
    }

    /// Returns the parameter mapped to the edge's start vertex.
    #[must_use]
    pub const fn start(self) -> f64 {
        self.start
    }

    /// Returns the parameter mapped to the edge's end vertex.
    #[must_use]
    pub const fn end(self) -> f64 {
        self.end
    }
}

impl TryFrom<[f64; 2]> for ParameterInterval {
    type Error = ParameterRangeError;

    fn try_from(value: [f64; 2]) -> Result<Self, Self::Error> {
        if !value[0].is_finite() || !value[1].is_finite() || value[0] >= value[1] {
            return Err(ParameterRangeError);
        }
        Ok(Self {
            start: value[0],
            end: value[1],
        })
    }
}

impl From<ParameterInterval> for [f64; 2] {
    fn from(value: ParameterInterval) -> Self {
        [value.start, value.end]
    }
}

/// The rectangular parameter domain of a surface.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct SurfaceDomain {
    u: ParameterRange,
    v: ParameterRange,
}

impl SurfaceDomain {
    /// Creates a surface domain from independent U and V ranges.
    #[must_use]
    pub const fn new(u: ParameterRange, v: ParameterRange) -> Self {
        Self { u, v }
    }

    /// Returns the U range.
    #[must_use]
    pub const fn u(self) -> ParameterRange {
        self.u
    }

    /// Returns the V range.
    #[must_use]
    pub const fn v(self) -> ParameterRange {
        self.v
    }
}
