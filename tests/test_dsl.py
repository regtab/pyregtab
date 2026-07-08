"""Embedded RTL DSL parity — port of jRegTab's DslSpikeTest.

Each test shows an RTL source next to its `pyregtab.dsl` mirror and asserts the
DSL builds byte-identical ATP: both the serialized form and the structural
`TablePattern` must equal what `RtlCompiler.compile` produces.
"""

# flake8: noqa
from pyregtab import AtpToRtlSerializer, RtlCompiler, TablePattern
from pyregtab.dsl import *


def assert_mirrors(rtl: str, dsl: TablePattern) -> None:
    compiled = RtlCompiler.compile(rtl)
    assert AtpToRtlSerializer.serialize(compiled) == AtpToRtlSerializer.serialize(dsl)
    assert compiled == dsl


def test_task001():
    assert_mirrors(
        r"""
        { [ [VAL : ST*->REC] [VAL]{2} []+ ]
        [ [] [VAL]{4} []+ ] }+
        """,
        table(
            subtable(
                row(cell(VAL, rec(ST.unbounded())), cell(VAL).exactly(2), skip().one_or_more()),
                row(skip(), cell(VAL).exactly(4), skip().one_or_more()),
            ).one_or_more()
        ),
    )


def test_task002():
    assert_mirrors(
        r"""
        { [ [VAL=NORM] [] ]{2}
          [ [!BLANK ? VAL : (SC{2}, SR)->REC(2)] [VAL] ]+
          [ [BLANK] [] ]? }+
        """,
        table(
            subtable(
                row(cell(val().extract(NORM)), skip()).exactly(2),
                row(cell(not_blank(), VAL, rec(2, SC.card(2), SR)), cell(VAL)).one_or_more(),
                row(cell(blank()), skip()).zero_or_one(),
            ).one_or_more()
        ),
    )


def test_task006():
    assert_mirrors(
        r"""
        { [ [VAL : ST*->REC] [BLANK ? _ | VAL]+ ]
          [ [BLANK ? _ | VAL]+ ]{4} }+
        """,
        table(
            subtable(
                row(cell(VAL, rec(ST.unbounded())), cell(when(blank(), SKIP, VAL)).one_or_more()),
                row(cell(when(blank(), SKIP, VAL)).one_or_more()).exactly(4),
            ).one_or_more()
        ),
    )


def test_task009():
    assert_mirrors(
        r"""
        [ [] [VAL = REPL('\s+', '')]{5} ]
        [ { [VAL] [BLANK? _ | VAL : (SR, SC)->REC(2)]+ } ]+
        """,
        table(
            subtable(
                row(skip(), cell(val().extract(repl(r"\s+", ""))).exactly(5)),
                row(
                    subrow(
                        cell(VAL),
                        cell(when(blank(), SKIP, val(rec(2, SR, SC)))).one_or_more(),
                    )
                ).one_or_more(),
            )
        ),
    )


def test_task013():
    assert_mirrors(
        r"""
        [ [ATTR]{5} []+ ]
        [ [VAL : SC->AVP, (SR&C2, SR&C4, SR&C1, SR&C3)->REC] [VAL : SC->AVP]{4} []+ ]+
        """,
        table(
            subtable(
                row(cell(ATTR).exactly(5), skip().one_or_more()),
                row(
                    cell(
                        VAL,
                        avp(SC),
                        rec(SR.and_(C(2)), SR.and_(C(4)), SR.and_(C(1)), SR.and_(C(3))),
                    ),
                    cell(VAL, avp(SC)).exactly(4),
                    skip().one_or_more(),
                ).one_or_more(),
            )
        ),
    )


def test_task015():
    assert_mirrors(
        r"""
        [ [VAL ' ' VAL : CL->REC(1) ' ' VAL : CL->REC(1) ' ' VAL : CL->REC(1)] ]+
        """,
        table(
            subtable(
                row(
                    cell(
                        val()
                        .then(" ", val(rec(1, CL)))
                        .then(" ", val(rec(1, CL)))
                        .then(" ", val(rec(1, CL)))
                    )
                ).one_or_more()
            )
        ),
    )


