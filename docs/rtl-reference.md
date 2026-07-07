# RTL Reference

RTL (Regular Table Language) is a compact textual DSL that compiles to ATP via `RtlCompiler.compile(rtl)`.
The normative grammar is at `grammar/RTL.g4` (pinned from jRegTab).
RTL tokens are case-insensitive.

!!! tip "Where the examples come from"
    Most RTL snippets on this page are taken verbatim from the project's test suite
    (`src/test/java/ru/icc/regtab/rtl/RtlTask*Test.java`). Each task test compiles its RTL,
    matches it against fixture tables, and asserts the extracted recordset — so the patterns
    shown here are known to compile and run.

---

## RTL, ATP, and the compiler

RTL is **not** a separate model — it is a concrete surface syntax for [ATP](model/atp.md).
Every RTL string denotes an ATP `TablePattern`, and the two are connected by a compiler and a
serializer:

```mermaid
flowchart LR
    A["RTL string"]
    B["TablePattern (ATP)"]
    C["RTL string"]
    A -->|"RtlCompiler.compile()"| B
    B -->|"AtpToRtlSerializer.serialize()"| C
```

**Compilation** (`RtlCompiler`) runs the native hand-written lexer/parser (structurally following the grammar) over
the RTL grammar, then builds the pattern from the AST:

| Stage | Class | Role |
|---|---|---|
| Parse | hand-written lexer + recursive-descent parser (follows `RTL.g4`) | RTL text → AST |
| Build ATP | `ATPBuilder` | AST → `TablePattern` and nested pattern objects |
| Resolve providers | `ProviderTemplateResolver` | provider templates + constraints → `ProviderSpec` |
| Settings / inline transforms | `RtlCompiler` | `<NORM, ANCH(n), SPLIT("s")>` and inline `REC(n)`/`REC('s')` → `RecordsetTransformation` list attached to the `TablePattern` |

On any lexer or parser error, `compile` throws `RtlCompileError` carrying the line and
column. The compiler also validates cross-cutting constraints — e.g. conflicting `ANCH(n)` /
`REC(n)` anchor positions, or conflicting `SPLIT` / `REC('s')` delimiters — and raises
`RtlCompileError` for those too.

The result is exactly the same kind of `TablePattern` you could build with the Python fluent API
(`TablePattern.of(...)`); see the [ATP page](model/atp.md) for the object model. Once compiled,
a pattern is matched and interpreted the usual way:

```python
pattern = RtlCompiler.compile(rtl)
itm = AtpMatcher.match(pattern, syntax)
rs = pattern.transform(
    TableInterpreter()
    .with_strategy(SchemaConstructionStrategy.RECORD_FIRST)
    .interpret(itm)
)
```

