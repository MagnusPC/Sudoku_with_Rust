pub mod constraint; //folder
pub mod error; //file
pub mod sudoku_generator; //file
pub mod solver; //folder
pub mod utilities; //file

// #[cfg(test)]
// mod fix_tests;

// #[cfg(test)]
// mod random_tests;

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
    } 
    
    pub fn to_parseable_string(&self) -> String {
        let mut s = format!("{}x{}", self.block_width, self.block_height);
        let cells = self.cells.iter()
            .map(to_string)
            .collect::<Vec<String>>()
            .join(",");
        s.push_str(cells.as_str());
        s
    }

    pub fn block_width(&self) -> usize {
        self.block_width
    }

    pub fn block_height(&self) -> usize {
        self.block_height
    }

    pub fn size(&self) -> usize {
        self.size
    }

    pub fn get_cell(&self, column: usize, row: usize) -> SudokuResult<Option<usize>>{
        let index = index(column, row, self.size())?;
        Ok(self.cells[index])
    }

    pub fn has_number(&self, column: usize, row: usize, number: usize) -> SudokuResult<bool> {
        if let Some(content) = self.get_cell(column, row)?{
            Ok(number == content)
        }
        else {
            Ok(false)
        }
    }

    pub fn set_cell(&mut self, column: usize, row: usize, number: usize) -> SudokuResult<()> {
        let size = self.size();
        let index = index(column, row, size)?;

        if number == 0 || number > size {
            return Err(SudokuError::InvalidNumber);
        }

        self.cells[index] = Some(number);
        Ok(())
    }

    pub fn clear_cell(&mut self, column: usize, row: usize) -> SudokuResult<()>{
        let index = index(column, row, self.size())?;
        self.cells[index] = None;
        Ok(())
    }

    fn verify_dimensions(&self, other: &SudokuGrid) -> SudokuResult<()>{
        if self.block_width != other.block_width || self.block_height != other.block_height {
            Err(SudokuError::InvalidDimensions)
        }
        else{
            Ok(())
        }
    }

    pub fn assign(&mut self, other: &SudokuGrid) -> SudokuResult<()>{
        self.verify_dimensions(other)?;
        self.cells.copy_from_slice(&other.cells);
        Ok(())
    }

    pub fn count_clues(&self) -> usize {
        let size = self.size();
        let mut clues = 0usize;

        for row in 0..size {
            for column in 0..size {
                if self.get_cell(column, row).unwrap().is_some() {
                    clues += 1;
                }
            }
        }

        clues
    }

    pub fn is_full(&self) -> bool {
        !self.cells.iter().any(|c| c == &None)
    }

    pub fn is_empty(&self) -> bool {
        self.cells.iter().all(|c| c == &None)
    }

    pub fn is_subset(&self, other: &SudokuGrid) -> SudokuResult<bool> {
        self.verify_dimensions(other)?;
        Ok(self.cells.iter()
            .zip(other.cells.iter())
            .all(|(self_cell, other_cell)| {
                match self_cell {
                    Some(self_number) => match other_cell {
                        Some(other_number) => self_number == other_number,
                        None => false
                    },
                    None => true
                }
            }))
    }

    pub fn is_superset(&self, other: &SudokuGrid) -> SudokuResult<bool>{
        other.is_subset(self)
    }

    pub fn cells(&self) -> &Vec<Option<usize>> {
        &self.cells
    }

    pub fn cells_mut(&mut self) -> &mut Vec<Option<usize>>{
        &mut self.cells
    }
}

#[derive(Clone, Deserialize, Serialize)]
pub struct Sudoku<C: Constraint + Clone> {
    grid: SudokuGrid,
    constraint: C
}

impl<C: Constraint + Clone> Sudoku<C> {
    pub fn new_empty(block_width: usize, block_height: usize, constraint: C) -> SudokuResult<Sudoku<C>>{
        Ok(Sudoku { grid: SudokuGrid::new(block_width, block_height)?, constraint})
    }

