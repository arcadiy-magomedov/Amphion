//! Normative model-space conventions.

use serde::{Deserialize, Serialize};

/// Handedness of a three-dimensional coordinate frame.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Handedness {
    /// Positive rotations follow the right-hand rule.
    Right,
}

/// A principal model-space axis.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub enum Axis3 {
    /// The positive X axis.
    X,
    /// The positive Y axis.
    Y,
    /// The positive Z axis.
    Z,
}

/// A supported display or exchange length unit.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum LengthUnit {
    /// Metres.
    Meter,
    /// Millimetres.
    Millimeter,
    /// Centimetres.
    Centimeter,
    /// Micrometres.
    Micrometer,
    /// Inches.
    Inch,
    /// Feet.
    Foot,
}

/// A supported angle unit.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[non_exhaustive]
pub enum AngleUnit {
    /// Radians.
    Radian,
    /// Degrees.
    Degree,
}

/// Amphion model frames are always right-handed.
pub const MODEL_HANDEDNESS: Handedness = Handedness::Right;

/// Canonical model-space lengths are measured in metres.
pub const MODEL_LENGTH_UNIT: LengthUnit = LengthUnit::Meter;

/// Canonical model-space angles are measured in radians.
pub const MODEL_ANGLE_UNIT: AngleUnit = AngleUnit::Radian;
