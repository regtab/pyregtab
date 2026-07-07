//! PyO3 bindings: the public Python API mirroring jRegTab.

use crate::interp::{ActionStrategy, InterpreterCfg, SchemaStrategy};
use crate::matcher;
use crate::recordset::{RecordCore, RecordsetCore, Schema as SchemaCore};
use crate::rtl::{self, BindingsCore, RtlErr};
use crate::semantics::{CtxItem, SemanticsCore};
use crate::spec as sp;
use crate::spec::{EvalEnv, PyFunc};
use crate::syntax::{
    CellColor as ColorCore, FontFamily, HorizontalAlignment, SyntaxCore, VerticalAlignment,
};
use crate::util::{CoreErr, CoreResult};
use pyo3::create_exception;
use pyo3::exceptions::{PyException, PyIndexError, PyKeyError, PyRuntimeError, PyValueError};
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList, PyTuple};
use std::collections::BTreeSet;
use std::sync::Arc;

create_exception!(pyregtab, RtlCompileError, PyException);

fn core_err(e: CoreErr) -> PyErr {
    match e {
        CoreErr::Msg(m) => PyRuntimeError::new_err(m),
        CoreErr::Py(e) => e,
    }
}

fn rtl_err(e: RtlErr) -> PyErr {
    let msg = if e.line >= 0 {
        format!("RTL compile error at {}:{}: {}", e.line, e.col, e.msg)
    } else {
        format!("RTL compile error: {}", e.msg)
    };
    RtlCompileError::new_err(msg)
}

// ================================================================ callbacks

/// Invoked by `CellPredicate::External/Custom`.
pub fn call_cell_predicate(
    func: &PyFunc,
    env: &EvalEnv,
    row: usize,
    col: usize,
) -> CoreResult<bool> {
    let table = env
        .py_table
        .ok_or_else(|| CoreErr::from("Custom cell predicate requires a table context"))?;
    Python::with_gil(|py| -> CoreResult<bool> {
        let table: Py<PyTableSyntax> = table.extract(py).map_err(CoreErr::Py)?;
        let cell = PyCell0 { table, row, col };
        let res = func.0.bind(py).call1((cell,)).map_err(CoreErr::Py)?;
        res.is_truthy().map_err(CoreErr::Py)
    })
}

/// Invoked by `FilterTerm::External/Custom` and `FilterCond::Custom`.
pub fn call_item_filter(
    func: &PyFunc,
    env: &EvalEnv,
    a: &crate::semantics::CellItem,
    c: &crate::semantics::CellItem,
) -> CoreResult<bool> {
    Python::with_gil(|py| -> CoreResult<bool> {
        let table: Option<Py<PyTableSyntax>> = match env.py_table {
            Some(t) => Some(t.extract(py).map_err(CoreErr::Py)?),
            None => None,
        };
        let mk = |it: &crate::semantics::CellItem| PyCellDerivedItem {
            s: it.s.clone(),
            tags: it.tags.clone(),
            index: it.index,
            ty: it.ty,
            row: it.row,
            col: it.col,
            table: table.as_ref().map(|t| t.clone_ref(py)),
        };
        let res = func
            .0
            .bind(py)
            .call1((mk(a), mk(c)))
            .map_err(CoreErr::Py)?;
        res.is_truthy().map_err(CoreErr::Py)
    })
}

/// Invoked by `Extractor::Custom`.
pub fn call_extractor(func: &PyFunc, input: &str) -> CoreResult<String> {
    Python::with_gil(|py| -> CoreResult<String> {
        let res = func.0.bind(py).call1((input,)).map_err(CoreErr::Py)?;
        res.extract::<String>().map_err(CoreErr::Py)
    })
}

/// Invoked by the missing-value handler.
pub fn call_missing_handler(func: &PyFunc, attribute: &str) -> CoreResult<Option<String>> {
    Python::with_gil(|py| -> CoreResult<Option<String>> {
        let res = func.0.bind(py).call1((attribute,)).map_err(CoreErr::Py)?;
        res.extract::<Option<String>>().map_err(CoreErr::Py)
    })
}

// ================================================================ small value types

#[pyclass(name = "GridPosition", frozen, eq)]
#[derive(Clone, PartialEq)]
pub struct PyGridPosition {
    #[pyo3(get)]
    pub row: usize,
    #[pyo3(get)]
    pub col: usize,
}

#[pymethods]
impl PyGridPosition {
    #[new]
    fn new(row: usize, col: usize) -> Self {
        PyGridPosition { row, col }
    }
    fn __repr__(&self) -> String {
        format!("GridPosition(row={}, col={})", self.row, self.col)
    }
}

#[pyclass(name = "BoundingBox", frozen, eq)]
#[derive(Clone, PartialEq)]
pub struct PyBoundingBox {
    #[pyo3(get)]
    pub top_row: usize,
    #[pyo3(get)]
    pub left_col: usize,
    #[pyo3(get)]
    pub bottom_row: usize,
    #[pyo3(get)]
    pub right_col: usize,
}

#[pymethods]
impl PyBoundingBox {
    fn row_span(&self) -> usize {
        self.bottom_row - self.top_row + 1
    }
    fn col_span(&self) -> usize {
        self.right_col - self.left_col + 1
    }
    fn __repr__(&self) -> String {
        format!(
            "BoundingBox(({}, {})..({}, {}))",
            self.top_row, self.left_col, self.bottom_row, self.right_col
        )
    }
}

#[pyclass(name = "CellColor", frozen, eq)]
#[derive(Clone, PartialEq)]
pub struct PyCellColor {
    #[pyo3(get)]
    pub r: u8,
    #[pyo3(get)]
    pub g: u8,
    #[pyo3(get)]
    pub b: u8,
}

#[pymethods]
impl PyCellColor {
    #[new]
    fn new(r: u8, g: u8, b: u8) -> Self {
        PyCellColor { r, g, b }
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn BLACK() -> PyCellColor {
        PyCellColor { r: 0, g: 0, b: 0 }
    }
    #[classattr]
    #[allow(non_snake_case)]
    fn WHITE() -> PyCellColor {
        PyCellColor { r: 255, g: 255, b: 255 }
    }
    fn __repr__(&self) -> String {
        format!("CellColor({}, {}, {})", self.r, self.g, self.b)
    }
}

impl From<ColorCore> for PyCellColor {
    fn from(c: ColorCore) -> Self {
        PyCellColor { r: c.r, g: c.g, b: c.b }
    }
}
impl From<&PyCellColor> for ColorCore {
    fn from(c: &PyCellColor) -> Self {
        ColorCore { r: c.r, g: c.g, b: c.b }
    }
}

// ================================================================ TableSyntax + handles

#[pyclass(name = "TableSyntax")]
pub struct PyTableSyntax {
    pub core: SyntaxCore,
}

#[pymethods]
impl PyTableSyntax {
    #[new]
    fn new(num_rows: usize, num_cols: usize) -> PyResult<Self> {
        Ok(PyTableSyntax {
            core: SyntaxCore::new(num_rows, num_cols).map_err(core_err)?,
        })
    }

    #[getter]
    fn num_rows(&self) -> usize {
        self.core.num_rows
    }
    #[getter]
    fn num_cols(&self) -> usize {
        self.core.num_cols
    }

    fn cell(slf: &Bound<'_, Self>, row: usize, col: usize) -> PyResult<PyCell0> {
        {
            let core = &slf.borrow().core;
            core.check_bounds(row, col).map_err(core_err)?;
        }
        Ok(PyCell0 { table: slf.clone().unbind(), row, col })
    }

    /// Alias mirroring Java's `getCell`.
    fn get_cell(slf: &Bound<'_, Self>, row: usize, col: usize) -> PyResult<PyCell0> {
        Self::cell(slf, row, col)
    }

    fn row(slf: &Bound<'_, Self>, index: usize) -> PyResult<PyRow> {
        if index >= slf.borrow().core.num_rows {
            return Err(PyIndexError::new_err(format!("row: {index}")));
        }
        Ok(PyRow { table: slf.clone().unbind(), index })
    }

    fn rows(slf: &Bound<'_, Self>) -> Vec<PyRow> {
        let n = slf.borrow().core.num_rows;
        (0..n)
            .map(|i| PyRow { table: slf.clone().unbind(), index: i })
            .collect()
    }

    fn subtables(slf: &Bound<'_, Self>) -> Vec<PySubtable> {
        let n = slf.borrow().core.subtables.len();
        (0..n)
            .map(|i| PySubtable { table: slf.clone().unbind(), index: i })
            .collect()
    }

    fn all_cells(slf: &Bound<'_, Self>) -> Vec<PyCell0> {
        let (nr, nc) = {
            let c = &slf.borrow().core;
            (c.num_rows, c.num_cols)
        };
        let mut out = Vec::with_capacity(nr * nc);
        for r in 0..nr {
            for c in 0..nc {
                out.push(PyCell0 { table: slf.clone().unbind(), row: r, col: c });
            }
        }
        out
    }

    #[pyo3(signature = (*boundaries))]
    fn define_subtables(&mut self, boundaries: Vec<usize>) -> PyResult<()> {
        self.core.define_subtables(&boundaries).map_err(core_err)
    }

    fn define_subrow(&mut self, row_index: usize, col_start: usize, col_end: usize) -> PyResult<()> {
        self.core
            .define_subrow(row_index, col_start, col_end)
            .map_err(core_err)
    }

    fn __repr__(&self) -> String {
        format!(
            "TableSyntax({}x{})",
            self.core.num_rows, self.core.num_cols
        )
    }
}

/// Cell handle: (table, row, col).
#[pyclass(name = "Cell")]
pub struct PyCell0 {
    pub table: Py<PyTableSyntax>,
    pub row: usize,
    pub col: usize,
}

macro_rules! cell_get {
    ($self:ident, $py:ident, $field:ident) => {{
        let t = $self.table.bind($py).borrow();
        t.core.cell($self.row, $self.col).$field.clone()
    }};
}
macro_rules! cell_set {
    ($self:ident, $py:ident, $field:ident, $value:expr) => {{
        let mut t = $self.table.bind($py).borrow_mut();
        t.core.cell_mut($self.row, $self.col).$field = $value;
    }};
}

#[pymethods]
impl PyCell0 {
    #[getter]
    fn row(&self) -> usize {
        self.row
    }
    #[getter]
    fn col(&self) -> usize {
        self.col
    }
    #[getter]
    fn pos(&self) -> PyGridPosition {
        PyGridPosition { row: self.row, col: self.col }
    }
    #[getter]
    fn bbox(&self, py: Python<'_>) -> PyBoundingBox {
        let t = self.table.bind(py).borrow();
        let b = t.core.cell(self.row, self.col).bbox;
        PyBoundingBox {
            top_row: b.top_row,
            left_col: b.left_col,
            bottom_row: b.bottom_row,
            right_col: b.right_col,
        }
    }
    #[getter]
    fn merged(&self, py: Python<'_>) -> bool {
        cell_get!(self, py, merged)
    }

    // --- content ---
    #[getter]
    fn text(&self, py: Python<'_>) -> String {
        cell_get!(self, py, text)
    }
    #[setter(text)]
    fn set_text_prop(&self, py: Python<'_>, text: String) {
        let mut t = self.table.bind(py).borrow_mut();
        t.core.cell_mut(self.row, self.col).set_text(text);
    }
    fn set_text(&self, py: Python<'_>, text: String) {
        self.set_text_prop(py, text);
    }
    #[getter]
    fn text_blank(&self, py: Python<'_>) -> bool {
        cell_get!(self, py, text_blank)
    }
    #[getter]
    fn text_multiline(&self, py: Python<'_>) -> bool {
        cell_get!(self, py, text_multiline)
    }
    #[getter]
    fn text_indent(&self, py: Python<'_>) -> usize {
        cell_get!(self, py, text_indent)
    }

