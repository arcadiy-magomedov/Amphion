//! Immutable, lineage- and generation-checked snapshot of all topology entities.
//!
//! A [`TopologyStore`] is the single source of truth for one B-Rep snapshot.
//! All entity lookups are generation-checked to detect handles borrowed from a
//! different snapshot. The store is `Send + Sync` and may be shared across
//! threads.

use crate::arena::{Arena, ArenaLookupError};
use crate::entity::{Body, Coedge, Edge, Face, Loop, Region, Shell, Vertex};
use crate::error::TopologyError;
use crate::id::{
    BodyId, CoedgeId, EdgeId, FaceId, LoopId, RegionId, ShellId, TopologyLineageId,
    TopologySnapshotId, VertexId,
};
use crate::reference::TopologyKind;

/// An immutable generation-checked snapshot of all topology entities.
///
/// Build one with [`crate::builder::TopologyBuilder`]. Entities are immutable
/// once the store is created; create a new store to represent a modified model.
///
/// All public types and evaluator traits are [`Send`] + [`Sync`].
#[derive(Clone, Debug)]
pub struct TopologyStore {
    generation: u32,
    lineage: TopologyLineageId,
    snapshot: TopologySnapshotId,
    vertices: Arena<Vertex>,
    edges: Arena<Edge>,
    coedges: Arena<Coedge>,
    loops: Arena<Loop>,
    faces: Arena<Face>,
    shells: Arena<Shell>,
    regions: Arena<Region>,
    bodies: Arena<Body>,
}

