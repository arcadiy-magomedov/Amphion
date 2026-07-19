//! Immutable boundary-representation topology contracts.
//!
//! Topology refers to canonical geometry by typed IDs. Construction and
//! mutation occur through validated builders and transactions rather than
//! direct public field mutation.

mod entity;
mod id;
mod orientation;
mod provenance;
mod reference;

pub use entity::{Body, Coedge, Edge, Face, Loop, Region, Shell, Vertex};
pub use id::{
    BodyId, CoedgeId, EdgeId, FaceId, LoopId, RegionId, ShellId, TopologyHandle, VertexId,
};
pub use orientation::{LoopKind, Orientation, ShellKind};
pub use provenance::{Provenance, ProvenanceRole, ProvenanceRoleError};
pub use reference::{TopologyKind, TopologyRef};
