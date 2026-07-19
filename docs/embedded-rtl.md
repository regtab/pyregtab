# Embedded RTL

**Embedded RTL** is a Python DSL (module `pyregtab.dsl`) that mirrors RTL syntax while
remaining ordinary Python code. It combines the brevity of RTL with full Python integration:
callables as constraints, IDE completion, and pattern composition with plain variables and
functions.

Patterns built with embedded RTL produce **exactly the same `TablePattern` objects** as
`RtlCompiler.compile(...)` — this equivalence is verified task-by-task for a representative
subset of the benchmark corpus (`tests/test_dsl.py`): for every lambda-free pattern the DSL
builds a structurally identical ATP and serializes to the same canonical RTL.

```python
from pyregtab.dsl import *
```

## Quick example

RTL (task 001):

```rtl
{ [ [VAL : ST*->REC] [VAL]{2} []+ ]
  [ []               [VAL]{4} []+ ] }+
```

Embedded RTL:

```python
p = table(
    subtable(
        row(cell(VAL, rec(ST.unbounded())), cell(VAL).exactly(2), skip().one_or_more()),
        row(skip(),                         cell(VAL).exactly(4), skip().one_or_more()),
    ).one_or_more())
```

The result is a plain `pyregtab.TablePattern` — match and interpret it exactly like any other:

```python
from pyregtab import AtpMatcher, TableInterpreter

itm = AtpMatcher.match(p, syntax)
rs = TableInterpreter().interpret(itm)
```

---

## Vocabulary

### Pattern levels and quantifiers

| RTL | Embedded RTL |
|---|---|
| table pattern | `table(subtable…)` |
| `{ rows }` subtable | `subtable(row…)` |
| `[ cells ]` row | `row(cell…)` |
| `{ cells }` explicit subrow | `subrow(cell…)`, mixed with implicit runs: `row(subrow(…), subrow(…))` |
| `+` `*` `?` `{n}` | postfix `.one_or_more()` `.zero_or_more()` `.zero_or_one()` `.exactly(n)` |

A row with plain cells (`row(cell…)`) wraps them into one implicit subrow, exactly like the
compiler.

### Cells and guards

| RTL | Embedded RTL |
|---|---|
| `[]` | `skip()` |
| `[VAL : acts]` | `cell(VAL, acts…)` |
| `[!BLANK ? VAL : acts]` | `cell(not_blank(), VAL, acts…)` |
| `[BLANK]` (condition-only) | `cell(blank())` |
| guards `BLANK` `"re"` `~"s"` (+ `!`) | `blank()` `re("…")` `contains("…")` / `not_blank()` `not_re("…")` `not_contains("…")` |
| — (escape hatch) | `cell(where("desc", lambda c: …), VAL)` |

### Content specifications

| RTL | Embedded RTL |
|---|---|
| `VAL` / `ATTR` / `AUX` / `_` | constants `VAL ATTR AUX SKIP`; atoms `val(acts…)` `attr(…)` `aux(…)` |
| `VAL #'tag'` | `val(…).tagged("tag")` |
| `VAL = NORM` (also `TRIM UC LC REPL SUBSTR`, chains) | `val(…).extract(NORM)`, `repl("rx","rep")`, `substr(b,e)`, `chain(…)` |
| compound `A "d" B` | `val(…).then(" ", val(…))` (chainable) |
| delimited `(VAL : …){","}` | `val(…).split_by(",")` |
| conditional `BLANK ? _ \| VAL` | `when(blank(), SKIP, VAL)` (directives and specs mix freely) |

### Providers and constraints

| RTL | Embedded RTL |
|---|---|
| `LT RT AV BW ROW COL SR SC ST NCL CL STR` | constants with the same names (type `Prov`) |
| `Cn` / `Ca..b` / `C+n`, `C-n` / `C+a..b` / `C+a..*` | `C(n)` / `C(lo,hi)` / `Crel(±n)` / `Crel(lo,hi)` / `CrelFrom(lo)` |
| `Rn Ra..b R±n`, `Pn Pa..b P±n` | `R(…)`, `Rrel(…)`, `P(…)`, `Prel(…)` |
| `#'t'` / `!#'t'` | `tag("t")` / `not_tag("t")` |
| item `"re"`, `~"s"`, `BLANK` (+ `!`) | `item_re item_contains item_blank` / `item_not_re item_not_contains item_not_blank` |
| `&` conjunction | `.and_(…)` |
| `\|` disjunction | `.or_(…)`; distribution matches the compiler: `A.and_(B.or_(C))` ≙ `(A&B)\|(A&C)` |
| `{n}` / `*` cardinality | `.card(n)` / `.unbounded()` |
| `-` / `^` / `-^` traversal | `.reversed()` / `.col_major()` / `.reversed_col_major()` |
| — (escape hatch) | `ROW.where("desc", lambda anchor, candidate: …)` |

