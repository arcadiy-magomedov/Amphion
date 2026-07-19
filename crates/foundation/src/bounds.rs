//! Validated finite axis-aligned bounding boxes.
//!
//! [`Bounds2`] and [`Bounds3`] represent axis-aligned bounding boxes (AABBs)
//! with finite, non-inverted coordinate bounds. Construction is checked;
//! arithmetic operations that can overflow or invert bounds return `Result`.

use core::error::Error;
use core::fmt;

use serde::{Deserialize, Serialize};

use crate::math::{Point2, Point3, Xy, Xyz};

#[derive(Clone, Copy, Serialize, Deserialize)]
struct Bounds2Repr {
    min: Xy,
    max: Xy,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
struct Bounds3Repr {
    min: Xyz,
    max: Xyz,
}

/// Error from invalid bounds construction or operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundsError {
    /// A coordinate is NaN or infinite.
    NonFinite,
    /// The minimum exceeds the maximum in at least one dimension.
    InvertedBounds,
    /// An expansion or arithmetic operation overflowed.
    Overflow,
}

impl fmt::Display for BoundsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NonFinite => "bounds coordinates must be finite",
            Self::InvertedBounds => "bounds minimum must not exceed maximum in any dimension",
            Self::Overflow => "bounds operation overflowed",
        };
        formatter.write_str(message)
    }
}

impl Error for BoundsError {}

/// A finite axis-aligned bounding box in 2-D.
///
/// Invariant: all coordinates are finite and `min.x ≤ max.x`, `min.y ≤ max.y`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Bounds2Repr", into = "Bounds2Repr")]
pub struct Bounds2 {
    min: Point2,
    max: Point2,
}

impl Bounds2 {
    /// Constructs `[min, max]`.
    ///
    /// # Errors
    ///
    /// Returns [`BoundsError::InvertedBounds`] when `min.x > max.x` or
    /// `min.y > max.y`.
    pub fn try_new(min: Point2, max: Point2) -> Result<Self, BoundsError> {
        if min.x() > max.x() || min.y() > max.y() {
            return Err(BoundsError::InvertedBounds);
        }
        Ok(Self { min, max })
    }

    /// Constructs a degenerate (point) bounding box `[p, p]`.
    #[must_use]
    pub fn from_point(point: Point2) -> Self {
        Self {
            min: point,
            max: point,
        }
    }

    /// Returns the minimum corner.
    #[must_use]
    pub fn min(self) -> Point2 {
        self.min
    }

    /// Returns the maximum corner.
    #[must_use]
    pub fn max(self) -> Point2 {
        self.max
    }

    /// Returns `true` when `point` lies within (inclusive) the bounding box.
    #[must_use]
    pub fn contains(self, point: Point2) -> bool {
        point.x() >= self.min.x()
            && point.x() <= self.max.x()
            && point.y() >= self.min.y()
            && point.y() <= self.max.y()
    }

