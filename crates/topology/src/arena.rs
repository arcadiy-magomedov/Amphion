//! Generic generation-checked immutable arena for topology snapshots.
//!
//! Entities are stored at sequential slot indices assigned at insertion time.
//! Every lookup checks that the handle's generation matches the slot's stored
//! generation, which detects handles borrowed from a different snapshot.

/// One storage cell inside an [`Arena`].
#[derive(Clone, Debug)]
struct Slot<T> {
    generation: u32,
    value: T,
}

/// Failure produced by an [`Arena`] lookup.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ArenaLookupError {
    /// The slot index exceeds the arena's current size.
    SlotOutOfRange,
    /// The handle's generation differs from the slot's stored generation.
    StaleGeneration {
        /// Generation encoded in the caller's handle.
        handle: u32,
        /// Generation stored in the arena slot.
        stored: u32,
    },
}

/// Failure produced when an [`Arena`]'s slot space is exhausted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ArenaOverflowError;

/// A generation-checked immutable arena for topology entities.
///
/// Entities are stored in deterministic slot order (insertion order). The
/// arena is populated by [`crate::builder::TopologyBuilder`] and must not be
/// mutated afterwards.
#[derive(Clone, Debug)]
pub(crate) struct Arena<T> {
    slots: Vec<Slot<T>>,
}

impl<T> Arena<T> {
    /// Creates an empty arena.
    pub(crate) fn new() -> Self {
        Self { slots: Vec::new() }
    }

    /// Inserts a value at the next available slot with the given generation
    /// and returns the assigned slot index.
    ///
    /// # Errors
    ///
    /// Returns [`ArenaOverflowError`] if the slot count would exceed
    /// `u32::MAX`.
    pub(crate) fn push(&mut self, generation: u32, value: T) -> Result<u32, ArenaOverflowError> {
        let slot = u32::try_from(self.slots.len()).map_err(|_| ArenaOverflowError)?;
        self.slots.push(Slot { generation, value });
        Ok(slot)
    }

    /// Looks up an entity by slot index and expected generation.
    ///
    /// Returns [`ArenaLookupError::SlotOutOfRange`] when the slot is beyond
    /// the arena bounds and [`ArenaLookupError::StaleGeneration`] when the
    /// stored generation differs from the one encoded in the caller's handle.
    pub(crate) fn try_get(&self, slot: u32, generation: u32) -> Result<&T, ArenaLookupError> {
        let index = usize::try_from(slot).map_err(|_| ArenaLookupError::SlotOutOfRange)?;
        let entry = self
            .slots
            .get(index)
            .ok_or(ArenaLookupError::SlotOutOfRange)?;
        if entry.generation == generation {
            Ok(&entry.value)
        } else {
            Err(ArenaLookupError::StaleGeneration {
                handle: generation,
                stored: entry.generation,
            })
        }
    }

    /// Returns an iterator over all stored entities in deterministic slot
    /// order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = &T> {
        self.slots.iter().map(|s| &s.value)
    }

    /// Returns the number of entities in the arena.
    pub(crate) fn len(&self) -> usize {
        self.slots.len()
    }

    /// Returns `true` if the arena contains no entities.
    #[allow(dead_code)]
    pub(crate) fn is_empty(&self) -> bool {
        self.slots.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{Arena, ArenaLookupError};

    #[test]
    fn sequential_slots_are_assigned() {
        let mut arena: Arena<u32> = Arena::new();
        let s0 = arena.push(1, 100).expect("slot 0");
        let s1 = arena.push(1, 200).expect("slot 1");
        assert_eq!(s0, 0);
        assert_eq!(s1, 1);
        assert_eq!(arena.len(), 2);
    }

    #[test]
    fn correct_generation_succeeds() {
        let mut arena: Arena<&str> = Arena::new();
        let _ = arena.push(7, "hello").expect("push");
        assert_eq!(arena.try_get(0, 7), Ok(&"hello"));
    }

    #[test]
    fn wrong_generation_returns_stale_error() {
        let mut arena: Arena<u8> = Arena::new();
        let _ = arena.push(3, 42u8).expect("push");
        assert_eq!(
            arena.try_get(0, 9),
            Err(ArenaLookupError::StaleGeneration {
                handle: 9,
                stored: 3
            })
        );
    }

    #[test]
    fn out_of_range_slot_returns_error() {
        let arena: Arena<u8> = Arena::new();
        assert_eq!(arena.try_get(0, 0), Err(ArenaLookupError::SlotOutOfRange));
    }

    #[test]
    fn deterministic_iteration_order() {
        let mut arena: Arena<i32> = Arena::new();
        for v in [10, 20, 30] {
            let _ = arena.push(0, v).expect("push");
        }
        let values: Vec<i32> = arena.iter().copied().collect();
        assert_eq!(values, [10, 20, 30]);
    }
}
