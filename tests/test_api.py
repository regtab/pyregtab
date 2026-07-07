"""API smoke tests: syntax layer, EXT bindings, custom predicates,
interpreter options, transformations (ports of scattered jRegTab unit tests)."""

import pytest

from pyregtab import (
    AtpMatcher,
    Bindings,
    CellColor,
    DelimitedFieldSplit,
    FontFamily,
    Recordset,
    RtlCompileError,
    RtlCompiler,
    Schema,
    SchemaReordering,
    StringExtractor,
    TableInterpreter,
    TableSyntax,
    WhitespaceNormalization,
    compile as rtl_compile,
)


def make_table(rows):
    t = TableSyntax(len(rows), len(rows[0]))
    for r, row in enumerate(rows):
        for c, v in enumerate(row):
            t.cell(r, c).set_text(v)
    return t


def test_syntax_structure():
    t = TableSyntax(3, 4)
    assert t.num_rows == 3 and t.num_cols == 4
    assert len(t.subtables()) == 1
    t.define_subtables(0, 2)
    assert len(t.subtables()) == 2
    assert t.cell(2, 0).subtable.row_start == 2
    t.define_subrow(0, 0, 1)
    t.define_subrow(0, 2, 3)
    assert len(t.row(0).subrows()) == 2
    assert [c.col for c in t.row(0).subrows()[1].cells()] == [2, 3]
    cell = t.cell(0, 0)
    cell.text = "  hi\nthere"
    assert cell.text_indent == 2 and cell.text_multiline and not cell.text_blank
    cell.font_bold = True
    cell.font_family = FontFamily.MONOSPACED
    cell.bg_color = CellColor(1, 2, 3)
    assert cell.font_bold and cell.font_family == FontFamily.MONOSPACED
    assert cell.bg_color == CellColor(1, 2, 3)


def test_schema_and_recordset():
    s = Schema(["A", "B"])
    with pytest.raises(Exception):
        Schema(["A", "A"])
    rs = Recordset(s, [{"A": "1", "B": "2"}, {"A": "3"}])
    assert len(rs) == 2
    assert rs[0]["A"] == "1"
    assert rs[1]["B"] is None
    assert rs[0].values() == {"A": "1", "B": "2"}


def test_string_extractors():
    assert StringExtractor.trimmed().apply("  x ") == "x"
    assert StringExtractor.whitespace_normalized().apply(" a\t b ") == "a b"
    assert StringExtractor.substring(0, 2).apply("abcdef") == "ab"
    assert StringExtractor.upper_case().apply("ab") == "AB"
    assert StringExtractor.replaced(r"\d+", "#").apply("a1b22") == "a#b#"
    chain = StringExtractor.chain(
        StringExtractor.replaced(",", "."), StringExtractor.trimmed()
    )
    assert chain.apply(" 1,5 ") == "1.5"
    assert StringExtractor.custom("x", lambda s: s[::-1]).apply("abc") == "cba"


def test_ext_bindings():
    table = make_table([["Total", "5"], ["Item", "3"]])
    bindings = Bindings.of().cell("isTotal", lambda c: c.text.startswith("Total"))
    p = RtlCompiler.compile(
        "[ [EXT('isTotal') ? VAL : SR->REC] [VAL] ]\n[ [] [] ]", bindings
    )
    itm = AtpMatcher.match(p, table)
    assert itm is not None
    rs = TableInterpreter().interpret(itm)
    assert len(rs) == 1


def test_ext_unbound_raises():
    with pytest.raises(RtlCompileError):
        RtlCompiler.compile("[ [EXT('nope') ? VAL] ]")


def test_missing_value_handler():
    table = make_table([["a", "b"]])
    p = rtl_compile("[ [VAL : SR{1}->REC] [VAL] ]")
    itm = AtpMatcher.match(p, table)
    rs = (
        TableInterpreter()
        .with_missing_value_handler(lambda attr: f"<{attr}>")
        .interpret(itm)
    )
    assert len(rs) == 1


def test_anonymous_attribute_template():
    table = make_table([["a", "b"]])
    p = rtl_compile("[ [VAL : SR{1}->REC] [VAL] ]")
    itm = AtpMatcher.match(p, table)
    rs = TableInterpreter().with_anonymous_attribute_template("A%i").interpret(itm)
    assert rs.schema.attributes == ["A1", "A2"]


def test_transformations():
    s = Schema(["A", "B"])
    rs = Recordset(s, [{"A": " x  y ", "B": "1/2"}])
    rs2 = WhitespaceNormalization().apply(rs)
    assert rs2[0]["A"] == "x y"
    rs3 = DelimitedFieldSplit("/").apply(rs)
    assert rs3.schema.attributes == ["$a_1", "$a_2", "$a_3"]
    assert [rs3[0][a] for a in rs3.schema.attributes] == [" x  y ", "1", "2"]
    rs4 = SchemaReordering(["B", "A"]).apply(rs)
    assert rs4.schema.attributes == ["B", "A"]
    assert rs4[0]["B"] == "1/2"


def test_custom_cell_predicate_matching():
    from pyregtab import (
        AtomicContentSpec,
        CellMatchCondition,
        CellPattern,
        CellPredicate,
        Quantifier,
        RowPattern,
        SubtablePattern,
        TablePattern,
    )

    table = make_table([["keep", "drop"]])
    cond = CellMatchCondition(
        CellPredicate.custom("starts with k", lambda c: c.text.startswith("k"))
    )
    p = TablePattern.of(
        SubtablePattern.of(
            RowPattern.of(
                CellPattern.of(cond, Quantifier.one(), AtomicContentSpec.val()),
                CellPattern.skip(),
            )
        )
    )
    itm = AtpMatcher.match(p, table)
    assert itm is not None
    items = itm.semantics.cell_derived_items()
    assert [i.str for i in items] == ["keep"]
    assert items[0].cell.text == "keep"


def test_parallel_matching_releases_gil():
    """Batch scenario (plan §4.5): matching/interpretation of pure-native
    patterns runs correctly from a thread pool (GIL released inside)."""
    from concurrent.futures import ThreadPoolExecutor

    p = rtl_compile("[ [VAL : ST*->REC] [VAL] ]\n[ [] [VAL] ]")

    def work(i):
        t = make_table([[f"a{i}", "b"], ["", f"c{i}"]])
        itm = AtpMatcher.match(p, t)
        rs = TableInterpreter().interpret(itm)
        return rs[0][rs.schema.attributes[0]]

    with ThreadPoolExecutor(max_workers=8) as ex:
        results = list(ex.map(work, range(64)))
    assert results == [f"a{i}" for i in range(64)]


def test_serializer_and_repr():
    p = rtl_compile("[ [VAL : ST*->REC] [VAL] ]")
    from pyregtab import AtpToRtlSerializer

    s = AtpToRtlSerializer.serialize(p)
    assert "REC" in s and s.startswith("[")
