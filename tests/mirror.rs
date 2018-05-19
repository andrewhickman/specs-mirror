extern crate shrev;
extern crate specs;
extern crate specs_mirror;

use shrev::ReaderId;
use specs::prelude::*;
use specs::world::Index;
use specs_mirror::{CloneData, Event, MirroredStorage};

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Comp(Arc<str>);

impl Component for Comp {
    type Storage = MirroredStorage<Self, CloneData>;
}

struct CompSystem {
    reader: ReaderId<Event<Comp, CloneData>>,
    store: HashMap<Index, Comp>,
}

impl CompSystem {
    fn new(mut store: WriteStorage<Comp>) -> Self {
        let reader = store.unprotected_storage_mut().chan_mut().register_reader();
        let store = HashMap::new();
        CompSystem { reader, store }
    }
}

impl<'a> System<'a> for CompSystem {
    type SystemData = ReadStorage<'a, Comp>;

    fn run(&mut self, comp: Self::SystemData) {
        for event in comp.unprotected_storage().read(&mut self.reader) {
            match *event {
                Event::Inserted((id, ref data)) => {
                    assert!(self.store.insert(id, data.clone()).is_none())
                }
                Event::Removed((id, ref data)) => {
                    assert_eq!(self.store.remove(&id), Some(data.clone()))
                }
            }
        }
    }
}

fn modify(comps: &mut WriteStorage<Comp>, ent: Entity, i: usize) {
    match i % 3 {
        0 => {
            comps
                .insert(ent, Comp(Arc::from(ent.id().to_string())))
                .unwrap();
        }
        1 => {
            comps.remove(ent);
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
    const N: u32 = 8;

    let mut world = World::new();
    world.register::<Comp>();

    let mut sys = CompSystem::new(world.write_storage::<Comp>());

    let entities: Vec<Entity> = world.create_iter().take(3usize.pow(N)).collect();

    sys.run_now(&mut world.res);
    test_synced(world.entities(), world.read_storage::<Comp>(), &sys.store);

    {
        let mut comps = world.write_storage::<Comp>();
        for (mut i, &ent) in entities.iter().enumerate() {
            for _ in 0..N {
                modify(&mut comps, ent, i);
                i /= 3;
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
                i /= 3;
            }
            assert_eq!(i, 0);
        }
    }

    sys.run_now(&mut world.res);
    test_synced(world.entities(), world.read_storage::<Comp>(), &sys.store);
}
