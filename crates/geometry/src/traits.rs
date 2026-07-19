//! Thread-safe evaluator interfaces.
//!
//! # Certified rational-arithmetic backend
//!
//! `evaluate()`, `project()`, and `project_into()` accept an explicit
//! [`EvaluationContext`] (tolerance plus a [`crate::CertificationBudget`])
//! rather than a bare `ToleranceContext`. `CurveEvaluation2`,
//! `CurveEvaluation3`, and `SurfaceEvaluation` carry certified error-bound
//! fields (`position_error_bound: PositionBound`,
//! `first_error_bound`/`first_u_error_bound`/`first_v_error_bound: Option<FirstDerivativeBound>`,
//! `second_error_bound`/`second_uu_error_bound`/`second_uv_error_bound`/`second_vv_error_bound: Option<SecondDerivativeBound>`).
//! Evaluators must return [`GeometryError::Uncertified`] when the
//! implementation cannot bound the evaluation error within the supplied
//! tolerance or certification budget.
//!
//! Every geometry family — including the trig-dependent `Circle2`,
//! `Circle3`, `Cylinder`, and `Cone` — now returns a certified numeric
//! result rather than an unconditional `Uncertified`. Transcendental
//! quantities (angles) are computed via the certified rational-interval
//! backend in [`crate::analytic::trig`], which encloses `sin`, `cos`, and
//! `atan2` in exact `BigRational` intervals rather than relying on any
//! `f64` trigonometric implementation. See that module's documentation for
//! the algorithm references and the survey of rejected transcendental
//! backends.

use amphion_foundation::{Point2, Point3};

use crate::{
    CurveEvaluation2, CurveEvaluation3, CurveKind, CurveProjection2, CurveProjection3,
    DerivativeOrder, EvaluationContext, GeometryError, ParameterRange, SurfaceDomain,
    SurfaceEvaluation, SurfaceKind, SurfaceProjection,
};

/// A canonical parameter-space curve evaluator.
pub trait Curve2Evaluator: Send + Sync + 'static {
    /// Returns the curve family.
    fn kind(&self) -> CurveKind;

    /// Returns the declared parameter range.
    fn domain(&self) -> ParameterRange;

    /// Evaluates position and requested derivatives.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError`] for non-finite/out-of-domain parameters,
    /// singular evaluation, or an uncertified result.
    fn evaluate(
        &self,
        parameter: f64,
        order: DerivativeOrder,
        context: &EvaluationContext,
    ) -> Result<CurveEvaluation2, GeometryError>;

    /// Finds every certified projection inside the declared domain.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError`] when inverse mapping is unsupported,
    /// singular, or cannot certify all returned solutions.
    fn project(
        &self,
        point: Point2,
        context: &EvaluationContext,
    ) -> Result<Vec<CurveProjection2>, GeometryError> {
        let mut output = Vec::new();
        self.project_into(point, context, &mut output)?;
        Ok(output)
    }

    /// Writes every certified projection into a reusable output buffer.
    ///
    /// Implementations clear `output` before writing and leave it empty on
    /// error.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError`] when inverse mapping is unsupported,
    /// singular, or cannot certify all returned solutions.
    fn project_into(
        &self,
        point: Point2,
        context: &EvaluationContext,
        output: &mut Vec<CurveProjection2>,
    ) -> Result<(), GeometryError>;
}

/// A canonical model-space curve evaluator.
pub trait Curve3Evaluator: Send + Sync + 'static {
    /// Returns the curve family.
    fn kind(&self) -> CurveKind;

    /// Returns the declared parameter range.
    fn domain(&self) -> ParameterRange;

    /// Evaluates position and requested derivatives.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError`] for non-finite/out-of-domain parameters,
    /// singular evaluation, or an uncertified result.
    fn evaluate(
        &self,
        parameter: f64,
        order: DerivativeOrder,
        context: &EvaluationContext,
    ) -> Result<CurveEvaluation3, GeometryError>;

    /// Finds every certified projection inside the declared domain.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError`] when inverse mapping is unsupported,
    /// singular, or cannot certify all returned solutions.
    fn project(
        &self,
        point: Point3,
        context: &EvaluationContext,
    ) -> Result<Vec<CurveProjection3>, GeometryError> {
        let mut output = Vec::new();
        self.project_into(point, context, &mut output)?;
        Ok(output)
    }

    /// Writes every certified projection into a reusable output buffer.
    ///
    /// Implementations clear `output` before writing and leave it empty on
    /// error.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError`] when inverse mapping is unsupported,
    /// singular, or cannot certify all returned solutions.
    fn project_into(
        &self,
        point: Point3,
        context: &EvaluationContext,
        output: &mut Vec<CurveProjection3>,
    ) -> Result<(), GeometryError>;
}

/// A canonical model-space surface evaluator.
pub trait SurfaceEvaluator: Send + Sync + 'static {
    /// Returns the surface family.
    fn kind(&self) -> SurfaceKind;

    /// Returns the declared UV domain.
    fn domain(&self) -> SurfaceDomain;

    /// Evaluates position and requested derivatives.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError`] for non-finite/out-of-domain parameters,
    /// singular evaluation, or an uncertified result.
    fn evaluate(
        &self,
        u: f64,
        v: f64,
        order: DerivativeOrder,
        context: &EvaluationContext,
    ) -> Result<SurfaceEvaluation, GeometryError>;

    /// Finds every certified projection inside the declared domain.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError`] when inverse mapping is unsupported,
    /// singular, or cannot certify all returned solutions.
    fn project(
        &self,
        point: Point3,
        context: &EvaluationContext,
    ) -> Result<Vec<SurfaceProjection>, GeometryError> {
        let mut output = Vec::new();
        self.project_into(point, context, &mut output)?;
        Ok(output)
    }

    /// Writes every certified projection into a reusable output buffer.
    ///
    /// Implementations clear `output` before writing and leave it empty on
    /// error.
    ///
    /// # Errors
    ///
    /// Returns [`GeometryError`] when inverse mapping is unsupported,
    /// singular, or cannot certify all returned solutions.
    fn project_into(
        &self,
        point: Point3,
        context: &EvaluationContext,
        output: &mut Vec<SurfaceProjection>,
    ) -> Result<(), GeometryError>;
}

#[cfg(test)]
mod tests {
    use super::{Curve2Evaluator, Curve3Evaluator, SurfaceEvaluator};

    fn assert_send_sync_static<T: Send + Sync + 'static + ?Sized>() {}

    #[test]
    fn evaluator_contracts_are_thread_safe() {
        assert_send_sync_static::<dyn Curve2Evaluator>();
        assert_send_sync_static::<dyn Curve3Evaluator>();
        assert_send_sync_static::<dyn SurfaceEvaluator>();
    }
}
