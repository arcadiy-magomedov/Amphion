//! Thread-safe evaluator interfaces.

use amphion_foundation::{Point2, Point3, ToleranceContext};

use crate::{
    CurveEvaluation2, CurveEvaluation3, CurveKind, CurveProjection2, CurveProjection3,
    DerivativeOrder, GeometryError, ParameterRange, SurfaceDomain, SurfaceEvaluation, SurfaceKind,
    SurfaceProjection,
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
        tolerance: &ToleranceContext,
    ) -> Result<Vec<CurveProjection2>, GeometryError> {
        let mut output = Vec::new();
        self.project_into(point, tolerance, &mut output)?;
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
        tolerance: &ToleranceContext,
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
        tolerance: &ToleranceContext,
    ) -> Result<Vec<CurveProjection3>, GeometryError> {
        let mut output = Vec::new();
        self.project_into(point, tolerance, &mut output)?;
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
        tolerance: &ToleranceContext,
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
        tolerance: &ToleranceContext,
    ) -> Result<Vec<SurfaceProjection>, GeometryError> {
        let mut output = Vec::new();
        self.project_into(point, tolerance, &mut output)?;
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
        tolerance: &ToleranceContext,
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