    /// Returns the smallest bounding box that contains both `self` and `other`.
    ///
    /// Union is always valid and infallible.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self {
            min: Point2::from_finite_unchecked([
                self.min.x().min(other.min.x()),
                self.min.y().min(other.min.y()),
            ]),
            max: Point2::from_finite_unchecked([
                self.max.x().max(other.max.x()),
                self.max.y().max(other.max.y()),
            ]),
        }
    }

    /// Returns the intersection, or `None` when the boxes do not overlap.
    #[must_use]
    pub fn intersection(self, other: Self) -> Option<Self> {
        let min_x = self.min.x().max(other.min.x());
        let min_y = self.min.y().max(other.min.y());
        let max_x = self.max.x().min(other.max.x());
        let max_y = self.max.y().min(other.max.y());
        if min_x <= max_x && min_y <= max_y {
            Some(Self {
                min: Point2::from_finite_unchecked([min_x, min_y]),
                max: Point2::from_finite_unchecked([max_x, max_y]),
            })
        } else {
            None
        }
    }

    /// Returns the bounding box expanded outward by `delta` on every side.
    ///
    /// A negative `delta` contracts the box; the result must remain
    /// non-inverted.
    ///
    /// # Errors
    ///
    /// Returns [`BoundsError::NonFinite`] when `delta` is NaN or infinite.
    /// Returns [`BoundsError::Overflow`] when any expanded coordinate
    /// overflows.
    /// Returns [`BoundsError::InvertedBounds`] when a negative `delta`
    /// inverts the box.
    pub fn try_expand(self, delta: f64) -> Result<Self, BoundsError> {
        if !delta.is_finite() {
            return Err(BoundsError::NonFinite);
        }
        let min_x = self.min.x() - delta;
        let min_y = self.min.y() - delta;
        let max_x = self.max.x() + delta;
        let max_y = self.max.y() + delta;
        if !min_x.is_finite() || !min_y.is_finite() || !max_x.is_finite() || !max_y.is_finite() {
            return Err(BoundsError::Overflow);
        }
        if min_x > max_x || min_y > max_y {
            return Err(BoundsError::InvertedBounds);
        }
        Ok(Self {
            min: Point2::from_finite_unchecked([min_x, min_y]),
            max: Point2::from_finite_unchecked([max_x, max_y]),
        })
    }
}

impl TryFrom<Bounds2Repr> for Bounds2 {
    type Error = BoundsError;

    fn try_from(value: Bounds2Repr) -> Result<Self, Self::Error> {
        let min =
            Point2::try_from([value.min.x, value.min.y]).map_err(|_| BoundsError::NonFinite)?;
        let max =
            Point2::try_from([value.max.x, value.max.y]).map_err(|_| BoundsError::NonFinite)?;
        Self::try_new(min, max)
    }
}

impl From<Bounds2> for Bounds2Repr {
    fn from(value: Bounds2) -> Self {
        Self {
            min: Xy {
                x: value.min.x(),
                y: value.min.y(),
            },
            max: Xy {
                x: value.max.x(),
                y: value.max.y(),
            },
        }
    }
}

/// A finite axis-aligned bounding box in 3-D model space.
///
/// Invariant: all coordinates are finite and each `min` component ≤ the
/// corresponding `max` component.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Bounds3Repr", into = "Bounds3Repr")]
pub struct Bounds3 {
    min: Point3,
    max: Point3,
}

impl Bounds3 {
    /// Constructs `[min, max]`.
    ///
    /// # Errors
    ///
    /// Returns [`BoundsError::InvertedBounds`] when any minimum component
    /// exceeds its corresponding maximum component.
    pub fn try_new(min: Point3, max: Point3) -> Result<Self, BoundsError> {
        if min.x() > max.x() || min.y() > max.y() || min.z() > max.z() {
            return Err(BoundsError::InvertedBounds);
        }
        Ok(Self { min, max })
    }

    /// Constructs a degenerate (point) bounding box `[p, p]`.
    #[must_use]
    pub fn from_point(point: Point3) -> Self {
        Self {
            min: point,
            max: point,
        }
    }

    /// Returns the minimum corner.
    #[must_use]
    pub fn min(self) -> Point3 {
        self.min
    }

    /// Returns the maximum corner.
    #[must_use]
    pub fn max(self) -> Point3 {
        self.max
    }

    /// Returns `true` when `point` lies within (inclusive) the bounding box.
    #[must_use]
    pub fn contains(self, point: Point3) -> bool {
        point.x() >= self.min.x()
            && point.x() <= self.max.x()
            && point.y() >= self.min.y()
            && point.y() <= self.max.y()
            && point.z() >= self.min.z()
            && point.z() <= self.max.z()
    }

    /// Returns the smallest bounding box that contains both `self` and `other`.
    #[must_use]
    pub fn union(self, other: Self) -> Self {
        Self {
            min: Point3::from_finite_unchecked([
                self.min.x().min(other.min.x()),
                self.min.y().min(other.min.y()),
                self.min.z().min(other.min.z()),
            ]),
            max: Point3::from_finite_unchecked([
                self.max.x().max(other.max.x()),
                self.max.y().max(other.max.y()),
                self.max.z().max(other.max.z()),
            ]),
        }
    }

