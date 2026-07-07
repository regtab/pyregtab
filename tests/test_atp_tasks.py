"""ATP task suite: patterns built with the Python fluent API instead of RTL
(port of AtpTask001/002/005Test — acceptance test of the spec factories)."""

import pytest

from pyregtab import (
    ActionSpec,
    AtomicContentSpec,
    CellMatchCondition,
    CellPattern,
    CellPredicate,
    ItemFilterConditionSpec,
    ProviderSpec,
    Quantifier,
    RowPattern,
    StringExtractor,
    SubtablePattern,
    TablePattern,
    UNBOUNDED,
)
from task_runner import run_task_variant, variants_of

SAME_SUBTABLE = ItemFilterConditionSpec.same_subtable()
SAME_SUBCOLUMN = ItemFilterConditionSpec.same_subcol()
SAME_SUBROW = ItemFilterConditionSpec.same_subrow()
NOT_BLANK = CellMatchCondition(CellPredicate.not_blank())
BLANK = CellMatchCondition(CellPredicate.blank())


def pattern_001():
    return TablePattern.of(
        SubtablePattern.of(
            Quantifier.one_or_more(),
            RowPattern.of(
                CellPattern.of(
                    AtomicContentSpec.val(
                        ActionSpec.rec(ProviderSpec.val(SAME_SUBTABLE, UNBOUNDED))
                    )
                ),
                CellPattern.of(Quantifier.exactly(2), AtomicContentSpec.val()),
                CellPattern.skip(Quantifier.one_or_more()),
            ),
            RowPattern.of(
                CellPattern.skip(),
                CellPattern.of(Quantifier.exactly(4), AtomicContentSpec.val()),
                CellPattern.skip(Quantifier.one_or_more()),
            ),
        )
    )


def pattern_002():
    return TablePattern.of(
        SubtablePattern.of(
            Quantifier.one_or_more(),
            RowPattern.of(
                Quantifier.exactly(2),
                CellPattern.of(
                    AtomicContentSpec.val(
                        extractor=StringExtractor.whitespace_normalized()
                    )
                ),
                CellPattern.skip(),
            ),
            RowPattern.of(
                Quantifier.one_or_more(),
                CellPattern.of(
                    NOT_BLANK,
                    Quantifier.one(),
                    AtomicContentSpec.val(
                        ActionSpec.rec(
                            ProviderSpec.val(SAME_SUBCOLUMN, 2),
                            ProviderSpec.val(SAME_SUBROW, 1),
                            anchor_pos=2,
                        )
                    ),
                ),
                CellPattern.of(AtomicContentSpec.val()),
            ),
            RowPattern.of(
                Quantifier.zero_or_one(),
                CellPattern.of(BLANK, Quantifier.one(), None),
                CellPattern.skip(),
            ),
        )
    )


def pattern_005():
    return TablePattern.of(
        SubtablePattern.of(
            RowPattern.of(
                CellPattern.skip(),
                CellPattern.of(Quantifier.one_or_more(), AtomicContentSpec.val()),
            ),
            RowPattern.of(CellPattern.skip(Quantifier.one_or_more())),
            RowPattern.of(
                Quantifier.one_or_more(),
                CellPattern.of(AtomicContentSpec.val()),
                CellPattern.of(
                    Quantifier.one_or_more(),
                    AtomicContentSpec.val(
                        ActionSpec.rec(
                            ProviderSpec.val(SAME_SUBROW, 1),
                            ProviderSpec.val(SAME_SUBCOLUMN, 1),
                            anchor_pos=2,
                        )
                    ),
                ),
            ),
        )
    )


PATTERNS = {"001": pattern_001, "002": pattern_002, "005": pattern_005}

PARAMS = [
    pytest.param(t, v, id=f"task_{t}-variant_{v}")
    for t in sorted(PATTERNS)
    for v in variants_of(t)
]


@pytest.mark.parametrize("task_id,variant", PARAMS)
def test_atp_task(task_id, variant):
    run_task_variant(task_id, variant, PATTERNS[task_id]())
