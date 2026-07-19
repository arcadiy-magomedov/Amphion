//! Normalized direction abstractions guaranteed to be finite and non-zero.
//!
//! [`UnitVector2`] and [`UnitVector3`] are constructed only through
//! [`try_normalize`][UnitVector3::try_normalize], which rejects zero
//! and non-finite inputs. The normalized components are guaranteed to have a
//! Euclidean magnitude within **4×`f64::EPSILON`** of 1.0.
//!
//! # Scale-safe normalization
//!
//! Naive `sqrt(x²+y²+z²)` overflows for vectors with components near
//! `f64::MAX` (e.g., `[f64::MAX, f64::MAX, f64::MAX]`). The implementation
//! divides every component by the maximum absolute component before computing
//! the magnitude. This keeps all intermediate values in `[-1, 1]`, so the
//! scaled magnitude is in `[1, √3]` — always finite and representable. The
//! same logic is used by the serde-validation path (`TryFrom`).

use core::error::Error;
use core::fmt;
use core::ops;

use serde::{Deserialize, Serialize};

use crate::math::{Vector2, Vector3, Xy, Xyz};

/// Error returned when a vector cannot be normalized.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NormalizationError {
    /// One or more components are not finite.
    NonFinite,
    /// The vector magnitude is exactly zero.
    ZeroMagnitude,
    /// The provided values do not form a unit vector (deserialization path).
    ///
    /// The magnitude deviated from 1.0 by more than 4×`f64::EPSILON`.
    /// Use [`UnitVector3::try_normalize`] to normalize an arbitrary vector.
    NotNormalized,
}

impl fmt::Display for NormalizationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::NonFinite => "unit-vector components must be finite",
            Self::ZeroMagnitude => "cannot normalize a zero-length vector",
            Self::NotNormalized => {
                "magnitude deviates from 1.0 by more than 4\u{00d7}f64::EPSILON; \
                 use try_normalize to normalize an arbitrary vector"
            }
        };
        formatter.write_str(message)
    }
}

impl Error for NormalizationError {}

// ── UnitVector2 ───────────────────────────────────────────────────────────────

/// A finite, non-zero, normalized direction in 2-D.
///
/// Invariant: all components are finite and the Euclidean magnitude is within
/// **4×`f64::EPSILON`** of 1.0. The only way to obtain a value is through
/// [`UnitVector2::try_normalize`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Xy", into = "Xy")]
pub struct UnitVector2([f64; 2]);

impl UnitVector2 {
    /// Normalizes `vector` and wraps the result.
    ///
    /// Uses scale-safe normalization: all components are divided by the
    /// maximum absolute component before computing the magnitude. This ensures
    /// that any nonzero finite vector — including vectors with components near
    /// `f64::MAX` — is normalized without intermediate overflow.
    ///
    /// # Errors
    ///
    /// Returns [`NormalizationError`] when any component is non-finite, or
    /// when the vector is the zero vector.
    pub fn try_normalize(vector: Vector2) -> Result<Self, NormalizationError> {
        let [x, y] = vector.into_array();
        if !x.is_finite() || !y.is_finite() {
            return Err(NormalizationError::NonFinite);
        }
        let max_abs = x.abs().max(y.abs());
        if max_abs == 0.0 {
            return Err(NormalizationError::ZeroMagnitude);
        }
        // Scale so the dominant component becomes ±1; intermediate magnitude is in [1, √2].
        let xs = x / max_abs;
        let ys = y / max_abs;
        let scaled_mag = (xs * xs + ys * ys).sqrt();
        Ok(Self([xs / scaled_mag, ys / scaled_mag]))
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

    /// Returns the underlying components as a [`Vector2`].
    #[must_use]
    pub fn as_vector(self) -> Vector2 {
        Vector2::from_finite_unchecked(self.0)
    }

    /// Returns components in canonical order.
    #[must_use]
    pub const fn into_array(self) -> [f64; 2] {
        self.0
    }

    /// Returns the dot product with another unit vector.
    ///
    /// The result may be very slightly outside `[-1.0, 1.0]` due to
    /// floating-point rounding. Use [`Self::cos_angle_to`] to obtain a value
    /// safe for `f64::acos`.
    #[must_use]
    pub fn dot(self, rhs: Self) -> f64 {
        self.0[0] * rhs.0[0] + self.0[1] * rhs.0[1]
    }

    /// Returns `self · rhs` clamped to `[-1.0, 1.0]`.
    ///
    /// Use this — not [`Self::dot`] — when computing the angle between two
    /// directions via `f64::acos`.
    #[must_use]
    pub fn cos_angle_to(self, rhs: Self) -> f64 {
        self.dot(rhs).clamp(-1.0, 1.0)
    }

    /// Returns the scalar Z-component of the 3-D cross product (self × rhs).
    #[must_use]
    pub fn cross_z(self, rhs: Self) -> f64 {
        self.0[0] * rhs.0[1] - self.0[1] * rhs.0[0]
    }
}

impl ops::Neg for UnitVector2 {
    type Output = Self;

