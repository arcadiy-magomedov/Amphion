//! Typed, generation-checked topology handles.

use amphion_foundation::SemanticId;
use serde::{Deserialize, Serialize};

/// A 128-bit caller-supplied lineage identifier for a topology snapshot chain.
///
/// Stable across all snapshots of a document chain. Two stores with different
/// lineage IDs are always non-interchangeable.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TopologyLineageId(SemanticId);

impl TopologyLineageId {
    /// Creates a lineage ID from a deterministic 128-bit semantic identifier.
    #[must_use]
    pub const fn new(id: SemanticId) -> Self {
        Self(id)
    }

    /// Returns the underlying 128-bit semantic identifier.
    #[must_use]
    pub const fn as_semantic_id(self) -> SemanticId {
        self.0
    }

    /// Returns the canonical 16-byte representation of this lineage.
    #[must_use]
    pub const fn as_bytes(self) -> [u8; 16] {
        self.0.into_bytes()
    }
}

/// A 128-bit caller-supplied per-snapshot identifier.
///
/// Unlike [`TopologyLineageId`], which is stable across all snapshots in a
/// document chain, a `TopologySnapshotId` is unique to one specific snapshot
/// instance. Two branches produced from the same predecessor by calling
/// `successor_builder` twice with different snapshot IDs are both at generation
/// N+1 but have distinct snapshot IDs — handles from one are rejected by the
/// other via [`TopologyError::WrongSnapshot`].
///
/// Every root store and every successor store must receive an explicit,
/// caller-supplied `TopologySnapshotId`. There is no default value.
///
/// [`TopologyError::WrongSnapshot`]: crate::error::TopologyError::WrongSnapshot
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TopologySnapshotId(SemanticId);

impl TopologySnapshotId {
    /// Creates a snapshot ID from a deterministic 128-bit semantic identifier.
    ///
    /// The caller is responsible for deriving a unique value per snapshot
    /// instance (e.g. a per-operation UUID). Do **not** reuse the same
    /// snapshot ID for two independent builds even if their lineage and
    /// generation happen to match.
    #[must_use]
    pub const fn new(id: SemanticId) -> Self {
        Self(id)
    }

    /// Returns the underlying semantic identifier.
    #[must_use]
    pub const fn as_semantic_id(self) -> SemanticId {
        self.0
    }

    /// Returns the canonical 16-byte representation.
    #[must_use]
    pub const fn as_bytes(self) -> [u8; 16] {
        self.0.into_bytes()
    }
}

/// A deterministic topology arena slot, generation, lineage, and snapshot.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TopologyHandle {
    slot: u32,
    generation: u32,
    lineage: TopologyLineageId,
    snapshot: TopologySnapshotId,
}

impl TopologyHandle {
    /// Creates a topology handle.
    #[must_use]
    pub const fn new(
        slot: u32,
        generation: u32,
        lineage: TopologyLineageId,
        snapshot: TopologySnapshotId,
    ) -> Self {
        Self {
            slot,
            generation,
            lineage,
            snapshot,
        }
    }

    /// Returns the deterministic arena slot.
    #[must_use]
    pub const fn slot(self) -> u32 {
        self.slot
    }

    /// Returns the generation used to reject stale references.
    #[must_use]
    pub const fn generation(self) -> u32 {
        self.generation
    }

    /// Returns the lineage used to reject cross-store references.
    #[must_use]
    pub const fn lineage(self) -> TopologyLineageId {
        self.lineage
    }

    /// Returns the snapshot ID used to reject cross-branch references.
    #[must_use]
    pub const fn snapshot(self) -> TopologySnapshotId {
        self.snapshot
    }
}

macro_rules! topology_id {
    ($name:ident, $docs:literal) => {
        #[doc = $docs]
        #[derive(
            Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize,
        )]
        pub struct $name(TopologyHandle);

        impl $name {
            /// Creates a typed topology ID.
            #[must_use]
            pub const fn new(
                slot: u32,
                generation: u32,
                lineage: TopologyLineageId,
                snapshot: TopologySnapshotId,
            ) -> Self {
                Self(TopologyHandle::new(slot, generation, lineage, snapshot))
            }

            /// Returns the untyped handle for deterministic arena access.
            #[must_use]
            pub const fn handle(self) -> TopologyHandle {
                self.0
            }

            /// Returns the lineage carried by this ID.
            #[must_use]
            pub const fn lineage(self) -> TopologyLineageId {
                self.0.lineage()
            }

            /// Returns the snapshot ID carried by this ID.
            #[must_use]
            pub const fn snapshot(self) -> TopologySnapshotId {
                self.0.snapshot()
            }
        }
    };
}

topology_id!(BodyId, "Identity of a B-Rep body.");
topology_id!(RegionId, "Identity of a connected material region.");
topology_id!(ShellId, "Identity of an oriented face shell.");
topology_id!(FaceId, "Identity of a trimmed oriented surface.");
topology_id!(LoopId, "Identity of an ordered face-boundary loop.");
topology_id!(CoedgeId, "Identity of one oriented use of an edge.");
topology_id!(EdgeId, "Identity of a bounded model-space curve.");
topology_id!(VertexId, "Identity of a topological point.");
