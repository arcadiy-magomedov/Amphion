//! Type-safe and dynamically tagged topology references.

use serde::{Deserialize, Serialize};

use crate::{BodyId, CoedgeId, EdgeId, FaceId, LoopId, RegionId, ShellId, VertexId};

/// Stable topology entity family.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum TopologyKind {
    /// Body.
    Body,
    /// Material region.
    Region,
    /// Face shell.
    Shell,
    /// Face.
    Face,
    /// Face loop.
    Loop,
    /// Oriented edge use.
    Coedge,
    /// Edge.
    Edge,
    /// Vertex.
    Vertex,
}

/// A dynamically tagged topology reference used by selection and diagnostics.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub enum TopologyRef {
    /// Body reference.
    Body(BodyId),
    /// Region reference.
    Region(RegionId),
    /// Shell reference.
    Shell(ShellId),
    /// Face reference.
    Face(FaceId),
    /// Loop reference.
    Loop(LoopId),
    /// Coedge reference.
    Coedge(CoedgeId),
    /// Edge reference.
    Edge(EdgeId),
    /// Vertex reference.
    Vertex(VertexId),
}

impl TopologyRef {
    /// Returns the referenced entity family.
    #[must_use]
    pub const fn kind(self) -> TopologyKind {
        match self {
            Self::Body(_) => TopologyKind::Body,
            Self::Region(_) => TopologyKind::Region,
            Self::Shell(_) => TopologyKind::Shell,
            Self::Face(_) => TopologyKind::Face,
            Self::Loop(_) => TopologyKind::Loop,
            Self::Coedge(_) => TopologyKind::Coedge,
            Self::Edge(_) => TopologyKind::Edge,
            Self::Vertex(_) => TopologyKind::Vertex,
        }
    }
}