    // --- formatting ---
    #[getter]
    fn font_family(&self, py: Python<'_>) -> FontFamily {
        cell_get!(self, py, font_family)
    }
    #[setter(font_family)]
    fn set_font_family(&self, py: Python<'_>, v: FontFamily) {
        cell_set!(self, py, font_family, v);
    }
    #[getter]
    fn font_bold(&self, py: Python<'_>) -> bool {
        cell_get!(self, py, font_bold)
    }
    #[setter(font_bold)]
    fn set_font_bold(&self, py: Python<'_>, v: bool) {
        cell_set!(self, py, font_bold, v);
    }
    #[getter]
    fn font_italic(&self, py: Python<'_>) -> bool {
        cell_get!(self, py, font_italic)
    }
    #[setter(font_italic)]
    fn set_font_italic(&self, py: Python<'_>, v: bool) {
        cell_set!(self, py, font_italic, v);
    }
    #[getter]
    fn font_strikeout(&self, py: Python<'_>) -> bool {
        cell_get!(self, py, font_strikeout)
    }
    #[setter(font_strikeout)]
    fn set_font_strikeout(&self, py: Python<'_>, v: bool) {
        cell_set!(self, py, font_strikeout, v);
    }
    #[getter]
    fn font_underline(&self, py: Python<'_>) -> bool {
        cell_get!(self, py, font_underline)
    }
    #[setter(font_underline)]
    fn set_font_underline(&self, py: Python<'_>, v: bool) {
        cell_set!(self, py, font_underline, v);
    }
    #[getter]
    fn horz_align(&self, py: Python<'_>) -> HorizontalAlignment {
        cell_get!(self, py, horz_align)
    }
    #[setter(horz_align)]
    fn set_horz_align(&self, py: Python<'_>, v: HorizontalAlignment) {
        cell_set!(self, py, horz_align, v);
    }
    #[getter]
    fn vert_align(&self, py: Python<'_>) -> VerticalAlignment {
        cell_get!(self, py, vert_align)
    }
    #[setter(vert_align)]
    fn set_vert_align(&self, py: Python<'_>, v: VerticalAlignment) {
        cell_set!(self, py, vert_align, v);
    }
    #[getter]
    fn left_border(&self, py: Python<'_>) -> bool {
        cell_get!(self, py, left_border)
    }
    #[setter(left_border)]
    fn set_left_border(&self, py: Python<'_>, v: bool) {
        cell_set!(self, py, left_border, v);
    }
    #[getter]
    fn top_border(&self, py: Python<'_>) -> bool {
        cell_get!(self, py, top_border)
    }
    #[setter(top_border)]
    fn set_top_border(&self, py: Python<'_>, v: bool) {
        cell_set!(self, py, top_border, v);
    }
    #[getter]
    fn right_border(&self, py: Python<'_>) -> bool {
        cell_get!(self, py, right_border)
    }
    #[setter(right_border)]
    fn set_right_border(&self, py: Python<'_>, v: bool) {
        cell_set!(self, py, right_border, v);
    }
    #[getter]
    fn bottom_border(&self, py: Python<'_>) -> bool {
        cell_get!(self, py, bottom_border)
    }
    #[setter(bottom_border)]
    fn set_bottom_border(&self, py: Python<'_>, v: bool) {
        cell_set!(self, py, bottom_border, v);
    }
    #[getter]
    fn bg_color(&self, py: Python<'_>) -> PyCellColor {
        cell_get!(self, py, bg_color).into()
    }
    #[setter(bg_color)]
    fn set_bg_color(&self, py: Python<'_>, v: PyCellColor) {
        cell_set!(self, py, bg_color, (&v).into());
    }
    #[getter]
    fn fg_color(&self, py: Python<'_>) -> PyCellColor {
        cell_get!(self, py, fg_color).into()
    }
    #[setter(fg_color)]
    fn set_fg_color(&self, py: Python<'_>, v: PyCellColor) {
        cell_set!(self, py, fg_color, (&v).into());
    }
    #[getter]
    fn rotation(&self, py: Python<'_>) -> f64 {
        cell_get!(self, py, rotation)
    }
    #[setter(rotation)]
    fn set_rotation(&self, py: Python<'_>, v: f64) {
        cell_set!(self, py, rotation, v);
    }

    // --- structure ---
    #[getter]
    fn parent_row(&self, py: Python<'_>) -> PyRow {
        PyRow { table: self.table.clone_ref(py), index: self.row }
    }
    #[getter]
    fn subtable(&self, py: Python<'_>) -> Option<PySubtable> {
        let idx = {
            let t = self.table.bind(py).borrow();
            t.core.subtable_of_row(self.row)
        };
        idx.map(|i| PySubtable { table: self.table.clone_ref(py), index: i })
    }
    #[getter]
    fn subrow(&self, py: Python<'_>) -> Option<PySubrow> {
        let idx = {
            let t = self.table.bind(py).borrow();
            t.core.subrow_of(self.row, self.col)
        };
        idx.map(|i| PySubrow {
            table: self.table.clone_ref(py),
            row: self.row,
            index: i,
        })
    }

    fn __repr__(&self, py: Python<'_>) -> String {
        format!(
            "Cell[pos=GridPosition(row={}, col={}), text=\"{}\"]",
            self.row,
            self.col,
            self.text(py)
        )
    }
}

#[pyclass(name = "Row")]
pub struct PyRow {
    pub table: Py<PyTableSyntax>,
    pub index: usize,
}

#[pymethods]
impl PyRow {
    #[getter]
    fn index(&self) -> usize {
        self.index
    }
    fn subrows(&self, py: Python<'_>) -> Vec<PySubrow> {
        let n = {
            let t = self.table.bind(py).borrow();
            t.core.rows[self.index].subrows.len()
        };
        (0..n)
            .map(|i| PySubrow {
                table: self.table.clone_ref(py),
                row: self.index,
                index: i,
            })
            .collect()
    }
    #[getter]
    fn subtable(&self, py: Python<'_>) -> Option<PySubtable> {
        let idx = {
            let t = self.table.bind(py).borrow();
            t.core.subtable_of_row(self.index)
        };
        idx.map(|i| PySubtable { table: self.table.clone_ref(py), index: i })
    }
}

#[pyclass(name = "Subrow")]
pub struct PySubrow {
    pub table: Py<PyTableSyntax>,
    pub row: usize,
    pub index: usize,
}

#[pymethods]
impl PySubrow {
    #[getter]
    fn col_start(&self, py: Python<'_>) -> usize {
        let t = self.table.bind(py).borrow();
        t.core.rows[self.row].subrows[self.index].col_start
    }
    #[getter]
    fn col_end(&self, py: Python<'_>) -> usize {
        let t = self.table.bind(py).borrow();
        t.core.rows[self.row].subrows[self.index].col_end
    }
    fn cells(&self, py: Python<'_>) -> Vec<PyCell0> {
        let (cs, ce) = {
            let t = self.table.bind(py).borrow();
            let sr = &t.core.rows[self.row].subrows[self.index];
            (sr.col_start, sr.col_end)
        };
        (cs..=ce)
            .map(|c| PyCell0 {
                table: self.table.clone_ref(py),
                row: self.row,
                col: c,
            })
            .collect()
    }
}

#[pyclass(name = "Subtable")]
pub struct PySubtable {
    pub table: Py<PyTableSyntax>,
    pub index: usize,
}

#[pymethods]
impl PySubtable {
    #[getter]
    fn row_start(&self, py: Python<'_>) -> usize {
        let t = self.table.bind(py).borrow();
        t.core.subtables[self.index].row_start
    }
    #[getter]
    fn row_end(&self, py: Python<'_>) -> usize {
        let t = self.table.bind(py).borrow();
        t.core.subtables[self.index].row_end
    }
    fn rows(&self, py: Python<'_>) -> Vec<PyRow> {
        let (rs, re) = {
            let t = self.table.bind(py).borrow();
            let st = &t.core.subtables[self.index];
            (st.row_start, st.row_end)
        };
        (rs..=re)
            .map(|r| PyRow { table: self.table.clone_ref(py), index: r })
            .collect()
    }
}

// ================================================================ items

#[pyclass(name = "CellDerivedItem")]
pub struct PyCellDerivedItem {
    pub s: String,
    pub tags: Vec<String>,
    pub index: usize,
    pub ty: sp::ItemType,
    pub row: usize,
    pub col: usize,
    pub table: Option<Py<PyTableSyntax>>,
}

#[pymethods]
impl PyCellDerivedItem {
    #[getter]
    fn str(&self) -> String {
        self.s.clone()
    }
    #[getter]
    fn tags(&self) -> Vec<String> {
        self.tags.clone()
    }
    #[getter]
    fn index(&self) -> usize {
        self.index
    }
    #[getter]
    fn type_(&self) -> sp::ItemType {
        self.ty
    }
    #[getter]
    fn cell(&self, py: Python<'_>) -> PyResult<PyCell0> {
        match &self.table {
            Some(t) => Ok(PyCell0 {
                table: t.clone_ref(py),
                row: self.row,
                col: self.col,
            }),
            None => Err(PyRuntimeError::new_err("item has no bound table")),
        }
    }
    fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
    fn __repr__(&self) -> String {
        format!(
            "CellDerivedItem[str=\"{}\", index={}, cell=({}, {}), type={:?}]",
            self.s, self.index, self.row, self.col, self.ty
        )
    }
}

#[pyclass(name = "ContextDerivedItem")]
#[derive(Clone)]
pub struct PyContextDerivedItem {
    pub core: CtxItem,
}

#[pymethods]
impl PyContextDerivedItem {
    #[new]
    #[pyo3(signature = (s, ty, const_value=None))]
    fn new(s: String, ty: sp::ItemType, const_value: Option<String>) -> Self {
        PyContextDerivedItem {
            core: CtxItem { s, ty, const_value },
        }
    }
    #[getter]
    fn str(&self) -> String {
        self.core.s.clone()
    }
    #[getter]
    fn type_(&self) -> sp::ItemType {
        self.core.ty
    }
    #[getter]
    fn const_value(&self) -> Option<String> {
        self.core.const_value.clone()
    }
    fn __repr__(&self) -> String {
        format!(
            "ContextDerivedItem[str=\"{}\", type={:?}]",
            self.core.s, self.core.ty
        )
    }
}

// ================================================================ recordset

#[pyclass(name = "Schema", frozen, eq)]
#[derive(Clone, PartialEq)]
pub struct PySchema {
    pub core: SchemaCore,
}

#[pymethods]
impl PySchema {
    #[new]
    fn new(attributes: Vec<String>) -> PyResult<Self> {
        Ok(PySchema {
            core: SchemaCore::new(attributes).map_err(core_err)?,
        })
    }
    #[getter]
    fn attributes(&self) -> Vec<String> {
        self.core.attributes.clone()
    }
    fn size(&self) -> usize {
        self.core.attributes.len()
    }
    fn __len__(&self) -> usize {
        self.size()
    }
    fn index_of(&self, attribute: &str) -> i64 {
        self.core.index_of(attribute).map(|i| i as i64).unwrap_or(-1)
    }
    fn contains(&self, attribute: &str) -> bool {
        self.core.contains(attribute)
    }
    fn __contains__(&self, attribute: &str) -> bool {
        self.contains(attribute)
    }
    fn __repr__(&self) -> String {
        format!("Schema{:?}", self.core.attributes)
    }
}

#[pyclass(name = "Recordset")]
pub struct PyRecordset {
    pub core: RecordsetCore,
}

#[pymethods]
impl PyRecordset {
    #[new]
    fn new(schema: PySchema, records: Vec<std::collections::HashMap<String, Option<String>>>) -> Self {
        let recs = records
            .into_iter()
            .map(|m| RecordCore {
                values: schema
                    .core
                    .attributes
                    .iter()
                    .map(|a| m.get(a).cloned().flatten())
                    .collect(),
            })
            .collect();
        PyRecordset {
            core: RecordsetCore { schema: schema.core, records: recs },
        }
    }
    #[getter]
    fn schema(&self) -> PySchema {
        PySchema { core: self.core.schema.clone() }
    }
    #[getter]
    fn records(slf: &Bound<'_, Self>) -> Vec<PyRecord> {
        let n = slf.borrow().core.records.len();
        (0..n)
            .map(|i| PyRecord { rs: slf.clone().unbind(), index: i })
            .collect()
    }
    fn size(&self) -> usize {
        self.core.records.len()
    }
    fn __len__(&self) -> usize {
        self.size()
    }
    fn get(slf: &Bound<'_, Self>, index: usize) -> PyResult<PyRecord> {
        if index >= slf.borrow().core.records.len() {
            return Err(PyIndexError::new_err(index));
        }
        Ok(PyRecord { rs: slf.clone().unbind(), index })
    }
    fn __getitem__(slf: &Bound<'_, Self>, index: usize) -> PyResult<PyRecord> {
        Self::get(slf, index)
    }

