"""Test infrastructure: port of jRegTab's CsvTableLoader, CsvRecordsetLoader,
RecordsetAssert and TaskMatchOptionsLoader."""

from __future__ import annotations

import csv
import json
from collections import Counter
from pathlib import Path

from pyregtab import (
    AtpMatcher,
    Recordset,
    RtlCompiler,
    Schema,
    SchemaConstructionStrategy,
    TableInterpreter,
    TableSyntax,
)

TESTS_DIR = Path(__file__).parent
TASKS_ROOT = TESTS_DIR / "fixtures" / "tasks"
CONFORMANCE = TESTS_DIR.parent / "conformance"

STRICT = "STRICT"
FLEXIBLE = "FLEXIBLE"


def load_table(path: Path) -> TableSyntax:
    """CSV -> TableSyntax. No header, delimiter ',', quote '"', UTF-8,
    empty lines ignored (CsvTableLoader)."""
    with open(path, newline="", encoding="utf-8") as f:
        records = [rec for rec in csv.reader(f) if rec]
    if not records:
        raise ValueError(f"Empty CSV: {path}")
    num_rows = len(records)
    num_cols = len(records[0])
    for i, r in enumerate(records):
        if len(r) != num_cols:
            raise ValueError(f"Inconsistent column count at row {i + 1}")
    syntax = TableSyntax(num_rows, num_cols)
    for r, rec in enumerate(records):
        for c, val in enumerate(rec):
            syntax.cell(r, c).set_text(val if val is not None else "")
    return syntax


def load_recordset(path: Path, schema: Schema | None = None) -> Recordset:
    """CSV -> Recordset. With schema: header-less, positional mapping.
    Without: first row = attribute names (CsvRecordsetLoader)."""
    with open(path, newline="", encoding="utf-8") as f:
        records = [rec if rec else [""] for rec in csv.reader(f)]
    if schema is not None:
        attrs = schema.attributes
        rows = [
            {attrs[j]: (rec[j] if j < len(rec) and rec[j] is not None else "")
             for j in range(len(attrs))}
            for rec in records
        ]
        return Recordset(schema, rows)
    if not records:
        raise ValueError(f"Empty CSV: {path}")
    attrs = [a.strip() if a is not None else "" for a in records[0]]
    schema = Schema(attrs)
    rows = [
        {attrs[j]: (rec[j] if j < len(rec) and rec[j] is not None else "")
         for j in range(len(attrs))}
        for rec in records[1:]
    ]
    return Recordset(schema, rows)


def load_match_options(task_id: str) -> dict:
    """Merged options for a task (TaskMatchOptionsLoader)."""
    opts = {"attributeOrder": STRICT, "recordOrder": STRICT, "expectedHasHeader": True}

    root_file = TASKS_ROOT / "task_match_options.json"
    if root_file.is_file():
        root = json.loads(root_file.read_text(encoding="utf-8-sig"))
        for k in ("attributeOrder", "recordOrder", "expectedHasHeader"):
            if root.get(k) not in (None, ""):
                opts[k] = root[k]
        patch = (root.get("tasks") or {}).get(task_id)
        if patch:
            for k in ("attributeOrder", "recordOrder", "expectedHasHeader"):
                if patch.get(k) not in (None, ""):
                    opts[k] = patch[k]

    task_file = TASKS_ROOT / f"task_{task_id}" / "task_match_options.json"
    if task_file.is_file():
        patch = json.loads(task_file.read_text(encoding="utf-8-sig"))
        for k in ("attributeOrder", "recordOrder", "expectedHasHeader"):
            if patch.get(k) not in (None, ""):
                opts[k] = patch[k]

    opts["attributeOrder"] = str(opts["attributeOrder"]).upper()
    opts["recordOrder"] = str(opts["recordOrder"]).upper()
    return opts


def _norm(v):
    return "" if v is None else v


def assert_matches(actual: Recordset, expected: Recordset, opts: dict) -> None:
    """Port of RecordsetAssert.assertMatches."""
    exp_attrs = expected.schema.attributes
    act_attrs = actual.schema.attributes

    if opts["attributeOrder"] == STRICT:
        assert exp_attrs == act_attrs, (
            f"Schema (attribute order) mismatch: expected {exp_attrs}, got {act_attrs}"
        )
    else:
        assert set(exp_attrs) == set(act_attrs), (
            f"Schema (attribute set) mismatch: expected {set(exp_attrs)}, got {set(act_attrs)}"
        )

    assert len(expected) == len(actual), (
        f"Record count mismatch: expected {len(expected)}, got {len(actual)}"
    )

    if opts["recordOrder"] == STRICT:
        for i in range(len(expected)):
            e, a = expected[i], actual[i]
            for attr in exp_attrs:
                assert _norm(e[attr]) == _norm(a[attr]), (
                    f"Record {i}, attribute '{attr}': "
                    f"expected {e[attr]!r}, got {a[attr]!r}"
                )
    else:
        sorted_attrs = sorted(set(exp_attrs))

        def fingerprint(rec):
            return tuple((a, _norm(rec.get(a))) for a in sorted_attrs)

        exp_counts = Counter(fingerprint(expected[i]) for i in range(len(expected)))
        act_counts = Counter(fingerprint(actual[i]) for i in range(len(actual)))
        assert exp_counts == act_counts, "Record multiset mismatch (order-independent)"


def task_rtl(task_id: str) -> str:
    """RTL source of a task from the conformance corpus."""
    return (CONFORMANCE / "positive" / f"task_{task_id}.rtl").read_bytes().decode("utf-8")


def run_task_variant(task_id: str, variant: int, pattern) -> None:
    """Port of RtlTaskBase/AtpTaskBase.runVariant."""
    task_dir = TASKS_ROOT / f"task_{task_id}"
    syntax = load_table(task_dir / f"input_{variant}.csv")

    itm = AtpMatcher.match(pattern, syntax)
    assert itm is not None, f"Task {task_id} pattern did not match variant {variant}"

    actual = pattern.transform(
        TableInterpreter()
        .with_strategy(SchemaConstructionStrategy.RECORD_FIRST)
        .interpret(itm)
    )

    opts = load_match_options(task_id)
    expected_path = task_dir / f"expected_{variant}.csv"
    if opts["expectedHasHeader"]:
        expected = load_recordset(expected_path)
    else:
        expected = load_recordset(expected_path, actual.schema)
    assert_matches(actual, expected, opts)
    assert len(actual) > 0


def task_ids() -> list[str]:
    return sorted(
        d.name[5:]
        for d in TASKS_ROOT.iterdir()
        if d.is_dir() and d.name.startswith("task_")
    )


def variants_of(task_id: str) -> list[int]:
    task_dir = TASKS_ROOT / f"task_{task_id}"
    return [i for i in range(1, 10) if (task_dir / f"input_{i}.csv").exists()]


def compile_task(task_id: str):
    return RtlCompiler.compile(task_rtl(task_id))
