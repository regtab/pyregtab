"""ATP patterns for all benchmark tasks, mechanically translated from
jRegTab's AtpTaskNNNTest.java fluent builders (see tools/translate_atp.py).
Do not edit by hand except to fix translation issues."""

# flake8: noqa
from pyregtab import (
    ActionSpec,
    AtomicContentSpec,
    CellMatchCondition,
    CellPattern,
    CellPredicate,
    CompoundContentSpec,
    ConditionalContentSpec,
    DelimitedContentSpec,
    FilterTerm,
    ItemFilterConditionSpec,
    ProviderSpec,
    Quantifier,
    RowPattern,
    StringExtractor,
    SubrowPattern,
    SubtablePattern,
    TablePattern,
    TraversalOrder,
    UNBOUNDED,
)

def pattern_001():
    SAME_SUBTABLE = ItemFilterConditionSpec.same_subtable()
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBTABLE, UNBOUNDED)))), CellPattern.of(Quantifier.exactly(2), AtomicContentSpec.val()), CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.exactly(4), AtomicContentSpec.val()), CellPattern.skip(Quantifier.one_or_more()))))

def pattern_002():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    BLANK = CellMatchCondition(CellPredicate.blank())
    SAME_SUBCOLUMN = ItemFilterConditionSpec.same_subcol()
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(Quantifier.exactly(2), CellPattern.of(AtomicContentSpec.val(extractor=StringExtractor.whitespace_normalized())), CellPattern.skip()), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBCOLUMN, 2), ProviderSpec.val(SAME_SUBROW, 1), anchor_pos=2))), CellPattern.of(AtomicContentSpec.val())), RowPattern.of(Quantifier.zero_or_one(), CellPattern.of(BLANK, Quantifier.one(), None), CellPattern.skip())))

def pattern_003():
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val()), CellPattern.of(Quantifier.exactly(2), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBROW, 1), anchor_pos=1))))))

def pattern_004():
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val()), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBROW, 1), anchor_pos=1))))))

def pattern_005():
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    SAME_SUBCOLUMN = ItemFilterConditionSpec.same_subcol()
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val())), RowPattern.of(CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val()), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBROW, 1), ProviderSpec.val(SAME_SUBCOLUMN, 1), anchor_pos=2))))))

def pattern_006():
    BLANK = CellMatchCondition(CellPredicate.blank())
    SAME_SUBTABLE = ItemFilterConditionSpec.same_subtable()
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBTABLE, UNBOUNDED)))), CellPattern.of(Quantifier.one_or_more(), ConditionalContentSpec(BLANK, AtomicContentSpec.skip(), AtomicContentSpec.val()))), RowPattern.of(Quantifier.exactly(4), CellPattern.of(Quantifier.one_or_more(), ConditionalContentSpec(BLANK, AtomicContentSpec.skip(), AtomicContentSpec.val())))))

def pattern_007():
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    SAME_SUBCOLUMN = ItemFilterConditionSpec.same_subcol()
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(Quantifier.exactly(3)), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(Quantifier.exactly(3), AtomicContentSpec.val()), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBROW, 3), ProviderSpec.val(SAME_SUBCOLUMN, 1), anchor_pos=4))))))

def pattern_008():
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val()), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBROW, 1), anchor_pos=1))))))

def pattern_009():
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    SAME_SUBCOLUMN = ItemFilterConditionSpec.same_subcol()
    BLANK = CellMatchCondition(CellPredicate.blank())
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.exactly(5), AtomicContentSpec.val(extractor=StringExtractor.replaced("\\s+", "")))), RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(CellPattern.of(AtomicContentSpec.val()), CellPattern.of(Quantifier.one_or_more(), ConditionalContentSpec(BLANK, AtomicContentSpec.skip(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBROW, 1), ProviderSpec.val(SAME_SUBCOLUMN, 1), anchor_pos=2))))))))

def pattern_010():
    BLANK = CellMatchCondition(CellPredicate.blank())
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(Quantifier.zero_or_more(), CellPattern.skip(Quantifier.exactly(4)), CellPattern.of(BLANK, Quantifier.one(), None), CellPattern.skip(Quantifier.exactly(3))), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBROW, UNBOUNDED)))), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val())), RowPattern.of(Quantifier.zero_or_one(), CellPattern.of(BLANK, Quantifier.one_or_more(), None))))

def pattern_011():
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    SAME_SUBCOLUMN = ItemFilterConditionSpec.same_subcol()
    BLANK = CellMatchCondition(CellPredicate.blank())
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val())), RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(CellPattern.of(AtomicContentSpec.val()), CellPattern.of(Quantifier.one_or_more(), ConditionalContentSpec(BLANK, AtomicContentSpec.skip(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBROW, 1), ProviderSpec.val(SAME_SUBCOLUMN, 1), anchor_pos=2))))))))

def pattern_012():
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.bare(FilterTerm.col_exact(5)), UNBOUNDED)))), CellPattern.skip(Quantifier.exactly(4)), CellPattern.of(AtomicContentSpec.val())), RowPattern.of(Quantifier.one_or_more(), CellPattern.skip(Quantifier.exactly(5)), CellPattern.of(AtomicContentSpec.val()))))

def pattern_013():
    SAME_SUBCOLUMN = ItemFilterConditionSpec.same_subcol()
    SAME_SUBROW_COL2 = ItemFilterConditionSpec.and_(FilterTerm.same_subrow(), FilterTerm.col_exact(2))
    SAME_SUBROW_COL4 = ItemFilterConditionSpec.and_(FilterTerm.same_subrow(), FilterTerm.col_exact(4))
    SAME_SUBROW_COL1 = ItemFilterConditionSpec.and_(FilterTerm.same_subrow(), FilterTerm.col_exact(1))
    SAME_SUBROW_COL3 = ItemFilterConditionSpec.and_(FilterTerm.same_subrow(), FilterTerm.col_exact(3))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(Quantifier.exactly(5), AtomicContentSpec.attr()), CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_SUBCOLUMN)), ActionSpec.rec(ProviderSpec.val(SAME_SUBROW_COL2, 1), ProviderSpec.val(SAME_SUBROW_COL4, 1), ProviderSpec.val(SAME_SUBROW_COL1, 1), ProviderSpec.val(SAME_SUBROW_COL3, 1)))), CellPattern.of(Quantifier.exactly(4), AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_SUBCOLUMN)))), CellPattern.skip(Quantifier.one_or_more()))))

def pattern_014():
    SAME_SUBTABLE_COL0 = ItemFilterConditionSpec.and_(FilterTerm.same_subtable(), FilterTerm.col_exact(0))
    SAME_SUBTABLE_COL1 = ItemFilterConditionSpec.and_(FilterTerm.same_subtable(), FilterTerm.col_exact(1))
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    BLANK = CellMatchCondition(CellPredicate.blank())
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(NOT_BLANK, Quantifier.exactly(2), AtomicContentSpec.val()), CellPattern.of(BLANK, Quantifier.one(), None)), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(NOT_BLANK, Quantifier.exactly(2), AtomicContentSpec.val()), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBTABLE_COL0, 1), ProviderSpec.val(SAME_SUBTABLE_COL1, 1), ProviderSpec.val(SAME_SUBROW, 2), anchor_pos=4))))))

def pattern_015():
    SAME_CELL = ItemFilterConditionSpec.same_cell()
    compoundSpec = CompoundContentSpec.of(AtomicContentSpec.val(), (" ", AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_CELL, 1), anchor_pos=1))), (" ", AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_CELL, 1), anchor_pos=1))), (" ", AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_CELL, 1), anchor_pos=1))))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), CellPattern.of(compoundSpec))))

def pattern_016():
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    BELOW_STR = ItemFilterConditionSpec.and_(FilterTerm.below(), FilterTerm.same_str())
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(RIGHT_OF, 1)), ActionSpec.join(ProviderSpec.val(BELOW_STR, UNBOUNDED), key_positions=0))), CellPattern.of(AtomicContentSpec.val()))))

def pattern_017():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    BELOW = ItemFilterConditionSpec.below()
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(BELOW, UNBOUNDED))))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val())), RowPattern.of(Quantifier.zero_or_one(), CellPattern.skip())))

def pattern_018():
    SAME_CELL = ItemFilterConditionSpec.same_cell()
    BELOW_SUBTABLE = ItemFilterConditionSpec.same_subtable()
    firstRow = CompoundContentSpec.of(AtomicContentSpec.attr(), ("=", AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(BELOW_SUBTABLE, UNBOUNDED)), ActionSpec.avp(ProviderSpec.attr(SAME_CELL)))))
    otherRows = CompoundContentSpec.of(AtomicContentSpec.attr(), ("=", AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_CELL)))))
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(firstRow)), RowPattern.of(Quantifier.exactly(15), CellPattern.of(otherRows))))

def pattern_019():
    BELOW = ItemFilterConditionSpec.below()
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(BELOW, UNBOUNDED))))), RowPattern.of(Quantifier.exactly(3), CellPattern.of(AtomicContentSpec.val()))))

def pattern_020():
    SAME_SUBTABLE = ItemFilterConditionSpec.same_subtable()
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBTABLE, UNBOUNDED)))), CellPattern.of(AtomicContentSpec.val())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(Quantifier.exactly(2), AtomicContentSpec.val()))))

def pattern_021():
    BELOW = ItemFilterConditionSpec.below()
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(BELOW, UNBOUNDED)), extractor=StringExtractor.whitespace_normalized()))), RowPattern.of(Quantifier.exactly(2), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(extractor=StringExtractor.whitespace_normalized())))))

def pattern_022():
    SAME_SUBTABLE_COLS2_5 = ItemFilterConditionSpec.and_(FilterTerm.same_subtable(), FilterTerm.col_absolute_range(2, 5))
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBTABLE_COLS2_5, UNBOUNDED, traversal_order=TraversalOrder.COLUMN_MAJOR)))), CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val())), RowPattern.of(CellPattern.skip(Quantifier.exactly(2)), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val()))))

def pattern_023():
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    BELOW_STR = ItemFilterConditionSpec.and_(FilterTerm.below(), FilterTerm.same_str())
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(Quantifier.exactly(3), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp(""), ActionSpec.rec(ProviderSpec.val(SAME_SUBROW, UNBOUNDED)), ActionSpec.join(ProviderSpec.val(BELOW_STR, UNBOUNDED), key_positions=0))), CellPattern.of(AtomicContentSpec.attr(ActionSpec.suffix("", ProviderSpec.any(RIGHT_OF, 1, traversal_order=TraversalOrder.ROW_MAJOR)))), CellPattern.of(AtomicContentSpec.aux()), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_SUBROW)))))))

def pattern_024():
    BELOW = ItemFilterConditionSpec.below()
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(BELOW, UNBOUNDED))))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val()))))

def pattern_025():
    SEP = "/"
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    BELOW_STR = ItemFilterConditionSpec.and_(FilterTerm.below(), FilterTerm.same_str())
    SUBROW_AFTER_ANCHOR = ItemFilterConditionSpec.and_(FilterTerm.right_of(), FilterTerm.col_range(2, UNBOUNDED))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.suffix(SEP, ProviderSpec.any(RIGHT_OF, 1)), ActionSpec.rec(ProviderSpec.val(SUBROW_AFTER_ANCHOR, UNBOUNDED), split_delimiter=SEP), ActionSpec.join(ProviderSpec.val(BELOW_STR, UNBOUNDED), key_positions=0))), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val()))))

def pattern_026():
    SAME_SUBTABLE_COL2 = ItemFilterConditionSpec.and_(FilterTerm.same_subtable(), FilterTerm.col_exact(2))
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.avp(""), ActionSpec.rec(ProviderSpec.val(SAME_SUBTABLE_COL2, UNBOUNDED)))), CellPattern.of(AtomicContentSpec.attr()), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_SUBROW))))), RowPattern.of(Quantifier.exactly(5), CellPattern.skip(), CellPattern.of(AtomicContentSpec.attr()), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_SUBROW)))))))

def pattern_027():
    BELOW = ItemFilterConditionSpec.below()
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(BELOW, UNBOUNDED))))), RowPattern.of(CellPattern.skip()), RowPattern.of(Quantifier.exactly(9), CellPattern.of(AtomicContentSpec.val()))))

def pattern_028():
    SAME_SUBTABLE = ItemFilterConditionSpec.same_subtable()
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBTABLE, UNBOUNDED)))), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val()))))