    /// Returns the intersection, or `None` when the boxes do not overlap.
    #[must_use]
    pub fn intersection(self, other: Self) -> Option<Self> {
        let min_x = self.min.x().max(other.min.x());
        let min_y = self.min.y().max(other.min.y());
        let min_z = self.min.z().max(other.min.z());
        let max_x = self.max.x().min(other.max.x());
        let max_y = self.max.y().min(other.max.y());
        let max_z = self.max.z().min(other.max.z());
        if min_x <= max_x && min_y <= max_y && min_z <= max_z {
            Some(Self {
                min: Point3::from_finite_unchecked([min_x, min_y, min_z]),
                max: Point3::from_finite_unchecked([max_x, max_y, max_z]),
            })
        } else {
            None
        }
    }

    /// Returns the bounding box expanded outward by `delta` on every side.
    ///
    /// # Errors
    ///
    /// Returns [`BoundsError::NonFinite`] when `delta` is NaN or infinite.
    /// Returns [`BoundsError::Overflow`] when any expanded coordinate
    /// overflows.
    /// Returns [`BoundsError::InvertedBounds`] when a negative `delta`
    /// inverts the box.
    pub fn try_expand(self, delta: f64) -> Result<Self, BoundsError> {
        if !delta.is_finite() {
            return Err(BoundsError::NonFinite);
        }
        let min_x = self.min.x() - delta;
        let min_y = self.min.y() - delta;
        let min_z = self.min.z() - delta;
        let max_x = self.max.x() + delta;
        let max_y = self.max.y() + delta;
        let max_z = self.max.z() + delta;
        if !min_x.is_finite()
            || !min_y.is_finite()
            || !min_z.is_finite()
            || !max_x.is_finite()
            || !max_y.is_finite()
            || !max_z.is_finite()
        {
            return Err(BoundsError::Overflow);
        }
        if min_x > max_x || min_y > max_y || min_z > max_z {
            return Err(BoundsError::InvertedBounds);
        }
        Ok(Self {
            min: Point3::from_finite_unchecked([min_x, min_y, min_z]),
            max: Point3::from_finite_unchecked([max_x, max_y, max_z]),
        })
    }
}

impl TryFrom<Bounds3Repr> for Bounds3 {
    type Error = BoundsError;

    fn try_from(value: Bounds3Repr) -> Result<Self, Self::Error> {
        let min = Point3::try_from([value.min.x, value.min.y, value.min.z])
            .map_err(|_| BoundsError::NonFinite)?;
        let max = Point3::try_from([value.max.x, value.max.y, value.max.z])
            .map_err(|_| BoundsError::NonFinite)?;
        Self::try_new(min, max)
    }
}

