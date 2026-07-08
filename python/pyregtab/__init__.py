"""pyRegTab: pattern-based extraction of recordsets from tables.

A Python port of jRegTab with a native (Rust) core. The public API mirrors
the Java API 1:1 (classes and semantics), with snake_case method names:

    TableSyntax -> RtlCompiler/TablePattern -> AtpMatcher -> TableInterpreter -> Recordset
"""

from pyregtab._core import (
    # itm.syntax
    TableSyntax,
    Cell,
    Row,
    Subrow,
    Subtable,
    GridPosition,
    BoundingBox,
    CellColor,
    FontFamily,
    HorizontalAlignment,
    VerticalAlignment,
    # itm.semantics
    ItemType,
    CellDerivedItem,
    ContextDerivedItem,
    TableSemantics,
    InterpretableTable,
    # recordset
    Schema,
    Record,
    Recordset,
    # atp.spec
    ItemDerivationDirective,
    OperationType,
    TraversalOrder,
    CellDerivedProviderKind,
    ContextDerivedProviderKind,
    Quantifier,
    CellPredicate,
    CellMatchCondition,
    FilterTerm,
    ItemFilterConditionSpec,
    StringExtractor,
    ProviderSpec,
    ActionSpec,
    AtomicContentSpec,
    DelimitedContentSpec,
    CompoundContentSpec,
    ConditionalContentSpec,
    CellPattern,
    SubrowPattern,
    RowPattern,
    SubtablePattern,
    TablePattern,
    # atp / interpret
    AtpMatcher,
    TableInterpreter,
    SchemaConstructionStrategy,
    ActionApplicationStrategy,
    WhitespaceNormalization,
    AnchorAttributeAtPosition,
    DelimitedFieldSplit,
    FieldSplitting,
    SchemaReordering,
    # rtl
    Bindings,
    RtlCompiler,
    AtpToRtlSerializer,
    RtlCompileError,
    compile,
    UNBOUNDED,
)

from pyregtab import dsl

__version__ = "0.2.0"

__all__ = [
    "TableSyntax", "Cell", "Row", "Subrow", "Subtable", "GridPosition",
    "BoundingBox", "CellColor", "FontFamily", "HorizontalAlignment",
    "VerticalAlignment", "ItemType", "CellDerivedItem", "ContextDerivedItem",
    "TableSemantics", "InterpretableTable", "Schema", "Record", "Recordset",
    "ItemDerivationDirective", "OperationType", "TraversalOrder",
    "CellDerivedProviderKind", "ContextDerivedProviderKind", "Quantifier",
    "CellPredicate", "CellMatchCondition", "FilterTerm",
    "ItemFilterConditionSpec", "StringExtractor", "ProviderSpec", "ActionSpec",
    "AtomicContentSpec", "DelimitedContentSpec", "CompoundContentSpec",
    "ConditionalContentSpec", "CellPattern", "SubrowPattern", "RowPattern",
    "SubtablePattern", "TablePattern", "AtpMatcher", "TableInterpreter",
    "SchemaConstructionStrategy", "ActionApplicationStrategy",
    "WhitespaceNormalization", "AnchorAttributeAtPosition",
    "DelimitedFieldSplit", "FieldSplitting", "SchemaReordering", "Bindings",
    "RtlCompiler", "AtpToRtlSerializer", "RtlCompileError", "compile",
    "UNBOUNDED", "dsl",
]
