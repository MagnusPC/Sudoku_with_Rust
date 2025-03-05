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

//line 82