    /// Optional pandas integration (`pip install pyregtab[pandas]`).
    fn to_pandas(&self, py: Python<'_>) -> PyResult<PyObject> {
        let pandas = py.import("pandas")?;
        let data = PyList::empty(py);
        for r in &self.core.records {
            let row = PyList::empty(py);
            for v in &r.values {
                match v {
                    Some(s) => row.append(s)?,
                    None => row.append(py.None())?,
                }
            }
            data.append(row)?;
        }
        let kwargs = PyDict::new(py);
        kwargs.set_item("columns", self.core.schema.attributes.clone())?;
        Ok(pandas
            .getattr("DataFrame")?
            .call((data,), Some(&kwargs))?
            .unbind())
    }

    fn __repr__(&self) -> String {
        let mut sb = format!(
            "Recordset[schema=Schema{:?}, records=[\n",
            self.core.schema.attributes
        );
        for r in &self.core.records {
            sb.push_str("  Record{");
            let mut first = true;
            for (a, v) in self.core.schema.attributes.iter().zip(&r.values) {
                if !first {
                    sb.push_str(", ");
                }
                match v {
                    Some(v) => sb.push_str(&format!("{a}={v}")),
                    None => sb.push_str(&format!("{a}=None")),
                }
                first = false;
            }
            sb.push_str("}\n");
        }
        sb.push_str("]]");
        sb
    }
}

#[pyclass(name = "Record")]
pub struct PyRecord {
    pub rs: Py<PyRecordset>,
    pub index: usize,
}

#[pymethods]
impl PyRecord {
    #[getter]
    fn schema(&self, py: Python<'_>) -> PySchema {
        let rs = self.rs.bind(py).borrow();
        PySchema { core: rs.core.schema.clone() }
    }
    fn get(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<Option<String>> {
        let rs = self.rs.bind(py).borrow();
        let rec = &rs.core.records[self.index];
        if let Ok(i) = key.extract::<usize>() {
            return rec
                .values
                .get(i)
                .cloned()
                .ok_or_else(|| PyIndexError::new_err(i));
        }
        let attr: String = key.extract()?;
        match rs.core.schema.index_of(&attr) {
            Some(i) => Ok(rec.values[i].clone()),
            None => Ok(None),
        }
    }
    fn __getitem__(&self, py: Python<'_>, key: &Bound<'_, PyAny>) -> PyResult<Option<String>> {
        let rs = self.rs.bind(py).borrow();
        let rec = &rs.core.records[self.index];
        if let Ok(i) = key.extract::<usize>() {
            return rec
                .values
                .get(i)
                .cloned()
                .ok_or_else(|| PyIndexError::new_err(i));
        }
        let attr: String = key.extract()?;
        match rs.core.schema.index_of(&attr) {
            Some(i) => Ok(rec.values[i].clone()),
            None => Err(PyKeyError::new_err(attr)),
        }
    }
    fn values(&self, py: Python<'_>) -> PyResult<PyObject> {
        let rs = self.rs.bind(py).borrow();
        let rec = &rs.core.records[self.index];
        let d = PyDict::new(py);
        for (a, v) in rs.core.schema.attributes.iter().zip(&rec.values) {
            d.set_item(a, v.clone())?;
        }
        Ok(d.unbind().into())
    }
    fn __repr__(&self, py: Python<'_>) -> String {
        let rs = self.rs.bind(py).borrow();
        let rec = &rs.core.records[self.index];
        let mut sb = String::from("Record{");
        let mut first = true;
        for (a, v) in rs.core.schema.attributes.iter().zip(&rec.values) {
            if !first {
                sb.push_str(", ");
            }
            match v {
                Some(v) => sb.push_str(&format!("{a}={v}")),
                None => sb.push_str(&format!("{a}=None")),
            }
            first = false;
        }
        sb.push('}');
        sb
    }
}

// ================================================================ helpers for callables

fn pyfunc(f: Bound<'_, PyAny>) -> PyFunc {
    PyFunc(Arc::new(f.unbind()))
}

// ================================================================ spec wrappers

#[pyclass(name = "Quantifier", frozen, eq)]
#[derive(Clone, PartialEq)]
pub struct PyQuantifier {
    pub core: sp::Quantifier,
}

#[pymethods]
impl PyQuantifier {
    #[staticmethod]
    fn one() -> Self {
        PyQuantifier { core: sp::Quantifier::ONE }
    }
    #[staticmethod]
    fn zero_or_one() -> Self {
        PyQuantifier { core: sp::Quantifier::ZERO_OR_ONE }
    }
    #[staticmethod]
    fn one_or_more() -> Self {
        PyQuantifier { core: sp::Quantifier::ONE_OR_MORE }
    }
    #[staticmethod]
    fn zero_or_more() -> Self {
        PyQuantifier { core: sp::Quantifier::ZERO_OR_MORE }
    }
    #[staticmethod]
    fn exactly(n: i64) -> PyResult<Self> {
        Ok(PyQuantifier { core: sp::Quantifier::exactly(n).map_err(core_err)? })
    }
    #[getter]
    fn kind(&self) -> &'static str {
        match self.core.kind {
            sp::QKind::ZeroOrOne => "ZERO_OR_ONE",
            sp::QKind::One => "ONE",
            sp::QKind::Exactly => "EXACTLY",
            sp::QKind::OneOrMore => "ONE_OR_MORE",
            sp::QKind::ZeroOrMore => "ZERO_OR_MORE",
        }
    }
    #[getter]
    fn n(&self) -> i64 {
        self.core.n
    }
    fn min(&self) -> i64 {
        self.core.min()
    }
    fn max(&self) -> i64 {
        self.core.max()
    }
    fn __repr__(&self) -> String {
        format!("Quantifier({})", self.kind())
    }
}

#[pyclass(name = "CellPredicate", frozen)]
#[derive(Clone)]
pub struct PyCellPredicate {
    pub core: sp::CellPredicate,
}

#[pymethods]
impl PyCellPredicate {
    #[staticmethod]
    fn blank() -> Self {
        PyCellPredicate { core: sp::CellPredicate::Blank }
    }
    #[staticmethod]
    fn not_blank() -> Self {
        PyCellPredicate { core: sp::CellPredicate::NotBlank }
    }
    #[staticmethod]
    fn regex_matched(pattern: String) -> Self {
        PyCellPredicate { core: sp::CellPredicate::Regex(pattern) }
    }
    #[staticmethod]
    fn not_regex_matched(pattern: String) -> Self {
        PyCellPredicate { core: sp::CellPredicate::NotRegex(pattern) }
    }
    #[staticmethod]
    fn contains(substring: String) -> Self {
        PyCellPredicate { core: sp::CellPredicate::Contains(substring) }
    }
    #[staticmethod]
    fn not_contains(substring: String) -> Self {
        PyCellPredicate { core: sp::CellPredicate::NotContains(substring) }
    }
    #[staticmethod]
    fn external(name: String, predicate: Bound<'_, PyAny>) -> Self {
        PyCellPredicate {
            core: sp::CellPredicate::External { name, func: pyfunc(predicate) },
        }
    }
    #[staticmethod]
    fn custom(description: String, predicate: Bound<'_, PyAny>) -> Self {
        PyCellPredicate {
            core: sp::CellPredicate::Custom { description, func: pyfunc(predicate) },
        }
    }
    fn to_rtl(&self) -> PyResult<String> {
        self.core.to_rtl().map_err(core_err)
    }
    fn test(&self, py: Python<'_>, cell: &PyCell0) -> PyResult<bool> {
        let table = cell.table.bind(py).borrow();
        let any: Py<PyAny> = cell.table.clone_ref(py).into_any();
        let env = EvalEnv { syntax: &table.core, py_table: Some(&any) };
        self.core
            .test(table.core.cell(cell.row, cell.col), &env)
            .map_err(core_err)
    }
}

#[pyclass(name = "CellMatchCondition", frozen)]
#[derive(Clone)]
pub struct PyCellMatchCondition {
    pub core: sp::CellPredicate,
}

#[pymethods]
impl PyCellMatchCondition {
    #[new]
    fn new(cell_predicate: PyCellPredicate) -> Self {
        PyCellMatchCondition { core: cell_predicate.core }
    }
    #[getter]
    fn cell_predicate(&self) -> PyCellPredicate {
        PyCellPredicate { core: self.core.clone() }
    }
    fn test(&self, py: Python<'_>, cell: &PyCell0) -> PyResult<bool> {
        PyCellPredicate { core: self.core.clone() }.test(py, cell)
    }
}

#[pyclass(name = "FilterTerm", frozen)]
#[derive(Clone)]
pub struct PyFilterTerm {
    pub core: sp::FilterTerm,
}

#[pymethods]
impl PyFilterTerm {
    #[staticmethod]
    fn left_of() -> Self {
        PyFilterTerm { core: sp::FilterTerm::LeftOf }
    }
    #[staticmethod]
    fn right_of() -> Self {
        PyFilterTerm { core: sp::FilterTerm::RightOf }
    }
    #[staticmethod]
    fn above() -> Self {
        PyFilterTerm { core: sp::FilterTerm::Above }
    }
    #[staticmethod]
    fn below() -> Self {
        PyFilterTerm { core: sp::FilterTerm::Below }
    }
    #[staticmethod]
    fn same_subrow() -> Self {
        PyFilterTerm { core: sp::FilterTerm::SameSubrow }
    }
    #[staticmethod]
    fn same_subcol() -> Self {
        PyFilterTerm { core: sp::FilterTerm::SameSubcol }
    }
    #[staticmethod]
    fn same_subtable() -> Self {
        PyFilterTerm { core: sp::FilterTerm::SameSubtable }
    }
    #[staticmethod]
    fn same_row() -> Self {
        PyFilterTerm { core: sp::FilterTerm::SameRow }
    }
    #[staticmethod]
    fn same_col() -> Self {
        PyFilterTerm { core: sp::FilterTerm::SameCol }
    }
    #[staticmethod]
    fn not_same_cell() -> Self {
        PyFilterTerm { core: sp::FilterTerm::NotSameCell }
    }
    #[staticmethod]
    fn same_cell() -> Self {
        PyFilterTerm { core: sp::FilterTerm::SameCell }
    }
    #[staticmethod]
    fn col_exact(n: i64) -> Self {
        PyFilterTerm { core: sp::FilterTerm::ColExact(n) }
    }
    #[staticmethod]
    fn col_offset(delta: i64) -> Self {
        PyFilterTerm { core: sp::FilterTerm::ColOffset(delta) }
    }
    #[staticmethod]
    #[pyo3(signature = (from, to=None))]
    fn col_range(from: i64, to: Option<i64>) -> Self {
        PyFilterTerm {
            core: sp::FilterTerm::ColRange(from, to.unwrap_or(sp::UNBOUNDED)),
        }
    }
    #[staticmethod]
    #[pyo3(signature = (lo, hi=None))]
    fn col_absolute_range(lo: i64, hi: Option<i64>) -> Self {
        PyFilterTerm {
            core: sp::FilterTerm::ColAbsoluteRange(lo, hi.unwrap_or(sp::UNBOUNDED)),
        }
    }
    #[staticmethod]
    fn row_exact(n: i64) -> Self {
        PyFilterTerm { core: sp::FilterTerm::RowExact(n) }
    }
    #[staticmethod]
    fn row_offset(delta: i64) -> Self {
        PyFilterTerm { core: sp::FilterTerm::RowOffset(delta) }
    }
    #[staticmethod]
    #[pyo3(signature = (lo, hi=None))]
    fn row_absolute_range(lo: i64, hi: Option<i64>) -> Self {
        PyFilterTerm {
            core: sp::FilterTerm::RowAbsoluteRange(lo, hi.unwrap_or(sp::UNBOUNDED)),
        }
    }
    #[staticmethod]
    fn pos_exact(n: i64) -> Self {
        PyFilterTerm { core: sp::FilterTerm::PosExact(n) }
    }
    #[staticmethod]
    fn pos_offset(delta: i64) -> Self {
        PyFilterTerm { core: sp::FilterTerm::PosOffset(delta) }
    }
    #[staticmethod]
    #[pyo3(signature = (lo, hi=None))]
    fn pos_range(lo: i64, hi: Option<i64>) -> Self {
        PyFilterTerm {
            core: sp::FilterTerm::PosRange(lo, hi.unwrap_or(sp::UNBOUNDED)),
        }
    }
    #[staticmethod]
    fn regex_matched(pattern: String) -> Self {
        PyFilterTerm { core: sp::FilterTerm::Regex(pattern) }
    }
    #[staticmethod]
    fn not_regex_matched(pattern: String) -> Self {
        PyFilterTerm { core: sp::FilterTerm::NotRegex(pattern) }
    }
    #[staticmethod]
    fn contains(substring: String) -> Self {
        PyFilterTerm { core: sp::FilterTerm::Contains(substring) }
    }
    #[staticmethod]
    fn not_contains(substring: String) -> Self {
        PyFilterTerm { core: sp::FilterTerm::NotContains(substring) }
    }
    #[staticmethod]
    fn blank() -> Self {
        PyFilterTerm { core: sp::FilterTerm::Blank }
    }
    #[staticmethod]
    fn not_blank() -> Self {
        PyFilterTerm { core: sp::FilterTerm::NotBlank }
    }
    #[staticmethod]
    fn tagged(tag: String) -> Self {
        PyFilterTerm { core: sp::FilterTerm::Tagged(tag) }
    }
    #[staticmethod]
    fn not_tagged(tag: String) -> Self {
        PyFilterTerm { core: sp::FilterTerm::NotTagged(tag) }
    }
    #[staticmethod]
    fn same_str() -> Self {
        PyFilterTerm { core: sp::FilterTerm::SameStr }
    }
    #[staticmethod]
    fn external(name: String, predicate: Bound<'_, PyAny>) -> Self {
        PyFilterTerm {
            core: sp::FilterTerm::External { name, func: pyfunc(predicate) },
        }
    }
    #[staticmethod]
    fn custom(description: String, predicate: Bound<'_, PyAny>) -> Self {
        PyFilterTerm {
            core: sp::FilterTerm::Custom { description, func: pyfunc(predicate) },
        }
    }
    fn to_rtl(&self) -> PyResult<String> {
        self.core.to_rtl().map_err(core_err)
    }
}

#[pyclass(name = "ItemFilterConditionSpec", frozen)]
#[derive(Clone)]
pub struct PyFilterCond {
    pub core: sp::FilterCond,
}

fn terms_from_args(args: &Bound<'_, PyTuple>) -> PyResult<Vec<sp::FilterTerm>> {
    let mut out = Vec::new();
    for a in args.iter() {
        out.push(a.extract::<PyFilterTerm>()?.core);
    }
    Ok(out)
}

#[pymethods]
impl PyFilterCond {
    #[staticmethod]
    fn bare(constraint: PyFilterTerm) -> Self {
        PyFilterCond { core: sp::FilterCond::Bare(constraint.core) }
    }
    #[staticmethod]
    #[pyo3(name = "and_", signature = (*terms))]
    fn and_(terms: &Bound<'_, PyTuple>) -> PyResult<Self> {
        Ok(PyFilterCond { core: sp::FilterCond::And(terms_from_args(terms)?) })
    }
    #[staticmethod]
    #[pyo3(name = "or_", signature = (*groups))]
    fn or_(groups: &Bound<'_, PyTuple>) -> PyResult<Self> {
        let mut out = Vec::new();
        for g in groups.iter() {
            let cond = g.extract::<PyFilterCond>()?;
            match cond.core {
                sp::FilterCond::Bare(t) => out.push(vec![t]),
                sp::FilterCond::And(ts) => out.push(ts),
                sp::FilterCond::Or(gs) => out.extend(gs),
                sp::FilterCond::Custom { .. } => {
                    return Err(PyValueError::new_err("Custom spec cannot be used in OR"))
                }
            }
        }
        Ok(PyFilterCond { core: sp::FilterCond::Or(out) })
    }
    #[staticmethod]
    fn custom(description: String, predicate: Bound<'_, PyAny>) -> Self {
        PyFilterCond {
            core: sp::FilterCond::Custom { description, func: pyfunc(predicate) },
        }
    }
    // bare shorthands
    #[staticmethod]
    fn same_subtable() -> Self {
        Self::bare(PyFilterTerm::same_subtable())
    }
    #[staticmethod]
    fn same_subrow() -> Self {
        Self::bare(PyFilterTerm::same_subrow())
    }
    #[staticmethod]
    fn same_subcol() -> Self {
        Self::bare(PyFilterTerm::same_subcol())
    }
    #[staticmethod]
    fn same_cell() -> Self {
        Self::bare(PyFilterTerm::same_cell())
    }
    #[staticmethod]
    fn same_row() -> Self {
        Self::bare(PyFilterTerm::same_row())
    }
    #[staticmethod]
    fn same_col() -> Self {
        Self::bare(PyFilterTerm::same_col())
    }
    #[staticmethod]
    fn below() -> Self {
        Self::bare(PyFilterTerm::below())
    }
    #[staticmethod]
    fn above() -> Self {
        Self::bare(PyFilterTerm::above())
    }
    #[staticmethod]
    fn right_of() -> Self {
        Self::bare(PyFilterTerm::right_of())
    }
    #[staticmethod]
    fn left_of() -> Self {
        Self::bare(PyFilterTerm::left_of())
    }
    fn to_rtl(&self) -> PyResult<String> {
        self.core.to_rtl().map_err(core_err)
    }
}

#[pyclass(name = "StringExtractor", frozen)]
#[derive(Clone)]
pub struct PyExtractor {
    pub core: sp::Extractor,
}

#[pymethods]
impl PyExtractor {
    #[staticmethod]
    fn verbatim() -> Self {
        PyExtractor { core: sp::Extractor::Verbatim }
    }
    #[staticmethod]
    fn replaced(regex: String, replacement: String) -> Self {
        PyExtractor { core: sp::Extractor::Replaced(regex, replacement) }
    }
    #[staticmethod]
    fn whitespace_normalized() -> Self {
        PyExtractor { core: sp::Extractor::WhitespaceNormalized }
    }
    #[staticmethod]
    fn trimmed() -> Self {
        PyExtractor { core: sp::Extractor::Trimmed }
    }
    #[staticmethod]
    fn substring(begin: i64, end: i64) -> Self {
        PyExtractor { core: sp::Extractor::Substring(begin, end) }
    }
    #[staticmethod]
    fn upper_case() -> Self {
        PyExtractor { core: sp::Extractor::UpperCase }
    }
    #[staticmethod]
    fn lower_case() -> Self {
        PyExtractor { core: sp::Extractor::LowerCase }
    }
    #[staticmethod]
    #[pyo3(signature = (*steps))]
    fn chain(steps: &Bound<'_, PyTuple>) -> PyResult<Self> {
        let mut out = Vec::new();
        for s in steps.iter() {
            out.push(s.extract::<PyExtractor>()?.core);
        }
        Ok(PyExtractor { core: sp::Extractor::Chain(out) })
    }
    #[staticmethod]
    fn custom(description: String, fn_: Bound<'_, PyAny>) -> Self {
        PyExtractor {
            core: sp::Extractor::Custom { description, func: pyfunc(fn_) },
        }
    }
    fn apply(&self, input: &str) -> PyResult<String> {
        self.core.apply(input).map_err(core_err)
    }
    fn to_rtl(&self) -> PyResult<String> {
        self.core.to_rtl().map_err(core_err)
    }
}

#[pyclass(name = "ProviderSpec", frozen)]
#[derive(Clone)]
pub struct PyProviderSpec {
    pub core: sp::ProviderSpec,
}

fn make_cell_provider(
    kind: sp::CellKind,
    cardinality: i64,
    order: Option<sp::TraversalOrder>,
    condition: PyFilterCond,
) -> PyResult<PyProviderSpec> {
    Ok(PyProviderSpec {
        core: sp::ProviderSpec::cell(
            kind,
            cardinality,
            order.unwrap_or(sp::TraversalOrder::RowMajor),
            condition.core,
        )
        .map_err(core_err)?,
    })
}

#[pymethods]
impl PyProviderSpec {
    #[classattr]
    #[allow(non_snake_case)]
    fn UNBOUNDED() -> i64 {
        sp::UNBOUNDED
    }
    #[staticmethod]
    #[pyo3(signature = (condition, cardinality=1, traversal_order=None))]
    fn any(
        condition: PyFilterCond,
        cardinality: i64,
        traversal_order: Option<sp::TraversalOrder>,
    ) -> PyResult<Self> {
        make_cell_provider(sp::CellKind::Unrestricted, cardinality, traversal_order, condition)
    }
    #[staticmethod]
    #[pyo3(signature = (condition, cardinality=1, traversal_order=None))]
    fn val(
        condition: PyFilterCond,
        cardinality: i64,
        traversal_order: Option<sp::TraversalOrder>,
    ) -> PyResult<Self> {
        make_cell_provider(sp::CellKind::Val, cardinality, traversal_order, condition)
    }
    #[staticmethod]
    #[pyo3(signature = (condition, traversal_order=None))]
    fn attr(condition: PyFilterCond, traversal_order: Option<sp::TraversalOrder>) -> PyResult<Self> {
        make_cell_provider(sp::CellKind::Attr, 1, traversal_order, condition)
    }
    #[staticmethod]
    #[pyo3(signature = (condition, cardinality=1, traversal_order=None))]
    fn aux(
        condition: PyFilterCond,
        cardinality: i64,
        traversal_order: Option<sp::TraversalOrder>,
    ) -> PyResult<Self> {
        make_cell_provider(sp::CellKind::Aux, cardinality, traversal_order, condition)
    }
    #[staticmethod]
    fn ctx(text: String, ty: sp::ItemType) -> Self {
        PyProviderSpec { core: sp::ProviderSpec::ctx(text, ty) }
    }
    #[staticmethod]
    fn ctx_attr(text: String) -> Self {
        PyProviderSpec { core: sp::ProviderSpec::ctx(text, sp::ItemType::Attribute) }
    }
    #[staticmethod]
    fn ctx_val(text: String) -> Self {
        PyProviderSpec { core: sp::ProviderSpec::ctx(text, sp::ItemType::Value) }
    }
    #[staticmethod]
    fn ctx_aux(text: String) -> Self {
        PyProviderSpec { core: sp::ProviderSpec::ctx(text, sp::ItemType::Auxiliary) }
    }
    #[staticmethod]
    fn ctx_avp(attr_name: String, value: String) -> Self {
        PyProviderSpec { core: sp::ProviderSpec::ctx_avp(attr_name, value) }
    }
    #[getter]
    fn cardinality(&self) -> i64 {
        self.core.cardinality
    }
    #[getter]
    fn traversal_order(&self) -> sp::TraversalOrder {
        self.core.traversal_order
    }
    fn is_context_literal(&self) -> bool {
        self.core.is_context_literal()
    }
}

#[pyclass(name = "ActionSpec", frozen)]
#[derive(Clone)]
pub struct PyActionSpec {
    pub core: sp::ActionSpec,
}

/// Accepts ProviderSpec or ItemFilterConditionSpec (converted to a VAL
/// provider with the given cardinality, mirroring `ActionSpec.rec(int, ...)`).
fn providers_from_args(
    args: &Bound<'_, PyTuple>,
    cond_cardinality: i64,
) -> PyResult<Vec<sp::ProviderSpec>> {
    let mut out = Vec::new();
    for a in args.iter() {
        if let Ok(p) = a.extract::<PyProviderSpec>() {
            out.push(p.core);
        } else if let Ok(c) = a.extract::<PyFilterCond>() {
            out.push(
                sp::ProviderSpec::cell(
                    sp::CellKind::Val,
                    cond_cardinality,
                    sp::TraversalOrder::RowMajor,
                    c.core,
                )
                .map_err(core_err)?,
            );
        } else {
            return Err(PyValueError::new_err(
                "expected ProviderSpec or ItemFilterConditionSpec",
            ));
        }
    }
    Ok(out)
}

#[pymethods]
impl PyActionSpec {
    #[new]
    #[pyo3(signature = (operation_type, delimiter=None, providers=vec![], anchor_pos=None, split_delimiter=None, key_positions=None, inherited=false))]
    fn new(
        operation_type: sp::OperationType,
        delimiter: Option<String>,
        providers: Vec<PyProviderSpec>,
        anchor_pos: Option<i64>,
        split_delimiter: Option<String>,
        key_positions: Option<BTreeSet<i64>>,
        inherited: bool,
    ) -> PyResult<Self> {
        Ok(PyActionSpec {
            core: sp::ActionSpec::new(
                operation_type,
                delimiter,
                providers.into_iter().map(|p| p.core).collect(),
                anchor_pos,
                split_delimiter,
                key_positions.unwrap_or_default(),
                inherited,
            )
            .map_err(core_err)?,
        })
    }