    pub fn new_with_grid(grid: SudokuGrid, constraint: C) -> Sudoku<C>{
        Sudoku {
            grid,
            constraint
        }
    }

    pub fn parse(code: &str, constraint: C) -> SudokuParseResult<Sudoku<C>> {
        Ok(Sudoku::new_with_grid(SudokuGrid::parse(code)?, constraint))
    }

    pub fn grid(&self) -> &SudokuGrid {
        &self.grid
    }

    pub fn grid_mut(&mut self) -> &mut SudokuGrid {
        &mut self.grid
    }

    pub fn constraint(&self) -> &C {
        &self.constraint
    }

    pub fn constraint_mut(&mut self) -> &mut C {
        &mut self.constraint
    }

    pub fn is_valid(&self) -> bool {
        self.constraint.check(&self.grid)
    }

    pub fn is_valid_cell(&self, column: usize, row: usize) -> SudokuResult<bool> {
        let size = self.grid.size();

        if column >= size || row >= size {
            Err(SudokuError::OutOfBounds)
        }
        else {
            Ok(self.constraint.check_cell(&self.grid, column, row))
        }
    }

    pub fn is_valid_number(&self,column: usize, row: usize, number: usize) -> SudokuResult<bool> {
        let size = self.grid.size();

        if column >= size || row >= size {
            Err(SudokuError::OutOfBounds)
        }
        else if number == 0 || number > size {
            Err(SudokuError::InvalidNumber)
        }
        else {
            Ok(self.constraint.check_number(&self.grid, column, row, number))
        }
    }

    pub fn is_valid_solution(&self, solution: &SudokuGrid) -> SudokuResult<bool>{
        Ok(self.grid.is_subset(solution)? && self.constraint.check(solution) && solution.is_full())
    }

