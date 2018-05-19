//! This example demonstrates the use of `MirroredStorage` to keep the constraints in the `specs`
//! world synchronised with the constraints in a `cassowary` solver.

extern crate cassowary;
extern crate specs_mirror;
extern crate specs;
extern crate shrev;

use cassowary::{Constraint, Solver, Variable};
use cassowary::WeightedRelation::*;
use cassowary::strength::*;
use specs_mirror::{CloneData, Event,  MirroredStorage};
use specs::prelude::*;
use shrev::ReaderId;

#[derive(Clone, Debug)]
struct Constraints(Vec<Constraint>);

impl Component for Constraints {
    type Storage = MirroredStorage<Self, CloneData>;
}

struct LayoutSystem {
    solver: Solver,
    reader: ReaderId<Event<Constraints, CloneData>>,
}

impl LayoutSystem {
    fn new(mut cns: WriteStorage<Constraints>) -> Self {
        let solver = Solver::new();
        let reader = cns.unprotected_storage_mut().register_reader();
        LayoutSystem { solver, reader }
    }
}

impl<'a> System<'a> for LayoutSystem {
    type SystemData = ReadStorage<'a, Constraints>;

    fn run(&mut self, cns: Self::SystemData) {
        // Synchronise the changes to constraints in specs with the solver.
        for event in cns.unprotected_storage().read(&mut self.reader) {
            match event {
                Event::Inserted((_, data)) => {
                    self.solver.add_constraints(&data.0).ok();
                },
                Event::Removed((_, data)) => for cn in &data.0 {
                    self.solver.remove_constraint(cn).ok();
                },
            }
        }

        for &(var, val) in self.solver.fetch_changes() {
            println!("var: {:?}, val: {}", var, val);
        }
    }
}

fn main() {
    let mut world = World::new();
    world.register::<Constraints>();

    let mut sys = LayoutSystem::new(world.write_storage::<Constraints>());

    let var0 = Variable::new();
    let var1 = Variable::new();

    let e1 = world.create_entity()
        .with(Constraints(vec![var0 + 50.0 |EQ(REQUIRED)| var1]))
        .build();
    let e2 = world.create_entity()
        .with(Constraints(vec![var1 |EQ(REQUIRED)| 100.0]))
        .build();

    sys.run_now(&mut world.res);

    world.delete_entity(e1);
    let e3 = world.create_entity()
        .with(Constraints(vec![var1 * 2.0 |EQ(REQUIRED)| var0]))
        .build();

    sys.run_now(&mut world.res);
}