def test_task016():
    assert_mirrors(
        r"""
        [ [VAL : RT->REC, BW&STR*->JOIN(0)] [VAL] ]+
        """,
        table(
            subtable(
                row(
                    cell(VAL, rec(RT), join(0, BW.and_(STR).unbounded())),
                    cell(VAL),
                ).one_or_more()
            )
        ),
    )


def test_task022():
    assert_mirrors(
        r"""
        { [ [VAL : ^ST&C2..5*->REC] [] [VAL]+ ] [ []{2} [VAL]+ ] }+
        """,
        table(
            subtable(
                row(
                    cell(VAL, rec(ST.and_(C(2, 5)).unbounded().col_major())),
                    skip(),
                    cell(VAL).one_or_more(),
                ),
                row(skip().exactly(2), cell(VAL).one_or_more()),
            ).one_or_more()
        ),
    )


def test_task023():
    assert_mirrors(
        r"""
        { [ [VAL : ''->AVP, SR*->REC, BW&STR*->JOIN(0)] [ATTR : RT->SUFFIX] [AUX] [VAL : SR->AVP] ]{3} }+
        """,
        table(
            subtable(
                row(
                    cell(VAL, avp(""), rec(SR.unbounded()), join(0, BW.and_(STR).unbounded())),
                    cell(ATTR, suffix(RT)),
                    cell(AUX),
                    cell(VAL, avp(SR)),
                ).exactly(3)
            ).one_or_more()
        ),
    )


def test_task025():
    assert_mirrors(
        r"""
        [ [VAL : RT->SUFFIX('/'), RT&C+2..*->REC('/'), BW&STR*->JOIN(0)] [VAL]+ ]+
        """,
        table(
            subtable(
                row(
                    cell(
                        VAL,
                        suffix("/", RT),
                        rec_split("/", RT.and_(CrelFrom(2)).unbounded()),
                        join(0, BW.and_(STR).unbounded()),
                    ),
                    cell(VAL).one_or_more(),
                ).one_or_more()
            )
        ),
    )


def test_task029():
    assert_mirrors(
        r"""
        [ [VAL]{6} { [VAL : (ROW{6}, RT*)->REC(6)] [VAL]{3} }+ ]+
        """,
        table(
            subtable(
                row(
                    subrow(cell(VAL).exactly(6)),
                    subrow(
                        cell(VAL, rec(6, ROW.card(6), RT.unbounded())),
                        cell(VAL).exactly(3),
                    ).one_or_more(),
                ).one_or_more()
            )
        ),
    )


def test_task045():
    assert_mirrors(
        r"""
        [ [!BLANK? VAL] [!BLANK? (VAL : SR&C0->REC(1)){','}] ]+
        """,
        table(
            subtable(
                row(
                    cell(not_blank(), VAL),
                    cell(not_blank(), val(rec(1, SR.and_(C(0)))).split_by(",")),
                ).one_or_more()
            )
        ),
    )


def test_task052():
    assert_mirrors(
        r"""
        [ [] [VAL : 'AIRLINE'->AVP]+ ]
        [ [VAL : 'AIRPORT'->AVP]
          [VAL : (COL, ROW, CL, @'YEAR'='2025')->REC, 'ND'->AVP " " VAL : 'MON'->AVP]+ ]+
        """,
        table(
            subtable(
                row(skip(), cell(VAL, avp("AIRLINE")).one_or_more()),
                row(
                    cell(VAL, avp("AIRPORT")),
                    cell(
                        val(rec(COL, ROW, CL, ctx_avp("YEAR", "2025")), avp("ND")).then(
                            " ", val(avp("MON"))
                        )
                    ).one_or_more(),
                ).one_or_more(),
            )
        ),
    )


def test_task068():
    assert_mirrors(
        r"""
        [ [BLANK] [VAL #'HEAD']+ ]+
        [ [!BLANK? VAL] [VAL: (COL&#'HEAD'*, ROW)->REC]+ ]+
        """,
        table(
            subtable(
                row(cell(blank()), cell(val().tagged("HEAD")).one_or_more()).one_or_more(),
                row(
                    cell(not_blank(), VAL),
                    cell(VAL, rec(COL.and_(tag("HEAD")).unbounded(), ROW)).one_or_more(),
                ).one_or_more(),
            )
        ),
    )


