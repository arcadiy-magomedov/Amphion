//! Typed, generation-checked topology handles.

use serde::{Deserialize, Serialize};

/// A deterministic topology arena slot and generation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct TopologyHandle {
    slot: u32,
    generation: u32,
}

impl TopologyHandle {
    /// Creates a topology handle.
    #[must_use]
    pub const fn new(slot: u32, generation: u32) -> Self {
        Self { slot, generation }
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
            pub const fn new(slot: u32, generation: u32) -> Self {
                Self(TopologyHandle::new(slot, generation))
            }

            /// Returns the untyped handle for deterministic arena access.
            #[must_use]
            pub const fn handle(self) -> TopologyHandle {
                self.0
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
