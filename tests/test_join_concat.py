"""CONCAT (fold) vs JOIN (record product), named keys, diagnostics, strict
preconditions, zero-width subrows and the {0}/{1} quantifiers — parity with
jRegTab 0.6.0–0.7.1 (ports of TableInterpreterMultiRecordTest, the named-key
compiler checks and the SyntaxMatcher zero-width cases)."""

import pytest

from pyregtab import (
    ActionSpec,
    AtpMatcher,
    OperationType,
    Quantifier,
    RecordKey,
    RtlCompileError,
    RtlCompiler,
    SchemaConstructionStrategy,
    TableInterpreter,
    TableSyntax,
    AtpToRtlSerializer,
)
from pyregtab.dsl import BW, C, COL, RT, STR, VAL, cell, concat, join, rec, row, subtable, table
from pyregtab import FilterTerm, ProviderSpec, ItemFilterConditionSpec as F


def make_table(rows):
    t = TableSyntax(len(rows), max(len(r) for r in rows))
    for r, line in enumerate(rows):
        for c, v in enumerate(line):
            t.cell(r, c).set_text(v)
    return t


def run(interpreter, rtl, rows):
    pattern = RtlCompiler.compile(rtl)
    itm = AtpMatcher.match(pattern, make_table(rows))
    assert itm is not None, f"pattern did not match:\n{rtl}"
    return pattern.transform(interpreter.interpret(itm))


def rows_of(rs, *attrs):
    return [[rec[a] for a in attrs] for rec in rs.records]


# id | x | y  /  a;b | 1 | 2  /  c | 3 | 4 — explode × stack, six records.
EXPLODE_STACK = [["id", "x", "y"], ["a;b", "1", "2"], ["c", "3", "4"]]

JOIN_PRODUCT = """
[ [ATTR] [VAL: 'var'->AVP]+ ]
[ [(VAL: COL->AVP, ()->REC, RT*->JOIN){';'}] [VAL: 'value'->AVP, COL->REC]+ ]+
"""

# k | v  /  A | 5  /  A | 7 — the two rows share the named attribute v: a CONCAT conflict.
CONFLICT = [["k", "v"], ["A", "5"], ["A", "7"]]

CONCAT_CONFLICT = """
[ [ATTR]+ ]
[ [VAL: COL->AVP, RT->REC, BW&STR*->CONCAT(0)] [VAL: COL->AVP] ]+
"""

PQ = [["p", "q"]]


# ------------------------------------------------------------------ JOIN: record product


def test_join_product_record_first():
    interpreter = TableInterpreter()
    rs = run(interpreter, JOIN_PRODUCT, EXPLODE_STACK)

    assert rs.schema.attributes == ["id", "value", "var"]
    assert len(rs) == 6
    assert rows_of(rs, "id", "var", "value") == [
        ["a", "x", "1"], ["a", "y", "2"],
        ["b", "x", "1"], ["b", "y", "2"],
        ["c", "x", "3"], ["c", "y", "4"],
    ]
    assert interpreter.diagnostics() == []


def test_join_product_position_first_same_records():
    interpreter = TableInterpreter().with_strategy(SchemaConstructionStrategy.POSITION_FIRST)
    rs = run(interpreter, JOIN_PRODUCT, EXPLODE_STACK)

    assert rs.schema.attributes == ["id", "value", "var"]
    assert len(rs) == 6
    assert rows_of(rs, "id", "var", "value")[3] == ["b", "y", "2"]


def test_join_equi_key_by_position_and_by_name_agree():
    # k | v | k | u  /  X | 5 | X | kg  /  Y | 8 | Y | pc  (conformance: join_equi_key / join_named_key)
    rows = [["k", "v", "k", "u"], ["X", "5", "X", "kg"], ["Y", "8", "Y", "pc"]]
    head = "[ [ATTR]+ ]\n"
    by_pos = head + "[ [VAL: COL->AVP, RT->REC, C2*->JOIN(0)] [VAL: COL->AVP] [VAL: COL->AVP, RT->REC] [VAL: COL->AVP] ]+"
    by_name = head + "[ [VAL: COL->AVP, RT->REC, C2*->JOIN('k')] [VAL: COL->AVP] [VAL: COL->AVP, RT->REC] [VAL: COL->AVP] ]+"
    outs = [run(TableInterpreter(), rtl, rows) for rtl in (by_pos, by_name)]
    for rs in outs:
        assert rs.schema.attributes == ["k", "v", "u"]
        assert rows_of(rs, "k", "v", "u") == [["X", "5", "kg"], ["Y", "8", "pc"]]


