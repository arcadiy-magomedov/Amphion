//! Private arithmetic helpers operating on raw `f64` arrays.
//!
//! All operations are infallible on the hot path. Callers must validate
//! finiteness at public boundaries and, when wrapping results in foundation
//! types, call [`crate::GeometryError`]-returning helpers from the parent
//! module.
//!
//! # Certified bounds are arithmetic-only
//!
//! [`arithmetic_proj_bound2`], [`arithmetic_proj_bound3`], and
//! [`arithmetic_eval_bound`] derive a certified upper bound on floating-point
//! error for computations that use **only** IEEE 754 basic operations
//! (addition, subtraction, multiplication, division, and `sqrt`), all of
//! which are correctly rounded per IEEE 754-2008/2019 §5.4. Under Higham
//! (2002), *Accuracy and Stability of Numerical Algorithms*, 2nd ed., SIAM,
//! Theorem 3.5 and §2.2: for a computation composed of `n` such operations,
//! `|fl(x) − x| ≤ γ_n · |x|` where `γ_n = n·ε / (1 − n·ε)`. Line and plane
//! evaluation/projection involve at most `~16` elementary operations
//! (dot products, additions, subtractions, one `sqrt` for magnitude); we use
//! `γ_32 ≤ 64·ε` as a conservative round-number bound covering both the 2-D
//! and 3-D cases with margin.
//!
//! **This bound is only valid for arithmetic-only computations.** No
//! certified, WASM-compatible, formally-proved correctly-rounded `sin`,
//! `cos`, or `atan2` implementation currently exists in the Rust ecosystem:
//!
//! - `libm` (MIT, pure Rust, WASM-compatible): empirically ~1–2 ULP, but
//!   **not formally proved** correctly rounded.
//! - `core-math` (MIT, 0.5 ULP correctly rounded): requires `fenv.h` C FFI
//!   for directed-rounding control, **not WASM-compatible**.
//! - `inari` / `rug` (interval arithmetic via MPFR): require GMP/MPFR C
//!   libraries, **not WASM-compatible**.
//! - `inari_wasm`: calls `f64::sin` directly without directed rounding, so
//!   it is **not rigorous** as an interval implementation.
//! - IEEE 754-2019 §9.2 only *recommends* (does not *require*)
//!   correctly-rounded `sin`/`cos`/`atan2`, so no portable error bound on
//!   these functions can be assumed by a certified kernel.
//!
//! Consequently, every evaluator whose formula involves `sin`, `cos`, or
//! `atan2` (`Circle2`, `Circle3`, `Cylinder`, `Cone` — both `evaluate()` and
//! `project_into()`) returns [`GeometryError::Uncertified`] instead of a
//! numeric bound, until a formally-proved WASM-compatible trigonometric
//! implementation is integrated.
//!
//! **Rejection criterion**: for arithmetic-only bounds, if the certified
//! floating-point error allowance exceeds `tolerance.effective_length(scale)`
//! at the relevant world-coordinate scale, we return
//! `GeometryError::Uncertified` rather than assert a bound we cannot
//! support.

use amphion_foundation::{Interval, NormalizationError, ToleranceContext, Vector2, Vector3};

use crate::{DistanceBound, GeometryError, ParameterRange};

use super::ConstructionError;

/// Serde validation tolerance: accept vectors whose squared-norm differs from
/// 1 by at most `4ε`, matching the magnitude-deviation guarantee of the
/// `UnitVector2`/`UnitVector3` types introduced in foundation commit
/// `670516d`. Honest round-tripped values produced by this module's own
/// normalization have `||v||² − 1| ≤ 2ε`, so they pass with a `2×` margin;
/// clearly corrupted or hand-crafted non-unit vectors are rejected.
pub(super) const UNIT_VECTOR_TOL: f64 = 4.0 * f64::EPSILON;

