//! pyRegTab native core: a Rust port of jRegTab (RTL compiler, ATP matcher,
//! table interpreter) exposed to Python as `pyregtab._core`.

pub mod interp;
pub mod matcher;
pub mod py;
pub mod recordset;
pub mod rtl;
pub mod semantics;
pub mod spec;
pub mod syntax;
#[cfg(test)]
mod tests;
pub mod util;

use pyo3::prelude::*;

#[pymodule]
fn _core(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // syntax
    m.add_class::<syntax::FontFamily>()?;
    m.add_class::<syntax::HorizontalAlignment>()?;
    m.add_class::<syntax::VerticalAlignment>()?;
    m.add_class::<py::PyGridPosition>()?;
    m.add_class::<py::PyBoundingBox>()?;
    m.add_class::<py::PyCellColor>()?;
    m.add_class::<py::PyTableSyntax>()?;
    m.add_class::<py::PyCell0>()?;
    m.add_class::<py::PyRow>()?;
    m.add_class::<py::PySubrow>()?;
    m.add_class::<py::PySubtable>()?;
    // items / semantics
    m.add_class::<spec::ItemType>()?;
    m.add_class::<py::PyCellDerivedItem>()?;
    m.add_class::<py::PyContextDerivedItem>()?;
    m.add_class::<py::PyTableSemantics>()?;
    m.add_class::<py::PyInterpretableTable>()?;
    // recordset
    m.add_class::<py::PySchema>()?;
    m.add_class::<py::PyRecord>()?;
    m.add_class::<py::PyRecordset>()?;
    // spec
    m.add_class::<spec::Idd>()?;
    m.add_class::<spec::OperationType>()?;
    m.add_class::<spec::TraversalOrder>()?;
    m.add_class::<spec::CellKind>()?;
    m.add_class::<spec::CtxKind>()?;
    m.add_class::<py::PyQuantifier>()?;
    m.add_class::<py::PyCellPredicate>()?;
    m.add_class::<py::PyCellMatchCondition>()?;
    m.add_class::<py::PyFilterTerm>()?;
    m.add_class::<py::PyFilterCond>()?;
    m.add_class::<py::PyExtractor>()?;
    m.add_class::<py::PyProviderSpec>()?;
    m.add_class::<py::PyActionSpec>()?;
    m.add_class::<py::PyAtomicSpec>()?;
    m.add_class::<py::PyDelimitedSpec>()?;
    m.add_class::<py::PyCompoundSpec>()?;
    m.add_class::<py::PyConditionalSpec>()?;
    m.add_class::<py::PyCellPattern>()?;
    m.add_class::<py::PySubrowPattern>()?;
    m.add_class::<py::PyRowPattern>()?;
    m.add_class::<py::PySubtablePattern>()?;
    m.add_class::<py::PyTablePattern>()?;
    // interpret
    m.add_class::<interp::SchemaStrategy>()?;
    m.add_class::<interp::ActionStrategy>()?;
    m.add_class::<py::PyAtpMatcher>()?;
    m.add_class::<py::PyTableInterpreter>()?;
    m.add_class::<py::PyWhitespaceNormalization>()?;
    m.add_class::<py::PyAnchorAttributeAtPosition>()?;
    m.add_class::<py::PyDelimitedFieldSplit>()?;
    m.add_class::<py::PyFieldSplitting>()?;
    m.add_class::<py::PySchemaReordering>()?;
    // rtl
    m.add_class::<py::PyBindings>()?;
    m.add_class::<py::PyRtlCompiler>()?;
    m.add_class::<py::PyAtpToRtlSerializer>()?;
    m.add_function(wrap_pyfunction!(py::compile, m)?)?;
    m.add("RtlCompileError", m.py().get_type::<py::RtlCompileError>())?;
    m.add("UNBOUNDED", spec::UNBOUNDED)?;
    Ok(())
}
