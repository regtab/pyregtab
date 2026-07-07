# Table patterns (ATP)

The **Abstract Table Pattern (ATP)** is the formal model for specifying *classes* of
tables.  An ATP instance describes, in abstract and syntax-independent terms:

- what the members of a table class look like — through a hierarchy of patterns and
  cell match conditions; and
- what they mean — through interpretation action specifications embedded at the
  cell level.

The relationship between ATP and [ITM](itm.md) is dual and complementary.  An ITM
instance is a single concrete table; an ATP instance is a description of all possible
realisations of a table class.  *Matching* bridges the two: given an ATP instance and
an ITM instance whose syntactic layer is populated but whose semantic layer is absent,
matching checks whether the table belongs to the class and, if so, uses the ATP's
action specifications to automatically populate the semantic layer.

---

## Pattern hierarchy

An ATP instance mirrors the row-oriented substructure hierarchy of ITM:

```
TablePattern
└── SubtablePattern+    (quantifier, optional condition)
    └── RowPattern+     (quantifier, optional condition)
        └── SubrowPattern+  (quantifier, optional condition)
            └── CellPattern+  (quantifier, optional condition, optional content spec)
```

At each level a pattern specifies:

- an optional **cell match condition** `λ` constraining the cells in the matched
  (sub)structure;
- an optional **quantifier** `q` controlling how many consecutive (sub)structures
  of the same kind are matched at that level;
- child patterns (or, at the cell level, a content specification).

Conditions from ancestor patterns compose *conjunctively*: a cell is admissible
only if it satisfies the conditions of its cell pattern, its subrow pattern, its
row pattern, its subtable pattern, and the top-level table pattern.

### Quantifiers

| Quantifier | Meaning |
|---|---|
| `?` | zero or one occurrence |
| `1` (default) | exactly one occurrence |
| `{n}` | exactly `n` occurrences (`n ≥ 2`) |
| `+` | one or more occurrences |
| `*` | zero or more occurrences |

Quantifiers have identical semantics at every level of the hierarchy.

??? note "API mapping — pattern hierarchy and quantifiers"
    **Definition (ATP instance):** `P_tbl = (q, λ, ⟨P_st¹, …⟩)`.

    | Formal concept | Python class | Constructor |
    |---|---|---|
    | Table pattern `P_tbl` | `TablePattern` | `TablePattern.of(subtablePatterns…)` |
    | Subtable pattern `P_st` | `SubtablePattern` | `SubtablePattern.of(rowPatterns…)` |
    | Row pattern `P_row` | `RowPattern` | `RowPattern.of(subrowOrCellPatterns…)` |
    | Subrow pattern `P_sr` | `SubrowPattern` | `SubrowPattern.of(cellPatterns…)` |
    | Cell pattern `P_cell` | `CellPattern` | `CellPattern.of(contentSpec)`, `CellPattern.skip()` |

    Quantifier factories (`Quantifier` record):

    | Notation | Factory |
    |---|---|
    | `?` | `Quantifier.zero_or_one()` |
    | default / `1` | `Quantifier.one()` (implied if omitted) |
    | `{n}` | `Quantifier.exactly(n)` |
    | `+` | `Quantifier.one_or_more()` |
    | `*` | `Quantifier.zero_or_more()` |

    Pass the quantifier as the first argument:
    `CellPattern.of(Quantifier.one_or_more(), contentSpec)`,
    `RowPattern.of(Quantifier.zero_or_more(), cellPatterns…)`.

---

## Cell match conditions

A **cell match condition** `λ : C → {true, false}` is a predicate on cells
expressed as a finite Boolean combination of *atomic constraints* of the form
`property θ value`, where `property` is any cell property from the syntactic
layer, `θ` is a comparison or matching operator, and `value` is a constant.

When used in a table, subtable, row, or subrow pattern, `λ` must hold for *every*
cell belonging to the matched (sub)structure.  When used in a cell pattern, it must
hold for that individual cell only.

