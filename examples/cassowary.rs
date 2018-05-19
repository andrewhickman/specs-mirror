//! This example demonstrates the use of `MirroredStorage` to keep the constraints in the `specs`
//! world synchronised with the constraints in a `cassowary` solver.

extern crate cassowary;
extern crate specs_mirror;
extern crate specs;
extern crate shrev;

use cassowary::{Constraint, Solver};
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
        for event in cns.unprotected_storage().read(&mut self.reader) {
            match event {
                Event::Inserted((_, data)) => self.solver.add_constraints(&data.0).unwrap(),
                Event::Removed((_, data)) => for cn in &data.0 {
                    self.solver.remove_constraint(cn).unwrap();
                },
            }
        }

        // Fetch and apply changes here ...
    }
}

fn main() {}