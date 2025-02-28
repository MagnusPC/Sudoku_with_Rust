// generate the sudokus
// aka main programme

use crate::{Sudoku, SudokuGrid};
use crate::constraint::{reducible, Constraint};
use crate::error::{SudokuError, SudokuResult};
use crate::solver::{BacktrackingSolver, Solution, Solver};

use rand::Rng;
use rand::rngs::ThreadRng;

use rand_distr::Normal;

use std::f64::consts;

pub struct Generator<R: Rng>{
    rng: R
}

impl Generator<ThreadRng> {
    pub fn new_defaults() -> Generator<ThreadRng> {
        Generator::new(rand::thread_rng())
    }
}

pub(crate) fn shuffle<T>(rng: &mut impl Rng, values: impl Iterator<Item = T>) -> Vec<T> {
    let mut vec: Vec<T> = values.collect();
    let len = vec.len();

    for i in 0..(len - 1) {
        let j = rng.gen_range(i..len);
        vec.swap(i, j);
    }

    vec
}

impl<R: Rng> Generator<R> {
    pub fn new(rng: R) -> Generator<R> {
        Generator {
            rng
        }
    }

    fn fill_rec<C: Constraint + Clone>(&mut self, sudoku: &mut Sudoku<C>, column: usize, row: usize) -> {
        let size = sudoku.grid().size();

        if row == size {
            return true;
        }

        let next_column = (column + 1) % size;
        let next_row = if next_column == 0 { row + 1} else {row};

        if sudoku.grid().get_cell(column, row).unwrap().is_some() {
            return self.fill_rec(sudoku, next_column, next_row);
        }

        for number in shuffle(&mut self.rng, 1..=size) {
            if sudoku.is_valid_number(column, row, number).unwrap() {
                sudoku.grid_mut().set_cell(column, row, number).unwrap();
                if self.fill_rec(sudoku, next_column, next_row) {
                    return true;
                }

                sudoku.grid_mut().clear_cell(column, row).unwrap();
            }
        }

        false
    }

    pub fn fill<C>(&mut self, sudoku: &mut Sudoku<C>) -> SudokuResult<()> where C: Constraint + Clone {
        if self.fill_rec(sudoku, 0, 0) {
            Ok(())
        }
        else {
            Err(SudokuError::UnsatisfiableConstraint)
        }
    }

    pub fn generate<C>(&mut self, block_width: usize, block_height: usize, constraint: C) -> SudokuResult<Sudoku<C>> where C: Constraint + Clone {
        let mut sudoku = Sudoku::new_empty(block_width, block_height, constraint)?;
        self.fill(&mut sudoku)?;
        Ok(sudoku)
    }
}

pub trait ReductionPrioritizer<R> {
    fn rough_priority(&mut self, reduction: &R) -> f64;
}

struct EqualPrioritizer;

impl<R> ReductionPrioritizer<R> for EqualPrioritizer {
    fn rough_priority(&mut self, _: &R) -> f64 {
        0.0
    }
}

impl<R, F: Fn(&R) -> f64> ReductionPrioritizer<R> for F {
    fn rough_priority(&mut self, reduction: &R) -> f64 {
        self(reduction)
    }
}

pub struct Reducer<S: Solver, R: Rng> {
    solver: S,
    rng: R
}

impl Reducer<BacktrackingSolver, ThreadRng> {
    pub fn new_default() -> Reducer<BacktrackingSolver, ThreadRng> {
        Reducer::new(BacktrackingSolver, rand::thread_rng())
    }
}

pub enum Reduction<R> {
    RemoveDigit {
        column: usize,
        row: usize
    },

    ReduceConstraint {
        reduction: R
    }
}

impl<R> Reduction<R> {
    fn apply<S, C>(&self, sudoku: &mut Sudoku<C>, solution: &SudokuGrid, solver: &S) where S: Solver, C: Constraint<Reduction = R> + Clone + 'static {
        match self {
            Reduction::RemoveDigit { column, row } => {
                let number = sudoku.grid().get_cell(*column, *row).unwrap().unwrap();
                sudoku.grid_mut().clear_cell(*column, *row).unwrap();

                if let Solution::Unique(_) = solver.solve(sudoku) { }
                else {
                    sudoku.grid_mut().set_cell(*column, *row, number).unwrap();
                }
            },
            Reduction::ReduceConstraint { reduction: r} => {
                let constraint = sudoku.constraint_mut();
                let reduce_res = constraint.reduce(solution, r);

                if let Ok(revert_info) = reduce_res {
                    if let Solution::Unique(_) = solver.solve(sudoku) { }
                    else {
                        let constraint = sudoku.constraint_mut();
                        constraint.revert(solution, r, revert_info);
                    }
                }
            }
        }
    }
}

fn reduction<R, C>(sudoku: &Sudoku<C>) -> impl Iterator<Item = Reduction<R>> where C: Constraint<Reduction = R> + Clone {
    let size = sudoku.grid().size();
    //line 281
}