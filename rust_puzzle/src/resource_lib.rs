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
