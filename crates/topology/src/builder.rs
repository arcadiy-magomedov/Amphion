//! Validated builder for immutable topology snapshots.
//!
//! Construction follows a bottom-up dependency order:
//!
//! 1. [`add_vertex`] — requires no prior entities.
//! 2. [`add_edge`] — references two [`VertexId`]s.
//! 3. [`add_face`] — requires a [`SurfaceId`]; loops are inferred later.
//! 4. [`add_loop`] — references a [`FaceId`]; coedges are inferred later.
//! 5. [`add_coedge`] — references an [`EdgeId`] and a [`LoopId`].
//! 6. [`add_shell`] — references one or more [`FaceId`]s.
//! 7. [`add_region`] — references a shell as outer boundary.
//! 8. [`add_body`] — references one or more [`RegionId`]s.
//!
//! Call [`build`] after all entities have been added. The builder performs
//! exhaustive cross-reference and structural validation, computing back-
//! references (`face.outer_loop`, `loop.coedges`, `edge.coedges`,
//! `vertex.incident_edges`) automatically before constructing the immutable
//! [`TopologyStore`].
//!
//! [`add_vertex`]: TopologyBuilder::add_vertex
//! [`add_edge`]: TopologyBuilder::add_edge
//! [`add_face`]: TopologyBuilder::add_face
//! [`add_loop`]: TopologyBuilder::add_loop
//! [`add_coedge`]: TopologyBuilder::add_coedge
//! [`add_shell`]: TopologyBuilder::add_shell
//! [`add_region`]: TopologyBuilder::add_region
//! [`add_body`]: TopologyBuilder::add_body
//! [`build`]: TopologyBuilder::build

#[cfg(test)]
use std::cell::Cell;

use amphion_foundation::{LengthTolerance, Point3, SemanticId};
use amphion_geometry::{Curve2Id, Curve3Id, ParameterInterval, SurfaceId};

use crate::provenance::Provenance;

use crate::arena::Arena;

// Thread-local counters keep parallel unit tests from contaminating measurements.
#[cfg(test)]
std::thread_local! {
    static CONNECTIVITY_VISIT_COUNT: Cell<usize> = const { Cell::new(0) };
    static REACHABILITY_VISIT_COUNT: Cell<usize> = const { Cell::new(0) };
    static INCIDENCE_VISIT_COUNT: Cell<usize> = const { Cell::new(0) };
}

#[cfg(test)]
fn count_connectivity_visits(count: usize) {
    CONNECTIVITY_VISIT_COUNT.set(CONNECTIVITY_VISIT_COUNT.get() + count);
}

#[cfg(test)]
fn count_reachability_visits(count: usize) {
    REACHABILITY_VISIT_COUNT.set(REACHABILITY_VISIT_COUNT.get() + count);
}

#[cfg(test)]
fn count_incidence_visits(count: usize) {
    INCIDENCE_VISIT_COUNT.set(INCIDENCE_VISIT_COUNT.get() + count);
}

/// Returns the current connectivity visit count and resets it to zero.
#[cfg(test)]
pub(crate) fn take_connectivity_visit_count() -> usize {
    CONNECTIVITY_VISIT_COUNT.replace(0)
}

/// Returns the current reachability visit count and resets it to zero.
#[cfg(test)]
pub(crate) fn take_reachability_visit_count() -> usize {
    REACHABILITY_VISIT_COUNT.replace(0)
}

/// Returns the current closed-shell incidence visit count and resets it to zero.
#[cfg(test)]
pub(crate) fn take_incidence_visit_count() -> usize {
    INCIDENCE_VISIT_COUNT.replace(0)
}
use crate::entity::{Body, Coedge, Edge, Face, Loop, Region, Shell, Vertex};
use crate::error::{ReferrerContext, TopologyError, collect_errors};
use crate::id::{
    BodyId, CoedgeId, EdgeId, FaceId, LoopId, RegionId, ShellId, TopologyLineageId,
    TopologySnapshotId, VertexId,
};
use crate::orientation::{LoopKind, Orientation, ShellKind};
use crate::reference::TopologyKind;
use crate::store::TopologyStore;

// ── Parameter structs ─────────────────────────────────────────────────────────

/// Parameters for creating a [`Vertex`].
pub struct VertexParams {
    /// Certified model-space position.
    pub position: Point3,
    /// Certified positional tolerance in metres.
    pub tolerance: LengthTolerance,
    /// Semantic derivation metadata.
    pub provenance: Provenance,
}

/// Parameters for creating an [`Edge`].
pub struct EdgeParams {
    /// Canonical model-space curve identity.
    pub curve: Curve3Id,
    /// Directed trimming interval on the canonical curve.
    pub parameter_interval: ParameterInterval,
    /// Start vertex (canonical curve direction, `Forward` coedge perspective).
    pub start_vertex: VertexId,
    /// End vertex (canonical curve direction, `Forward` coedge perspective).
    pub end_vertex: VertexId,
    /// Certified edge tolerance in metres.
    pub tolerance: LengthTolerance,
    /// Semantic derivation metadata.
    pub provenance: Provenance,
}

/// Parameters for creating a [`Face`].
///
/// The face's loops are not supplied here; they are inferred from the
/// [`LoopParams::face`] references of loops added later.
pub struct FaceParams {
    /// Canonical support surface identity.
    pub surface: SurfaceId,
    /// Orientation relative to the support surface normal.
    pub orientation: Orientation,
    /// Semantic derivation metadata.
    pub provenance: Provenance,
}

/// Parameters for creating a [`Loop`].
///
/// Coedges are not supplied here; they are inferred from the
/// [`CoedgeParams::loop_id`] references of coedges added later.
pub struct LoopParams {
    /// Owning face.
    pub face: FaceId,
    /// Outer boundary or hole.
    pub kind: LoopKind,
    /// Semantic derivation metadata.
    pub provenance: Provenance,
}

/// Parameters for creating a [`Coedge`].
pub struct CoedgeParams {
    /// The model-space edge used by this coedge.
    pub edge: EdgeId,
    /// The loop this coedge belongs to.
    pub loop_id: LoopId,
    /// Traversal direction relative to the canonical edge curve.
    pub orientation: Orientation,
    /// Parameter-space curve synchronized with the edge's 3-D curve.
    pub pcurve: Curve2Id,
    /// Semantic derivation metadata.
    pub provenance: Provenance,
}

/// Parameters for creating a [`Shell`].
pub struct ShellParams {
    /// Claimed topological closure of the shell.
    pub kind: ShellKind,
    /// Faces in deterministic order.
    pub faces: Vec<FaceId>,
    /// Semantic derivation metadata.
    pub provenance: Provenance,
}

/// Parameters for creating a [`Region`].
pub struct RegionParams {
    /// The outer bounding shell.
    pub outer_shell: ShellId,
    /// Cavity shells in deterministic order.
    pub inner_shells: Vec<ShellId>,
    /// Semantic derivation metadata.
    pub provenance: Provenance,
}

/// Parameters for creating a [`Body`].
pub struct BodyParams {
    /// Connected material regions in deterministic order.
    pub regions: Vec<RegionId>,
    /// Semantic derivation metadata.
    pub provenance: Provenance,
}

// ── Internal raw storage ──────────────────────────────────────────────────────

struct VertexRaw {
    id: VertexId,
    position: Point3,
    tolerance: LengthTolerance,
    provenance: Provenance,
}

struct EdgeRaw {
    id: EdgeId,
    curve: Curve3Id,
    parameter_interval: ParameterInterval,
    vertices: [VertexId; 2],
    tolerance: LengthTolerance,
    provenance: Provenance,
}

struct FaceRaw {
    id: FaceId,
    surface: SurfaceId,
    orientation: Orientation,
    provenance: Provenance,
}

struct LoopRaw {
    id: LoopId,
    face: FaceId,
    kind: LoopKind,
    provenance: Provenance,
}

struct CoedgeRaw {
    id: CoedgeId,
    edge: EdgeId,
    loop_id: LoopId,
    orientation: Orientation,
    pcurve: Curve2Id,
    provenance: Provenance,
}

struct ShellRaw {
    id: ShellId,
    kind: ShellKind,
    faces: Vec<FaceId>,
    provenance: Provenance,
}

