//! Immutable boundary-representation topology for the Amphion kernel.
//!
//! Topology refers to canonical geometry by typed IDs. Construction and
//! mutation occur through validated builders and transactions rather than
//! direct public field mutation.
//!
//! # Quick-start
//!
//! ```text
//! use amphion_topology::*;
//!
//! let mut b = TopologyBuilder::new();
//! let v0 = b.add_vertex(VertexParams { /* ... */ })?;
//! // ... add edges, faces, loops, coedges, shells, regions ...
//! let store = b.build()?;
//! let vertex = store.vertex(v0)?;
//! ```

mod arena;
mod builder;
mod entity;
mod error;
mod euler;
mod id;
mod orientation;
mod provenance;
mod reference;
mod store;
mod traversal;

// ── Frozen cross-crate entity types ──────────────────────────────────────────
pub use entity::{Body, Coedge, Edge, Face, Loop, Region, Shell, Vertex};

// ── Typed handles ─────────────────────────────────────────────────────────────
pub use id::{
    BodyId, CoedgeId, EdgeId, FaceId, LoopId, RegionId, ShellId, TopologyHandle, TopologyLineageId,
    TopologySnapshotId, VertexId,
};

// ── Orientation and structural kinds ─────────────────────────────────────────
pub use orientation::{LoopKind, Orientation, ShellKind};

// ── Provenance ────────────────────────────────────────────────────────────────
pub use provenance::{Provenance, ProvenanceRole, ProvenanceRoleError};

// ── Tagged references (used by diagnostics and selection) ────────────────────
pub use reference::{TopologyKind, TopologyRef};

// ── Errors ────────────────────────────────────────────────────────────────────
pub use error::{ReferrerContext, TopologyError};

// ── Immutable snapshot ────────────────────────────────────────────────────────
pub use store::TopologyStore;

// ── Builder and parameter types ───────────────────────────────────────────────
pub use builder::{
    BodyParams, CoedgeParams, EdgeParams, FaceParams, LoopParams, RegionParams, ShellParams,
    TopologyBuilder, VertexParams,
};

// ── Euler characteristic and manifold helpers ─────────────────────────────────
pub use euler::{
    EulerMetrics, body_euler_metrics, body_total_euler_ve_f, face_inner_loop_count,
    face_vertex_ids, is_closed_manifold_shell, shell_euler_metrics, shell_loop_kind_counts,
};

// ── Orientation-aware traversal ───────────────────────────────────────────────
pub use traversal::{
    coedge_end_vertex_id, coedge_start_vertex_id, edge_adjacent_face_ids, face_adjacent_face_ids,
    face_loop_ids, loop_coedge_ids, loop_next_coedge_id,
};

#[cfg(test)]
mod tests;
