use crate::constraint::{Constraint, Group, ReductionError};
use crate::SudokuGrid;
use serde::{Deserialize, Serialize};
use std::any::Any;

#[derive(Clone, Deserialize, Serialize)]
pub struct CompositeConstraint<C1, C2>
where
    C1: Constraint + Clone + 'static,
    C2: Constraint + Clone + 'static,
{
    c1: C1,
    c2: C2,
}

impl<C1, C2> CompositeConstraint<C1, C2>
where
    C1: Constraint + Clone + 'static,
    C2: Constraint + Clone + 'static,
{
    pub fn new(c1: C1, c2: C2) -> CompositeConstraint<C1, C2> {
        CompositeConstraint { c1, c2 }
    }

    pub fn first(&self) -> &C1 {
        &self.c1
    }

    pub fn first_mut(&mut self) -> &mut C1 {
        &mut self.c1
    }

    pub fn second(&self) -> &C2 {
        &self.c2
    }

    pub fn second_mut(&mut self) -> &mut C2 {
        &mut self.c2
    }

    pub fn into_components(self) -> (C1, C2) {
        (self.c1, self.c2)
    }
}

pub enum CompositeData<D1, D2> {
    First(D1),
    Second(D2),
}

//line 108