impl From<Bounds3> for Bounds3Repr {
    fn from(value: Bounds3) -> Self {
        Self {
            min: Xyz {
                x: value.min.x(),
                y: value.min.y(),
                z: value.min.z(),
            },
            max: Xyz {
                x: value.max.x(),
                y: value.max.y(),
                z: value.max.z(),
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Bounds2, Bounds3, BoundsError};
    use crate::math::{Point2, Point3};

    fn p2(x: f64, y: f64) -> Point2 {
        Point2::try_new(x, y).unwrap()
    }

    fn p3(x: f64, y: f64, z: f64) -> Point3 {
        Point3::try_new(x, y, z).unwrap()
    }

    #[test]
    fn bounds2_construction_valid() {
        let bounds = Bounds2::try_new(p2(1.0, 2.0), p2(3.0, 4.0)).unwrap();
        assert_eq!(bounds.min(), p2(1.0, 2.0));
        assert_eq!(bounds.max(), p2(3.0, 4.0));
    }

    #[test]
    fn bounds2_from_point_is_degenerate() {
        let point = p2(2.0, 5.0);
        let bounds = Bounds2::from_point(point);
        assert_eq!(bounds.min(), point);
        assert_eq!(bounds.max(), point);
    }

    #[test]
    fn bounds2_construction_rejects_inverted() {
        assert_eq!(
            Bounds2::try_new(p2(2.0, 0.0), p2(1.0, 1.0)),
            Err(BoundsError::InvertedBounds)
        );
    }

    #[test]
    fn bounds2_contains_corners_inclusive() {
        let bounds = Bounds2::try_new(p2(1.0, 2.0), p2(3.0, 4.0)).unwrap();
        assert!(bounds.contains(p2(1.0, 2.0)));
        assert!(bounds.contains(p2(3.0, 4.0)));
    }

    #[test]
    fn bounds2_does_not_contain_exterior() {
        let bounds = Bounds2::try_new(p2(1.0, 2.0), p2(3.0, 4.0)).unwrap();
        assert!(!bounds.contains(p2(3.1, 4.0)));
    }

    #[test]
    fn bounds2_union_spans_both() {
        let left = Bounds2::try_new(p2(0.0, 0.0), p2(1.0, 1.0)).unwrap();
        let right = Bounds2::try_new(p2(2.0, 3.0), p2(4.0, 5.0)).unwrap();
        let union = left.union(right);
        assert_eq!(union.min(), p2(0.0, 0.0));
        assert_eq!(union.max(), p2(4.0, 5.0));
    }

    #[test]
    fn bounds2_intersection_overlapping() {
        let a = Bounds2::try_new(p2(0.0, 0.0), p2(4.0, 4.0)).unwrap();
        let b = Bounds2::try_new(p2(2.0, 1.0), p2(5.0, 3.0)).unwrap();
        let intersection = a.intersection(b).unwrap();
        assert_eq!(intersection.min(), p2(2.0, 1.0));
        assert_eq!(intersection.max(), p2(4.0, 3.0));
    }

    #[test]
    fn bounds2_intersection_disjoint_is_none() {
        let a = Bounds2::try_new(p2(0.0, 0.0), p2(1.0, 1.0)).unwrap();
        let b = Bounds2::try_new(p2(2.0, 2.0), p2(3.0, 3.0)).unwrap();
        assert_eq!(a.intersection(b), None);
    }

    #[test]
    fn bounds2_expand_grows_outward() {
        let bounds = Bounds2::try_new(p2(1.0, 2.0), p2(3.0, 4.0)).unwrap();
        let expanded = bounds.try_expand(0.5).unwrap();
        assert_eq!(expanded.min(), p2(0.5, 1.5));
        assert_eq!(expanded.max(), p2(3.5, 4.5));
    }

    #[test]
    fn bounds2_expand_nan_rejected() {
        let bounds = Bounds2::try_new(p2(1.0, 2.0), p2(3.0, 4.0)).unwrap();
        assert_eq!(bounds.try_expand(f64::NAN), Err(BoundsError::NonFinite));
    }

    #[test]
    fn bounds2_expand_overflow_rejected() {
        let bounds = Bounds2::try_new(p2(f64::MAX, f64::MAX), p2(f64::MAX, f64::MAX)).unwrap();
        assert_eq!(bounds.try_expand(f64::MAX), Err(BoundsError::Overflow));
    }

    #[test]
    fn bounds2_serde_round_trip() {
        let bounds = Bounds2::try_new(p2(1.0, 2.0), p2(3.0, 4.0)).unwrap();
        let json = serde_json::to_string(&bounds).unwrap();
        let decoded: Bounds2 = serde_json::from_str(&json).unwrap();
        assert_eq!(bounds, decoded);
    }

    #[test]
    fn bounds2_serde_rejects_inverted() {
        let bad: Result<Bounds2, _> =
            serde_json::from_str(r#"{"min":{"x":2.0,"y":0.0},"max":{"x":1.0,"y":1.0}}"#);
        assert!(bad.is_err(), "inverted bounds must be rejected");
    }

    #[test]
    fn bounds2_serde_rejects_non_finite() {
        let bad: Result<Bounds2, _> =
            serde_json::from_str(r#"{"min":{"x":0.0,"y":0.0},"max":{"x":1e400,"y":1.0}}"#);
        assert!(bad.is_err(), "non-finite bounds must be rejected");
    }

    #[test]
    fn bounds2_serde_json_shape_is_named() {
        let bounds = Bounds2::try_new(p2(1.0, 2.0), p2(3.0, 4.0)).unwrap();
        let json = serde_json::to_string(&bounds).unwrap();
        assert!(json.contains("\"min\"") && json.contains("\"max\"") && json.contains("\"x\""));
    }

    #[test]
    fn bounds3_construction_valid() {
        let bounds = Bounds3::try_new(p3(0.0, 1.0, 2.0), p3(3.0, 4.0, 5.0)).unwrap();
        assert_eq!(bounds.min(), p3(0.0, 1.0, 2.0));
        assert_eq!(bounds.max(), p3(3.0, 4.0, 5.0));
    }

    #[test]
    fn bounds3_contains_point() {
        let bounds = Bounds3::try_new(p3(0.0, 0.0, 0.0), p3(3.0, 4.0, 5.0)).unwrap();
        assert!(bounds.contains(p3(2.0, 3.0, 4.0)));
    }

    #[test]
    fn bounds3_union_expands() {
        let a = Bounds3::try_new(p3(0.0, 0.0, 0.0), p3(1.0, 1.0, 1.0)).unwrap();
        let b = Bounds3::try_new(p3(-1.0, 2.0, 3.0), p3(4.0, 5.0, 6.0)).unwrap();
        let union = a.union(b);
        assert_eq!(union.min(), p3(-1.0, 0.0, 0.0));
        assert_eq!(union.max(), p3(4.0, 5.0, 6.0));
    }

    #[test]
    fn bounds3_intersection_touching_edge() {
        let a = Bounds3::try_new(p3(0.0, 0.0, 0.0), p3(1.0, 1.0, 1.0)).unwrap();
        let b = Bounds3::try_new(p3(1.0, 0.25, 0.25), p3(2.0, 0.75, 0.75)).unwrap();
        let intersection = a.intersection(b).unwrap();
        assert_eq!(intersection.min(), p3(1.0, 0.25, 0.25));
        assert_eq!(intersection.max(), p3(1.0, 0.75, 0.75));
    }

    #[test]
    fn bounds3_expand_negative_delta_contracts() {
        let bounds = Bounds3::try_new(p3(0.0, 0.0, 0.0), p3(4.0, 4.0, 4.0)).unwrap();
        let contracted = bounds.try_expand(-1.0).unwrap();
        assert_eq!(contracted.min(), p3(1.0, 1.0, 1.0));
        assert_eq!(contracted.max(), p3(3.0, 3.0, 3.0));
    }

    #[test]
    fn bounds3_expand_over_contracts_returns_inverted() {
        let bounds = Bounds3::try_new(p3(0.0, 0.0, 0.0), p3(1.0, 1.0, 1.0)).unwrap();
        assert_eq!(bounds.try_expand(-1.0), Err(BoundsError::InvertedBounds));
    }

    #[test]
    fn bounds3_serde_round_trip() {
        let bounds = Bounds3::try_new(p3(0.0, 1.0, 2.0), p3(3.0, 4.0, 5.0)).unwrap();
        let json = serde_json::to_string(&bounds).unwrap();
        let decoded: Bounds3 = serde_json::from_str(&json).unwrap();
        assert_eq!(bounds, decoded);
    }

    #[test]
    fn bounds3_serde_rejects_non_finite() {
        let bad: Result<Bounds3, _> = serde_json::from_str(
            r#"{"min":{"x":0.0,"y":0.0,"z":0.0},"max":{"x":1e400,"y":1.0,"z":1.0}}"#,
        );
        assert!(bad.is_err());
    }

    #[test]
    fn bounds2_deterministic() {
        let bounds = Bounds2::try_new(p2(0.0, 0.0), p2(1.0, 1.0)).unwrap();
        let other = Bounds2::try_new(p2(2.0, 3.0), p2(4.0, 5.0)).unwrap();
        let first = bounds.union(other);
        let second = bounds.union(other);
        assert_eq!(first, second);
    }
}
