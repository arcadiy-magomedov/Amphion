//! Finite model-space values and transform storage conventions.

use core::error::Error;
use core::fmt;
use core::ops;

use serde::{Deserialize, Serialize};

/// Named serialization representation for 2-D coordinates.
///
/// Satisfies the CONTRACTS.md requirement for explicit field names in
/// serialized forms.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) struct Xy {
    pub(crate) x: f64,
    pub(crate) y: f64,
}

/// Named serialization representation for 3-D coordinates.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub(crate) struct Xyz {
    pub(crate) x: f64,
    pub(crate) y: f64,
    pub(crate) z: f64,
}

/// Named serialization representation for [`Transform3`].
///
/// Each component is named `mRC` where `R ∈ {0,1,2}` is the row and
/// `C ∈ {0,1,2,3}` is the column of the first three rows of the 4×4 row-major
/// affine matrix (CONTRACTS.md).
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
struct Transform3Repr {
    m00: f64,
    m01: f64,
    m02: f64,
    m03: f64,
    m10: f64,
    m11: f64,
    m12: f64,
    m13: f64,
    m20: f64,
    m21: f64,
    m22: f64,
    m23: f64,
}

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
#[serde(try_from = "Xy", into = "Xy")]
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

    pub(crate) const fn from_finite_unchecked(values: [f64; 2]) -> Self {
        Self(values)
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

impl TryFrom<Xy> for Point2 {
    type Error = NonFiniteValue;

    fn try_from(value: Xy) -> Result<Self, Self::Error> {
        Self::try_from([value.x, value.y])
    }
}

impl From<Point2> for Xy {
    fn from(value: Point2) -> Self {
        Self {
            x: value.0[0],
            y: value.0[1],
        }
    }
}

/// A finite vector in a two-dimensional coordinate frame.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Xy", into = "Xy")]
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

    pub(crate) const fn from_finite_unchecked(values: [f64; 2]) -> Self {
        Self(values)
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

impl TryFrom<Xy> for Vector2 {
    type Error = NonFiniteValue;

    fn try_from(value: Xy) -> Result<Self, Self::Error> {
        Self::try_from([value.x, value.y])
    }
}

impl From<Vector2> for Xy {
    fn from(value: Vector2) -> Self {
        Self {
            x: value.0[0],
            y: value.0[1],
        }
    }
}

/// A finite point in three-dimensional model space.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Xyz", into = "Xyz")]
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

    pub(crate) const fn from_finite_unchecked(values: [f64; 3]) -> Self {
        Self(values)
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

impl TryFrom<Xyz> for Point3 {
    type Error = NonFiniteValue;

    fn try_from(value: Xyz) -> Result<Self, Self::Error> {
        Self::try_from([value.x, value.y, value.z])
    }
}

impl From<Point3> for Xyz {
    fn from(value: Point3) -> Self {
        Self {
            x: value.0[0],
            y: value.0[1],
            z: value.0[2],
        }
    }
}

/// A finite vector in three-dimensional model space.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Xyz", into = "Xyz")]
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

    pub(crate) const fn from_finite_unchecked(values: [f64; 3]) -> Self {
        Self(values)
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

impl TryFrom<Xyz> for Vector3 {
    type Error = NonFiniteValue;

    fn try_from(value: Xyz) -> Result<Self, Self::Error> {
        Self::try_from([value.x, value.y, value.z])
    }
}

impl From<Vector3> for Xyz {
    fn from(value: Vector3) -> Self {
        Self {
            x: value.0[0],
            y: value.0[1],
            z: value.0[2],
        }
    }
}

/// A finite affine transform stored as a row-major 3 by 4 matrix.
///
/// Points are column vectors. `a.compose(b)` will mean "apply `b`, then apply
/// `a`" when composition is implemented by the numerics layer.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Transform3Repr", into = "Transform3Repr")]
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

impl TryFrom<Transform3Repr> for Transform3 {
    type Error = NonFiniteValue;

    fn try_from(value: Transform3Repr) -> Result<Self, Self::Error> {
        Self::try_from([
            value.m00, value.m01, value.m02, value.m03, value.m10, value.m11, value.m12, value.m13,
            value.m20, value.m21, value.m22, value.m23,
        ])
    }
}

impl From<Transform3> for Transform3Repr {
    fn from(value: Transform3) -> Self {
        let matrix = value.0;
        Self {
            m00: matrix[0],
            m01: matrix[1],
            m02: matrix[2],
            m03: matrix[3],
            m10: matrix[4],
            m11: matrix[5],
            m12: matrix[6],
            m13: matrix[7],
            m20: matrix[8],
            m21: matrix[9],
            m22: matrix[10],
            m23: matrix[11],
        }
    }
}

// ── Vector2 arithmetic ────────────────────────────────────────────────────────

impl Vector2 {
    /// Returns the dot product of two vectors.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] when the products or their sum overflow to
    /// infinity or cancel to NaN.
    pub fn try_dot(self, rhs: Self) -> Result<f64, NonFiniteValue> {
        let dot = self.0[0] * rhs.0[0] + self.0[1] * rhs.0[1];
        if !dot.is_finite() {
            return Err(NonFiniteValue::new(0));
        }
        Ok(dot)
    }

    /// Returns the scalar Z-component of the 3-D cross product (self × rhs).
    ///
    /// Positive when `rhs` is counter-clockwise from `self`.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] when the products or their difference
    /// overflow to infinity or cancel to NaN.
    pub fn try_cross_z(self, rhs: Self) -> Result<f64, NonFiniteValue> {
        let cross_z = self.0[0] * rhs.0[1] - self.0[1] * rhs.0[0];
        if !cross_z.is_finite() {
            return Err(NonFiniteValue::new(0));
        }
        Ok(cross_z)
    }

    /// Returns the squared Euclidean magnitude.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] when squaring or summing the components
    /// overflows.
    pub fn try_magnitude_squared(self) -> Result<f64, NonFiniteValue> {
        self.try_dot(self)
    }

    /// Returns the Euclidean magnitude using scale-safe evaluation.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] when the true magnitude is not representable
    /// as a finite `f64`.
    pub fn try_magnitude(self) -> Result<f64, NonFiniteValue> {
        let [x, y] = self.0;
        let max_abs = x.abs().max(y.abs());
        if max_abs == 0.0 {
            return Ok(0.0);
        }
        let scaled_x = x / max_abs;
        let scaled_y = y / max_abs;
        let magnitude = max_abs * (scaled_x * scaled_x + scaled_y * scaled_y).sqrt();
        if magnitude.is_finite() {
            Ok(magnitude)
        } else {
            Err(NonFiniteValue::new(0))
        }
    }

    /// Adds `rhs` component-wise.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] if any result component overflows to NaN or
    /// infinity (possible when components are near `f64::MAX`).
    pub fn try_add(self, rhs: Self) -> Result<Self, NonFiniteValue> {
        ensure_finite([self.0[0] + rhs.0[0], self.0[1] + rhs.0[1]]).map(Self)
    }

    /// Subtracts `rhs` component-wise.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] if any result component overflows.
    pub fn try_sub(self, rhs: Self) -> Result<Self, NonFiniteValue> {
        ensure_finite([self.0[0] - rhs.0[0], self.0[1] - rhs.0[1]]).map(Self)
    }

    /// Scales every component by `scalar`.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] if `scalar` is non-finite or if any
    /// result component overflows.
    pub fn try_scale(self, scalar: f64) -> Result<Self, NonFiniteValue> {
        ensure_finite([self.0[0] * scalar, self.0[1] * scalar]).map(Self)
    }
}

/// Negation is always finite for finite inputs and is the only infallible
/// arithmetic operation provided for [`Vector2`].
impl ops::Neg for Vector2 {
    type Output = Self;

    fn neg(self) -> Self {
        Self([-self.0[0], -self.0[1]])
    }
}

// ── Point2 arithmetic ─────────────────────────────────────────────────────────

impl Point2 {
    /// Translates the point by adding `v`.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] if any result coordinate overflows.
    pub fn try_add_vector(self, v: Vector2) -> Result<Self, NonFiniteValue> {
        ensure_finite([self.0[0] + v.0[0], self.0[1] + v.0[1]]).map(Self)
    }

    /// Translates the point by subtracting `v`.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] if any result coordinate overflows.
    pub fn try_sub_vector(self, v: Vector2) -> Result<Self, NonFiniteValue> {
        ensure_finite([self.0[0] - v.0[0], self.0[1] - v.0[1]]).map(Self)
    }

    /// Returns the displacement vector from `other` to `self` (`self − other`).
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] if any result component overflows
    /// (possible when the points are on opposite sides of the coordinate
    /// origin with magnitudes near `f64::MAX`).
    pub fn try_sub_point(self, other: Self) -> Result<Vector2, NonFiniteValue> {
        ensure_finite([self.0[0] - other.0[0], self.0[1] - other.0[1]]).map(Vector2)
    }
}

// ── Vector3 arithmetic ────────────────────────────────────────────────────────

impl Vector3 {
    /// Returns the dot product of two vectors.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] when the products or their sum overflow to
    /// infinity or cancel to NaN.
    pub fn try_dot(self, rhs: Self) -> Result<f64, NonFiniteValue> {
        let dot = self.0[0] * rhs.0[0] + self.0[1] * rhs.0[1] + self.0[2] * rhs.0[2];
        if !dot.is_finite() {
            return Err(NonFiniteValue::new(0));
        }
        Ok(dot)
    }

    /// Returns the squared Euclidean magnitude.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] when squaring or summing the components
    /// overflows.
    pub fn try_magnitude_squared(self) -> Result<f64, NonFiniteValue> {
        self.try_dot(self)
    }

    /// Returns the Euclidean magnitude using scale-safe evaluation.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] when the true magnitude is not representable
    /// as a finite `f64`.
    pub fn try_magnitude(self) -> Result<f64, NonFiniteValue> {
        let [x, y, z] = self.0;
        let max_abs = x.abs().max(y.abs()).max(z.abs());
        if max_abs == 0.0 {
            return Ok(0.0);
        }
        let scaled_x = x / max_abs;
        let scaled_y = y / max_abs;
        let scaled_z = z / max_abs;
        let magnitude =
            max_abs * (scaled_x * scaled_x + scaled_y * scaled_y + scaled_z * scaled_z).sqrt();
        if magnitude.is_finite() {
            Ok(magnitude)
        } else {
            Err(NonFiniteValue::new(0))
        }
    }

    /// Adds `rhs` component-wise.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] if any result component overflows to NaN or
    /// infinity.
    pub fn try_add(self, rhs: Self) -> Result<Self, NonFiniteValue> {
        ensure_finite([
            self.0[0] + rhs.0[0],
            self.0[1] + rhs.0[1],
            self.0[2] + rhs.0[2],
        ])
        .map(Self)
    }

    /// Subtracts `rhs` component-wise.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] if any result component overflows.
    pub fn try_sub(self, rhs: Self) -> Result<Self, NonFiniteValue> {
        ensure_finite([
            self.0[0] - rhs.0[0],
            self.0[1] - rhs.0[1],
            self.0[2] - rhs.0[2],
        ])
        .map(Self)
    }

    /// Scales every component by `scalar`.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] if `scalar` is non-finite or if any
    /// result component overflows.
    pub fn try_scale(self, scalar: f64) -> Result<Self, NonFiniteValue> {
        ensure_finite([self.0[0] * scalar, self.0[1] * scalar, self.0[2] * scalar]).map(Self)
    }

    /// Returns the cross product `self × rhs`.
    ///
    /// Each component is a difference of two products; overflow to infinity
    /// and subsequent cancellation to NaN are possible when components are
    /// near `f64::MAX`.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] if any result component is not finite.
    pub fn try_cross(self, rhs: Self) -> Result<Self, NonFiniteValue> {
        ensure_finite([
            self.0[1] * rhs.0[2] - self.0[2] * rhs.0[1],
            self.0[2] * rhs.0[0] - self.0[0] * rhs.0[2],
            self.0[0] * rhs.0[1] - self.0[1] * rhs.0[0],
        ])
        .map(Self)
    }
}

/// Negation is always finite for finite inputs and is the only infallible
/// arithmetic operation provided for [`Vector3`].
impl ops::Neg for Vector3 {
    type Output = Self;

    fn neg(self) -> Self {
        Self([-self.0[0], -self.0[1], -self.0[2]])
    }
}

// ── Point3 arithmetic ─────────────────────────────────────────────────────────

impl Point3 {
    /// Translates the point by adding `v`.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] if any result coordinate overflows.
    pub fn try_add_vector(self, v: Vector3) -> Result<Self, NonFiniteValue> {
        ensure_finite([self.0[0] + v.0[0], self.0[1] + v.0[1], self.0[2] + v.0[2]]).map(Self)
    }

    /// Translates the point by subtracting `v`.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] if any result coordinate overflows.
    pub fn try_sub_vector(self, v: Vector3) -> Result<Self, NonFiniteValue> {
        ensure_finite([self.0[0] - v.0[0], self.0[1] - v.0[1], self.0[2] - v.0[2]]).map(Self)
    }

    /// Returns the displacement vector from `other` to `self` (`self − other`).
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] if any result component overflows
    /// (possible when the points are far apart and near `±f64::MAX`).
    pub fn try_sub_point(self, other: Self) -> Result<Vector3, NonFiniteValue> {
        ensure_finite([
            self.0[0] - other.0[0],
            self.0[1] - other.0[1],
            self.0[2] - other.0[2],
        ])
        .map(Vector3)
    }
}

// ── Transform3 application and composition ────────────────────────────────────

impl Transform3 {
    /// Applies the transform to a point (includes translation).
    ///
    /// Matrix-vector multiplication can overflow (e.g. large rotation matrix
    /// applied to a large point) and can produce NaN through `∞ − ∞`
    /// cancellation.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] if any result coordinate is not finite.
    pub fn try_apply_to_point(self, point: Point3) -> Result<Point3, NonFiniteValue> {
        let mat = self.0;
        let [px, py, pz] = point.0;
        ensure_finite([
            mat[0] * px + mat[1] * py + mat[2] * pz + mat[3],
            mat[4] * px + mat[5] * py + mat[6] * pz + mat[7],
            mat[8] * px + mat[9] * py + mat[10] * pz + mat[11],
        ])
        .map(Point3)
    }

    /// Applies the linear (rotation/scale) part of the transform to a free
    /// vector (no translation).
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] if any result component is not finite.
    pub fn try_apply_to_vector(self, vec: Vector3) -> Result<Vector3, NonFiniteValue> {
        let mat = self.0;
        let [vx, vy, vz] = vec.0;
        ensure_finite([
            mat[0] * vx + mat[1] * vy + mat[2] * vz,
            mat[4] * vx + mat[5] * vy + mat[6] * vz,
            mat[8] * vx + mat[9] * vy + mat[10] * vz,
        ])
        .map(Vector3)
    }

    /// Returns the composition `self ∘ rhs`: applies `rhs` first, then
    /// `self`.
    ///
    /// This follows the right-operand-first convention from `CONTRACTS.md`.
    /// The 12 matrix products can overflow or cancel to NaN for matrices
    /// with components near `f64::MAX`.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] if any result entry is not finite.
    pub fn try_compose(self, rhs: Self) -> Result<Self, NonFiniteValue> {
        let a = self.0;
        let b = rhs.0;
        ensure_finite([
            a[0] * b[0] + a[1] * b[4] + a[2] * b[8],
            a[0] * b[1] + a[1] * b[5] + a[2] * b[9],
            a[0] * b[2] + a[1] * b[6] + a[2] * b[10],
            a[0] * b[3] + a[1] * b[7] + a[2] * b[11] + a[3],
            a[4] * b[0] + a[5] * b[4] + a[6] * b[8],
            a[4] * b[1] + a[5] * b[5] + a[6] * b[9],
            a[4] * b[2] + a[5] * b[6] + a[6] * b[10],
            a[4] * b[3] + a[5] * b[7] + a[6] * b[11] + a[7],
            a[8] * b[0] + a[9] * b[4] + a[10] * b[8],
            a[8] * b[1] + a[9] * b[5] + a[10] * b[9],
            a[8] * b[2] + a[9] * b[6] + a[10] * b[10],
            a[8] * b[3] + a[9] * b[7] + a[10] * b[11] + a[11],
        ])
        .map(Self)
    }

    /// Creates a pure translation transform.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] if any translation component is NaN or
    /// infinite.
    pub fn from_translation(dx: f64, dy: f64, dz: f64) -> Result<Self, NonFiniteValue> {
        Self::try_from_row_major([1.0, 0.0, 0.0, dx, 0.0, 1.0, 0.0, dy, 0.0, 0.0, 1.0, dz])
    }

    /// Creates a uniform-scale transform about the origin.
    ///
    /// # Errors
    ///
    /// Returns [`NonFiniteValue`] if `factor` is NaN or infinite.
    pub fn from_uniform_scale(factor: f64) -> Result<Self, NonFiniteValue> {
        Self::try_from_row_major([
            factor, 0.0, 0.0, 0.0, 0.0, factor, 0.0, 0.0, 0.0, 0.0, factor, 0.0,
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::{Point2, Point3, Transform3, Vector2, Vector3};

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
        assert!(Point2::try_new(f64::NAN, 0.0).is_err());
        assert!(Point2::try_new(0.0, f64::NEG_INFINITY).is_err());
        assert!(Point3::try_new(0.0, f64::INFINITY, 0.0).is_err());
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn vector3_dot_is_correct() {
        let i = Vector3::try_new(1.0, 0.0, 0.0).unwrap();
        let j = Vector3::try_new(0.0, 1.0, 0.0).unwrap();
        assert_eq!(i.try_dot(j).unwrap(), 0.0);
        assert_eq!(i.try_dot(i).unwrap(), 1.0);
    }

    #[test]
    fn vector3_try_cross_is_correct() {
        let i = Vector3::try_new(1.0, 0.0, 0.0).unwrap();
        let j = Vector3::try_new(0.0, 1.0, 0.0).unwrap();
        let k = Vector3::try_new(0.0, 0.0, 1.0).unwrap();
        assert_eq!(i.try_cross(j).unwrap(), k);
        assert_eq!(j.try_cross(i).unwrap(), -k);
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn vector3_magnitude_is_correct() {
        let v = Vector3::try_new(3.0, 4.0, 0.0).unwrap();
        assert_eq!(v.try_magnitude_squared().unwrap(), 25.0);
        assert_eq!(v.try_magnitude().unwrap(), 5.0);
    }

    #[test]
    fn vector2_cross_z_encodes_orientation() {
        let e1 = Vector2::try_new(1.0, 0.0).unwrap();
        let e2 = Vector2::try_new(0.0, 1.0).unwrap();
        assert!(
            e1.try_cross_z(e2).unwrap() > 0.0,
            "e1×e2 should be positive (CCW)"
        );
        assert!(
            e2.try_cross_z(e1).unwrap() < 0.0,
            "e2×e1 should be negative (CW)"
        );
    }

    #[test]
    fn point3_displacement_vector_is_correct() {
        let a = Point3::try_new(1.0, 2.0, 3.0).unwrap();
        let b = Point3::try_new(4.0, 6.0, 8.0).unwrap();
        let v = b.try_sub_point(a).unwrap();
        assert_eq!(v, Vector3::try_new(3.0, 4.0, 5.0).unwrap());
        let b2 = a.try_add_vector(v).unwrap();
        assert_eq!(b2, b);
    }

    #[test]
    fn transform3_identity_is_no_op() {
        let p = Point3::try_new(1.0, 2.0, 3.0).unwrap();
        let v = Vector3::try_new(4.0, 5.0, 6.0).unwrap();
        assert_eq!(Transform3::IDENTITY.try_apply_to_point(p).unwrap(), p);
        assert_eq!(Transform3::IDENTITY.try_apply_to_vector(v).unwrap(), v);
    }

    #[test]
    fn transform3_translation_moves_point_not_vector() {
        let t = Transform3::from_translation(1.0, 2.0, 3.0).unwrap();
        let origin = Point3::try_new(0.0, 0.0, 0.0).unwrap();
        let moved = t.try_apply_to_point(origin).unwrap();
        assert_eq!(moved, Point3::try_new(1.0, 2.0, 3.0).unwrap());
        let v = Vector3::try_new(1.0, 0.0, 0.0).unwrap();
        assert_eq!(
            t.try_apply_to_vector(v).unwrap(),
            v,
            "translation must not affect vectors"
        );
    }

    #[test]
    fn transform3_compose_right_operand_first() {
        let scale2 = Transform3::from_uniform_scale(2.0).unwrap();
        let trans = Transform3::from_translation(1.0, 0.0, 0.0).unwrap();
        // Apply scale first: (1,0,0) → (2,0,0), then translate → (3,0,0).
        let composed = trans.try_compose(scale2).unwrap();
        let p = Point3::try_new(1.0, 0.0, 0.0).unwrap();
        assert_eq!(
            composed.try_apply_to_point(p).unwrap(),
            Point3::try_new(3.0, 0.0, 0.0).unwrap()
        );
    }

    #[test]
    fn transform3_identity_compose_is_identity() {
        let t = Transform3::from_translation(3.0, -1.0, 2.0).unwrap();
        assert_eq!(t.try_compose(Transform3::IDENTITY).unwrap(), t);
        assert_eq!(Transform3::IDENTITY.try_compose(t).unwrap(), t);
    }

    #[test]
    fn transform3_compose_associative() {
        let t1 = Transform3::from_translation(1.0, 0.0, 0.0).unwrap();
        let t2 = Transform3::from_translation(0.0, 2.0, 0.0).unwrap();
        let t3 = Transform3::from_translation(0.0, 0.0, 3.0).unwrap();
        let p = Point3::try_new(0.0, 0.0, 0.0).unwrap();
        let lhs = t1
            .try_compose(t2.try_compose(t3).unwrap())
            .unwrap()
            .try_apply_to_point(p)
            .unwrap();
        let rhs = t1
            .try_compose(t2)
            .unwrap()
            .try_compose(t3)
            .unwrap()
            .try_apply_to_point(p)
            .unwrap();
        assert_eq!(lhs, rhs);
    }

    #[test]
    fn serde_rejects_nan_and_infinity() {
        let bad_point: Result<Point3, _> = serde_json::from_str(r#"{"x":1.0,"y":1e400,"z":3.0}"#);
        assert!(bad_point.is_err(), "infinite y must be rejected");
        let good_vec: Result<Vector3, _> = serde_json::from_str(r#"{"x":1.0,"y":2.0,"z":2.0}"#);
        assert!(
            good_vec.is_ok(),
            "finite named-field vector must deserialize"
        );
    }

    #[test]
    fn serde_json_shape_is_named_fields() {
        let point = Point3::try_new(1.0, 2.0, 3.0).unwrap();
        let json = serde_json::to_string(&point).unwrap();
        assert!(json.contains("\"x\""), "JSON must contain field name x");
        assert!(json.contains("\"y\""), "JSON must contain field name y");
        assert!(json.contains("\"z\""), "JSON must contain field name z");
    }

    #[test]
    fn serde_transform3_json_shape_is_named_components() {
        let json = serde_json::to_string(&Transform3::IDENTITY).unwrap();
        assert!(
            json.contains("\"m00\"") && json.contains("\"m11\"") && json.contains("\"m23\""),
            "Transform3 JSON must use explicit component names m00..m23, got: {json}"
        );
        let decoded: Transform3 = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, Transform3::IDENTITY);
    }

    #[test]
    fn serde_transform3_rejects_non_finite_component() {
        let bad: Result<Transform3, _> = serde_json::from_str(
            r#"{"m00":1e400,"m01":0.0,"m02":0.0,"m03":0.0,
                 "m10":0.0,"m11":1.0,"m12":0.0,"m13":0.0,
                 "m20":0.0,"m21":0.0,"m22":1.0,"m23":0.0}"#,
        );
        assert!(bad.is_err(), "non-finite m00 must be rejected");
    }

    #[test]
    fn vector3_arithmetic_deterministic() {
        let a = Vector3::try_new(1.0, 2.0, 3.0).unwrap();
        let b = Vector3::try_new(4.0, 5.0, 6.0).unwrap();
        let r1 = a.try_add(b).unwrap();
        let r2 = a.try_add(b).unwrap();
        assert_eq!(r1, r2);
    }

    // ── Overflow / boundary tests ─────────────────────────────────────────────

    #[test]
    fn vector3_add_overflow_is_rejected() {
        let big = Vector3::try_new(f64::MAX, 0.0, 0.0).unwrap();
        assert!(big.try_add(big).is_err(), "f64::MAX + f64::MAX must fail");
    }

    #[test]
    fn vector3_sub_cancellation_nan_is_rejected() {
        // Inf - Inf = NaN: build a vector whose subtraction would produce NaN.
        // MAX * 2 is Inf; we simulate via scale + cross.
        let big = Vector3::try_new(f64::MAX, f64::MAX, 0.0).unwrap();
        let neg_big = Vector3::try_new(f64::MAX, f64::MAX, 0.0).unwrap();
        // MAX - MAX = 0, not NaN — subtraction of equal values is safe.
        assert!(big.try_sub(neg_big).is_ok());
    }

    #[test]
    fn vector3_scale_overflow_is_rejected() {
        let v = Vector3::try_new(f64::MAX, 0.0, 0.0).unwrap();
        assert!(v.try_scale(2.0).is_err(), "MAX * 2 must overflow");
    }

    #[test]
    fn vector3_scale_nan_scalar_is_rejected() {
        let v = Vector3::try_new(1.0, 0.0, 0.0).unwrap();
        assert!(
            v.try_scale(f64::NAN).is_err(),
            "NaN scalar must be rejected"
        );
    }

    #[test]
    fn vector3_try_cross_nan_is_rejected() {
        // cross product can overflow then cancel: a×b component = Inf − Inf = NaN
        let a = Vector3::try_new(f64::MAX, f64::MAX, 0.0).unwrap();
        let b = Vector3::try_new(f64::MAX, f64::MAX, 0.0).unwrap();
        // (MAX*0 - 0*MAX, 0*MAX - MAX*0, MAX*MAX - MAX*MAX)
        // = (0, 0, MAX² - MAX²) = (0, 0, Inf - Inf) = (0, 0, NaN)
        let result = a.try_cross(b);
        assert!(
            result.is_err(),
            "Inf − Inf cross component must be rejected"
        );
    }

    #[test]
    fn vector3_min_positive_is_normalizable() {
        // Very small but nonzero vector: scale-safe normalization handles this.
        let v = Vector3::try_new(f64::MIN_POSITIVE, 0.0, 0.0).unwrap();
        assert!(v.try_sub(v).is_ok());
    }

    #[test]
    fn point3_sub_overflow_is_rejected() {
        let a = Point3::try_new(f64::MAX, 0.0, 0.0).unwrap();
        let b = Point3::try_new(-f64::MAX, 0.0, 0.0).unwrap();
        assert!(
            a.try_sub_point(b).is_err(),
            "MAX - (-MAX) = 2*MAX must overflow"
        );
    }

    #[test]
    fn transform_apply_nan_from_cancellation_is_rejected() {
        // Construct a matrix and point whose product cancels to NaN.
        // Row 0: [MAX, -MAX, 0, 0] applied to (MAX, MAX, 0) → MAX*MAX + (-MAX)*MAX = Inf - Inf = NaN
        let mat = Transform3::try_from_row_major([
            f64::MAX,
            -f64::MAX,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
            0.0,
            0.0,
            0.0,
            1.0,
            0.0,
        ])
        .unwrap();
        let p = Point3::try_new(f64::MAX, f64::MAX, 0.0).unwrap();
        assert!(
            mat.try_apply_to_point(p).is_err(),
            "NaN from Inf cancellation must be rejected"
        );
    }

    #[test]
    fn vector2_neg_is_infallible() {
        // Negation is the only arithmetic guaranteed to preserve finiteness.
        let v = Vector2::try_new(f64::MAX, -f64::MIN_POSITIVE).unwrap();
        let neg = -v;
        assert!(neg.x().is_finite() && neg.y().is_finite());
    }

    #[test]
    fn vector3_neg_is_infallible() {
        let v = Vector3::try_new(f64::MAX, f64::MIN_POSITIVE, 42.0).unwrap();
        let _ = -v; // must compile and not panic
    }

    #[test]
    fn vector3_subnormal_add_does_not_panic() {
        // Subnormal inputs: smallest positive denormal
        let tiny = f64::from_bits(1);
        let v1 = Vector3::try_new(tiny, 0.0, 0.0).unwrap();
        let v2 = Vector3::try_new(tiny, 0.0, 0.0).unwrap();
        assert!(v1.try_add(v2).is_ok());
    }

    #[test]
    fn vector2_try_dot_overflow_is_rejected() {
        let big = Vector2::try_new(f64::MAX, f64::MAX).unwrap();
        assert!(big.try_dot(big).is_err());
    }

    #[test]
    fn vector2_try_cross_z_overflow_is_rejected() {
        let a = Vector2::try_new(f64::MAX, -f64::MAX).unwrap();
        let b = Vector2::try_new(f64::MAX, f64::MAX).unwrap();
        assert!(a.try_cross_z(b).is_err());
    }

    #[test]
    fn vector3_try_magnitude_overflow_is_rejected() {
        let v = Vector3::try_new(f64::MAX, f64::MAX, f64::MAX).unwrap();
        assert!(v.try_magnitude().is_err());
    }

    #[test]
    #[allow(clippy::float_cmp)]
    fn vector3_try_magnitude_half_max_is_finite() {
        let v = Vector3::try_new(f64::MAX / 2.0, 0.0, 0.0).unwrap();
        assert_eq!(v.try_magnitude().unwrap(), f64::MAX / 2.0);
    }

    #[test]
    fn vector3_try_magnitude_squared_overflow_is_rejected() {
        let v = Vector3::try_new(f64::MAX, 0.0, 0.0).unwrap();
        assert!(v.try_magnitude_squared().is_err());
    }
}
