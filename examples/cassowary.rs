//! This example demonstrates the use of `MirroredStorage` to keep the constraints in the `specs`
//! world synchronized with the constraints in a `cassowary` solver.

extern crate cassowary;
extern crate shrev;
extern crate specs;
extern crate specs_mirror;

use cassowary::strength::*;
use cassowary::WeightedRelation::*;
use cassowary::{Constraint, Solver, Variable};
use shrev::ReaderId;
use specs::prelude::*;
use specs_mirror::*;

#[derive(Clone, Debug)]
struct Constraints(Vec<Constraint>);

impl Component for Constraints {
    type Storage = MirroredStorage<Self, CloneData>;
}

struct LayoutSystem {
    solver: Solver,
    reader: ReaderId<UpdateEvent<Constraints, CloneData>>,
}

impl LayoutSystem {
    fn new(mut cns: WriteStorage<Constraints>) -> Self {
        let solver = Solver::new();
        let reader = cns.register_reader();
        LayoutSystem { solver, reader }
    }
}

impl<'a> System<'a> for LayoutSystem {
    type SystemData = ReadStorage<'a, Constraints>;

    fn run(&mut self, cns: Self::SystemData) {
        // synchronize the changes to constraints in specs with the solver.
        for event in cns.read_events(&mut self.reader) {
            match event {
                UpdateEvent::Inserted(_, data) => {
                    self.solver.add_constraints(&data.0).ok();
                }
                UpdateEvent::Removed(_, data) => for cn in &data.0 {
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

    let e1 = world
        .create_entity()
        .with(Constraints(vec![var0 + 50.0 | EQ(REQUIRED) | var1]))
        .build();
    let _e2 = world
        .create_entity()
        .with(Constraints(vec![var1 | EQ(REQUIRED) | 100.0]))
        .build();

    sys.run_now(&mut world.res);

    world.delete_entity(e1).unwrap();
    let _e3 = world
        .create_entity()
        .with(Constraints(vec![var1 * 2.0 | EQ(REQUIRED) | var0]))
        .build();

    sys.run_now(&mut world.res);
}
