//! Canonical B-Rep entity relationships.

use amphion_foundation::{LengthTolerance, Point3};
use amphion_geometry::{Curve2Id, Curve3Id, ParameterInterval, SurfaceId};
use serde::{Deserialize, Serialize};

use crate::{
    BodyId, CoedgeId, EdgeId, FaceId, LoopId, LoopKind, Orientation, Provenance, RegionId, ShellId,
    ShellKind, VertexId,
};

/// A collection of one or more connected material regions.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Body {
    id: BodyId,
    regions: Vec<RegionId>,
    provenance: Provenance,
}

impl Body {
    /// Returns the local body ID.
    #[must_use]
    pub const fn id(&self) -> BodyId {
        self.id
    }

    /// Returns connected material regions in deterministic order.
    #[must_use]
    pub fn regions(&self) -> &[RegionId] {
        &self.regions
    }

    /// Returns semantic derivation metadata.
    #[must_use]
    pub const fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}

/// A connected material region bounded by one outer and zero or more void shells.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Region {
    id: RegionId,
    outer_shell: ShellId,
    inner_shells: Vec<ShellId>,
    provenance: Provenance,
}

impl Region {
    /// Returns the local region ID.
    #[must_use]
    pub const fn id(&self) -> RegionId {
        self.id
    }

    /// Returns the material region's outer shell.
    #[must_use]
    pub const fn outer_shell(&self) -> ShellId {
        self.outer_shell
    }

    /// Returns cavity shells in deterministic order.
    #[must_use]
    pub fn inner_shells(&self) -> &[ShellId] {
        &self.inner_shells
    }

    /// Returns semantic derivation metadata.
    #[must_use]
    pub const fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}

/// An oriented collection of faces.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Shell {
    id: ShellId,
    kind: ShellKind,
    faces: Vec<FaceId>,
    provenance: Provenance,
}

impl Shell {
    /// Returns the local shell ID.
    #[must_use]
    pub const fn id(&self) -> ShellId {
        self.id
    }

    /// Returns whether the shell is open or closed.
    #[must_use]
    pub const fn kind(&self) -> ShellKind {
        self.kind
    }

    /// Returns faces in deterministic order.
    #[must_use]
    pub fn faces(&self) -> &[FaceId] {
        &self.faces
    }

    /// Returns semantic derivation metadata.
    #[must_use]
    pub const fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}

/// A trimmed, oriented use of a canonical surface.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Face {
    id: FaceId,
    surface: SurfaceId,
    orientation: Orientation,
    outer_loop: LoopId,
    inner_loops: Vec<LoopId>,
    provenance: Provenance,
}

impl Face {
    /// Returns the local face ID.
    #[must_use]
    pub const fn id(&self) -> FaceId {
        self.id
    }

    /// Returns the canonical support surface.
    #[must_use]
    pub const fn surface(&self) -> SurfaceId {
        self.surface
    }

    /// Returns orientation relative to the support surface normal.
    #[must_use]
    pub const fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Returns the outer trimming loop.
    #[must_use]
    pub const fn outer_loop(&self) -> LoopId {
        self.outer_loop
    }

    /// Returns inner trimming loops in deterministic order.
    #[must_use]
    pub fn inner_loops(&self) -> &[LoopId] {
        &self.inner_loops
    }

    /// Returns semantic derivation metadata.
    #[must_use]
    pub const fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}

/// An ordered boundary traversal on one face.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Loop {
    id: LoopId,
    face: FaceId,
    kind: LoopKind,
    coedges: Vec<CoedgeId>,
    provenance: Provenance,
}

impl Loop {
    /// Returns the local loop ID.
    #[must_use]
    pub const fn id(&self) -> LoopId {
        self.id
    }

    /// Returns the owning face.
    #[must_use]
    pub const fn face(&self) -> FaceId {
        self.face
    }

    /// Returns whether this is the outer boundary or a hole.
    #[must_use]
    pub const fn kind(&self) -> LoopKind {
        self.kind
    }

    /// Returns coedges in traversal order.
    #[must_use]
    pub fn coedges(&self) -> &[CoedgeId] {
        &self.coedges
    }

    /// Returns semantic derivation metadata.
    #[must_use]
    pub const fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}

/// One oriented use of an edge in a face loop.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct Coedge {
    id: CoedgeId,
    edge: EdgeId,
    loop_id: LoopId,
    orientation: Orientation,
    pcurve: Curve2Id,
    provenance: Provenance,
}

impl Coedge {
    /// Returns the local coedge ID.
    #[must_use]
    pub const fn id(&self) -> CoedgeId {
        self.id
    }

    /// Returns the used model-space edge.
    #[must_use]
    pub const fn edge(&self) -> EdgeId {
        self.edge
    }

    /// Returns the containing loop.
    #[must_use]
    pub const fn loop_id(&self) -> LoopId {
        self.loop_id
    }

    /// Returns traversal orientation relative to the edge curve.
    #[must_use]
    pub const fn orientation(&self) -> Orientation {
        self.orientation
    }

    /// Returns the synchronized curve in the face parameter space.
    #[must_use]
    pub const fn pcurve(&self) -> Curve2Id {
        self.pcurve
    }

    /// Returns semantic derivation metadata.
    #[must_use]
    pub const fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}

/// A bounded model-space curve shared by one or more coedges.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    id: EdgeId,
    curve: Curve3Id,
    parameter_interval: ParameterInterval,
    vertices: [VertexId; 2],
    coedges: Vec<CoedgeId>,
    tolerance: LengthTolerance,
    provenance: Provenance,
}

impl Edge {
    /// Returns the local edge ID.
    #[must_use]
    pub const fn id(&self) -> EdgeId {
        self.id
    }

    /// Returns the canonical model-space curve.
    #[must_use]
    pub const fn curve(&self) -> Curve3Id {
        self.curve
    }

    /// Returns the directed trimming interval on the canonical curve.
    #[must_use]
    pub const fn parameter_interval(&self) -> ParameterInterval {
        self.parameter_interval
    }

    /// Returns start and end vertices in canonical curve direction.
    #[must_use]
    pub const fn vertices(&self) -> [VertexId; 2] {
        self.vertices
    }

    /// Returns oriented uses in deterministic order.
    #[must_use]
    pub fn coedges(&self) -> &[CoedgeId] {
        &self.coedges
    }

    /// Returns the edge's certified model-space tolerance.
    #[must_use]
    pub const fn tolerance(&self) -> LengthTolerance {
        self.tolerance
    }

    /// Returns semantic derivation metadata.
    #[must_use]
    pub const fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}

/// A topological point with a certified model-space tolerance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vertex {
    id: VertexId,
    position: Point3,
    tolerance: LengthTolerance,
    incident_edges: Vec<EdgeId>,
    provenance: Provenance,
}

impl Vertex {
    /// Returns the local vertex ID.
    #[must_use]
    pub const fn id(&self) -> VertexId {
        self.id
    }

    /// Returns the canonical model-space position.
    #[must_use]
    pub const fn position(&self) -> Point3 {
        self.position
    }

    /// Returns the vertex's certified model-space tolerance.
    #[must_use]
    pub const fn tolerance(&self) -> LengthTolerance {
        self.tolerance
    }

    /// Returns incident edges in deterministic order.
    #[must_use]
    pub fn incident_edges(&self) -> &[EdgeId] {
        &self.incident_edges
    }

    /// Returns semantic derivation metadata.
    #[must_use]
    pub const fn provenance(&self) -> &Provenance {
        &self.provenance
    }
}
