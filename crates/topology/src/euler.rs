//! Euler characteristic and manifold bookkeeping helpers.
//!
//! These helpers support validation and primitive construction by computing
//! topological invariants for shells and bodies without requiring geometry
//! evaluation.
//!
//! ## Euler-Poincaré formula
//!
//! For a B-Rep shell the generalised Euler-Poincaré formula is:
//!
//! ```text
//! V - E + F - L_inner + 2*S_genus = 2*(S - H)
//! ```
//!
//! where:
//!
//! - `V` = vertex count (distinct vertices used by the shell)
//! - `E` = edge count (distinct edges used by the shell)
//! - `F` = face count
//! - `L_inner` = number of inner (hole) loops across all faces
//! - `S_genus` = number of through-holes (genus)
//! - `S` = number of connected shells
//! - `H` = number of void cavities (holes through solid)
//!
//! For a simple closed manifold genus-0 shell (e.g. a sphere or cube):
//! `V - E + F = 2`.

use crate::error::TopologyError;
use crate::id::{FaceId, ShellId, VertexId};
use crate::orientation::LoopKind;
use crate::store::TopologyStore;

/// Topological counts and Euler characteristics for one shell.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EulerMetrics {
    /// Number of distinct vertices used by the shell's edges.
    pub vertices: u64,
    /// Number of distinct edges used by the shell's faces.
    pub edges: u64,
    /// Number of faces in the shell.
    pub faces: u64,
    /// Number of inner (hole) loops across all faces.
    pub inner_loops: u64,
    /// Simple Euler characteristic `V - E + F`.
    ///
    /// For a closed genus-0 manifold shell this must equal `2`.
    pub euler_ve_f: i64,
    /// Euler-Poincaré characteristic `V - E + F - L_inner`.
    ///
    /// For a closed genus-0 manifold shell with no hole-loops this also
    /// equals `2`.
    pub euler_brep: i64,
}

/// Computes [`EulerMetrics`] for a single shell.
///
/// The counts are taken from the entities reachable through the shell's faces;
/// duplicate vertex or edge references (e.g. shared edges between adjacent
/// faces) are counted only once.
///
/// # Errors
///
/// Returns [`TopologyError`] for any stale or missing reference encountered
/// while traversing the shell, or [`TopologyError::ArithmeticOverflow`] if
/// entity counts exceed `i64::MAX` (not possible in practice since arena slots
/// are bounded by `u32::MAX`).
pub fn shell_euler_metrics(
    store: &TopologyStore,
    shell_id: ShellId,
) -> Result<EulerMetrics, TopologyError> {
    let shell = store.shell(shell_id)?;

    let mut all_vertex_slots: Vec<u32> = Vec::new();
    let mut all_edge_slots: Vec<u32> = Vec::new();
    let mut face_count: u64 = 0;
    let mut inner_loop_count: u64 = 0;

    for &face_id in shell.faces() {
        face_count += 1;
        let face = store.face(face_id)?;

        // Count inner loops.
        inner_loop_count += face.inner_loops().len() as u64;

        // Walk all loops of this face.
        for &loop_id in core::iter::once(&face.outer_loop()).chain(face.inner_loops().iter()) {
            let lp = store.get_loop(loop_id)?;
            for &coedge_id in lp.coedges() {
                let coedge = store.coedge(coedge_id)?;
                let edge_slot = coedge.edge().handle().slot();
                all_edge_slots.push(edge_slot);

                let edge = store.edge(coedge.edge())?;
                for v in edge.vertices() {
                    all_vertex_slots.push(v.handle().slot());
                }
            }
        }
    }

    all_vertex_slots.sort_unstable();
    all_vertex_slots.dedup();
    all_edge_slots.sort_unstable();
    all_edge_slots.dedup();

    let v = i64::try_from(all_vertex_slots.len()).map_err(|_| TopologyError::ArithmeticOverflow)?;
    let e = i64::try_from(all_edge_slots.len()).map_err(|_| TopologyError::ArithmeticOverflow)?;
    let f = i64::try_from(face_count).map_err(|_| TopologyError::ArithmeticOverflow)?;
    let l_inner = i64::try_from(inner_loop_count).map_err(|_| TopologyError::ArithmeticOverflow)?;

    Ok(EulerMetrics {
        vertices: all_vertex_slots.len() as u64,
        edges: all_edge_slots.len() as u64,
        faces: face_count,
        inner_loops: inner_loop_count,
        euler_ve_f: v - e + f,
        euler_brep: v - e + f - l_inner,
    })
}