def pattern_029():
    SAME_ROW = ItemFilterConditionSpec.same_row()
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(CellPattern.of(Quantifier.exactly(6), AtomicContentSpec.val())), SubrowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_ROW, 6), ProviderSpec.val(RIGHT_OF, UNBOUNDED), anchor_pos=6))), CellPattern.of(Quantifier.exactly(3), AtomicContentSpec.val())))))

def pattern_030():
    BELOW = ItemFilterConditionSpec.below()
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(BELOW, UNBOUNDED))))), RowPattern.of(Quantifier.exactly(3), CellPattern.of(AtomicContentSpec.val()))))

def pattern_031():
    BELOW = ItemFilterConditionSpec.below()
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(BELOW, UNBOUNDED))))), RowPattern.of(Quantifier.exactly(4), CellPattern.of(AtomicContentSpec.val())), RowPattern.of(CellPattern.skip())))

def pattern_032():
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    SAME_SUBCOLUMN = ItemFilterConditionSpec.same_subcol()
    BLANK = CellMatchCondition(CellPredicate.blank())
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val()), CellPattern.of(Quantifier.one_or_more(), ConditionalContentSpec(BLANK, AtomicContentSpec.skip(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBROW, 1), ProviderSpec.val(SAME_SUBCOLUMN, 1), anchor_pos=2)))))))

def pattern_033():
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    BELOW_STR = ItemFilterConditionSpec.and_(FilterTerm.below(), FilterTerm.same_str())
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBROW, UNBOUNDED)), ActionSpec.join(ProviderSpec.val(BELOW_STR, UNBOUNDED), key_positions=0))), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val()))))

def pattern_034():
    BELOW = ItemFilterConditionSpec.below()
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(BELOW, UNBOUNDED))))), RowPattern.of(Quantifier.exactly(4), CellPattern.of(AtomicContentSpec.val()))))

def pattern_035():
    BELOW = ItemFilterConditionSpec.below()
    COMPANY_ROW = CellMatchCondition(CellPredicate.contains("*Company"))
    NOT_COMPANY_ROW = CellMatchCondition(CellPredicate.not_contains("*Company"))
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(COMPANY_ROW, Quantifier.one(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(BELOW, UNBOUNDED)), extractor=StringExtractor.replaced("\\*", "")))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(NOT_COMPANY_ROW, Quantifier.one(), AtomicContentSpec.val()))))

def pattern_036():
    SAME_SUBTABLE_COL2 = ItemFilterConditionSpec.and_(FilterTerm.same_subtable(), FilterTerm.col_exact(2))
    LEFT_OF = ItemFilterConditionSpec.left_of()
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.avp(""), ActionSpec.rec(ProviderSpec.val(SAME_SUBTABLE_COL2, UNBOUNDED)))), CellPattern.of(AtomicContentSpec.attr()), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(LEFT_OF))))), RowPattern.of(Quantifier.exactly(11), CellPattern.skip(), CellPattern.of(AtomicContentSpec.attr()), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(LEFT_OF)))))))

def pattern_037():
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    SAME_SUBCOLUMN = ItemFilterConditionSpec.same_subcol()
    BLANK = CellMatchCondition(CellPredicate.blank())
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val()), CellPattern.of(Quantifier.one_or_more(), ConditionalContentSpec(BLANK, AtomicContentSpec.skip(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBROW, 1), ProviderSpec.val(SAME_SUBCOLUMN, 1), anchor_pos=2)))))))

def pattern_038():
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    ABOVE = ItemFilterConditionSpec.above()
    BLANK = CellMatchCondition(CellPredicate.blank())
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBROW, UNBOUNDED)))), CellPattern.of(AtomicContentSpec.val()), CellPattern.of(ConditionalContentSpec(BLANK, AtomicContentSpec.val(ActionSpec.fill("", ProviderSpec.any(ABOVE, 1, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR))), AtomicContentSpec.val())))))

def pattern_039():
    SAME_CELL = ItemFilterConditionSpec.same_cell()
    priceBedroomSpec = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_CELL, UNBOUNDED))), (" / ", AtomicContentSpec.val()), ("br", AtomicContentSpec.skip()))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), CellPattern.of(priceBedroomSpec))))

def pattern_040():
    REPORTED_CRIME_TITLE = CellMatchCondition(CellPredicate.contains("Reported crime in"))
    SAME_SUBTABLE_COL1 = ItemFilterConditionSpec.and_(FilterTerm.same_subtable(), FilterTerm.col_exact(1))
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(REPORTED_CRIME_TITLE, Quantifier.one(), AtomicContentSpec.val(ActionSpec.avp(""), ActionSpec.rec(ProviderSpec.val(SAME_SUBTABLE_COL1, UNBOUNDED)), extractor=StringExtractor.chain(StringExtractor.replaced("Reported crime in", ""), StringExtractor.trimmed()))), CellPattern.skip()), RowPattern.of(CellPattern.skip(Quantifier.exactly(2))), RowPattern.of(Quantifier.exactly(5), CellPattern.of(AtomicContentSpec.attr()), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_SUBROW))))), RowPattern.of(Quantifier.zero_or_one(), CellPattern.skip(Quantifier.exactly(2)))))

def pattern_041():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    BLANK = CellMatchCondition(CellPredicate.blank())
    SAME_CELL = ItemFilterConditionSpec.same_cell()
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    pairValSpec = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.fill("", ProviderSpec.ctx_attr("")), ActionSpec.rec(ProviderSpec.val(SAME_CELL, 1), ProviderSpec.val(RIGHT_OF, 1))), ("", AtomicContentSpec.val()))
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(Quantifier.zero_or_one(), CellPattern.of(NOT_BLANK, Quantifier.one(), pairValSpec), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val())), RowPattern.of(Quantifier.zero_or_one(), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(RIGHT_OF, 1), ProviderSpec.ctx_val("")))), CellPattern.of(BLANK, Quantifier.one(), None))))

def pattern_042():
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    SAME_CELL = ItemFilterConditionSpec.same_cell()
    subjectValue = CompoundContentSpec.of(AtomicContentSpec.attr(), (":", AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_CELL)))))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp(""), ActionSpec.rec(ProviderSpec.val(RIGHT_OF, UNBOUNDED)))), CellPattern.of(Quantifier.exactly(2), subjectValue))))

def pattern_043():
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    SAME_CELL = ItemFilterConditionSpec.same_cell()
    subjectValue = CompoundContentSpec.of(AtomicContentSpec.attr(), (":", AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_CELL)))))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp(""), ActionSpec.rec(ProviderSpec.val(RIGHT_OF, UNBOUNDED)))), CellPattern.of(Quantifier.exactly(3), subjectValue))))

def pattern_044():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    BLANK = CellMatchCondition(CellPredicate.blank())
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    SAME_CELL = ItemFilterConditionSpec.same_cell()
    commaPair = CompoundContentSpec.of(AtomicContentSpec.val(), (",", AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_CELL, UNBOUNDED)))))
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBROW, 1)))), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val()), CellPattern.of(BLANK, Quantifier.one(), None)), RowPattern.of(Quantifier.zero_or_one(), CellPattern.of(BLANK, Quantifier.exactly(2), None), CellPattern.of(NOT_BLANK, Quantifier.one(), commaPair))))

def pattern_045():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    SAME_SUBROW_COL0 = ItemFilterConditionSpec.and_(FilterTerm.same_subrow(), FilterTerm.col_exact(0))
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val()), CellPattern.of(NOT_BLANK, Quantifier.one(), DelimitedContentSpec(",", AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBROW_COL0, 1), anchor_pos=1)))))))

def pattern_046():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    BELOW_STR = ItemFilterConditionSpec.and_(FilterTerm.below(), FilterTerm.same_str())
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(ActionSpec.avp(""), ActionSpec.rec(ProviderSpec.val(SAME_SUBROW, UNBOUNDED)), ActionSpec.join(ProviderSpec.val(BELOW_STR, UNBOUNDED), key_positions=0))), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.attr()), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_SUBROW)))))))

def pattern_047():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    BELOW_STR = ItemFilterConditionSpec.and_(FilterTerm.below(), FilterTerm.same_str())
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBROW, UNBOUNDED)), ActionSpec.join(ProviderSpec.val(BELOW_STR, UNBOUNDED), key_positions=0))), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val()))))

def pattern_048():
    BLANK = CellMatchCondition(CellPredicate.blank())
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    SAME_SUBTABLE_COL1 = ItemFilterConditionSpec.and_(FilterTerm.same_subtable(), FilterTerm.col_exact(1))
    SAME_CELL = ItemFilterConditionSpec.same_cell()
    telFaxSpec = CompoundContentSpec.of(AtomicContentSpec.attr(), (":", AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_CELL)))))
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.exactly(2), CellPattern.skip(Quantifier.exactly(2)))), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(ActionSpec.avp(""), ActionSpec.rec(ProviderSpec.val(SAME_SUBTABLE_COL1, UNBOUNDED)))), CellPattern.of(NOT_BLANK, Quantifier.one(), telFaxSpec)), RowPattern.of(CellPattern.of(BLANK, Quantifier.one(), None), CellPattern.of(NOT_BLANK, Quantifier.one(), telFaxSpec)), RowPattern.of(Quantifier.zero_or_one(), CellPattern.of(BLANK, Quantifier.exactly(2), None))))

def pattern_049():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    SAME_SUBCOLUMN = ItemFilterConditionSpec.same_subcol()
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(CellPattern.skip(Quantifier.one()), CellPattern.of(NOT_BLANK, Quantifier.one_or_more(), AtomicContentSpec.val())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val()), CellPattern.of(NOT_BLANK, Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBROW, 1), ProviderSpec.val(SAME_SUBCOLUMN, 1), anchor_pos=2))))))

def pattern_050():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    BELOW_STR = ItemFilterConditionSpec.and_(FilterTerm.below(), FilterTerm.same_str())
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(ActionSpec.avp(""), ActionSpec.rec(ProviderSpec.val(SAME_SUBROW, UNBOUNDED)), ActionSpec.join(ProviderSpec.val(BELOW_STR, UNBOUNDED), key_positions=0))), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.attr()), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_SUBROW)))))))

def pattern_051():
    SAME_COL = ItemFilterConditionSpec.same_col()
    SAME_ROW = ItemFilterConditionSpec.same_row()
    SAME_CELL = ItemFilterConditionSpec.same_cell()
    dataCell = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_COL, 1), ProviderSpec.val(SAME_ROW, 1), ProviderSpec.val(SAME_CELL, 1)), ActionSpec.avp("ND")), (" ", AtomicContentSpec.val(ActionSpec.avp("MON"))))
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("AIRLINE"))))), SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("AIRPORT"))), CellPattern.of(Quantifier.one_or_more(), dataCell))))

def pattern_052():
    SAME_COL = ItemFilterConditionSpec.same_col()
    SAME_ROW = ItemFilterConditionSpec.same_row()
    SAME_CELL = ItemFilterConditionSpec.same_cell()
    dataCell = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_COL, 1), ProviderSpec.val(SAME_ROW, 1), ProviderSpec.val(SAME_CELL, 1), ProviderSpec.ctx_avp("YEAR", "2025")), ActionSpec.avp("ND")), (" ", AtomicContentSpec.val(ActionSpec.avp("MON"))))
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("AIRLINE"))))), SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("AIRPORT"))), CellPattern.of(Quantifier.one_or_more(), dataCell))))

def pattern_053():
    SAME_ROW = ItemFilterConditionSpec.same_row()
    BELOW_STR = ItemFilterConditionSpec.and_(FilterTerm.below(), FilterTerm.same_str())
    ABOVE = ItemFilterConditionSpec.above()
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.aux())), RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_ROW, UNBOUNDED)), ActionSpec.join(ProviderSpec.val(BELOW_STR, 1), key_positions=0), ActionSpec.avp("ID")))), SubrowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.attr(ActionSpec.prefix("_", ProviderSpec.any(ABOVE, 1)))), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_SUBROW))))))))

def pattern_054():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    BLANK = CellMatchCondition(CellPredicate.blank())
    SAME_SUBCOL = ItemFilterConditionSpec.same_subcol()
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one(), SubrowPattern.of(Quantifier.one_or_more(), CellPattern.skip(), CellPattern.of(NOT_BLANK, Quantifier.one_or_more(), AtomicContentSpec.val()), CellPattern.of(BLANK, Quantifier.zero_or_one(), None))), RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val()), CellPattern.of(NOT_BLANK, Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBCOL), ProviderSpec.val(SAME_SUBROW)))), CellPattern.of(BLANK, Quantifier.zero_or_one(), None)))))

