//! Port of `ru.icc.regtab.itm.syntax`: the syntactic layer of an ITM instance.
//!
//! The Java object graph (Cell -> Row/Subrow/Subtable back-references) is
//! replaced by an arena: `SyntaxCore` owns flat vectors, links are indices.
//! Identity comparisons (`c.subrow() == a.subrow()`) become index equality.

use crate::util::{java_is_blank, CoreResult};

#[pyo3::pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FontFamily {
    #[pyo3(name = "SERIF")]
    Serif,
    #[pyo3(name = "SANS_SERIF")]
    SansSerif,
    #[pyo3(name = "MONOSPACED")]
    Monospaced,
}

#[pyo3::pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HorizontalAlignment {
    #[pyo3(name = "LEFT")]
    Left,
    #[pyo3(name = "CENTER")]
    Center,
    #[pyo3(name = "RIGHT")]
    Right,
    #[pyo3(name = "JUSTIFY")]
    Justify,
}

#[pyo3::pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum VerticalAlignment {
    #[pyo3(name = "TOP")]
    Top,
    #[pyo3(name = "CENTER")]
    Center,
    #[pyo3(name = "BOTTOM")]
    Bottom,
    #[pyo3(name = "JUSTIFY")]
    Justify,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CellColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl CellColor {
    pub const BLACK: CellColor = CellColor { r: 0, g: 0, b: 0 };
    pub const WHITE: CellColor = CellColor { r: 255, g: 255, b: 255 };
}

/// Merged-cell bounding box: (top_row, left_col, bottom_row, right_col).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct BoundingBox {
    pub top_row: usize,
    pub left_col: usize,
    pub bottom_row: usize,
    pub right_col: usize,
}

#[derive(Clone, Debug)]
pub struct CellData {
    pub row: usize,
    pub col: usize,
    pub bbox: BoundingBox,
    pub merged: bool,
    // formatting
    pub font_family: FontFamily,
    pub font_bold: bool,
    pub font_italic: bool,
    pub font_strikeout: bool,
    pub font_underline: bool,
    pub horz_align: HorizontalAlignment,
    pub vert_align: VerticalAlignment,
    pub left_border: bool,
    pub top_border: bool,
    pub right_border: bool,
    pub bottom_border: bool,
    pub bg_color: CellColor,
    pub fg_color: CellColor,
    pub rotation: f64,
    // content
    pub text: String,
    pub text_blank: bool,
    pub text_multiline: bool,
    pub text_indent: usize,
}

impl CellData {
    fn new(row: usize, col: usize) -> Self {
        CellData {
            row,
            col,
            bbox: BoundingBox {
                top_row: row,
                left_col: col,
                bottom_row: row,
                right_col: col,
            },
            merged: false,
            font_family: FontFamily::Serif,
            font_bold: false,
            font_italic: false,
            font_strikeout: false,
            font_underline: false,
            horz_align: HorizontalAlignment::Left,
            vert_align: VerticalAlignment::Top,
            left_border: false,
            top_border: false,
            right_border: false,
            bottom_border: false,
            bg_color: CellColor::WHITE,
            fg_color: CellColor::BLACK,
            rotation: 0.0,
            text: String::new(),
            text_blank: true,
            text_multiline: false,
            text_indent: 0,
        }
    }

    pub fn set_text(&mut self, text: String) {
        self.text_blank = java_is_blank(&text);
        self.text_multiline = text.contains('\n');
        self.text_indent = text.chars().take_while(|&c| c == ' ').count();
        self.text = text;
    }
}

#[derive(Clone, Debug)]
pub struct SubrowData {
    pub col_start: usize,
    pub col_end: usize,
}

#[derive(Clone, Debug)]
pub struct RowData {
    pub subrows: Vec<SubrowData>,
}

#[derive(Clone, Debug)]
pub struct SubtableData {
    pub row_start: usize,
    pub row_end: usize,
}

#[derive(Clone, Debug)]
pub struct SyntaxCore {
    pub num_rows: usize,
    pub num_cols: usize,
    cells: Vec<CellData>, // row-major
    pub rows: Vec<RowData>,
    pub subtables: Vec<SubtableData>,
}