Commonly used atomic constraints (see
[ITM — Layout hierarchy](itm.md#layout-hierarchy) for the full property list):

| Constraint | Meaning |
|---|---|
| `txtBlank = true` / `txtBlank = false` | cell is blank / non-blank |
| `txt matches "regex"` | cell text matches a regular expression |
| `col = n` | cell is in column `n` |
| `tags anyMatch {t1, t2}` | cell item carries at least one of the listed tags |

In RTL notation, a condition appears inside `[ ]` at the cell level and inside
`{ }` (or `[ ]`) at higher levels, separated from the content specification by
`?`.

??? note "API mapping — CellMatchCondition and CellPredicate"
    | Concept | Python |
    |---|---|
    | Cell match condition `λ` | `CellMatchCondition` wraps a `CellPredicate` |
    | `txtBlank = true` | `CellPredicate.blank()` |
    | `txtBlank = false` | `CellPredicate.not_blank()` |
    | `txt matches "pat"` | `CellPredicate.regex_matched(pattern)` |
    | `txt not matches "pat"` | `CellPredicate.not_regex_matched(pattern)` |

    Conditions are composed using `CellMatchCondition.and(…)` and
    `CellMatchCondition.or(…)`.

---

## Content specifications

A **content specification** describes the items to be derived from a matched cell
and the interpretation actions to be instantiated upon those items.  Four kinds
exist, from simplest to most expressive.

### Atomic content specification

An **atomic content specification** describes exactly *one* item derived from a cell:

```
S_atom = (idd, ξ, u⃗, ⟨S_act¹, …, S_actᵐ⟩)
```

| Component | Description |
|---|---|
| `idd` | Item derivation directive: `VAL`, `ATTR`, `AUX`, or `SKIP` |
| `ξ` | Optional string extractor applied to the raw cell text before creating the item string |
| `u⃗` | Optional sequence of user-defined tags attached to the derived item |
| `S_act¹ … S_actᵐ` | Sequence of interpretation action specifications (may be empty) |

When `idd = SKIP`, no item is derived and no actions are instantiated; the cell is
consumed but ignored.

The derived item becomes the *anchor item* for all action specifications in the
list.

**String extractors** (`ξ`) transform the raw cell text before item creation.
Extractors are applied in order and may be chained:

| Extractor | RTL notation | Effect |
|---|---|---|
| Whitespace normalisation | `=NORM` | Trim + collapse internal whitespace |
| Upper case | `=UC` | Convert to upper case |
| Lower case | `=LC` | Convert to lower case |
| Trimmed | `=TRIM` | Strip leading/trailing whitespace only |
| Substring | `=SUBSTR(n,m)` | Extract characters at positions `n` to `m` |
| Replaced | `=REPL("a","b")` | Replace all occurrences of `"a"` with `"b"` |

Chained example: `=REPL("x","").NORM` — first removes `"x"`, then normalises whitespace.

??? note "API mapping — AtomicContentSpec and StringExtractor"
    | Concept | Python |
    |---|---|
    | `idd = VAL` | `AtomicContentSpec.val(actionSpecs…)` |
    | `idd = ATTR` | `AtomicContentSpec.attr(actionSpecs…)` |
    | `idd = AUX` | `AtomicContentSpec.aux(actionSpecs…)` |
    | `idd = SKIP` | `AtomicContentSpec.skip()` or `CellPattern.skip()` |

    StringExtractor constants and factories:

    | RTL notation | Python |
    |---|---|
    | `=NORM` | `StringExtractor.whitespace_normalized()` |
    | `=UC` | `StringExtractor.upper_case()` |
    | `=LC` | `StringExtractor.lower_case()` |
    | `=TRIM` | `StringExtractor.trimmed()` |
    | `=SUBSTR(n,m)` | `StringExtractor.substring(n, m)` |
    | `=REPL("a","b")` | `StringExtractor.replaced("a", "b")` |
    | chained `=X.Y` | `StringExtractor.chain(x, y)` |

    Pass the extractor to the spec factory:
    `AtomicContentSpec.val(StringExtractor.whitespace_normalized(), actionSpecs…)`.

### Delimited content specification

A **delimited content specification** splits the cell text by a delimiter `δ` and
applies the same atomic specification to each resulting substring:

```
S_delim = (δ, S_atom)
```

If the cell text decomposes as `s₁ · δ · s₂ · δ · … · δ · sₙ`, then `S_atom` is
applied independently to each `sₖ`, deriving one item per substring.  This is
used, for example, when a single cell contains a comma-separated list of values.

??? note "API mapping — DelimitedContentSpec"
    ```python
    DelimitedContentSpec.of(atomSpec, delimiter)
    ```

    Example — comma-separated list of VAL items, each tagged AVP to `"CODES"`:
    ```python
    DelimitedContentSpec.of(
        AtomicContentSpec.val(ActionSpec.avp("CODES")),
        ","
    )
    ```

### Compound content specification

A **compound content specification** describes a cell whose text contains multiple
semantically distinct parts separated by known delimiters:

```
S_comp = (δ₀, S_x¹, δ₁, S_x², δ₂, …, S_xⁿ, δₙ)
```

where each `S_xⁱ` is either an atomic or a delimited content specification, and
each `δᵢ ∈ Σ*` is a (possibly empty) delimiter string.  The raw cell text must
match the pattern `δ₀ · s₁ · δ₁ · s₂ · δ₂ · … · sₙ · δₙ`; each substring `sᵢ` is
passed to the corresponding `S_xⁱ` as its input text.

This is used, for example, when a cell contains a value and a unit separated by a
space: `"42 km"` → `S_atom³` receives `"42"`, `S_atom⁴` receives `"km"`.

??? note "API mapping — CompoundContentSpec"
    ```python
    CompoundContentSpec.of(
        headerAtomSpec,
        Segment.of(delimiter1, atomSpec2),
        Segment.of(delimiter2, atomSpec3),
        …
    )
    ```

    The `headerAtomSpec` corresponds to `S_x¹`; each `Segment(δ, S_x)` corresponds to
    one `(δᵢ, S_xⁱ⁺¹)` pair.  Empty leading delimiter `δ₀` is implied.

### Conditional content specification

A **conditional content specification** selects between two alternative
specifications based on whether the matched cell satisfies a condition `λ`:

```
S_cond = (λ, S_x⁺, S_x⁻)
```

If `c ⊨ λ` then `S_x⁺` governs the cell; otherwise `S_x⁻` governs it.  Each
branch may be atomic, delimited, or compound.

??? note "API mapping — ConditionalContentSpec"
    ```python
    ConditionalContentSpec.of(condition, thenSpec, elseSpec)
    ```

    where `condition` is a `CellMatchCondition` and `thenSpec` / `elseSpec` are any
    content specification instances.

---

## Item provider specifications

An **item provider specification** `S_prov` is a template for constructing an item
provider at match time.  Two forms exist.

**Cell-derived provider specification:**

```
S_prov = (ipt, k, τ, κ)
```

| Component | Description |
|---|---|
| `ipt` | Provider type: `VAL`, `ATTR`, or `AUX` |
| `k` | Cardinality (max items to retrieve); use `UNBOUNDED` for `∞` |
| `τ` | Traversal order: `→` (row-major), `←` (reverse), `↓` (col-major), `↑` (reverse col-major) |
| `κ` | Filter condition — a Boolean combination of spatial and content constraints |

The constructed provider retrieves items of the indicated type that satisfy `κ`
relative to the anchor, ordered by `τ`, up to `k` items.

**Context-derived provider specification:**

```
S_prov = (ipt, s⃗)
```

where `ipt` is the provider type and `s⃗ = ⟨s₁, …, sₙ⟩` is a non-empty sequence
of string constants.  The constructed provider always returns a fixed sequence of
context-derived items regardless of the anchor — effectively injecting constants
into the interpretation.

See [ITM — Item providers](itm.md#item-providers) for the complete `ProviderSpec`
API and the table of 27 atomic filter terms (`FilterTerm`).

---

## Interpretation action specifications

An **interpretation action specification** `S_act` is a template from which a
concrete interpretation action is instantiated at match time, with the derived item
bound as the anchor.  Two forms exist.

**Cell-derived anchor form** (the common case):

```
S_act = (op, ⟨S_prov¹, …, S_provⁿ⟩)
```

**Context-derived anchor form** (the anchor is a string constant):

```
S_act = (op, s, ⟨S_prov¹, …, S_provⁿ⟩)
```

In both forms, `op` is one of the six working-state update operations (`FILL`,
`PREFIX`, `SUFFIX`, `AVP`, `REC`, `JOIN`) and `S_prov¹ … S_provⁿ` are item
provider specifications whose types must satisfy the consistency constraints for the
chosen operation (see [ITM — Interpretation actions](itm.md#interpretation-actions)).

??? note "API mapping — ActionSpec"
    | Operation | Python factory | Notes |
    |---|---|---|
    | `REC` | `ActionSpec.rec(providers…)` | anchor → first field; providers supply remaining fields |
    | `REC(n)` | `ActionSpec.rec(int n, providers…)` | adds `AnchorAttributeAtPosition` post-step |
    | `REC('s')` | `ActionSpec.rec(String delim, providers…)` | adds `DelimitedFieldSplit` post-step |
    | `AVP` | `ActionSpec.avp(provider)` | associates VAL anchor with ATTR item |
    | `AVP "name"` | `ActionSpec.avp("ATTR_NAME")` | context-derived ATTR constant |
    | `JOIN` | `ActionSpec.join(providers…)` | joins records, dedup by named attribute |
    | `JOIN(K)` | `ActionSpec.join(Set.of(k…), providers…)` | joins with key positions K dropped |
    | `FILL` | `ActionSpec.fill(delimiter, providers…)` | fills anchor value from providers |
    | `PREFIX` | `ActionSpec.prefix(delimiter, providers…)` | prepends provider values |
    | `SUFFIX` | `ActionSpec.suffix(delimiter, providers…)` | appends provider values |

    Lambda form: providers may be written as `(anchor, candidate) -> candidate.same_col(anchor)`
    when passed directly to `ActionSpec.rec(…)` (shorthand matching `ProviderSpec.val(…)`).

---

## Matching ATP against ITM

Matching is the process that bridges the ATP and ITM models.  It proceeds in two
stages.

### Stage 1 — Syntactic layer matching

The pattern hierarchy of the ATP instance is structurally matched against the rows
and cells of the ITM instance.  This:

- partitions the rows of the table into subtables, guided by subtable patterns;
- partitions the cells of each row into subrows, guided by subrow patterns;
- produces a correspondence `M` between cell patterns and individual cells.

Matching proceeds *top-down* through the hierarchy using a **greedy strategy with
backtracking**:

1. A pattern with a non-exact quantifier (`+`, `*`, or `?`) attempts to consume as
   many consecutive rows (or cells) as possible.
2. If a subsequent sibling pattern then fails to match, the algorithm backtracks by
   releasing one row (or cell) at a time until a valid assignment is found or all
   possibilities are exhausted.
3. Matching fails if any required pattern cannot be satisfied (minimum count not
   reached, or condition violated).

The table-level condition (if present) is checked first: if any cell in the table
violates it, matching fails immediately.

A match is *complete* at the top level only when *all* rows of the ITM instance
have been consumed by the subtable patterns.

All accumulated pattern–substructure pairs are applied to the ITM instance only
after the entire syntactic matching succeeds; if matching fails, the ITM instance
is left unmodified.

### Stage 2 — Semantic layer construction

After a successful syntactic match, each cell pattern–cell pair `(P_cell, c) ∈ M`
is processed in the order induced by the pattern hierarchy:

**Phase 1 — Content resolution:** the content specification of `P_cell` is applied
to cell `c`:

- An *atomic* spec is used directly, deriving one item from the raw cell text.
- A *delimited* spec splits the cell text by its delimiter and derives one item per
  substring.
- A *compound* spec parses the cell text according to its delimiter structure and
  derives items from each component substring.
- A *conditional* spec evaluates its condition against `c` and applies the
  appropriate branch.

For each resulting atomic spec with `idd ≠ SKIP`, a cell-derived item is created
(with the optional string extractor applied) and added to the ITM instance's item
set of the corresponding type (VAL, ATTR, or AUX).

If the cell text does not conform to the expected structure (e.g. a required
delimiter is missing), phase 1 fails and matching is aborted.

**Phase 2 — Action instantiation:** for each item derived in phase 1, every action
specification in its atomic spec is instantiated into a concrete interpretation
action with the derived item as the anchor.  Context-derived items referenced by
provider specifications are also created at this point and added to the ITM
instance.  The resulting actions are added to the ITM instance's action set `A`.

Once all pairs in `M` are processed successfully, [table
interpretation](itm.md#table-interpretation) is executed.  Matching is
considered successful only if table interpretation produces a valid recordset; if it
fails, the ITM instance is not modified.

---

## End-to-end example

This section shows how to build an ATP instance for a concrete table class and how
it is matched against a table.  The [Table interpretation](itm.md#end-to-end-example)
page continues the trace through working state completion and recordset extraction.

### Table class and goal

Assume a class `C` of tables listing the numbers of departures operated by airlines
from airports in certain months.  All tables share the same structure: the first row
contains an empty first cell followed by one or more airline codes; each remaining
row contains an airport code in the leftmost cell followed by one or more cells each
containing a departure count and a month name separated by a space:

| (empty) | SU     | … | YC    |
|---------|--------|---|-------|
| ARH     | 11 Jan | … | 0 Dec |
| ⋮       | ⋮      | ⋱ | ⋮     |
| YKS     | 27 Jan | … | 0 Dec |

Goal: extract a recordset with schema `S = ⟨ND, AIRLINE, AIRPORT, MON⟩`.

### ATP pattern

The table pattern `P_tbl` consists of a single subtable pattern `P_st¹` with two
row patterns.

**Header row** (`P_row¹`, matched exactly once):

- `P_cell¹` — one skipped cell (the empty top-left corner).
- `P_cell²` — one or more cells (`+`), each matched by atomic spec `S_atom¹`:
  derive a VAL item from the cell text, attach AVP action with context-derived ATTR
  constant `"AIRLINE"`.

**Data rows** (`P_row²`, matched one or more times):

- `P_cell³` — one cell matched by `S_atom²`: derive a VAL item, attach AVP action
  with `"AIRPORT"`.
- `P_cell⁴` — one or more cells (`+`), each matched by compound spec
  `S_comp = (S_atom³, " ", S_atom⁴)`:
    - `S_atom³` (before the space) — derive VAL item for the departure count;
      attach two actions:
        1. REC action with three providers: same-column VAL (airline header),
           same-row VAL (airport), same-cell VAL (month);
        2. AVP action with `"ND"`.
    - `S_atom⁴` (after the space) — derive VAL item for the month; attach AVP
      action with `"MON"`.

### Syntactic match on a concrete table

For the 3 × 3 instance `t₀`:

| (empty) | CA     | HU    |
|---------|--------|-------|
| IKT     | 0 Jan  | 8 Feb |
| SVO     | 31 Jan | 40 Feb |

The subtable pattern `P_st¹` consumes all three rows.  The cell-pattern assignment
is:

|           | col 0            | col 1     | col 2     |
|-----------|------------------|-----------|-----------|
| **row 0** | `P_cell¹` (skip) | `P_cell²` | `P_cell²` |
| **row 1** | `P_cell³`        | `P_cell⁴` | `P_cell⁴` |
| **row 2** | `P_cell³`        | `P_cell⁴` | `P_cell⁴` |

Content spec resolution:

|           | col 0     | col 1                         | col 2                         |
|-----------|-----------|-------------------------------|-------------------------------|
| **row 0** | —         | `S_atom¹`                     | `S_atom¹`                     |
| **row 1** | `S_atom²` | `S_atom³` + `" "` + `S_atom⁴` | `S_atom³` + `" "` + `S_atom⁴` |
| **row 2** | `S_atom²` | `S_atom³` + `" "` + `S_atom⁴` | `S_atom³` + `" "` + `S_atom⁴` |

### Python implementation

```python
TablePattern.of(
  SubtablePattern.of(
    RowPattern.of(
      CellPattern.skip(),
      CellPattern.of(Quantifier.one_or_more(),
        AtomicContentSpec.val(
          ActionSpec.avp("AIRLINE")
        )
      )
    ),
    RowPattern.of(Quantifier.one_or_more(),
      CellPattern.of(
        AtomicContentSpec.val(
          ActionSpec.avp("AIRPORT")
        )
      ),
      CellPattern.of(Quantifier.one_or_more(),
        CompoundContentSpec.of(
          AtomicContentSpec.val(
            ActionSpec.rec(1,
              (a, c) -> c.same_col(a),
              (a, c) -> c.same_row(a),
              (a, c) -> c.same_cell(a)
            ),
            ActionSpec.avp("ND")
          ),
          Segment.of(" ",
            AtomicContentSpec.val(
              ActionSpec.avp("MON")
            )
          )
        )
      )
    )
  )
);
```

This pattern is implemented as `AtpIllustrativeExampleTest` in the pyRegTab test
suite.
