"""RTL conformance corpus, item 5 of the contract in conformance/README.md
(port of RtlSemanticConformanceTest):

    for every semantic/<case>/, matching pattern.rtl against input.csv and
    interpreting the result yields a recordset equal to expected.csv.

Items 1-4 pin syntax and canonical form only (test_conformance.py); two
implementations can agree on the canonical RTL of a pattern and still execute
it differently. This closes that gap.
"""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from task_runner import CONFORMANCE, FLEXIBLE, STRICT, assert_matches, load_recordset, load_table

from pyregtab import AtpMatcher, RtlCompiler, SchemaConstructionStrategy, TableInterpreter

SEMANTIC = CONFORMANCE / "semantic"

PATTERN = "pattern.rtl"
INPUT = "input.csv"
EXPECTED = "expected.csv"
OPTIONS = "options.json"

# Semantic cases default to a header-less expected.csv compared positionally
# against the schema the pattern produced, so that attribute names invented by
# the implementation never leak into the contract. A case whose pattern names
# its attributes (via AVP) can set expectedHasHeader in options.json.
#
# Note this differs from the task defaults in task_runner.load_match_options,
# where expectedHasHeader is True -- do not reuse that loader here.
DEFAULTS = {"attributeOrder": STRICT, "recordOrder": STRICT, "expectedHasHeader": False}

CASES = sorted(p for p in SEMANTIC.iterdir() if p.is_dir())


def _policy(raw, fallback: str) -> str:
    if raw is None or not str(raw).strip():
        return fallback
    value = str(raw).strip().upper()
    if value not in (STRICT, FLEXIBLE):
        raise ValueError(f"Unknown order policy: {raw} (use {STRICT} or {FLEXIBLE})")
    return value


def load_case_options(case: Path) -> dict:
    """Merged options for one semantic case; unknown keys are ignored."""
    opts = dict(DEFAULTS)
    file = case / OPTIONS
    if not file.is_file():
        return opts
    patch = json.loads(file.read_text(encoding="utf-8-sig")) or {}
    opts["attributeOrder"] = _policy(patch.get("attributeOrder"), opts["attributeOrder"])
    opts["recordOrder"] = _policy(patch.get("recordOrder"), opts["recordOrder"])
    if patch.get("expectedHasHeader") is not None:
        opts["expectedHasHeader"] = bool(patch["expectedHasHeader"])
    return opts


@pytest.mark.parametrize("case", CASES, ids=lambda p: p.name)
def test_every_case_is_complete(case):
    for required in (PATTERN, INPUT, EXPECTED):
        assert (case / required).is_file(), f"missing {required} in {case}"


@pytest.mark.parametrize("case", CASES, ids=lambda p: p.name)
def test_semantic_case(case):
    syntax = load_table(case / INPUT)
    # binary read: string literals may carry raw CR/CRLF payload
    pattern = RtlCompiler.compile((case / PATTERN).read_bytes().decode("utf-8"))

    itm = AtpMatcher.match(pattern, syntax)
    assert itm is not None, f"pattern did not match {case.name}/{INPUT}"

    actual = pattern.transform(
        TableInterpreter()
        .with_strategy(SchemaConstructionStrategy.RECORD_FIRST)
        .interpret(itm)
    )

    opts = load_case_options(case)
    expected_path = case / EXPECTED
    if opts["expectedHasHeader"]:
        expected = load_recordset(expected_path)
    else:
        # header-less: columns matched positionally against the pattern's schema
        expected = load_recordset(expected_path, actual.schema)

    assert_matches(actual, expected, opts)
