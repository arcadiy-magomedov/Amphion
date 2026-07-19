//! Failures returned by analytic geometry constructors.

use core::error::Error;
use core::fmt;

/// A failure returned when constructing an analytic curve or surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ConstructionError {
    /// An input coordinate or parameter contained NaN or infinity.
    NonFiniteInput,
    /// A direction, axis, or normal vector has zero length and cannot be
    /// normalized.
    DegenerateAxis,
    /// A scalar parameter that must be strictly positive (radius, etc.) was
    /// zero or negative.
    NotPositive,
    /// A cone half-angle was outside the open interval `(0, π/2)`.
    InvalidHalfAngle,
    /// Two provided axes are linearly dependent and cannot form an independent
    /// frame.
    DependentAxes,
}

impl fmt::Display for ConstructionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NonFiniteInput => "analytic geometry input contains NaN or infinity",
            Self::DegenerateAxis => {
                "direction, axis, or normal vector has zero length and cannot be normalized"
            }
            Self::NotPositive => {
                "scalar geometry parameter (e.g. radius) must be strictly positive"
            }
            Self::InvalidHalfAngle => "cone half-angle must be strictly between 0 and π/2",
            Self::DependentAxes => {
                "two provided axes are linearly dependent and cannot form an independent frame"
            }
        };
        formatter.write_str(message)
    }
}

impl Error for ConstructionError {}
