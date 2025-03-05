// generate the sudokus
// aka main programme

use crate::constraint::{reducible, Constraint};
use crate::error::{SudokuError, SudokuResult};
use crate::solver::{BacktrackingSolver, Solution, Solver};
use crate::{Sudoku, SudokuGrid};

use rand::rngs::ThreadRng;
use rand::Rng;

use rand_distr::Normal;

use std::f64::consts::{self, FRAC_1_SQRT_2};

pub struct Generator<R: Rng> {
    rng: R,
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
        Generator { rng }
    }

    fn fill_rec<C: Constraint + Clone>(
        &mut self,
        sudoku: &mut Sudoku<C>,
        column: usize,
        row: usize,
    ) -> bool {
        let size = sudoku.grid().size();

        if row == size {
            return true;
        }

        let next_column = (column + 1) % size;
        let next_row = if next_column == 0 { row + 1 } else { row };

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

    pub fn fill<C>(&mut self, sudoku: &mut Sudoku<C>) -> SudokuResult<()>
    where
        C: Constraint + Clone,
    {
        if self.fill_rec(sudoku, 0, 0) {
            Ok(())
        } else {
            Err(SudokuError::UnsatisfiableConstraint)
        }
    }

    pub fn generate<C>(
        &mut self,
        block_width: usize,
        block_height: usize,
        constraint: C,
    ) -> SudokuResult<Sudoku<C>>
    where
        C: Constraint + Clone,
    {
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
    rng: R,
}

impl Reducer<BacktrackingSolver, ThreadRng> {
    pub fn new_default() -> Reducer<BacktrackingSolver, ThreadRng> {
        Reducer::new(BacktrackingSolver, rand::thread_rng())
    }
}

pub enum Reduction<R> {
    RemoveDigit { column: usize, row: usize },

    ReduceConstraint { reduction: R },
}

impl<R> Reduction<R> {
    fn apply<S, C>(&self, sudoku: &mut Sudoku<C>, solution: &SudokuGrid, solver: &S)
    where
        S: Solver,
        C: Constraint<Reduction = R> + Clone + 'static,
    {
        match self {
            Reduction::RemoveDigit { column, row } => {
                let number = sudoku.grid().get_cell(*column, *row).unwrap().unwrap();
                sudoku.grid_mut().clear_cell(*column, *row).unwrap();

                if let Solution::Unique(_) = solver.solve(sudoku) {
                } else {
                    sudoku.grid_mut().set_cell(*column, *row, number).unwrap();
                }
            }
            Reduction::ReduceConstraint { reduction: r } => {
                let constraint = sudoku.constraint_mut();
                let reduce_res = constraint.reduce(solution, r);

                if let Ok(revert_info) = reduce_res {
                    if let Solution::Unique(_) = solver.solve(sudoku) {
                    } else {
                        let constraint = sudoku.constraint_mut();
                        constraint.revert(solution, r, revert_info);
                    }
                }
            }
        }
    }
}

fn reduction<R, C>(sudoku: &Sudoku<C>) -> impl Iterator<Item = Reduction<R>>
where
    C: Constraint<Reduction = R> + Clone,
{
    let size = sudoku.grid().size();
    let digit_reductions = (0..size)
        .flat_map(move |column| (0..size).map(move |row| Reduction::RemoveDigit { column, row }));
    let constraint_reductions = sudoku
        .constraint()
        .list_reductions(sudoku.grid())
        .into_iter()
        .map(|r| Reduction::ReduceConstraint { reduction: r });
    digit_reductions.chain(constraint_reductions)
}

fn prioritize<RED, P, RNG>(reduction: &RED, prioritizer: &mut P, rng: &mut RNG) -> f64
where
    P: ReductionPrioritizer<RED>,
    RNG: Rng,
{
    let distr = Normal::new(0.0, consts::FRAC_1_SQRT_2).unwrap();
    prioritizer.rough_priority(reduction) + rng.sample(distr)
}

impl<S: Solver, R: Rng> Reducer<S, R> {
    pub fn new(solver: S, rng: R) -> Reducer<S, R> {
        Reducer { solver, rng }
    }

    pub fn reduce<C>(&mut self, sudoku: &mut Sudoku<C>)
    where
        C: Constraint + Clone + 'static,
    {
        self.reduce_with_priority(sudoku, EqualPrioritizer)
    }

    pub fn reduce_with_priority<C, P>(&mut self, sudoku: &mut Sudoku<C>, mut prioritizer: P)
    where
        C: Constraint + Clone + 'static,
        P: ReductionPrioritizer<Reduction<C::Reduction>>,
    {
        let mut reductions = reductions(sudoku)
            .map(|r| (prioritize(&r, &mut prioritizer, &mut self.rng), r))
            .collect::<Vec<_>>();
        reductions.sort_by(|(p1, _), (p2, _)| p1.partial_cmp(p2).unwrap());
        let solution = sudoku.grid().clone();
        for (_, reduction) in reductions {
            reduction.apply(sudoku, &solution, &self.solver);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::{
        CompositeConstraint, DefaultConstraint, Group, KillerConstraint, ReductionError,
    };
    use crate::solver::strategy::solvers::StrategicBacktrackingSolver;
    use crate::solver::strategy::{CompositeStrategy, NakedSingleStrategy, OnlyCellStrategy};

    //LINE 389
}