    fn neg(self) -> Self {
        Self([-self.0[0], -self.0[1]])
    }
}

impl TryFrom<[f64; 2]> for UnitVector2 {
    type Error = NormalizationError;

    /// Validates that `value` is already a unit vector (magnitude within
    /// 4×`f64::EPSILON` of 1.0) using scale-safe magnitude computation.
    ///
    /// Use [`UnitVector2::try_normalize`] to normalize an arbitrary vector.
    fn try_from(value: [f64; 2]) -> Result<Self, Self::Error> {
        if !value[0].is_finite() || !value[1].is_finite() {
            return Err(NormalizationError::NonFinite);
        }
        let max_abs = value[0].abs().max(value[1].abs());
        if max_abs == 0.0 {
            return Err(NormalizationError::ZeroMagnitude);
        }
        let xs = value[0] / max_abs;
        let ys = value[1] / max_abs;
        // True magnitude = max_abs × sqrt(xs² + ys²).
        let mag = (xs * xs + ys * ys).sqrt() * max_abs;
        if (mag - 1.0).abs() > 4.0 * f64::EPSILON {
            return Err(NormalizationError::NotNormalized);
        }
        Ok(Self(value))
    }
}

impl From<UnitVector2> for [f64; 2] {
    fn from(value: UnitVector2) -> Self {
        value.0
    }
}

impl TryFrom<Xy> for UnitVector2 {
    type Error = NormalizationError;

    fn try_from(value: Xy) -> Result<Self, Self::Error> {
        Self::try_from([value.x, value.y])
    }
}

impl From<UnitVector2> for Xy {
    fn from(value: UnitVector2) -> Self {
        Xy {
            x: value.0[0],
            y: value.0[1],
        }
    }
}

// ── UnitVector3 ───────────────────────────────────────────────────────────────

/// A finite, non-zero, normalized direction in 3-D model space.
///
/// Invariant: all components are finite and the Euclidean magnitude is within
/// **4×`f64::EPSILON`** of 1.0. The only way to obtain a value is through
/// [`UnitVector3::try_normalize`].
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "Xyz", into = "Xyz")]
pub struct UnitVector3([f64; 3]);

impl UnitVector3 {
    /// Normalizes `vector` and wraps the result.
    ///
    /// Uses scale-safe normalization: all components are divided by the
    /// maximum absolute component before computing the magnitude. This ensures
    /// that any nonzero finite vector — including vectors with components near
    /// `f64::MAX` — is normalized without intermediate overflow.
    ///
    /// # Errors
    ///
    /// Returns [`NormalizationError`] when any component is non-finite, or
    /// when the vector is the zero vector.
    pub fn try_normalize(vector: Vector3) -> Result<Self, NormalizationError> {
        let [x, y, z] = vector.into_array();
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return Err(NormalizationError::NonFinite);
        }
        let max_abs = x.abs().max(y.abs()).max(z.abs());
        if max_abs == 0.0 {
            return Err(NormalizationError::ZeroMagnitude);
        }
        // Scale so the dominant component becomes ±1; intermediate magnitude is in [1, √3].
        let xs = x / max_abs;
        let ys = y / max_abs;
        let zs = z / max_abs;
        let scaled_mag = (xs * xs + ys * ys + zs * zs).sqrt();
        Ok(Self([xs / scaled_mag, ys / scaled_mag, zs / scaled_mag]))
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

