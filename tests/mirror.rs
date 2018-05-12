extern crate specs_mirror;
extern crate specs;
extern crate shrev;

use specs::prelude::*;
use specs::world::Index;
use specs_mirror::{MirroredStorage, CloneData, Event};
use shrev::ReaderId;

use std::collections::HashMap;
use std::sync::Arc;

#[derive(Clone)]
struct Comp;

impl Component for Comp {
    type Storage = MirroredStorage<Self, CloneData>;
}

pub struct CompSystem {
    reader: ReaderId<Event<Comp, CloneData>>,
    store: HashMap<Index, Comp>,
}

impl CompSystem {
    fn new(store: WriteStorage<Comp>) -> Self {
        let reader = store.chan_mut().register_reader();
        let store = HashMap::new();
        CompSystem {
            reader, store
        }
    }
}

impl<'a> System<'a> for CompSystem {
    type SystemData = ReadStorage<'a, Comp>;

    fn run(&mut self, comp: Self::SystemData) {
        for event in comp.chan().read(&mut self.reader) {
            match event {
                Event::Inserted((id, data)) => assert!(self.store.insert(id, data).is_none()),
                Event::Removed((id, data)) => assert!(self.store.remove(id, data).is_none()),
            }
        }
    }
}

#[test]
fn test() {
    /*
    let mut world = World::new();
    world.register::<Comp>();

    let mut sys = CompSystem::new(world.write_storage::<Comp>());

    let entities: Vec<Entity> = world.create_iter().take(1000).collect();
    */
}