struct RegionRaw {
    id: RegionId,
    outer_shell: ShellId,
    inner_shells: Vec<ShellId>,
    provenance: Provenance,
}

struct BodyRaw {
    id: BodyId,
    regions: Vec<RegionId>,
    provenance: Provenance,
}

// ── Builder ───────────────────────────────────────────────────────────────────

/// Constructs and validates an immutable [`TopologyStore`] snapshot.
///
/// All entities are assigned generation-checked IDs at add-time. Call
/// [`build`] after adding all entities to perform cross-reference validation
/// and construct the store.
///
/// [`build`]: TopologyBuilder::build
pub struct TopologyBuilder {
    generation: u32,
    lineage: TopologyLineageId,
    snapshot: TopologySnapshotId,
    vertices: Vec<VertexRaw>,
    edges: Vec<EdgeRaw>,
    faces: Vec<FaceRaw>,
    loops: Vec<LoopRaw>,
    coedges: Vec<CoedgeRaw>,
    shells: Vec<ShellRaw>,
    regions: Vec<RegionRaw>,
    bodies: Vec<BodyRaw>,
}

impl TopologyBuilder {
    /// Creates a root builder with explicit lineage and snapshot IDs at generation 0.
    ///
    /// This is the canonical public constructor. Callers must supply both a
    /// stable [`TopologyLineageId`] (derived from a document or project
    /// identity) and a [`TopologySnapshotId`] (unique per snapshot instance).
    /// There is no default constructor; all callers must be explicit about
    /// their identity namespace.
    #[must_use]
    pub fn with_lineage_and_snapshot(
        lineage: TopologyLineageId,
        snapshot: TopologySnapshotId,
    ) -> Self {
        Self::_from_lineage_snapshot_gen(lineage, snapshot, 0)
    }

    pub(crate) fn _from_lineage_snapshot_gen(
        lineage: TopologyLineageId,
        snapshot: TopologySnapshotId,
        generation: u32,
    ) -> Self {
        Self {
            generation,
            lineage,
            snapshot,
            vertices: Vec::new(),
            edges: Vec::new(),
            faces: Vec::new(),
            loops: Vec::new(),
            coedges: Vec::new(),
            shells: Vec::new(),
            regions: Vec::new(),
            bodies: Vec::new(),
        }
    }

    /// Creates a builder with an explicit generation, lineage, and snapshot.
    ///
    /// Intended only for tests that need to exercise generation-mismatch
    /// detection (e.g., `next_generation_u32max_overflows`). Production code
    /// must use [`with_lineage_and_snapshot`] or
    /// [`TopologyStore::successor_builder`].
    ///
    /// [`with_lineage_and_snapshot`]: Self::with_lineage_and_snapshot
    #[cfg(test)]
    #[must_use]
    pub(crate) fn _with_generation_for_testing(
        lineage: TopologyLineageId,
        snapshot: TopologySnapshotId,
        generation: u32,
    ) -> Self {
        Self::_from_lineage_snapshot_gen(lineage, snapshot, generation)
    }

    /// Returns the snapshot generation assigned to all IDs from this builder.
    #[must_use]
    pub const fn generation(&self) -> u32 {
        self.generation
    }

    /// Returns the lineage assigned to all IDs from this builder.
    #[must_use]
    pub const fn lineage(&self) -> TopologyLineageId {
        self.lineage
    }

    /// Returns the snapshot ID assigned to all IDs from this builder.
    #[must_use]
    pub const fn snapshot(&self) -> TopologySnapshotId {
        self.snapshot
    }

    // ── Add methods ───────────────────────────────────────────────────────────

    /// Adds a vertex and returns its generation-checked ID.
    ///
    /// # Errors
    ///
    /// Returns [`TopologyError::ArenaOverflow`] if the vertex count would
    /// exceed `u32::MAX`.
    pub fn add_vertex(&mut self, params: VertexParams) -> Result<VertexId, TopologyError> {
        let slot = next_slot(self.vertices.len())?;
        let id = VertexId::new(slot, self.generation, self.lineage, self.snapshot);
        self.vertices.push(VertexRaw {
            id,
            position: params.position,
            tolerance: params.tolerance,
            provenance: params.provenance,
        });
        Ok(id)
    }

    /// Adds an edge and returns its generation-checked ID.
    ///
    /// # Errors
    ///
    /// Returns [`TopologyError::ArenaOverflow`] if the edge count would exceed
    /// `u32::MAX`.
    pub fn add_edge(&mut self, params: EdgeParams) -> Result<EdgeId, TopologyError> {
        let slot = next_slot(self.edges.len())?;
        let id = EdgeId::new(slot, self.generation, self.lineage, self.snapshot);
        self.edges.push(EdgeRaw {
            id,
            curve: params.curve,
            parameter_interval: params.parameter_interval,
            vertices: [params.start_vertex, params.end_vertex],
            tolerance: params.tolerance,
            provenance: params.provenance,
        });
        Ok(id)
    }

    /// Adds a face and returns its generation-checked ID.
    ///
    /// Loops are **not** supplied here; they are inferred during [`build`]
    /// from the [`LoopParams::face`] references of subsequently added loops.
    ///
    /// # Errors
    ///
    /// Returns [`TopologyError::ArenaOverflow`] if the face count would exceed
    /// `u32::MAX`.
    ///
    /// [`build`]: Self::build
    pub fn add_face(&mut self, params: FaceParams) -> Result<FaceId, TopologyError> {
        let slot = next_slot(self.faces.len())?;
        let id = FaceId::new(slot, self.generation, self.lineage, self.snapshot);
        self.faces.push(FaceRaw {
            id,
            surface: params.surface,
            orientation: params.orientation,
            provenance: params.provenance,
        });
        Ok(id)
    }

    /// Adds a loop and returns its generation-checked ID.
    ///
    /// Coedges are **not** supplied here; they are inferred during [`build`]
    /// from the [`CoedgeParams::loop_id`] references of subsequently added
    /// coedges, in insertion order.
    ///
    /// # Errors
    ///
    /// Returns [`TopologyError::ArenaOverflow`] if the loop count would exceed
    /// `u32::MAX`.
    ///
    /// [`build`]: Self::build
    pub fn add_loop(&mut self, params: LoopParams) -> Result<LoopId, TopologyError> {
        let slot = next_slot(self.loops.len())?;
        let id = LoopId::new(slot, self.generation, self.lineage, self.snapshot);
        self.loops.push(LoopRaw {
            id,
            face: params.face,
            kind: params.kind,
            provenance: params.provenance,
        });
        Ok(id)
    }

    /// Adds a coedge and returns its generation-checked ID.
    ///
    /// # Errors
    ///
    /// Returns [`TopologyError::ArenaOverflow`] if the coedge count would
    /// exceed `u32::MAX`.
    pub fn add_coedge(&mut self, params: CoedgeParams) -> Result<CoedgeId, TopologyError> {
        let slot = next_slot(self.coedges.len())?;
        let id = CoedgeId::new(slot, self.generation, self.lineage, self.snapshot);
        self.coedges.push(CoedgeRaw {
            id,
            edge: params.edge,
            loop_id: params.loop_id,
            orientation: params.orientation,
            pcurve: params.pcurve,
            provenance: params.provenance,
        });
        Ok(id)
    }

    /// Adds a shell and returns its generation-checked ID.
    ///
    /// # Errors
    ///
    /// Returns [`TopologyError::ArenaOverflow`] if the shell count would
    /// exceed `u32::MAX`.
    pub fn add_shell(&mut self, params: ShellParams) -> Result<ShellId, TopologyError> {
        let slot = next_slot(self.shells.len())?;
        let id = ShellId::new(slot, self.generation, self.lineage, self.snapshot);
        self.shells.push(ShellRaw {
            id,
            kind: params.kind,
            faces: params.faces,
            provenance: params.provenance,
        });
        Ok(id)
    }