/// Converts a foundation [`NormalizationError`] into the analytic
/// geometry crate's own [`ConstructionError`].
///
/// `NonFinite` and `NotNormalized` both indicate the supplied vector cannot
/// be trusted as-is, which this crate reports as
/// [`ConstructionError::NonFiniteInput`]; `ZeroMagnitude` maps to
/// [`ConstructionError::DegenerateAxis`], matching the prior local
/// `normalize2`/`normalize3` behavior of returning `None` only for the zero
/// vector.
pub(super) fn normalization_to_construction(err: NormalizationError) -> ConstructionError {
    match err {
        NormalizationError::NonFinite | NormalizationError::NotNormalized => {
            ConstructionError::NonFiniteInput
        }
        NormalizationError::ZeroMagnitude => ConstructionError::DegenerateAxis,
    }
}

/// Ill-conditioning threshold for Gram-Schmidt orthogonalization.  If the
/// component of the supplied x-axis perpendicular to the main axis has
/// magnitude below this value (`16 · √ε ≈ 2.4e-7`), the normalization would
/// amplify rounding errors by a factor of `> 1/√ε ≈ 6.7e7` and the result
/// would be unreliable.
pub(super) const ILL_COND_THRESH: f64 = 2.384_185_791_015_625e-7;

/// Certified Higham `γ_32` bound (see module-level documentation) for
/// arithmetic-only (no-trig) computations: `γ_32 = 32ε/(1−32ε) ≤ 64ε`.
///
/// Valid ONLY for computations built exclusively from IEEE 754 basic
/// operations (add, sub, mul, div, sqrt). Never apply to a computation that
/// calls `sin`, `cos`, or `atan2`.
const GAMMA_32: f64 = 64.0 * f64::EPSILON;

/// Higham `γ_64` bound used for evaluation error, which additionally accounts
/// for the ≤ `UNIT_VECTOR_TOL` per-component deviation of a stored direction
/// vector from a true unit vector. Still arithmetic-only; still ≤ a small
/// constant multiple of ε.
const GAMMA_64: f64 = 128.0 * f64::EPSILON;

// ─── 3-D helpers ────────────────────────────────────────────────────────────

pub(super) fn dot3(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

pub(super) fn mag3(v: [f64; 3]) -> f64 {
    let scale = v[0].abs().max(v[1].abs()).max(v[2].abs());
    if scale == 0.0 {
        0.0
    } else {
        let vs = [v[0] / scale, v[1] / scale, v[2] / scale];
        scale * (vs[0] * vs[0] + vs[1] * vs[1] + vs[2] * vs[2]).sqrt()
    }
}

pub(super) fn add3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}

pub(super) fn sub3(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

pub(super) fn scale3(v: [f64; 3], s: f64) -> [f64; 3] {
    [v[0] * s, v[1] * s, v[2] * s]
}

pub(super) fn all_finite3(v: [f64; 3]) -> bool {
    v[0].is_finite() && v[1].is_finite() && v[2].is_finite()
}

// ─── 2-D helpers ────────────────────────────────────────────────────────────

pub(super) fn dot2(a: [f64; 2], b: [f64; 2]) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}

pub(super) fn mag2(v: [f64; 2]) -> f64 {
    let scale = v[0].abs().max(v[1].abs());
    if scale == 0.0 {
        0.0
    } else {
        let vs = [v[0] / scale, v[1] / scale];
        scale * (vs[0] * vs[0] + vs[1] * vs[1]).sqrt()
    }
}

pub(super) fn add2(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] + b[0], a[1] + b[1]]
}

pub(super) fn sub2(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0] - b[0], a[1] - b[1]]
}

pub(super) fn scale2(v: [f64; 2], s: f64) -> [f64; 2] {
    [v[0] * s, v[1] * s]
}

/// Rotates `v` by 90° CCW, giving the perpendicular direction.
pub(super) fn perp2(v: [f64; 2]) -> [f64; 2] {
    [-v[1], v[0]]
}

pub(super) fn all_finite2(v: [f64; 2]) -> bool {
    v[0].is_finite() && v[1].is_finite()
}

// ─── Domain helpers ──────────────────────────────────────────────────────────

