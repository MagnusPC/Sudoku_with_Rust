use std::fmt::{self, Display, Formatter};
use std::num::ParseIntError;

#[derive(Debug, Eq, PartialEq)]
pub enum SudokuError{
    InvalidDimensions,
    InvalidNumber,
    OutOfBounds,
    UnsatisfiableConstraint
}

impl Display for SudokuError{
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            SudokuError::InvalidDimensions => write!(f, "invalid dimensions"),
            SudokuError::InvalidNumber => write!(f, "invalid number"),
            SudokuError::OutOfBounds => write!(f, "out of bounds"),
            SudokuError::UnsatisfiableConstraint => write!(f, "unsatisfiable constraint")
        }
    }
}

pub type SudokuResult<V> = Result<V, SudokuError>;

#[derive(Debug, Eq, PartialEq)]
pub enum SudokuParseError{
    WrongNumberOfParts,
    WrongNumberOfCells,
    MalformedDimensions,
    InvalidDimensions,
    NumberFormatError,
    InvalidNumber
}

impl From<ParseIntError> for SudokuParseError {
    fn from(_: ParseIntError) -> Self {
        SudokuParseError::NumberFormatError
    }
}

pub type SudokuParseResult<V> = Result<V, SudokuParseError>;