impl TopologyStore {
    /// Creates a store from pre-built arenas. Called only by the builder.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(
        generation: u32,
        lineage: TopologyLineageId,
        snapshot: TopologySnapshotId,
        vertices: Arena<Vertex>,
        edges: Arena<Edge>,
        coedges: Arena<Coedge>,
        loops: Arena<Loop>,
        faces: Arena<Face>,
        shells: Arena<Shell>,
        regions: Arena<Region>,
        bodies: Arena<Body>,
    ) -> Self {
        Self {
            generation,
            lineage,
            snapshot,
            vertices,
            edges,
            coedges,
            loops,
            faces,
            shells,
            regions,
            bodies,
        }
    }

    /// Returns the snapshot generation used for stale-handle detection.
    #[must_use]
    pub const fn generation(&self) -> u32 {
        self.generation
    }

    /// Returns the lineage shared by all handles issued from this store.
    #[must_use]
    pub const fn lineage(&self) -> TopologyLineageId {
        self.lineage
    }

    /// Returns the snapshot ID shared by all handles issued from this store.
    #[must_use]
    pub const fn snapshot(&self) -> TopologySnapshotId {
        self.snapshot
    }

    /// Returns the next generation for a follow-up snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::TopologyError::GenerationOverflow`] if this
    /// store's generation is already `u32::MAX`.
    pub fn next_generation(&self) -> Result<u32, crate::error::TopologyError> {
        self.generation
            .checked_add(1)
            .ok_or(crate::error::TopologyError::GenerationOverflow)
    }

    /// Creates a [`crate::builder::TopologyBuilder`] for the next snapshot in
    /// this lineage.
    ///
    /// The caller must supply an explicit [`TopologySnapshotId`] that is
    /// unique to the new snapshot. Two calls to `successor_builder` on the
    /// same store with different snapshot IDs produce independent branches
    /// whose handles are mutually non-interchangeable (`WrongSnapshot`).
    ///
    /// # Errors
    ///
    /// Returns [`TopologyError::GenerationOverflow`] if this store's
    /// generation is `u32::MAX`.
    pub fn successor_builder(
        &self,
        new_snapshot: TopologySnapshotId,
    ) -> Result<crate::builder::TopologyBuilder, TopologyError> {
        let next_gen = self.next_generation()?;
        Ok(crate::builder::TopologyBuilder::_from_lineage_snapshot_gen(
            self.lineage,
            new_snapshot,
            next_gen,
        ))
    }

    // ── Typed generation-checked lookups ─────────────────────────────────────

    /// Looks up a vertex by its generation-checked ID.
    ///
    /// # Errors
    ///
    /// Returns [`TopologyError::WrongLineage`] when lineages differ,
    /// [`TopologyError::WrongSnapshot`] when snapshots differ,
    /// [`TopologyError::StaleHandle`] when generations differ, or
    /// [`TopologyError::MissingEntity`] when the slot is out of range.
    pub fn vertex(&self, id: VertexId) -> Result<&Vertex, TopologyError> {
        lookup(
            &self.vertices,
            id.handle().slot(),
            id.handle().lineage(),
            id.handle().snapshot(),
            id.handle().generation(),
            self.lineage,
            self.snapshot,
            self.generation,
            TopologyKind::Vertex,
        )
    }

    /// Looks up an edge by its generation-checked ID.
    ///
    /// # Errors
    ///
    /// See [`Self::vertex`].
    pub fn edge(&self, id: EdgeId) -> Result<&Edge, TopologyError> {
        lookup(
            &self.edges,
            id.handle().slot(),
            id.handle().lineage(),
            id.handle().snapshot(),
            id.handle().generation(),
            self.lineage,
            self.snapshot,
            self.generation,
            TopologyKind::Edge,
        )
    }

    /// Looks up a coedge by its generation-checked ID.
    ///
    /// # Errors
    ///
    /// See [`Self::vertex`].
    pub fn coedge(&self, id: CoedgeId) -> Result<&Coedge, TopologyError> {
        lookup(
            &self.coedges,
            id.handle().slot(),
            id.handle().lineage(),
            id.handle().snapshot(),
            id.handle().generation(),
            self.lineage,
            self.snapshot,
            self.generation,
            TopologyKind::Coedge,
        )
    }

    /// Looks up a loop by its generation-checked ID.
    ///
    /// The method is named `get_loop` because `loop` is a reserved keyword.
    ///
    /// # Errors
    ///
    /// See [`Self::vertex`].
    pub fn get_loop(&self, id: LoopId) -> Result<&Loop, TopologyError> {
        lookup(
            &self.loops,
            id.handle().slot(),
            id.handle().lineage(),
            id.handle().snapshot(),
            id.handle().generation(),
            self.lineage,
            self.snapshot,
            self.generation,
            TopologyKind::Loop,
        )
    }

    /// Looks up a face by its generation-checked ID.
    ///
    /// # Errors
    ///
    /// See [`Self::vertex`].
    pub fn face(&self, id: FaceId) -> Result<&Face, TopologyError> {
        lookup(
            &self.faces,
            id.handle().slot(),
            id.handle().lineage(),
            id.handle().snapshot(),
            id.handle().generation(),
            self.lineage,
            self.snapshot,
            self.generation,
            TopologyKind::Face,
        )
    }

    /// Looks up a shell by its generation-checked ID.
    ///
    /// # Errors
    ///
    /// See [`Self::vertex`].
    pub fn shell(&self, id: ShellId) -> Result<&Shell, TopologyError> {
        lookup(
            &self.shells,
            id.handle().slot(),
            id.handle().lineage(),
            id.handle().snapshot(),
            id.handle().generation(),
            self.lineage,
            self.snapshot,
            self.generation,
            TopologyKind::Shell,
        )
    }

    /// Looks up a region by its generation-checked ID.
    ///
    /// # Errors
    ///
    /// See [`Self::vertex`].
    pub fn region(&self, id: RegionId) -> Result<&Region, TopologyError> {
        lookup(
            &self.regions,
            id.handle().slot(),
            id.handle().lineage(),
            id.handle().snapshot(),
            id.handle().generation(),
            self.lineage,
            self.snapshot,
            self.generation,
            TopologyKind::Region,
        )
    }

    /// Looks up a body by its generation-checked ID.
    ///
    /// # Errors
    ///
    /// See [`Self::vertex`].
    pub fn body(&self, id: BodyId) -> Result<&Body, TopologyError> {
        lookup(
            &self.bodies,
            id.handle().slot(),
            id.handle().lineage(),
            id.handle().snapshot(),
            id.handle().generation(),
            self.lineage,
            self.snapshot,
            self.generation,
            TopologyKind::Body,
        )
    }

    // ── Deterministic iterators ───────────────────────────────────────────────

    /// Returns all vertices in deterministic slot order.
    pub fn vertices(&self) -> impl Iterator<Item = &Vertex> {
        self.vertices.iter()
    }

    /// Returns all edges in deterministic slot order.
    pub fn edges(&self) -> impl Iterator<Item = &Edge> {
        self.edges.iter()
    }

    /// Returns all coedges in deterministic slot order.
    pub fn coedges(&self) -> impl Iterator<Item = &Coedge> {
        self.coedges.iter()
    }

    /// Returns all loops in deterministic slot order.
    pub fn loops(&self) -> impl Iterator<Item = &Loop> {
        self.loops.iter()
    }

    /// Returns all faces in deterministic slot order.
    pub fn faces(&self) -> impl Iterator<Item = &Face> {
        self.faces.iter()
    }

    /// Returns all shells in deterministic slot order.
    pub fn shells(&self) -> impl Iterator<Item = &Shell> {
        self.shells.iter()
    }

    /// Returns all regions in deterministic slot order.
    pub fn regions(&self) -> impl Iterator<Item = &Region> {
        self.regions.iter()
    }

    /// Returns all bodies in deterministic slot order.
    pub fn bodies(&self) -> impl Iterator<Item = &Body> {
        self.bodies.iter()
    }

    // ── Counts ────────────────────────────────────────────────────────────────

    /// Returns the number of vertices.
    #[must_use]
    pub fn vertex_count(&self) -> usize {
        self.vertices.len()
    }

    /// Returns the number of edges.
    #[must_use]
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Returns the number of coedges.
    #[must_use]
    pub fn coedge_count(&self) -> usize {
        self.coedges.len()
    }

    /// Returns the number of loops.
    #[must_use]
    pub fn loop_count(&self) -> usize {
        self.loops.len()
    }

    /// Returns the number of faces.
    #[must_use]
    pub fn face_count(&self) -> usize {
        self.faces.len()
    }

    /// Returns the number of shells.
    #[must_use]
    pub fn shell_count(&self) -> usize {
        self.shells.len()
    }

    /// Returns the number of regions.
    #[must_use]
    pub fn region_count(&self) -> usize {
        self.regions.len()
    }

    /// Returns the number of bodies.
    #[must_use]
    pub fn body_count(&self) -> usize {
        self.bodies.len()
    }
}

