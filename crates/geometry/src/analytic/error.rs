//! Failures returned by analytic geometry constructors.

use core::error::Error;
use core::fmt;

use amphion_foundation::SchemaVersion;

/// A failure returned when constructing an analytic curve or surface.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ConstructionError {
    /// An input coordinate or parameter contained NaN or infinity.
    NonFiniteInput,
    /// A direction, axis, or normal vector has zero length and cannot be
    /// normalized.
    DegenerateAxis,
    /// Two provided axes are nearly dependent, making orthogonalization
    /// numerically unreliable.
    IllConditionedAxes,
    /// A scalar parameter that must be strictly positive (radius, etc.) was
    /// zero or negative.
    NotPositive,
    /// A cone half-angle was outside the open interval `(0, π/2)`.
    InvalidHalfAngle,
    /// Two provided axes are linearly dependent and cannot form an independent
    /// frame.
    DependentAxes,
    /// A serialized representation carried a [`SchemaVersion`] that does not
    /// exactly match the version supported by this build. Deserialization is
    /// rejected rather than silently reinterpreting incompatible data.
    UnsupportedSchemaVersion {
        /// The version found in the serialized representation.
        found: SchemaVersion,
        /// The exact version supported by this build.
        supported: SchemaVersion,
    },
}

impl fmt::Display for ConstructionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NonFiniteInput => "analytic geometry input contains NaN or infinity",
            Self::DegenerateAxis => {
                "direction, axis, or normal vector has zero length and cannot be normalized"
            }
            Self::IllConditionedAxes => {
                "provided axes are too close to dependent for stable orthogonalization"
            }
            Self::NotPositive => {
                "scalar geometry parameter (e.g. radius) must be strictly positive"
            }
            Self::InvalidHalfAngle => "cone half-angle must be strictly between 0 and π/2",
            Self::DependentAxes => {
                "two provided axes are linearly dependent and cannot form an independent frame"
            }
            Self::UnsupportedSchemaVersion { found, supported } => {
                return write!(
                    formatter,
                    "unsupported schema version {}.{} (this build supports exactly {}.{})",
                    found.major(),
                    found.minor(),
                    supported.major(),
                    supported.minor(),
                );
            }
        };
        formatter.write_str(message)
    }
}

impl Error for ConstructionError {}
