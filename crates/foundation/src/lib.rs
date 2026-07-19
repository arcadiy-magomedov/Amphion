//! Mathematical and numerical contracts shared by every Amphion subsystem.
//!
//! Model coordinates use metres, angles use radians, and coordinate frames are
//! right-handed. Operations that compare geometry must receive an explicit
//! [`ToleranceContext`].

mod conventions;
mod diagnostic;
mod identity;
mod math;
mod tolerance;
mod version;

pub use conventions::{
    AngleUnit, Axis3, Handedness, LengthUnit, MODEL_ANGLE_UNIT, MODEL_HANDEDNESS, MODEL_LENGTH_UNIT,
};
pub use diagnostic::{
    Diagnostic, DiagnosticCode, DiagnosticCodeError, DiagnosticPathSegment, Severity,
};
pub use identity::{OperationId, SemanticId};
pub use math::{NonFiniteValue, Point2, Point3, Transform3, Vector2, Vector3};
pub use tolerance::{LengthTolerance, ToleranceContext, ToleranceError};
pub use version::SchemaVersion;
