//! Analytic right circular cone surface.
//!
//! # Parameterization
//!
//! ```text
//! p(u, v) = apex + v·axis + v·tan(α)·(cos(u)·x_axis + sin(u)·y_axis)
//! ```
//!
//! where `α = half_angle`, `y_axis = axis × x_axis`.  `axis`, `x_axis`, and
//! `y_axis` form a right-handed orthonormal frame.  The parameter `v` is the
//! signed axial distance from the apex; both nappes (`v > 0` and `v < 0`) are
//! included.
//!
//! - U domain: `[0, 2π)` with period `2π` (angular)
//! - V domain: `(−∞, +∞)` (axial signed height from apex)
//!
//! The apex `v = 0` is a conical singularity: `∂p/∂u = 0` there.  Requesting
//! first- or second-order derivatives at `v = 0` returns
//! [`GeometryError::Singular`] (this is a geometric fact, independent of
//! trig certification).
//!
//! [`evaluate`](crate::traits::SurfaceEvaluator::evaluate) and
//! [`project_into`](crate::traits::SurfaceEvaluator::project_into) require
//! `sin`, `cos`, and `tan`. No pure-Rust, WASM-compatible, formally-proved
//! correctly-rounded implementation of these functions currently exists (see
//! the `analytic::helpers` module docs for the survey of candidates), so
//! both methods return [`GeometryError::Uncertified`] once the
//! domain/finiteness/singularity checks have passed.

#![allow(
    clippy::many_single_char_names,
    clippy::missing_panics_doc,
    clippy::similar_names
)]

use std::f64::consts::TAU;

use amphion_foundation::{Point3, ToleranceContext, Transform3, Vector3};
use serde::{Deserialize, Serialize};

use crate::traits::SurfaceEvaluator;
use crate::{
    DerivativeOrder, GeometryError, ParameterRange, SurfaceDomain, SurfaceEvaluation, SurfaceKind,
    SurfaceProjection,
};

use super::{
    ConstructionError, TransformError,
    helpers::{
        ILL_COND_THRESH, all_finite3, cross3, dot3, in_range, mag3, normalize3, scale3, sub3,
        validate_orthogonal3, validate_unit3,
    },
    transform::{apply_to_point, apply_to_vector, similarity_scale},
};

fn angular_range() -> ParameterRange {
    // (0.0, TAU, TAU) is a compile-time constant with lo < hi; this is not
    // an input-dependent path, so a static-invariant `expect` is acceptable
    // here (see CONTRACTS.md).
    ParameterRange::try_new(Some(0.0), Some(TAU), Some(TAU))
        .expect("angular [0, 2π) domain is always valid")
}

fn unbounded_range() -> ParameterRange {
    // (None, None, None) is a compile-time constant and always valid; this
    // is not an input-dependent path, so a static-invariant `expect` is
    // acceptable here (see CONTRACTS.md).
    ParameterRange::try_new(None, None, None).expect("unbounded domain is always valid")
}

#[derive(Serialize, Deserialize)]
struct ConeRepr {
    apex: Point3,
    axis: Vector3,
    half_angle: f64,
    x_axis: Vector3,
}

/// A right circular cone surface, including both nappes.
///
/// Parameterization:
/// ```text
/// p(u, v) = apex + v·axis + v·tan(α)·(cos(u)·x_axis + sin(u)·y_axis)
/// ```
/// U ∈ `[0, 2π)` (periodic), V ∈ `(−∞, +∞)`.  `α = half_angle` ∈ `(0, π/2)`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "ConeRepr", into = "ConeRepr")]
pub struct Cone {
    apex: Point3,
    axis: Vector3,
    half_angle: f64,
    x_axis: Vector3,
    y_axis: Vector3,
}

