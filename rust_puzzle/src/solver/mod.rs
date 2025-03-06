use std::backtrace;

use crate::constraint::Constraint;
use crate::{Sudoku, SudokuGrid};

pub mod strategy;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Solution {
    Impossible,
    Unique(SudokuGrid),
    Ambiguous,
}

impl Solution {
    pub fn union(self, other: Solution) -> Solution {
        match self {
            Solution::Impossible => other,
            Solution::Unique(g) => match other {
                Solution::Impossible => Solution::Unique(g),
                Solution::Unique(other_g) => {
                    if g == other_g {
                        Solution::Unique(g)
                    } else {
                        Solution::Ambiguous
                    }
                }
                Solution::Ambiguous => Solution::Ambiguous,
            },
            Solution::Ambiguous => Solution::Ambiguous,
        }
    }
}

pub trait Solver {
    fn solve<C>(&self, sudoku: &Sudoku<C>) -> Solution
    where
        C: Constraint + Clone + 'static;
}

#[derive(Clone)]
pub struct BacktrackingSolver;

impl BacktrackingSolver {
    fn solve_rec<C>(sudoku: &mut Sudoku<C>, column: usize, row: usize) -> Solution
    where
        C: Constraint + Clone + 'static,
    {
        let size = sudoku.grid().size();
        let last_cell = row == size;

        if last_cell {
            return Solution::Unique(sudoku.grid().clone());
        }

        let next_column = (column + 1) % size;
        let next_row = if next_column == 0 { row + 1 } else { row };

        if sudoku.grid().get_cell(column, row).unwrap().is_some() {
            BacktrackingSolver::solve_rec(sudoku, next_column, next_row)
        } else {
            let mut solution = Solution::Impossible;
            for number in 1..=size {
                if sudoku.is_valid_number(column, row, number).unwrap() {
                    sudoku.grid_mut().set_cell(column, row, number).unwrap();
                    let next_solution =
                        BacktrackingSolver::solve_rec(sudoku, next_column, next_row);
                    sudoku.grid_mut().clear_cell(column, row).unwrap();
                    solution = solution.union(next_solution);

                    if solution == Solution::Ambiguous {
                        break;
                    }
                }
            }

            solution
        }
    }

    fn solve<C>(sudoku: &mut Sudoku<C>) -> Solution
    where
        C: Constraint + Clone + 'static,
    {
        BacktrackingSolver::solve_rec(sudoku, 0, 0)
    }
}

