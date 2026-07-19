//! Finite model-space values and transform storage conventions.

use core::error::Error;
use core::fmt;

use serde::{Deserialize, Serialize};

/// Error returned when a model-space value contains NaN or infinity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct NonFiniteValue {
    component: usize,
}

impl NonFiniteValue {
    const fn new(component: usize) -> Self {
        Self { component }
    }

    /// Returns the zero-based component containing the invalid value.
    #[must_use]
    pub const fn component(self) -> usize {
        self.component
    }
}

impl fmt::Display for NonFiniteValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "model-space component {} must be finite",
            self.component
        )
    }
}

impl Error for NonFiniteValue {}

fn ensure_finite<const N: usize>(values: [f64; N]) -> Result<[f64; N], NonFiniteValue> {
    for (index, value) in values.iter().enumerate() {
        if !value.is_finite() {
            return Err(NonFiniteValue::new(index));
        }
    }
    Ok(values)
}

/// A finite point in a two-dimensional coordinate frame.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "[f64; 2]", into = "[f64; 2]")]
pub struct Point2([f64; 2]);

impl Point2 {
    /// Creates a point if both coordinates are finite.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] when either coordinate is NaN or infinite.
    pub fn try_new(x: f64, y: f64) -> Result<Self, NonFiniteValue> {
        Self::try_from([x, y])
    }

    /// Returns the X coordinate.
    #[must_use]
    pub const fn x(self) -> f64 {
        self.0[0]
    }

    /// Returns the Y coordinate.
    #[must_use]
    pub const fn y(self) -> f64 {
        self.0[1]
    }

    /// Returns coordinates in canonical order.
    #[must_use]
    pub const fn into_array(self) -> [f64; 2] {
        self.0
    }
}

impl TryFrom<[f64; 2]> for Point2 {
    type Error = NonFiniteValue;

    fn try_from(value: [f64; 2]) -> Result<Self, Self::Error> {
        ensure_finite(value).map(Self)
    }
}

impl From<Point2> for [f64; 2] {
    fn from(value: Point2) -> Self {
        value.0
    }
}

/// A finite vector in a two-dimensional coordinate frame.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "[f64; 2]", into = "[f64; 2]")]
pub struct Vector2([f64; 2]);

impl Vector2 {
    /// Creates a vector if both components are finite.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] when either component is NaN or infinite.
    pub fn try_new(x: f64, y: f64) -> Result<Self, NonFiniteValue> {
        Self::try_from([x, y])
    }

    /// Returns the X component.
    #[must_use]
    pub const fn x(self) -> f64 {
        self.0[0]
    }

    /// Returns the Y component.
    #[must_use]
    pub const fn y(self) -> f64 {
        self.0[1]
    }

    /// Returns components in canonical order.
    #[must_use]
    pub const fn into_array(self) -> [f64; 2] {
        self.0
    }
}

impl TryFrom<[f64; 2]> for Vector2 {
    type Error = NonFiniteValue;

    fn try_from(value: [f64; 2]) -> Result<Self, Self::Error> {
        ensure_finite(value).map(Self)
    }
}

impl From<Vector2> for [f64; 2] {
    fn from(value: Vector2) -> Self {
        value.0
    }
}

/// A finite point in three-dimensional model space.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "[f64; 3]", into = "[f64; 3]")]
pub struct Point3([f64; 3]);

impl Point3 {
    /// Creates a point if every coordinate is finite.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] when any coordinate is NaN or infinite.
    pub fn try_new(x: f64, y: f64, z: f64) -> Result<Self, NonFiniteValue> {
        Self::try_from([x, y, z])
    }

    /// Returns the X coordinate.
    #[must_use]
    pub const fn x(self) -> f64 {
        self.0[0]
    }

    /// Returns the Y coordinate.
    #[must_use]
    pub const fn y(self) -> f64 {
        self.0[1]
    }