impl Cone {
    /// Constructs a cone.
    ///
    /// `axis` and `x_axis` are normalized internally.  `x_axis` is
    /// orthogonalized against `axis` (Gram-Schmidt).
    ///
    /// # Errors
    ///
    /// - [`ConstructionError::NonFiniteInput`] — any NaN/Inf input
    /// - [`ConstructionError::DegenerateAxis`] — zero-length `axis` or `x_axis`
    /// - [`ConstructionError::InvalidHalfAngle`] — `half_angle ∉ (0, π/2)`
    /// - [`ConstructionError::DependentAxes`] — `x_axis` parallel to `axis`
    /// - [`ConstructionError::IllConditionedAxes`] — `x_axis` nearly parallel
    ///   to `axis`
    pub fn try_new(
        apex: Point3,
        axis: Vector3,
        half_angle: f64,
        x_axis: Vector3,
    ) -> Result<Self, ConstructionError> {
        let ap = apex.into_array();
        let a = axis.into_array();
        let x = x_axis.into_array();
        if !all_finite3(ap) || !all_finite3(a) || !half_angle.is_finite() || !all_finite3(x) {
            return Err(ConstructionError::NonFiniteInput);
        }
        if half_angle <= 0.0 || half_angle >= std::f64::consts::FRAC_PI_2 {
            return Err(ConstructionError::InvalidHalfAngle);
        }
        let a_unit = normalize3(a).ok_or(ConstructionError::DegenerateAxis)?;
        let x_norm = normalize3(x).ok_or(ConstructionError::DegenerateAxis)?;
        // Orthogonalize x against axis.
        let dot_xa = dot3(x_norm, a_unit);
        let x_perp = sub3(x_norm, scale3(a_unit, dot_xa));
        let perp_mag = mag3(x_perp);
        if perp_mag == 0.0 {
            return Err(ConstructionError::DependentAxes);
        }
        if perp_mag < ILL_COND_THRESH {
            return Err(ConstructionError::IllConditionedAxes);
        }
        let x_unit = normalize3(x_perp).ok_or(ConstructionError::DependentAxes)?;
        let y_arr = cross3(a_unit, x_unit);
        Ok(Self {
            apex: Point3::try_new(ap[0], ap[1], ap[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            axis: Vector3::try_new(a_unit[0], a_unit[1], a_unit[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            half_angle,
            x_axis: Vector3::try_new(x_unit[0], x_unit[1], x_unit[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            y_axis: Vector3::try_new(y_arr[0], y_arr[1], y_arr[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
        })
    }

    /// Returns the apex point.
    #[must_use]
    pub fn apex(&self) -> Point3 {
        self.apex
    }

    /// Returns the unit axis direction.
    #[must_use]
    pub fn axis(&self) -> Vector3 {
        self.axis
    }

    /// Returns the half-angle in radians (strictly between 0 and π/2).
    #[must_use]
    pub fn half_angle(&self) -> f64 {
        self.half_angle
    }

    /// Returns the unit reference direction for `u = 0`.
    #[must_use]
    pub fn x_axis(&self) -> Vector3 {
        self.x_axis
    }

    /// Returns the unit y-axis: `axis × x_axis`.
    #[must_use]
    pub fn y_axis(&self) -> Vector3 {
        self.y_axis
    }

    /// Applies a similarity `transform` (rigid motion plus uniform scale, no
    /// reflection) to this cone, returning a new cone with the same
    /// `half_angle` (an angle is invariant under a similarity transform).
    ///
    /// A general affine transform does not map a circular cone to a
    /// circular cone, so only similarity transforms are accepted; see the
    /// `transform` module documentation for the (provisional, heuristic)
    /// similarity test.
    ///
    /// # Errors
    ///
    /// - [`TransformError::NotSimilarity`] — the transform's linear part is
    ///   not (within tolerance) a uniform-scale rotation
    /// - [`TransformError::NonFiniteResult`] — the transformed apex or axes
    ///   contain a non-finite component
    /// - [`TransformError::DegenerateResult`] — the transformed axes fail
    ///   cone construction
    pub fn try_transform(&self, transform: &Transform3) -> Result<Self, TransformError> {
        let m = transform.into_row_major();
        // The scale factor itself is irrelevant to a cone (half_angle and
        // the apex-relative direction fully determine its shape); only its
        // existence certifies that `transform` is a similarity.
        similarity_scale(m).ok_or(TransformError::NotSimilarity)?;
        let ap =
            apply_to_point(m, self.apex.into_array()).ok_or(TransformError::NonFiniteResult)?;
        let a =
            apply_to_vector(m, self.axis.into_array()).ok_or(TransformError::NonFiniteResult)?;
        let x =
            apply_to_vector(m, self.x_axis.into_array()).ok_or(TransformError::NonFiniteResult)?;
        Self::try_new(
            Point3::try_new(ap[0], ap[1], ap[2]).map_err(|_| TransformError::NonFiniteResult)?,
            Vector3::try_new(a[0], a[1], a[2]).map_err(|_| TransformError::NonFiniteResult)?,
            self.half_angle,
            Vector3::try_new(x[0], x[1], x[2]).map_err(|_| TransformError::NonFiniteResult)?,
        )
        .map_err(|_| TransformError::DegenerateResult)
    }
}

impl TryFrom<ConeRepr> for Cone {
    type Error = ConstructionError;
    fn try_from(repr: ConeRepr) -> Result<Self, Self::Error> {
        let apex = repr.apex.into_array();
        let axis = repr.axis.into_array();
        let x_axis = repr.x_axis.into_array();
        if !all_finite3(apex)
            || !all_finite3(axis)
            || !repr.half_angle.is_finite()
            || !all_finite3(x_axis)
        {
            return Err(ConstructionError::NonFiniteInput);
        }
        if repr.half_angle <= 0.0 || repr.half_angle >= std::f64::consts::FRAC_PI_2 {
            return Err(ConstructionError::InvalidHalfAngle);
        }
        validate_unit3(axis)?;
        validate_unit3(x_axis)?;
        validate_orthogonal3(axis, x_axis)?;
        let y_arr = cross3(axis, x_axis);
        Ok(Self {
            apex: repr.apex,
            axis: repr.axis,
            half_angle: repr.half_angle,
            x_axis: repr.x_axis,
            y_axis: Vector3::try_new(y_arr[0], y_arr[1], y_arr[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
        })
    }
}

impl From<Cone> for ConeRepr {
    fn from(c: Cone) -> Self {
        Self {
            apex: c.apex,
            axis: c.axis,
            half_angle: c.half_angle,
            x_axis: c.x_axis,
        }
    }
}

impl SurfaceEvaluator for Cone {
    fn kind(&self) -> SurfaceKind {
        SurfaceKind::Cone
    }

    fn domain(&self) -> SurfaceDomain {
        SurfaceDomain::new(angular_range(), unbounded_range())
    }

    fn evaluate(
        &self,
        u: f64,
        v: f64,
        order: DerivativeOrder,
        _tolerance: &ToleranceContext,
    ) -> Result<SurfaceEvaluation, GeometryError> {
        if !u.is_finite() || !v.is_finite() {
            return Err(GeometryError::NonFiniteParameter);
        }
        if !in_range(u, self.domain().u()) {
            return Err(GeometryError::OutsideDomain);
        }
        // Apex singularity: ∂p/∂u = 0 at v = 0. This is a genuine geometric
        // singularity independent of trig certification (it holds regardless
        // of how accurately sin/cos/tan are computed), so it is checked
        // before falling through to the Uncertified result below.
        if v == 0.0 && order != DerivativeOrder::Position {
            return Err(GeometryError::Singular);
        }
        // p(u, v) = apex + v·axis + v·tan(α)·(cos(u)·x_axis + sin(u)·y_axis)
        // requires `cos`, `sin`, and `tan`. No pure-Rust, WASM-compatible,
        // formally-proved correctly-rounded implementation of these
        // functions currently exists (see the `analytic::helpers` module
        // docs for the survey of candidates), so no certified error bound
        // can be produced.
        Err(GeometryError::Uncertified {
            reason: "cone evaluation requires certified sin/cos/tan; no formally-proved \
                     WASM-compatible implementation is available. libm (MIT, WASM) gives \
                     ~1-2 ULP empirically but is not formally proved. core-math (MIT, 0.5 ULP) \
                     requires C FFI incompatible with WASM. IEEE 754-2019 §9.2 recommends but \
                     does not require correctly-rounded transcendentals."
                .to_owned(),
        })
    }

    fn project_into(
        &self,
        _point: Point3,
        _tolerance: &ToleranceContext,
        output: &mut Vec<SurfaceProjection>,
    ) -> Result<(), GeometryError> {
        output.clear();
        // u = atan2(...) is an uncertified std transcendental; the nappe
        // disambiguation and sin(u)/cos(u)/tan(α) reconstruction of the
        // projected point are also uncertified. See the `analytic::helpers`
        // module docs.
        Err(GeometryError::Uncertified {
            reason: "cone projection requires certified atan2/sin/cos/tan; pending certified \
                     trig integration. See: libm crate (empirical accuracy only), core-math \
                     (0.5 ULP, not WASM-compatible)."
                .to_owned(),
        })
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use std::f64::consts::{FRAC_PI_2, FRAC_PI_4, TAU};

    use amphion_foundation::{Point3, ToleranceContext, Vector3};
    use serde_json::json;

    use crate::analytic::helpers::ILL_COND_THRESH;
    use crate::traits::SurfaceEvaluator;
    use crate::{DerivativeOrder, GeometryError};

    use super::{Cone, ConeRepr, ConstructionError};

    fn tol() -> ToleranceContext {
        ToleranceContext::try_new(1e-9, 1e-8, 1e-10, 1e-12).unwrap()
    }

    fn assert_uncertified(err: &GeometryError) {
        match err {
            GeometryError::Uncertified { reason } => {
                assert!(!reason.is_empty(), "reason string must not be empty");
            }
            other => panic!("expected Uncertified, got {other:?}"),
        }
    }

    fn std_cone() -> Cone {
        Cone::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            FRAC_PI_4,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn cone_construction_valid() {
        // half_angle is stored exactly as passed; FRAC_PI_4 round-trips bit-for-bit.
        assert_eq!(std_cone().half_angle().to_bits(), FRAC_PI_4.to_bits());
    }

    #[test]
    fn cone_construction_rejects_zero_half_angle() {
        let err = Cone::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            0.0,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::InvalidHalfAngle);
    }

    #[test]
    fn cone_construction_rejects_half_angle_equals_pi_over_2() {
        let err = Cone::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            FRAC_PI_2,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::InvalidHalfAngle);
    }

    #[test]
    fn cone_construction_rejects_degenerate_axis() {
        let err = Cone::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 0.0).unwrap(),
            FRAC_PI_4,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::DegenerateAxis);
    }

    #[test]
    fn cone_construction_rejects_dependent_axes() {
        let err = Cone::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            FRAC_PI_4,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::DependentAxes);
    }

    #[test]
    fn cone_construction_rejects_ill_conditioned_axes() {
        let err = Cone::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            FRAC_PI_4,
            Vector3::try_new(ILL_COND_THRESH / 2.0, 0.0, 1.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::IllConditionedAxes);
    }

    #[test]
    fn cone_evaluate_apex_derivative_singular() {
        // The apex ∂p/∂u = 0 singularity is a geometric fact independent of
        // trig certification, so it is still reported (and takes priority
        // over Uncertified).
        let c = std_cone();
        assert_eq!(
            c.evaluate(0.0, 0.0, DerivativeOrder::First, &tol()),
            Err(GeometryError::Singular)
        );
        assert_eq!(
            c.evaluate(0.0, 0.0, DerivativeOrder::Second, &tol()),
            Err(GeometryError::Singular)
        );
    }

    #[test]
    fn cone_evaluate_returns_uncertified_pending_trig() {
        // No pure-Rust, WASM-compatible, formally-proved correctly-rounded
        // sin/cos/tan implementation exists; evaluate() must be honest
        // about this rather than assert an unproven bound.
        let c = std_cone();
        let err = c
            .evaluate(0.0, 1.0, DerivativeOrder::Position, &tol())
            .unwrap_err();
        assert_uncertified(&err);
        let err = c
            .evaluate(0.0, 1.0, DerivativeOrder::Second, &tol())
            .unwrap_err();
        assert_uncertified(&err);
    }

    #[test]
    fn cone_evaluate_rejects_out_of_domain() {
        let c = std_cone();
        assert_eq!(
            c.evaluate(-0.001, 1.0, DerivativeOrder::Position, &tol()),
            Err(GeometryError::OutsideDomain)
        );
        assert_eq!(
            c.evaluate(TAU, 1.0, DerivativeOrder::Position, &tol()),
            Err(GeometryError::OutsideDomain)
        );
    }

    #[test]
    fn cone_evaluate_rejects_non_finite() {
        let c = std_cone();
        assert_eq!(
            c.evaluate(f64::NAN, 1.0, DerivativeOrder::Position, &tol()),
            Err(GeometryError::NonFiniteParameter)
        );
        assert_eq!(
            c.evaluate(0.0, f64::INFINITY, DerivativeOrder::Position, &tol()),
            Err(GeometryError::NonFiniteParameter)
        );
    }

    #[test]
    fn cone_project_returns_uncertified_pending_trig() {
        // u = atan2(...), nappe disambiguation, and sin/cos/tan
        // reconstruction are all uncertified std transcendentals;
        // project_into must report Uncertified rather than a bound it
        // cannot support.
        let c = std_cone();
        let q = Point3::try_new(2.0, 0.0, 3.0).unwrap();
        let err = c.project(q, &tol()).unwrap_err();
        assert_uncertified(&err);
    }

    #[test]
    fn cone_project_into_clears_output_on_error() {
        let c = std_cone();
        let mut output = vec![];
        let err = c.project_into(Point3::try_new(1.0, 0.0, 1.0).unwrap(), &tol(), &mut output);
        assert_uncertified(&err.unwrap_err());
        assert!(output.is_empty());
    }

    #[test]
    fn cone_serde_round_trip() {
        let c = Cone::try_new(
            Point3::try_new(1.0, 2.0, 3.0).unwrap(),
            Vector3::try_new(
                1.0 / 3.0_f64.sqrt(),
                1.0 / 3.0_f64.sqrt(),
                1.0 / 3.0_f64.sqrt(),
            )
            .unwrap(),
            0.5,
            Vector3::try_new(1.0 / 2.0_f64.sqrt(), -1.0 / 2.0_f64.sqrt(), 0.0).unwrap(),
        )
        .unwrap();
        let json = serde_json::to_string(&c).unwrap();
        let decoded: Cone = serde_json::from_str(&json).unwrap();
        assert_eq!(c, decoded);
    }

    #[test]
    fn cone_serde_rejects_bad_axis_angle_and_orthogonality() {
        let bad_axis: ConeRepr = serde_json::from_value(json!({
            "apex": [1.0, 2.0, 3.0],
            "axis": [2.0, 0.0, 0.0],
            "half_angle": 0.5,
            "x_axis": [0.0, 1.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Cone::try_from(bad_axis),
            Err(ConstructionError::DegenerateAxis)
        );

        let bad_frame: ConeRepr = serde_json::from_value(json!({
            "apex": [1.0, 2.0, 3.0],
            "axis": [1.0, 0.0, 0.0],
            "half_angle": 0.5,
            "x_axis": [1.0, 0.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Cone::try_from(bad_frame),
            Err(ConstructionError::DependentAxes)
        );

        let bad_angle: ConeRepr = serde_json::from_value(json!({
            "apex": [1.0, 2.0, 3.0],
            "axis": [0.0, 0.0, 1.0],
            "half_angle": 0.0,
            "x_axis": [1.0, 0.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Cone::try_from(bad_angle),
            Err(ConstructionError::InvalidHalfAngle)
        );
    }

    #[test]
    fn cone_serde_rejects_nan_and_inf_fields() {
        assert!(serde_json::from_str::<Cone>(
            r#"{"apex":[NaN,0.0,0.0],"axis":[0.0,0.0,1.0],"half_angle":0.5,"x_axis":[1.0,0.0,0.0]}"#
        )
        .is_err());
        assert!(serde_json::from_str::<Cone>(
            r#"{"apex":[0.0,0.0,0.0],"axis":[Infinity,0.0,1.0],"half_angle":0.5,"x_axis":[1.0,0.0,0.0]}"#
        )
        .is_err());
    }

    #[test]
    fn cone_try_transform_identity_is_noop() {
        let c = std_cone();
        let out = c
            .try_transform(&amphion_foundation::Transform3::IDENTITY)
            .unwrap();
        assert_eq!(out, c);
    }

    #[test]
    fn cone_try_transform_similarity_preserves_half_angle() {
        // Rotation by 90° about Z, uniform scale 2, plus translation.
        let m = [
            0.0, -2.0, 0.0, 5.0, //
            2.0, 0.0, 0.0, -3.0, //
            0.0, 0.0, 2.0, 7.0,
        ];
        let t = amphion_foundation::Transform3::try_from_row_major(m).unwrap();
        let c = std_cone();
        let out = c.try_transform(&t).unwrap();
        assert!((out.half_angle() - FRAC_PI_4).abs() < 1e-9);
        let [ax, ay, az] = out.apex().into_array();
        assert!((ax - 5.0).abs() < 1e-9);
        assert!((ay - (-3.0)).abs() < 1e-9);
        assert!((az - 7.0).abs() < 1e-9);
    }

    #[test]
    fn cone_try_transform_rejects_non_similarity() {
        let m = [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 2.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0,
        ];
        let t = amphion_foundation::Transform3::try_from_row_major(m).unwrap();
        let c = std_cone();
        assert_eq!(
            c.try_transform(&t),
            Err(super::TransformError::NotSimilarity)
        );
    }
}
