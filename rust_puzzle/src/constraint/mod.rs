use std::any::Any;

use crate::SudokuGrid;

pub mod composite;
pub mod irreducible;
pub mod reducible;

pub use composite::*;
pub use irreducible::*;
pub use reducible::*;

pub type Group = Vec<(usize, usize)>;

#[inline]
pub(crate) fn default_check<C>(this: &C, grid: &SudokuGrid) -> bool 
where C: Constraint + ?Sized {
    let size = grid.size();

    for row in 0..size {
        for column in 0..size {
            if !this.check_cell(grid, column, row) {
                return false;
            }
        }
    }

    true
}

#[inline]
pub(crate) fn default_check_cell<C>(this: &C, grid: &SudokuGrid, column: usize, row: usize) -> bool
where C: Constraint + ?Sized {
    if let Some(number) = grid.get_cell(column, row).unwrap() {
        this.check_number(grid, column, row, number)
    }
    else {
        true
    }
}

#[derive(Debug)]
pub enum ReductionError {
    InvalidReduction
}

pub trait Constraint{
    type Reduction;
    type RevertInfo;
    
    fn check(&self, grid: &SudokuGrid) -> bool {
        default_check(self, grid)
    }

    fn check_cell(&self, grid: &SudokuGrid, column: usize, row: usize) -> bool {
        default_check_cell(self, grid, column, row)
    }

    fn check_number(&self, grid: &SudokuGrid, column: usize, row: usize, number: usize) -> bool;
    
    fn get_groups(&self, grid: &SudokuGrid) -> Vec<Group>;

    fn list_reductions(&self, solution: &SudokuGrid) -> Vec<Self::Reduction>;

    fn reduce(&mut self, solution: &SudokuGrid, reduction: &Self::Reduction) -> Result<Self::RevertInfo, ReductionError>;

    fn revert(&mut self, solution: &SudokuGrid, reduction: &Self::Reduction, revert_info: Self::RevertInfo);

    fn to_objects(&self) -> Vec<&dyn Any>
    where Self: Sized + 'static {
        vec![self]
    }
}

pub trait Subconstraint {
    fn get_subconstraint<S: Constraint + Sized + 'static>(&self) -> Option<&S>;

    fn has_subconstraints<S>(&self) -> bool
    where S: Constraint + Sized + 'static {
        self.get_subconstraint::<S>().is_some()
    }
}

impl<C: Constraint + Sized + 'static> Subconstraint for C {
    fn get_subconstraint<S>(&self) -> Option<&S>
    where S: Constraint + Sized + 'static {
        for object in self.to_objects() {
            let subconstraint = object.downcast_ref();

            if subconstraint.is_some() {
                return subconstraint;
            }
        }

        None
    }
}