    /// Adds a region and returns its generation-checked ID.
    ///
    /// # Errors
    ///
    /// Returns [`TopologyError::ArenaOverflow`] if the region count would
    /// exceed `u32::MAX`.
    pub fn add_region(&mut self, params: RegionParams) -> Result<RegionId, TopologyError> {
        let slot = next_slot(self.regions.len())?;
        let id = RegionId::new(slot, self.generation, self.lineage, self.snapshot);
        self.regions.push(RegionRaw {
            id,
            outer_shell: params.outer_shell,
            inner_shells: params.inner_shells,
            provenance: params.provenance,
        });
        Ok(id)
    }

    /// Adds a body and returns its generation-checked ID.
    ///
    /// # Errors
    ///
    /// Returns [`TopologyError::ArenaOverflow`] if the body count would exceed
    /// `u32::MAX`.
    pub fn add_body(&mut self, params: BodyParams) -> Result<BodyId, TopologyError> {
        let slot = next_slot(self.bodies.len())?;
        let id = BodyId::new(slot, self.generation, self.lineage, self.snapshot);
        self.bodies.push(BodyRaw {
            id,
            regions: params.regions,
            provenance: params.provenance,
        });
        Ok(id)
    }

    // ── Build ─────────────────────────────────────────────────────────────────