def pattern_055():
    SAME_CELL = ItemFilterConditionSpec.same_cell()
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(CompoundContentSpec([("", AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_CELL, UNBOUNDED)))), (",", DelimitedContentSpec(",", AtomicContentSpec.val()))], "")))))

def pattern_056():
    BELOW = ItemFilterConditionSpec.below()
    SAME_ROW = ItemFilterConditionSpec.same_row()
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(CellPattern.of(AtomicContentSpec.attr()), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(BELOW, UNBOUNDED)), ActionSpec.avp(ProviderSpec.attr(SAME_ROW))))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.attr()), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_ROW)))))))

def pattern_057():
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    TRIM = StringExtractor.trimmed()
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(RIGHT_OF)))), CellPattern.of(CompoundContentSpec([("", AtomicContentSpec.val(extractor=TRIM)), ("-", AtomicContentSpec.val(extractor=TRIM))], "")))))

def pattern_058():
    SAME_CELL = ItemFilterConditionSpec.same_cell()
    TRIM = StringExtractor.trimmed()
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(Quantifier.one_or_more(), CompoundContentSpec([("", AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_CELL)), extractor=TRIM)), ("=", AtomicContentSpec.val(extractor=TRIM))], "")))))

def pattern_059():
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(RIGHT_OF)))), CellPattern.of(AtomicContentSpec.val(ActionSpec.suffix(", ", ProviderSpec.any(RIGHT_OF, UNBOUNDED)))), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.aux()))))

def pattern_060():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    BLANK = CellMatchCondition(CellPredicate.blank())
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    SAME_COL = ItemFilterConditionSpec.same_col()
    ABOVE_NBLANK = ItemFilterConditionSpec.and_(FilterTerm.above(), FilterTerm.not_blank())
    colAvp = ActionSpec.avp(ProviderSpec.attr(SAME_COL))
    fill = ActionSpec.fill("", ProviderSpec.any(ABOVE_NBLANK, 1, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR))
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr()))), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(colAvp, ActionSpec.rec(ProviderSpec.val(RIGHT_OF, UNBOUNDED)))), CellPattern.of(Quantifier.one_or_more(), ConditionalContentSpec(BLANK, AtomicContentSpec.val(colAvp, fill), AtomicContentSpec.val(colAvp)))), RowPattern.of(Quantifier.zero_or_one(), CellPattern.of(BLANK, Quantifier.one_or_more(), None))))

def pattern_061():
    SAME_CELL = ItemFilterConditionSpec.same_cell()
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(Quantifier.one_or_more(), CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_CELL, UNBOUNDED)), ActionSpec.avp("A")), (" ", AtomicContentSpec.val(ActionSpec.avp("B"))), (" ", AtomicContentSpec.val(ActionSpec.avp("N"))))))))

def pattern_062():
    NOT_X = CellMatchCondition(CellPredicate.not_regex_matched("x"))
    MATCH_X = CellMatchCondition(CellPredicate.regex_matched("x"))
    ABOVE = ItemFilterConditionSpec.above()
    LEFT_OF = ItemFilterConditionSpec.left_of()
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val())), RowPattern.of(NOT_X, Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val()), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(ABOVE), ProviderSpec.val(LEFT_OF))))), RowPattern.of(Quantifier.zero_or_one(), CellPattern.of(MATCH_X, Quantifier.one_or_more(), None))))

def pattern_063():
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    SAME_COL = ItemFilterConditionSpec.same_col()
    revColAvp = ActionSpec.avp(ProviderSpec.attr(SAME_COL, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR))
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(revColAvp, ActionSpec.rec(ProviderSpec.val(RIGHT_OF, UNBOUNDED)))), CellPattern.of(Quantifier.zero_or_more(), AtomicContentSpec.val(revColAvp))), RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr()))))

def pattern_064():
    BELOW = ItemFilterConditionSpec.below()
    SAME_ROW = ItemFilterConditionSpec.same_row()
    revRowAvp = ActionSpec.avp(ProviderSpec.attr(SAME_ROW, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR))
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(BELOW, UNBOUNDED)), revRowAvp)), CellPattern.of(AtomicContentSpec.attr())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(revRowAvp)), CellPattern.of(AtomicContentSpec.attr()))))

def pattern_065():
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.suffix(", ", ProviderSpec.any(RIGHT_OF, 1)), ActionSpec.rec(ProviderSpec.val(RIGHT_OF, 1)))), CellPattern.of(CompoundContentSpec.of(AtomicContentSpec.aux(), (", ", AtomicContentSpec.val()))))))

def pattern_066():
    CONTAINS_EQ = CellMatchCondition(CellPredicate.contains("="))
    SAME_CELL = ItemFilterConditionSpec.same_cell()
    TRIM = StringExtractor.trimmed()
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(Quantifier.one_or_more(), ConditionalContentSpec(CONTAINS_EQ, CompoundContentSpec([("", AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_CELL)), extractor=TRIM)), ("=", AtomicContentSpec.val(extractor=TRIM))], ""), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.ctx_val(""))))))))

def pattern_067():
    BLANK = CellMatchCondition(CellPredicate.blank())
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    SAME_COL = ItemFilterConditionSpec.same_col()
    ABOVE_NBLANK = ItemFilterConditionSpec.and_(FilterTerm.above(), FilterTerm.not_blank())
    colAvp = ActionSpec.avp(ProviderSpec.attr(SAME_COL))
    rec = ActionSpec.rec(ProviderSpec.val(RIGHT_OF, UNBOUNDED))
    fill = ActionSpec.fill("", ProviderSpec.any(ABOVE_NBLANK, 1, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR))
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(ConditionalContentSpec(BLANK, AtomicContentSpec.val(colAvp, rec, fill), AtomicContentSpec.val(colAvp, rec))), CellPattern.of(Quantifier.one_or_more(), ConditionalContentSpec(BLANK, AtomicContentSpec.val(colAvp, fill), AtomicContentSpec.val(colAvp))))))

def pattern_068():
    BLANK = CellMatchCondition(CellPredicate.blank())
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    COL_HEAD = ItemFilterConditionSpec.and_(FilterTerm.same_col(), FilterTerm.tagged("#HEAD"))
    SAME_ROW = ItemFilterConditionSpec.same_row()
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(BLANK, Quantifier.one(), None), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val_tagged("#HEAD"))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val()), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(COL_HEAD, UNBOUNDED), ProviderSpec.val(SAME_ROW)))))))

def pattern_069():
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    BELOW = ItemFilterConditionSpec.below()
    ROW_TAG1 = ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.tagged("#1"))
    ROW_TAG2 = ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.tagged("#2"))
    avpSR = ActionSpec.avp(ProviderSpec.attr(SAME_SUBROW))
    recBW = ActionSpec.rec(ProviderSpec.val(BELOW, UNBOUNDED))
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one(), SubrowPattern.of(Quantifier.zero_or_more(), CellPattern.of(AtomicContentSpec.attr()), CellPattern.of(AtomicContentSpec.val_tagged("#1", avpSR, recBW, ActionSpec.join(ProviderSpec.val(ROW_TAG1, UNBOUNDED)))), CellPattern.of(AtomicContentSpec.val_tagged("#2", avpSR, recBW, ActionSpec.join(ProviderSpec.val(ROW_TAG2, UNBOUNDED)))))), RowPattern.of(Quantifier.zero_or_more(), SubrowPattern.of(Quantifier.zero_or_more(), CellPattern.of(AtomicContentSpec.attr()), CellPattern.of(Quantifier.exactly(2), AtomicContentSpec.val(avpSR))))))

def pattern_070():
    BLANK = CellMatchCondition(CellPredicate.blank())
    NOT_DIGIT = CellMatchCondition(CellPredicate.not_regex_matched("\\d+"))
    DIGIT = CellMatchCondition(CellPredicate.regex_matched("\\d+"))
    COL_H = ItemFilterConditionSpec.and_(FilterTerm.same_col(), FilterTerm.tagged("#H"))
    ROW_S = ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.tagged("#S"))
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(BLANK, Quantifier.one_or_more(), None), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val_tagged("#H"))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(NOT_DIGIT, Quantifier.one_or_more(), AtomicContentSpec.val_tagged("#S")), CellPattern.of(DIGIT, Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(COL_H, UNBOUNDED), ProviderSpec.val(ROW_S, UNBOUNDED)))))))

def pattern_071():
    BLANK = CellMatchCondition(CellPredicate.blank())
    NOT_DIGIT = CellMatchCondition(CellPredicate.not_regex_matched("\\d+"))
    DIGIT = CellMatchCondition(CellPredicate.regex_matched("\\d+"))
    BW_H = ItemFilterConditionSpec.and_(FilterTerm.below(), FilterTerm.tagged("#H"))
    RT_S = ItemFilterConditionSpec.and_(FilterTerm.right_of(), FilterTerm.tagged("#S"))
    SAME_COL = ItemFilterConditionSpec.same_col()
    SAME_ROW = ItemFilterConditionSpec.same_row()
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(BLANK, Quantifier.one_or_more(), None), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val_tagged("#H", ActionSpec.suffix("/", ProviderSpec.any(BW_H, UNBOUNDED))))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(NOT_DIGIT, Quantifier.one_or_more(), AtomicContentSpec.val_tagged("#S", ActionSpec.suffix("/", ProviderSpec.any(RT_S, UNBOUNDED)))), CellPattern.of(DIGIT, Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_COL), ProviderSpec.val(SAME_ROW)))))))

def pattern_072():
    BLANK = CellMatchCondition(CellPredicate.blank())
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    SAME_COL = ItemFilterConditionSpec.same_col()
    colAvp = ActionSpec.avp(ProviderSpec.attr(SAME_COL))
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(ConditionalContentSpec(BLANK, AtomicContentSpec.skip(), AtomicContentSpec.val(colAvp, ActionSpec.rec(ProviderSpec.val(RIGHT_OF, UNBOUNDED))))), CellPattern.of(Quantifier.one_or_more(), ConditionalContentSpec(BLANK, AtomicContentSpec.skip(), AtomicContentSpec.val(colAvp))))))

def pattern_073():
    BLANK = CellMatchCondition(CellPredicate.blank())
    SAME_COL = ItemFilterConditionSpec.same_col()
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(Quantifier.one_or_more(), ConditionalContentSpec(BLANK, AtomicContentSpec.skip(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_COL))))))))

def pattern_074():
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    SAME_COL = ItemFilterConditionSpec.same_col()
    colAvp = ActionSpec.avp(ProviderSpec.attr(SAME_COL))
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(CellPattern.of(Quantifier.exactly(3), AtomicContentSpec.attr())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(colAvp, ActionSpec.rec(ProviderSpec.val(RIGHT_OF, UNBOUNDED), ProviderSpec.ctx_avp("D", "d")))), CellPattern.of(Quantifier.exactly(2), AtomicContentSpec.val(colAvp)))))

def pattern_075():
    BLANK = CellMatchCondition(CellPredicate.blank())
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    LT_NBLANK = ItemFilterConditionSpec.and_(FilterTerm.left_of(), FilterTerm.not_blank())
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(RIGHT_OF, UNBOUNDED)))), CellPattern.of(Quantifier.one_or_more(), ConditionalContentSpec(BLANK, AtomicContentSpec.val(ActionSpec.fill("", ProviderSpec.any(LT_NBLANK, 1, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR))), AtomicContentSpec.val())))))

def pattern_076():
    BLANK = CellMatchCondition(CellPredicate.blank())
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    TRIM = StringExtractor.trimmed()
    SAME_COL = ItemFilterConditionSpec.same_col()
    SAME_ROW = ItemFilterConditionSpec.same_row()
    ST_COL0 = ItemFilterConditionSpec.and_(FilterTerm.same_subtable(), FilterTerm.col_exact(0))
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val()))), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val()), CellPattern.of(BLANK, Quantifier.one_or_more(), None)), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(extractor=TRIM)), CellPattern.of(NOT_BLANK, Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_COL), ProviderSpec.val(ST_COL0), ProviderSpec.val(SAME_ROW)))))))

def pattern_077():
    BW_R2 = ItemFilterConditionSpec.and_(FilterTerm.below(), FilterTerm.row_offset(2))
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.exactly(2), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(BW_R2))))), RowPattern.of(Quantifier.exactly(2), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val()))))

