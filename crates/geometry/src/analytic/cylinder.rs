//! Analytic right circular cylinder surface.
//!
//! # Parameterization
//!
//! ```text
//! p(u, v) = axis_origin + v·axis_dir + r·cos(u)·x_axis + r·sin(u)·y_axis
//! ```
//!
//! where `y_axis = axis_dir × x_axis`.  `axis_dir`, `x_axis`, and `y_axis`
//! form a right-handed orthonormal frame.
//!
//! - U domain: `[0, 2π)` with period `2π` (angular, CCW around `axis_dir`)
//! - V domain: `(−∞, +∞)` (axial)
//!
//! Derivatives:
//! ```text
//! ∂p/∂u  =  r·(−sin u·x_axis + cos u·y_axis)
//! ∂p/∂v  =  axis_dir
//! ∂²p/∂u²  =  −r·(cos u·x_axis + sin u·y_axis)
//! ∂²p/∂u∂v  =  0
//! ∂²p/∂v²   =  0
//! ```
//!
//! Projection: decompose `q − axis_origin` into axial and radial components;
//! `v` is the axial component and `u` is the angle of the radial direction.
//! Returns [`GeometryError::Singular`] when the radial component is exactly
//! zero (point on the cylinder axis).

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
struct CylinderRepr {
    axis_origin: Point3,
    axis_dir: Vector3,
    radius: f64,
    x_axis: Vector3,
}

/// A right circular cylinder surface.
///
/// Parameterization:
/// ```text
/// p(u, v) = axis_origin + v·axis_dir + r·cos(u)·x_axis + r·sin(u)·y_axis
/// ```
/// U ∈ `[0, 2π)` (periodic), V ∈ `(−∞, +∞)` (axial).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(try_from = "CylinderRepr", into = "CylinderRepr")]
pub struct Cylinder {
    axis_origin: Point3,
    axis_dir: Vector3,
    radius: f64,
    x_axis: Vector3,
    y_axis: Vector3,
}