def test_task069():
    assert_mirrors(
        r"""
        [ BW*->REC { [ATTR] [VAL#'1': ROW&#'1'*->JOIN][VAL#'2': ROW&#'2'*->JOIN] }* ]
        """,
        table(
            subtable(
                row(
                    acts(rec(BW.unbounded())),
                    subrow(
                        cell(ATTR),
                        cell(val(join(ROW.and_(tag("1")).unbounded())).tagged("1")),
                        cell(val(join(ROW.and_(tag("2")).unbounded())).tagged("2")),
                    ).zero_or_more(),
                )
            )
        ),
    )


def test_task070():
    assert_mirrors(
        r"""
        [ [BLANK]+           [VAL#'H']+ ]+
        [ [!'\d+'? VAL#'S']+ ['\d+'? VAL: (COL&#'H'*, ROW&#'S'*)->REC]+ ]+
        """,
        table(
            subtable(
                row(cell(blank()).one_or_more(), cell(val().tagged("H")).one_or_more()).one_or_more(),
                row(
                    cell(not_re(r"\d+"), val().tagged("S")).one_or_more(),
                    cell(
                        re(r"\d+"),
                        VAL,
                        rec(COL.and_(tag("H")).unbounded(), ROW.and_(tag("S")).unbounded()),
                    ).one_or_more(),
                ).one_or_more(),
            )
        ),
    )


def test_task071():
    assert_mirrors(
        r"""
        [ [BLANK]+       [VAL#'H': BW&#'H'*->SUFFIX('/')]+ ]+
        [ [!'\d+'? VAL#'S': RT&#'S'*->SUFFIX('/')]+ ['\d+'? VAL: (COL, ROW)->REC]+ ]+
        """,
        table(
            subtable(
                row(
                    cell(blank()).one_or_more(),
                    cell(val(suffix("/", BW.and_(tag("H")).unbounded())).tagged("H")).one_or_more(),
                ).one_or_more(),
                row(
                    cell(
                        not_re(r"\d+"),
                        val(suffix("/", RT.and_(tag("S")).unbounded())).tagged("S"),
                    ).one_or_more(),
                    cell(re(r"\d+"), VAL, rec(COL, ROW)).one_or_more(),
                ).one_or_more(),
            )
        ),
    )


def test_task074():
    assert_mirrors(
        r"""
        [ COL->AVP [VAL: (RT*, @'D'='d')->REC][VAL]{2} ]+
        """,
        table(
            subtable(
                row(
                    acts(avp(COL)),
                    cell(VAL, rec(RT.unbounded(), ctx_avp("D", "d"))),
                    cell(VAL).exactly(2),
                ).one_or_more()
            )
        ),
    )


def test_task107():
    v = cell(re(r"\d+"), VAL, rec(COL.and_(tag("H")).unbounded(), ROW.and_(tag("S")).unbounded()))
    assert_mirrors(
        r"""
        $V=['\d+' ? VAL: (COL&#'H'*,ROW&#'S'*)->REC]
        [ [BLANK]+ [!BLANK ? VAL#'H'] [BLANK ? VAL#'H': -LT&!BLANK->FILL | VAL#'H']+ ]+
        {
        [ ['\D.*' ? VAL#'S']+ [$V]+ ]
        [ [BLANK ? VAL#'S': SC->FILL]+ ['\D.*' ? VAL#'S']+ [$V]+ ]*
        }+
        """,
        table(
            subtable(
                row(
                    cell(blank()).one_or_more(),
                    cell(not_blank(), val().tagged("H")),
                    cell(
                        when(
                            blank(),
                            val(fill(LT.and_(item_not_blank()).reversed())).tagged("H"),
                            val().tagged("H"),
                        )
                    ).one_or_more(),
                ).one_or_more()
            ),
            subtable(
                row(cell(re(r"\D.*"), val().tagged("S")).one_or_more(), v.one_or_more()),
                row(
                    cell(blank(), val(fill(SC)).tagged("S")).one_or_more(),
                    cell(re(r"\D.*"), val().tagged("S")).one_or_more(),
                    v.one_or_more(),
                ).zero_or_more(),
            ).one_or_more(),
        ),
    )