def pattern_078():
    BELOW = ItemFilterConditionSpec.below()
    SAME_ROW = ItemFilterConditionSpec.same_row()
    rowAvp = ActionSpec.avp(ProviderSpec.attr(SAME_ROW))
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(CellPattern.of(AtomicContentSpec.attr()), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(rowAvp, ActionSpec.rec(ProviderSpec.val(BELOW, UNBOUNDED))))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.attr()), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(rowAvp)))))

def pattern_079():
    BELOW = ItemFilterConditionSpec.below()
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(BELOW, UNBOUNDED))))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val()))))

def pattern_080():
    SAME_COL = ItemFilterConditionSpec.same_col()
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_COL), anchor_pos=1))))))

def pattern_081():
    BELOW = ItemFilterConditionSpec.below()
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(BELOW))))), RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val()))))

def pattern_082():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    BLANK = CellMatchCondition(CellPredicate.blank())
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    SAME_SC = ItemFilterConditionSpec.same_subcol()
    scAvp = ActionSpec.avp(ProviderSpec.attr(SAME_SC))
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(scAvp, ActionSpec.rec(ProviderSpec.val(RIGHT_OF, UNBOUNDED)))), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(scAvp))), RowPattern.of(Quantifier.zero_or_one(), CellPattern.of(BLANK, Quantifier.one_or_more(), None))))

def pattern_083():
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    BELOW = ItemFilterConditionSpec.below()
    SAME_COL = ItemFilterConditionSpec.same_col()
    colAvp = ActionSpec.avp(ProviderSpec.attr(SAME_COL))
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr()))), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val(colAvp, ActionSpec.rec(ProviderSpec.val(RIGHT_OF, UNBOUNDED), ProviderSpec.val(BELOW, 1)))), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(colAvp))), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("D"))), CellPattern.skip(Quantifier.one_or_more()))))

def pattern_084():
    SAME_ROW = ItemFilterConditionSpec.same_row()
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(Quantifier.one(), CellPattern.of(AtomicContentSpec.val())), SubrowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_ROW), ProviderSpec.val(RIGHT_OF), anchor_pos=1))), CellPattern.of(AtomicContentSpec.val())))))

def pattern_085():
    R2_C1 = ItemFilterConditionSpec.and_(FilterTerm.row_exact(2), FilterTerm.col_exact(1))
    R0_C2 = ItemFilterConditionSpec.and_(FilterTerm.row_exact(0), FilterTerm.col_exact(2))
    R0_C1 = ItemFilterConditionSpec.and_(FilterTerm.row_exact(0), FilterTerm.col_exact(1))
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(R2_C1)))), CellPattern.of(AtomicContentSpec.val()), CellPattern.of(AtomicContentSpec.val())), RowPattern.of(CellPattern.skip(), CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(R0_C2)))), CellPattern.skip()), RowPattern.of(CellPattern.skip(), CellPattern.of(AtomicContentSpec.val()), CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(R0_C1)))))))

def pattern_086():
    SAME_COL = ItemFilterConditionSpec.same_col()
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    R1_C2 = ItemFilterConditionSpec.and_(FilterTerm.row_offset(1), FilterTerm.col_exact(2))
    colAvp = ActionSpec.avp(ProviderSpec.attr(SAME_COL))
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(CellPattern.of(Quantifier.exactly(3), AtomicContentSpec.attr()))), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val(colAvp, ActionSpec.rec(ProviderSpec.val(RIGHT_OF, UNBOUNDED), ProviderSpec.val(R1_C2)))), CellPattern.of(Quantifier.exactly(2), AtomicContentSpec.val(colAvp))), RowPattern.of(CellPattern.skip(Quantifier.exactly(2)), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("CtxAttr"))))))

def pattern_087():
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(RIGHT_OF, UNBOUNDED)))), CellPattern.of(Quantifier.exactly(2), AtomicContentSpec.val())))))

def pattern_088():
    BLANK_COND = CellMatchCondition(CellPredicate.blank())
    NOT_BLANK_COND = CellMatchCondition(CellPredicate.not_blank())
    BELOW = ItemFilterConditionSpec.below()
    SAME_ROW = ItemFilterConditionSpec.same_row()
    substr = StringExtractor.substring(4, 5)
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one(), SubrowPattern.of(CellPattern.of(AtomicContentSpec.attr(extractor=substr))), SubrowPattern.of(Quantifier.one_or_more(), CellPattern(NOT_BLANK_COND, Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(BELOW, UNBOUNDED)), ActionSpec.avp(ProviderSpec.attr(SAME_ROW)))), CellPattern(BLANK_COND, Quantifier.zero_or_one(), None))), RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(CellPattern.of(AtomicContentSpec.attr(extractor=substr))), SubrowPattern.of(Quantifier.one_or_more(), CellPattern(NOT_BLANK_COND, Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_ROW)))), CellPattern(BLANK_COND, Quantifier.zero_or_one(), None)))))

def pattern_089():
    BLANK_COND = CellMatchCondition(CellPredicate.blank())
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    SAME_COL = ItemFilterConditionSpec.same_col()
    colAvp = ActionSpec.avp(ProviderSpec.attr(SAME_COL))
    rec = ActionSpec.rec(ProviderSpec.val(RIGHT_OF, UNBOUNDED))
    attrPlus = CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr())
    valAnchor = CellPattern.of(AtomicContentSpec.val(rec, colAvp))
    valContPlus = CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(colAvp))
    blankPlus = CellPattern(BLANK_COND, Quantifier.one_or_more(), None)
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(attrPlus)), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(valAnchor, valContPlus), RowPattern.of(blankPlus)), SubtablePattern.of(Quantifier.one(), RowPattern.of(attrPlus), RowPattern.of(Quantifier.one_or_more(), valAnchor, valContPlus)))

def pattern_090():
    BELOW = ItemFilterConditionSpec.below()
    extractor = StringExtractor.replaced("\\*", "")
    suffix = ActionSpec.suffix("/", ProviderSpec.any(BELOW, UNBOUNDED))
    rec = ActionSpec.rec()
    valPlus = CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(suffix, rec, extractor=extractor))
    auxPlus = CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.aux())
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one(), SubrowPattern.of(valPlus)), RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(auxPlus))))

def pattern_091():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    BLANK = CellMatchCondition(CellPredicate.blank())
    LEFT_OF = ItemFilterConditionSpec.left_of()
    SAME_ROW = ItemFilterConditionSpec.same_row()
    ltAvp = ActionSpec.avp(ProviderSpec.attr(LEFT_OF, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR))
    rowRec = ActionSpec.rec(ProviderSpec.val(SAME_ROW, UNBOUNDED))
    notBlankAttr = CellPattern(NOT_BLANK, Quantifier.one(), AtomicContentSpec.attr())
    notBlankValAnchor = CellPattern(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(rowRec, ltAvp))
    notBlankVal = CellPattern(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(ltAvp))
    blankCell = CellPattern(BLANK, Quantifier.one_or_more(), None)
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(notBlankAttr, notBlankValAnchor), SubrowPattern.of(Quantifier.one_or_more(), notBlankAttr, notBlankVal)), RowPattern.of(Quantifier.zero_or_one(), blankCell)))

def pattern_092():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    BLANK = CellMatchCondition(CellPredicate.blank())
    rowAndColRight = ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.col_range(1, UNBOUNDED))
    rowRight = ProviderSpec.val(rowAndColRight, UNBOUNDED)
    leftOf = ProviderSpec.val(ItemFilterConditionSpec.left_of())
    rec = ActionSpec.rec(rowRight, leftOf)
    plainVal = CellPattern(None, Quantifier.one(), AtomicContentSpec.val())
    recVal = CellPattern(None, Quantifier.one(), AtomicContentSpec.val(rec))
    notBlankVal = CellPattern(NOT_BLANK, Quantifier.one_or_more(), AtomicContentSpec.val())
    blankCell = CellPattern(BLANK, Quantifier.zero_or_one(), None)
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(plainVal, recVal), SubrowPattern.of(Quantifier.one_or_more(), notBlankVal, blankCell))))

def pattern_093():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    BLANK = CellMatchCondition(CellPredicate.blank())
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    SAME_SC = ItemFilterConditionSpec.same_subcol()
    scAvp = ActionSpec.avp(ProviderSpec.attr(SAME_SC))
    rtRec = ActionSpec.rec(ProviderSpec.val(RIGHT_OF, UNBOUNDED))
    notBlankAttr = CellPattern.of(NOT_BLANK, Quantifier.one_or_more(), AtomicContentSpec.attr())
    optionalBlank = CellPattern.of(BLANK, Quantifier.zero_or_one(), None)
    anchorVal = CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(scAvp, rtRec))
    otherVals = CellPattern.of(NOT_BLANK, Quantifier.one_or_more(), AtomicContentSpec.val(scAvp))
    manyBlanks = CellPattern.of(BLANK, Quantifier.one_or_more(), None)
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(Quantifier.one(), SubrowPattern.of(Quantifier.one_or_more(), notBlankAttr, optionalBlank)), RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(Quantifier.one_or_more(), anchorVal, otherVals, optionalBlank)), RowPattern.of(Quantifier.zero_or_one(), manyBlanks)))

def pattern_094():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    BLANK = CellMatchCondition(CellPredicate.blank())
    sameCol = ItemFilterConditionSpec.same_col()
    rowColRightStr = ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.col_range(1, UNBOUNDED), FilterTerm.same_str())
    colRec = ActionSpec.rec(ProviderSpec.val(sameCol, UNBOUNDED))
    rowJoin = ActionSpec.join(ProviderSpec.val(rowColRightStr, UNBOUNDED), key_positions=0)
    headerCell = CellPattern.of(NOT_BLANK, Quantifier.one_or_more(), AtomicContentSpec.val(colRec, rowJoin))
    optBlank = CellPattern.of(BLANK, Quantifier.zero_or_one(), None)
    dataCell = CellPattern.of(NOT_BLANK, Quantifier.one_or_more(), AtomicContentSpec.val())
    manyBlanks = CellPattern.of(BLANK, Quantifier.one_or_more(), None)
    headerSubrow = SubrowPattern.of(Quantifier.one_or_more(), headerCell, optBlank)
    dataSubrow = SubrowPattern.of(Quantifier.one_or_more(), dataCell, optBlank)
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one(), headerSubrow)), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(Quantifier.one_or_more(), dataSubrow), RowPattern.of(Quantifier.zero_or_one(), manyBlanks)))

def pattern_095():
    BELOW = ItemFilterConditionSpec.below()
    SAME_CELL = ItemFilterConditionSpec.same_cell()
    headerCell = CompoundContentSpec.of(AtomicContentSpec.attr(), ("=", AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(BELOW, UNBOUNDED)).as_inherited(), ActionSpec.avp(ProviderSpec.attr(SAME_CELL)).as_inherited())))
    dataCell = CompoundContentSpec.of(AtomicContentSpec.attr(), ("=", AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_CELL)).as_inherited())))
    return TablePattern.of(SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(Quantifier.one_or_more(), headerCell)), RowPattern.of(Quantifier.exactly(2), CellPattern.of(Quantifier.one_or_more(), dataCell))))

def pattern_096():
    SAME_COL = ItemFilterConditionSpec.same_col()
    SAME_SUBTABLE = ItemFilterConditionSpec.same_subtable()
    COL_AND_R1 = ItemFilterConditionSpec.and_(FilterTerm.same_col(), FilterTerm.row_exact(1))
    recCell = AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_SUBTABLE, UNBOUNDED)), ActionSpec.avp(ProviderSpec.attr(SAME_COL)).as_inherited())
    avpCell = AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_COL)).as_inherited())
    r1Cell = AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(COL_AND_R1)))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(Quantifier.exactly(2), AtomicContentSpec.attr())), RowPattern.of(CellPattern.skip(), CellPattern.of(AtomicContentSpec.attr()))), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(recCell), CellPattern.of(avpCell)), RowPattern.of(CellPattern.skip(), CellPattern.of(r1Cell))))

def pattern_097():
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    BELOW_STR = ItemFilterConditionSpec.and_(FilterTerm.below(), FilterTerm.same_str())
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(RIGHT_OF, UNBOUNDED)), ActionSpec.join(ProviderSpec.val(BELOW_STR, UNBOUNDED), key_positions={0, 1}))), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val()))))