    pub fn into_raw_parts(self) -> (SudokuGrid, C) {
        (self.grid, self.constraint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use crate::constraint::DefaultConstraint;

    #[test]
    fn parse_ok(){
        let grid_res = SudokuGrid::parse("2x2; 1,,,2, ,3,,4, ,2,,, 3,,,");

        if let Ok(grid) = grid_res {
            assert_eq!(2, grid.block_width());
            assert_eq!(2, grid.block_height());
            assert_eq!(Some(1), grid.get_cell(0,0).unwrap());
            assert_eq!(None, grid.get_cell(1, 0).unwrap());
            assert_eq!(None, grid.get_cell(2, 0).unwrap());
            assert_eq!(Some(2), grid.get_cell(3, 0).unwrap());
            assert_eq!(None, grid.get_cell(0, 1).unwrap());
            assert_eq!(Some(3), grid.get_cell(1, 1).unwrap());
            assert_eq!(None, grid.get_cell(2, 1).unwrap());
            assert_eq!(Some(4), grid.get_cell(3, 1).unwrap());
            assert_eq!(None, grid.get_cell(0, 2).unwrap());
            assert_eq!(Some(2), grid.get_cell(1, 2).unwrap());
            assert_eq!(None, grid.get_cell(2, 2).unwrap());
            assert_eq!(None, grid.get_cell(3, 2).unwrap());
            assert_eq!(Some(3), grid.get_cell(0, 3).unwrap());
            assert_eq!(None, grid.get_cell(1, 3).unwrap());
            assert_eq!(None, grid.get_cell(2, 3).unwrap());
            assert_eq!(None, grid.get_cell(3, 3).unwrap());
        }
        else {
            panic!("Parsing valid grid failed.");
        }
    }

    #[test]
    fn parse_malformed_dimensions(){
        assert_eq!(Err(SudokuParseError::MalformedDimensions), SudokuGrid::parse("2x2x2;,,,,,,,,,,,,,,,"));
    }

    #[test]
    fn parse_invalid_dimensions(){
        assert_eq!(Err(SudokuParseError::InvalidDimensions), SudokuGrid::parse("2x0;,"));
    }

    #[test]
    fn parse_wrong_number_of_parts() {
        assert_eq!(Err(SudokuParseError::WrongNumberOfParts),
            SudokuGrid::parse("2x2;,,,,,,,,,,,,,,,;whatever"));
    }

    #[test]
    fn parse_number_format_error() {
        assert_eq!(Err(SudokuParseError::NumberFormatError),
            SudokuGrid::parse("2x#;,"));
    }

    #[test]
    fn parse_invalid_number() {
        assert_eq!(Err(SudokuParseError::InvalidNumber),
            SudokuGrid::parse("2x2;,,,4,,,5,,,,,,,,,"));
    }

    #[test]
    fn parse_wrong_number_of_cells() {
        assert_eq!(Err(SudokuParseError::WrongNumberOfCells),
            SudokuGrid::parse("2x2;1,2,3,4,1,2,3,4,1,2,3,4,1,2,3"));
        assert_eq!(Err(SudokuParseError::WrongNumberOfCells),
            SudokuGrid::parse("2x2;1,2,3,4,1,2,3,4,1,2,3,4,1,2,3,4,1"));
    }

    #[test]
    fn to_parseable_string() {
        let mut grid = SudokuGrid::new(2, 2).unwrap();

        assert_eq!("2x2;,,,,,,,,,,,,,,,", grid.to_parseable_string().as_str());

        grid.set_cell(0, 0, 1).unwrap();
        grid.set_cell(1, 1, 2).unwrap();
        grid.set_cell(2, 2, 3).unwrap();
        grid.set_cell(3, 3, 4).unwrap();

        assert_eq!("2x2;1,,,,,2,,,,,3,,,,,4",
            grid.to_parseable_string().as_str());

        let grid = SudokuGrid::new(4, 1).unwrap();

        assert_eq!("4x1;,,,,,,,,,,,,,,,", grid.to_parseable_string().as_str());
    }

    #[test]
    fn size() {
        let grid1x1 = SudokuGrid::new(1, 1).unwrap();
        let grid3x2 = SudokuGrid::new(3, 2).unwrap();
        let grid3x4 = SudokuGrid::new(3, 4).unwrap();
        assert_eq!(1, grid1x1.size());
        assert_eq!(6, grid3x2.size());
        assert_eq!(12, grid3x4.size());
    }

    #[test]
    fn count_clues_and_empty_and_full() {
        let empty = SudokuGrid::parse("2x2;,,,,,,,,,,,,,,,").unwrap();
        let partial = SudokuGrid::parse("2x2;1,,3,2,4,,,,,,,,,,1,").unwrap();
        let full = SudokuGrid::parse("2x2;2,3,4,1,1,4,2,3,4,1,3,2,3,2,1,4")
            .unwrap();

        assert_eq!(0, empty.count_clues());
        assert_eq!(5, partial.count_clues());
        assert_eq!(16, full.count_clues());

        assert!(empty.is_empty());
        assert!(!partial.is_empty());
        assert!(!full.is_empty());

        assert!(!empty.is_full());
        assert!(!partial.is_full());
        assert!(full.is_full());
    }

    fn assert_subset_relation(a: &SudokuGrid, b: &SudokuGrid, a_subset_b: bool,
            b_subset_a: bool) {
        assert!(a.is_subset(b).unwrap() == a_subset_b);
        assert!(a.is_superset(b).unwrap() == b_subset_a);
        assert!(b.is_subset(a).unwrap() == b_subset_a);
        assert!(b.is_superset(a).unwrap() == a_subset_b);
    }

    fn assert_true_subset(a: &SudokuGrid, b: &SudokuGrid) {
        assert_subset_relation(a, b, true, false)
    }

    fn assert_equal_set(a: &SudokuGrid, b: &SudokuGrid) {
        assert_subset_relation(a, b, true, true)
    }

    fn assert_unrelated_set(a: &SudokuGrid, b: &SudokuGrid) {
        assert_subset_relation(a, b, false, false)
    }

    #[test]
    fn empty_is_subset() {
        let empty = SudokuGrid::new(2, 2).unwrap();
        let non_empty = SudokuGrid::parse("2x2;1,,,,,,,,,,,,,,,").unwrap();
        let full = SudokuGrid::parse("2x2;1,2,3,4,3,4,1,2,2,3,1,4,4,1,3,2")
            .unwrap();

        assert_equal_set(&empty, &empty);
        assert_true_subset(&empty, &non_empty);
        assert_true_subset(&empty, &full);
    }

    #[test]
    fn equal_grids_subsets() {
        let g = SudokuGrid::parse("2x2;1,,3,,2,,,,4,,4,3,,,,2").unwrap();
        assert_equal_set(&g, &g);
    }

    #[test]
    fn true_subset() {
        let g1 = SudokuGrid::parse("2x2;1,,3,,2,,,,4,,4,3,,,,2").unwrap();
        let g2 = SudokuGrid::parse("2x2;1,2,3,,2,,3,,4,,4,3,,,1,2").unwrap();
        assert_true_subset(&g1, &g2);
    }

    #[test]
    fn unrelated_grids_not_subsets() {
        // g1 and g2 differ in the third digit (3 in g1, 4 in g2)
        let g1 = SudokuGrid::parse("2x2;1,,3,,2,,,,4,,4,3,,,,2").unwrap();
        let g2 = SudokuGrid::parse("2x2;1,2,4,,2,,3,,4,,4,3,,,1,2").unwrap();
        assert_unrelated_set(&g1, &g2);
    }

    fn solution_example_sudoku() -> Sudoku<DefaultConstraint> {
        Sudoku::parse("2x2;\
            2, , , ,\
             , ,3, ,\
             , , ,4,\
             ,2, , ", DefaultConstraint).unwrap()
    }

    #[test]
    fn solution_not_full() {
        let sudoku = solution_example_sudoku();
        let solution = SudokuGrid::parse("2x2;\
            2,3,4,1,\
            1,4,3, ,\
            3,1,2,4,\
            4,2,1,3").unwrap();
        assert!(!sudoku.is_valid_solution(&solution).unwrap());
    }

    #[test]
    fn solution_not_superset() {
        let sudoku = solution_example_sudoku();
        let solution = SudokuGrid::parse("2x2;\
            2,3,4,1,\
            1,4,3,2,\
            3,2,1,4,\
            4,1,2,3").unwrap();
        assert!(!sudoku.is_valid_solution(&solution).unwrap());
    }

    #[test]
    fn solution_violates_constraint() {
        let sudoku = solution_example_sudoku();
        let solution = SudokuGrid::parse("2x2;\
            2,3,4,1,\
            1,3,3,2,\
            3,1,2,4,\
            4,2,1,3").unwrap();
        assert!(!sudoku.is_valid_solution(&solution).unwrap());
    }

    #[test]
    fn solution_correct() {
        let sudoku = solution_example_sudoku();
        let solution = SudokuGrid::parse("2x2;\
            2,3,4,1,\
            1,4,3,2,\
            3,1,2,4,\
            4,2,1,3").unwrap();
        assert!(sudoku.is_valid_solution(&solution).unwrap());
    }

    #[test]
    fn sudoku_grid_serde_consistent() {
        let grid = SudokuGrid::parse("3x2;\
            1, ,3, ,5, ,\
             ,2, ,4, ,6,\
            3, ,5, ,1, ,\
             ,4, ,6, ,2,\
            5, ,1, ,3, ,\
             ,6, ,2, ,4").unwrap();
        let json = serde_json::to_string(&grid).unwrap();
        let reconstructed_grid: SudokuGrid =
            serde_json::from_str(&json).unwrap();

        assert_eq!(grid, reconstructed_grid);
    }

}