    #[staticmethod]
    #[pyo3(signature = (*providers, anchor_pos=None, split_delimiter=None, cardinality=1))]
    fn rec(
        providers: &Bound<'_, PyTuple>,
        anchor_pos: Option<i64>,
        split_delimiter: Option<String>,
        cardinality: i64,
    ) -> PyResult<Self> {
        Ok(PyActionSpec {
            core: sp::ActionSpec::new(
                sp::OperationType::Rec,
                None,
                providers_from_args(providers, cardinality)?,
                anchor_pos,
                split_delimiter,
                BTreeSet::new(),
                false,
            )
            .map_err(core_err)?,
        })
    }

    #[staticmethod]
    fn avp(provider: &Bound<'_, PyAny>) -> PyResult<Self> {
        let p = if let Ok(p) = provider.extract::<PyProviderSpec>() {
            p.core
        } else if let Ok(literal) = provider.extract::<String>() {
            sp::ProviderSpec::ctx(literal, sp::ItemType::Attribute)
        } else if let Ok(c) = provider.extract::<PyFilterCond>() {
            sp::ProviderSpec::cell(
                sp::CellKind::Attr,
                1,
                sp::TraversalOrder::RowMajor,
                c.core,
            )
            .map_err(core_err)?
        } else {
            return Err(PyValueError::new_err(
                "expected ProviderSpec, ItemFilterConditionSpec or str",
            ));
        };
        Ok(PyActionSpec {
            core: sp::ActionSpec::new(
                sp::OperationType::Avp,
                None,
                vec![p],
                None,
                None,
                BTreeSet::new(),
                false,
            )
            .map_err(core_err)?,
        })
    }