    /// Returns the underlying components as a [`Vector3`].
    #[must_use]
    pub fn as_vector(self) -> Vector3 {
        Vector3::from_finite_unchecked(self.0)
    }

    /// Returns components in canonical order.
    #[must_use]
    pub const fn into_array(self) -> [f64; 3] {
        self.0
    }

    /// Returns the dot product with another unit vector.
    ///
    /// The result may be very slightly outside `[-1.0, 1.0]` due to
    /// floating-point rounding. Use [`Self::cos_angle_to`] to obtain a value
    /// safe for `f64::acos`.
    #[must_use]
    pub fn dot(self, rhs: Self) -> f64 {
        self.0[0] * rhs.0[0] + self.0[1] * rhs.0[1] + self.0[2] * rhs.0[2]
    }

    /// Returns `self · rhs` clamped to `[-1.0, 1.0]`.
    ///
    /// Use this — not [`Self::dot`] — when computing the angle between two
    /// directions via `f64::acos`.
    #[must_use]
    pub fn cos_angle_to(self, rhs: Self) -> f64 {
        self.dot(rhs).clamp(-1.0, 1.0)
    }

    /// Returns the cross product `self × rhs` as a raw [`Vector3`].
    ///
    /// The result has magnitude `sin(θ)` where θ is the angle between the
    /// two directions; it is not a unit vector unless `θ = π/2`. The result
    /// is always finite since unit-vector components are in `[-1, 1]`.
    #[must_use]
    pub fn cross(self, rhs: Self) -> Vector3 {
        Vector3::from_finite_unchecked([
            self.0[1] * rhs.0[2] - self.0[2] * rhs.0[1],
            self.0[2] * rhs.0[0] - self.0[0] * rhs.0[2],
            self.0[0] * rhs.0[1] - self.0[1] * rhs.0[0],
        ])
    }
}

impl ops::Neg for UnitVector3 {
    type Output = Self;

    fn neg(self) -> Self {
        Self([-self.0[0], -self.0[1], -self.0[2]])
    }
}

impl TryFrom<[f64; 3]> for UnitVector3 {
    type Error = NormalizationError;

    /// Validates that `value` is already a unit vector (magnitude within
    /// 4×`f64::EPSILON` of 1.0) using scale-safe magnitude computation.
    ///
    /// Use [`UnitVector3::try_normalize`] to normalize an arbitrary vector.
    fn try_from(value: [f64; 3]) -> Result<Self, Self::Error> {
        if !value[0].is_finite() || !value[1].is_finite() || !value[2].is_finite() {
            return Err(NormalizationError::NonFinite);
        }
        let max_abs = value[0].abs().max(value[1].abs()).max(value[2].abs());
        if max_abs == 0.0 {
            return Err(NormalizationError::ZeroMagnitude);
        }
        let xs = value[0] / max_abs;
        let ys = value[1] / max_abs;
        let zs = value[2] / max_abs;
        // True magnitude = max_abs × sqrt(xs² + ys² + zs²).
        let mag = (xs * xs + ys * ys + zs * zs).sqrt() * max_abs;
        if (mag - 1.0).abs() > 4.0 * f64::EPSILON {
            return Err(NormalizationError::NotNormalized);
        }
        Ok(Self(value))
    }
}

impl From<UnitVector3> for [f64; 3] {
    fn from(value: UnitVector3) -> Self {
        value.0
    }
}

