# Getting started

## Requirements

- Python 3.10 or later (binary wheels for Windows, Linux, macOS; x86-64 and arm64)

## Installation

```
pip install pyregtab
```

Building from the sdist (exotic platforms) additionally requires a Rust toolchain.

## Core concepts

pyRegTab extracts structured records from a table in three steps:

1. **Describe the table structure** — write a pattern (either as Python objects or as an RTL string).
2. **Match** the pattern against the table — `AtpMatcher.match(pattern, syntax)`.
3. **Interpret** the match result — `TableInterpreter.interpret(itm)` returns a `Recordset`.

A pattern is an **Abstract Table Pattern (ATP)**. You can construct one in Python using the spec builder API (`TablePattern.of(...)`), or compile one from an **RTL** (Regular Table Language) string — a compact DSL designed for this purpose.

## First example

Consider a cross-tabulation of airline departures by airport:

```
        | CA     | HU
IKT     | 0 Jan  | 8 Feb
SVO     | 31 Jan | 40 Feb
```

The column headers are airlines (`CA`, `HU`), the row headers are airports (`IKT`, `SVO`), and each
body cell holds a compound `"ND MON"` value — a number of departures plus a month. The goal is to
*unpivot* this matrix into a flat recordset `⟨ND, AIRLINE, AIRPORT, MON⟩`:

```
ND | AIRLINE | AIRPORT | MON
0  | CA      | IKT     | Jan
8  | HU      | IKT     | Feb
31 | CA      | SVO     | Jan
40 | HU      | SVO     | Feb
```

### Step 1 — build the table

```python
from pyregtab import TableSyntax

syntax = TableSyntax(3, 3)
syntax.cell(0, 1).set_text("CA");  syntax.cell(0, 2).set_text("HU")
syntax.cell(1, 0).set_text("IKT"); syntax.cell(1, 1).set_text("0 Jan"); syntax.cell(1, 2).set_text("8 Feb")
syntax.cell(2, 0).set_text("SVO"); syntax.cell(2, 1).set_text("31 Jan"); syntax.cell(2, 2).set_text("40 Feb")
# The empty corner cell (0, 0) defaults to "".
```

### Step 2 — write the pattern

**Option A — RTL string** (recommended for readability):

```python
from pyregtab import RtlCompiler

pattern = RtlCompiler.compile("""
    [ [] [VAL : 'AIRLINE'->AVP]+ ]
    [ [VAL : 'AIRPORT'->AVP]
      [VAL : (COL, ROW, CL)->REC, 'ND'->AVP " " VAL : 'MON'->AVP]+ ]+
""")
```

- `[ [] [VAL : 'AIRLINE'->AVP]+ ]` — header subtable: skip the empty corner `[]`, then one-or-more
  column headers, each bound to the attribute `AIRLINE`.
- `[ [VAL : 'AIRPORT'->AVP] … ]+` — data subtable: one-or-more rows whose first cell is bound to `AIRPORT`.
- `[VAL : … " " VAL : 'MON'->AVP]` — the compound body cell is split at the space into two values:
  `ND` (the first segment) and `MON` (the second).
- `(COL, ROW, CL)->REC` — the `ND` value forms one record from the same-column `AIRLINE`,
  the same-row `AIRPORT`, and the same-cell `MON`.

**Option B — Python builder API** (full control):

```python
from pyregtab import (
    ActionSpec, AtomicContentSpec, CellPattern, CompoundContentSpec,
    ItemFilterConditionSpec, ProviderSpec, Quantifier, RowPattern,
    SubtablePattern, TablePattern,
)

same_col = ItemFilterConditionSpec.same_col()
same_row = ItemFilterConditionSpec.same_row()
same_cell = ItemFilterConditionSpec.same_cell()

# Compound body cell: "0 Jan" → ND ("0") + MON ("Jan")
data_cell = CompoundContentSpec.of(
    AtomicContentSpec.val(
        ActionSpec.rec(
            ProviderSpec.val(same_col),    # AIRLINE (column header)
            ProviderSpec.val(same_row),    # AIRPORT (leftmost cell)
            ProviderSpec.val(same_cell),   # MON (same compound cell)
        ),
        ActionSpec.avp("ND"),
    ),
    (" ", AtomicContentSpec.val(ActionSpec.avp("MON"))),
)

pattern = TablePattern.of(
    # Header subtable: skip the empty corner + one-or-more airline cells
    SubtablePattern.of(
        RowPattern.of(
            CellPattern.skip(),
            CellPattern.of(Quantifier.one_or_more(),
                           AtomicContentSpec.val(ActionSpec.avp("AIRLINE"))),
        )
    ),
    # Data subtable: one-or-more rows of airport cell + one-or-more body cells
    SubtablePattern.of(
        RowPattern.of(Quantifier.one_or_more(),
                      CellPattern.of(AtomicContentSpec.val(ActionSpec.avp("AIRPORT"))),
                      CellPattern.of(Quantifier.one_or_more(), data_cell)),
    ),
)
```

### Step 3 — match and interpret

```python
from pyregtab import AtpMatcher, SchemaConstructionStrategy, TableInterpreter

itm = AtpMatcher.match(pattern, syntax)
if itm is None:
    print("Pattern did not match.")
    raise SystemExit(1)

rs = (
    TableInterpreter()
    .with_strategy(SchemaConstructionStrategy.RECORD_FIRST)
    .interpret(itm)
)

print(rs.schema.attributes)  # ['ND', 'AIRLINE', 'AIRPORT', 'MON']
for record in rs.records:
    print(f"{record['AIRPORT']}/{record['AIRLINE']}: {record['ND']} ({record['MON']})")
# IKT/CA: 0 (Jan)
# IKT/HU: 8 (Feb)
# SVO/CA: 31 (Jan)
# SVO/HU: 40 (Feb)
```

## What's next

- [RTL reference](rtl-reference.md) — complete syntax for the RTL DSL.
- [ITM](model/itm.md) — syntactic and semantic layers, working state, table interpretation.
- [ATP](model/atp.md) — pattern hierarchy, content specs, matching algorithm.
- [API reference](api.md) — public classes and methods.