    #[staticmethod]
    #[pyo3(signature = (*providers, key_positions=None, cardinality=1))]
    fn join(
        providers: &Bound<'_, PyTuple>,
        key_positions: Option<&Bound<'_, PyAny>>,
        cardinality: i64,
    ) -> PyResult<Self> {
        let keys: BTreeSet<i64> = match key_positions {
            None => BTreeSet::new(),
            Some(kp) => {
                if let Ok(one) = kp.extract::<i64>() {
                    BTreeSet::from([one])
                } else {
                    kp.extract::<BTreeSet<i64>>()?
                }
            }
        };
        Ok(PyActionSpec {
            core: sp::ActionSpec::new(
                sp::OperationType::Join,
                None,
                providers_from_args(providers, cardinality)?,
                None,
                None,
                keys,
                false,
            )
            .map_err(core_err)?,
        })
    }

    #[staticmethod]
    #[pyo3(signature = (delimiter, *providers, cardinality=1))]
    fn fill(
        delimiter: String,
        providers: &Bound<'_, PyTuple>,
        cardinality: i64,
    ) -> PyResult<Self> {
        Self::str_op(sp::OperationType::Fill, delimiter, providers, cardinality)
    }
    #[staticmethod]
    #[pyo3(signature = (delimiter, *providers, cardinality=1))]
    fn prefix(
        delimiter: String,
        providers: &Bound<'_, PyTuple>,
        cardinality: i64,
    ) -> PyResult<Self> {
        Self::str_op(sp::OperationType::Prefix, delimiter, providers, cardinality)
    }
    #[staticmethod]
    #[pyo3(signature = (delimiter, *providers, cardinality=1))]
    fn suffix(
        delimiter: String,
        providers: &Bound<'_, PyTuple>,
        cardinality: i64,
    ) -> PyResult<Self> {
        Self::str_op(sp::OperationType::Suffix, delimiter, providers, cardinality)
    }

