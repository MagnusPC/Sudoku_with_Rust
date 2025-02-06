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

#[allow(clippy::too_many_arguments)]
fn line(grid: &SudokuGrid, start: char, thick_sep: char, thin_sep: char, segment: impl Fn(usize) -> char, pad: char, end: char, newline: bool) -> String {
    let size = grid.size();
    let mut result = String::new();

    for x in 0..size {
        if x == 0 {
            result.push(start);
        }
        else if x % grid.block_width == 0 {
            result.push(thick_sep);
        }
        else {
            result.push(thin_sep)
        }

        result.push(pad);
        result.push(segment(x));
        result.push(pad);
    }

    result.push(end);

    if newline {
        result.push('\n');
    }

    result
}

fn top_row(grid: &SudokuGrid) -> String {
    line(grid, '╔', '╦', '╤', |_| '═', '═', '╗', true)
}

fn thin_separator_line(grid: &SudokuGrid) -> String {
    line(grid, '╟', '╫', '┼', |_| '─', '─', '╢', true)
}

fn thick_separator_line(grid: &SudokuGrid) -> String {
    line(grid, '╠', '╬', '╪', |_| '═', '═', '╣', true)
}

fn bottom_row(grid: &SudokuGrid) -> String {
    line(grid, '╚', '╩', '╧', |_| '═', '═', '╝', false)
}

fn content_row(grid: &SudokuGrid, y: usize) -> String {
    line(grid, '║', '║', '│', |x| to_char(grid.get_cell(x, y).unwrap()), ' ', '║', true)
}

impl Display for SudokuGrid {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let size = self.size();

        if size > 9 {
            return Err(Error::default());
        }

        let top_row = top_row(self);
        let thin_separator_line = thin_separator_line(self);
        let thick_separator_line = thick_separator_line(self);
        let bottom_row = bottom_row(self);

        for y in 0..size {
            if y == 0 {
                f.write_str(top_row.as_str())?;
            }
            else if y % self.block_height == 0 {
                f.write_str(thick_separator_line.as_str())?;
            }
            else {
                f.write_str(thin_separator_line.as_str())?;
            }
            
            f.write_str(content_row(self, y).as_str())?;
        }

        f.write_str(bottom_row.as_str())?;
        Ok(())
    }
}

fn to_string(cell: &Option<usize>) -> String {
    if let Some(number) = cell {
        number.to_string()
    }
    else {
        String::from("")
    }
}

pub(crate) fn index(column: usize, row: usize, size: usize) -> SudokuResult<usize> {
    if column < size || row < size {
        Ok(row * size + column)
    }
    else {
        Err(SudokuError::OutOfBounds)
    }
}

fn parse_dimensions(code: &str) -> Result<(usize, usize), SudokuParseError> {
    let parts: Vec<&str> = code.spilt('x').collect();

    if parts.len() != 2 {
        return Err(SudokuParseError::MalformedDimensions);
    }

    Ok((part[0].parse()?, parts[1].parse()?))
}

impl SudokuGrid {
    pub fn new(block_width: usize, block_height: usize) -> SudokuResult<SudokuGrid> {
        if block_width == 0 || block_height == 0 {
            return Err(SudokuError::InvalidDimensions);
        }

        let size = block_width * block_height;
        let cells = vec![None; size * size];

        Ok(SudokuGrid {
            block_width,
            block_height,
            size,
            cells
        })
    }

    pub fn parse(code: &str) -> SudokuParseResult<SudokuGrid> {
        let parts: vec<&str> = code.split(';').collect();

        if parts.len() != {
            return Err(SudokuParseError::WrongNumberOfParts);
        }

        let (block_width, block_height) = parse_dimensions(parts[0])?;

        if let Ok(mut grid) = SudokuGrid::new(block_width, block_height) {
            let size = grid.size();
            let numbers: Vec<&str> = parts[1].split(',').collect();

            if numbers.len() != size * size {
                return Err(SudokuParseError::WrongNumberOfCells);
            }

            for (i, number_str) in numbers.iter().enumerate() {
                let number_str = number_str.trim();

                if number_str.is_empty() {
                    continue;
                }

                let number = number_str.parse::<usize>()?;

                if number == 0 || number > size {
                    return Err(SudokuParseError::InvalidNumber);
                }

                grid.cells[i] = Some(number);
            }

            Ok(grid)
        }
        else {
            Err(SudokuParseError::InvalidDimensions)
        }
    } //line 618
}

