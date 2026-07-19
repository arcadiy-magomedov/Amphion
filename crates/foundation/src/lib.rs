//! Mathematical and numerical contracts shared by every Amphion subsystem.
//!
//! Model coordinates use metres, angles use radians, and coordinate frames are
//! right-handed. Operations that compare geometry must receive an explicit
//! [`ToleranceContext`].

mod bounds;
mod conventions;
mod diagnostic;
mod identity;
mod interval;
mod math;
mod predicates;
mod tolerance;
mod unit;
mod version;

pub use bounds::{Bounds2, Bounds3, BoundsError};
pub use conventions::{
    AngleUnit, Axis3, Handedness, LengthUnit, MODEL_ANGLE_UNIT, MODEL_HANDEDNESS, MODEL_LENGTH_UNIT,
};
pub use diagnostic::{
    Diagnostic, DiagnosticCode, DiagnosticCodeError, DiagnosticPathSegment, Severity,
};
pub use identity::{OperationId, SemanticId};
pub use interval::{Interval, IntervalError};
pub use math::{NonFiniteValue, Point2, Point3, Transform3, Vector2, Vector3};
pub use predicates::{OrientationSign, PredicateError, orient2d, orient3d};
pub use tolerance::{Classification, LengthTolerance, ToleranceContext, ToleranceError};
pub use unit::{NormalizationError, UnitVector2, UnitVector3};
pub use version::SchemaVersion;
