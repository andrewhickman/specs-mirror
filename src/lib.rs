extern crate hibitset;
extern crate shrev;
extern crate specs;

use hibitset::BitSetLike;
use shrev::EventChannel;
use specs::prelude::*;
use specs::storage::{TryDefault, UnprotectedStorage};
use specs::world::Index;

/// A `specs` storage intended for synchronisation with external libraries.
pub struct MirroredStorage<C, D: EventData<C>, S = DenseVecStorage<C>> {
    chan: EventChannel<Event<C, D>>,
    store: S,
}

impl<C, D: EventData<C>, S> MirroredStorage<C, D, S> {
    pub fn chan(&self) -> &EventChannel<Event<C, D>> {
        &self.chan
    }

    pub fn chan_mut(&mut self) -> &mut EventChannel<Event<C, D>> {
        &mut self.chan
    }
}

/// An event produced when components are inserted and removed from a `MirroredStorage`.
/// The type parameter `D` controls what data is sent with this event.
pub enum Event<C, D: EventData<C>> {
    Inserted(D::InsertData),
    Removed(D::RemoveData),
}

impl<C, D: EventData<C>> Event<C, D> {
    fn inserted(id: Index, comp: &mut C) -> Self {
        Event::Inserted(D::insert_data(id, comp))
    }

    fn removed(id: Index, comp: &mut C) -> Self {
        Event::Removed(D::remove_data(id, comp))
    }
}

/// Data that can be send along with an `Event`.
pub trait EventData<C>: 'static {
    type InsertData: Send + Sync + 'static;
    type RemoveData: Send + Sync + 'static;

    fn insert_data(id: Index, comp: &mut C) -> Self::InsertData;
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

/// An implementation of `EventData` which provides the `Index` of inserted and removed
/// components.
pub struct IndexData;

impl<C> EventData<C> for IndexData {
    type InsertData = Index;
    type RemoveData = Index;

    fn insert_data(id: Index, _: &mut C) -> Self::InsertData {
        id
    }

    fn remove_data(id: Index, _: &mut C) -> Self::RemoveData {
        id
    }
}

/// An implementation of `EventData` which provides both the index and a clone of inserted or 
/// removed components.
pub struct CloneData;

impl<C> EventData<C> for CloneData
where
    C: Clone + shrev::Event,
{
    type InsertData = (Index, C);
    type RemoveData = (Index, C);

    fn insert_data(id: Index, comp: &mut C) -> Self::InsertData {
        (id, comp.clone())
    }

    fn remove_data(id: Index, comp: &mut C) -> Self::RemoveData {
        (id, comp.clone())
    }
}