impl Cylinder {
    /// Constructs a cylinder.
    ///
    /// `axis_dir` and `x_axis` are normalized internally.  `x_axis` is
    /// orthogonalized against `axis_dir` (Gram-Schmidt).
    ///
    /// # Errors
    ///
    /// - [`ConstructionError::NonFiniteInput`] — any NaN/Inf input
    /// - [`ConstructionError::DegenerateAxis`] — zero-length axis or x-axis
    /// - [`ConstructionError::NotPositive`] — `radius <= 0`
    /// - [`ConstructionError::DependentAxes`] — `x_axis` parallel to `axis_dir`
    /// - [`ConstructionError::IllConditionedAxes`] — `x_axis` nearly parallel
    ///   to `axis_dir`
    pub fn try_new(
        axis_origin: Point3,
        axis_dir: Vector3,
        radius: f64,
        x_axis: Vector3,
    ) -> Result<Self, ConstructionError> {
        let o = axis_origin.into_array();
        let a = axis_dir.into_array();
        let x = x_axis.into_array();
        if !all_finite3(o) || !all_finite3(a) || !radius.is_finite() || !all_finite3(x) {
            return Err(ConstructionError::NonFiniteInput);
        }
        if radius <= 0.0 {
            return Err(ConstructionError::NotPositive);
        }
        let a_unit = normalize3(a).ok_or(ConstructionError::DegenerateAxis)?;
        let x_norm = normalize3(x).ok_or(ConstructionError::DegenerateAxis)?;
        // Orthogonalize x against axis_dir.
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
            axis_origin: Point3::try_new(o[0], o[1], o[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            axis_dir: Vector3::try_new(a_unit[0], a_unit[1], a_unit[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            radius,
            x_axis: Vector3::try_new(x_unit[0], x_unit[1], x_unit[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
            y_axis: Vector3::try_new(y_arr[0], y_arr[1], y_arr[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
        })
    }

    /// Returns the axis origin.
    #[must_use]
    pub fn axis_origin(&self) -> Point3 {
        self.axis_origin
    }

    /// Returns the unit axis direction.
    #[must_use]
    pub fn axis_dir(&self) -> Vector3 {
        self.axis_dir
    }

    /// Returns the radius.
    #[must_use]
    pub fn radius(&self) -> f64 {
        self.radius
    }

    /// Returns the unit reference direction for `u = 0`.
    #[must_use]
    pub fn x_axis(&self) -> Vector3 {
        self.x_axis
    }

    /// Returns the unit y-axis: `axis_dir × x_axis`.
    #[must_use]
    pub fn y_axis(&self) -> Vector3 {
        self.y_axis
    }

    /// Applies a similarity `transform` (rigid motion plus uniform scale, no
    /// reflection) to this cylinder, returning a new cylinder whose radius
    /// is scaled accordingly.
    ///
    /// A general affine transform does not map a circular cylinder to a
    /// circular cylinder, so only similarity transforms are accepted; see
    /// the `transform` module documentation for the (provisional, heuristic)
    /// similarity test.
    ///
    /// # Errors
    ///
    /// - [`TransformError::NotSimilarity`] — the transform's linear part is
    ///   not (within tolerance) a uniform-scale rotation
    /// - [`TransformError::NonFiniteResult`] — the transformed axis origin
    ///   or axes contain a non-finite component
    /// - [`TransformError::DegenerateResult`] — the transformed axes or
    ///   scaled radius fail cylinder construction
    pub fn try_transform(&self, transform: &Transform3) -> Result<Self, TransformError> {
        let m = transform.into_row_major();
        let scale = similarity_scale(m).ok_or(TransformError::NotSimilarity)?;
        let o = apply_to_point(m, self.axis_origin.into_array())
            .ok_or(TransformError::NonFiniteResult)?;
        let a = apply_to_vector(m, self.axis_dir.into_array())
            .ok_or(TransformError::NonFiniteResult)?;
        let x =
            apply_to_vector(m, self.x_axis.into_array()).ok_or(TransformError::NonFiniteResult)?;
        let new_radius = self.radius * scale;
        Self::try_new(
            Point3::try_new(o[0], o[1], o[2]).map_err(|_| TransformError::NonFiniteResult)?,
            Vector3::try_new(a[0], a[1], a[2]).map_err(|_| TransformError::NonFiniteResult)?,
            new_radius,
            Vector3::try_new(x[0], x[1], x[2]).map_err(|_| TransformError::NonFiniteResult)?,
        )
        .map_err(|_| TransformError::DegenerateResult)
    }
}

impl TryFrom<CylinderRepr> for Cylinder {
    type Error = ConstructionError;
    fn try_from(repr: CylinderRepr) -> Result<Self, Self::Error> {
        let axis_origin = repr.axis_origin.into_array();
        let axis_dir = repr.axis_dir.into_array();
        let x_axis = repr.x_axis.into_array();
        if !all_finite3(axis_origin)
            || !all_finite3(axis_dir)
            || !repr.radius.is_finite()
            || !all_finite3(x_axis)
        {
            return Err(ConstructionError::NonFiniteInput);
        }
        if repr.radius <= 0.0 {
            return Err(ConstructionError::NotPositive);
        }
        validate_unit3(axis_dir)?;
        validate_unit3(x_axis)?;
        validate_orthogonal3(axis_dir, x_axis)?;
        let y_arr = cross3(axis_dir, x_axis);
        Ok(Self {
            axis_origin: repr.axis_origin,
            axis_dir: repr.axis_dir,
            radius: repr.radius,
            x_axis: repr.x_axis,
            y_axis: Vector3::try_new(y_arr[0], y_arr[1], y_arr[2])
                .map_err(|_| ConstructionError::NonFiniteInput)?,
        })
    }
}

impl From<Cylinder> for CylinderRepr {
    fn from(c: Cylinder) -> Self {
        Self {
            axis_origin: c.axis_origin,
            axis_dir: c.axis_dir,
            radius: c.radius,
            x_axis: c.x_axis,
        }
    }
}

impl SurfaceEvaluator for Cylinder {
    fn kind(&self) -> SurfaceKind {
        SurfaceKind::Cylinder
    }

    fn domain(&self) -> SurfaceDomain {
        SurfaceDomain::new(angular_range(), unbounded_range())
    }

    fn evaluate(
        &self,
        u: f64,
        v: f64,
        _order: DerivativeOrder,
        _tolerance: &ToleranceContext,
    ) -> Result<SurfaceEvaluation, GeometryError> {
        if !u.is_finite() || !v.is_finite() {
            return Err(GeometryError::NonFiniteParameter);
        }
        if !in_range(u, self.domain().u()) {
            return Err(GeometryError::OutsideDomain);
        }
        // p(u, v) = axis_origin + v·axis_dir + r·cos(u)·x_axis + r·sin(u)·y_axis
        // requires `cos` and `sin`. No pure-Rust, WASM-compatible,
        // formally-proved correctly-rounded implementation of these
        // functions currently exists (see the `analytic::helpers` module
        // docs for the survey of candidates), so no certified error bound
        // can be produced.
        Err(GeometryError::Uncertified {
            reason: "cylinder evaluation requires certified sin/cos; no formally-proved \
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
        // u = atan2(...) is an uncertified std transcendental; the
        // sin(u)/cos(u) reconstruction of the projected point is also
        // uncertified. See the `analytic::helpers` module docs.
        Err(GeometryError::Uncertified {
            reason: "cylinder projection requires certified atan2/sin/cos; pending certified \
                     trig integration. See: libm crate (empirical accuracy only), core-math \
                     (0.5 ULP, not WASM-compatible)."
                .to_owned(),
        })
    }
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use amphion_foundation::{Point3, ToleranceContext, Vector3};
    use serde_json::json;

    use crate::analytic::helpers::ILL_COND_THRESH;
    use crate::traits::SurfaceEvaluator;
    use crate::{DerivativeOrder, GeometryError};

    use super::{ConstructionError, Cylinder, CylinderRepr};

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

    fn unit_cylinder() -> Cylinder {
        Cylinder::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            1.0,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap()
    }

    #[test]
    fn cylinder_construction_valid() {
        assert!(unit_cylinder().radius() > 0.0);
    }

    #[test]
    fn cylinder_construction_rejects_zero_radius() {
        let err = Cylinder::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            0.0,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::NotPositive);
    }

    #[test]
    fn cylinder_construction_rejects_degenerate_axis() {
        let err = Cylinder::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 0.0).unwrap(),
            1.0,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::DegenerateAxis);
    }

    #[test]
    fn cylinder_construction_rejects_dependent_axes() {
        // x_axis parallel to axis_dir
        let err = Cylinder::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            1.0,
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::DependentAxes);
    }

    #[test]
    fn cylinder_construction_rejects_ill_conditioned_axes() {
        let err = Cylinder::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            1.0,
            Vector3::try_new(ILL_COND_THRESH / 2.0, 0.0, 1.0).unwrap(),
        )
        .unwrap_err();
        assert_eq!(err, ConstructionError::IllConditionedAxes);
    }

    #[test]
    fn cylinder_y_axis_right_handed() {
        // axis_dir=+Z, x_axis=+X → y_axis = Z×X = +Y
        let c = unit_cylinder();
        let y = c.y_axis().into_array();
        assert!((y[0]).abs() < 1e-14 && (y[1] - 1.0).abs() < 1e-14 && y[2].abs() < 1e-14);
    }

    #[test]
    fn cylinder_evaluate_returns_uncertified_pending_trig() {
        // No pure-Rust, WASM-compatible, formally-proved correctly-rounded
        // sin/cos implementation exists; evaluate() must be honest about
        // this rather than assert an unproven bound.
        let c = unit_cylinder();
        let err = c
            .evaluate(0.0, 0.0, DerivativeOrder::Position, &tol())
            .unwrap_err();
        assert_uncertified(&err);
        let err = c
            .evaluate(0.0, 5.0, DerivativeOrder::Second, &tol())
            .unwrap_err();
        assert_uncertified(&err);
    }

    #[test]
    fn cylinder_evaluate_rejects_out_of_domain() {
        let c = unit_cylinder();
        assert_eq!(
            c.evaluate(-0.001, 0.0, DerivativeOrder::Position, &tol()),
            Err(GeometryError::OutsideDomain)
        );
        assert_eq!(
            c.evaluate(
                std::f64::consts::TAU,
                0.0,
                DerivativeOrder::Position,
                &tol()
            ),
            Err(GeometryError::OutsideDomain)
        );
    }

    #[test]
    fn cylinder_evaluate_rejects_non_finite() {
        let c = unit_cylinder();
        assert_eq!(
            c.evaluate(f64::NAN, 0.0, DerivativeOrder::Position, &tol()),
            Err(GeometryError::NonFiniteParameter)
        );
        assert_eq!(
            c.evaluate(0.0, f64::INFINITY, DerivativeOrder::Position, &tol()),
            Err(GeometryError::NonFiniteParameter)
        );
    }

    #[test]
    fn cylinder_project_returns_uncertified_pending_trig() {
        // u = atan2(...) and its sin/cos reconstruction are uncertified std
        // transcendentals; project_into must report Uncertified rather than
        // a bound it cannot support.
        let c = Cylinder::try_new(
            Point3::try_new(0.0, 0.0, 0.0).unwrap(),
            Vector3::try_new(0.0, 0.0, 1.0).unwrap(),
            3.0,
            Vector3::try_new(1.0, 0.0, 0.0).unwrap(),
        )
        .unwrap();
        let q = Point3::try_new(2.0, 0.0, 3.0).unwrap();
        let err = c.project(q, &tol()).unwrap_err();
        assert_uncertified(&err);
    }

    #[test]
    fn cylinder_project_into_clears_output_on_error() {
        let c = unit_cylinder();
        let mut output = vec![];
        let err = c.project_into(Point3::try_new(1.0, 0.0, 0.0).unwrap(), &tol(), &mut output);
        assert_uncertified(&err.unwrap_err());
        assert!(output.is_empty());
    }

    #[test]
    fn cylinder_serde_round_trip() {
        let c = Cylinder::try_new(
            Point3::try_new(1.0, 2.0, 3.0).unwrap(),
            Vector3::try_new(
                1.0 / 3.0_f64.sqrt(),
                1.0 / 3.0_f64.sqrt(),
                1.0 / 3.0_f64.sqrt(),
            )
            .unwrap(),
            2.5,
            Vector3::try_new(1.0 / 2.0_f64.sqrt(), -1.0 / 2.0_f64.sqrt(), 0.0).unwrap(),
        )
        .unwrap();
        let json = serde_json::to_string(&c).unwrap();
        let decoded: Cylinder = serde_json::from_str(&json).unwrap();
        assert_eq!(c, decoded);
    }

    #[test]
    fn cylinder_serde_rejects_bad_axis_radius_and_orthogonality() {
        let bad_axis: CylinderRepr = serde_json::from_value(json!({
            "axis_origin": [1.0, 2.0, 3.0],
            "axis_dir": [2.0, 0.0, 0.0],
            "radius": 2.5,
            "x_axis": [0.0, 1.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Cylinder::try_from(bad_axis),
            Err(ConstructionError::DegenerateAxis)
        );

        let bad_frame: CylinderRepr = serde_json::from_value(json!({
            "axis_origin": [1.0, 2.0, 3.0],
            "axis_dir": [1.0, 0.0, 0.0],
            "radius": 2.5,
            "x_axis": [1.0, 0.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Cylinder::try_from(bad_frame),
            Err(ConstructionError::DependentAxes)
        );

        let bad_radius: CylinderRepr = serde_json::from_value(json!({
            "axis_origin": [1.0, 2.0, 3.0],
            "axis_dir": [0.0, 0.0, 1.0],
            "radius": 0.0,
            "x_axis": [1.0, 0.0, 0.0]
        }))
        .unwrap();
        assert_eq!(
            Cylinder::try_from(bad_radius),
            Err(ConstructionError::NotPositive)
        );
    }

    #[test]
    fn cylinder_serde_rejects_nan_and_inf_fields() {
        assert!(serde_json::from_str::<Cylinder>(
            r#"{"axis_origin":[NaN,0.0,0.0],"axis_dir":[0.0,0.0,1.0],"radius":1.0,"x_axis":[1.0,0.0,0.0]}"#
        )
        .is_err());
        assert!(serde_json::from_str::<Cylinder>(
            r#"{"axis_origin":[0.0,0.0,0.0],"axis_dir":[Infinity,0.0,1.0],"radius":1.0,"x_axis":[1.0,0.0,0.0]}"#
        )
        .is_err());
    }

    #[test]
    fn cylinder_try_transform_identity_is_noop() {
        let c = unit_cylinder();
        let out = c
            .try_transform(&amphion_foundation::Transform3::IDENTITY)
            .unwrap();
        assert_eq!(out, c);
    }

    #[test]
    fn cylinder_try_transform_similarity_scales_radius() {
        // Rotation by 90° about Z, uniform scale 2, plus translation.
        let m = [
            0.0, -2.0, 0.0, 5.0, //
            2.0, 0.0, 0.0, -3.0, //
            0.0, 0.0, 2.0, 7.0,
        ];
        let t = amphion_foundation::Transform3::try_from_row_major(m).unwrap();
        let c = unit_cylinder();
        let out = c.try_transform(&t).unwrap();
        assert!((out.radius() - 2.0).abs() < 1e-9);
        let [ox, oy, oz] = out.axis_origin().into_array();
        assert!((ox - 5.0).abs() < 1e-9);
        assert!((oy - (-3.0)).abs() < 1e-9);
        assert!((oz - 7.0).abs() < 1e-9);
    }

    #[test]
    fn cylinder_try_transform_rejects_non_similarity() {
        let m = [
            1.0, 0.0, 0.0, 0.0, //
            0.0, 2.0, 0.0, 0.0, //
            0.0, 0.0, 1.0, 0.0,
        ];
        let t = amphion_foundation::Transform3::try_from_row_major(m).unwrap();
        let c = unit_cylinder();
        assert_eq!(
            c.try_transform(&t),
            Err(super::TransformError::NotSimilarity)
        );
    }
}