/// Returns `true` when `t` is finite and inside the declared parameter range.
///
/// For a periodic domain `[lower, upper)` the upper bound is **exclusive**.
/// For a bounded non-periodic domain `[lower, upper]` both bounds are inclusive.
/// For a half-open or infinite domain the absent bound is unchecked.
pub(super) fn in_range(t: f64, range: ParameterRange) -> bool {
    if !t.is_finite() {
        return false;
    }
    let lo_ok = range.lower().is_none_or(|lo| t >= lo);
    let hi_ok = range.upper().is_none_or(|hi| {
        if range.period().is_some() {
            t < hi
        } else {
            t <= hi
        }
    });
    lo_ok && hi_ok
}

/// Certified upper bound on `|query − projection|` for a LINE or PLANE
/// projection in 2-D (no trig involved), and checks that the certified
/// floating-point error allowance fits within `tolerance`.
///
/// # Derivation (Higham 2002, §2.2, Theorem 3.5)
///
/// Line/plane projection involves only IEEE 754 basic operations (add, sub,
/// mul, sqrt — all correctly rounded by IEEE 754-2008/2019 mandate). For `n`
/// elementary operations: `|fl(x) − x| ≤ γ_n · |x|` where
/// `γ_n = n·ε / (1 − n·ε)`. 2-D line/plane projection plus residual
/// magnitude involves `≤ 16` such operations, so `γ_16 ≤ 32ε`; we use the
/// conservative round number `γ_32 ≤ 64ε` ([`GAMMA_32`]) to cover both the
/// 2-D and 3-D cases with margin.
///
/// The characteristic scale `scale = |query| + |projection|` is a
/// world-coordinate magnitude (not translation-invariant): it bounds every
/// intermediate magnitude the projection arithmetic can plausibly produce,
/// so the resulting `fp_err` allowance is valid regardless of where `query`
/// sits relative to the primitive's anchor.
///
/// This function is **only** valid for arithmetic-only computations (lines,
/// planes). Functions involving `sin`/`cos`/`atan2` must return
/// [`GeometryError::Uncertified`] directly instead of calling this helper.
///
/// # Errors
///
/// Returns [`GeometryError::Uncertified`] if the certified FP error
/// allowance exceeds `tolerance.effective_length(scale)`, i.e. the
/// primitive's scale is too large relative to the requested tolerance for
/// `f64` arithmetic to certify a bound.
pub(super) fn arithmetic_proj_bound2(
    query: [f64; 2],
    projection: [f64; 2],
    tolerance: &ToleranceContext,
) -> Result<f64, GeometryError> {
    // Use Vector2 checked magnitude for scale-safe residual computation.
    let diff = [query[0] - projection[0], query[1] - projection[1]];
    let diff_vec = Vector2::try_new(diff[0], diff[1]).map_err(|_| GeometryError::Uncertified {
        reason: "projection difference is non-finite".to_owned(),
    })?;
    let residual = diff_vec
        .try_magnitude()
        .map_err(|_| GeometryError::Uncertified {
            reason: "projection residual overflowed".to_owned(),
        })?;

    // World-coordinate scale for Higham error bound.
    let q_vec = Vector2::try_new(query[0], query[1]).map_err(|_| GeometryError::Uncertified {
        reason: "query is non-finite".to_owned(),
    })?;
    let p_vec =
        Vector2::try_new(projection[0], projection[1]).map_err(|_| GeometryError::Uncertified {
            reason: "projection is non-finite".to_owned(),
        })?;
    let q_mag = q_vec
        .try_magnitude()
        .map_err(|_| GeometryError::Uncertified {
            reason: "query magnitude overflowed".to_owned(),
        })?;
    let p_mag = p_vec
        .try_magnitude()
        .map_err(|_| GeometryError::Uncertified {
            reason: "projection magnitude overflowed".to_owned(),
        })?;
    let scale = q_mag + p_mag;
    let fp_err = GAMMA_32 * scale;

    // Use Interval::point + widen for outward-rounded final bound.
    // Interval::widen uses next_down/next_up internally (stabilized Rust 1.87).
    let bound = Interval::point(residual)
        .map_err(|_| GeometryError::Uncertified {
            reason: "residual is non-finite for interval bound".to_owned(),
        })?
        .widen(fp_err)
        .map_err(|_| GeometryError::Uncertified {
            reason: "certified distance bound overflowed representable range".to_owned(),
        })?
        .hi();

    let eff_tol = tolerance
        .effective_length(scale)
        .map_err(|_| GeometryError::Uncertified {
            reason: "world scale is invalid for tolerance computation".to_owned(),
        })?;
    if fp_err > eff_tol {
        return Err(GeometryError::Uncertified {
            reason: "f64 coordinate granularity exceeds requested tolerance at this world \
                     scale; consider reducing the coordinate magnitude or relaxing the tolerance"
                .to_owned(),
        });
    }
    Ok(bound)
}

