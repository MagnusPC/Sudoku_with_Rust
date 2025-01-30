pub mod constraint; //folder
pub mod error; //file
pub mod generator; //file
pub mod solver; //folder
pub mod util; //file

#[cfg(test)]
mod fix_tests;

#[cfg(test)]
mod random_tests;

use constraint::Constraint;
use error::{SudokuError, SudokuParseError, SudokuParseResult, SudokuResult};

use serde::{Deserialize, Serialize};
use serde::de::{self, Deserializer, MapAccess, SeqAccess, Visitor};

use std::fmt::{self, Display, Error, Formatter};

// main grid struct
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct SudokuGrid {
    block_width: usize,
    block_height: usize,
    #[serde(skip_serializing)]
    size: usize,
    cells: Vec<Option<usize>>,
}

// create grid or throw error
fn build_sudoku_grid<E: de::Error>(
    block_width: usize,
    block_height: usize,
    cells: Vec<Option<usize>>,
) -> Result<SudokuGrid, E> {
    let mut grid = match SudokuGrid::new(block_width, block_height) {
        Ok(grid) => grid,
        Err(e) => return Err(E::custom(e)),
    };
    let size = grid.size();

    if cells.len() != size * size {
        return Err(E::custom("invalid number of cells"));
    }

    grid.cells = cells;
    Ok(grid)
}

fn read_field<'de, M, V>(buffer: &mut Option<V>, map: &mut M, field_name: &'static str) -> Result<(), M::Error> where
    M: MapAccess<'de>,
    V: Deserialize<'de>{
            if buffer.is_some() {
                Err(<M::Error as de::Error>::duplicate_field(field_name))
            }
            else {
                *buffer = Some(map.next_value()?);
                Ok(())
            }
}