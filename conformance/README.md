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
└── negative/
    └── <name>.rtl             — must be rejected with a compile error
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

Byte-equality of canonical forms transitively guarantees that two implementations build
the same ATP without comparing object graphs across languages.

In jRegTab the contract is executed by `ru.icc.regtab.conformance.RtlConformanceTest`;
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

Commit the result. Negative cases are maintained by hand — when adding a new error
branch to the grammar or compiler, add a case here.

## Evolving RTL

Any change to the RTL language follows this order:

1. Change the grammar `RTL.g4` (the normative specification) in jRegTab.
2. Add/extend corpus cases (positive with canonical forms, negative for new error branches).
3. Implement in the jRegTab compiler; CI (`conformance` job) must be green.
4. Downstream implementations update their pinned upstream commit, sync the corpus copy,
   and implement the change; their conformance suite must be green.