impl SyntaxCore {
    /// Default structure: 1 subtable, N rows, 1 full-width subrow per row.
    pub fn new(num_rows: usize, num_cols: usize) -> CoreResult<Self> {
        if num_rows == 0 {
            return Err(format!("numRows must be positive: {num_rows}").into());
        }
        if num_cols == 0 {
            return Err(format!("numCols must be positive: {num_cols}").into());
        }
        let mut cells = Vec::with_capacity(num_rows * num_cols);
        let mut rows = Vec::with_capacity(num_rows);
        for r in 0..num_rows {
            for c in 0..num_cols {
                cells.push(CellData::new(r, c));
            }
            rows.push(RowData {
                subrows: vec![SubrowData {
                    col_start: 0,
                    col_end: num_cols - 1,
                }],
            });
        }
        Ok(SyntaxCore {
            num_rows,
            num_cols,
            cells,
            rows,
            subtables: vec![SubtableData {
                row_start: 0,
                row_end: num_rows - 1,
            }],
        })
    }

    #[inline]
    pub fn cell(&self, row: usize, col: usize) -> &CellData {
        &self.cells[row * self.num_cols + col]
    }

    #[inline]
    pub fn cell_mut(&mut self, row: usize, col: usize) -> &mut CellData {
        &mut self.cells[row * self.num_cols + col]
    }

    pub fn check_bounds(&self, row: usize, col: usize) -> CoreResult<()> {
        if row >= self.num_rows {
            return Err(format!("row: {row}").into());
        }
        if col >= self.num_cols {
            return Err(format!("col: {col}").into());
        }
        Ok(())
    }

    /// Index of the subtable that contains the given row.
    pub fn subtable_of_row(&self, row: usize) -> Option<usize> {
        self.subtables
            .iter()
            .position(|st| st.row_start <= row && row <= st.row_end)
    }

    /// Index (within the row) of the subrow containing the given column.
    pub fn subrow_of(&self, row: usize, col: usize) -> Option<usize> {
        self.rows[row]
            .subrows
            .iter()
            .position(|sr| sr.col_start <= col && col <= sr.col_end)
    }

    /// Cells of a row in subrow order (matches Java `cellsOf(row)`).
    pub fn cells_of_row(&self, row: usize) -> Vec<(usize, usize)> {
        let mut out = Vec::new();
        for sr in &self.rows[row].subrows {
            for c in sr.col_start..=sr.col_end {
                out.push((row, c));
            }
        }
        out
    }

    /// Redefines the subtable partitioning; replaces all subtables.
    pub fn define_subtables(&mut self, boundaries: &[usize]) -> CoreResult<()> {
        if boundaries.is_empty() {
            return Err("At least one boundary is required".into());
        }
        if boundaries[0] != 0 {
            return Err(format!("First boundary must be 0, got: {}", boundaries[0]).into());
        }
        for i in 1..boundaries.len() {
            if boundaries[i] <= boundaries[i - 1] {
                return Err(format!(
                    "Boundaries must be strictly ascending: {} >= {}",
                    boundaries[i - 1],
                    boundaries[i]
                )
                .into());
            }
            if boundaries[i] >= self.num_rows {
                return Err(format!("Boundary out of range: {}", boundaries[i]).into());
            }
        }
        self.subtables.clear();
        for i in 0..boundaries.len() {
            let row_start = boundaries[i];
            let row_end = if i + 1 < boundaries.len() {
                boundaries[i + 1] - 1
            } else {
                self.num_rows - 1
            };
            self.subtables.push(SubtableData { row_start, row_end });
        }
        Ok(())
    }

    /// Defines a subrow within a row. First call for a row replaces the
    /// default full-width subrow; subsequent calls append.
    pub fn define_subrow(&mut self, row: usize, col_start: usize, col_end: usize) -> CoreResult<()> {
        if row >= self.num_rows {
            return Err(format!("rowIndex: {row}").into());
        }
        if col_start >= self.num_cols {
            return Err(format!("colStart: {col_start}").into());
        }
        if col_end < col_start || col_end >= self.num_cols {
            return Err(format!("colEnd must be in [colStart, numCols-1]: {col_end}").into());
        }
        let row_data = &mut self.rows[row];
        let is_default = row_data.subrows.len() == 1
            && row_data.subrows[0].col_start == 0
            && row_data.subrows[0].col_end == self.num_cols - 1;
        if is_default {
            row_data.subrows.clear();
        }
        row_data.subrows.push(SubrowData { col_start, col_end });
        Ok(())
    }
}
