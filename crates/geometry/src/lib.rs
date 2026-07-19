//! Contracts for exact analytic curves and surfaces.
//!
//! Geometry remains canonical. Tessellation is a derived representation and is
//! intentionally absent from these evaluator traits.

mod domain;
mod error;
mod evaluation;
mod id;
mod kind;
mod traits;

pub use domain::{ParameterInterval, ParameterRange, ParameterRangeError, SurfaceDomain};
pub use error::GeometryError;
pub use evaluation::{
    CurveEvaluation2, CurveEvaluation3, CurveProjection2, CurveProjection3, DerivativeOrder,
    DistanceBound, ParameterValue, ProjectionValueError, SurfaceEvaluation, SurfaceProjection,
};
pub use id::{Curve2Id, Curve3Id, GeometryHandle, SurfaceId};
pub use kind::{CurveKind, SurfaceKind};
pub use traits::{Curve2Evaluator, Curve3Evaluator, SurfaceEvaluator};