    /// Returns the Z coordinate.
    #[must_use]
    pub const fn z(self) -> f64 {
        self.0[2]
    }

    /// Returns coordinates in canonical order.
    #[must_use]
    pub const fn into_array(self) -> [f64; 3] {
        self.0
    }
}

impl TryFrom<[f64; 3]> for Point3 {
    type Error = NonFiniteValue;

    fn try_from(value: [f64; 3]) -> Result<Self, Self::Error> {
        ensure_finite(value).map(Self)
    }
}

impl From<Point3> for [f64; 3] {
    fn from(value: Point3) -> Self {
        value.0
    }
}

/// A finite vector in three-dimensional model space.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "[f64; 3]", into = "[f64; 3]")]
pub struct Vector3([f64; 3]);

impl Vector3 {
    /// Creates a vector if every component is finite.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] when any component is NaN or infinite.
    pub fn try_new(x: f64, y: f64, z: f64) -> Result<Self, NonFiniteValue> {
        Self::try_from([x, y, z])
    }

    /// Returns the X component.
    #[must_use]
    pub const fn x(self) -> f64 {
        self.0[0]
    }

    /// Returns the Y component.
    #[must_use]
    pub const fn y(self) -> f64 {
        self.0[1]
    }

    /// Returns the Z component.
    #[must_use]
    pub const fn z(self) -> f64 {
        self.0[2]
    }

    /// Returns components in canonical order.
    #[must_use]
    pub const fn into_array(self) -> [f64; 3] {
        self.0
    }
}

impl TryFrom<[f64; 3]> for Vector3 {
    type Error = NonFiniteValue;

    fn try_from(value: [f64; 3]) -> Result<Self, Self::Error> {
        ensure_finite(value).map(Self)
    }
}

impl From<Vector3> for [f64; 3] {
    fn from(value: Vector3) -> Self {
        value.0
    }
}

/// A finite affine transform stored as a row-major 3 by 4 matrix.
///
/// Points are column vectors. `a.compose(b)` will mean "apply `b`, then apply
/// `a`" when composition is implemented by the numerics layer.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "[f64; 12]", into = "[f64; 12]")]
pub struct Transform3([f64; 12]);

impl Transform3 {
    /// The identity transform.
    pub const IDENTITY: Self = Self([1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0]);

    /// Creates a transform from a finite row-major 3 by 4 affine matrix.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] when any matrix component is NaN or
    /// infinite.
    pub fn try_from_row_major(values: [f64; 12]) -> Result<Self, NonFiniteValue> {
        Self::try_from(values)
    }

    /// Returns the row-major 3 by 4 affine matrix.
    #[must_use]
    pub const fn into_row_major(self) -> [f64; 12] {
        self.0
    }
}

impl TryFrom<[f64; 12]> for Transform3 {
    type Error = NonFiniteValue;

    fn try_from(value: [f64; 12]) -> Result<Self, Self::Error> {
        ensure_finite(value).map(Self)
    }
}

impl From<Transform3> for [f64; 12] {
    fn from(value: Transform3) -> Self {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::{Point3, Transform3};

    #[test]
    fn finite_values_round_trip_through_json() {
        let point = match Point3::try_new(1.0, 2.0, 3.0) {
            Ok(point) => point,
            Err(error) => panic!("unexpected point error: {error}"),
        };
        let json = match serde_json::to_string(&point) {
            Ok(json) => json,
            Err(error) => panic!("unexpected serialization error: {error}"),
        };
        let decoded: Point3 = match serde_json::from_str(&json) {
            Ok(decoded) => decoded,
            Err(error) => panic!("unexpected deserialization error: {error}"),
        };
        assert_eq!(point, decoded);
    }

    #[test]
    fn non_finite_values_are_rejected() {
        assert!(Point3::try_new(f64::NAN, 0.0, 0.0).is_err());
        assert!(Transform3::try_from_row_major([f64::INFINITY; 12]).is_err());
    }
}