# ------------------------------------------------------------------ CONCAT: fold, conflicts, diagnostics


def test_concat_folds_rows_with_the_same_key():
    # book | 5 / book | 6 / book | 7 -> book,5,6,7 (task 016 shape)
    rs = run(
        TableInterpreter(),
        "[ [VAL : RT->REC, BW&STR*->CONCAT(0)] [VAL] ]+",
        [["book", "5"], ["book", "6"], ["book", "7"], ["cat", "2"], ["cat", "3"]],
    )
    assert [list(r.values().values()) for r in rs.records] == [
        ["book", "5", "6", "7"],
        ["cat", "2", "3", None],
    ]


def test_concat_conflict_no_effect_both_records_survive_diagnostic_reported():
    interpreter = TableInterpreter()
    rs = run(interpreter, CONCAT_CONFLICT, CONFLICT)

    assert len(rs) == 2, "neither row is folded, nothing is lost silently"
    assert rows_of(rs, "k", "v") == [["A", "5"], ["A", "7"]]
    diags = interpreter.diagnostics()
    assert len(diags) == 1
    d = diags[0]
    assert d.operation == "CONCAT"
    assert "'v'" in d.message
    assert d.anchor.str == "A" and d.anchor.cell.row == 1
    assert str(d).startswith("CONCAT skipped at CellDerivedItem[")


def test_concat_conflict_strict_preconditions_raises():
    interpreter = TableInterpreter().with_strict_preconditions(True)
    with pytest.raises(RuntimeError, match="CONCAT"):
        run(interpreter, CONCAT_CONFLICT, CONFLICT)


def test_concat_named_key_independent_of_field_order():
    # Repeated named fields to the RIGHT of the varying column: CONCAT(0,1,'A','B')
    # is the same key as the positional CONCAT(0,1,3,4) (conformance: concat_named_key).
    rows = [
        ["", "", "C", "A", "B"],
        ["k1", "k11", "c1", "a1", "b1"],
        ["k1", "k11", "c2", "a1", "b1"],
        ["k2", "k22", "c3", "a2", "b2"],
        ["k2", "k22", "c4", "a2", "b2"],
    ]
    head = "[ [] [] [ATTR]+ ]\n"
    named = head + "[ [VAL: RT*->REC, (BW&STR)*->CONCAT(0, 1, 'A', 'B')] [VAL] [VAL] [VAL: COL->AVP]{2} ]+"
    positional = head + "[ [VAL: RT*->REC, (BW&STR)*->CONCAT(0, 1, 3, 4)] [VAL] [VAL] [VAL: COL->AVP]{2} ]+"
    for rtl in (named, positional):
        interpreter = TableInterpreter().with_strategy(SchemaConstructionStrategy.RECORD_FIRST)
        rs = run(interpreter, rtl, rows)
        assert [list(r.values().values()) for r in rs.records] == [
            ["k1", "k11", "c1", "a1", "b1", "c2"],
            ["k2", "k22", "c3", "a2", "b2", "c4"],
        ]
        assert interpreter.diagnostics() == []


# ------------------------------------------------------------------ anchors without records


def test_explicit_join_anchor_without_record_reported():
    interpreter = TableInterpreter()
    rs = run(interpreter, "[ [VAL: RT*->JOIN] [VAL] ]", PQ)

    assert len(rs) == 0, "the anchor has no record, so nothing is extracted"
    diags = interpreter.diagnostics()
    assert len(diags) == 1
    assert diags[0].operation == "JOIN"
    assert diags[0].anchor.str == "p"
    assert "REC missing" in diags[0].message


def test_explicit_join_with_own_rec_not_reported():
    interpreter = TableInterpreter()
    rs = run(interpreter, JOIN_PRODUCT, EXPLODE_STACK)
    assert len(rs) == 6
    assert interpreter.diagnostics() == [], "()->REC gives the token its record"


def test_inherited_join_anchor_without_record_not_reported():
    interpreter = TableInterpreter()
    run(interpreter, "[ RT*->JOIN [VAL] [VAL] ]", PQ)
    assert interpreter.diagnostics() == [], "row-level JOIN reaches cells without records routinely"


