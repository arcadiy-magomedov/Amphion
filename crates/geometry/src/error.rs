//! Structured analytic geometry failures.

use core::error::Error;
use core::fmt;

use serde::{Deserialize, Serialize};

/// A recoverable geometry evaluation or projection failure.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum GeometryError {
    /// An input parameter was NaN or infinite.
    NonFiniteParameter,
    /// A parameter was outside the declared domain.
    OutsideDomain,
    /// The requested derivative or inverse mapping is singular.
    Singular,
    /// The operation is not supported by this geometry family.
    Unsupported {
        /// Stable operation identifier.
        operation: String,
    },
    /// A numerical method could not certify a result.
    Uncertified {
        /// Stable reason suitable for diagnostics and tests.
        reason: String,
    },
}

impl fmt::Display for GeometryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteParameter => formatter.write_str("geometry parameter must be finite"),
            Self::OutsideDomain => formatter.write_str("geometry parameter is outside its domain"),
            Self::Singular => formatter.write_str("geometry evaluation is singular"),
            Self::Unsupported { operation } => {
                write!(formatter, "geometry operation is unsupported: {operation}")
            }
            Self::Uncertified { reason } => {
                write!(
                    formatter,
                    "geometry result could not be certified: {reason}"
                )
            }
        }
    }
}

impl Error for GeometryError {}
