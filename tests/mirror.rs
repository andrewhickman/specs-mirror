extern crate shrev;
extern crate specs;
extern crate specs_mirror;

use shrev::{EventChannel, ReaderId};
use specs::prelude::*;
use specs::world::Index;
use specs_mirror::*;

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Comp(Arc<str>);

impl Component for Comp {
    type Storage = MirroredStorage<Self>;
}

enum CompEvent {
    Inserted(Index, Comp),
    Removed(Index, Comp),
}

impl Mirrored for Comp {
    type State = Comp;
    type Event = CompEvent;

    fn insert(&mut self, chan: &mut EventChannel<Self::Event>, id: Index) {
        chan.single_write(CompEvent::Inserted(id, self.clone()));
    }

    fn remove(&mut self, chan: &mut EventChannel<Self::Event>, id: Index) {
        chan.single_write(CompEvent::Removed(id, self.clone()));
    }

    fn modify(&mut self, chan: &mut EventChannel<Self::Event>, entity: Entity, state: Self::State) {
        chan.single_write(CompEvent::Removed(entity.id(), self.clone()));
        *self = state;
        chan.single_write(CompEvent::Inserted(entity.id(), self.clone()));
    }
}

struct CompSystem {
    reader: ReaderId<CompEvent>,
    store: HashMap<Index, Comp>,
}

impl CompSystem {
    fn new(mut store: WriteStorage<Comp>) -> Self {
        let reader = store.register_reader();
        let store = HashMap::new();
        CompSystem { reader, store }
    }
}

impl<'a> System<'a> for CompSystem {
    type SystemData = ReadStorage<'a, Comp>;

    fn run(&mut self, comp: Self::SystemData) {
        for event in comp.read_events(&mut self.reader) {
            match *event {
                CompEvent::Inserted(id, ref data) => {
                    assert!(self.store.insert(id, data.clone()).is_none())
                }
                CompEvent::Removed(id, ref data) => {
                    assert_eq!(self.store.remove(&id), Some(data.clone()))
                }
            }
        }
    }
}

const ACTIONS: usize = 4;

fn modify(comps: &mut WriteStorage<Comp>, ent: Entity, i: usize) {
    match i % ACTIONS {
        0 => {
            comps
                .insert(ent, Comp(Arc::from(ent.id().to_string())))
                .unwrap();
        }
        1 => {
            comps.remove(ent);
        }
        3 => {
            comps
                .modify(ent, Comp(Arc::from(ent.id().to_string())));
        }
        _ => (),
    }
}

fn test_synced(ents: Entities, left: ReadStorage<Comp>, right: &HashMap<Index, Comp>) {
    let left: HashMap<Index, Comp> = (&*ents, &left)
        .join()
        .map(|(ent, comp)| (ent.id(), comp.clone()))
        .collect();
    assert_eq!(&left, right);
}

#[test]
fn test() {
    const N: u32 = 6;

    let mut world = World::new();
    world.register::<Comp>();

    let mut sys = CompSystem::new(world.write_storage::<Comp>());

    let entities: Vec<Entity> = world.create_iter().take(ACTIONS.pow(N)).collect();

    sys.run_now(&mut world.res);
    test_synced(world.entities(), world.read_storage::<Comp>(), &sys.store);

    {
        let mut comps = world.write_storage::<Comp>();
        for (mut i, &ent) in entities.iter().enumerate() {
            for _ in 0..N {
                modify(&mut comps, ent, i);
                i /= ACTIONS;
            }
            assert_eq!(i, 0);
        }
    }

    sys.run_now(&mut world.res);
    test_synced(world.entities(), world.read_storage::<Comp>(), &sys.store);

    {
        let mut comps = world.write_storage::<Comp>();
        for (mut i, &ent) in entities.iter().rev().enumerate() {
            for _ in 0..N {
                modify(&mut comps, ent, i);
                i /= ACTIONS;
            }
            assert_eq!(i, 0);
        }
    }

    sys.run_now(&mut world.res);
    test_synced(world.entities(), world.read_storage::<Comp>(), &sys.store);
}