/// Certified upper bound on `|query − projection|` for a LINE or PLANE
/// projection in 3-D.  See [`arithmetic_proj_bound2`] for the derivation.
pub(super) fn arithmetic_proj_bound3(
    query: [f64; 3],
    projection: [f64; 3],
    tolerance: &ToleranceContext,
) -> Result<f64, GeometryError> {
    // Use Vector3 checked magnitude for scale-safe residual computation.
    let diff = [
        query[0] - projection[0],
        query[1] - projection[1],
        query[2] - projection[2],
    ];
    let diff_vec =
        Vector3::try_new(diff[0], diff[1], diff[2]).map_err(|_| GeometryError::Uncertified {
            reason: "projection difference is non-finite".to_owned(),
        })?;
    let residual = diff_vec
        .try_magnitude()
        .map_err(|_| GeometryError::Uncertified {
            reason: "projection residual overflowed".to_owned(),
        })?;

    // World-coordinate scale for Higham error bound.
    let q_vec =
        Vector3::try_new(query[0], query[1], query[2]).map_err(|_| GeometryError::Uncertified {
            reason: "query is non-finite".to_owned(),
        })?;
    let p_vec = Vector3::try_new(projection[0], projection[1], projection[2]).map_err(|_| {
        GeometryError::Uncertified {
            reason: "projection is non-finite".to_owned(),
        }
    })?;
    let q_mag = q_vec
        .try_magnitude()
        .map_err(|_| GeometryError::Uncertified {
            reason: "query magnitude overflowed".to_owned(),
        })?;
    let p_mag = p_vec
        .try_magnitude()
        .map_err(|_| GeometryError::Uncertified {
            reason: "projection magnitude overflowed".to_owned(),
        })?;
    let scale = q_mag + p_mag;
    let fp_err = GAMMA_32 * scale;

    // Use Interval::point + widen for outward-rounded final bound.
    let bound = Interval::point(residual)
        .map_err(|_| GeometryError::Uncertified {
            reason: "residual is non-finite for interval bound".to_owned(),
        })?
        .widen(fp_err)
        .map_err(|_| GeometryError::Uncertified {
            reason: "certified distance bound overflowed representable range".to_owned(),
        })?
        .hi();

    let eff_tol = tolerance
        .effective_length(scale)
        .map_err(|_| GeometryError::Uncertified {
            reason: "world scale is invalid for tolerance computation".to_owned(),
        })?;
    if fp_err > eff_tol {
        return Err(GeometryError::Uncertified {
            reason: "f64 coordinate granularity exceeds requested tolerance at this world scale"
                .to_owned(),
        });
    }
    Ok(bound)
}