!!! note "Why `C(n)` and `Crel(n)` are separate"
    `C(n)` and `C(lo, hi)` are absolute column constraints (one column, or a range); `Crel`
    takes a signed anchor-relative delta: `Crel(1)` ≙ `C+1`, `Crel(-1)` ≙ `C-1`, and
    `Crel(lo, hi)` / `CrelFrom(lo)` give relative and open-ended relative ranges. The naming
    mirrors jRegTab's, where absolute and relative forms must be distinct factories.

### Actions and context providers

| RTL | Embedded RTL |
|---|---|
| `(…)->REC` / `REC(n)` / `REC('s')` | `rec(…)` / `rec(n, …)` / `rec_split("s", …)` |
| `prov->AVP` / `'NAME'->AVP` | `avp(prov)` / `avp("NAME")` |
| `(…)->JOIN` / `JOIN(k)` | `join(…)` / `join(k, …)` |
| `(…)->FILL('d')`, `PREFIX`, `SUFFIX` | `fill("d", …)`, `prefix(…)`, `suffix(…)` (delimiter optional) |
| `'EUR'` context literal | `lit("EUR")` (VALUE under REC/JOIN, ATTRIBUTE otherwise — as in the compiler) |
| `@'K'='V'` | `ctx_avp("K", "V")` |

Provider kinds (VAL/ATTR/UNRESTRICTED) are inferred from the action, exactly as in the RTL
compiler — `rec(ST)` builds a VAL provider, `avp(SC)` an ATTR provider.

### Level-scoped (inherited) actions and conditions

RTL allows action specs and a condition at the table, subtable, row, and subrow level; they are
merged down into every atom below. Embedded RTL mirrors this with `acts(…)` and an optional
leading `CellPredicate`:

```python
# RTL: [ BW*->REC { [ATTR] [VAL] }* ]
row(acts(rec(BW.unbounded())),
    subrow(cell(ATTR), cell(VAL)).zero_or_more())

# RTL: !BLANK ? BW*->REC [ [VAL] ]+
table(not_blank(), acts(rec(BW.unbounded())), subtable(row(cell(VAL)).one_or_more()))
```

### Settings

The RTL settings prefix maps to recordset transformations:

```python
# RTL: <NORM,ANCH(1),SPLIT(",")> …
table(…).with_transformations(norm(), anch(1), split(","))
```

### Fragments are just Python

RTL named fragments `$name=[…]` become plain variables — and gain parameterisation for free:

```python
# RTL: $V=['\d+' ? VAL: (COL&#'H'*,ROW&#'S'*)->REC]  …  [$V]+
v = cell(re(r"\d+"), VAL, rec(COL.and_(tag("H")).unbounded(),
                             ROW.and_(tag("S")).unbounded()))
row(cell(blank()).one_or_more(), v.one_or_more())
```

---

## Escape hatches into Python

The reason embedded RTL exists: anywhere the model accepts a predicate, a plain Python callable
works.

```python
p = table(subtable(row(
    cell(where("is_total", lambda c: c.text.startswith("Total")), VAL,
         rec(ROW.where("is_num", lambda a, c: c.str.isdigit()).unbounded())),
    cell(VAL).one_or_more())))
```

Patterns containing `where(...)` **cannot be serialized back to RTL** (an opaque Python callable
has no RTL analog) and two patterns with distinct callables never compare equal. The serializable
alternative is a named `EXT('name')` binding in the string compiler, which round-trips:

```python
from pyregtab import RtlCompiler, Bindings

p = RtlCompiler.compile(
    "{ [ [EXT('is_total') ? VAL : ST*->REC] []+ ] }+",
    Bindings.of().cell("is_total", lambda cell: cell.text.startswith("Total")),
)
```

---

## Relation to the other two APIs

| | RTL string | Embedded RTL | ATP API |
|---|---|---|---|
| Brevity | ✅✅ | ✅ | ❌ |
| Python callables | via `EXT` + `Bindings` | ✅ direct | ✅ direct |
| IDE completion / structural typing | ❌ | ✅ | ✅ |
| Layer | compiles to ATP | thin sugar over ATP | the model itself |

Embedded RTL is a construction layer only — it adds no expressive power beyond ATP, and every
pattern it builds is an ordinary `TablePattern`. The [ATP API](model/atp.md) remains the
documented low-level layer.