/// Translates an [`ArenaLookupError`] into a [`TopologyError`].
///
/// Priority order: `WrongLineage` > `WrongSnapshot` > `StaleHandle` > `MissingEntity`.
#[allow(clippy::too_many_arguments)]
fn lookup<T>(
    arena: &Arena<T>,
    slot: u32,
    handle_lineage: TopologyLineageId,
    handle_snapshot: TopologySnapshotId,
    generation: u32,
    store_lineage: TopologyLineageId,
    store_snapshot: TopologySnapshotId,
    store_generation: u32,
    kind: TopologyKind,
) -> Result<&T, TopologyError> {
    if handle_lineage != store_lineage {
        return Err(TopologyError::WrongLineage {
            kind,
            slot,
            handle_lineage: handle_lineage.as_bytes(),
            store_lineage: store_lineage.as_bytes(),
            referrer: None,
        });
    }
    if handle_snapshot != store_snapshot {
        return Err(TopologyError::WrongSnapshot {
            kind,
            slot,
            handle_snapshot: handle_snapshot.as_bytes(),
            store_snapshot: store_snapshot.as_bytes(),
            referrer: None,
        });
    }
    if generation != store_generation {
        return Err(TopologyError::StaleHandle {
            kind,
            slot,
            handle_generation: generation,
            store_generation,
            referrer: None,
        });
    }
    match arena.try_get(slot, generation) {
        Ok(value) => Ok(value),
        Err(ArenaLookupError::SlotOutOfRange) => Err(TopologyError::MissingEntity {
            kind,
            slot,
            referrer: None,
        }),
        Err(ArenaLookupError::StaleGeneration { handle, stored }) => {
            Err(TopologyError::StaleHandle {
                kind,
                slot,
                handle_generation: handle,
                store_generation: stored,
                referrer: None,
            })
        }
    }
}