def test_task116():
    v1 = cell(VAL, prefix(", ", AV.reversed()))
    v2 = cell(
        VAL,
        avp("VALUE"),
        rec(ROW, COL.and_(R(1, 3)).unbounded(), AV.and_(tag("IND")).reversed()),
    )
    assert_mirrors(
        r"""
        $V1=[VAL: -AV->PREFIX(', ')]
        $V2=[VAL: 'VALUE'->AVP, (ROW, COL&R1..3*, -AV&#'IND')->REC]
        [ []+ ]
        [ [] [VAL: 'TERRITORY'->AVP]+ ]
        [ [AUX]+ ]
        [ 'LOCATION'->AVP [] [$V1]{4} [VAL] []
                             [VAL] [$V1] [VAL]
                             [$V1] [VAL] []
                             { [VAL] [$V1] [VAL] [] }? ]
        { [ [VAL#'IND': 'INDICATOR'->AVP ',' VAL: 'UNIT'->AVP]+ ]
          [ ['20\d\d' ? VAL: 'YEAR'->AVP]
            { [$V2]{5} [] }{2}
            { [$V2]{3} [] }?
          ]+
        }+
        """,
        table(
            subtable(
                row(skip().one_or_more()),
                row(skip(), cell(VAL, avp("TERRITORY")).one_or_more()),
                row(cell(AUX).one_or_more()),
                row(
                    acts(avp("LOCATION")),
                    subrow(
                        skip(), v1.exactly(4), cell(VAL), skip(),
                        cell(VAL), v1, cell(VAL),
                        v1, cell(VAL), skip(),
                    ),
                    subrow(cell(VAL), v1, cell(VAL), skip()).zero_or_one(),
                ),
            ),
            subtable(
                row(
                    cell(
                        val(avp("INDICATOR")).tagged("IND").then(",", val(avp("UNIT")))
                    ).one_or_more()
                ),
                row(
                    subrow(cell(re(r"20\d\d"), VAL, avp("YEAR"))),
                    subrow(v2.exactly(5), skip()).exactly(2),
                    subrow(v2.exactly(3), skip()).zero_or_one(),
                ).one_or_more(),
            ).one_or_more(),
        ),
    )


# ---- ad-hoc constructions ----


def test_adhoc_or_disjunction():
    assert_mirrors(
        r"""
        [ [VAL : (SR&#'t1'|#'t2')*->REC] [VAL] ]+
        """,
        table(
            subtable(
                row(
                    cell(VAL, rec(SR.and_(tag("t1")).or_(tag("t2")).unbounded())),
                    cell(VAL),
                ).one_or_more()
            )
        ),
    )


def test_adhoc_distributed_or():
    assert_mirrors(
        r"""
        [ [VAL : (SR&(#'a'|#'b'))*->REC] [VAL] ]+
        """,
        table(
            subtable(
                row(
                    cell(VAL, rec(SR.and_(tag("a").or_(tag("b"))).unbounded())),
                    cell(VAL),
                ).one_or_more()
            )
        ),
    )


def test_adhoc_settings():
    assert_mirrors(
        r"""
        <NORM> [ [VAL : SR->REC] [VAL] ]+
        """,
        table(subtable(row(cell(VAL, rec(SR)), cell(VAL)).one_or_more())).with_transformations(
            norm()
        ),
    )


def test_adhoc_cell_level_acts():
    assert_mirrors(
        r"""
        [ [RT*->REC BLANK ? _ | VAL] [VAL] ]+
        """,
        table(
            subtable(
                row(
                    cell(acts(rec(RT.unbounded())), when(blank(), SKIP, VAL)),
                    cell(VAL),
                ).one_or_more()
            )
        ),
    )


def test_adhoc_table_level():
    assert_mirrors(
        r"""
        !BLANK ? BW*->REC [ [VAL] ]+
        """,
        table(
            not_blank(),
            acts(rec(BW.unbounded())),
            subtable(row(cell(VAL)).one_or_more()),
        ),
    )


def test_escape_hatch_builds():
    # No RTL equivalent by design — just verify it builds a valid pattern.
    p = table(
        subtable(
            row(
                cell(VAL, rec(ROW.where("is_num", lambda a, c: c.str.isdigit()).unbounded())),
                cell(VAL).one_or_more(),
            )
        )
    )
    assert isinstance(p, TablePattern)
