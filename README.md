# pyRegTab

RegTab: pattern-driven data extraction from document tables with regular
structure — the Python port of jRegTab with a native Rust core.

pyRegTab compiles **RTL** (Regular Table Language) patterns into abstract
table patterns (ATP), matches them against a table's syntactic layer (ITM),
and interprets the match into a relational **recordset**:

```
TableSyntax → RtlCompiler/TablePattern → AtpMatcher → TableInterpreter → Recordset
```

**pyRegTab 0.1.x ≙ jRegTab 0.4.0** (same API, same semantics, same test
corpus).

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

Named Python predicates are attached to RTL via `EXT('name')`:

```python
from pyregtab import Bindings

p = RtlCompiler.compile(
    "{ [ [EXT('isTotal') ? VAL : ST*->REC] []+ ] }+",
    Bindings.of().cell("isTotal", lambda cell: cell.text.startswith("Total")),
)
```

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

`pytest tests` runs (1 878 tests):

- the full benchmark suite — tasks 001–150 (Foofah, RegTab, Baikal),
  every fixture variant, **both** via RTL patterns and via ATP patterns
  built with the Python spec API (1 500 task variants in total; fixtures
  are copied verbatim from jRegTab into `tests/fixtures/tasks`, ATP
  builders are mechanically translated from the Java tests by
  `tools/translate_atp.py`);
- the RTL conformance corpus (positive canonical forms, fixed points,
  negative rejections);
- RTL↔ATP round-trip for tasks 001–050;
- API unit tests (syntax layer, extractors, EXT bindings, custom
  predicates, transformations, interpreter options, GIL-released batch
  matching from a thread pool).

`cargo test` additionally runs the conformance corpus and an end-to-end
smoke test against the native core alone. Differential testing against the
Java reference (`tools/differential.py` + `tools/RecordsetDumpMain.java`)
compares recordsets cell-by-cell on all 750 task variants — zero
mismatches against jRegTab v0.4.0.

## IDE support

`ide/vscode/` is a VS Code extension (and IntelliJ/PyCharm TextMate bundle)
that highlights `.rtl` files and RTL embedded in Python strings passed to
`RtlCompiler.compile(...)`. See [`ide/README.md`](ide/README.md). RTL is also
validated at compile time: `RtlCompiler.compile(...)` raises `RtlCompileError`
with a `line:col` position on an invalid pattern.

## Development

```
python -m venv .venv && . .venv/bin/activate   # or .venv\Scripts\activate
pip install maturin pytest
maturin develop --release
pytest tests -q
```

## License

Apache-2.0