    #[getter]
    fn operation_type(&self) -> sp::OperationType {
        self.core.operation_type
    }
    #[getter]
    fn inherited(&self) -> bool {
        self.core.inherited
    }
    fn as_inherited(&self) -> Self {
        PyActionSpec { core: self.core.as_inherited() }
    }
}

impl PyActionSpec {
    fn str_op(
        op: sp::OperationType,
        delimiter: String,
        providers: &Bound<'_, PyTuple>,
        cardinality: i64,
    ) -> PyResult<Self> {
        // string-op providers may be unrestricted conditions
        let mut out = Vec::new();
        for a in providers.iter() {
            if let Ok(p) = a.extract::<PyProviderSpec>() {
                out.push(p.core);
            } else if let Ok(c) = a.extract::<PyFilterCond>() {
                out.push(
                    sp::ProviderSpec::cell(
                        sp::CellKind::Unrestricted,
                        cardinality,
                        sp::TraversalOrder::RowMajor,
                        c.core,
                    )
                    .map_err(core_err)?,
                );
            } else {
                return Err(PyValueError::new_err(
                    "expected ProviderSpec or ItemFilterConditionSpec",
                ));
            }
        }
        Ok(PyActionSpec {
            core: sp::ActionSpec::new(op, Some(delimiter), out, None, None, BTreeSet::new(), false)
                .map_err(core_err)?,
        })
    }
}

// ---------------------------------------------------------------- content specs

#[pyclass(name = "AtomicContentSpec", frozen)]
#[derive(Clone)]
pub struct PyAtomicSpec {
    pub core: sp::AtomicSpec,
}

fn actions_from_args(args: &Bound<'_, PyTuple>) -> PyResult<Vec<sp::ActionSpec>> {
    let mut out = Vec::new();
    for a in args.iter() {
        out.push(a.extract::<PyActionSpec>()?.core);
    }
    Ok(out)
}

fn atomic(idd: sp::Idd, extractor: Option<PyExtractor>, tags: Vec<String>, actions: Vec<sp::ActionSpec>) -> PyAtomicSpec {
    PyAtomicSpec {
        core: sp::AtomicSpec {
            idd,
            extractor: extractor.map(|x| x.core),
            tags,
            actions,
        },
    }
}

#[pymethods]
impl PyAtomicSpec {
    #[new]
    #[pyo3(signature = (idd, extractor=None, tags=vec![], actions=vec![]))]
    fn new(
        idd: sp::Idd,
        extractor: Option<PyExtractor>,
        tags: Vec<String>,
        actions: Vec<PyActionSpec>,
    ) -> Self {
        atomic(idd, extractor, tags, actions.into_iter().map(|a| a.core).collect())
    }
    #[staticmethod]
    #[pyo3(signature = (*actions, extractor=None))]
    fn val(actions: &Bound<'_, PyTuple>, extractor: Option<PyExtractor>) -> PyResult<Self> {
        Ok(atomic(sp::Idd::Val, extractor, vec![], actions_from_args(actions)?))
    }
    #[staticmethod]
    #[pyo3(signature = (*actions, extractor=None))]
    fn attr(actions: &Bound<'_, PyTuple>, extractor: Option<PyExtractor>) -> PyResult<Self> {
        Ok(atomic(sp::Idd::Attr, extractor, vec![], actions_from_args(actions)?))
    }
    #[staticmethod]
    #[pyo3(signature = (*actions, extractor=None))]
    fn aux(actions: &Bound<'_, PyTuple>, extractor: Option<PyExtractor>) -> PyResult<Self> {
        Ok(atomic(sp::Idd::Aux, extractor, vec![], actions_from_args(actions)?))
    }
    #[staticmethod]
    fn skip() -> Self {
        atomic(sp::Idd::Skip, None, vec![], vec![])
    }
    #[staticmethod]
    #[pyo3(signature = (tag, *actions))]
    fn val_tagged(tag: &Bound<'_, PyAny>, actions: &Bound<'_, PyTuple>) -> PyResult<Self> {
        let tags: Vec<String> = if let Ok(one) = tag.extract::<String>() {
            vec![one]
        } else {
            tag.extract::<Vec<String>>()?
        };
        Ok(atomic(sp::Idd::Val, None, tags, actions_from_args(actions)?))
    }
    /// Copy with tags appended (stored with the leading `#`).
    #[pyo3(signature = (*new_tags))]
    fn tagged(&self, new_tags: &Bound<'_, PyTuple>) -> PyResult<Self> {
        let mut core = self.core.clone();
        for t in new_tags.iter() {
            core.tags.push(format!("#{}", t.extract::<String>()?));
        }
        Ok(PyAtomicSpec { core })
    }
    /// Copy with the given string extractor.
    fn extract(&self, extractor: PyExtractor) -> Self {
        let mut core = self.core.clone();
        core.extractor = Some(extractor.core);
        PyAtomicSpec { core }
    }
    /// Wraps into a delimited spec (RTL `(atom){"delim"}`).
    fn split_by(&self, delimiter: String) -> PyResult<PyDelimitedSpec> {
        Ok(PyDelimitedSpec {
            core: sp::DelimitedSpec::new(delimiter, self.core.clone()).map_err(core_err)?,
        })
    }
    /// Chains with the next segment into a compound spec.
    fn then(&self, py: Python<'_>, delimiter: String, next: &Bound<'_, PyAny>) -> PyResult<PyCompoundSpec> {
        let _ = py;
        let first = sp::CompoundSegment::new(String::new(), sp::ContentSpec::Atomic(self.core.clone()))
            .map_err(core_err)?;
        let second =
            sp::CompoundSegment::new(delimiter, extract_content_spec(next)?).map_err(core_err)?;
        Ok(PyCompoundSpec {
            core: sp::CompoundSpec::new(vec![first, second], String::new()).map_err(core_err)?,
        })
    }
    #[getter]
    fn idd(&self) -> sp::Idd {
        self.core.idd
    }
    #[getter]
    fn tags(&self) -> Vec<String> {
        self.core.tags.clone()
    }
}

#[pyclass(name = "DelimitedContentSpec", frozen)]
#[derive(Clone)]
pub struct PyDelimitedSpec {
    pub core: sp::DelimitedSpec,
}

#[pymethods]
impl PyDelimitedSpec {
    #[new]
    fn new(delimiter: String, atomic_spec: PyAtomicSpec) -> PyResult<Self> {
        Ok(PyDelimitedSpec {
            core: sp::DelimitedSpec::new(delimiter, atomic_spec.core).map_err(core_err)?,
        })
    }
    #[getter]
    fn delimiter(&self) -> String {
        self.core.delimiter.clone()
    }
    #[getter]
    fn atomic_spec(&self) -> PyAtomicSpec {
        PyAtomicSpec { core: self.core.atom.clone() }
    }
    fn then(&self, delimiter: String, next: &Bound<'_, PyAny>) -> PyResult<PyCompoundSpec> {
        let first =
            sp::CompoundSegment::new(String::new(), sp::ContentSpec::Delimited(self.core.clone()))
                .map_err(core_err)?;
        let second =
            sp::CompoundSegment::new(delimiter, extract_content_spec(next)?).map_err(core_err)?;
        Ok(PyCompoundSpec {
            core: sp::CompoundSpec::new(vec![first, second], String::new()).map_err(core_err)?,
        })
    }
}

#[pyclass(name = "CompoundContentSpec", frozen)]
#[derive(Clone)]
pub struct PyCompoundSpec {
    pub core: sp::CompoundSpec,
}

fn extract_content_spec(obj: &Bound<'_, PyAny>) -> PyResult<sp::ContentSpec> {
    if let Ok(a) = obj.extract::<PyAtomicSpec>() {
        return Ok(sp::ContentSpec::Atomic(a.core));
    }
    if let Ok(d) = obj.extract::<PyDelimitedSpec>() {
        return Ok(sp::ContentSpec::Delimited(d.core));
    }
    if let Ok(c) = obj.extract::<PyCompoundSpec>() {
        return Ok(sp::ContentSpec::Compound(c.core));
    }
    if let Ok(c) = obj.extract::<PyConditionalSpec>() {
        return Ok(sp::ContentSpec::Conditional(Box::new(c.core)));
    }
    Err(PyValueError::new_err("expected a content specification"))
}

fn content_spec_to_py(py: Python<'_>, cs: &sp::ContentSpec) -> PyResult<PyObject> {
    Ok(match cs {
        sp::ContentSpec::Atomic(a) => {
            PyAtomicSpec { core: a.clone() }.into_pyobject(py)?.into_any().unbind()
        }
        sp::ContentSpec::Delimited(d) => {
            PyDelimitedSpec { core: d.clone() }.into_pyobject(py)?.into_any().unbind()
        }
        sp::ContentSpec::Compound(c) => {
            PyCompoundSpec { core: c.clone() }.into_pyobject(py)?.into_any().unbind()
        }
        sp::ContentSpec::Conditional(c) => PyConditionalSpec { core: (**c).clone() }
            .into_pyobject(py)?
            .into_any()
            .unbind(),
    })
}

#[pymethods]
impl PyCompoundSpec {
    #[new]
    #[pyo3(signature = (segments, trailing_delimiter=String::new()))]
    fn new(segments: Vec<(String, Bound<'_, PyAny>)>, trailing_delimiter: String) -> PyResult<Self> {
        let mut segs = Vec::new();
        for (d, s) in segments {
            segs.push(sp::CompoundSegment::new(d, extract_content_spec(&s)?).map_err(core_err)?);
        }
        Ok(PyCompoundSpec {
            core: sp::CompoundSpec::new(segs, trailing_delimiter).map_err(core_err)?,
        })
    }
    /// `CompoundContentSpec.of(first, (delim, spec), ...)`
    #[staticmethod]
    #[pyo3(signature = (first, *rest))]
    fn of(first: &Bound<'_, PyAny>, rest: &Bound<'_, PyTuple>) -> PyResult<Self> {
        let mut segs = vec![
            sp::CompoundSegment::new(String::new(), extract_content_spec(first)?)
                .map_err(core_err)?,
        ];
        for item in rest.iter() {
            let (d, s): (String, Bound<'_, PyAny>) = item.extract()?;
            segs.push(sp::CompoundSegment::new(d, extract_content_spec(&s)?).map_err(core_err)?);
        }
        Ok(PyCompoundSpec {
            core: sp::CompoundSpec::new(segs, String::new()).map_err(core_err)?,
        })
    }
    fn then(&self, delimiter: String, next: &Bound<'_, PyAny>) -> PyResult<PyCompoundSpec> {
        let mut core = self.core.clone();
        core.segments
            .push(sp::CompoundSegment::new(delimiter, extract_content_spec(next)?).map_err(core_err)?);
        Ok(PyCompoundSpec { core })
    }
    #[getter]
    fn trailing_delimiter(&self) -> String {
        self.core.trailing_delimiter.clone()
    }
}

#[pyclass(name = "ConditionalContentSpec", frozen)]
#[derive(Clone)]
pub struct PyConditionalSpec {
    pub core: sp::ConditionalSpec,
}

#[pymethods]
impl PyConditionalSpec {
    #[new]
    fn new(
        condition: &Bound<'_, PyAny>,
        positive: &Bound<'_, PyAny>,
        negative: &Bound<'_, PyAny>,
    ) -> PyResult<Self> {
        Ok(PyConditionalSpec {
            core: sp::ConditionalSpec {
                condition: extract_cell_predicate(condition)?,
                positive: extract_content_spec(positive)?,
                negative: extract_content_spec(negative)?,
            },
        })
    }
}

fn extract_cell_predicate(obj: &Bound<'_, PyAny>) -> PyResult<sp::CellPredicate> {
    if let Ok(c) = obj.extract::<PyCellMatchCondition>() {
        return Ok(c.core);
    }
    if let Ok(p) = obj.extract::<PyCellPredicate>() {
        return Ok(p.core);
    }
    Err(PyValueError::new_err(
        "expected CellMatchCondition or CellPredicate",
    ))
}

// ---------------------------------------------------------------- patterns

#[pyclass(name = "CellPattern", frozen)]
#[derive(Clone)]
pub struct PyCellPattern {
    pub core: Arc<sp::CellPattern>,
}

/// Parses leading (condition)? (quantifier)? prefix of an `of(...)` call.
struct OfPrefix {
    cond: Option<sp::CellPredicate>,
    quant: sp::Quantifier,
    rest: Vec<PyObject>,
}

fn parse_of_prefix(py: Python<'_>, args: &Bound<'_, PyTuple>) -> PyResult<OfPrefix> {
    let mut cond = None;
    let mut quant = sp::Quantifier::ONE;
    let mut rest = Vec::new();
    let mut i = 0;
    let items: Vec<Bound<'_, PyAny>> = args.iter().collect();
    if i < items.len() {
        if let Ok(c) = items[i].extract::<PyCellMatchCondition>() {
            cond = Some(c.core);
            i += 1;
        } else if let Ok(c) = items[i].extract::<PyCellPredicate>() {
            cond = Some(c.core);
            i += 1;
        }
    }
    if i < items.len() {
        if let Ok(q) = items[i].extract::<PyQuantifier>() {
            quant = q.core;
            i += 1;
        }
    }
    for item in &items[i..] {
        rest.push(item.clone().unbind());
    }
    let _ = py;
    Ok(OfPrefix { cond, quant, rest })
}

#[pymethods]
impl PyCellPattern {
    #[new]
    #[pyo3(signature = (condition=None, quantifier=None, content_spec=None))]
    fn new(
        condition: Option<&Bound<'_, PyAny>>,
        quantifier: Option<PyQuantifier>,
        content_spec: Option<&Bound<'_, PyAny>>,
    ) -> PyResult<Self> {
        Ok(PyCellPattern {
            core: Arc::new(sp::CellPattern {
                condition: condition.map(extract_cell_predicate).transpose()?,
                quantifier: quantifier.map(|q| q.core).unwrap_or(sp::Quantifier::ONE),
                content_spec: content_spec.map(extract_content_spec).transpose()?,
            }),
        })
    }

    /// `of(cs)`, `of(q, cs)`, `of(cond, q, cs)`
    #[staticmethod]
    #[pyo3(signature = (*args))]
    fn of(py: Python<'_>, args: &Bound<'_, PyTuple>) -> PyResult<Self> {
        let p = parse_of_prefix(py, args)?;
        if p.rest.len() != 1 {
            return Err(PyValueError::new_err("CellPattern.of expects one content spec"));
        }
        let first = p.rest[0].bind(py);
        let cs = if first.is_none() {
            None
        } else {
            Some(extract_content_spec(first)?)
        };
        Ok(PyCellPattern {
            core: Arc::new(sp::CellPattern {
                condition: p.cond,
                quantifier: p.quant,
                content_spec: cs,
            }),
        })
    }

    #[staticmethod]
    #[pyo3(signature = (quantifier=None))]
    fn skip(quantifier: Option<PyQuantifier>) -> Self {
        PyCellPattern {
            core: Arc::new(sp::CellPattern {
                condition: None,
                quantifier: quantifier.map(|q| q.core).unwrap_or(sp::Quantifier::ONE),
                content_spec: None,
            }),
        }
    }

    fn one_or_more(&self) -> Self {
        self.with_quant(sp::Quantifier::ONE_OR_MORE)
    }
    fn zero_or_more(&self) -> Self {
        self.with_quant(sp::Quantifier::ZERO_OR_MORE)
    }
    fn zero_or_one(&self) -> Self {
        self.with_quant(sp::Quantifier::ZERO_OR_ONE)
    }
    fn exactly(&self, n: i64) -> PyResult<Self> {
        Ok(self.with_quant(sp::Quantifier::exactly(n).map_err(core_err)?))
    }
    #[getter]
    fn quantifier(&self) -> PyQuantifier {
        PyQuantifier { core: self.core.quantifier }
    }
    #[getter]
    fn content_spec(&self, py: Python<'_>) -> PyResult<Option<PyObject>> {
        self.core
            .content_spec
            .as_ref()
            .map(|cs| content_spec_to_py(py, cs))
            .transpose()
    }
}

impl PyCellPattern {
    fn with_quant(&self, q: sp::Quantifier) -> Self {
        let mut c = (*self.core).clone();
        c.quantifier = q;
        PyCellPattern { core: Arc::new(c) }
    }
}

#[pyclass(name = "SubrowPattern", frozen)]
#[derive(Clone)]
pub struct PySubrowPattern {
    pub core: Arc<sp::SubrowPattern>,
}

#[pymethods]
impl PySubrowPattern {
    /// `of(cells...)`, `of(q, cells...)`
    #[staticmethod]
    #[pyo3(signature = (*args))]
    fn of(py: Python<'_>, args: &Bound<'_, PyTuple>) -> PyResult<Self> {
        let p = parse_of_prefix(py, args)?;
        let mut cells = Vec::new();
        for item in &p.rest {
            cells.push(item.bind(py).extract::<PyCellPattern>()?.core);
        }
        Ok(PySubrowPattern {
            core: Arc::new(
                sp::SubrowPattern::new(p.cond, p.quant, cells).map_err(core_err)?,
            ),
        })
    }
    fn one_or_more(&self) -> Self {
        self.with_quant(sp::Quantifier::ONE_OR_MORE)
    }
    fn zero_or_more(&self) -> Self {
        self.with_quant(sp::Quantifier::ZERO_OR_MORE)
    }
    fn zero_or_one(&self) -> Self {
        self.with_quant(sp::Quantifier::ZERO_OR_ONE)
    }
    fn exactly(&self, n: i64) -> PyResult<Self> {
        Ok(self.with_quant(sp::Quantifier::exactly(n).map_err(core_err)?))
    }
    #[getter]
    fn quantifier(&self) -> PyQuantifier {
        PyQuantifier { core: self.core.quantifier }
    }
}

impl PySubrowPattern {
    fn with_quant(&self, q: sp::Quantifier) -> Self {
        let mut c = (*self.core).clone();
        c.quantifier = q;
        PySubrowPattern { core: Arc::new(c) }
    }
}

#[pyclass(name = "RowPattern", frozen)]
#[derive(Clone)]
pub struct PyRowPattern {
    pub core: Arc<sp::RowPattern>,
}

#[pymethods]
impl PyRowPattern {
    /// `of(cells...)`, `of(q, cells...)`, `of(q, subrows...)`, `of(cond, q, cells...)`
    #[staticmethod]
    #[pyo3(signature = (*args))]
    fn of(py: Python<'_>, args: &Bound<'_, PyTuple>) -> PyResult<Self> {
        let p = parse_of_prefix(py, args)?;
        let mut subrows: Vec<Arc<sp::SubrowPattern>> = Vec::new();
        let mut cells: Vec<Arc<sp::CellPattern>> = Vec::new();
        for item in &p.rest {
            let b = item.bind(py);
            if let Ok(sr) = b.extract::<PySubrowPattern>() {
                subrows.push(sr.core);
            } else if let Ok(c) = b.extract::<PyCellPattern>() {
                cells.push(c.core);
            } else {
                return Err(PyValueError::new_err(
                    "expected CellPattern or SubrowPattern",
                ));
            }
        }
        if !cells.is_empty() && !subrows.is_empty() {
            return Err(PyValueError::new_err(
                "cannot mix CellPattern and SubrowPattern in RowPattern.of",
            ));
        }
        if subrows.is_empty() {
            subrows.push(Arc::new(
                sp::SubrowPattern::new(None, sp::Quantifier::ONE, cells).map_err(core_err)?,
            ));
        }
        Ok(PyRowPattern {
            core: Arc::new(sp::RowPattern::new(p.cond, p.quant, subrows).map_err(core_err)?),
        })
    }
    fn one_or_more(&self) -> Self {
        self.with_quant(sp::Quantifier::ONE_OR_MORE)
    }
    fn zero_or_more(&self) -> Self {
        self.with_quant(sp::Quantifier::ZERO_OR_MORE)
    }
    fn zero_or_one(&self) -> Self {
        self.with_quant(sp::Quantifier::ZERO_OR_ONE)
    }
    fn exactly(&self, n: i64) -> PyResult<Self> {
        Ok(self.with_quant(sp::Quantifier::exactly(n).map_err(core_err)?))
    }
    #[getter]
    fn quantifier(&self) -> PyQuantifier {
        PyQuantifier { core: self.core.quantifier }
    }
}

impl PyRowPattern {
    fn with_quant(&self, q: sp::Quantifier) -> Self {
        let mut c = (*self.core).clone();
        c.quantifier = q;
        PyRowPattern { core: Arc::new(c) }
    }
}

#[pyclass(name = "SubtablePattern", frozen)]
#[derive(Clone)]
pub struct PySubtablePattern {
    pub core: Arc<sp::SubtablePattern>,
}

#[pymethods]
impl PySubtablePattern {
    /// `of(rows...)`, `of(q, rows...)`
    #[staticmethod]
    #[pyo3(signature = (*args))]
    fn of(py: Python<'_>, args: &Bound<'_, PyTuple>) -> PyResult<Self> {
        let p = parse_of_prefix(py, args)?;
        let mut rows = Vec::new();
        for item in &p.rest {
            rows.push(item.bind(py).extract::<PyRowPattern>()?.core);
        }
        Ok(PySubtablePattern {
            core: Arc::new(sp::SubtablePattern::new(p.cond, p.quant, rows).map_err(core_err)?),
        })
    }
    fn one_or_more(&self) -> Self {
        self.with_quant(sp::Quantifier::ONE_OR_MORE)
    }
    fn zero_or_more(&self) -> Self {
        self.with_quant(sp::Quantifier::ZERO_OR_MORE)
    }
    fn zero_or_one(&self) -> Self {
        self.with_quant(sp::Quantifier::ZERO_OR_ONE)
    }
    fn exactly(&self, n: i64) -> PyResult<Self> {
        Ok(self.with_quant(sp::Quantifier::exactly(n).map_err(core_err)?))
    }
    #[getter]
    fn quantifier(&self) -> PyQuantifier {
        PyQuantifier { core: self.core.quantifier }
    }
}

impl PySubtablePattern {
    fn with_quant(&self, q: sp::Quantifier) -> Self {
        let mut c = (*self.core).clone();
        c.quantifier = q;
        PySubtablePattern { core: Arc::new(c) }
    }
}

#[pyclass(name = "TablePattern", frozen)]
#[derive(Clone)]
pub struct PyTablePattern {
    pub core: sp::TablePattern,
}

fn extract_transformation(obj: &Bound<'_, PyAny>) -> PyResult<sp::Transformation> {
    if let Ok(t) = obj.extract::<PyWhitespaceNormalization>() {
        let _ = t;
        return Ok(sp::Transformation::WhitespaceNormalization);
    }
    if let Ok(t) = obj.extract::<PyAnchorAttributeAtPosition>() {
        return Ok(t.core);
    }
    if let Ok(t) = obj.extract::<PyDelimitedFieldSplit>() {
        return Ok(t.core);
    }
    if let Ok(t) = obj.extract::<PyFieldSplitting>() {
        return Ok(t.core);
    }
    if let Ok(t) = obj.extract::<PySchemaReordering>() {
        return Ok(t.core);
    }
    Err(PyValueError::new_err("expected a RecordsetTransformation"))
}

#[pymethods]
impl PyTablePattern {
    #[staticmethod]
    #[pyo3(signature = (*subtables))]
    fn of(subtables: &Bound<'_, PyTuple>) -> PyResult<Self> {
        let mut subs = Vec::new();
        for s in subtables.iter() {
            subs.push(s.extract::<PySubtablePattern>()?.core);
        }
        Ok(PyTablePattern {
            core: sp::TablePattern::of(subs).map_err(core_err)?,
        })
    }
    #[pyo3(signature = (*transforms))]
    fn with_transformations(&self, transforms: &Bound<'_, PyTuple>) -> PyResult<Self> {
        let mut out = Vec::new();
        for t in transforms.iter() {
            out.push(extract_transformation(&t)?);
        }
        Ok(PyTablePattern {
            core: sp::TablePattern {
                condition: self.core.condition.clone(),
                subtable_patterns: self.core.subtable_patterns.clone(),
                transformations: out,
            },
        })
    }
    /// Applies all transformations in order to the given recordset.
    fn transform(&self, rs: &PyRecordset) -> PyResult<PyRecordset> {
        Ok(PyRecordset {
            core: self.core.transform(rs.core.clone()).map_err(core_err)?,
        })
    }
    fn __eq__(&self, other: &PyTablePattern) -> bool {
        self.core == other.core
    }
    fn __repr__(&self) -> String {
        match rtl::serialize::serialize(&self.core) {
            Ok(s) => format!("TablePattern<{s}>"),
            Err(_) => "TablePattern<unserializable>".to_string(),
        }
    }
}

// ---------------------------------------------------------------- transformations

#[pyclass(name = "WhitespaceNormalization", frozen)]
#[derive(Clone)]
pub struct PyWhitespaceNormalization;

#[pymethods]
impl PyWhitespaceNormalization {
    #[new]
    fn new() -> Self {
        PyWhitespaceNormalization
    }
    fn apply(&self, rs: &PyRecordset) -> PyResult<PyRecordset> {
        Ok(PyRecordset {
            core: sp::Transformation::WhitespaceNormalization
                .apply(rs.core.clone())
                .map_err(core_err)?,
        })
    }
}

#[pyclass(name = "AnchorAttributeAtPosition", frozen)]
#[derive(Clone)]
pub struct PyAnchorAttributeAtPosition {
    pub core: sp::Transformation,
}

#[pymethods]
impl PyAnchorAttributeAtPosition {
    #[new]
    fn new(position: i64) -> PyResult<Self> {
        if position < 0 {
            return Err(PyValueError::new_err(format!(
                "position must be non-negative: {position}"
            )));
        }
        Ok(PyAnchorAttributeAtPosition {
            core: sp::Transformation::AnchorAttributeAtPosition(position),
        })
    }
    fn apply(&self, rs: &PyRecordset) -> PyResult<PyRecordset> {
        Ok(PyRecordset {
            core: self.core.apply(rs.core.clone()).map_err(core_err)?,
        })
    }
}

#[pyclass(name = "DelimitedFieldSplit", frozen)]
#[derive(Clone)]
pub struct PyDelimitedFieldSplit {
    pub core: sp::Transformation,
}

#[pymethods]
impl PyDelimitedFieldSplit {
    #[new]
    #[pyo3(signature = (delimiter, only_attributes=None, anonymous_attribute_template=String::from("$a_%i")))]
    fn new(
        delimiter: String,
        only_attributes: Option<BTreeSet<String>>,
        anonymous_attribute_template: String,
    ) -> PyResult<Self> {
        if delimiter.is_empty() {
            return Err(PyValueError::new_err("delimiter must be non-empty"));
        }
        if !anonymous_attribute_template.contains("%i") {
            return Err(PyValueError::new_err(
                "Anonymous attribute template must contain the placeholder %i",
            ));
        }
        Ok(PyDelimitedFieldSplit {
            core: sp::Transformation::DelimitedFieldSplit {
                delimiter,
                only_attributes: only_attributes.filter(|s| !s.is_empty()),
                template: anonymous_attribute_template,
            },
        })
    }
    fn apply(&self, rs: &PyRecordset) -> PyResult<PyRecordset> {
        Ok(PyRecordset {
            core: self.core.apply(rs.core.clone()).map_err(core_err)?,
        })
    }
}

#[pyclass(name = "FieldSplitting", frozen)]
#[derive(Clone)]
pub struct PyFieldSplitting {
    pub core: sp::Transformation,
}

#[pymethods]
impl PyFieldSplitting {
    #[new]
    #[pyo3(signature = (attribute, delimiter, part_attribute_names=vec![]))]
    fn new(attribute: String, delimiter: String, part_attribute_names: Vec<String>) -> Self {
        PyFieldSplitting {
            core: sp::Transformation::FieldSplitting {
                attribute,
                delimiter,
                part_attribute_names,
            },
        }
    }
    fn apply(&self, rs: &PyRecordset) -> PyResult<PyRecordset> {
        Ok(PyRecordset {
            core: self.core.apply(rs.core.clone()).map_err(core_err)?,
        })
    }
}

#[pyclass(name = "SchemaReordering", frozen)]
#[derive(Clone)]
pub struct PySchemaReordering {
    pub core: sp::Transformation,
}

#[pymethods]
impl PySchemaReordering {
    #[new]
    fn new(order: Vec<String>) -> Self {
        PySchemaReordering {
            core: sp::Transformation::SchemaReordering(order),
        }
    }
    fn apply(&self, rs: &PyRecordset) -> PyResult<PyRecordset> {
        Ok(PyRecordset {
            core: self.core.apply(rs.core.clone()).map_err(core_err)?,
        })
    }
}

// ---------------------------------------------------------------- InterpretableTable / matcher / interpreter

#[pyclass(name = "TableSemantics")]
pub struct PyTableSemantics {
    pub sem: Arc<SemanticsCore>,
    pub table: Py<PyTableSyntax>,
}

#[pymethods]
impl PyTableSemantics {
    fn cell_derived_items(&self, py: Python<'_>) -> Vec<PyCellDerivedItem> {
        self.sem
            .cell_items
            .iter()
            .map(|it| PyCellDerivedItem {
                s: it.s.clone(),
                tags: it.tags.clone(),
                index: it.index,
                ty: it.ty,
                row: it.row,
                col: it.col,
                table: Some(self.table.clone_ref(py)),
            })
            .collect()
    }
    fn context_derived_items(&self) -> Vec<PyContextDerivedItem> {
        self.sem
            .ctx_items
            .iter()
            .map(|it| PyContextDerivedItem { core: it.clone() })
            .collect()
    }
    fn action_count(&self) -> usize {
        self.sem.actions.len()
    }
}

#[pyclass(name = "InterpretableTable")]
pub struct PyInterpretableTable {
    pub table: Py<PyTableSyntax>,
    pub sem: Arc<SemanticsCore>,
}

#[pymethods]
impl PyInterpretableTable {
    #[getter]
    fn syntax(&self, py: Python<'_>) -> Py<PyTableSyntax> {
        self.table.clone_ref(py)
    }
    #[getter]
    fn semantics(&self, py: Python<'_>) -> PyTableSemantics {
        PyTableSemantics {
            sem: self.sem.clone(),
            table: self.table.clone_ref(py),
        }
    }
}

#[pyclass(name = "AtpMatcher")]
pub struct PyAtpMatcher;

#[pymethods]
impl PyAtpMatcher {
    /// `AtpMatcher.match(pattern, syntax, context_items=None)` →
    /// `InterpretableTable | None`.
    #[staticmethod]
    #[pyo3(name = "match", signature = (atp, syntax, context_items=None))]
    fn match_(
        py: Python<'_>,
        atp: &PyTablePattern,
        syntax: &Bound<'_, PyTableSyntax>,
        context_items: Option<Vec<PyContextDerivedItem>>,
    ) -> PyResult<Option<PyInterpretableTable>> {
        let table_any: Py<PyAny> = syntax.clone().unbind().into_any();
        // Phase 1: syntactic matching (immutable borrow)
        let result = {
            let s = syntax.borrow();
            let env = EvalEnv { syntax: &s.core, py_table: Some(&table_any) };
            matcher::syntax_match(&atp.core, &s.core, &env).map_err(core_err)?
        };
        let Some(result) = result else {
            return Ok(None);
        };
        // Phase 2: apply matched structure (mutable borrow)
        {
            let mut s = syntax.borrow_mut();
            matcher::apply_structure(&mut s.core, &result).map_err(core_err)?;
        }
        // Phase 3: semantic construction (immutable borrow)
        let ctx: Vec<CtxItem> = context_items
            .unwrap_or_default()
            .into_iter()
            .map(|c| c.core)
            .collect();
        let sem = {
            let s = syntax.borrow();
            let env = EvalEnv { syntax: &s.core, py_table: Some(&table_any) };
            match matcher::construct_semantics(&result.pairs, ctx, &s.core, &env) {
                Ok(sem) => Some(sem),
                Err(matcher::SemErr::Match(_)) => None,
                Err(matcher::SemErr::Other(e)) => return Err(core_err(e)),
            }
        };
        let _ = py;
        Ok(sem.map(|sem| PyInterpretableTable {
            table: syntax.clone().unbind(),
            sem: Arc::new(sem),
        }))
    }
}

#[pyclass(name = "TableInterpreter")]
pub struct PyTableInterpreter {
    strategy: SchemaStrategy,
    action_strategy: ActionStrategy,
    missing: Option<PyFunc>,
    transformations: Vec<sp::Transformation>,
    template: String,
}

#[pymethods]
impl PyTableInterpreter {
    #[new]
    fn new() -> Self {
        PyTableInterpreter {
            strategy: SchemaStrategy::RecordFirst,
            action_strategy: ActionStrategy::RowFirst,
            missing: None,
            transformations: Vec::new(),
            template: "$a_%i".to_string(),
        }
    }

    fn with_strategy<'py>(
        mut slf: PyRefMut<'py, Self>,
        strategy: SchemaStrategy,
    ) -> PyRefMut<'py, Self> {
        slf.strategy = strategy;
        slf
    }
    fn with_action_application_strategy<'py>(
        mut slf: PyRefMut<'py, Self>,
        strategy: ActionStrategy,
    ) -> PyRefMut<'py, Self> {
        slf.action_strategy = strategy;
        slf
    }
    fn with_missing_value_handler<'py>(
        mut slf: PyRefMut<'py, Self>,
        handler: Bound<'py, PyAny>,
    ) -> PyRefMut<'py, Self> {
        slf.missing = Some(pyfunc(handler));
        slf
    }
    fn with_transformations<'py>(
        mut slf: PyRefMut<'py, Self>,
        transformations: Vec<Bound<'py, PyAny>>,
    ) -> PyResult<PyRefMut<'py, Self>> {
        let mut out = Vec::new();
        for t in &transformations {
            out.push(extract_transformation(t)?);
        }
        slf.transformations = out;
        Ok(slf)
    }
    fn with_anonymous_attribute_template<'py>(
        mut slf: PyRefMut<'py, Self>,
        template: String,
    ) -> PyResult<PyRefMut<'py, Self>> {
        if !template.contains("%i") {
            return Err(PyValueError::new_err(format!(
                "Template must contain the placeholder %i: {template}"
            )));
        }
        slf.template = template;
        Ok(slf)
    }

    fn interpret(&self, py: Python<'_>, table: &PyInterpretableTable) -> PyResult<PyRecordset> {
        let cfg = InterpreterCfg {
            strategy: self.strategy,
            action_strategy: self.action_strategy,
            missing_value_handler: self.missing.clone(),
            transformations: self.transformations.clone(),
            anonymous_attribute_template: self.template.clone(),
        };
        let table_any: Py<PyAny> = table.table.clone_ref(py).into_any();
        let s = table.table.bind(py).borrow();
        let rs = crate::interp::interpret(&cfg, &s.core, &table.sem, Some(&table_any))
            .map_err(core_err)?;
        Ok(PyRecordset { core: rs })
    }
}

