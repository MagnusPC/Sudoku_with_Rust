// generate the sudokus
// aka main programme

use crate::{Sudoku, SudokuGrid};
use crate::constraint::Constraint;
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

    //line 105
}