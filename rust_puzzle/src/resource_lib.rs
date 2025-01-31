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

impl<'de> Deserialize<'de> for SudokuGrid {
    fn deserialize<D>(deserializer: D) -> Result<SudokuGrid, D::Error>
    where
        D: Deserializer<'de>{
            #[derive(Deserialize)]
            #[serde(field_identifier, rename_all = "snake_case")]
            enum Field {
                BlockWidth,
                BlockHeight,
                Cells
            }

            const BLOCK_WIDTH_NAME: &str = "block_width";
            const BLOCK_HEIGHT_NAME: &str = "block_height";
            const CELLS_NAME: &str = "cells";

            struct SudokuGridVisitor;

            impl<'de> Visitor<'de> for SudokuGridVisitor{
                type Value = SudokuGrid;

                fn expecting(&self, f: &mut formatter) -> fmt::Result{
                    write!(f, "struct SudokuGrid")
                }

                fn visit_seq<V>(self, mut seq: V) -> Result<SudokuGrid, V::Error> where V: SeqAccess<'de>
                {
                    let block_width = seq.next_element()?
                        .ok_or_else(|| de:Error::invalid_length(0, &self))?;
                    let block_height = seq.next_element()?
                        .ok_or_else(|| de:Error::invalid_length(1, &self))?;
                    let cells = seq.next_element()?
                        .ok_or_else(|| de::Error:invalid_length(2, &self))?;
                    build_sudoku_grid(block_width, block_height, cells)
                }

                fn visit_map<V>(self, mut map: V) -> Result<SudokuGrid, V::Error> where V: MapAccess<'de>
                {
                    let mut block_width = None;
                    let mut block_height = None;
                    let mut cells = None;

                    while let Some(key) = map.next_key()?{
                        match key {
                            Field::BlockWidth =>
                                read_field(&mut block_width, &mut map, BLOCK_WIDTH_NAME)?,
                            Field::BlockHeight =>
                                read_field(&mut block_height, &mut map, BLOCK_HEIGHT_NAME)?,
                            Field::Cells =>
                                read_field(&mut cells, &mut map, CELLS_NAME)?;
                        }
                    }

                    let block_width = block_width.ok_or_else(|| de::Error::missing_field(BLOCK_WIDTH_NAME))?;
                    let block_height = block_height.ok_or_else(|| de::Error::missing_field(BLOCK_HEIGHT_NAME))?;
                    let cells = cells.ok_or_else(|| de::Error::missing_field(CELLS_NAME))?;
                    build_sudoku_grid(block_width, block_height, cells)
                }
            }

            const FIELDS: &[&str] = &[
                BLOCK_WIDTH_NAME,
                BLOCK_HEIGHT_NAME,
                CELLS_NAME
            ];
            deserializer.deserialize_struct("SudokuGrid", FIELDS, SudokuGridVisitor)
        }
}

fn to_char(cell: Option<usize>) -> char {
    if let Some(n) = cell {
        (b'0' + n as u8) as char
    }
    else {
        ' '
    }
}