// ---------------------------------------------------------------- RTL facades

#[pyclass(name = "Bindings", frozen)]
#[derive(Clone, Default)]
pub struct PyBindings {
    pub core: BindingsCore,
}

#[pymethods]
impl PyBindings {
    #[staticmethod]
    fn of() -> Self {
        PyBindings::default()
    }
    fn cell(&self, name: String, predicate: Bound<'_, PyAny>) -> PyResult<Self> {
        if name.trim().is_empty() {
            return Err(PyValueError::new_err("Binding name must not be blank"));
        }
        if self.core.cell.contains_key(&name) {
            return Err(PyValueError::new_err(format!(
                "Duplicate cell binding: '{name}'"
            )));
        }
        let mut core = self.core.clone();
        core.cell.insert(name, pyfunc(predicate));
        Ok(PyBindings { core })
    }
    fn filter(&self, name: String, predicate: Bound<'_, PyAny>) -> PyResult<Self> {
        if name.trim().is_empty() {
            return Err(PyValueError::new_err("Binding name must not be blank"));
        }
        if self.core.filter.contains_key(&name) {
            return Err(PyValueError::new_err(format!(
                "Duplicate filter binding: '{name}'"
            )));
        }
        let mut core = self.core.clone();
        core.filter.insert(name, pyfunc(predicate));
        Ok(PyBindings { core })
    }
}

#[pyclass(name = "RtlCompiler")]
pub struct PyRtlCompiler;

#[pymethods]
impl PyRtlCompiler {
    #[staticmethod]
    #[pyo3(signature = (rtl, bindings=None))]
    fn compile(rtl: &str, bindings: Option<PyBindings>) -> PyResult<PyTablePattern> {
        let b = bindings.map(|b| b.core).unwrap_or_default();
        match rtl::compile(rtl, &b) {
            Ok(p) => Ok(PyTablePattern { core: p }),
            Err(e) => Err(rtl_err(e)),
        }
    }
}

#[pyclass(name = "AtpToRtlSerializer")]
pub struct PyAtpToRtlSerializer;

#[pymethods]
impl PyAtpToRtlSerializer {
    #[staticmethod]
    fn serialize(pattern: &PyTablePattern) -> PyResult<String> {
        rtl::serialize::serialize(&pattern.core).map_err(core_err)
    }
}

/// Module-level alias: `pyregtab.compile(rtl, bindings=None)`.
#[pyfunction]
#[pyo3(signature = (rtl, bindings=None))]
pub fn compile(rtl: &str, bindings: Option<PyBindings>) -> PyResult<PyTablePattern> {
    PyRtlCompiler::compile(rtl, bindings)
}

