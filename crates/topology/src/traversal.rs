//! Orientation-aware topology traversal functions.
//!
//! All traversal respects coedge and face orientation semantics as defined in
//! `CONTRACTS.md`:
//!
//! - A [`Forward`] coedge traverses the canonical edge curve from
//!   `edge.vertices()[0]` to `edge.vertices()[1]`.
//! - A [`Reversed`] coedge traverses in the opposite direction.
//! - An [`Orientation::Forward`] face normal aligns with the support-surface
//!   normal; [`Orientation::Reversed`] inverts it.
//!
//! [`Forward`]: crate::orientation::Orientation::Forward
//! [`Reversed`]: crate::orientation::Orientation::Reversed

use std::collections::BTreeSet;

use amphion_foundation::SemanticId;

use crate::error::TopologyError;
use crate::id::{CoedgeId, EdgeId, FaceId, LoopId, VertexId};
use crate::orientation::Orientation;
use crate::store::TopologyStore;

// ── Vertex access ─────────────────────────────────────────────────────────────

/// Returns the start [`VertexId`] of a coedge, respecting its orientation.
///
/// For [`Orientation::Forward`] the start is `edge.vertices()[0]`.
/// For [`Orientation::Reversed`] the start is `edge.vertices()[1]`.
///
/// # Errors
///
/// Returns [`TopologyError`] if the coedge or its edge is not present or is
/// stale.
pub fn coedge_start_vertex_id(
    store: &TopologyStore,
    coedge_id: CoedgeId,
) -> Result<VertexId, TopologyError> {
    let coedge = store.coedge(coedge_id)?;
    let edge = store.edge(coedge.edge())?;
    Ok(match coedge.orientation() {
        Orientation::Forward => edge.vertices()[0],
        Orientation::Reversed => edge.vertices()[1],
    })
}

/// Returns the end [`VertexId`] of a coedge, respecting its orientation.
///
/// For [`Orientation::Forward`] the end is `edge.vertices()[1]`.
/// For [`Orientation::Reversed`] the end is `edge.vertices()[0]`.
///
/// # Errors
///
/// Returns [`TopologyError`] if the coedge or its edge is not present or is
/// stale.
pub fn coedge_end_vertex_id(
    store: &TopologyStore,
    coedge_id: CoedgeId,
) -> Result<VertexId, TopologyError> {
    let coedge = store.coedge(coedge_id)?;
    let edge = store.edge(coedge.edge())?;
    Ok(match coedge.orientation() {
        Orientation::Forward => edge.vertices()[1],
        Orientation::Reversed => edge.vertices()[0],
    })
}

// ── Loop traversal ────────────────────────────────────────────────────────────

/// Returns the coedge IDs of a loop in traversal order.
///
/// The returned slice reflects the insertion order used at builder time and
/// is the authoritative traversal sequence for the loop.
///
/// # Errors
///
/// Returns [`TopologyError`] if the loop is not present or is stale.
pub fn loop_coedge_ids(
    store: &TopologyStore,
    loop_id: LoopId,
) -> Result<&[CoedgeId], TopologyError> {
    let lp = store.get_loop(loop_id)?;
    Ok(lp.coedges())
}

/// Returns the [`LoopId`]s of a face in declaration order: outer loop first,
/// then inner loops in deterministic (sorted) order.
///
/// # Errors
///
/// Returns [`TopologyError`] if the face is not present or is stale.
pub fn face_loop_ids(store: &TopologyStore, face_id: FaceId) -> Result<Vec<LoopId>, TopologyError> {
    let face = store.face(face_id)?;
    let mut ids = Vec::with_capacity(1 + face.inner_loops().len());
    ids.push(face.outer_loop());
    ids.extend_from_slice(face.inner_loops());
    Ok(ids)
}

// ── Adjacency queries ─────────────────────────────────────────────────────────