/// Returns `true` if the shell is topologically closed.
///
/// A closed shell is one where every edge reachable from the shell's faces
/// has exactly two coedge uses (i.e., no boundary edges).
///
/// **Note:** After building through [`crate::builder::TopologyBuilder`], the
/// builder enforces that no edge is shared between different shells, so the
/// stored `edge.coedges()` count equals the per-shell count.  This function
/// is correct for stores produced by the builder.
///
/// This checks closedness only; it does not re-verify orientation consistency.
/// Use the builder's `Closed` declaration to enforce both.
///
/// # Errors
///
/// Returns [`TopologyError`] for any stale or missing reference.
pub fn is_closed_manifold_shell(
    store: &TopologyStore,
    shell_id: ShellId,
) -> Result<bool, TopologyError> {
    let shell = store.shell(shell_id)?;

    for &face_id in shell.faces() {
        let face = store.face(face_id)?;
        for &loop_id in core::iter::once(&face.outer_loop()).chain(face.inner_loops().iter()) {
            let lp = store.get_loop(loop_id)?;
            for &coedge_id in lp.coedges() {
                let coedge = store.coedge(coedge_id)?;
                let edge = store.edge(coedge.edge())?;
                if edge.coedges().len() != 2 {
                    return Ok(false);
                }
            }
        }
    }

    Ok(true)
}

/// Returns the distinct face-set Euler characteristic for a body.
///
/// Iterates all shells reachable from all regions of the body and accumulates
/// the vertex, edge, and face counts (duplicates across shells are counted
/// once per shell, following B-Rep convention for disjoint shells).
///
/// # Errors
///
/// Returns [`TopologyError`] for any stale or missing reference.
pub fn body_euler_metrics(
    store: &TopologyStore,
    body_id: crate::id::BodyId,
) -> Result<Vec<(ShellId, EulerMetrics)>, TopologyError> {
    let body = store.body(body_id)?;
    let mut result = Vec::new();
    for &region_id in body.regions() {
        let region = store.region(region_id)?;
        for &shell_id in core::iter::once(&region.outer_shell()).chain(region.inner_shells().iter())
        {
            let metrics = shell_euler_metrics(store, shell_id)?;
            result.push((shell_id, metrics));
        }
    }
    Ok(result)
}

/// Returns the aggregate Euler characteristic `V - E + F` summed over all
/// shells in the body.
///
/// For a body consisting of a single closed manifold genus-0 region this
/// should equal `2`.
///
/// # Errors
///
/// Returns [`TopologyError`] for any stale or missing reference, or
/// [`TopologyError::ArithmeticOverflow`] if the accumulated sum overflows
/// `i64`.
pub fn body_total_euler_ve_f(
    store: &TopologyStore,
    body_id: crate::id::BodyId,
) -> Result<i64, TopologyError> {
    let shell_metrics = body_euler_metrics(store, body_id)?;
    let mut total: i64 = 0;
    for (_, m) in &shell_metrics {
        total = total
            .checked_add(m.euler_ve_f)
            .ok_or(TopologyError::ArithmeticOverflow)?;
    }
    Ok(total)
}

/// Collects the set of vertex IDs used by a face (through all its loops and
/// coedges).
///
/// Each vertex appears at most once in the returned vector.
///
/// # Errors
///
/// Returns [`TopologyError`] if any entity handle is stale or missing.
pub fn face_vertex_ids(
    store: &TopologyStore,
    face_id: FaceId,
) -> Result<Vec<VertexId>, TopologyError> {
    let face = store.face(face_id)?;
    let mut vertices: Vec<VertexId> = Vec::new();
    for &loop_id in core::iter::once(&face.outer_loop()).chain(face.inner_loops().iter()) {
        let lp = store.get_loop(loop_id)?;
        for &coedge_id in lp.coedges() {
            let coedge = store.coedge(coedge_id)?;
            let edge = store.edge(coedge.edge())?;
            for v_id in edge.vertices() {
                vertices.push(v_id);
            }
        }
    }
    vertices.sort_unstable();
    vertices.dedup();
    Ok(vertices)
}

/// Returns the number of inner (hole) loops on a face.
///
/// # Errors
///
/// Returns [`TopologyError`] if the face is stale or missing.
pub fn face_inner_loop_count(
    store: &TopologyStore,
    face_id: FaceId,
) -> Result<usize, TopologyError> {
    let face = store.face(face_id)?;
    Ok(face.inner_loops().len())
}

/// Returns the [`LoopKind`] breakdown for all loops in a shell.
///
/// Returns `(outer_count, inner_count)`.
///
/// # Errors
///
/// Returns [`TopologyError`] for stale or missing references.
pub fn shell_loop_kind_counts(
    store: &TopologyStore,
    shell_id: ShellId,
) -> Result<(u64, u64), TopologyError> {
    let shell = store.shell(shell_id)?;
    let mut outer = 0u64;
    let mut inner = 0u64;
    for &face_id in shell.faces() {
        let face = store.face(face_id)?;
        for &loop_id in core::iter::once(&face.outer_loop()).chain(face.inner_loops().iter()) {
            let lp = store.get_loop(loop_id)?;
            match lp.kind() {
                LoopKind::Outer => outer += 1,
                LoopKind::Inner => inner += 1,
            }
        }
    }
    Ok((outer, inner))
}
