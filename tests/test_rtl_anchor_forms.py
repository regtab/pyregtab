"""ANCH(n) / REC(n) at the RTL level: the settings prefix, inline REC(n) on an
atomic content specification and inline REC(n) inside a delimited one request the
same transformation, and it moves the anchor *attribute* -- name together with its
values (port of jRegTab's RtlAnchorPositionFormsTest)."""

from pyregtab import (
    AtpMatcher,
    RtlCompiler,
    SchemaConstructionStrategy,
    TableInterpreter,
    TableSyntax,
)

# Header row + two data rows; the anchor column ("Lokaler") sits in the middle.
TABLE = [
    ["Dato", "Lokaler", "Klasse"],
    ["20.05", "AU", "0"],
    ["11.06", "A2.1", "1"],
]

SETTINGS_PREFIX = """\
<ANCH(1)>
[ [ATTR]+ ]
[ COL->AVP [VAL] [VAL: ROW*->REC] [VAL] ]+
"""

INLINE_ATOMIC = """\
[ [ATTR]+ ]
[ COL->AVP [VAL] [VAL: ROW*->REC(1)] [VAL] ]+
"""

INLINE_DELIMITED = """\
[ [ATTR]+ ]
[ COL->AVP [VAL] [(VAL: ROW*->REC(1)){','}] [VAL] ]+
"""


def make_table(rows):
    t = TableSyntax(len(rows), len(rows[0]))
    for r, row in enumerate(rows):
        for c, v in enumerate(row):
            t.cell(r, c).set_text(v)
    return t


def run(rtl, rows):
    pattern = RtlCompiler.compile(rtl)
    itm = AtpMatcher.match(pattern, make_table(rows))
    assert itm is not None, f"pattern did not match:\n{rtl}"
    return pattern.transform(
        TableInterpreter()
        .with_strategy(SchemaConstructionStrategy.RECORD_FIRST)
        .interpret(itm)
    )


def values(rs, record):
    return [rs[record][a] for a in rs.schema.attributes]


def dump(rs):
    return "\n".join(
        [str(rs.schema.attributes)] + [str(values(rs, i)) for i in range(len(rs))]
    )


def test_settings_prefix_moves_the_anchor_attribute():
    # The extracted schema is anchor-first (Lokaler, Dato, Klasse); ANCH(1) restores
    # the column order of the table with every attribute-value binding intact.
    rs = run(SETTINGS_PREFIX, TABLE)
    assert rs.schema.attributes == ["Dato", "Lokaler", "Klasse"]
    assert values(rs, 0) == ["20.05", "AU", "0"]
    assert values(rs, 1) == ["11.06", "A2.1", "1"]


def test_all_three_forms_agree():
    via_settings = run(SETTINGS_PREFIX, TABLE)
    via_atomic = run(INLINE_ATOMIC, TABLE)
    via_delimited = run(INLINE_DELIMITED, TABLE)
    assert dump(via_settings) == dump(via_atomic)
    assert dump(via_settings) == dump(via_delimited)


def test_inline_rec_inside_delimited_specification():
    # One record per token, names intact; tokens are raw, so " C1.1" keeps its
    # leading space (the S_delim rule).
    rows = [
        ["Dato", "Lokaler", "Klasse"],
        ["20.05", "AU, C1.1", "0"],
        ["11.06", "A2.1", "1"],
    ]
    rs = run(INLINE_DELIMITED, rows)
    assert rs.schema.attributes == ["Dato", "Lokaler", "Klasse"]
    assert len(rs) == 3
    assert values(rs, 0) == ["20.05", "AU", "0"]
    assert values(rs, 1) == ["20.05", " C1.1", "0"]
    assert values(rs, 2) == ["11.06", "A2.1", "1"]