**Serialization** (`AtpToRtlSerializer.serialize(pattern)`) performs the
reverse mapping, turning an ATP `TablePattern` back into an RTL string. It is the tool used to
cross-check the two representations. Two limitations apply: actions are emitted at the atom level
only (inherited-level action specs are not reconstructed), and custom Python predicates
(`CellPredicate.custom`, `ItemFilterConditionSpec.custom`) cannot be serialized. Named external
bindings (`EXT('name')`, see [External Python bindings](#external-python-bindings-extname)) *are*
serializable — the name identifies the binding; compiling the output back requires the same
`Bindings` object.

---

## Pattern structure

```
tablePattern     : [cond ?] [<settings>] [acts] subtablePattern+

subtablePattern  : rowPattern+                          // implicit (no braces)
                 | { [cond ?] [acts] rowPattern+ } q?   // explicit

rowPattern       : [ [cond ?] [acts] subrowPattern+ ] q?

subrowPattern    : cellPattern+                         // implicit
                 | { [cond ?] [acts] cellPattern+ } q?  // explicit

cellPattern      : [ ] q?                               // skip cell
                 | [ cellPatternBody ] q?

cellPatternBody  : cond ? [acts] contSpec   // guarded: ? required when contSpec follows
                 | cond                    // condition-only: no ? (skip cell with guard)
                 | [acts] contSpec          // unguarded (condContSpec bare form included)
```

The bracket style encodes the hierarchy level: `[ … ]` wraps rows and cells, `{ … }` wraps
subtables and subrows. A minimal flat table is a sequence of row patterns, each a sequence of
cell patterns:

```
[ [VAL] [VAL : SR->REC(1)]{2} ]+
```

*(Task 03 — a row-key anchor followed by exactly two value cells per row.)*

**Quantifiers** (suffix on any `{ }` or `[ ]` block):

| Syntax | Meaning |
|---|---|
| *(absent)* | exactly 1 |
| `?` | 0 or 1 |
| `*` | 0 or more |
| `+` | 1 or more |
| `{n}` | exactly *n* |

The `+` on the outer row pattern above means "one or more data rows"; the `{2}` means "exactly
two value cells".

**Inherited action specs** — `[acts]` placed at the table, subtable, row, or subrow level are
inherited by all descendant cells. Inherited actions are merged with any local actions on the
cell's `contSpec`. Incompatible inherited actions (e.g. `COL->AVP` on an `ATTR` anchor) are
silently skipped. For instance, in Task 116 the row-level action `'LOCATION'->AVP` is inherited
by every cell of that header row:

```
[ 'LOCATION'->AVP [] [$V1]{4} [VAL] [] … ]
```

---

## Settings prefix

Optional prefix before the first subtable pattern: `<setting, …>`. Settings compile to
post-extraction `RecordsetTransformation`s on the resulting `TablePattern`.

| Setting | Effect |
|---|---|
| `NORM` | Apply whitespace normalisation to all field values after extraction |
| `ANCH(n)` | Use position *n* in the first record as the attribute name for all records |
| `SPLIT("s")` | Split all field values by delimiter *s* after extraction |

Example: `<NORM, ANCH(2)> [ … ]` — normalise and anchor at position 2.

!!! note "Inline equivalents"
    The same two transformations can be requested *inline* on a `REC` action:
    `REC(n)` is equivalent to the `ANCH(n)` setting, and `REC('s')` is equivalent to
    `SPLIT("s")`. Inline forms are by far the more common in practice (see Tasks 02, 03).
    The compiler merges inline and prefix forms and raises `RtlCompileError` if they
    conflict (e.g. `ANCH(1)` together with `REC(2)`).

---

## Cell match conditions

A cell match condition guards pattern application; it tests the **cell**, not the item.

| Syntax | Condition |
|---|---|
| `"regex" ?` | Cell text matches the Java regex |
| `!"regex" ?` | Cell text does not match |
| `BLANK ?` | Cell text is blank |
| `!BLANK ?` | Cell text is not blank |
| `~"sub" ?` | Cell text contains the substring |
| `EXT('name') ?` | Cell satisfies the Python predicate bound under `name` — see [External Python bindings](#external-python-bindings-extname) |

The `?` separator is required when a `contSpec` follows the condition (guarded form).
When the cell body contains **only** a condition and nothing else, `?` must be omitted:

| Form | Meaning |
|---|---|
| `[!BLANK ? VAL : …]` | Guarded cell — match non-blank, derive VAL |
| `[!BLANK]` | Condition-only skip cell — consume non-blank, produce no item |
| `[BLANK]` | Condition-only skip cell — consume blank, produce no item |

Examples from the test suite:

```
[ [!BLANK? VAL] [!BLANK? (VAL : SR&C0->REC(1)){','}] ]+
```

*(Task 45 — both cells of each row are guarded as non-blank; the second is also a delimited
cell.)*

```
[ [BLANK] [] ]?
```

*(Task 02 — an optional footer row whose first cell must be blank; `[BLANK]` is a condition-only
skip cell, `?` quantifies the whole row.)*

A regex guard at row level selects header/data rows by content:

```
[ ['20\d\d' ? VAL: 'YEAR'->AVP] … ]+
```

*(Task 116 — the leading cell of every data row must look like a 4-digit year.)*

---

## External Python bindings — `EXT('name')`

`EXT('name')` is the escape hatch from RTL into Python: it references a named predicate supplied
alongside the RTL string via `ru.icc.regtab.rtl.Bindings`. The same syntax works in two
positions, resolved by where it appears:

| Position | Binding kind | Python callable |
|---|---|---|
| Cell match condition (`EXT('n') ?`, `[EXT('n')]`) | `Bindings.cell(name, …)` | `Predicate<Cell>` |
| Provider constraint (`(ROW & EXT('n'))->REC`) | `Bindings.filter(name, …)` | `BiPredicate<CellDerivedItem, CellDerivedItem>` (anchor, candidate) |

```python
import re
from pyregtab import Bindings, RtlCompiler

p = RtlCompiler.compile(
    "{ [ [EXT('isTotal') ? VAL : (ROW & EXT('isNum'))*->REC] [VAL]+ ] }+",
    Bindings.of()
    .cell("isTotal", lambda c: c.text.startswith("Total"))
    .filter("isNum", lambda a, c: re.fullmatch(r"\d+", c.str) is not None),
)
```

Rules:

- Referencing a name that is not bound (or bound under the other kind) raises
  `RtlCompileError` at compile time, with the position of the `EXT` constraint.
- The two kinds form independent namespaces; a `Bindings` object may carry bindings that a
  particular pattern does not use.
- Unlike opaque `Custom` predicates, `EXT` constraints survive ATP→RTL serialization
  (they serialize back to `EXT('name')`); recompiling the output requires the same `Bindings`.

---

## Content specifications

### Atomic — `contSpec`

```
itemDerivDir [tags] [= strExtr] [: actSpecs]
```

| `itemDerivDir` | Meaning |
|---|---|
| `VAL` | Value-associated item |
| `ATTR` | Attribute-associated item |
| `AUX` | Auxiliary item |
| `SKIP` or `_` | No item derived (cell is consumed but ignored) |

A plain `VAL` with a single action is the most common atom:

```
[VAL : ST*->REC]
```

*(Task 01 — derive a value and collect all same-subtable values into one record.)*

**Tag annotation** (user-defined tags, for use with `TAG` filter):

```
VAL #tag1 #tag2
```

Tags let a later provider find exactly the right items. In Task 107 header values are tagged
`#H` (column headers) and `#S` (row headers) so that data cells can gather them:

```
[!BLANK ? VAL#'H']            — tag a column-header value
VAL: (COL&#'H'*, ROW&#'S'*)->REC  — collect tagged headers into the record
```

**String extractor** (after `=`):

| Extractor | Effect |
|---|---|
| `NORM` | Collapse whitespace |
| `UC` | To upper case |
| `LC` | To lower case |
| `TRIM` | Trim |
| `SUBSTR(n,m)` | Substring starting at position *n*, length *m* |
| `REPL("a","b")` | Replace *a* with *b* (Java regex) |

Extractors can be chained with `.`: `=REPL(" ","_").LC`. Real uses:

```
[VAL=NORM] [] ]{2}                 — normalise whitespace in header cells (Task 02)
[VAL=SUBSTR(0,4): 'YEAR'->AVP]+    — keep the first 4 chars as the year (Task 127)
[VAL=TRIM: 'UNIT'->AVP]            — trim the unit token (Task 127)
```

**Action specs** (after `:`):

```
(prov1, prov2, …)->op
```

or with a single provider:

```
prov->op
```

### Delimited

```
(VAL [tags] [= extr] [: acts]){"sep"}
```

Splits the cell text by `"sep"` and derives one item per token.

```
[!BLANK? (VAL : SR&C0->REC(1)){','}]
```

*(Task 45 — a cell like `"a,b,c"` yields three VAL items, each forming a record bound to the
row key via `SR & C0`.)*

### Compound

```
["open"] VAL [acts] "sep" VAL [acts] ["close"]
```

Matches a cell whose text is a sequence of segments separated by fixed delimiters.
Opening and closing delimiters are optional.

```
VAL: (COL,ROW,CL)->REC, 'ND'->AVP ' ' VAL: 'MON'->AVP
```

*(Illustrative example — a cell like `"0 Jan"` is split on the space into a count and a month,
each becoming its own VAL item with its own actions.)*

A three-part compound with `-` and a newline as delimiters:

```
VAL: 'MIN'->AVP '-' VAL: 'MAX'->AVP '\n' VAL: 'AVE'->AVP, (CL*,ROW&C1)->REC
```

*(Task 126 — a cell like `"12-48\n30"` yields MIN, MAX and AVE items.)*

### Conditional

```
cond ? trueSpec | falseSpec
```

Branches on a cell match condition; both branches must be `atomContSpec`, `delimContSpec`, or `compContSpec`.

Parentheses are **not allowed**; the bare form is the only valid syntax:

```
[BLANK ? _ | VAL]                    — skip blank cells, derive VAL otherwise
[RT*->REC BLANK ? _ | VAL]           — with preceding actSpec
```

A common idiom is to skip empty/dash-only cells and otherwise parse a compound value:

```
['\s*-?\s*' ? _ | VAL: 'MIN'->AVP '-' VAL: 'MAX'->AVP '\n' VAL: 'AVE'->AVP, (CL*,ROW&C1)->REC]+
```

*(Task 126 — cells matching `\s*-?\s*` (blank or a lone dash) are skipped; the rest are parsed as
MIN-MAX/AVE compounds.)*

---

## Action specifications

```
provSpecs -> op
```

`provSpecs` is a single provider spec, a parenthesised comma-separated list, or empty parentheses
`()` (no additional providers — anchor only).

| Operation | Syntax | Effect |
|---|---|---|
| `REC` | `prov->REC` | Anchor item → record entry; provider supplies additional field values |
| `REC` | `()->REC` | Anchor item → single-field record (no additional providers; useful after `SUFFIX`/`PREFIX`/`FILL` has enriched the anchor value) |
| `REC(n)` | `prov->REC(n)` | Same + use attribute at position *n* as the record's attribute name |
| `REC('s')` | `prov->REC('s')` | Same + split field values by delimiter *s* |
| `AVP` | `prov->AVP` | Associate anchor (VAL) with an attribute from the provider (ATTR) |
| `JOIN` | `prov->JOIN` | Join item-based records: all items included, then dedup by named attribute (K=∅) |
| `JOIN(K)` | `prov->JOIN(0)` | Join with key positions K dropped from each joined record before dedup (e.g. `JOIN(0)` drops the anchor position) |
| `FILL('s')` | `prov->FILL('/')` | Fill anchor value forward from provider, separated by *s* |
| `PREFIX('s')` | `prov->PREFIX(' ')` | Prepend provider value to anchor, separated by *s* |
| `SUFFIX('s')` | `prov->SUFFIX(' ')` | Append provider value to anchor, separated by *s* |

Examples by operation:

```
[VAL : ST*->REC]                        — REC, collect whole subtable (Task 01)
[VAL : SR->REC(1)]{2}                   — REC(1), name the record by attribute at position 1 (Task 03)
[VAL: 'AIRLINE'->AVP]                   — AVP with a literal attribute (Illustrative example)
[VAL: -AV->PREFIX(', ')]                — PREFIX: prepend the value above, separator ", " (Task 116)
[BLANK ? VAL#'H': -LT&!BLANK->FILL | …] — FILL: copy the nearest non-blank cell to the left (Task 107)
```

`PREFIX`/`SUFFIX`/`FILL` followed by `()->REC` is the idiom for building a single-field record out
of an anchor value that was first enriched from neighbours.

---

## Provider specifications

### Cell-derived provider (tblProvSpec)

```
[traversal] (spatConstr | spatConstr & constr & … | (constraints)) [cardinality]
```

**Traversal order** (prefix, default = ROW_MAJOR):

| Symbol | Order |
|---|---|
| *(absent)* | ROW_MAJOR |
| `-` | REVERSE_ROW_MAJOR |
| `^` | COLUMN_MAJOR |
| `-^` | REVERSE_COLUMN_MAJOR |

The reverse traversal `-` is what makes `-AV` ("the nearest cell above") and `-LT` ("the nearest
cell to the left") pick the *closest* neighbour rather than the farthest.

**Spatial constraints** (single bare form, bare `&`-conjunction, or inside parentheses):

| Token | Condition |
|---|---|
| `ST` | `same_subtable(a) && !same_cell(a)` |
| `SR` | `same_subrow(a) && !same_cell(a)` |
| `SC` | `same_subcol(a) && !same_cell(a)` |
| `CL` | `same_cell(a)` |
| `NCL` | `!same_cell(a)` |
| `ROW` | `same_row(a) && !same_cell(a)` |
| `COL` | `same_col(a) && !same_cell(a)` |
| `RT` | `same_subrow(a) && col > col(a)` |
| `LT` | `same_subrow(a) && col < col(a)` |
| `BW` | `same_subcol(a) && row > row(a)` |
| `AV` | `same_subcol(a) && row < row(a)` |

**Positional constraints** (spatial, absolute or relative):

| Token | Condition |
|---|---|
| `Cn` | `col == n` |
| `C+n` / `C-n` | `col == col(a) + n` |
| `Ca..b` | `a ≤ col ≤ b` (absolute) |
| `C+a..b` | `col(a)+a ≤ col ≤ col(a)+b` (relative) |
| `Rn` | `row == n` |
| `R+n` / `R-n` | `row == row(a) + n` |
| `Pn` | `index == n` |
| `P+n` / `P-n` | `index == index(a) + n` |
| `Pa..b` | `a ≤ index ≤ b` |

**Content constraints** (used inside parentheses, combined with `&` / `|`):

| Token | Condition |
|---|---|
| `"regex"` | `str.matches(regex)` |
| `!"regex"` | `!str.matches(regex)` |
| `~"sub"` | `str.contains(sub)` |
| `!~"sub"` | `!str.contains(sub)` |
| `BLANK` | `blankStr()` |
| `!BLANK` | `!blankStr()` |
| `TAG #t1 #t2` | any of the given tags matches (OR) |
| `!TAG #t1 #t2` | none of the given tags matches |
| `STR` | `sameStr(a)` (same string as the anchor) |
| `EXT('name')` | Python item filter bound under `name` — see [External Python bindings](#external-python-bindings-extname) |

When a content constraint is attached to a spatial one with `&`, the `#'tag'` shorthand stands
for `TAG #tag`: `COL&#'H'*` means "same column **and** tagged `H`, unbounded".

**Compound constraints** with `&` and `|`:

Parentheses are required for `|`-disjunctions and for constraints starting with a string literal.
For `&`-conjunctions starting with a `spatConstr` keyword, parentheses are **optional**:

```
RT&P0              — right-of AND position 0  (no parens needed)
CL&P2              — same-cell AND position 2
-LT&P0             — reverse-traversal, left-of AND position 0
(ST & !BLANK)      — same subtable AND not blank
(ROW | COL)        — same row OR same column  (parens required for |)
('regex' & RT)     — parens required: starts with a string literal
```

**Cardinality** (suffix, applies after the whole constraint expression):

| Token | Meaning |
|---|---|
| *(absent)* | at most 1 (default) |
| `{n}` | at most *n* |
| `*` | unbounded |

Examples from the test suite:

- `ST*` — all items in the same subtable (Task 01)
- `SR&C0` — the single same-subrow item in column 0 (Task 45)
- `(SC{2}, SR)` — two items from the same subcolumn (headers) plus one from the same subrow (Task 02)
- `COL&R1..3*` — all same-column items at rows 1–3, unbounded (Task 116)
- `(CL*, ROW&C1)` — all same-cell items plus the same-row item in column 1 (Task 126)

### Context-derived provider (literal)

A quoted string literal supplies a fixed string as an attribute or value:

```
('AIRLINE')->AVP
```

The item type is inferred from the action: `->AVP` → ATTR, `->REC` → VAL.

---

## Frequently used combinations

| RTL | Meaning |
|---|---|
| `ST*->REC` | Collect all same-subtable values into one record |
| `(SC{2}, SR)->REC(2)` | Two items from same subcolumn (headers) + one from same subrow |
| `^COL->AVP` | Associate with an attribute from the same column (column-major) |
| `('LABEL')->AVP` | Associate with a fixed string attribute |
| `(ST*)->REC` (in parentheses) | Same as `ST*->REC` but explicit grouping |
| `CL->JOIN(0)` | Join (drop anchor) another item from the same cell |
| `(COL)->FILL('/')` | Fill forward from same-column values, delimiter `/` |
| `-AV->PREFIX(', ')` | Prepend the nearest value above, separator ", " |

---

## Named fragment definitions

Repeated sub-patterns can be extracted into **named fragments** declared in a preamble
before the first `[` or `{` of the pattern body.

### Syntax

```
$NAME = fragmentBody
```

`$NAME` is a `$`-prefixed alphanumeric identifier (case-insensitive).
`fragmentBody` uses the same bracket style as the level it represents:

| Level | Definition | Reference |
|---|---|---|
| cell | `$N=[cellBody]` | `[$N]` in cell position |
| row | `$N=[ [sub]+ ]` | `[$N]` in row position |
| subrow | `$N={ [cell]+ }` | `{$N}` in subrow position |
| subtable | `$N={ [row]+ }` | `{$N}` in subtable position |

The reference can carry its own quantifier independently of the definition:

```
$V=[VAL: 'X'->AVP]
[ [$V]{4} [$V] ]   — four then one cell of the same form
```

### Semantics

Each reference is a **syntactic substitution**: it expands to a fresh pattern object equivalent
to the inline form. Fragment bodies inherit `actSpecs` from their call-site context, exactly as
inline patterns would. Forward references within the same preamble are allowed.
A reference to an undefined name throws `RtlCompileError` at compile time.

on the Examples page for a complete worked example with two reused cell fragments.