def pattern_098():
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    BELOW_STR = ItemFilterConditionSpec.and_(FilterTerm.below(), FilterTerm.same_str())
    SAME_COL = ItemFilterConditionSpec.same_col()
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(), CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(RIGHT_OF, UNBOUNDED)), ActionSpec.join(ProviderSpec.val(BELOW_STR, UNBOUNDED), key_positions={0, 1}))), CellPattern.of(AtomicContentSpec.val()), CellPattern.of(Quantifier.exactly(2), AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_COL)))), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val()))))

def pattern_099():
    CL_P0 = ItemFilterConditionSpec.and_(FilterTerm.same_cell(), FilterTerm.pos_exact(0))
    CL_P2 = ItemFilterConditionSpec.and_(FilterTerm.same_cell(), FilterTerm.pos_exact(2))
    CL_P4 = ItemFilterConditionSpec.and_(FilterTerm.same_cell(), FilterTerm.pos_exact(4))
    SAME_CELL = ItemFilterConditionSpec.same_cell()
    cellSpec = CompoundContentSpec.of(AtomicContentSpec.attr(), ("=", AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(CL_P0)), ActionSpec.rec(ProviderSpec.val(SAME_CELL, UNBOUNDED)))), ("\r\n", AtomicContentSpec.attr()), ("=", AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(CL_P2)))), ("\r\n", AtomicContentSpec.attr()), ("=", AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(CL_P4)))))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), CellPattern.of(Quantifier.one_or_more(), cellSpec))))

def pattern_100():
    RT_P0 = ItemFilterConditionSpec.and_(FilterTerm.right_of(), FilterTerm.pos_exact(0))
    RT_P1 = ItemFilterConditionSpec.and_(FilterTerm.right_of(), FilterTerm.pos_exact(1))
    RT_P2 = ItemFilterConditionSpec.and_(FilterTerm.right_of(), FilterTerm.pos_exact(2))
    anchorSpec = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(RT_P0, UNBOUNDED))), ("\\n", AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(RT_P1, UNBOUNDED)))), ("\\n", AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(RT_P2, UNBOUNDED)))))
    rightSpec = CompoundContentSpec.of(AtomicContentSpec.val(), ("\\n", AtomicContentSpec.val()), ("\\n", AtomicContentSpec.val()))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), CellPattern.of(anchorSpec), CellPattern.of(Quantifier.exactly(2), rightSpec))))

def pattern_101():
    SAME_CELL = ItemFilterConditionSpec.same_cell()
    cellSpec = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(SAME_CELL, UNBOUNDED))), ("\t", DelimitedContentSpec("\t", AtomicContentSpec.val())))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), CellPattern.of(Quantifier.one_or_more(), cellSpec))))

def pattern_102():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    BLANK = CellMatchCondition(CellPredicate.blank())
    SAME_COL = ItemFilterConditionSpec.same_col()
    SAME_SUBTABLE = ItemFilterConditionSpec.same_subtable()
    SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
    condAttr = ConditionalContentSpec(BLANK, AtomicContentSpec.skip(), AtomicContentSpec.attr())
    condVal = ConditionalContentSpec(BLANK, AtomicContentSpec.skip(), AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_SUBROW))))
    attrValSubrow = SubrowPattern.of(Quantifier.one_or_more(), CellPattern.of(condAttr), CellPattern.of(condVal))
    firstRow = RowPattern(None, Quantifier.one(), [SubrowPattern.of(CellPattern(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.attr(SAME_COL)), ActionSpec.rec(ProviderSpec.val(SAME_SUBTABLE, UNBOUNDED))))), attrValSubrow])
    contRow = RowPattern(None, Quantifier.one_or_more(), [SubrowPattern.of(CellPattern(BLANK, Quantifier.one(), None)), attrValSubrow])
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(AtomicContentSpec.attr()), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.skip()))), SubtablePattern(None, Quantifier.one_or_more(), [firstRow, contRow]))

def pattern_103():
    SAME_COL = ItemFilterConditionSpec.same_col()
    SAME_SUBTABLE = ItemFilterConditionSpec.same_subtable()
    stAvp = ActionSpec.avp(ProviderSpec.attr(SAME_SUBTABLE))
    colRec = ActionSpec.rec(ProviderSpec.val(SAME_COL, UNBOUNDED))
    anchorVal = AtomicContentSpec.val(colRec, stAvp)
    dataVal = AtomicContentSpec.val(stAvp)
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr())), RowPattern.of(CellPattern.of(Quantifier.one_or_more(), anchorVal))), SubtablePattern(None, Quantifier.one_or_more(), [RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr())), RowPattern.of(CellPattern.of(Quantifier.one_or_more(), dataVal))]))

def pattern_104():
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    suffixRt = ActionSpec.suffix(" ", ProviderSpec.any(RIGHT_OF, UNBOUNDED))
    emptyRec = ActionSpec.rec()
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(suffixRt, emptyRec)), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.aux()))))

def pattern_105():
    TABLE_COND = CellMatchCondition(CellPredicate.regex_matched("\\d+"))
    NOT_SAME_CELL = ItemFilterConditionSpec.bare(FilterTerm.not_same_cell())
    subtable = SubtablePattern.of(RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.rec(ProviderSpec.val(NOT_SAME_CELL, UNBOUNDED)))), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val())))
    return TablePattern(TABLE_COND, [subtable], [])

def pattern_106():
    SAME_CELL = ItemFilterConditionSpec.same_cell()
    SAME_ROW = ItemFilterConditionSpec.same_row()
    SAME_COL = ItemFilterConditionSpec.same_col()
    monAvp = ActionSpec.avp(ProviderSpec.ctx_attr("MON"))
    indicatorUnit = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.ctx_attr("INDICATOR"))), (",", AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.ctx_attr("UNIT")), extractor=StringExtractor.trimmed())))
    minMaxAve = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.ctx_attr("MIN")), ActionSpec.rec(ProviderSpec.val(SAME_CELL, UNBOUNDED), ProviderSpec.val(SAME_ROW, 2), ProviderSpec.val(SAME_COL))), ("-", AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.ctx_attr("MAX")))), ("/", AtomicContentSpec.val(ActionSpec.avp(ProviderSpec.ctx_attr("AVE")))))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(AtomicContentSpec.skip()), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(monAvp))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(indicatorUnit), CellPattern.of(Quantifier.one_or_more(), minMaxAve))))

def pattern_107():
    BLANK = CellMatchCondition(CellPredicate.blank())
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    NON_DIGIT = CellMatchCondition(CellPredicate.regex_matched("\\D.*"))
    DIGIT = CellMatchCondition(CellPredicate.regex_matched("\\d+"))
    LT_NOT_BLANK = ItemFilterConditionSpec.and_(FilterTerm.left_of(), FilterTerm.not_blank())
    COL_H = ItemFilterConditionSpec.and_(FilterTerm.same_col(), FilterTerm.tagged("#H"))
    ROW_S = ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.tagged("#S"))
    SAME_SUBCOL = ItemFilterConditionSpec.same_subcol()
    fillReverse = ActionSpec.fill("", ProviderSpec.any(LT_NOT_BLANK, 1, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR))
    recColRow = ActionSpec.rec(ProviderSpec.val(COL_H, UNBOUNDED), ProviderSpec.val(ROW_S, UNBOUNDED))
    scFill = ActionSpec.fill("", ProviderSpec.any(SAME_SUBCOL, 1))
    condH = ConditionalContentSpec(BLANK, AtomicContentSpec.val_tagged("#H", fillReverse), AtomicContentSpec.val_tagged("#H"))
    return TablePattern.of(SubtablePattern.of(Quantifier.one(), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(BLANK, Quantifier.one_or_more(), None), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val_tagged("#H")), CellPattern.of(Quantifier.one_or_more(), condH))), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(Quantifier.one(), CellPattern.of(NON_DIGIT, Quantifier.one_or_more(), AtomicContentSpec.val_tagged("#S")), CellPattern.of(DIGIT, Quantifier.one_or_more(), AtomicContentSpec.val(recColRow))), RowPattern.of(Quantifier.zero_or_more(), CellPattern.of(BLANK, Quantifier.one_or_more(), AtomicContentSpec.val_tagged("#S", scFill)), CellPattern.of(NON_DIGIT, Quantifier.one_or_more(), AtomicContentSpec.val_tagged("#S")), CellPattern.of(DIGIT, Quantifier.one_or_more(), AtomicContentSpec.val(recColRow)))))

def pattern_108():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    BLANK = CellMatchCondition(CellPredicate.blank())
    SAME_COL = ItemFilterConditionSpec.same_col()
    RIGHT_OF = ItemFilterConditionSpec.right_of()
    colAvp = ActionSpec.avp(ProviderSpec.attr(SAME_COL))
    rtRec = ActionSpec.rec(ProviderSpec.val(RIGHT_OF))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr()))), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(Quantifier.one(), SubrowPattern.of(Quantifier.one_or_more(), CellPattern.of(NOT_BLANK, Quantifier.one(), None), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(colAvp, rtRec)), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(colAvp)), CellPattern.of(BLANK, Quantifier.zero_or_more(), None))), RowPattern.of(Quantifier.zero_or_more(), CellPattern.of(BLANK, Quantifier.one_or_more(), None))))

def pattern_109():
    ROW_ODD = ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.tagged("#ODD"))
    ROW_EVEN = ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.tagged("#EVEN"))
    recOdd = ActionSpec.rec(ProviderSpec.val(ROW_ODD, UNBOUNDED))
    recEven = ActionSpec.rec(ProviderSpec.val(ROW_EVEN, UNBOUNDED))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(CellPattern.of(AtomicContentSpec.val(recOdd)), CellPattern.of(AtomicContentSpec.val(recEven))), SubrowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val_tagged("#ODD")), CellPattern.of(AtomicContentSpec.val_tagged("#EVEN"))))))

def pattern_110():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    BLANK = CellMatchCondition(CellPredicate.blank())
    recRt = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.right_of(), UNBOUNDED))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(Quantifier.one_or_more(), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(recRt)), CellPattern.of(NOT_BLANK, Quantifier.one_or_more(), AtomicContentSpec.val()), CellPattern.of(BLANK, Quantifier.zero_or_one(), None)))))

def pattern_111():
    recRowCol = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), UNBOUNDED), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 1))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(Quantifier.exactly(2)), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("UNIT")))), RowPattern.of(CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("INDICATOR"))), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("YEAR"))), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("MIN"))), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("MAX"))), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("AVE"), recRowCol)))))

def pattern_112():
    BLANK = CellMatchCondition(CellPredicate.blank())
    indicatorUnit = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp("INDICATOR")), (",", AtomicContentSpec.val(ActionSpec.avp("UNIT"), extractor=StringExtractor.trimmed())))
    recAve = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), 3), ProviderSpec.val(ItemFilterConditionSpec.left_of(), 2, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 1))
    aveSpec = ConditionalContentSpec(BLANK, AtomicContentSpec.skip(), AtomicContentSpec.val(ActionSpec.avp("AVE"), recAve))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(Quantifier.exactly(2)), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("LOCATION")))), RowPattern.of(CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(Quantifier.one(), CellPattern.of(indicatorUnit), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("YEAR")))), SubrowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("MIN"))), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("MAX"))), CellPattern.of(aveSpec)))))

def pattern_113():
    substr04 = StringExtractor.substring(0, 4)
    recRow = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), 1))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.exactly(7), AtomicContentSpec.val(ActionSpec.avp("YEAR"), extractor=substr04)), CellPattern.skip(Quantifier.zero_or_more())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("POLLUTANT"))), CellPattern.of(Quantifier.exactly(7), AtomicContentSpec.val(ActionSpec.avp("EMISSION"), recRow)), CellPattern.skip(Quantifier.zero_or_more()))))

def pattern_114():
    BLANK = CellMatchCondition(CellPredicate.blank())
    orgLoc = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp("ORGANIZATION")), (",", AtomicContentSpec.val(ActionSpec.avp("LOCATION"), extractor=StringExtractor.trimmed())))
    rowColRange = ItemFilterConditionSpec.or_(ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.col_absolute_range(0, 2)), ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.col_absolute_range(4, 5)))
    recFuel = ActionSpec.rec(ProviderSpec.val(rowColRange, UNBOUNDED))
    fuelSpec = ConditionalContentSpec(BLANK, AtomicContentSpec.skip(), AtomicContentSpec.val(ActionSpec.avp("FUEL_CONSUMPTION"), recFuel))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.exactly(2), CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(orgLoc), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("YEAR"))), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("FUEL_TYPE"))), CellPattern.of(fuelSpec), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("SULPHUR_CONTENT"))), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("ASH_CONTENT"))), CellPattern.skip(Quantifier.one_or_more()))))

