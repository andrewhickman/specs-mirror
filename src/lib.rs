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

/// A `specs` storage intended for synchronisation with external libraries.
pub struct MirroredStorage<C, D: EventData<C>, S = DenseVecStorage<C>> {
    chan: EventChannel<Event<C, D>>,
    store: S,
}

impl<C, D: EventData<C>, S> MirroredStorage<C, D, S> {
    /// Get access to the event channel.
    pub fn chan(&self) -> &EventChannel<Event<C, D>> {
        &self.chan
    }

    // Read events from the event channel.
    pub fn read_events(&self, reader: &mut ReaderId<Event<C, D>>) -> EventIterator<Event<C, D>> {
        self.chan().read(reader)
    }

    /// Get mutable access to the event channel.
    pub fn chan_mut(&mut self) -> &mut EventChannel<Event<C, D>> {
        &mut self.chan
    }

    /// Register a reader with the event channel.
    pub fn register_reader(&mut self) -> ReaderId<Event<C, D>> {
        self.chan_mut().register_reader()
    }
}

/// An event produced when components are inserted and removed from a
/// [`MirroredStorage`](struct.MirroredStorage.html).
/// The type parameter `D` controls what data is sent with this event.
pub enum Event<C, D: EventData<C>> {
    Inserted(Index, D::InsertData),
    Removed(Index, D::RemoveData),
}

impl<C, D: EventData<C>> Event<C, D> {
    fn inserted(id: Index, comp: &mut C) -> Self {
        Event::Inserted(id, D::insert_data(id, comp))
    }

    fn removed(id: Index, comp: &mut C) -> Self {
        Event::Removed(id, D::remove_data(id, comp))
    }
}

/// Describes data that can be send along with an [`Event`](enum.Event.html).
pub trait EventData<C>: 'static {
    /// The data that will be sent with insertion events.
    type InsertData: Send + Sync + 'static;
    /// The data that will be sent with removal events.
    type RemoveData: Send + Sync + 'static;

    /// Constructs an instance of `InsertData` from the component being inserted and its index.
    fn insert_data(id: Index, comp: &mut C) -> Self::InsertData;
    /// Constructs an instance of `RemoveData` from the component being removed and its index.
    fn remove_data(id: Index, comp: &mut C) -> Self::RemoveData;
}

impl<C, D, S> Default for MirroredStorage<C, D, S>
where
    C: Component,
    D: EventData<C>,
    S: TryDefault,
{
    fn default() -> Self {
        MirroredStorage {
            chan: EventChannel::new(),
            store: S::unwrap_default(),
        }
    }
}

impl<C, D, S> UnprotectedStorage<C> for MirroredStorage<C, D, S>
where
    C: Component,
    D: EventData<C>,
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
        self.chan.single_write(Event::inserted(id, &mut comp));
        self.store.insert(id, comp);
    }

    unsafe fn remove(&mut self, id: Index) -> C {
        let mut comp = self.store.remove(id);
        self.chan.single_write(Event::removed(id, &mut comp));
        comp
    }
}

/// An implementation of `EventData` which does not provide any data.
pub struct NoData;

impl<C> EventData<C> for NoData {
    type InsertData = ();
    type RemoveData = ();

    fn insert_data(_: Index, _: &mut C) -> Self::InsertData {
        ()
    }

    fn remove_data(_: Index, _: &mut C) -> Self::RemoveData {
        ()
    }
}

/// An implementation of `EventData` which provides a clone of inserted or removed components.
pub struct CloneData;

impl<C> EventData<C> for CloneData
where
    C: Clone + shrev::Event,
{
    type InsertData = C;
    type RemoveData = C;

    fn insert_data(_: Index, comp: &mut C) -> Self::InsertData {
        comp.clone()
    }

    fn remove_data(_: Index, comp: &mut C) -> Self::RemoveData {
        comp.clone()
    }
}

/// Extension methods for [`Storage`] to help read events from [`MirroredStorage`].
///
/// [`MirroredStorage`]: struct.MirroredStorage.html
/// [`Storage`]: ../specs/storage/struct.Storage.html
pub trait StorageExt<C, D: EventData<C>> {
    fn read_events(&self, reader: &mut ReaderId<Event<C, D>>) -> EventIterator<Event<C, D>>;
}

/// Extension methods for [`Storage`] to help read events from [`MirroredStorage`].
///
/// [`MirroredStorage`]: struct.MirroredStorage.html
/// [`Storage`]: ../specs/storage/struct.Storage.html
pub trait StorageMutExt<C, D: EventData<C>>: StorageExt<C, D> {
    fn register_reader(&mut self) -> ReaderId<Event<C, D>>;
}

impl<'a, C, D, S, M> StorageExt<C, D> for Storage<'a, C, M>
where
    C: Component<Storage = MirroredStorage<C, D, S>>,
    D: EventData<C>,
    S: UnprotectedStorage<C> + Any + Send + Sync,
    M: Deref<Target = MaskedStorage<C>>,
{
    fn read_events(&self, reader: &mut ReaderId<Event<C, D>>) -> EventIterator<Event<C, D>> {
        self.unprotected_storage().read_events(reader)
    }
}

impl<'a, C, D, S, M> StorageMutExt<C, D> for Storage<'a, C, M>
where
    C: Component<Storage = MirroredStorage<C, D, S>>,
    D: EventData<C>,
    S: UnprotectedStorage<C> + Any + Send + Sync,
    M: DerefMut<Target = MaskedStorage<C>>,
{
    fn register_reader(&mut self) -> ReaderId<Event<C, D>> {
        self.unprotected_storage_mut().register_reader()
    }
}