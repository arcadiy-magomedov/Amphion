//! Analytic curve and surface primitives.
//!
//! Every type in this module is immutable, `Send + Sync`, and free of hidden
//! global state.  Constructor functions normalize and orthogonalize direction
//! vectors; evaluated results are certified to floating-point precision.
//!
//! # Geometry families
//!
//! | Type | Family | Parameterization |
//! |------|--------|-----------------|
//! | [`Line2`] | 2-D line | `p(t) = origin + t·direction`, `t ∈ ℝ` |
//! | [`Line3`] | 3-D line | `p(t) = origin + t·direction`, `t ∈ ℝ` |
//! | [`Circle2`] | 2-D circle | `p(θ) = center + r·cos θ·x + r·sin θ·y`, `θ ∈ [0, 2π)` |
//! | [`Circle3`] | 3-D circle | `p(θ) = center + r·cos θ·x + r·sin θ·y`, `θ ∈ [0, 2π)` |
//! | [`Plane`] | plane | `p(u,v) = origin + u·u_axis + v·v_axis`, `u,v ∈ ℝ` |
//! | [`Cylinder`] | cylinder | `p(u,v) = axis_origin + v·axis + r·radial(u)`, `u ∈ [0,2π)`, `v ∈ ℝ` |
//! | [`Cone`] | cone | `p(u,v) = apex + v·axis + v·tan α·radial(u)`, `u ∈ [0,2π)`, `v ∈ ℝ` |
//!
//! # Transforms
//!
//! `Line3`, `Plane`, `Circle3`, `Cylinder`, and `Cone` support checked
//! transformation via `try_transform` (see [`TransformError`] for the error
//! set and each type's `try_transform` docs for the exact required transform
//! class). `Line2` and `Circle2` are not yet supported: the current
//! foundation has no 2-D transform type.

mod error;
pub(crate) mod exact;
mod helpers;
mod transform;
pub(crate) mod trig;

pub mod circle;
pub mod cone;
pub mod cylinder;
pub mod line;
pub mod plane;

pub use circle::{Circle2, Circle3};
pub use cone::Cone;
pub use cylinder::Cylinder;
pub use error::ConstructionError;
pub use line::{Line2, Line3};
pub use plane::Plane;
pub use transform::TransformError;
