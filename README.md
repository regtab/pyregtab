# pyRegTab

[![CI](https://github.com/regtab/pyregtab/actions/workflows/ci.yml/badge.svg)](https://github.com/regtab/pyregtab/actions/workflows/ci.yml)
[![PyPI](https://img.shields.io/pypi/v/pyregtab.svg)](https://pypi.org/project/pyregtab/)
[![Python](https://img.shields.io/badge/Python-3.10%2B-blue.svg)](https://www.python.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](https://opensource.org/licenses/MIT)

RegTab: pattern-driven data extraction from document tables with regular
structure — the Python port of jRegTab with a native Rust core.

pyRegTab compiles **RTL** (Regular Table Language) patterns into abstract
table patterns (ATP), matches them against a table's syntactic layer (ITM),
and interprets the match into a relational **recordset**:

```
TableSyntax → RtlCompiler/TablePattern → AtpMatcher → TableInterpreter → Recordset
```

**pyRegTab 0.7.2 ≙ jRegTab 0.7.1** (same API, same semantics, same test
corpus), including the
embedded RTL DSL `pyregtab.dsl` — a port of jRegTab's `ru.icc.regtab.dsl`
(added upstream in jRegTab 0.3.0). Python-side extras on top of the Java API:
`AtpMatcher.match_many` (parallel batch matching), `Recordset.to_pandas()`,
`Recordset.to_csv()`, `RtlCompileError.line`/`.col` attributes,
`CellDerivedItem.span` (the item's byte range within the raw cell text), and
the CLI runner `python -m pyregtab.runner` (see below).

Since 0.5.1 the language and the interpreter follow jRegTab 0.6.0–0.7.1:
`JOIN(K)` is the **record product** (a cross product for `K = ∅`, an equi-join
on the key `K` otherwise), the former folding operation is spelled
`CONCAT(K)` — a pattern written for 0.5.x must replace `JOIN(K)` by
`CONCAT(K)`; the key `K` may name attributes (`CONCAT(0, 1, 'A', 'B')`,
`JOIN('Year')`); skipped `CONCAT`/`JOIN` actions (a key mismatch, a shared
attribute, a forgotten `REC`) are reported through
`TableInterpreter.diagnostics()`; zero-width subrows match the empty
sequence and `{0}`/`{1}` are accepted; cell-derived providers use a spatial
index, so interpretation is linear in the number of cells (a 1.19 M-cell
table interprets in seconds).

## Installation

```
pip install pyregtab
```

Binary wheels are published for Windows, Linux and macOS (x86-64 / arm64),
CPython ≥ 3.10 (one `abi3` wheel per platform). Building from the sdist
requires a Rust toolchain.

## Example

```python
from pyregtab import TableSyntax, RtlCompiler, AtpMatcher, TableInterpreter

syntax = TableSyntax(3, 3)
syntax.cell(0, 1).set_text("CA");  syntax.cell(0, 2).set_text("HU")
syntax.cell(1, 0).set_text("IKT"); syntax.cell(1, 1).set_text("0 Jan"); syntax.cell(1, 2).set_text("8 Feb")
syntax.cell(2, 0).set_text("SVO"); syntax.cell(2, 1).set_text("31 Jan"); syntax.cell(2, 2).set_text("40 Feb")

pattern = RtlCompiler.compile("""
    [ [] [VAL : 'AIRLINE'->AVP]+ ]
    [ [VAL : 'AIRPORT'->AVP]
      [VAL : (COL, ROW, CL)->REC, 'ND'->AVP " " VAL : 'MON'->AVP]+ ]+
""")

itm = AtpMatcher.match(pattern, syntax)     # InterpretableTable | None
rs = TableInterpreter().interpret(itm)      # Recordset
rs.schema.attributes                        # ['ND', 'AIRLINE', 'AIRPORT', 'MON']
rs[0]["ND"]                                 # '0'
df = rs.to_pandas()                         # extras: pip install pyregtab[pandas]
```

Patterns can also be built without RTL, via the fluent spec API
(`TablePattern.of(SubtablePattern.of(...))` — same factories as in Java,
snake_case method names), and serialized back to RTL with
`AtpToRtlSerializer.serialize(pattern)`.

For a terser, RTL-like way to build patterns in code, use the **embedded RTL**
DSL (`pyregtab.dsl`) — see [Embedded RTL](#embedded-rtl) below.

Named Python predicates are attached to RTL via `EXT('name')`:

```python
from pyregtab import Bindings

p = RtlCompiler.compile(
    "{ [ [EXT('isTotal') ? VAL : ST*->REC] []+ ] }+",
    Bindings.of().cell("isTotal", lambda cell: cell.text.startswith("Total")),
)
```

## Embedded RTL

The `pyregtab.dsl` module is a fluent DSL that reads almost like RTL but is
ordinary Python — with IDE completion, structural typing, pattern composition
via plain variables, and Python callables as escape-hatch constraints. It builds
the **same `TablePattern`** objects as the compiler (verified byte-for-byte
against `RtlCompiler.compile` for a representative set of tasks in
`tests/test_dsl.py`).

```python
from pyregtab.dsl import *

# RTL: { [ [VAL : ST*->REC] [VAL]{2} []+ ]
#        [ []               [VAL]{4} []+ ] }+
p = table(
    subtable(
        row(cell(VAL, rec(ST.unbounded())), cell(VAL).exactly(2), skip().one_or_more()),
        row(skip(),                         cell(VAL).exactly(4), skip().one_or_more()),
    ).one_or_more())
```

Method names are snake_case (`.one_or_more()`, `.and_()`, `.split_by()`); the
vocabulary constants (`VAL`, `ST`, `COL`, `C(n)`, …) match RTL. See the
[Embedded RTL guide](docs/embedded-rtl.md) for the full mapping and the
`where(...)` escape hatch.

## API mapping (Java → Python)

| Java | Python |
|---|---|
| `RtlCompiler.compile(String)` | `RtlCompiler.compile(str)` / `pyregtab.compile(...)` |
| `AtpMatcher.match(p, s)` → `Optional<InterpretableTable>` | `AtpMatcher.match(p, s)` → `InterpretableTable \| None` |
| `Quantifier.oneOrMore()` | `Quantifier.one_or_more()` |
| `new TableInterpreter().withStrategy(s).interpret(itm)` | `TableInterpreter().with_strategy(s).interpret(itm)` |
| `rs.records().get(0).get("Name")` | `rs[0]["Name"]`, `rs.records`, `record.get("Name")` |
| `cell.text()` / `cell.setText(t)` | property `cell.text` (get/set); `cell.set_text(t)` also works |
| `RtlCompileException` | `RtlCompileError` |

## Architecture

Everything after the Python call boundary runs in a native core written in
Rust (`pyregtab._core`, built with [PyO3](https://pyo3.rs) and
[maturin](https://maturin.rs)); the Python layer is a thin re-export.

- `grammar/RTL.g4` — the **normative specification** of the RTL language
  (a verbatim copy from jRegTab; the upstream commit and the grammar's
  SHA-256 are recorded in `grammar/UPSTREAM`). The core's parser is a
  hand-written lexer + recursive descent that structurally follows the
  grammar rules. A CI job (`tools/check_grammar_sync.py`) fails the build if
  the copy drifts from the pinned hash, and — when a jRegTab read token is
  available — cross-checks it byte-for-byte against the upstream commit.
- `conformance/` — the shared RTL conformance corpus (also pinned from
  jRegTab, see `conformance/UPSTREAM` and `conformance/README.md`). Both
  implementations must compile every positive case to the same canonical
  form and reject every negative case; the corpus runs in CI of both
  projects. Any RTL language change flows: `RTL.g4` in jregtab → corpus
  extension → both parsers → green corpus in both CIs.
- Regular expressions in RTL constraints are executed by the Rust
  [`regex`](https://docs.rs/regex) crate (linear-time). The reference
  fixture corpus uses no lookaround/backreferences (audited), so the
  dialect is compatible with `java.util.regex` on this corpus. Documented
  divergences from Java: `\d`/`\s`/`\w` are Unicode-aware in `regex`
  (ASCII in Java), and `SUBSTR` indices count code points (UTF-16 units in
  Java) — identical behavior on the entire reference corpus.

## Testing

`pytest tests` runs (1 981 tests):

- the full benchmark suite — tasks 001–150 (Foofah, RegTab, Baikal),
  every fixture variant, **both** via RTL patterns and via ATP patterns
  built with the Python spec API (1 500 task variants in total; fixtures
  are copied verbatim from jRegTab into `tests/fixtures/tasks`, ATP
  builders are mechanically translated from the Java tests by
  `tools/translate_atp.py`);
- embedded RTL DSL parity — 26 representative tasks/constructs built with
  `pyregtab.dsl` produce byte-identical ATP to `RtlCompiler.compile`
  (`tests/test_dsl.py`);
- the RTL conformance corpus (positive canonical forms, fixed points,
  negative rejections);
- RTL↔ATP round-trip for tasks 001–050;
- API unit tests (syntax layer, extractors, EXT bindings, custom
  predicates, transformations, interpreter options, GIL-released batch
  matching from a thread pool and via `AtpMatcher.match_many`);
- `CONCAT`/`JOIN` semantics, named keys, diagnostics, strict preconditions,
  zero-width subrows and `{0}`/`{1}` (`tests/test_join_concat.py`, a port of
  jRegTab's `TableInterpreterMultiRecordTest`).

`cargo test` additionally runs the conformance corpus, an end-to-end smoke
test against the native core alone, the working-state unit tests for
`CONCAT`/`JOIN` (ports of `WorkingStateConcatTest`/`WorkingStateJoinTest`), the
zero-width matcher cases, and a randomized equivalence test of the indexed
provider against the reference definition Υ^{J,k}_{τ,κ} (full scan + sort).
Differential testing against the Java reference (`tools/differential.py` +
`tools/RecordsetDumpMain.java`) compares recordsets cell-by-cell on all 750
task variants; the 244 ATBench solutions of regtab-eval-on-atbench are
byte-identical between the jRegTab 0.7.1 runner and `python -m pyregtab.runner`
(`tools/atbench_bytecmp.py` runs both runners on every solution and compares
exit codes and `output.csv` bytes; the Java outputs are cached).

## IDE support

Install **[Regular Table Language (RTL)](https://marketplace.visualstudio.com/items?itemName=regtab.regtab)**
from the VS Code Marketplace (`ext install regtab.regtab`): syntax highlighting
for `.rtl` files and for RTL embedded in Python strings passed to
`RtlCompiler.compile(...)`, plus compile diagnostics and a live match preview
against CSV fixtures. The extension sources are at
[regtab/vscode-rtl](https://github.com/regtab/vscode-rtl); a TextMate bundle for
IntelliJ/PyCharm and other TextMate editors is under [`ide/`](ide/README.md).

RTL is also validated at compile time: `RtlCompiler.compile(...)` raises
`RtlCompileError` with a `line:col` position on an invalid pattern.

## Command-line runner

`python -m pyregtab.runner <solution.rtl>` (or `python -m pyregtab <solution.rtl>`)
reads `./input.csv`, applies the pattern (`RtlCompiler.compile` →
`AtpMatcher.match` → `TableInterpreter` with `RECORD_FIRST` →
`pattern.transform`) and writes `./output.csv` — the schema as the header row,
every field double-quoted, a missing value as the empty string, UTF-8, LF. It
is the drop-in counterpart of the `regtab-runner` jar used by
[regtab-eval-on-atbench](https://github.com/regtab/regtab-eval-on-atbench):
same I/O contract, same exit codes (0 — success, 1 — the pattern did not
match, 2 — invalid usage), same CSV parsing (RFC 4180 with multi-line quoted
cells, ragged rows padded); on the 244 ATBench solutions of that project the
two runners produce byte-identical `output.csv`. `--input`/`--output` override
the file names, `--strict` turns interpreter diagnostics into errors.

The runner loads the table in one native call (`TableSyntax.from_csv(path)`;
`TableSyntax.from_rows(rows)` and `TableSyntax.from_csv_text(text)` do the same
for rows or text already in memory, `pyregtab.runner.parse_csv` exposes the
RFC 4180 parser) and writes the recordset record by record
(`Recordset.to_csv(path)` streams through a buffered file instead of building
one string).

### Performance on large tables

Measured with the `comp-on-large-tables` experiment (the 10 largest ATBench
inputs, 98 thousand to 1.19 million cells, the same RTL solutions for both
engines, one process per run with the `input.csv → output.csv` contract,
medians of 5 runs, peak RSS of the process, Core Ultra 7 155H). On the largest
input (`stack_test11`: 1.19 million cells and 1.19 million output records)
pyRegTab 0.7.2 takes 0.76 s wall and 335 MB peak against 6.1 s and 1.66 GB in
0.7.1, 5.0 s and 2.2 GB for the jRegTab 0.7.1 runner and 1.24 s and 125 MB for
the pandas solution (`melt`); on the other nine inputs it is 3–5× faster than
0.7.1, 5–8× faster than jRegTab and 2.5–10× faster than pandas by wall time,
with 2.5–3.7× less memory than 0.7.1, 5–8× less than jRegTab and about the
memory of pandas. Inside the process on `stack_test11`: CSV → `TableSyntax`
0.07 s (Java 0.18 s), matching 0.18 s (1.10 s), interpretation 0.33 s
(3.30 s), writing 40 MB of CSV 0.04 s (0.25 s). Repeating the data rows up to
4.75 million cells the wall time grows by ×1.8–2.0 per doubling and the peak
reaches 1.27 GB where 0.7.1 needed 6.6 GB and jRegTab 6.2 GB. The design —
one native call for loading, a dense working state with interned attribute
names, one record arena and text shared between cells, items and records,
action templates shared by all anchors of a spec, 32-byte cells, a flat
recordset, streamed output, mimalloc — is documented in
[`plans/PERF_LARGE_TABLES.md`](plans/PERF_LARGE_TABLES.md) and
[`plans/PERF_MEMORY_LAYOUT.md`](plans/PERF_MEMORY_LAYOUT.md).

## Development

```
python -m venv .venv && . .venv/bin/activate   # or .venv\Scripts\activate
pip install maturin pytest
maturin develop --release
pytest tests -q
```

Building the extension needs a C compiler on the `PATH` (the module uses
[mimalloc](https://github.com/microsoft/mimalloc) as its allocator): MSVC or
MinGW-w64 `gcc` on Windows, `cc` on Linux/macOS. The pure-Rust core
(`cargo build --no-default-features`) has no such requirement.

## License

MIT