impl Solver for BacktrackingSolver {
    fn solve<C>(&self, sudoku: &Sudoku<C>) -> Solution
    where
        C: Constraint + Clone + 'static,
    {
        let mut clone = sudoku.clone();
        BacktrackingSolver::solve(&mut clone)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constraint::{
        AdjacentConsecutiveConstraint, CompositeConstraint, DefaultConstraint,
        DiagonallyAdjacentConstraint, DiagonalsConstraint, KillerCage, KillerConstraint,
        KingsMoveConstraint, KnightsMoveConstraint,
    };

    fn test_solves_correctly<C>(puzzle: &str, solution: &str, constraint: C)
    where
        C: Constraint + Clone + 'static,
    {
        let sudoku = Sudoku::parse(puzzle, constraint).unwrap();
        let solver = BacktrackingSolver;
        let found_solution = solver.solve(&sudoku);

        if let Solution::Unique(grid) = found_solution {
            let expected_grid = SudokuGrid::parse(solution).unwrap();
            assert_eq!(expected_grid, grid, "Solver gave wrong grid.");
        } else {
            panic!("Solvable sudoku marked as impossible or ambiguous.")
        }
    }

    #[test]
    fn backtracking_solves_classic_sudoku() {
        let puzzle = "3x3;\
             , , , ,8,1, , , ,\
             , ,2, , ,7,8, , ,\
             ,5,3, , , ,1,7, ,\
            3,7, , , , , , , ,\
            6, , , , , , , ,3,\
             , , , , , , ,2,4,\
             ,6,9, , , ,2,3, ,\
             , ,5,9, , ,4, , ,\
             , , ,6,5, , , , ";
        let solution = "3x3;\
            7,4,6,2,8,1,3,5,9,\
            9,1,2,5,3,7,8,4,6,\
            8,5,3,4,9,6,1,7,2,\
            3,7,4,1,2,5,6,9,8,\
            6,2,8,7,4,9,5,1,3,\
            5,9,1,3,6,8,7,2,4,\
            1,6,9,8,7,4,2,3,5,\
            2,8,5,9,1,3,4,6,7,\
            4,3,7,6,5,2,9,8,1";
        test_solves_correctly(puzzle, solution, DefaultConstraint);
    }

    #[test]
    fn backtracking_solves_diagonals_sudoku() {
        let puzzle = "3x3;\
             ,1,2,3,4,5,6,7, ,\
             , , , , , , , , ,\
             , , , , , , , , ,\
            7, , , , , , , ,5,\
            2, , , , , , , ,1,\
            9, , , , , , , ,3,\
             , , , , , , , , ,\
             , , , , , , , , ,\
             ,3,4,5,6,7,8,9, ";
        let solution = "3x3;\
            8,1,2,3,4,5,6,7,9,\
            3,7,5,6,8,9,1,2,4,\
            4,9,6,1,7,2,3,5,8,\
            7,4,1,9,3,6,2,8,5,\
            2,6,3,7,5,8,9,4,1,\
            9,5,8,4,2,1,7,6,3,\
            5,2,7,8,9,3,4,1,6,\
            6,8,9,2,1,4,5,3,7,\
            1,3,4,5,6,7,8,9,2";
        test_solves_correctly(
            puzzle,
            solution,
            CompositeConstraint::new(DefaultConstraint, DiagonalsConstraint),
        );
    }

    #[test]
    fn backtracking_solves_knights_move_sudoku() {
        let puzzle = "3x3;\
             ,8, ,1, ,5, , , ,\
            4, ,7, ,9, , , , ,\
             ,1, ,8, , , , , ,\
            1, ,8, , , , , ,5,\
             ,7, , , , , ,8, ,\
            5, , , , , ,3, ,4,\
             , , , , ,8, ,4, ,\
             , , , ,3, ,8, ,6,\
             , , ,5, ,4, ,3, ";
        let solution = "3x3;\
            2,8,3,1,4,5,6,9,7,\
            4,5,7,3,9,6,1,2,8,\
            9,1,6,8,2,7,4,5,3,\
            1,3,8,4,7,2,9,6,5,\
            6,7,4,9,5,3,2,8,1,\
            5,2,9,6,8,1,3,7,4,\
            3,9,1,7,6,8,5,4,2,\
            7,4,5,2,3,9,8,1,6,\
            8,6,2,5,1,4,7,3,9";
        test_solves_correctly(
            puzzle,
            solution,
            CompositeConstraint::new(DefaultConstraint, KnightsMoveConstraint),
        );
    }

    #[test]
    fn backtracking_solves_kings_move_sudoku() {
        let puzzle = "3x3;\
             , , , ,2,1, , , ,\
             ,6,1, , , , ,3, ,\
             , , , , ,4, ,7, ,\
            3, ,7, , , , , , ,\
            2, , , ,5, , , ,7,\
             , , , , , ,5, ,8,\
             ,8, ,1, , , , , ,\
             ,3, , , , ,6,4, ,\
             , , ,7,6, , , , ";
        let solution = "3x3;\
            5,7,3,9,2,1,4,8,6,\
            4,6,1,5,8,7,2,3,9,\
            8,2,9,6,3,4,1,7,5,\
            3,5,7,2,1,8,9,6,4,\
            2,9,8,4,5,6,3,1,7,\
            1,4,6,3,7,9,5,2,8,\
            6,8,5,1,4,2,7,9,3,\
            7,3,2,8,9,5,6,4,1,\
            9,1,4,7,6,3,8,5,2";
        test_solves_correctly(
            puzzle,
            solution,
            CompositeConstraint::new(DefaultConstraint, KingsMoveConstraint),
        );
        test_solves_correctly(
            puzzle,
            solution,
            CompositeConstraint::new(DefaultConstraint, DiagonallyAdjacentConstraint),
        );
    }

    #[test]
    fn backtracking_solves_adjacent_consecutive_sudoku() {
        let puzzle = "3x3;\
             , , , , , , , ,7,\
             , ,3,8, , , , , ,\
             ,4,6, , , , , , ,\
             ,7, , ,2, , , , ,\
             , , ,9,4,7, , , ,\
             , , , ,8, , ,5, ,\
             , , , , , , ,9, ,\
             , , , , ,4,6,2, ,\
            5, , , , , , , , ";
        let solution = "3x3;\
            2,5,8,4,1,6,9,3,7,\
            7,1,3,8,5,9,2,6,4,\
            9,4,6,3,7,2,5,8,1,\
            3,7,1,6,2,5,8,4,9,\
            8,2,5,9,4,7,3,1,6,\
            4,6,9,1,8,3,7,5,2,\
            6,8,2,7,3,1,4,9,5,\
            1,3,7,5,9,4,6,2,8,\
            5,9,4,2,6,8,1,7,3";
        test_solves_correctly(
            puzzle,
            solution,
            CompositeConstraint::new(DefaultConstraint, AdjacentConsecutiveConstraint),
        );
    }

    #[test]
    fn backtracking_solves_killer_sudoku() {
        let puzzle = "3x3;\
             ,9, , , , , , , ,\
             , , , , , , , ,6,\
             , , , , , , , , ,\
             , , , ,7, , , , ,\
             , , ,3, ,4, , , ,\
             , , , ,9, , , , ,\
             , , , , , , , , ,\
            2, , , , , , , , ,\
             , , , , , , ,9, ";
        let solution = "3x3;\
            8,9,7,6,4,1,5,2,3,\
            5,2,1,9,8,3,4,7,6,\
            6,4,3,7,5,2,1,8,9,\
            1,3,2,8,7,6,9,5,4,\
            9,8,5,3,2,4,7,6,1,\
            7,6,4,1,9,5,2,3,8,\
            4,7,9,2,6,8,3,1,5,\
            2,1,6,5,3,9,8,4,7,\
            3,5,8,4,1,7,6,9,2";
        let mut constraint = KillerConstraint::new();
        let cages = vec![
            KillerCage::new(vec![(2, 0), (2, 1), (1, 1)], 10).unwrap(),
            KillerCage::new(vec![(3, 0), (3, 1), (4, 1)], 23).unwrap(),
            KillerCage::new(vec![(6, 1), (6, 2), (5, 2)], 7).unwrap(),
            KillerCage::new(vec![(7, 1), (7, 2), (8, 2)], 24).unwrap(),
            KillerCage::new(vec![(1, 2), (2, 2), (2, 3)], 9).unwrap(),
            KillerCage::new(vec![(3, 2), (4, 2), (3, 3)], 20).unwrap(),
            KillerCage::new(vec![(5, 3), (6, 3), (6, 4)], 22).unwrap(),
            KillerCage::new(vec![(7, 3), (8, 3), (7, 4)], 15).unwrap(),
            KillerCage::new(vec![(1, 4), (1, 5), (0, 5)], 21).unwrap(),
            KillerCage::new(vec![(2, 4), (2, 5), (3, 5)], 10).unwrap(),
            KillerCage::new(vec![(5, 5), (5, 6), (4, 6)], 19).unwrap(),
            KillerCage::new(vec![(6, 5), (6, 6), (7, 6)], 6).unwrap(),
            KillerCage::new(vec![(0, 6), (1, 6), (1, 7)], 12).unwrap(),
            KillerCage::new(vec![(2, 6), (3, 6), (2, 7)], 17).unwrap(),
            KillerCage::new(vec![(4, 7), (5, 7), (5, 8)], 19).unwrap(),
            KillerCage::new(vec![(6, 7), (7, 7), (6, 8)], 18).unwrap(),
        ];

        for cage in cages.into_iter() {
            constraint.add_cage(cage).unwrap();
        }

        test_solves_correctly(
            puzzle,
            solution,
            CompositeConstraint::new(DefaultConstraint, constraint),
        );
    }
}
