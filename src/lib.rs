#![deny(missing_docs)]
//! This crate provides [`MirroredStorage`], an implementation of a [`specs`] storage
//! that can track additions, removals and changes to the component it contains.
//! 
//! [`MirroredStorage`]: struct.MirroredStorage.html
//! [`specs`]: https://crates.io/crates/specs

extern crate hibitset;
extern crate shrev;
extern crate specs;

use hibitset::BitSetLike;
use shrev::{EventChannel, EventIterator};
use specs::prelude::*;
use specs::storage::{TryDefault, MaskedStorage, UnprotectedStorage};
use specs::world::Index;

use std::any::Any;
use std::ops::{Deref, DerefMut};

/// A [`specs`] storage intended for synchronisation with external libraries.
/// 
/// [`specs`]: https://crates.io/crates/specs
pub struct MirroredStorage<C: Mirrored, S = DenseVecStorage<C>> {
    chan: EventChannel<C::Event>,
    store: S,
}

/// Components that can be tracked in a [`MirroredStorage`].
///
/// [`MirroredStorage`]: struct.MirroredStorage.html
pub trait Mirrored {
    /// Data used to modify this component.
    type State;

    /// The event type for reporting changes to this component.
    type Event: shrev::Event;

    /// Called when inserting the component.
    /// This method should not be called directly.
    fn insert(&mut self, chan: &mut EventChannel<Self::Event>, id: Index);

    /// Called when removing the component.
    /// This method should not be called directly.
    fn remove(&mut self, chan: &mut EventChannel<Self::Event>, id: Index);

    /// Called by [`StorageMutExt::modify`] when modifying the component.
    /// This method should not be called directly.
    ///
    /// [`StorageMutExt::modify`]: trait.StorageMutExt.html#tymethod.modify
    fn modify(&mut self, chan: &mut EventChannel<Self::Event>, entity: Entity, state: Self::State);
}

impl<C: Mirrored, S> MirroredStorage<C, S> {
    /// Get access to the event channel.
    fn chan(&self) -> &EventChannel<C::Event> {
        &self.chan
    }

    /// Get mutable access to the event channel.
    fn chan_mut(&mut self) -> &mut EventChannel<C::Event> {
        &mut self.chan
    }
}

impl<C: Mirrored, S: UnprotectedStorage<C>> MirroredStorage<C, S> {
    /// Modify the component at the given index.
    unsafe fn modify(&mut self, entity: Entity, state: C::State) {
        self.store.get_mut(entity.id()).modify(&mut self.chan, entity, state)
    }
}

impl<C: Mirrored, S> Default for MirroredStorage<C, S>
where
    S: TryDefault,
{
    fn default() -> Self {
        MirroredStorage {
            chan: EventChannel::new(),
            store: S::unwrap_default(),
        }
    }
}

impl<C, S> UnprotectedStorage<C> for MirroredStorage<C, S>
where
    C: Mirrored + Component,
    S: UnprotectedStorage<C>,
{
    unsafe fn clean<B>(&mut self, has: B)
    where
        B: BitSetLike,
    {
        self.store.clean(has)
    }

    unsafe fn get(&self, id: Index) -> &C {
        self.store.get(id)
    }

    unsafe fn get_mut(&mut self, id: Index) -> &mut C {
        self.store.get_mut(id)
    }

    unsafe fn insert(&mut self, id: Index, mut comp: C) {
        comp.insert(&mut self.chan, id);
        self.store.insert(id, comp);
    }

    unsafe fn remove(&mut self, id: Index) -> C {
        let mut comp = self.store.remove(id);
        comp.remove(&mut self.chan, id);
        comp
    }
}

/// Extension methods for [`Storage`] to help read events from [`MirroredStorage`].
///
/// [`MirroredStorage`]: struct.MirroredStorage.html
/// [`Storage`]: https://docs.rs/specs/0.11.2/specs/storage/struct.Storage.html
pub trait StorageExt<C: Mirrored> {
    /// Read insertion and removal events from the event channel.
    fn read_events(&self, reader: &mut ReaderId<C::Event>) -> EventIterator<C::Event>;
}

/// Extension methods for [`Storage`] to help read events from [`MirroredStorage`].
///
/// [`MirroredStorage`]: struct.MirroredStorage.html
/// [`Storage`]: https://docs.rs/specs/0.11.2/specs/storage/struct.Storage.html
pub trait StorageMutExt<C: Mirrored>: StorageExt<C> {
    /// Register a new reader of insertion and removal events.
    fn register_reader(&mut self) -> ReaderId<C::Event>;

    /// Update an entity with a new state. If the entity is not in this storage the
    /// state is returned.
    fn modify(&mut self, entity: Entity, state: C::State) -> Option<C::State>;
}

impl<'a, C, S, D> StorageExt<C> for Storage<'a, C, D>
where
    C: Mirrored + Component<Storage = MirroredStorage<C, S>>,
    S: UnprotectedStorage<C> + Any + Send + Sync,
    D: Deref<Target = MaskedStorage<C>>,
{
    fn read_events(&self, reader: &mut ReaderId<C::Event>) -> EventIterator<C::Event> {
        self.unprotected_storage().chan().read(reader)
    }
}

impl<'a, C, S, D> StorageMutExt<C> for Storage<'a, C, D>
where
    C: Mirrored + Component<Storage = MirroredStorage<C, S>>,
    S: UnprotectedStorage<C> + Any + Send + Sync,
    D: DerefMut<Target = MaskedStorage<C>>,
{
    fn register_reader(&mut self) -> ReaderId<C::Event> {
        self.unprotected_storage_mut().chan_mut().register_reader()
    }

    fn modify(&mut self, entity: Entity, state: C::State) -> Option<C::State> {
        if self.contains(entity) {
            unsafe {
                self.unprotected_storage_mut().modify(entity, state);
            }
            None
        } else {
            return Some(state);
        }
    }
}