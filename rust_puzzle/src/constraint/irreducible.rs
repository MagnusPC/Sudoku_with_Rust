use crate::constraint::{self, Constraint, Group, ReductionError};
use crate::utilities::USizeSet;
use crate::SudokuGrid;

use serde::{Deserialize, Serialize};

use std::any::Any;
use std::iter::Cloned;
use std::slice::Iter;

pub trait IrreducibleConstraint {
    #[inline]
    fn check(&self, grid: &SudokuGrid) -> bool {
        constraint::default_check(self, grid)
    }

    #[inline]
    fn check_cell(&self, grid: &SudokuGrid, column: usize, row: usize) -> bool {
        constraint::default_check_cell(self, grid, column, row)
    }

    fn check_number(&self, grid: &SudokuGrid, column: usize, row: usize, number: usize) -> bool;

    fn get_groups(&self, grid: &SudokuGrid) -> Vec<Group>;

    fn to_objects(&self) -> Vec<&dyn Any>
    where
        Self: Sized + 'static,
    {
        vec![self]
    }
}

//line 56
