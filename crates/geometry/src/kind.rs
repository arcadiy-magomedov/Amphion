//! Stable geometry classifications.

use serde::{Deserialize, Serialize};

/// Canonical curve family.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum CurveKind {
    /// A straight line.
    Line,
    /// A circular curve.
    Circle,
    /// An elliptical curve.
    Ellipse,
    /// A polynomial Bezier curve.
    Bezier,
    /// A B-spline curve.
    BSpline,
    /// A rational B-spline curve.
    Nurbs,
    /// A curve defined by a certified intersection or other procedure.
    Procedural,
}

/// Canonical surface family.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum SurfaceKind {
    /// A planar surface.
    Plane,
    /// A cylindrical surface.
    Cylinder,
    /// A conical surface.
    Cone,
    /// A spherical surface.
    Sphere,
    /// A toroidal surface.
    Torus,
    /// A polynomial Bezier surface.
    Bezier,
    /// A B-spline surface.
    BSpline,
    /// A rational B-spline surface.
    Nurbs,
    /// A surface defined procedurally with certified evaluation bounds.
    Procedural,
}
