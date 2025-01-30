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