"""CLI runner of RTL solutions — the pyRegTab counterpart of the
``regtab-runner`` (``RtlRunner.java``) used by regtab-eval-on-atbench, with the
same I/O contract so that one replaces the other in a harness:

    python -m pyregtab.runner <pattern.rtl>

reads ``./input.csv`` (RFC 4180: quoted fields may contain commas, doubled
quotes and line breaks; empty lines are skipped; ragged rows are padded with
empty cells to the widest row), applies the RTL pattern (``RtlCompiler.compile``
→ ``AtpMatcher.match`` → ``TableInterpreter`` with ``RECORD_FIRST`` →
``pattern.transform``) and writes ``./output.csv``: the schema as the header
row (by default), one row per record, every field double-quoted, a missing value
as the empty string, UTF-8, LF line endings.

Exit codes: 0 — success, 1 — the pattern did not match, 2 — invalid usage;
an RTL compile error raises (non-zero exit, message on stderr). Interpreter
diagnostics (skipped CONCAT/JOIN actions) are printed to stderr and do not
change the exit code. ``--input`` / ``--output`` override the file names;
``--strict`` turns diagnostics into errors. ``--no-header`` omits the schema
row; ``--header`` explicitly includes it. ``--threads N`` enables the native
worker pool for large independent interpretation phases (default: 1).
"""

from __future__ import annotations

import argparse
import sys
from pathlib import Path

from pyregtab import (
    AtpMatcher,
    Recordset,
    RtlCompiler,
    SchemaConstructionStrategy,
    TableInterpreter,
    TableSyntax,
)
from pyregtab._core import parse_csv as _parse_csv


def parse_csv(text: str) -> list[list[str]]:
    """RFC 4180 parser mirroring ``RtlRunner.parseCsv``: a quoted field may
    contain commas, doubled quotes and line breaks; CRLF, CR and LF all end a
    row; empty lines are skipped (a line holding only ``""`` is a row with one
    empty field). Rows are returned as parsed (ragged). Implemented in the
    native core (``pyregtab._core.parse_csv``)."""
    return _parse_csv(text)


def load_table(path: Path) -> TableSyntax:
    """CSV file (UTF-8) → ``TableSyntax`` with the runner's rules: the rows of
    ``parse_csv``, ragged rows padded with empty cells to the widest row. One
    native call (``TableSyntax.from_csv``); equivalent to
    ``TableSyntax.from_rows(parse_csv(path.read_text(encoding="utf-8")))``."""
    return TableSyntax.from_csv(path)


def write_csv(path: Path, rs: Recordset, *, header: bool = True) -> None:
    """The regtab-runner output format: the schema as the header row by default, every
    field double-quoted (quotes doubled), a missing value as the empty
    string, UTF-8, LF line endings. Set ``header=False`` to omit the schema row."""
    rs.to_csv(path, quote_all=True, newline="\n", header=header)


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(
        prog="python -m pyregtab.runner",
        description="Apply an RTL pattern to input.csv and write output.csv (regtab-runner contract).",
    )
    ap.add_argument("pattern", help="path to the .rtl solution")
    ap.add_argument("--input", default="input.csv", help="input table (default: ./input.csv)")
    ap.add_argument("--output", default="output.csv", help="output recordset (default: ./output.csv)")
    ap.add_argument("--header", action=argparse.BooleanOptionalAction, default=True,
                    help="include the schema as the output CSV header row (default: enabled)")
    ap.add_argument("--strict", action="store_true",
                    help="fail on a skipped CONCAT/JOIN action instead of reporting it")
    ap.add_argument("--threads", type=int, default=1,
                    help="native worker threads for large independent interpretation phases (default: 1)")
    try:
        args = ap.parse_args(argv)
    except SystemExit as e:
        return 2 if e.code else 0
    if args.threads < 1:
        print("--threads must be positive", file=sys.stderr)
        return 2

    rtl = Path(args.pattern).read_bytes().decode("utf-8")
    syntax = load_table(Path(args.input))
    pattern = RtlCompiler.compile(rtl)
    itm = AtpMatcher.match(pattern, syntax)
    if itm is None:
        print("RTL pattern did not match input", file=sys.stderr)
        return 1
    interpreter = (
        TableInterpreter()
        .with_strategy(SchemaConstructionStrategy.RECORD_FIRST)
        .with_strict_preconditions(args.strict)
        .with_num_threads(args.threads)
    )
    rs = pattern.transform(interpreter.interpret(itm))
    for d in interpreter.diagnostics():
        print(f"diagnostic: {d}", file=sys.stderr)
    write_csv(Path(args.output), rs, header=args.header)
    return 0


if __name__ == "__main__":
    sys.exit(main())