def pattern_115():
    orgLoc = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp("ORGANIZATION")), (",", AtomicContentSpec.val(ActionSpec.avp("LOCATION"), extractor=StringExtractor.trimmed())))
    colR1 = ItemFilterConditionSpec.and_(FilterTerm.same_col(), FilterTerm.row_exact(1))
    recEmission = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), 3), ProviderSpec.val(colR1, 1))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(CellPattern.skip(Quantifier.exactly(7)), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("POLLUTANT"))))), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(orgLoc), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("YEAR"))), CellPattern.skip(Quantifier.exactly(5)), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("EMISSION"), recEmission))), RowPattern.of(CellPattern.skip(Quantifier.one_or_more()))))

def pattern_116():
    YEAR_COND = CellMatchCondition(CellPredicate.regex_matched("20\\d\\d"))
    v1Prefix = ActionSpec.prefix(", ", ProviderSpec.any(ItemFilterConditionSpec.above(), 1, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR))
    locAvp = ActionSpec.avp("LOCATION").as_inherited()
    v1Cell = AtomicContentSpec.val(locAvp, v1Prefix)
    valLocCell = AtomicContentSpec.val(locAvp)
    colR1to3 = ItemFilterConditionSpec.and_(FilterTerm.same_col(), FilterTerm.row_absolute_range(1, 3))
    avInd = ItemFilterConditionSpec.and_(FilterTerm.above(), FilterTerm.tagged("#IND"))
    v2Cell = AtomicContentSpec.val(ActionSpec.avp("VALUE"), ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), 1), ProviderSpec.val(colR1to3, UNBOUNDED), ProviderSpec.val(avInd, 1, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR)))
    indUnit = CompoundContentSpec.of(AtomicContentSpec.val_tagged("#IND", ActionSpec.avp("INDICATOR")), (",", AtomicContentSpec.val(ActionSpec.avp("UNIT"))))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("TERRITORY")))), RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.aux())), RowPattern.of(Quantifier.one(), SubrowPattern.of(Quantifier.one(), CellPattern.skip(), CellPattern.of(Quantifier.exactly(4), v1Cell), CellPattern.of(valLocCell), CellPattern.skip(), CellPattern.of(valLocCell), CellPattern.of(v1Cell), CellPattern.of(valLocCell), CellPattern.of(v1Cell), CellPattern.of(valLocCell), CellPattern.skip()), SubrowPattern.of(Quantifier.zero_or_one(), CellPattern.of(valLocCell), CellPattern.of(v1Cell), CellPattern.of(valLocCell), CellPattern.skip()))), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(Quantifier.one_or_more(), indUnit)), RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(Quantifier.one(), CellPattern.of(YEAR_COND, Quantifier.one(), AtomicContentSpec.val(ActionSpec.avp("YEAR")))), SubrowPattern.of(Quantifier.exactly(2), CellPattern.of(Quantifier.exactly(5), v2Cell), CellPattern.skip()), SubrowPattern.of(Quantifier.zero_or_one(), CellPattern.of(Quantifier.exactly(3), v2Cell), CellPattern.skip()))))

def pattern_117():
    substr04 = StringExtractor.substring(0, 4)
    rowC1 = ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.col_exact(1))
    colR1 = ItemFilterConditionSpec.and_(FilterTerm.same_col(), FilterTerm.row_exact(1))
    recMln = ActionSpec.rec(ProviderSpec.val(rowC1, 1), ProviderSpec.val(colR1, 1), ProviderSpec.ctx_avp("UNIT", "MLN M3"))
    recTons = ActionSpec.rec(ProviderSpec.val(rowC1, 1), ProviderSpec.val(colR1, 1), ProviderSpec.ctx_avp("UNIT", "TONS"))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(CellPattern.skip(), CellPattern.of(AtomicContentSpec.attr()), CellPattern.of(Quantifier.exactly(5), AtomicContentSpec.val(ActionSpec.avp("YEAR"), extractor=substr04)), CellPattern.skip()), RowPattern.of(CellPattern.skip(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("POLLUTANT"))), CellPattern.of(Quantifier.exactly(5), AtomicContentSpec.val(ActionSpec.avp("EMISSION"), recMln)), CellPattern.skip()), RowPattern.of(Quantifier.one_or_more(), CellPattern.skip(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("POLLUTANT"))), CellPattern.of(Quantifier.exactly(5), AtomicContentSpec.val(ActionSpec.avp("EMISSION"), recTons)), CellPattern.skip())))

def pattern_118():
    CONTAINS_NL = CellMatchCondition(CellPredicate.contains("\\n"))
    BLANK = CellMatchCondition(CellPredicate.blank())
    indObs = ConditionalContentSpec(CONTAINS_NL, CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp("INDICATOR")), ("\\n", AtomicContentSpec.val(ActionSpec.avp("OBSERVATION")))), AtomicContentSpec.val(ActionSpec.avp("INDICATOR")))
    recAve = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), 3), ProviderSpec.val(ItemFilterConditionSpec.left_of(), 2, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 1))
    aveSpec = ConditionalContentSpec(BLANK, AtomicContentSpec.skip(), AtomicContentSpec.val(ActionSpec.avp("AVE"), recAve))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(Quantifier.exactly(2)), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("LOCATION")))), RowPattern.of(CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(Quantifier.one(), CellPattern.of(indObs), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("YEAR")))), SubrowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("MIN"))), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("MAX"))), CellPattern.of(aveSpec)))))

def pattern_119():
    BLANK = CellMatchCondition(CellPredicate.blank())
    recAve = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), 2), ProviderSpec.val(ItemFilterConditionSpec.left_of(), 2, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 1))
    aveSpec = ConditionalContentSpec(BLANK, AtomicContentSpec.skip(), AtomicContentSpec.val(ActionSpec.avp("AVE"), recAve))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(Quantifier.exactly(2)), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("LOCATION")))), RowPattern.of(CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(Quantifier.one(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("INDICATOR"))), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("YEAR")))), SubrowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("MIN"))), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("MAX"))), CellPattern.of(aveSpec)))))

def pattern_120():
    indUnit = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp("INDICATOR")), (",", AtomicContentSpec.val(ActionSpec.avp("UNIT"), extractor=StringExtractor.trimmed())))
    recAve = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), 2), ProviderSpec.val(ItemFilterConditionSpec.left_of(), 2, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 1))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("LOCATION")))), RowPattern.of(CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(Quantifier.one(), CellPattern.of(indUnit)), SubrowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("MIN"))), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("MAX"))), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("AVE"), recAve))))))

def pattern_121():
    DASH_OPT = CellMatchCondition(CellPredicate.regex_matched("\\s*-?\\s*"))
    rowC1 = ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.col_exact(1))
    recAve = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_cell(), UNBOUNDED), ProviderSpec.val(rowC1, 1), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 1))
    minMaxAve = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp("MIN")), ("-", AtomicContentSpec.val(ActionSpec.avp("MAX"))), ("\\n", AtomicContentSpec.val(ActionSpec.avp("AVE"), recAve)))
    cellSpec = ConditionalContentSpec(DASH_OPT, AtomicContentSpec.skip(), minMaxAve)
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(Quantifier.exactly(2)), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("MONTH")))), RowPattern.of(Quantifier.one_or_more(), CellPattern.skip(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("INDICATOR"))), CellPattern.of(Quantifier.one_or_more(), cellSpec))))

def pattern_122():
    DASH_OPT = CellMatchCondition(CellPredicate.regex_matched("\\s*-?\\s*"))
    substr04 = StringExtractor.substring(0, 4)
    minMax = ConditionalContentSpec(DASH_OPT, AtomicContentSpec.skip(), CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp("MIN")), ("-", AtomicContentSpec.val(ActionSpec.avp("MAX")))))
    recAve = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), 1), ProviderSpec.val(ItemFilterConditionSpec.above(), 2), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 2))
    aveSpec = ConditionalContentSpec(DASH_OPT, AtomicContentSpec.skip(), AtomicContentSpec.val(ActionSpec.avp("AVE"), recAve))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("YEAR"), extractor=substr04))), RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("MONTH"))))), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.exactly(6), minMax), CellPattern.skip()), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("INDICATOR"))), CellPattern.of(Quantifier.exactly(6), aveSpec), CellPattern.skip())))

def pattern_123():
    substr04 = StringExtractor.substring(0, 4)
    yearMonth = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp("YEAR"), extractor=substr04), ("\\n", AtomicContentSpec.val(ActionSpec.avp("MONTH"))))
    colR1 = ItemFilterConditionSpec.and_(FilterTerm.same_col(), FilterTerm.row_exact(1))
    recAve = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_cell(), UNBOUNDED), ProviderSpec.val(ItemFilterConditionSpec.same_row(), 1), ProviderSpec.val(colR1, UNBOUNDED))
    minMaxAve = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp("MIN")), ("-", AtomicContentSpec.val(ActionSpec.avp("MAX"))), ("\\n", AtomicContentSpec.val(ActionSpec.avp("AVE"), recAve)))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.exactly(4), yearMonth), CellPattern.skip()), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("INDICATOR"))), CellPattern.of(Quantifier.exactly(4), minMaxAve), CellPattern.skip())))

def pattern_124():
    substr04 = StringExtractor.substring(0, 4)
    recAve = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_cell(), UNBOUNDED), ProviderSpec.val(ItemFilterConditionSpec.same_row(), 1), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 2))
    dataCell = CompoundContentSpec([("", AtomicContentSpec.val(ActionSpec.avp("MIN"))), ("-", AtomicContentSpec.val(ActionSpec.avp("MAX"))), ("\\n", AtomicContentSpec.val(ActionSpec.avp("AVE"), recAve, extractor=StringExtractor.trimmed())), ("(", AtomicContentSpec.val(ActionSpec.avp("IN_NORTHWESTERN_SECTION")))], ")")
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.exactly(6), AtomicContentSpec.val(ActionSpec.avp("YEAR"), extractor=substr04)), CellPattern.skip(Quantifier.exactly(2))), RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.exactly(6), AtomicContentSpec.val(ActionSpec.avp("MONTH"))), CellPattern.skip(Quantifier.exactly(2))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("INDICATOR"))), CellPattern.of(Quantifier.exactly(6), dataCell), CellPattern.skip(Quantifier.exactly(2)))))

def pattern_125():
    DASH_OPT = CellMatchCondition(CellPredicate.regex_matched("\\s*-?\\s*"))
    rowC1 = ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.col_exact(1))
    recAve = ActionSpec.rec(ProviderSpec.val(rowC1, 1), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 1))
    aveSpec = ConditionalContentSpec(DASH_OPT, AtomicContentSpec.skip(), AtomicContentSpec.val(ActionSpec.avp("AVE"), recAve))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(Quantifier.exactly(2)), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("MONTH")))), RowPattern.of(Quantifier.one_or_more(), CellPattern.skip(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("INDICATOR"))), CellPattern.of(Quantifier.one_or_more(), aveSpec))))

def pattern_126():
    DASH_OPT = CellMatchCondition(CellPredicate.regex_matched("\\s*-?\\s*"))
    rowC1 = ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.col_exact(1))
    recAve = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_cell(), UNBOUNDED), ProviderSpec.val(rowC1, 1))
    minMaxAve = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp("MIN")), ("-", AtomicContentSpec.val(ActionSpec.avp("MAX"))), ("\\n", AtomicContentSpec.val(ActionSpec.avp("AVE"), recAve)))
    cellSpec = ConditionalContentSpec(DASH_OPT, AtomicContentSpec.skip(), minMaxAve)
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), CellPattern.skip(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("INDICATOR"))), CellPattern.of(Quantifier.one_or_more(), cellSpec))))