impl TryFrom<Xyz> for UnitVector3 {
    type Error = NormalizationError;

    fn try_from(value: Xyz) -> Result<Self, Self::Error> {
        Self::try_from([value.x, value.y, value.z])
    }
}

impl From<UnitVector3> for Xyz {
    fn from(value: UnitVector3) -> Self {
        Xyz {
            x: value.0[0],
            y: value.0[1],
            z: value.0[2],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NormalizationError, UnitVector2, UnitVector3};
    use crate::math::{Vector2, Vector3};

    #[test]
    fn unit_vector3_canonical_axes() {
        let i = UnitVector3::try_normalize(Vector3::try_new(1.0, 0.0, 0.0).unwrap()).unwrap();
        let j = UnitVector3::try_normalize(Vector3::try_new(0.0, 1.0, 0.0).unwrap()).unwrap();
        assert!((i.dot(j) - 0.0).abs() < f64::EPSILON);
        assert!((i.dot(i) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn unit_vector3_zero_rejected() {
        let v = Vector3::try_new(0.0, 0.0, 0.0).unwrap();
        assert_eq!(
            UnitVector3::try_normalize(v).unwrap_err(),
            NormalizationError::ZeroMagnitude
        );
    }

    #[test]
    fn unit_vector3_huge_vector_normalizes_successfully() {
        // With scale-safe normalization, [MAX, MAX, MAX] should succeed.
        let large = f64::MAX;
        let v = Vector3::try_new(large, large, large).unwrap();
        let u = UnitVector3::try_normalize(v).unwrap();
        let inv_sqrt3 = 1.0_f64 / 3.0_f64.sqrt();
        assert!(
            (u.x() - inv_sqrt3).abs() < 4.0 * f64::EPSILON,
            "x component"
        );
        assert!(
            (u.y() - inv_sqrt3).abs() < 4.0 * f64::EPSILON,
            "y component"
        );
        assert!(
            (u.z() - inv_sqrt3).abs() < 4.0 * f64::EPSILON,
            "z component"
        );
    }

    #[test]
    fn unit_vector3_tiny_vector_normalizes_successfully() {
        // MIN_POSITIVE (smallest normal f64) must normalize to (1, 0, 0).
        let v = Vector3::try_new(f64::MIN_POSITIVE, 0.0, 0.0).unwrap();
        let u = UnitVector3::try_normalize(v).unwrap();
        assert!((u.x() - 1.0).abs() < 4.0 * f64::EPSILON, "x should be 1.0");
    }

    #[test]
    fn unit_vector3_subnormal_component_normalizes_successfully() {
        // Smallest positive subnormal.
        let tiny = f64::from_bits(1);
        let v = Vector3::try_new(tiny, 0.0, 0.0).unwrap();
        let u = UnitVector3::try_normalize(v).unwrap();
        assert!((u.x() - 1.0).abs() < 4.0 * f64::EPSILON);
    }

    #[test]
    fn unit_vector3_magnitude_is_one() {
        let v = Vector3::try_new(3.0, 4.0, 0.0).unwrap();
        let u = UnitVector3::try_normalize(v).unwrap();
        let mag = (u.x() * u.x() + u.y() * u.y() + u.z() * u.z()).sqrt();
        assert!(
            (mag - 1.0).abs() <= 4.0 * f64::EPSILON,
            "magnitude must be within 4×EPSILON of 1.0, got {mag}"
        );
    }

    #[test]
    fn unit_vector2_canonical_axes() {
        let e1 = UnitVector2::try_normalize(Vector2::try_new(1.0, 0.0).unwrap()).unwrap();
        let e2 = UnitVector2::try_normalize(Vector2::try_new(0.0, 1.0).unwrap()).unwrap();
        assert!((e1.dot(e2) - 0.0).abs() < f64::EPSILON);
    }

    #[test]
    fn unit_vector2_zero_rejected() {
        let v = Vector2::try_new(0.0, 0.0).unwrap();
        assert_eq!(
            UnitVector2::try_normalize(v).unwrap_err(),
            NormalizationError::ZeroMagnitude
        );
    }

    #[test]
    fn unit_vector3_negation_is_antiparallel() {
        let v = Vector3::try_new(1.0, 2.0, 3.0).unwrap();
        let u = UnitVector3::try_normalize(v).unwrap();
        let neg = -u;
        assert!((u.dot(neg) - (-1.0)).abs() < 2.0 * f64::EPSILON);
    }

    #[test]
    fn unit_vector3_serde_round_trip() {
        let u = UnitVector3::try_normalize(Vector3::try_new(1.0, 1.0, 1.0).unwrap()).unwrap();
        let json = serde_json::to_string(&u).unwrap();
        let u2: UnitVector3 = serde_json::from_str(&json).unwrap();
        assert_eq!(u, u2);
    }

    #[test]
    fn unit_vector3_serde_rejects_non_unit() {
        // A raw array that is not normalized should be rejected on deserialize.
        let bad: Result<UnitVector3, _> = serde_json::from_str(r#"{"x":2.0,"y":0.0,"z":0.0}"#);
        assert!(
            bad.is_err(),
            "non-unit arrays must be rejected by deserialization"
        );
    }

    #[test]
    fn unit_vector3_serde_json_shape_is_named() {
        let unit = UnitVector3::try_normalize(Vector3::try_new(1.0, 0.0, 0.0).unwrap()).unwrap();
        let json = serde_json::to_string(&unit).unwrap();
        assert!(json.contains("\"x\"") && json.contains("\"y\"") && json.contains("\"z\""));
    }

    #[test]
    fn unit_vector3_cross_orthogonal_is_unit() {
        let i = UnitVector3::try_normalize(Vector3::try_new(1.0, 0.0, 0.0).unwrap()).unwrap();
        let j = UnitVector3::try_normalize(Vector3::try_new(0.0, 1.0, 0.0).unwrap()).unwrap();
        let k = i.cross(j);
        let expected = Vector3::try_new(0.0, 0.0, 1.0).unwrap();
        assert_eq!(k, expected);
    }

    #[test]
    fn unit_vector3_deterministic() {
        let v = Vector3::try_new(1.0, 2.0, 3.0).unwrap();
        let u1 = UnitVector3::try_normalize(v).unwrap();
        let u2 = UnitVector3::try_normalize(v).unwrap();
        assert_eq!(u1, u2, "normalization must be deterministic");
    }

    #[test]
    fn unit_vector2_serde_round_trip() {
        let u = UnitVector2::try_normalize(Vector2::try_new(1.0, 0.0).unwrap()).unwrap();
        let json = serde_json::to_string(&u).unwrap();
        let u2: UnitVector2 = serde_json::from_str(&json).unwrap();
        assert_eq!(u, u2);
    }

    #[test]
    fn unit_vector2_serde_rejects_non_unit() {
        let bad: Result<UnitVector2, _> = serde_json::from_str(r#"{"x":2.0,"y":0.0}"#);
        assert!(bad.is_err(), "non-unit array must be rejected");
    }

    #[test]
    fn unit_vector2_serde_rejects_malicious_unit() {
        let bad: Result<UnitVector2, _> = serde_json::from_str(r#"{"x":0.8,"y":0.8}"#);
        assert!(bad.is_err());
    }

    #[test]
    fn unit_vector2_serde_rejects_non_finite() {
        let bad: Result<UnitVector2, _> = serde_json::from_str(r#"{"x":1e400,"y":0.0}"#);
        assert!(bad.is_err(), "non-finite array must be rejected");
    }

    #[test]
    fn unit_vector2_serde_rejects_zero() {
        let bad: Result<UnitVector2, _> = serde_json::from_str(r#"{"x":0.0,"y":0.0}"#);
        assert!(bad.is_err(), "zero vector must be rejected");
    }
}
