# RTL conformance corpus

This directory is the **normative test corpus of the RTL language**. Together with the
grammar (`src/main/antlr4/ru/icc/regtab/rtl/RTL.g4`, the normative specification), it
defines what any RTL implementation must accept, reject, and produce. The reference
implementation is the jRegTab compiler; downstream implementations (e.g. the pyRegTab
hand-written parser) pin this repository as upstream and run the same corpus in their CI.

## Layout

```
conformance/
├── VERSION                    — generation date and source note
├── positive/
│   ├── <id>.rtl               — RTL source (UTF-8, no BOM, LF, trailing newline)
│   └── <id>.expected.rtl      — canonical form: serialize(compile(<id>.rtl))
├── negative/
│   └── <name>.rtl             — must be rejected with a compile error
└── semantic/
    └── <case>/                — execution semantics, see below
        ├── pattern.rtl
        ├── input.csv
        ├── expected.csv
        └── options.json       — optional
```

**Byte-exactness caveat:** RTL string literals may contain raw CR/CRLF bytes as
*payload* (e.g. `task_099` uses `'\r\n'` compound delimiters matching CRLF inside
cell text). Line endings of the files themselves are always LF. Tooling that syncs
or checks out the corpus must not perform any end-of-line conversion — in git the
corpus is marked `-text` (see `.gitattributes`).

Positive ids: `task_001` … `task_150` (the benchmark task suite) plus curated extras
(`illustrative` — the paper/README worked example).

## Contract

Any RTL implementation must satisfy, for this corpus:

1. Every `positive/<id>.rtl` **compiles** without errors.
2. `serialize(compile(<id>.rtl))` equals `<id>.expected.rtl` **byte-for-byte**
   (modulo the single trailing newline).
3. The canonical form is a **fixed point**: `serialize(compile(<id>.expected.rtl))`
   equals `<id>.expected.rtl`.
4. Every `negative/<name>.rtl` is **rejected** with a compile error
   (`RtlCompileException` in Java, `RtlCompileError` in Python). Reporting the error
   position is recommended but not normative.
5. For every `semantic/<case>/`, matching `pattern.rtl` against `input.csv` and
   interpreting the result yields a recordset equal to `expected.csv`.

Byte-equality of canonical forms transitively guarantees that two implementations build
the same ATP without comparing object graphs across languages. Items 1–4 stop there,
though: two implementations can agree on the canonical form of a pattern and still
*execute* it differently. Item 5 closes that gap for the behaviours where it matters.

## Semantic cases

A case is a directory under `semantic/` holding:

| File | Role |
|---|---|
| `pattern.rtl` | the RTL pattern (UTF-8, no BOM, LF, trailing newline) |
| `input.csv` | the table to match — no header row, `,` delimiter, `"` quotes, UTF-8 |
| `expected.csv` | the expected recordset, same CSV dialect |
| `options.json` | optional; `attributeOrder`, `recordOrder` (`STRICT`\|`FLEXIBLE`), `expectedHasHeader` |

Cell text is taken from the CSV **verbatim** — quoting is what makes leading and trailing
spaces significant, so `"a, b"` is one cell whose text is `a, b`, and `""` is an empty
cell. Readers must not strip surrounding whitespace.

By default `expected.csv` has **no header row**: its columns are matched positionally
against the schema the pattern produced, in record order. This keeps attribute names
invented by the implementation out of the contract. A case whose pattern names its
attributes (via `AVP`) may set `"expectedHasHeader": true` and put those names in the
first row.

Cases are maintained by hand, like `negative/` — the generator never writes here.
Keep each case minimal and focused on one rule, so that a failure names the rule.

## Semantics of S_delim

The canonical form cannot reveal this rule — both spellings below serialize
identically — so it is pinned by `semantic/` cases. The semantics of the delimited
content specification `S_delim = (δ, S_atom)` (`def:delimited-content-spec`):

- The input text is split on every occurrence of `δ`, keeping trailing empty fields
  (Java `String.split(…, -1)`, Python `str.split(δ)`).
- Each substring `sₖ ∈ Σ*` is passed to `S_atom` **verbatim**. Implementations must not
  trim substrings and must not drop empty ones: `n` substrings always derive `n` items,
  numbered `0..n-1`. `"a, b"` therefore yields `"a"` and `" b"`; `"a,,b"` yields
  `"a"`, `""`, `"b"`.
- Whitespace removal is opt-in, expressed by the atom's string extractor `ξ`:
  `(VAL=TRIM){","}` (or `=NORM`). The extractor applies to each substring separately.
- The same rules apply to a delimited specification nested in a compound one.

Positive case `delim_raw` pins both forms syntactically; the executable checks are
`semantic/delim_raw_tokens` (token whitespace survives),
`semantic/delim_empty_tokens` (empty tokens derive items),
`semantic/delim_trim` (`=TRIM` opts into trimming) and
`semantic/compound_delim_raw` (the same rules for a delimited segment nested in a
compound specification).

> Changed in jRegTab 0.5.0. Earlier versions trimmed each substring and silently
> dropped empty ones; patterns relying on that must add `=TRIM` to the delimited atom.

In jRegTab items 1–4 of the contract are executed by
`ru.icc.regtab.conformance.RtlConformanceTest` and item 5 by
`ru.icc.regtab.conformance.RtlSemanticConformanceTest`;
`ConformanceCorpusFreshnessTest` additionally guards the committed files against drift
from the task test suite.

## Regenerating (jRegTab side)

The positive part is generated from the RTL strings of `RtlTask001Test` … `RtlTask150Test`
plus curated extras listed in `ConformanceCorpus`:

```
mvn test-compile org.codehaus.mojo:exec-maven-plugin:3.5.0:java \
    -Dexec.mainClass=ru.icc.regtab.conformance.ConformanceCorpusGenerator \
    -Dexec.classpathScope=test
```

Commit the result. Negative and semantic cases are maintained by hand — when adding a
new error branch to the grammar or compiler, add a `negative/` case; when changing or
clarifying how a construct *executes*, add a `semantic/` one.

## Evolving RTL

Any change to the RTL language follows this order:

1. Change the grammar `RTL.g4` (the normative specification) in jRegTab.
2. Add/extend corpus cases (positive with canonical forms, negative for new error
   branches, semantic for new or changed execution behaviour).
3. Implement in the jRegTab compiler; CI (`conformance` job) must be green.
4. Downstream implementations update their pinned upstream commit, sync the corpus copy,
   and implement the change; their conformance suite must be green.