def pattern_127():
    DASH_OPT = CellMatchCondition(CellPredicate.regex_matched("\\s*-?\\s*"))
    substr04 = StringExtractor.substring(0, 4)
    indCell = AtomicContentSpec.val(ActionSpec.prefix("", ProviderSpec.any(ItemFilterConditionSpec.above(), 1, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR)), ActionSpec.avp("INDICATOR"))
    hgUnit = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp("HYDROBIONT_GROUP")), (",", AtomicContentSpec.val(ActionSpec.avp("UNIT"), extractor=StringExtractor.trimmed())))
    colR2 = ItemFilterConditionSpec.and_(FilterTerm.same_col(), FilterTerm.row_exact(2))
    rowC5 = ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.col_exact(5))
    rowC6 = ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.col_exact(6))
    rowC10 = ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.col_exact(10))
    recBlock1 = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_cell(), UNBOUNDED), ProviderSpec.val(ItemFilterConditionSpec.same_row(), 3), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 1), ProviderSpec.val(colR2, 1), ProviderSpec.val(rowC5, 1))
    recBlock2 = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_cell(), UNBOUNDED), ProviderSpec.val(ItemFilterConditionSpec.same_row(), 2), ProviderSpec.val(rowC6, 1), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 1), ProviderSpec.val(colR2, 1), ProviderSpec.val(rowC10, 1))
    block1Cell = ConditionalContentSpec(DASH_OPT, AtomicContentSpec.skip(), CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp("MIN")), ("-", AtomicContentSpec.val(ActionSpec.avp("MAX"))), ("\\n", AtomicContentSpec.val(ActionSpec.avp("AVE"), recBlock1))))
    block2Cell = ConditionalContentSpec(DASH_OPT, AtomicContentSpec.skip(), CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp("MIN")), ("-", AtomicContentSpec.val(ActionSpec.avp("MAX"))), ("\\n", AtomicContentSpec.val(ActionSpec.avp("AVE"), recBlock2))))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("YEAR"), extractor=substr04))), RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.aux())), RowPattern.of(Quantifier.one(), SubrowPattern.of(Quantifier.one(), CellPattern.skip()), SubrowPattern.of(Quantifier.one_or_more(), CellPattern.skip(), CellPattern.of(Quantifier.exactly(3), indCell), CellPattern.skip())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(hgUnit), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("TIME"))), CellPattern.of(Quantifier.exactly(3), block1Cell), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("AREA"))), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("TIME"))), CellPattern.of(Quantifier.exactly(3), block2Cell), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("AREA"))))))

def pattern_128():
    DASH_OPT = CellMatchCondition(CellPredicate.regex_matched("\\s*-?\\s*"))
    timeYear = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp("TIME")), (" ", AtomicContentSpec.val(ActionSpec.avp("YEAR"))))
    rowC1 = ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.col_exact(1))
    recAve = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_cell(), UNBOUNDED), ProviderSpec.val(rowC1, UNBOUNDED), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 1))
    minMaxAve = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp("MIN")), ("-", AtomicContentSpec.val(ActionSpec.avp("MAX"))), ("\\n", AtomicContentSpec.val(ActionSpec.avp("AVE"), recAve)))
    cellSpec = ConditionalContentSpec(DASH_OPT, AtomicContentSpec.skip(), minMaxAve)
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(Quantifier.exactly(2)), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("LOCATION")))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("HYDROBIONT_GROUP"))), CellPattern.of(timeYear), CellPattern.of(Quantifier.one_or_more(), cellSpec))))

def pattern_129():
    DASH = CellMatchCondition(CellPredicate.regex_matched("\\s*-\\s*"))
    substr04 = StringExtractor.substring(0, 4)
    hgUnit = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp("HYDROBIONT_GROUP")), (",", AtomicContentSpec.val(ActionSpec.avp("UNIT"), extractor=StringExtractor.trimmed())))
    rowC1 = ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.col_exact(1))
    recAve = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_cell(), UNBOUNDED), ProviderSpec.val(rowC1, UNBOUNDED), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 1))
    minMaxAve = CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp("MIN")), ("-", AtomicContentSpec.val(ActionSpec.avp("MAX"))), ("\\n", AtomicContentSpec.val(ActionSpec.avp("AVE"), recAve)))
    cellSpec = ConditionalContentSpec(DASH, AtomicContentSpec.skip(), minMaxAve)
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(Quantifier.exactly(2)), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("YEAR"), extractor=substr04))), RowPattern.of(Quantifier.one_or_more(), CellPattern.skip(), CellPattern.of(hgUnit), CellPattern.of(Quantifier.one_or_more(), cellSpec))))

def pattern_130():
    DASH_OPT = CellMatchCondition(CellPredicate.regex_matched("\\s*-?\\s*"))
    substr04 = StringExtractor.substring(0, 4)
    recAve = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), 1), ProviderSpec.val(ItemFilterConditionSpec.left_of(), 2, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 1), ProviderSpec.ctx_avp("UNIT", "MG/DM3"))
    mpcCell = ConditionalContentSpec(DASH_OPT, AtomicContentSpec.skip(), CompoundContentSpec.of(AtomicContentSpec.val(ActionSpec.avp("MPC_MIN")), ("-", AtomicContentSpec.val(ActionSpec.avp("MPC_MAX")))))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.exactly(4), AtomicContentSpec.val(ActionSpec.avp("YEAR"), extractor=substr04)), CellPattern.skip()), RowPattern.of(CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(Quantifier.one_or_more(), SubrowPattern.of(Quantifier.one(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("INDICATOR")))), SubrowPattern.of(Quantifier.exactly(2), CellPattern.of(mpcCell), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("AVE"), recAve))), SubrowPattern.of(Quantifier.one(), CellPattern.skip()))))

def pattern_131():
    CONTAINS_STAR = CellMatchCondition(CellPredicate.contains("*"))
    substr04 = StringExtractor.substring(0, 4)
    rowC4 = ItemFilterConditionSpec.and_(FilterTerm.same_row(), FilterTerm.col_exact(4))
    colR1 = ItemFilterConditionSpec.and_(FilterTerm.same_col(), FilterTerm.row_exact(1))
    recFreq = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), 2), ProviderSpec.val(rowC4, 1), ProviderSpec.val(colR1, 1))
    freqSpec = ConditionalContentSpec(CONTAINS_STAR, AtomicContentSpec.skip(), AtomicContentSpec.val(ActionSpec.avp("MPC_EXCEEDING_FREQUENCY"), recFreq))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(CellPattern.skip(Quantifier.exactly(2)), CellPattern.of(Quantifier.exactly(2), AtomicContentSpec.val(ActionSpec.avp("YEAR"), extractor=substr04)), CellPattern.skip()), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("POLLUTANT"))), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("MPC"))), CellPattern.of(Quantifier.exactly(2), freqSpec), CellPattern.skip())))

def pattern_132():
    BLANK = CellMatchCondition(CellPredicate.blank())
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    DASH_OPT = CellMatchCondition(CellPredicate.regex_matched("\\s*-?\\s*"))
    colR1 = ItemFilterConditionSpec.and_(FilterTerm.same_col(), FilterTerm.row_exact(1))
    stC0ind = ItemFilterConditionSpec.and_(FilterTerm.same_subtable(), FilterTerm.col_exact(0), FilterTerm.tagged("#IND"))
    stC1unit = ItemFilterConditionSpec.and_(FilterTerm.same_subtable(), FilterTerm.col_exact(1), FilterTerm.tagged("#UNIT"))
    recValue = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), 1), ProviderSpec.val(colR1, 1), ProviderSpec.val(stC0ind, 1), ProviderSpec.val(stC1unit, 1))
    valueSpec = ConditionalContentSpec(DASH_OPT, AtomicContentSpec.skip(), AtomicContentSpec.val(ActionSpec.avp("VALUE"), recValue))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(CellPattern.skip(Quantifier.exactly(2)), CellPattern.of(Quantifier.exactly(2), AtomicContentSpec.val(ActionSpec.avp("YEAR"))), CellPattern.skip(Quantifier.exactly(2)))), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val_tagged("#IND", ActionSpec.avp("POLLUTANT"))), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val_tagged("#UNIT", ActionSpec.avp("UNIT"))), CellPattern.skip(Quantifier.exactly(4))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("LOCATION"))), CellPattern.of(BLANK, Quantifier.one(), None), CellPattern.of(Quantifier.exactly(2), valueSpec), CellPattern.skip(Quantifier.exactly(2)))))

def pattern_133():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    recData = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), 1), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 1), ProviderSpec.val(ItemFilterConditionSpec.same_subtable(), 1))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("YEAR"))))), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("LOCATION"))), CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("INDICATOR"))), CellPattern.of(NOT_BLANK, Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("DATA"), recData)))))

def pattern_134():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    recData = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), 1), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 2), ProviderSpec.val(ItemFilterConditionSpec.same_subtable(), 1))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("MONTH")))), RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("DAY"))))), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("LOCATION"))), CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("INDICATOR"))), CellPattern.of(NOT_BLANK, Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("DATA"), recData)))))

def pattern_135():
    BLANK = CellMatchCondition(CellPredicate.blank())
    colAvp = ActionSpec.avp(ProviderSpec.attr(ItemFilterConditionSpec.same_col())).as_inherited()
    avNotBlank = ItemFilterConditionSpec.and_(FilterTerm.above(), FilterTerm.not_blank())
    fill = ActionSpec.fill("", ProviderSpec.any(avNotBlank, 1, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR))
    rec = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.left_of(), 2, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR), ProviderSpec.val(ItemFilterConditionSpec.same_row(), 4))
    fillOrVal = ConditionalContentSpec(BLANK, AtomicContentSpec.val(colAvp, fill), AtomicContentSpec.val(colAvp))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr(extractor=StringExtractor.upper_case()))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(Quantifier.exactly(3), fillOrVal), CellPattern.of(Quantifier.exactly(3), AtomicContentSpec.val(colAvp)), CellPattern.of(AtomicContentSpec.val(colAvp, rec)))))

def pattern_136():
    BLANK = CellMatchCondition(CellPredicate.blank())
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    colAvp = ActionSpec.avp(ProviderSpec.attr(ItemFilterConditionSpec.same_col())).as_inherited()
    avNotBlank = ItemFilterConditionSpec.and_(FilterTerm.above(), FilterTerm.not_blank())
    fill = ActionSpec.fill("", ProviderSpec.any(avNotBlank, 1, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR))
    rec = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_subtable(), 1), ProviderSpec.val(ItemFilterConditionSpec.same_row(), UNBOUNDED))
    fillOrVal = ConditionalContentSpec(BLANK, AtomicContentSpec.val(colAvp, fill), AtomicContentSpec.val(colAvp))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr(extractor=StringExtractor.upper_case())))), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("LOCATION"))), CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(Quantifier.exactly(2), fillOrVal), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(colAvp)), CellPattern.of(AtomicContentSpec.val(colAvp, rec)), CellPattern.of(Quantifier.exactly(4), AtomicContentSpec.val(colAvp)))))

def pattern_137():
    BLANK = CellMatchCondition(CellPredicate.blank())
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    colAvp = ActionSpec.avp(ProviderSpec.attr(ItemFilterConditionSpec.same_col())).as_inherited()
    avNotBlank = ItemFilterConditionSpec.and_(FilterTerm.above(), FilterTerm.not_blank())
    fill = ActionSpec.fill("", ProviderSpec.any(avNotBlank, 1, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR))
    rec = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_subtable(), 1), ProviderSpec.val(ItemFilterConditionSpec.same_row(), UNBOUNDED))
    fillOrVal = ConditionalContentSpec(BLANK, AtomicContentSpec.val(colAvp, fill), AtomicContentSpec.val(colAvp))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr(extractor=StringExtractor.upper_case())))), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("LOCATION"))), CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(colAvp, rec)), CellPattern.of(AtomicContentSpec.val(colAvp)), CellPattern.of(fillOrVal), CellPattern.of(NOT_BLANK, Quantifier.exactly(2), AtomicContentSpec.val(colAvp)))))

def pattern_138():
    colAvp = ActionSpec.avp(ProviderSpec.attr(ItemFilterConditionSpec.same_col()))
    c1Avp = ActionSpec.avp(ProviderSpec.attr(ItemFilterConditionSpec.bare(FilterTerm.col_exact(1))))
    rec = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), 1), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 1))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(Quantifier.exactly(2), AtomicContentSpec.attr(extractor=StringExtractor.upper_case())), CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("YEAR")))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(colAvp)), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(c1Avp, rec)))))

