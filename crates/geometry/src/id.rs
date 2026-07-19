//! Typed, generation-checked geometry handles.

use serde::{Deserialize, Serialize};

/// A deterministic arena slot and generation.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct GeometryHandle {
    slot: u32,
    generation: u32,
}

impl GeometryHandle {
    /// Creates a handle from a deterministic slot and generation.
    #[must_use]
    pub const fn new(slot: u32, generation: u32) -> Self {
        Self { slot, generation }
    }

    /// Returns the arena slot.
    #[must_use]
    pub const fn slot(self) -> u32 {
        self.slot
    }

    /// Returns the generation used to reject stale handles.
    #[must_use]
    pub const fn generation(self) -> u32 {
        self.generation
    }
}

macro_rules! geometry_id {
    ($name:ident, $docs:literal) => {
        #[doc = $docs]
        #[derive(
            Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize, Deserialize,
        )]
        pub struct $name(GeometryHandle);

        impl $name {
            /// Creates a typed geometry ID.
            #[must_use]
            pub const fn new(slot: u32, generation: u32) -> Self {
                Self(GeometryHandle::new(slot, generation))
            }

            /// Returns the untyped handle for serialization and arena access.
            #[must_use]
            pub const fn handle(self) -> GeometryHandle {
                self.0
            }
        }
    };
}

geometry_id!(Curve2Id, "Identity of a canonical parameter-space curve.");
geometry_id!(Curve3Id, "Identity of a canonical model-space curve.");
geometry_id!(SurfaceId, "Identity of a canonical model-space surface.");