/// Certified upper bound on `‖evaluated_position − true_p(t)‖` for
/// arithmetic-only evaluators (`Line2`/`Line3`, `Plane`).
///
/// Uses the [`GAMMA_64`] bound, which combines the `γ_32` arithmetic-error
/// bound with an allowance for the `UNIT_VECTOR_TOL` per-component deviation
/// of a stored direction vector from a true unit vector. `eval_scale` is the
/// world-coordinate magnitude of the evaluation result (e.g.
/// `|origin| + |t · direction|`).
///
/// # Errors
///
/// Returns [`GeometryError::Uncertified`] if `eval_scale` is not finite, so
/// the bound itself cannot be certified as finite and non-negative.
pub(super) fn arithmetic_eval_bound(eval_scale: f64) -> Result<DistanceBound, GeometryError> {
    if !eval_scale.is_finite() {
        return Err(GeometryError::Uncertified {
            reason: "evaluation scale is non-finite".to_owned(),
        });
    }
    let raw = GAMMA_64 * eval_scale.abs();
    // Use next_up() for outward rounding (sqrt is correctly rounded, so
    // the arithmetic bound via Higham is already valid; one next_up adds
    // the final-addition rounding).
    let bound = raw.next_up().max(0.0);
    DistanceBound::try_new(bound).map_err(|_| GeometryError::Uncertified {
        reason: "arithmetic evaluation error bound overflowed representable range".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use amphion_foundation::{NormalizationError, ToleranceContext, Vector2, Vector3};

    use super::{
        UNIT_VECTOR_TOL, arithmetic_eval_bound, arithmetic_proj_bound2, arithmetic_proj_bound3,
        normalization_to_construction,
    };
    use crate::analytic::ConstructionError;

    #[test]
    fn normalization_to_construction_maps_zero_magnitude_to_degenerate_axis() {
        assert_eq!(
            normalization_to_construction(NormalizationError::ZeroMagnitude),
            ConstructionError::DegenerateAxis
        );
    }

    #[test]
    fn normalization_to_construction_maps_non_finite_to_non_finite_input() {
        assert_eq!(
            normalization_to_construction(NormalizationError::NonFinite),
            ConstructionError::NonFiniteInput
        );
    }

    #[test]
    fn normalization_to_construction_maps_not_normalized_to_non_finite_input() {
        assert_eq!(
            normalization_to_construction(NormalizationError::NotNormalized),
            ConstructionError::NonFiniteInput
        );
    }

    #[test]
    fn arithmetic_eval_bound_rejects_non_finite_scale() {
        assert!(arithmetic_eval_bound(f64::NAN).is_err());
        assert!(arithmetic_eval_bound(f64::INFINITY).is_err());
    }

    #[test]
    fn arithmetic_eval_bound_is_non_negative_and_outward_rounded() {
        // next_up() on a positive value strictly increases it, so the
        // returned bound must be >= the raw Higham product.
        let bound = arithmetic_eval_bound(1.0).unwrap();
        assert!(bound.get() >= 0.0);
        assert!(bound.get() >= 128.0 * f64::EPSILON);
    }

    fn tol() -> ToleranceContext {
        ToleranceContext::try_new(1e-9, 1e-8, 1e-10, 1e-12).unwrap()
    }

    #[test]
    fn arithmetic_proj_bound2_certifies_zero_residual() {
        let bound = arithmetic_proj_bound2([1.0, 2.0], [1.0, 2.0], &tol()).unwrap();
        assert!(bound >= 0.0);
    }

    #[test]
    fn arithmetic_proj_bound3_certifies_zero_residual() {
        let bound = arithmetic_proj_bound3([1.0, 2.0, 3.0], [1.0, 2.0, 3.0], &tol()).unwrap();
        assert!(bound >= 0.0);
    }

    #[test]
    fn unit_vector_tol_matches_foundation_deviation_guarantee() {
        // UnitVector2/UnitVector3 guarantee a magnitude within 4*EPSILON of
        // 1.0 (see `amphion_foundation::unit`); this crate's own tolerance
        // for downstream orthogonality/consistency checks matches it.
        assert!((UNIT_VECTOR_TOL - 4.0 * f64::EPSILON).abs() < f64::EPSILON);
        let _ = Vector2::try_new(1.0, 0.0).unwrap();
        let _ = Vector3::try_new(1.0, 0.0, 0.0).unwrap();
    }
}