/// Returns the [`FaceId`]s of every face that shares at least one edge with
/// the given face, in deterministic (sorted) order.
///
/// The query face itself is never included in the result.
///
/// # Errors
///
/// Returns [`TopologyError`] for any stale or missing reference encountered
/// during traversal.
pub fn face_adjacent_face_ids(
    store: &TopologyStore,
    face_id: FaceId,
) -> Result<Vec<FaceId>, TopologyError> {
    let face = store.face(face_id)?;
    let mut adjacent = BTreeSet::new();

    for &loop_id in core::iter::once(&face.outer_loop()).chain(face.inner_loops().iter()) {
        let lp = store.get_loop(loop_id)?;
        for &coedge_id in lp.coedges() {
            let coedge = store.coedge(coedge_id)?;
            let edge = store.edge(coedge.edge())?;
            for &sibling_coedge_id in edge.coedges() {
                if sibling_coedge_id == coedge_id {
                    continue;
                }
                let sibling = store.coedge(sibling_coedge_id)?;
                let sibling_loop = store.get_loop(sibling.loop_id())?;
                let neighbor = sibling_loop.face();
                if neighbor != face_id {
                    adjacent.insert(neighbor);
                }
            }
        }
    }

    Ok(adjacent.into_iter().collect())
}

/// Returns the [`FaceId`]s of every face that directly uses the given edge,
/// in deterministic (sorted) order.
///
/// # Errors
///
/// Returns [`TopologyError`] for any stale or missing reference during
/// traversal.
pub fn edge_adjacent_face_ids(
    store: &TopologyStore,
    edge_id: EdgeId,
) -> Result<Vec<FaceId>, TopologyError> {
    let edge = store.edge(edge_id)?;
    let mut faces = BTreeSet::new();
    for &coedge_id in edge.coedges() {
        let coedge = store.coedge(coedge_id)?;
        let lp = store.get_loop(coedge.loop_id())?;
        faces.insert(lp.face());
    }
    Ok(faces.into_iter().collect())
}

/// Returns the next coedge ID in the loop traversal after `coedge_id`,
/// wrapping around to the first coedge after the last.
///
/// # Errors
///
/// Returns [`TopologyError::WrongLineage`] or [`TopologyError::WrongSnapshot`]
/// if the coedge ID's lineage/snapshot does not match the store.
/// Returns [`TopologyError::StaleHandle`] if the coedge ID's generation is stale.
/// Returns [`TopologyError::MissingEntity`] if the coedge slot is out of range
/// or the loop is not in the store.
/// Returns [`TopologyError::CoedgeNotInLoop`] if the coedge exists in the
/// store but does not belong to `loop_id`.
pub fn loop_next_coedge_id(
    store: &TopologyStore,
    loop_id: LoopId,
    coedge_id: CoedgeId,
) -> Result<CoedgeId, TopologyError> {
    // Validate that the coedge handle itself is valid (lineage/snapshot/gen/slot)
    // before checking membership. This ensures WrongLineage/WrongSnapshot/
    // StaleHandle/MissingEntity take precedence over CoedgeNotInLoop.
    let _ = store.coedge(coedge_id)?;
    let coedges = loop_coedge_ids(store, loop_id)?;
    if coedges.is_empty() {
        let mut related: Vec<SemanticId> = Vec::new();
        if let Ok(lp) = store.get_loop(loop_id) {
            related.push(lp.provenance().semantic_id());
        }
        if let Ok(ce) = store.coedge(coedge_id) {
            related.push(ce.provenance().semantic_id());
        }
        related.sort_unstable();
        related.dedup();
        return Err(TopologyError::CoedgeNotInLoop {
            loop_id,
            coedge_id,
            related,
        });
    }
    coedges
        .iter()
        .position(|&c| c == coedge_id)
        .map(|pos| coedges[(pos + 1) % coedges.len()])
        .ok_or_else(|| {
            let mut related: Vec<SemanticId> = Vec::new();
            if let Ok(lp) = store.get_loop(loop_id) {
                related.push(lp.provenance().semantic_id());
            }
            if let Ok(ce) = store.coedge(coedge_id) {
                related.push(ce.provenance().semantic_id());
            }
            related.sort_unstable();
            related.dedup();
            TopologyError::CoedgeNotInLoop {
                loop_id,
                coedge_id,
                related,
            }
        })
}