def pattern_139():
    BLANK = CellMatchCondition(CellPredicate.blank())
    colAvp = ActionSpec.avp(ProviderSpec.attr(ItemFilterConditionSpec.same_col())).as_inherited()
    avNotBlank = ItemFilterConditionSpec.and_(FilterTerm.above(), FilterTerm.not_blank())
    fill = ActionSpec.fill("", ProviderSpec.any(avNotBlank, 1, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR))
    rec = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), UNBOUNDED))
    fillOrVal = ConditionalContentSpec(BLANK, AtomicContentSpec.val(colAvp, fill), AtomicContentSpec.val(colAvp))
    skipOrRec = ConditionalContentSpec(BLANK, AtomicContentSpec.skip(), AtomicContentSpec.val(colAvp, rec))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr(extractor=StringExtractor.upper_case()))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(fillOrVal), CellPattern.of(AtomicContentSpec.val(colAvp)), CellPattern.of(skipOrRec))))

def pattern_140():
    colAvp = ActionSpec.avp(ProviderSpec.attr(ItemFilterConditionSpec.same_col())).as_inherited()
    rec = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), UNBOUNDED))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr(extractor=StringExtractor.upper_case()))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(colAvp, rec)), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(colAvp)))))

def pattern_141():
    recCol = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_col(), UNBOUNDED))
    rowAvp = ActionSpec.avp(ProviderSpec.attr(ItemFilterConditionSpec.same_row()))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.skip(), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("ZONE"), recCol))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.attr(extractor=StringExtractor.upper_case())), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(rowAvp)))))

def pattern_142():
    BLANK = CellMatchCondition(CellPredicate.blank())
    colAvp = ActionSpec.avp(ProviderSpec.attr(ItemFilterConditionSpec.same_col())).as_inherited()
    avNotBlank = ItemFilterConditionSpec.and_(FilterTerm.above(), FilterTerm.not_blank())
    fill = ActionSpec.fill("", ProviderSpec.any(avNotBlank, 1, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR))
    rec = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), UNBOUNDED))
    firstCell = ConditionalContentSpec(BLANK, AtomicContentSpec.val(colAvp, fill, rec), AtomicContentSpec.val(colAvp, rec))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr(extractor=StringExtractor.upper_case()))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(firstCell), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(colAvp)))))

def pattern_143():
    BLANK = CellMatchCondition(CellPredicate.blank())
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    colAvp = ActionSpec.avp(ProviderSpec.attr(ItemFilterConditionSpec.same_col())).as_inherited()
    avNotBlank = ItemFilterConditionSpec.and_(FilterTerm.above(), FilterTerm.not_blank())
    fill = ActionSpec.fill("", ProviderSpec.any(avNotBlank, 1, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR))
    rec = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), UNBOUNDED))
    firstCell = ConditionalContentSpec(BLANK, AtomicContentSpec.val(colAvp, fill, rec), AtomicContentSpec.val(colAvp, rec))
    fillOrVal = ConditionalContentSpec(BLANK, AtomicContentSpec.val(colAvp, fill), AtomicContentSpec.val(colAvp))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr(extractor=StringExtractor.upper_case())))), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val_tagged("#LOC", ActionSpec.avp("LOCATION"))), CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(firstCell), CellPattern.of(fillOrVal), CellPattern.of(NOT_BLANK, Quantifier.exactly(2), AtomicContentSpec.val(colAvp)))))

def pattern_144():
    BLANK = CellMatchCondition(CellPredicate.blank())
    rec = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.right_of(), UNBOUNDED))
    avNotBlank = ItemFilterConditionSpec.and_(FilterTerm.above(), FilterTerm.not_blank())
    fill = ActionSpec.fill("", ProviderSpec.any(avNotBlank, 1, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR))
    fillOrVal = ConditionalContentSpec(BLANK, AtomicContentSpec.val(fill), AtomicContentSpec.val())
    return TablePattern.of(SubtablePattern.of(RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(rec)), CellPattern.of(Quantifier.one_or_more(), fillOrVal))))

def pattern_145():
    BLANK = CellMatchCondition(CellPredicate.blank())
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    colAvp = ActionSpec.avp(ProviderSpec.attr(ItemFilterConditionSpec.same_col())).as_inherited()
    avNotBlank = ItemFilterConditionSpec.and_(FilterTerm.above(), FilterTerm.not_blank())
    fill = ActionSpec.fill("", ProviderSpec.any(avNotBlank, 1, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR))
    rec = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), 2), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 1))
    fillOrVal = ConditionalContentSpec(BLANK, AtomicContentSpec.val(colAvp, fill), AtomicContentSpec.val(colAvp))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(Quantifier.exactly(2), AtomicContentSpec.attr(extractor=StringExtractor.upper_case())), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("YEAR"))))), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val_tagged("#LOC", ActionSpec.avp("LOCATION"))), CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(fillOrVal), CellPattern.of(NOT_BLANK, Quantifier.one(), AtomicContentSpec.val(colAvp)), CellPattern.of(Quantifier.exactly(3), AtomicContentSpec.val(colAvp, ActionSpec.avp("DATA"), rec)))))

def pattern_146():
    colAvp = ActionSpec.avp(ProviderSpec.attr(ItemFilterConditionSpec.same_col()))
    rec = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), 1), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 1))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(AtomicContentSpec.attr(extractor=StringExtractor.upper_case())), CellPattern.of(Quantifier.exactly(5), AtomicContentSpec.val(ActionSpec.avp("YEAR"))), CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(colAvp)), CellPattern.of(Quantifier.exactly(5), AtomicContentSpec.val(ActionSpec.avp("DATA"), rec)), CellPattern.skip(Quantifier.one_or_more()))))

def pattern_147():
    BLANK = CellMatchCondition(CellPredicate.blank())
    colAvp = ActionSpec.avp(ProviderSpec.attr(ItemFilterConditionSpec.same_col()))
    avNotBlank = ItemFilterConditionSpec.and_(FilterTerm.above(), FilterTerm.not_blank())
    fill = ActionSpec.fill("", ProviderSpec.any(avNotBlank, 1, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR))
    rec = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), 3), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 2))
    fillOrVal = ConditionalContentSpec(BLANK, AtomicContentSpec.val(fill, colAvp), AtomicContentSpec.val(colAvp))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(Quantifier.exactly(3), AtomicContentSpec.attr(extractor=StringExtractor.upper_case())), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("INDICATOR")))), RowPattern.of(CellPattern.skip(Quantifier.exactly(3)), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("YEAR")))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(Quantifier.exactly(2), fillOrVal), CellPattern.of(AtomicContentSpec.val(colAvp)), CellPattern.of(Quantifier.exactly(5), AtomicContentSpec.val(ActionSpec.avp("DATA"), rec)), CellPattern.skip(Quantifier.one_or_more()))))

def pattern_148():
    NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
    colAvp = ActionSpec.avp(ProviderSpec.attr(ItemFilterConditionSpec.same_col())).as_inherited()
    rec = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), UNBOUNDED), ProviderSpec.val(ItemFilterConditionSpec.same_subtable(), 1))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr(extractor=StringExtractor.upper_case())))), SubtablePattern.of(Quantifier.one_or_more(), RowPattern.of(CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("LOCATION"))), CellPattern.skip(Quantifier.one_or_more())), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(colAvp, rec)), CellPattern.of(NOT_BLANK, Quantifier.one_or_more(), AtomicContentSpec.val(colAvp)))))

def pattern_149():
    BLANK = CellMatchCondition(CellPredicate.blank())
    colAvp = ActionSpec.avp(ProviderSpec.attr(ItemFilterConditionSpec.same_col())).as_inherited()
    avNotBlank = ItemFilterConditionSpec.and_(FilterTerm.above(), FilterTerm.not_blank())
    fill = ActionSpec.fill("", ProviderSpec.any(avNotBlank, 1, traversal_order=TraversalOrder.REVERSE_ROW_MAJOR))
    rec = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), UNBOUNDED))
    firstCell = ConditionalContentSpec(BLANK, AtomicContentSpec.val(colAvp, fill, rec), AtomicContentSpec.val(colAvp, rec))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.attr(extractor=StringExtractor.upper_case()))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(firstCell), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(colAvp)))))

def pattern_150():
    stAvp = ActionSpec.avp(ProviderSpec.attr(ItemFilterConditionSpec.same_subtable()))
    rec = ActionSpec.rec(ProviderSpec.val(ItemFilterConditionSpec.same_row(), 1), ProviderSpec.val(ItemFilterConditionSpec.same_col(), 1))
    return TablePattern.of(SubtablePattern.of(RowPattern.of(CellPattern.of(AtomicContentSpec.attr()), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(ActionSpec.avp("YEAR")))), RowPattern.of(Quantifier.one_or_more(), CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("LOCATION"))), CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val(stAvp, rec)))))

PATTERNS = {
    "001": pattern_001,
    "002": pattern_002,
    "003": pattern_003,
    "004": pattern_004,
    "005": pattern_005,
    "006": pattern_006,
    "007": pattern_007,
    "008": pattern_008,
    "009": pattern_009,
    "010": pattern_010,
    "011": pattern_011,
    "012": pattern_012,
    "013": pattern_013,
    "014": pattern_014,
    "015": pattern_015,
    "016": pattern_016,
    "017": pattern_017,
    "018": pattern_018,
    "019": pattern_019,
    "020": pattern_020,
    "021": pattern_021,
    "022": pattern_022,
    "023": pattern_023,
    "024": pattern_024,
    "025": pattern_025,
    "026": pattern_026,
    "027": pattern_027,
    "028": pattern_028,
    "029": pattern_029,
    "030": pattern_030,
    "031": pattern_031,
    "032": pattern_032,
    "033": pattern_033,
    "034": pattern_034,
    "035": pattern_035,
    "036": pattern_036,
    "037": pattern_037,
    "038": pattern_038,
    "039": pattern_039,
    "040": pattern_040,
    "041": pattern_041,
    "042": pattern_042,
    "043": pattern_043,
    "044": pattern_044,
    "045": pattern_045,
    "046": pattern_046,
    "047": pattern_047,
    "048": pattern_048,
    "049": pattern_049,
    "050": pattern_050,
    "051": pattern_051,
    "052": pattern_052,
    "053": pattern_053,
    "054": pattern_054,
    "055": pattern_055,
    "056": pattern_056,
    "057": pattern_057,
    "058": pattern_058,
    "059": pattern_059,
    "060": pattern_060,
    "061": pattern_061,
    "062": pattern_062,
    "063": pattern_063,
    "064": pattern_064,
    "065": pattern_065,
    "066": pattern_066,
    "067": pattern_067,
    "068": pattern_068,
    "069": pattern_069,
    "070": pattern_070,
    "071": pattern_071,
    "072": pattern_072,
    "073": pattern_073,
    "074": pattern_074,
    "075": pattern_075,
    "076": pattern_076,
    "077": pattern_077,
    "078": pattern_078,
    "079": pattern_079,
    "080": pattern_080,
    "081": pattern_081,
    "082": pattern_082,
    "083": pattern_083,
    "084": pattern_084,
    "085": pattern_085,
    "086": pattern_086,
    "087": pattern_087,
    "088": pattern_088,
    "089": pattern_089,
    "090": pattern_090,
    "091": pattern_091,
    "092": pattern_092,
    "093": pattern_093,
    "094": pattern_094,
    "095": pattern_095,
    "096": pattern_096,
    "097": pattern_097,
    "098": pattern_098,
    "099": pattern_099,
    "100": pattern_100,
    "101": pattern_101,
    "102": pattern_102,
    "103": pattern_103,
    "104": pattern_104,
    "105": pattern_105,
    "106": pattern_106,
    "107": pattern_107,
    "108": pattern_108,
    "109": pattern_109,
    "110": pattern_110,
    "111": pattern_111,
    "112": pattern_112,
    "113": pattern_113,
    "114": pattern_114,
    "115": pattern_115,
    "116": pattern_116,
    "117": pattern_117,
    "118": pattern_118,
    "119": pattern_119,
    "120": pattern_120,
    "121": pattern_121,
    "122": pattern_122,
    "123": pattern_123,
    "124": pattern_124,
    "125": pattern_125,
    "126": pattern_126,
    "127": pattern_127,
    "128": pattern_128,
    "129": pattern_129,
    "130": pattern_130,
    "131": pattern_131,
    "132": pattern_132,
    "133": pattern_133,
    "134": pattern_134,
    "135": pattern_135,
    "136": pattern_136,
    "137": pattern_137,
    "138": pattern_138,
    "139": pattern_139,
    "140": pattern_140,
    "141": pattern_141,
    "142": pattern_142,
    "143": pattern_143,
    "144": pattern_144,
    "145": pattern_145,
    "146": pattern_146,
    "147": pattern_147,
    "148": pattern_148,
    "149": pattern_149,
    "150": pattern_150,
}
