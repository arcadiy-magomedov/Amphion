//! Contracts for exact analytic curves and surfaces.
//!
//! Geometry remains canonical. Tessellation is a derived representation and is
//! intentionally absent from these evaluator traits.
//!
//! # Analytic primitives
//!
//! The [`analytic`] module provides ready-to-use implementations of every
//! primitive required for the first kernel milestone: lines, circles, planes,
//! cylinders, and cones.

pub mod analytic;

mod domain;
mod error;
mod evaluation;
mod id;
mod kind;
mod traits;

pub use analytic::{Circle2, Circle3, Cone, ConstructionError, Cylinder, Line2, Line3, Plane};
pub use domain::{ParameterInterval, ParameterRange, ParameterRangeError, SurfaceDomain};
pub use error::GeometryError;
pub use evaluation::{
    CertificationBudget, CurveEvaluation2, CurveEvaluation3, CurveProjection2, CurveProjection3,
    DerivativeOrder, DistanceBound, EvaluationContext, FirstDerivativeBound, ParameterValue,
    PositionBound, ProjectionValueError, SecondDerivativeBound, SurfaceEvaluation,
    SurfaceProjection,
};
pub use id::{Curve2Id, Curve3Id, GeometryHandle, SurfaceId};
pub use kind::{CurveKind, SurfaceKind};
pub use traits::{Curve2Evaluator, Curve3Evaluator, SurfaceEvaluator};
