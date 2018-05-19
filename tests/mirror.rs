extern crate specs_mirror;
extern crate specs;
extern crate shrev;

use specs::prelude::*;
use specs::world::Index;
use specs_mirror::{MirroredStorage, CloneData, Event};
use shrev::ReaderId;

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Copy, Clone)]
struct Comp;

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
        CompSystem {
            reader, store
        }
    }
}

impl<'a> System<'a> for CompSystem {
    type SystemData = ReadStorage<'a, Comp>;

    fn run(&mut self, comp: Self::SystemData) {
        for event in comp.unprotected_storage().chan().read(&mut self.reader) {
            match event {
                &Event::Inserted((id, data)) => assert!(self.store.insert(id, data).is_none()),
                &Event::Removed((id, _)) => assert!(self.store.remove(&id).is_some()),
            }
        }
    }
}

fn modify(comps: &mut WriteStorage<Comp>, ent: Entity, i: usize) {
    match i % 3 {
        0 => { comps.insert(ent, Comp).unwrap(); },
        1 => { comps.remove(ent); },
        _ => (),
    }
}

#[test]
fn test() {
    let mut world = World::new();
    world.register::<Comp>();

    let mut sys = CompSystem::new(world.write_storage::<Comp>());

    let entities: Vec<Entity> = world.create_iter().take(729).collect();

    sys.run_now(&mut world.res);

    {
        let mut comps = world.write_storage::<Comp>();
        for (mut i, &ent) in entities.iter().enumerate() {
            modify(&mut comps, ent, i);
            i /= 3;
            modify(&mut comps, ent, i);
            i /= 3;
            modify(&mut comps, ent, i);
            i /= 3;
            modify(&mut comps, ent, i);
        }
    }

    sys.run_now(&mut world.res);

    {
        let mut comps = world.write_storage::<Comp>();
        for (mut i, &ent) in entities.iter().rev().enumerate() {
            modify(&mut comps, ent, i);
            i /= 3;
            modify(&mut comps, ent, i);
            i /= 3;
            modify(&mut comps, ent, i);
            i /= 3;
            modify(&mut comps, ent, i);
        }
    }

    sys.run_now(&mut world.res);
}