def test_explicit_concat_concatenated_away_anchors_not_reported():
    interpreter = TableInterpreter()
    rs = run(
        interpreter,
        "[ [VAL: RT->REC, BW&STR*->CONCAT(0)] [VAL] ]+",
        [["A", "5"], ["A", "7"], ["A", "9"]],
    )
    assert len(rs) == 1, "the three rows fold into one record"
    assert interpreter.diagnostics() == [], "the CONCAT of a folded-away anchor is not an anchor without REC"


def test_explicit_join_no_provided_item_has_record_reported():
    interpreter = TableInterpreter()
    rs = run(interpreter, "[ [VAL: ()->REC, RT*->JOIN] [VAL] ]", PQ)

    assert len(rs) == 1, "the anchor keeps its single-field record"
    diags = interpreter.diagnostics()
    assert len(diags) == 1 and diags[0].operation == "JOIN"
    assert "provider side" in diags[0].message


def test_explicit_join_anchor_without_record_strict_preconditions_raises():
    interpreter = TableInterpreter().with_strict_preconditions(True)
    with pytest.raises(RuntimeError, match="REC missing"):
        run(interpreter, "[ [VAL: RT*->JOIN] [VAL] ]", PQ)


def test_diagnostics_are_reset_per_interpretation():
    interpreter = TableInterpreter()
    run(interpreter, CONCAT_CONFLICT, CONFLICT)
    assert len(interpreter.diagnostics()) == 1
    run(interpreter, JOIN_PRODUCT, EXPLODE_STACK)
    assert interpreter.diagnostics() == []


# ------------------------------------------------------------------ RecordKey and the spec API


def test_record_key_api():
    k = RecordKey.of({1, 0}, {"B", "A"})
    assert k.positions == {0, 1} and k.names == {"A", "B"}
    assert RecordKey.of([0, 1]) == RecordKey({0, 1})
    assert RecordKey.of(names=["A"]) == RecordKey(names=["A"])
    assert RecordKey.empty().is_empty() and RecordKey() == RecordKey.empty()
    assert "positions=[0, 1]" in repr(k)
    with pytest.raises(Exception):
        RecordKey.of(names=[" "])
    with pytest.raises(Exception):
        RecordKey.of([-1])


def test_action_spec_concat_and_join_keys():
    prov = ProviderSpec.val(F.and_(FilterTerm.below(), FilterTerm.same_str()), cardinality=2**31)
    a = ActionSpec.concat(prov, key=0)
    assert a.operation_type == OperationType.CONCAT
    assert a.key == RecordKey.of([0]) and a.key_positions == {0}
    b = ActionSpec.concat(prov, key=(0, 1, "A", "B"))
    assert b.key == RecordKey.of({0, 1}, {"A", "B"})
    c = ActionSpec.concat(prov, key="A")
    assert c.key == RecordKey.of(names=["A"])
    # legacy spelling still works
    d = ActionSpec.join(prov, key_positions={0, 1})
    assert d.operation_type == OperationType.JOIN and d.key == RecordKey.of([0, 1])
    e = ActionSpec(OperationType.JOIN, providers=[prov], key=RecordKey.of(names=["k"]))
    assert e.key == RecordKey.of(names=["k"])


def test_compiler_and_serializer_named_keys():
    canonical = AtpToRtlSerializer.serialize(
        RtlCompiler.compile("[ [VAL: RT*->REC, (BW&STR)*->CONCAT(\"B\", 'A', 1, 0)] [VAL] ]+")
    )
    assert "CONCAT(0, 1, 'A', 'B')" in canonical
    assert "JOIN('it''s')" in AtpToRtlSerializer.serialize(
        RtlCompiler.compile("[ [VAL: RT*->REC, RT*->JOIN('it''s')] [VAL] ]+")
    )
    with pytest.raises(RtlCompileError):
        RtlCompiler.compile("[ [VAL : RT->REC, BW*->CONCAT('')] ]")