    /// Validates all cross-references and structural invariants, then
    /// constructs an immutable [`TopologyStore`].
    ///
    /// On success every entity is fully assembled with computed back-references
    /// (`face.outer_loop`, `loop.coedges`, `edge.coedges`,
    /// `vertex.incident_edges`).
    ///
    /// On failure returns a [`TopologyError`] describing every violation
    /// found. Multiple violations are wrapped in [`TopologyError::Multiple`].
    ///
    /// # Errors
    ///
    /// Returns [`TopologyError`] for any of:
    /// - stale or missing ID references;
    /// - faces with no outer loop or with duplicate outer loops;
    /// - loops with no coedges;
    /// - loop coedge chains that are not topologically closed;
    /// - edges with more than two coedge uses per shell (non-manifold);
    /// - edges shared between different shells;
    /// - closed shells with same-direction paired edge uses (non-orientable);
    /// - open shells with no boundary edges;
    /// - disconnected shells (face adjacency graph not connected);
    /// - shells whose [`ShellKind`] claim is inconsistent with edge topology;
    /// - duplicate face/shell/region IDs within a collection;
    /// - faces, shells, or regions referenced by more than one parent;
    /// - outer or cavity shells that are not [`ShellKind::Closed`];
    /// - entities unreachable from any body (orphans);
    /// - bodies with no regions;
    /// - shells with no faces.
    pub fn build(self) -> Result<TopologyStore, TopologyError> {
        // Phase 1: cross-reference existence and generation checks.
        check_cross_refs(&self)?;
        // Phase 2: back-reference computation.
        let back = compute_back_refs(&self);
        // Phase 3: structural invariant validation.
        check_structural(&self, &back)?;
        // Phase 4: construct the immutable arenas.
        build_arenas(self, back)
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

// ── Back-reference container ──────────────────────────────────────────────────

/// Back-references computed from forward declarations during [`build`].
///
/// [`build`]: TopologyBuilder::build
struct BackRefs {
    loop_coedges: Vec<Vec<CoedgeId>>,
    face_outer_loop: Vec<Option<LoopId>>,
    face_all_outer_loops: Vec<Vec<LoopId>>,
    face_inner_loops: Vec<Vec<LoopId>>,
    face_outer_count: Vec<u32>,
    edge_coedges: Vec<Vec<CoedgeId>>,
    vertex_edges: Vec<Vec<EdgeId>>,
}

// ── Phase 1: Cross-reference checks ──────────────────────────────────────────

fn check_cross_refs(b: &TopologyBuilder) -> Result<(), TopologyError> {
    let mut errors: Vec<TopologyError> = Vec::new();
    check_primitive_refs(b, &mut errors);
    check_container_refs(b, &mut errors);
    if errors.is_empty() {
        Ok(())
    } else {
        Err(collect_errors(errors))
    }
}

/// Checks edges (vertex refs), coedges (edge + loop refs), and loops (face refs).
fn check_primitive_refs(b: &TopologyBuilder, errors: &mut Vec<TopologyError>) {
    let snap = b.generation;
    let lineage = b.lineage;
    let snapshot = b.snapshot;
    let (n_v, n_e, n_f, n_l) = (
        b.vertices.len(),
        b.edges.len(),
        b.faces.len(),
        b.loops.len(),
    );
    for (slot, edge) in b.edges.iter().enumerate() {
        let sem = Some(edge.provenance.semantic_id());
        check_vertex_ref(
            edge.vertices[0],
            lineage,
            snapshot,
            snap,
            n_v,
            TopologyKind::Edge,
            slot,
            "start_vertex",
            sem,
            errors,
        );
        check_vertex_ref(
            edge.vertices[1],
            lineage,
            snapshot,
            snap,
            n_v,
            TopologyKind::Edge,
            slot,
            "end_vertex",
            sem,
            errors,
        );
    }
    for (slot, coedge) in b.coedges.iter().enumerate() {
        let sem = Some(coedge.provenance.semantic_id());
        check_ref(
            coedge.edge,
            lineage,
            snapshot,
            snap,
            n_e,
            TopologyKind::Edge,
            TopologyKind::Coedge,
            slot,
            "edge",
            None,
            sem,
            errors,
        );
        check_ref(
            coedge.loop_id,
            lineage,
            snapshot,
            snap,
            n_l,
            TopologyKind::Loop,
            TopologyKind::Coedge,
            slot,
            "loop_id",
            None,
            sem,
            errors,
        );
    }
    for (slot, lp) in b.loops.iter().enumerate() {
        let sem = Some(lp.provenance.semantic_id());
        check_ref(
            lp.face,
            lineage,
            snapshot,
            snap,
            n_f,
            TopologyKind::Face,
            TopologyKind::Loop,
            slot,
            "face",
            None,
            sem,
            errors,
        );
    }
}

/// Checks shell face refs, region shell refs, and body region refs.
fn check_container_refs(b: &TopologyBuilder, errors: &mut Vec<TopologyError>) {
    let snap = b.generation;
    let lineage = b.lineage;
    let snapshot = b.snapshot;
    let (n_f, n_s, n_r) = (b.faces.len(), b.shells.len(), b.regions.len());
    for (slot, shell) in b.shells.iter().enumerate() {
        let sem = Some(shell.provenance.semantic_id());
        for (idx, &fid) in shell.faces.iter().enumerate() {
            check_ref(
                fid,
                lineage,
                snapshot,
                snap,
                n_f,
                TopologyKind::Face,
                TopologyKind::Shell,
                slot,
                "faces",
                Some(u32::try_from(idx).unwrap_or(u32::MAX)),
                sem,
                errors,
            );
        }
    }
    for (slot, region) in b.regions.iter().enumerate() {
        let sem = Some(region.provenance.semantic_id());
        check_ref(
            region.outer_shell,
            lineage,
            snapshot,
            snap,
            n_s,
            TopologyKind::Shell,
            TopologyKind::Region,
            slot,
            "outer_shell",
            None,
            sem,
            errors,
        );
        for (idx, &inner) in region.inner_shells.iter().enumerate() {
            check_ref(
                inner,
                lineage,
                snapshot,
                snap,
                n_s,
                TopologyKind::Shell,
                TopologyKind::Region,
                slot,
                "inner_shells",
                Some(u32::try_from(idx).unwrap_or(u32::MAX)),
                sem,
                errors,
            );
        }
    }
    for (slot, body) in b.bodies.iter().enumerate() {
        let sem = Some(body.provenance.semantic_id());
        for (idx, &rid) in body.regions.iter().enumerate() {
            check_ref(
                rid,
                lineage,
                snapshot,
                snap,
                n_r,
                TopologyKind::Region,
                TopologyKind::Body,
                slot,
                "regions",
                Some(u32::try_from(idx).unwrap_or(u32::MAX)),
                sem,
                errors,
            );
        }
    }
}

// ── Phase 2: Back-reference computation ──────────────────────────────────────

fn compute_back_refs(b: &TopologyBuilder) -> BackRefs {
    let (n_v, n_e, n_f, n_l) = (
        b.vertices.len(),
        b.edges.len(),
        b.faces.len(),
        b.loops.len(),
    );
    let mut loop_coedges: Vec<Vec<CoedgeId>> = vec![Vec::new(); n_l];
    for coedge in &b.coedges {
        let ls = coedge.loop_id.handle().slot() as usize;
        loop_coedges[ls].push(coedge.id);
    }
    let mut face_outer_loop: Vec<Option<LoopId>> = vec![None; n_f];
    let mut face_all_outer_loops: Vec<Vec<LoopId>> = vec![Vec::new(); n_f];
    let mut face_inner_loops: Vec<Vec<LoopId>> = vec![Vec::new(); n_f];
    let mut face_outer_count: Vec<u32> = vec![0; n_f];
    for lp in &b.loops {
        let fs = lp.face.handle().slot() as usize;
        match lp.kind {
            LoopKind::Outer => {
                face_outer_count[fs] += 1;
                face_outer_loop[fs] = Some(lp.id);
                face_all_outer_loops[fs].push(lp.id);
            }
            LoopKind::Inner => {
                face_inner_loops[fs].push(lp.id);
            }
        }
    }
    for inner in &mut face_inner_loops {
        inner.sort_unstable();
    }
    let mut edge_coedges: Vec<Vec<CoedgeId>> = vec![Vec::new(); n_e];
    for coedge in &b.coedges {
        let es = coedge.edge.handle().slot() as usize;
        edge_coedges[es].push(coedge.id);
    }
    let mut vertex_edges: Vec<Vec<EdgeId>> = vec![Vec::new(); n_v];
    for edge in &b.edges {
        let v0 = edge.vertices[0].handle().slot() as usize;
        let v1 = edge.vertices[1].handle().slot() as usize;
        vertex_edges[v0].push(edge.id);
        if v1 != v0 {
            vertex_edges[v1].push(edge.id);
        }
    }
    for ev in &mut vertex_edges {
        ev.sort_unstable();
        ev.dedup();
    }
    BackRefs {
        loop_coedges,
        face_outer_loop,
        face_all_outer_loops,
        face_inner_loops,
        face_outer_count,
        edge_coedges,
        vertex_edges,
    }
}

// ── Phase 3: Structural invariant checks ─────────────────────────────────────

fn check_structural(b: &TopologyBuilder, back: &BackRefs) -> Result<(), TopologyError> {
    let mut errors: Vec<TopologyError> = Vec::new();
    // Loop closure, outer-loop presence, manifold constraint.
    check_loop_and_face_invariants(b, back, &mut errors);
    // Ownership uniqueness (builds face_shell_owner table).
    let (face_shell_owner, shell_connectivity_eligible) = check_ownership(b, back, &mut errors);
    // Cross-shell edge sharing detection.
    let coedge_shell = build_coedge_shell(b, &face_shell_owner);
    check_cross_shell_edges(b, back, &coedge_shell, &mut errors);
    // Per-shell edge semantics: manifold + orientation pairs + open boundary.
    check_per_shell_edge_semantics(b, &coedge_shell, &mut errors);
    // Shell connectivity (face-adjacency BFS).
    check_shell_connectivity(
        b,
        &face_shell_owner,
        &shell_connectivity_eligible,
        &mut errors,
    );
    // Empty body check.
    for body in &b.bodies {
        if body.regions.is_empty() {
            errors.push(TopologyError::EmptyBody {
                body_id: body.id,
                related: vec![body.provenance.semantic_id()],
            });
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(collect_errors(errors))
    }
}

/// Phase 3a: loop/face structural invariants and global non-manifold check.
fn check_loop_and_face_invariants(
    b: &TopologyBuilder,
    back: &BackRefs,
    errors: &mut Vec<TopologyError>,
) {
    for (face_slot, &count) in back.face_outer_count.iter().enumerate() {
        let face_id = b.faces[face_slot].id;
        match count {
            0 => errors.push(TopologyError::MissingOuterLoop {
                face_id,
                related: vec![b.faces[face_slot].provenance.semantic_id()],
            }),
            1 => {}
            _ => {
                let mut related = vec![b.faces[face_slot].provenance.semantic_id()];
                for &loop_id in &back.face_all_outer_loops[face_slot] {
                    let loop_slot = loop_id.handle().slot() as usize;
                    if loop_slot < b.loops.len() {
                        related.push(b.loops[loop_slot].provenance.semantic_id());
                    }
                }
                related.sort_unstable();
                related.dedup();
                errors.push(TopologyError::DuplicateOuterLoop { face_id, related });
            }
        }
    }
    for (loop_slot, coedge_ids) in back.loop_coedges.iter().enumerate() {
        let loop_id = b.loops[loop_slot].id;
        if coedge_ids.is_empty() {
            errors.push(TopologyError::EmptyLoop {
                loop_id,
                related: vec![b.loops[loop_slot].provenance.semantic_id()],
            });
        } else {
            validate_loop_closure(
                loop_id,
                b.loops[loop_slot].provenance.semantic_id(),
                coedge_ids,
                &b.coedges,
                &b.edges,
                &b.vertices,
                errors,
            );
        }
    }
    for (edge_slot, coedge_ids) in back.edge_coedges.iter().enumerate() {
        if coedge_ids.len() > 2 {
            let mut related = vec![b.edges[edge_slot].provenance.semantic_id()];
            for &coedge_id in coedge_ids {
                let coedge_slot = coedge_id.handle().slot() as usize;
                if coedge_slot < b.coedges.len() {
                    related.push(b.coedges[coedge_slot].provenance.semantic_id());
                }
            }
            related.sort_unstable();
            related.dedup();
            errors.push(TopologyError::NonManifoldEdge {
                edge_id: b.edges[edge_slot].id,
                use_count: coedge_ids.len(),
                related,
            });
        }
    }
    for shell in &b.shells {
        if shell.faces.is_empty() {
            errors.push(TopologyError::EmptyShell {
                shell_id: shell.id,
                related: vec![shell.provenance.semantic_id()],
            });
        }
    }
}

/// Phase 3b: ownership uniqueness checks.
///
/// Returns `face_shell_owner[face_slot] = Option<shell_slot>`.
fn check_ownership(
    b: &TopologyBuilder,
    back: &BackRefs,
    errors: &mut Vec<TopologyError>,
) -> (Vec<Option<usize>>, Vec<bool>) {
    let mut face_shell_owner: Vec<Option<usize>> = vec![None; b.faces.len()];
    let mut shell_connectivity_eligible = vec![true; b.shells.len()];
    // Face uniqueness within shells.
    for shell in &b.shells {
        let current_shell_slot = shell.id.handle().slot() as usize;
        let mut seen_in_shell: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for &face_id in &shell.faces {
            let fs = face_id.handle().slot() as usize;
            let slot_u32 = face_id.handle().slot();
            if !seen_in_shell.insert(slot_u32) {
                if current_shell_slot < shell_connectivity_eligible.len() {
                    shell_connectivity_eligible[current_shell_slot] = false;
                }
                let mut related = if fs < b.faces.len() {
                    vec![b.faces[fs].provenance.semantic_id()]
                } else {
                    vec![]
                };
                related.push(shell.provenance.semantic_id());
                related.sort_unstable();
                related.dedup();
                errors.push(TopologyError::DuplicateIdInCollection {
                    kind: TopologyKind::Face,
                    slot: slot_u32,
                    related,
                });
            }
            if let Some(prev_shell_slot) = face_shell_owner.get(fs).copied().flatten() {
                if prev_shell_slot < shell_connectivity_eligible.len() {
                    shell_connectivity_eligible[prev_shell_slot] = false;
                }
                if current_shell_slot < shell_connectivity_eligible.len() {
                    shell_connectivity_eligible[current_shell_slot] = false;
                }
                if prev_shell_slot != current_shell_slot {
                    let mut related = vec![b.faces[fs].provenance.semantic_id()];
                    if prev_shell_slot < b.shells.len() {
                        related.push(b.shells[prev_shell_slot].provenance.semantic_id());
                    }
                    related.push(shell.provenance.semantic_id());
                    related.sort_unstable();
                    related.dedup();
                    errors.push(TopologyError::FaceOwnershipConflict { face_id, related });
                }
            } else if fs < face_shell_owner.len() {
                face_shell_owner[fs] = Some(current_shell_slot);
            }
        }
    }
    // Shell uniqueness within regions + outer/cavity must be Closed.
    let mut shell_region_owner: Vec<Option<usize>> = vec![None; b.shells.len()];
    check_region_shell_ownership(b, &mut shell_region_owner, errors);
    // Region uniqueness within bodies.
    check_body_region_ownership(b, errors);
    // Orphan entities.
    check_orphan_entities(b, back_reachable(b, back), errors);
    (face_shell_owner, shell_connectivity_eligible)
}

/// Checks shell ownership uniqueness and outer/cavity Closed requirement.
///
/// Both outer and inner shells must be [`ShellKind::Closed`].
fn check_region_shell_ownership(
    b: &TopologyBuilder,
    shell_region_owner: &mut [Option<usize>],
    errors: &mut Vec<TopologyError>,
) {
    for region in &b.regions {
        let rs = region.id.handle().slot() as usize;
        register_shell(
            region.outer_shell,
            region.id,
            rs,
            shell_region_owner,
            b,
            errors,
            true,
        );
        // Inner (cavity) shells: must be Closed.
        let mut seen_in_region: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for &inner in &region.inner_shells {
            let slot_u32 = inner.handle().slot();
            if !seen_in_region.insert(slot_u32) {
                let mut related = if slot_u32 as usize >= b.shells.len() {
                    vec![]
                } else {
                    vec![b.shells[slot_u32 as usize].provenance.semantic_id()]
                };
                related.push(region.provenance.semantic_id());
                related.sort_unstable();
                related.dedup();
                errors.push(TopologyError::DuplicateIdInCollection {
                    kind: TopologyKind::Shell,
                    slot: slot_u32,
                    related,
                });
            }
            register_shell(inner, region.id, rs, shell_region_owner, b, errors, true);
        }
        // Outer shell must not also appear as a cavity.
        let outer_slot = region.outer_shell.handle().slot();
        if region
            .inner_shells
            .iter()
            .any(|s| s.handle().slot() == outer_slot)
        {
            let mut related = vec![region.provenance.semantic_id()];
            if let Some(shell) = b.shells.get(outer_slot as usize) {
                related.push(shell.provenance.semantic_id());
            }
            related.sort_unstable();
            related.dedup();
            errors.push(TopologyError::ShellOwnershipConflict {
                shell_id: region.outer_shell,
                related,
            });
        }
    }
}

/// Registers a shell as owned by `region_slot`.
///
/// If `must_be_closed` is `true` (cavity shells), the shell must be
/// [`ShellKind::Closed`].
fn register_shell(
    shell_id: ShellId,
    region_id: RegionId,
    region_slot: usize,
    owner: &mut [Option<usize>],
    b: &TopologyBuilder,
    errors: &mut Vec<TopologyError>,
    must_be_closed: bool,
) {
    let ss = shell_id.handle().slot() as usize;
    if ss >= b.shells.len() {
        return; // cross-ref error already reported
    }
    if let Some(prev_region_slot) = owner[ss] {
        if prev_region_slot != region_slot {
            let mut related = vec![b.shells[ss].provenance.semantic_id()];
            if prev_region_slot < b.regions.len() {
                related.push(b.regions[prev_region_slot].provenance.semantic_id());
            }
            if region_slot < b.regions.len() {
                related.push(b.regions[region_slot].provenance.semantic_id());
            }
            related.sort_unstable();
            related.dedup();
            errors.push(TopologyError::ShellOwnershipConflict { shell_id, related });
        }
    } else {
        owner[ss] = Some(region_slot);
    }
    if must_be_closed && b.shells[ss].kind != ShellKind::Closed {
        let mut related = vec![
            b.regions[region_slot].provenance.semantic_id(),
            b.shells[ss].provenance.semantic_id(),
        ];
        related.sort_unstable();
        related.dedup();
        errors.push(TopologyError::OuterShellMustBeClosed {
            region_id,
            shell_id,
            related,
        });
    }
}

/// Checks region ownership uniqueness within bodies.
fn check_body_region_ownership(b: &TopologyBuilder, errors: &mut Vec<TopologyError>) {
    let mut region_body_owner: Vec<Option<usize>> = vec![None; b.regions.len()];
    for body in &b.bodies {
        let bs = body.id.handle().slot() as usize;
        let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
        for &region_id in &body.regions {
            let slot_u32 = region_id.handle().slot();
            if !seen.insert(slot_u32) {
                let mut related = if slot_u32 as usize >= b.regions.len() {
                    vec![]
                } else {
                    vec![b.regions[slot_u32 as usize].provenance.semantic_id()]
                };
                related.push(body.provenance.semantic_id());
                related.sort_unstable();
                related.dedup();
                errors.push(TopologyError::DuplicateIdInCollection {
                    kind: TopologyKind::Region,
                    slot: slot_u32,
                    related,
                });
            }
            let rs = region_id.handle().slot() as usize;
            if rs < b.regions.len() {
                if let Some(prev_body_slot) = region_body_owner[rs] {
                    if prev_body_slot != bs {
                        let mut related = vec![b.regions[rs].provenance.semantic_id()];
                        if prev_body_slot < b.bodies.len() {
                            related.push(b.bodies[prev_body_slot].provenance.semantic_id());
                        }
                        if bs < b.bodies.len() {
                            related.push(b.bodies[bs].provenance.semantic_id());
                        }
                        related.sort_unstable();
                        related.dedup();
                        errors.push(TopologyError::RegionOwnershipConflict { region_id, related });
                    }
                } else {
                    region_body_owner[rs] = Some(bs);
                }
            }
        }
    }
}

/// BFS reachability from all bodies.
///
/// Returns `(shells, faces, loops, coedges, edges, vertices, regions)` boolean
/// vectors indexed by slot; `true` means reachable.
#[allow(clippy::type_complexity)]
fn back_reachable(
    b: &TopologyBuilder,
    back: &BackRefs,
) -> (
    Vec<bool>,
    Vec<bool>,
    Vec<bool>,
    Vec<bool>,
    Vec<bool>,
    Vec<bool>,
    Vec<bool>,
) {
    let mut r_reachable = vec![false; b.regions.len()];
    let mut s_reachable = vec![false; b.shells.len()];
    let mut f_reachable = vec![false; b.faces.len()];
    let mut l_reachable = vec![false; b.loops.len()];
    let mut c_reachable = vec![false; b.coedges.len()];
    let mut e_reachable = vec![false; b.edges.len()];
    let mut v_reachable = vec![false; b.vertices.len()];
    for body in &b.bodies {
        for &rid in &body.regions {
            #[cfg(test)]
            count_reachability_visits(1);
            let rs = rid.handle().slot() as usize;
            if rs >= b.regions.len() {
                continue;
            }
            if r_reachable[rs] {
                continue;
            }
            r_reachable[rs] = true;
            let region = &b.regions[rs];
            #[cfg(test)]
            count_reachability_visits(1);
            mark_shell(
                region.outer_shell,
                b,
                back,
                &mut s_reachable,
                &mut f_reachable,
                &mut l_reachable,
                &mut c_reachable,
                &mut e_reachable,
                &mut v_reachable,
            );
            for &inner in &region.inner_shells {
                #[cfg(test)]
                count_reachability_visits(1);
                mark_shell(
                    inner,
                    b,
                    back,
                    &mut s_reachable,
                    &mut f_reachable,
                    &mut l_reachable,
                    &mut c_reachable,
                    &mut e_reachable,
                    &mut v_reachable,
                );
            }
        }
    }
    (
        s_reachable,
        f_reachable,
        l_reachable,
        c_reachable,
        e_reachable,
        v_reachable,
        r_reachable,
    )
}

#[allow(clippy::too_many_arguments)]
fn mark_shell(
    shell_id: ShellId,
    b: &TopologyBuilder,
    back: &BackRefs,
    s_reachable: &mut [bool],
    f_reachable: &mut [bool],
    l_reachable: &mut [bool],
    c_reachable: &mut [bool],
    e_reachable: &mut [bool],
    v_reachable: &mut [bool],
) {
    #[cfg(test)]
    count_reachability_visits(1);
    let ss = shell_id.handle().slot() as usize;
    if ss >= b.shells.len() {
        return;
    }
    if s_reachable[ss] {
        return;
    }
    s_reachable[ss] = true;
    for &fid in &b.shells[ss].faces {
        #[cfg(test)]
        count_reachability_visits(1);
        let fs = fid.handle().slot() as usize;
        if fs >= b.faces.len() {
            continue;
        }
        if f_reachable[fs] {
            continue;
        }
        f_reachable[fs] = true;
        if let Some(outer_loop_id) = back.face_outer_loop[fs] {
            #[cfg(test)]
            count_reachability_visits(1);
            mark_loop(
                outer_loop_id,
                b,
                back,
                l_reachable,
                c_reachable,
                e_reachable,
                v_reachable,
            );
        }
        for &inner_loop_id in &back.face_inner_loops[fs] {
            #[cfg(test)]
            count_reachability_visits(1);
            mark_loop(
                inner_loop_id,
                b,
                back,
                l_reachable,
                c_reachable,
                e_reachable,
                v_reachable,
            );
        }
    }
}

fn mark_loop(
    loop_id: LoopId,
    b: &TopologyBuilder,
    back: &BackRefs,
    l_reachable: &mut [bool],
    c_reachable: &mut [bool],
    e_reachable: &mut [bool],
    v_reachable: &mut [bool],
) {
    #[cfg(test)]
    count_reachability_visits(1);
    let ls = loop_id.handle().slot() as usize;
    if ls >= b.loops.len() {
        return;
    }
    if l_reachable[ls] {
        return;
    }
    l_reachable[ls] = true;
    for &coedge_id in &back.loop_coedges[ls] {
        #[cfg(test)]
        count_reachability_visits(1);
        let cs = coedge_id.handle().slot() as usize;
        if cs >= b.coedges.len() {
            continue;
        }
        c_reachable[cs] = true;
        let edge_slot = b.coedges[cs].edge.handle().slot() as usize;
        if edge_slot >= b.edges.len() {
            continue;
        }
        e_reachable[edge_slot] = true;
        for &vertex_id in &b.edges[edge_slot].vertices {
            #[cfg(test)]
            count_reachability_visits(1);
            let vertex_slot = vertex_id.handle().slot() as usize;
            if vertex_slot < v_reachable.len() {
                v_reachable[vertex_slot] = true;
            }
        }
    }
}

/// Reports orphan entities (not reachable from any body).
#[allow(clippy::type_complexity)]
fn check_orphan_entities(
    b: &TopologyBuilder,
    reachable: (
        Vec<bool>,
        Vec<bool>,
        Vec<bool>,
        Vec<bool>,
        Vec<bool>,
        Vec<bool>,
        Vec<bool>,
    ),
    errors: &mut Vec<TopologyError>,
) {
    let (s_r, f_r, l_r, c_r, e_r, v_r, r_r) = reachable;
    push_orphans_with_provenance(
        &b.regions,
        &r_r,
        TopologyKind::Region,
        |region| region.id.handle().slot(),
        |region| region.provenance.semantic_id(),
        errors,
    );
    push_orphans_with_provenance(
        &b.shells,
        &s_r,
        TopologyKind::Shell,
        |shell| shell.id.handle().slot(),
        |shell| shell.provenance.semantic_id(),
        errors,
    );
    push_orphans_with_provenance(
        &b.faces,
        &f_r,
        TopologyKind::Face,
        |face| face.id.handle().slot(),
        |face| face.provenance.semantic_id(),
        errors,
    );
    push_orphans_with_provenance(
        &b.loops,
        &l_r,
        TopologyKind::Loop,
        |lp| lp.id.handle().slot(),
        |lp| lp.provenance.semantic_id(),
        errors,
    );
    push_orphans_with_provenance(
        &b.coedges,
        &c_r,
        TopologyKind::Coedge,
        |coedge| coedge.id.handle().slot(),
        |coedge| coedge.provenance.semantic_id(),
        errors,
    );
    push_orphans_with_provenance(
        &b.edges,
        &e_r,
        TopologyKind::Edge,
        |edge| edge.id.handle().slot(),
        |edge| edge.provenance.semantic_id(),
        errors,
    );
    push_orphans_with_provenance(
        &b.vertices,
        &v_r,
        TopologyKind::Vertex,
        |vertex| vertex.id.handle().slot(),
        |vertex| vertex.provenance.semantic_id(),
        errors,
    );
}

fn push_orphans_with_provenance<E, FSlot, FSem>(
    entities: &[E],
    reachable: &[bool],
    kind: TopologyKind,
    slot_of: FSlot,
    semantic_id_of: FSem,
    errors: &mut Vec<TopologyError>,
) where
    FSlot: Fn(&E) -> u32,
    FSem: Fn(&E) -> SemanticId,
{
    for (index, entity) in entities.iter().enumerate() {
        if !reachable[index] {
            errors.push(TopologyError::OrphanEntity {
                kind,
                slot: slot_of(entity),
                related: vec![semantic_id_of(entity)],
            });
        }
    }
}

/// Builds `coedge_shell[coedge_slot] = Option<shell_slot>`.
fn build_coedge_shell(
    b: &TopologyBuilder,
    face_shell_owner: &[Option<usize>],
) -> Vec<Option<usize>> {
    let mut coedge_shell = vec![None; b.coedges.len()];
    for coedge in &b.coedges {
        let ls = coedge.loop_id.handle().slot() as usize;
        if ls >= b.loops.len() {
            continue;
        }
        let fs = b.loops[ls].face.handle().slot() as usize;
        if fs < face_shell_owner.len() {
            let cs = coedge.id.handle().slot() as usize;
            coedge_shell[cs] = face_shell_owner[fs];
        }
    }
    coedge_shell
}

/// Phase 3c: reject edges shared between different shells.
fn check_cross_shell_edges(
    b: &TopologyBuilder,
    back: &BackRefs,
    coedge_shell: &[Option<usize>],
    errors: &mut Vec<TopologyError>,
) {
    for (edge_slot, coedge_ids) in back.edge_coedges.iter().enumerate() {
        let mut shell_slot: Option<usize> = None;
        for &ce_id in coedge_ids {
            let cs = ce_id.handle().slot() as usize;
            let this_shell = coedge_shell.get(cs).copied().flatten();
            match shell_slot {
                None => shell_slot = this_shell,
                Some(s) if Some(s) != this_shell => {
                    let mut related = vec![b.edges[edge_slot].provenance.semantic_id()];
                    if s < b.shells.len() {
                        related.push(b.shells[s].provenance.semantic_id());
                    }
                    if let Some(ts) = this_shell
                        && ts < b.shells.len()
                    {
                        related.push(b.shells[ts].provenance.semantic_id());
                    }
                    related.sort_unstable();
                    related.dedup();
                    errors.push(TopologyError::CrossShellEdge {
                        edge_id: b.edges[edge_slot].id,
                        related,
                    });
                    break;
                }
                _ => {}
            }
        }
    }
}

/// Phase 3d: per-shell orientation pairs, open-shell boundary check, Closed/Open consistency.
fn check_per_shell_edge_semantics(
    b: &TopologyBuilder,
    coedge_shell: &[Option<usize>],
    errors: &mut Vec<TopologyError>,
) {
    let n_s = b.shells.len();
    // Per-shell edge uses retain coedge IDs for lossless diagnostics.
    let mut shell_edge_orients: Vec<Vec<(u32, Orientation, CoedgeId)>> = vec![Vec::new(); n_s];
    for coedge in &b.coedges {
        let cs = coedge.id.handle().slot() as usize;
        if let Some(ss) = coedge_shell.get(cs).copied().flatten() {
            let es = coedge.edge.handle().slot();
            shell_edge_orients[ss].push((es, coedge.orientation, coedge.id));
        }
    }
    for (shell_slot, shell) in b.shells.iter().enumerate() {
        check_shell_kind_semantics(
            shell,
            shell_slot,
            &b.edges,
            &b.coedges,
            &shell_edge_orients,
            errors,
        );
    }
}

fn check_shell_kind_semantics(
    shell: &ShellRaw,
    shell_slot: usize,
    edges: &[EdgeRaw],
    coedges: &[CoedgeRaw],
    shell_edge_orients: &[Vec<(u32, Orientation, CoedgeId)>],
    errors: &mut Vec<TopologyError>,
) {
    if shell_slot >= shell_edge_orients.len() {
        return;
    }
    let edge_orients = &shell_edge_orients[shell_slot];
    // Build per-shell edge orientation map: edge_slot → [orientations]
    // (reuse sorted pairs from edge_orients)
    let mut edge_map: std::collections::BTreeMap<u32, Vec<(Orientation, CoedgeId)>> =
        std::collections::BTreeMap::new();
    for &(es, orient, coedge_id) in edge_orients {
        edge_map.entry(es).or_default().push((orient, coedge_id));
    }
    match shell.kind {
        ShellKind::Closed => {
            check_closed_shell_edges(shell, &edge_map, edges, coedges, errors);
        }
        ShellKind::Open => {
            let has_boundary = edge_map.values().any(|v| v.len() == 1);
            if !has_boundary && !edge_map.is_empty() {
                errors.push(TopologyError::OpenShellHasNoBoundaryEdge {
                    shell_id: shell.id,
                    related: vec![shell.provenance.semantic_id()],
                });
            }
        }
    }
}

fn check_closed_shell_edges(
    shell: &ShellRaw,
    edge_map: &std::collections::BTreeMap<u32, Vec<(Orientation, CoedgeId)>>,
    edges: &[EdgeRaw],
    coedges: &[CoedgeRaw],
    errors: &mut Vec<TopologyError>,
) {
    for (&edge_slot, uses) in edge_map {
        #[cfg(test)]
        count_incidence_visits(1 + uses.len());
        let Some(edge) = edges.get(edge_slot as usize) else {
            continue;
        };
        let related = || {
            let mut related = vec![
                shell.provenance.semantic_id(),
                edge.provenance.semantic_id(),
            ];
            for &(_, coedge_id) in uses {
                if let Some(coedge) = coedges.get(coedge_id.handle().slot() as usize) {
                    related.push(coedge.provenance.semantic_id());
                }
            }
            related.sort_unstable();
            related.dedup();
            related
        };
        if uses.len() != 2 {
            errors.push(TopologyError::InconsistentShellKind {
                shell_id: shell.id,
                related: related(),
            });
            return;
        }
        if uses[0].0 == uses[1].0 {
            errors.push(TopologyError::SameDirectionEdgePair {
                edge_id: edge.id,
                shell_id: shell.id,
                related: related(),
            });
        }
    }
}

/// Phase 3e: shell face-adjacency BFS for connectivity.
fn check_shell_connectivity(
    b: &TopologyBuilder,
    face_shell_owner: &[Option<usize>],
    shell_connectivity_eligible: &[bool],
    errors: &mut Vec<TopologyError>,
) {
    let n_e = b.edges.len();
    let n_f = b.faces.len();

    // Pass 1: O(C) – collect face slots per edge from coedges.
    let mut edge_face_map: Vec<Vec<usize>> = vec![Vec::new(); n_e];
    for coedge in &b.coedges {
        let ls = coedge.loop_id.handle().slot() as usize;
        if ls >= b.loops.len() {
            continue;
        }
        let fs = b.loops[ls].face.handle().slot() as usize;
        if fs >= n_f {
            continue;
        }
        let es = coedge.edge.handle().slot() as usize;
        if es < n_e {
            edge_face_map[es].push(fs);
        }
        #[cfg(test)]
        count_connectivity_visits(1);
    }

    // Pass 2: O(E) – build global face adjacency (only within same shell).
    let mut face_adj: Vec<Vec<usize>> = vec![Vec::new(); n_f];
    for faces in &mut edge_face_map {
        faces.sort_unstable();
        faces.dedup();
        #[cfg(test)]
        count_connectivity_visits(1);
        if faces.len() == 2 {
            let (a, b_face) = (faces[0], faces[1]);
            let a_shell = face_shell_owner.get(a).copied().flatten();
            let b_shell = face_shell_owner.get(b_face).copied().flatten();
            if a_shell.is_some() && a_shell == b_shell {
                face_adj[a].push(b_face);
                face_adj[b_face].push(a);
                #[cfg(test)]
                count_connectivity_visits(1);
            }
        }
    }

    for adj in &mut face_adj {
        adj.sort_unstable();
        adj.dedup();
    }

    // Pass 3: per-shell BFS using face adjacency.
    for shell in &b.shells {
        let ss = shell.id.handle().slot() as usize;
        if !shell_connectivity_eligible.get(ss).copied().unwrap_or(true) {
            continue;
        }
        if shell.faces.len() < 2 {
            continue;
        }
        let face_slots: Vec<usize> = shell
            .faces
            .iter()
            .map(|fid| fid.handle().slot() as usize)
            .collect();
        let shell_face_count = face_slots.len();
        let face_to_local: std::collections::HashMap<usize, usize> = face_slots
            .iter()
            .enumerate()
            .map(|(index, &slot)| (slot, index))
            .collect();

        let mut visited = vec![false; shell_face_count];
        let mut queue = std::collections::VecDeque::new();
        queue.push_back(0usize);
        visited[0] = true;
        while let Some(local) = queue.pop_front() {
            let fs = face_slots[local];
            #[cfg(test)]
            count_connectivity_visits(1);
            for &neighbor_fs in &face_adj[fs] {
                #[cfg(test)]
                count_connectivity_visits(1);
                if let Some(&neighbor_local) = face_to_local.get(&neighbor_fs)
                    && !visited[neighbor_local]
                {
                    visited[neighbor_local] = true;
                    queue.push_back(neighbor_local);
                }
            }
        }
        if visited.iter().any(|&value| !value) {
            errors.push(TopologyError::DisconnectedShell {
                shell_id: shell.id,
                related: vec![shell.provenance.semantic_id()],
            });
        }
    }
}

// ── Phase 4: Arena construction ───────────────────────────────────────────────

#[allow(clippy::too_many_lines)]
fn build_arenas(b: TopologyBuilder, back: BackRefs) -> Result<TopologyStore, TopologyError> {
    let snap = b.generation;
    let lineage = b.lineage;
    let snapshot = b.snapshot;
    let BackRefs {
        loop_coedges,
        face_outer_loop,
        face_inner_loops,
        edge_coedges,
        vertex_edges,
        ..
    } = back;
    let mut v_arena: Arena<Vertex> = Arena::new();
    for (slot, raw) in b.vertices.into_iter().enumerate() {
        v_arena
            .push(
                snap,
                Vertex::new(
                    raw.id,
                    raw.position,
                    raw.tolerance,
                    vertex_edges[slot].clone(),
                    raw.provenance,
                ),
            )
            .map_err(|_| TopologyError::ArenaOverflow)?;
    }
    let mut e_arena: Arena<Edge> = Arena::new();
    for (slot, raw) in b.edges.into_iter().enumerate() {
        e_arena
            .push(
                snap,
                Edge::new(
                    raw.id,
                    raw.curve,
                    raw.parameter_interval,
                    raw.vertices,
                    edge_coedges[slot].clone(),
                    raw.tolerance,
                    raw.provenance,
                ),
            )
            .map_err(|_| TopologyError::ArenaOverflow)?;
    }
    let mut c_arena: Arena<Coedge> = Arena::new();
    for raw in b.coedges {
        c_arena
            .push(
                snap,
                Coedge::new(
                    raw.id,
                    raw.edge,
                    raw.loop_id,
                    raw.orientation,
                    raw.pcurve,
                    raw.provenance,
                ),
            )
            .map_err(|_| TopologyError::ArenaOverflow)?;
    }
    let mut l_arena: Arena<Loop> = Arena::new();
    for (slot, raw) in b.loops.into_iter().enumerate() {
        l_arena
            .push(
                snap,
                Loop::new(
                    raw.id,
                    raw.face,
                    raw.kind,
                    loop_coedges[slot].clone(),
                    raw.provenance,
                ),
            )
            .map_err(|_| TopologyError::ArenaOverflow)?;
    }
    let mut f_arena: Arena<Face> = Arena::new();
    for (slot, raw) in b.faces.into_iter().enumerate() {
        let outer = face_outer_loop[slot].ok_or(TopologyError::MissingOuterLoop {
            face_id: raw.id,
            related: vec![raw.provenance.semantic_id()],
        })?;
        f_arena
            .push(
                snap,
                Face::new(
                    raw.id,
                    raw.surface,
                    raw.orientation,
                    outer,
                    face_inner_loops[slot].clone(),
                    raw.provenance,
                ),
            )
            .map_err(|_| TopologyError::ArenaOverflow)?;
    }
    let mut s_arena: Arena<Shell> = Arena::new();
    for raw in b.shells {
        s_arena
            .push(
                snap,
                Shell::new(raw.id, raw.kind, raw.faces, raw.provenance),
            )
            .map_err(|_| TopologyError::ArenaOverflow)?;
    }
    let mut r_arena: Arena<Region> = Arena::new();
    for raw in b.regions {
        r_arena
            .push(
                snap,
                Region::new(raw.id, raw.outer_shell, raw.inner_shells, raw.provenance),
            )
            .map_err(|_| TopologyError::ArenaOverflow)?;
    }
    let mut b_arena: Arena<Body> = Arena::new();
    for raw in b.bodies {
        b_arena
            .push(snap, Body::new(raw.id, raw.regions, raw.provenance))
            .map_err(|_| TopologyError::ArenaOverflow)?;
    }
    Ok(TopologyStore::new(
        snap, lineage, snapshot, v_arena, e_arena, c_arena, l_arena, f_arena, s_arena, r_arena,
        b_arena,
    ))
}

/// Returns the next slot index, failing if the count would exceed `u32::MAX`.
fn next_slot(current_len: usize) -> Result<u32, TopologyError> {
    u32::try_from(current_len).map_err(|_| TopologyError::ArenaOverflow)
}

/// A trait for extracting the raw (slot, generation, lineage, snapshot) from a typed ID.
trait IdAccess {
    fn slot(self) -> u32;
    fn generation(self) -> u32;
    fn lineage(self) -> TopologyLineageId;
    fn snapshot(self) -> TopologySnapshotId;
}

macro_rules! impl_id_access {
    ($ty:ty) => {
        impl IdAccess for $ty {
            fn slot(self) -> u32 {
                self.handle().slot()
            }
            fn generation(self) -> u32 {
                self.handle().generation()
            }
            fn lineage(self) -> TopologyLineageId {
                self.handle().lineage()
            }
            fn snapshot(self) -> TopologySnapshotId {
                self.handle().snapshot()
            }
        }
    };
}

impl_id_access!(VertexId);
impl_id_access!(EdgeId);
impl_id_access!(FaceId);
impl_id_access!(LoopId);
impl_id_access!(CoedgeId);
impl_id_access!(ShellId);
impl_id_access!(RegionId);
impl_id_access!(BodyId);

/// Validates that a referenced ID is in-bounds and has the correct generation.
#[allow(clippy::too_many_arguments)]
fn check_ref<Id: IdAccess + Copy>(
    id: Id,
    expected_lineage: TopologyLineageId,
    expected_snapshot: TopologySnapshotId,
    expected_gen: u32,
    entity_count: usize,
    kind: TopologyKind,
    referrer_kind: TopologyKind,
    referrer_slot: usize,
    referrer_field: &'static str,
    referrer_index: Option<u32>,
    referrer_semantic_id: Option<SemanticId>,
    errors: &mut Vec<TopologyError>,
) {
    let slot = id.slot();
    let lineage = id.lineage();
    let id_snapshot = id.snapshot();
    let snap = id.generation();
    let referrer = Some(ReferrerContext {
        kind: referrer_kind,
        slot: u32::try_from(referrer_slot).unwrap_or(u32::MAX),
        field: referrer_field,
        index: referrer_index,
        semantic_id: referrer_semantic_id,
    });
    if lineage != expected_lineage {
        errors.push(TopologyError::WrongLineage {
            kind,
            slot,
            handle_lineage: lineage.as_bytes(),
            store_lineage: expected_lineage.as_bytes(),
            referrer,
        });
    } else if id_snapshot != expected_snapshot {
        errors.push(TopologyError::WrongSnapshot {
            kind,
            slot,
            handle_snapshot: id_snapshot.as_bytes(),
            store_snapshot: expected_snapshot.as_bytes(),
            referrer,
        });
    } else if snap != expected_gen {
        errors.push(TopologyError::StaleHandle {
            kind,
            slot,
            handle_generation: snap,
            store_generation: expected_gen,
            referrer,
        });
    } else if (slot as usize) >= entity_count {
        errors.push(TopologyError::MissingEntity {
            kind,
            slot,
            referrer,
        });
    }
}

/// Wraps [`check_ref`] for vertex references (uses `TopologyKind::Vertex`).
#[allow(clippy::too_many_arguments)]
fn check_vertex_ref(
    id: VertexId,
    expected_lineage: TopologyLineageId,
    expected_snapshot: TopologySnapshotId,
    expected_gen: u32,
    n_vertices: usize,
    referrer_kind: TopologyKind,
    referrer_slot: usize,
    referrer_field: &'static str,
    referrer_semantic_id: Option<SemanticId>,
    errors: &mut Vec<TopologyError>,
) {
    check_ref(
        id,
        expected_lineage,
        expected_snapshot,
        expected_gen,
        n_vertices,
        TopologyKind::Vertex,
        referrer_kind,
        referrer_slot,
        referrer_field,
        None,
        referrer_semantic_id,
        errors,
    );
}

/// Validates that a loop's coedge chain forms a closed vertex cycle.
///
/// For each consecutive pair of coedges `(c_i, c_{i+1 mod n})` the end
/// vertex of `c_i` (respecting orientation) must equal the start vertex of
/// `c_{i+1}`.
fn validate_loop_closure(
    loop_id: LoopId,
    loop_semantic_id: SemanticId,
    coedge_ids: &[CoedgeId],
    coedges: &[CoedgeRaw],
    edges: &[EdgeRaw],
    vertices: &[VertexRaw],
    errors: &mut Vec<TopologyError>,
) {
    let n = coedge_ids.len();
    for i in 0..n {
        let curr_cs = coedge_ids[i].handle().slot() as usize;
        let next_cs = coedge_ids[(i + 1) % n].handle().slot() as usize;
        let curr = &coedges[curr_cs];
        let next = &coedges[next_cs];

        let curr_edge_slot = curr.edge.handle().slot() as usize;
        let next_edge_slot = next.edge.handle().slot() as usize;
        let edge_curr = &edges[curr_edge_slot];
        let edge_next = &edges[next_edge_slot];

        let end_v = match curr.orientation {
            Orientation::Forward => edge_curr.vertices[1],
            Orientation::Reversed => edge_curr.vertices[0],
        };
        let start_v = match next.orientation {
            Orientation::Forward => edge_next.vertices[0],
            Orientation::Reversed => edge_next.vertices[1],
        };

        if end_v != start_v {
            let mut related = vec![loop_semantic_id];
            related.push(coedges[curr_cs].provenance.semantic_id());
            related.push(edges[curr_edge_slot].provenance.semantic_id());
            let end_v_slot = match coedges[curr_cs].orientation {
                Orientation::Forward => edges[curr_edge_slot].vertices[1].handle().slot() as usize,
                Orientation::Reversed => edges[curr_edge_slot].vertices[0].handle().slot() as usize,
            };
            if let Some(vertex) = vertices.get(end_v_slot) {
                related.push(vertex.provenance.semantic_id());
            }
            related.push(coedges[next_cs].provenance.semantic_id());
            related.push(edges[next_edge_slot].provenance.semantic_id());
            let start_v_slot = match coedges[next_cs].orientation {
                Orientation::Forward => edges[next_edge_slot].vertices[0].handle().slot() as usize,
                Orientation::Reversed => edges[next_edge_slot].vertices[1].handle().slot() as usize,
            };
            if let Some(vertex) = vertices.get(start_v_slot) {
                related.push(vertex.provenance.semantic_id());
            }
            related.sort_unstable();
            related.dedup();
            errors.push(TopologyError::LoopVertexMismatch {
                loop_id,
                position: i,
                related,
            });
            return; // report the first break only
        }
    }
}