def test_dsl_concat_and_join_mirror_the_compiler():
    def canon(p):
        return AtpToRtlSerializer.serialize(p)

    compiled = RtlCompiler.compile("[ [VAL : RT->REC, BW&STR*->CONCAT(0, 'A')] [VAL] ]+")
    built = table(subtable(row(
        cell(VAL, rec(RT), concat((0, "A"), BW.and_(STR).unbounded())),
        cell(VAL),
    ).one_or_more()))
    assert canon(built) == canon(compiled)

    compiled = RtlCompiler.compile("[ [VAL: COL->AVP, RT->REC, C2*->JOIN('k')] [VAL: COL->AVP] ]+")
    built = table(subtable(row(
        cell(VAL, __import__("pyregtab.dsl", fromlist=["avp"]).avp(COL), rec(RT), join("k", C(2).unbounded())),
        cell(VAL, __import__("pyregtab.dsl", fromlist=["avp"]).avp(COL)),
    ).one_or_more()))
    assert canon(built) == canon(compiled)


# ------------------------------------------------------------------ zero-width subrows and {n}, n < 2


ZERO_WIDTH_ROWS = [["A", "B"], ["x", "1"], ["y", "2"]]


@pytest.mark.parametrize(
    "body",
    [
        "[VAL: COL->AVP, ROW*->REC] { [BLANK]* } [VAL: COL->AVP]",          # mid
        "[VAL: COL->AVP, ROW*->REC] [VAL: COL->AVP] { [BLANK]* }",          # tail
        "[VAL: COL->AVP, ROW*->REC] { [BLANK]* }+ [VAL: COL->AVP] { [BLANK]* }+",
        "[VAL: COL->AVP, ROW*->REC] { [BLANK]* }* [VAL: COL->AVP] { [BLANK]* }*",
        "[VAL: COL->AVP, ROW*->REC] { [BLANK]* }{1} [VAL: COL->AVP]",
        "[VAL: COL->AVP, ROW*->REC] { [BLANK]* }{0} [VAL: COL->AVP]",
        "[VAL: COL->AVP, ROW*->REC] { [BLANK]* }{3} [VAL: COL->AVP]",
    ],
)
def test_zero_width_subrow_matches_the_empty_sequence(body):
    rs = run(TableInterpreter(), f"[ [ATTR]+ ]\n[ {body} ]+", ZERO_WIDTH_ROWS)
    assert rows_of(rs, "A", "B") == [["x", "1"], ["y", "2"]]


def test_empty_subrow_is_not_materialized_in_the_syntax_layer():
    t = make_table(ZERO_WIDTH_ROWS)
    pattern = RtlCompiler.compile("[ [ATTR]+ ]\n[ [VAL: COL->AVP, ROW*->REC] { [BLANK]* } [VAL: COL->AVP] ]+")
    assert AtpMatcher.match(pattern, t) is not None
    # the empty subrow covers no cells: the header row keeps its single subrow,
    # the data rows are split into the two implicit subrows around it, and
    # every subrow is non-empty
    assert [len(t.row(r).subrows()) for r in range(3)] == [1, 2, 2]
    for r in range(3):
        for sr in t.row(r).subrows():
            assert sr.col_end >= sr.col_start


def test_empty_subtable_all_rows_optional():
    rs = run(
        TableInterpreter(),
        "{ [ [BLANK]+ ]* } [ [ATTR]+ ] [ [VAL: COL->AVP, ROW*->REC] [VAL: COL->AVP] ]+",
        ZERO_WIDTH_ROWS,
    )
    assert rows_of(rs, "A", "B") == [["x", "1"], ["y", "2"]]


def test_quantifier_small_n():
    assert Quantifier.exactly(1).min() == 1 and Quantifier.exactly(1).max() == 1
    assert Quantifier.exactly(0).min() == 0 and Quantifier.exactly(0).max() == 0
    with pytest.raises(Exception):
        Quantifier.exactly(-1)
    # {1} is equivalent to no quantifier, {0} matches nothing
    assert AtpToRtlSerializer.serialize(RtlCompiler.compile("[ [VAL]{1} [VAL]{0} { [BLANK]* }{1} ]")) == \
        "[ [ VAL ]{1} [ VAL ]{0} { [ BLANK ]* }{1} ]"
    with pytest.raises(RtlCompileError):
        RtlCompiler.compile("[ [VAL]{-1} ]")
    err = None
    try:
        RtlCompiler.compile("[ [VAL]{99999999999} ]")
    except RtlCompileError as e:
        err = e
    assert err is not None and (err.line, err.col) == (1, 8)  # the INT token, as in Java
