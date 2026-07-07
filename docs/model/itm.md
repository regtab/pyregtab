# Interpretable Table Model (ITM)

The **Interpretable Table Model (ITM)** is the formal model that pyRegTab uses to represent a
document table at two complementary levels: a *syntactic layer* that captures the raw
structure, formatting, and content of cells, and a *semantic layer* that records the items
derived from those cells together with the interpretation actions that relate items to
one another and ultimately to a structured recordset.

A table represented as a complete ITM instance (both layers populated) can be
automatically interpreted to extract a recordset conforming to a target schema.

---

## Syntactic layer

The syntactic layer `L_syn = (C, P)` consists of a finite set of cells `C` and a set of
cell properties `P = P_layout ∪ P_format ∪ P_content`.

### Layout hierarchy

ITM organises a table as a row-oriented hierarchy of nested substructures:

```
Table
└── Subtable+       (one or more consecutive groups of rows)
    └── Row+        (one or more consecutive rows)
        └── Subrow+ (one or more consecutive column slices per row)
            └── Cell+
```

Each cell `c ∈ C` occupies a unique position `pos(c) = (row(c), col(c))` in the
row-column grid.  Merged cells from the source document are *normalised*: a
distinct cell object is created for each covered grid position, with the
formatting and content of the original merged cell replicated in every copy.  The
original bounding box is preserved in the `bbox(c)` property so that the merge
geometry is not lost.

??? note "API mapping — syntactic layer"
    **Definition (Syntactic layer):** `L_syn = (C, P)` where `C` is the finite set of
    cells and `P = P_layout ∪ P_format ∪ P_content`.

    | Formal concept | Python class / field |
    |---|---|
    | Cell set `C` | `TableSyntax` — cells accessed via `cell(row, col)` |
    | Primary position `pos(c) = (row, col)` | `Cell.row()`, `Cell.col()` |
    | Merged-cell bounding box `bbox(c)` | `Cell.bbox()` → `BoundingBox` |
    | Merge flag `merg(c)` | `Cell.isMerged()` |
    | Subtable membership `subtable(c)` | `Cell.subtable()` → `Subtable` |
    | Subrow membership `subrow(c)` | `Cell.subrow()` → `Subrow` |

    Substructure hierarchy traversal:

    | Formal concept | Python |
    |---|---|
    | Subtable set `ST` | `TableSyntax.subtables()` → `List<Subtable>` |
    | Row set (within subtable) | `Subtable.rows()` → `List<Row>` |
    | Subrow set `SR` | `Row.subrows()` → `List<Subrow>` |
    | Cell set (within subrow) | `Subrow.cells()` → `List<Cell>` |

### Formatting properties

Each cell carries a set of formatting properties:

| Property | Values |
|---|---|
| `fntFamily` | `SERIF`, `SANS_SERIF`, `MONOSPACED` |
| `fntBold`, `fntItalic`, `fntStrikeout`, `fntUnderline` | Boolean |
| `horzAlign` | `LEFT`, `CENTER`, `RIGHT`, `JUSTIFY` |
| `vertAlign` | `TOP`, `CENTER`, `BOTTOM`, `JUSTIFY` |
| `ltBorder`, `tpBorder`, `rtBorder`, `bmBorder` | Boolean (presence of each border edge) |
| `bgColor`, `fgColor` | RGB triple in [0, 255]³ |
| `rotat` | Angle in [0°, 360°) |

### Content properties

| Property | Description |
|---|---|
| `txt` | Raw text of the cell (possibly empty) |
| `txtBlank` | `true` when the text is empty or whitespace-only |
| `txtMultiline` | `true` when the text spans multiple lines |
| `txtIndent` | Number of leading spaces |

??? note "API mapping — formatting and content"
    | Formal property | Python accessor |
    |---|---|
    | `fntFamily` | `Cell.fontFamily()` → `FontFamily` (SERIF / SANS_SERIF / MONOSPACED) |
    | `fntBold`, `fntItalic`, `fntStrikeout`, `fntUnderline` | `Cell.isBold()`, `Cell.isItalic()`, … |
    | `horzAlign` | `Cell.horzAlign()` → `HorizontalAlignment` |
    | `vertAlign` | `Cell.vertAlign()` → `VerticalAlignment` |
    | `ltBorder`, `tpBorder`, `rtBorder`, `bmBorder` | `Cell.hasLeftBorder()`, `Cell.hasTopBorder()`, … |
    | `bgColor`, `fgColor` | `Cell.bgColor()`, `Cell.fgColor()` → `CellColor` |
    | `txt` | `Cell.text()` |
    | `txtBlank` | `Cell.isBlank()` |

---

## Semantic layer

The semantic layer `L_sem = (I_tbl, I_ctx, A)` adds interpretive meaning to the
syntactic structure.  It is absent in a freshly loaded table and is populated by
the matching process (see [Table patterns (ATP)](atp.md)).

### Items

An **item** is the atomic unit of information in the semantic layer.  There are two
kinds:

**Cell-derived item** — a triple `(s, u⃗, i)` where `s` is a string extracted or
transformed from the source cell text, `u⃗` is a (possibly empty) sequence of
user-defined tags, and `i` is the zero-based index of the item within the cell.
Every cell-derived item is permanently associated with exactly one cell; a single
cell may yield one or more items (e.g. when its text is split by a delimiter).

**Context-derived item** — a string constant `s` supplied from the external
context (constants, dictionary entries, etc.) rather than from any cell.

Each item belongs to exactly one of three **item types**:

| Type | Symbol | Role |
|---|---|---|
| Value-associated | `VAL` | Atomic data value (number, date, string, …) |
| Attribute-associated | `ATTR` | Symbolic name for a semantic role |
| Auxiliary | `AUX` | Supporting information used during interpretation |

??? note "API mapping — items"
    **Definition (Semantic layer):** `L_sem = (I_tbl, I_ctx, A)` where `I_tbl` is the set
    of cell-derived items, `I_ctx` is the set of context-derived items, and `A` is the
    set of interpretation actions.

    | Formal concept | Python class / method |
    |---|---|
    | Cell-derived item `(s, u⃗, i)` | `CellDerivedItem` |
    | String `str(ι)` | `CellDerivedItem.str()` |
    | Tags `tags(ι)` | `CellDerivedItem.tags()` |
    | Index `pos(ι)` | `CellDerivedItem.index()` |
    | Source cell `cell(ι)` | `CellDerivedItem.cell()` |
    | Item type (VAL / ATTR / AUX) | `CellDerivedItem.type()` → `ItemType` |
    | Context-derived item `s` | `ContextDerivedItem` |
    | Semantic layer container `L_sem` | `TableSemantics` |

### Item providers

An **item provider** retrieves a sequence of items that are *relevant* to a given
*anchor item* at interpretation time.  Two families exist:

**Cell-derived item provider** `Υ(τ, κ, J, k)` — parameterised by:

- a traversal order `τ ∈ {→, ←, ↓, ↑}` (row-major, reverse row-major,
  column-major, reverse column-major);
- a filter condition `κ(ι_anch, ι_cand)` — a Boolean combination of spatial and
  content constraints on the candidate item relative to the anchor;
- a target item set `J ⊆ I_tbl` (VAL, ATTR, or AUX items);
- a cardinality `k ∈ ℕ ∪ {∞}` — the maximum number of items to return.

The provider returns the first `k` items of `J ∖ {ι_anch}` that satisfy `κ`,
sorted by `τ`.

Three standard instances are recognised:

| Instance | Anchor type | Target set `J` | `k` |
|---|---|---|---|
| `Υ_val` — value-associated provider | VAL | VAL items | `k` (any) |
| `Υ_attr` — attribute-associated provider | VAL | ATTR items | 1 |
| `Υ_aux` — auxiliary provider | VAL or ATTR | all items | `k` (any) |

**Context-derived item provider** — always returns the same fixed sequence of
context-derived items regardless of the anchor, effectively injecting a constant
into the computation.

??? note "API — ProviderSpec"
    **Item filter condition:** a predicate `κ : I_tbl × I_tbl → {false, true}` taking
    an anchor item and a candidate item.

    | Concept | Python |
    |---|---|
    | Filter condition `κ` | `ItemFilterCondition` (functional interface), `ItemFilterConditionSpec` (structured) |
    | Filter `Φ_κ(ι_anch, J)` | `ItemFilter.filter(anchor, items)` |
    | Cell-derived provider spec `S_prov = (ipt, k, τ, κ)` | `ProviderSpec` (record: `cardinality`, `traversalOrder`, `filterCondition`, `targetItemKind`) |
    | Cardinality `k` | `ProviderSpec.cardinality()` — use `ProviderSpec.UNBOUNDED` for ∞ |
    | Traversal order `τ` | `ProviderSpec.traversalOrder()` → `TraversalOrder` (ROW_MAJOR / REVERSE_ROW_MAJOR / COLUMN_MAJOR / REVERSE_COLUMN_MAJOR) |
    | Typed VAL provider `Υ_tbl^val` | `ProviderSpec.val(…)` |
    | Typed ATTR provider `Υ_tbl^attr` | `ProviderSpec.attr(…)` — always cardinality 1 |
    | Typed AUX provider `Υ_tbl^aux` | `ProviderSpec.aux(…)` |
    | Unrestricted provider | `ProviderSpec.any(…)` |
    | Context-derived provider | `ProviderSpec.ctx_attr(text)`, `ProviderSpec.ctx_val(text)`, `ProviderSpec.ctx_aux(text)` |
    | Runtime provider | `ItemProvider` (functional: `provide(anchor)`) |

    Compound conditions: `ItemFilterConditionSpec.and(terms…)` → `(c1 & c2 & …)`;
    `ItemFilterConditionSpec.or(groups…)` → `(g1 | g2 | …)`.

??? note "API — Filter terms (FilterTerm)"
    These implement the atomic constraints of filter condition `κ` and mirror the
    RTL `spatConstr` / `contConstr` tokens.

    | FilterTerm | RTL token | Condition |
    |---|---|---|
    | `SameSubtable` | `ST` | `same_subtable(a) && !same_cell(a)` |
    | `SameSubrow` | `SR` | `same_subrow(a) && !same_cell(a)` |
    | `SameSubcol` | `SC` | `same_subcol(a) && !same_cell(a)` |
    | `SameCell` | `CL` | `same_cell(a)` |
    | `NotSameCell` | `NCL` | `!same_cell(a)` |
    | `SameRow` | `ROW` | `same_row(a) && !same_cell(a)` |
    | `SameCol` | `COL` | `same_col(a) && !same_cell(a)` |
    | `RightOf` | `RT` | `same_subrow(a) && col > col(a)` |
    | `LeftOf` | `LT` | `same_subrow(a) && col < col(a)` |
    | `Below` | `BW` | `same_subcol(a) && row > row(a)` |
    | `Above` | `AV` | `same_subcol(a) && row < row(a)` |
    | `ColExact(n)` | `Cn` | `col == n` |
    | `ColOffset(d)` | `C+n` / `C-n` | `col == col(a) + d` |
    | `ColRange(f,t)` | `C+n..m` | `col(a)+f ≤ col ≤ col(a)+t` |
    | `ColAbsoluteRange(lo,hi)` | `Clo..hi` | `lo ≤ col ≤ hi` |
    | `RowExact(n)` | `Rn` | `row == n` |
    | `RowOffset(d)` | `R+n` / `R-n` | `row == row(a) + d` |
    | `PosExact(n)` | `Pn` | `index == n` |
    | `PosOffset(d)` | `P+n` / `P-n` | `index == index(a) + d` |
    | `PosRange(lo,hi)` | `Plo..hi` | `lo ≤ index ≤ hi` |
    | `RegexMatched(pat)` | `"pat"` | `str.matches(pat)` |
    | `NotRegexMatched(pat)` | `!"pat"` | `!str.matches(pat)` |
    | `Contains(sub)` | `~"sub"` | `str.contains(sub)` |
    | `NotContains(sub)` | `!~"sub"` | `!str.contains(sub)` |
    | `Blank` | `BLANK` | `blankStr()` |
    | `NotBlank` | `!BLANK` | `!blankStr()` |
    | `Tagged(tags)` | `TAG #t1 #t2` | any tag matches (OR) |
    | `NotTagged(tags)` | `!TAG #t1 #t2` | no tag matches |
    | `SameStr` | `STR` | `sameStr(a)` |

### Working state and update operations

During interpretation, semantic information is accumulated in a **working state**
`(V, A, val, attr, avp, rec)`:

- `V ⊆ Σ*` — the set of values;
- `A ⊆ Σ⁺` — the set of attributes;
- `val(ι)` — maps each VAL item to a value in `V`;
- `attr(ι)` — maps each ATTR item to an attribute in `A`;
- `avp(ι)` — partial map from VAL items to `(attribute, value)` pairs;
- `rec(ι)` — partial map from cell-derived VAL items to sequences of VAL items
  (the *item-based records*).

Six **working-state update operations** populate or modify the working state:

| Operation | Symbol | Effect |
|---|---|---|
| Fill | `O_fill^δ` | Replaces the anchor's value/attribute with the concatenation (delimited by `δ`) of the provider items' strings |
| Prefix | `O_prefix^δ` | Prepends provider strings (joined by `δ`) to the anchor's value/attribute |
| Suffix | `O_suffix^δ` | Appends provider strings (joined by `δ`) to the anchor's value/attribute |
| AVP construction | `O_avp` | Creates an attribute-value pair `(attr(ι₁), val(ι_anch))` for the anchor VAL item using the single ATTR item `ι₁` returned by the provider |
| Record construction | `O_rec` | Creates an item-based record with the anchor VAL item as its first element and the provided VAL items as the remaining elements |
| Record join | `O_join^K` | Merges previously created records; key positions `K` are dropped from joined records, duplicate named attributes are removed, and the merged result is stored under the anchor |

??? note "API mapping — WorkingState"
    **Definition (Working state):** `ws = (V, A, val, attr, avp, rec)`.

    | Formal component | Python |
    |---|---|
    | Value set `V ⊆ Σ*` | `WorkingState.allVal()` → `Map<Item, String>` |
    | Attribute set `A ⊆ Σ⁺` | `WorkingState.allAttr()` → `Map<Item, String>` |
    | `val(ι)` | `WorkingState.val(item)` |
    | `attr(ι)` | `WorkingState.attr(item)` |
    | `avp(ι)` | `WorkingState.avp(item)` → `AttributeValuePair(attribute, value)` |
    | `rec(ι)` | `WorkingState.rec(item)` → `List<Item>` |
    | Derived `assoc(ι)` | `WorkingState.assoc(item)` — attribute of `avp(ι)`, or `null` |

    Six working-state update operations:

    | Formal operation | Python factory | Anchor type |
    |---|---|---|
    | `O_fill^δ` | `WorkingState.applyFill(anchor, items, delimiter)` | VAL or ATTR |
    | `O_prefix^δ` | `WorkingState.applyPrefix(anchor, items, delimiter)` | VAL or ATTR |
    | `O_suffix^δ` | `WorkingState.applySuffix(anchor, items, delimiter)` | VAL or ATTR |
    | `O_avp` | `WorkingState.applyAvp(anchor, items)` | VAL |
    | `O_rec` | `WorkingState.applyRec(anchor, items)` | cell-derived VAL |
    | `O_join^K` | `WorkingState.applyJoin(anchor, items, keyPositions)` | cell-derived VAL |

    Consistency predicates:

    | Predicate | Python factory |
    |---|---|
    | Basic consistency: `rec(ι)[0] = ι` and `avp(ι) = (a,v) ⟹ val(ι) = v` | `WorkingState.isConsistent()` |
    | Recordset-consistency: uniform anchor attribute + distinct per-record attributes | `WorkingState.isRecordsetConsistent()` |

### Interpretation actions

An **interpretation action** `(ι_anch, π⃗, o)` combines:

- an anchor item `ι_anch`;
- a sequence of item providers `π⃗ = ⟨π₁, …, πₙ⟩` that supply the operand items;
- a working-state update operation `o`.

When executed, the action first collects a concatenated item sequence
`ι⃗ = π₁(ι_anch) · … · πₙ(ι_anch)`, then applies `o` to the working state with
`ι_anch` as the anchor and `ι⃗` as the operand sequence.

Actions must be *consistent*: the provider types and resulting item counts must
satisfy the constraints of the chosen operation (Tab. I in the paper):

| Action subset | Anchor type | Provider type | `|ι⃗|` |
|---|---|---|---|
| String modification | VAL or ATTR (cell-derived) | any | ≥ 1 |
| AVP construction | VAL (cell- or context-derived) | ATTR provider | = 1 |
| Record construction | VAL (cell-derived) | VAL providers | ≥ 0 |
| Record join | VAL (cell-derived) | VAL providers (cell-derived) | ≥ 0 |

??? note "API — ActionSpec"
    **Action spec:** `S_act = (op, ⟨S_prov¹, …, S_provⁿ⟩)` where `op` is a
    working-state update operation.

    | Operation | Python factory | Effect |
    |---|---|---|
    | `REC` | `ActionSpec.rec(providers…)` | Anchor item → record; providers supply the remaining fields |
    | `AVP` | `ActionSpec.avp(provider)` | Associates a VAL item (anchor) with an ATTR item from the provider |
    | `JOIN` | `ActionSpec.join(providers…)` | Joins item-based records; dedup by named attribute (K=∅) |
    | `JOIN(K)` | `ActionSpec.join(Set.of(0), providers…)` | Joins with key positions K dropped (e.g. `JOIN(0)` drops the anchor position) |
    | `FILL` | `ActionSpec.fill(delimiter, providers…)` | Fills anchor value using provider values |
    | `PREFIX` | `ActionSpec.prefix(delimiter, providers…)` | Prepends provider values to the anchor |
    | `SUFFIX` | `ActionSpec.suffix(delimiter, providers…)` | Appends provider values to the anchor |
    | AVP with literal | `ActionSpec.avp("ATTR_NAME")` | Context-derived ATTR constant |

    `ActionSpec.rec(int anchorPos, providers…)` adds an `AnchorAttributeAtPosition`
    post-processing step (RTL: `REC(n)`).
    `ActionSpec.rec(String splitDelimiter, providers…)` adds a `DelimitedFieldSplit`
    step (RTL: `REC('s')`).

---

## Recordset and schema

**Definition (Recordset):** given a schema `S = ⟨a₁, …, aₙ⟩`, a *record* is an
n-tuple `⟨(a₁,v₁), …, (aₙ,vₙ)⟩`; a *recordset* is a finite sequence of records.

??? note "API mapping — Schema, Record, Recordset"
    | Formal concept | Python class / method |
    |---|---|
    | Schema `S = ⟨a₁, …, aₙ⟩` | `Schema` — `attributes()` → `List<String>` |
    | Record `⟨(a₁,v₁), …, (aₙ,vₙ)⟩` | `Record` — `get(attribute)`, `get(index)` |
    | Recordset | `Recordset` — `schema()`, `records()`, `size()`, `get(index)` |

---

## Table interpretation

Given a fully populated ITM instance (both layers), table interpretation proceeds
in four sequential phases.

### Phase 1 — Working state initialisation

An initial working state `ws₀` is constructed directly from the semantic layer:

- each VAL item is assigned its value `val(ι) ∈ V`;
- each ATTR item is assigned its attribute `attr(ι) ∈ A`;
- `dom(avp)` and `dom(rec)` are initialised to empty.

### Phase 2 — Working state completion

The interpretation actions `A` are applied to `ws₀` in a fixed order:

1. String-modification actions (`O_fill`, `O_prefix`, `O_suffix`);
2. AVP-construction actions (`O_avp`);
3. Record-construction actions (`O_rec`);
4. Record-join actions (`O_join`).

Within each phase, actions are applied in *traversal order* over their anchor
items.  The default strategy visits anchors in row-major order
(**row-first**, `Γ_row`); a **column-first** strategy `Γ_col` is also available.

The result is the *completed working state* `ws_comp`.

### Phase 3 — Recordset extraction

Recordset extraction from `ws_comp` has two steps.

**Schema construction** collects the set of named attributes that appear across all
item-based records:

1. Determine the *anchor attribute* `a₁`: if any anchor item has an associated
   attribute (via `avp`), all such anchors share the same attribute by a
   consistency invariant, and that attribute becomes `a₁`; otherwise a fresh
   *anonymous attribute* is generated for position 1.
2. Visit all non-anchor items in the order defined by the *schema construction
   strategy* (default: **record-first** `Γ_rec`, which visits items record by
   record; **position-first** `Γ_pos` groups items by their position across
   records).  Each new named attribute encountered is appended to the schema
   `S = ⟨a₁, a₂, …, aₙ⟩`; unnamed items receive a fresh anonymous attribute for
   their position.

**Record generation** iterates over `dom(rec)` in the order in which record-construction
actions were applied:

- For each anchor `ι`, initialise all `n` field values to a *missing value* (via
  an optional user-defined handler `μ`; default: `⊥`).
- For each item `ι'` in `rec(ι)` that has an associated attribute in `S`, fill in
  `val(ι')` at the corresponding position.
- Emit the resulting record `⟨(a₁, v₁), …, (aₙ, vₙ)⟩`.

### Phase 4 — Recordset transformation

The extracted recordset may be further post-processed by optional operations:

- **Schema reordering** — rearranges the columns of the schema according to a
  user-defined order.
- **Field splitting** — splits a field that contains a delimited value (e.g.
  `"v1/v2/v3"`) into multiple atomic fields using the same delimiter as the
  string-modification operations.
- **Whitespace normalisation** — trims leading and trailing whitespace and
  collapses internal whitespace to a single space across all values.

??? note "API mapping — TableInterpreter"
    `TableInterpreter.interpret(table)` executes all four phases.

    | Phase | Entry point |
    |---|---|
    | 1 — Working state initialisation | `initWorkingState(sem)` (private) |
    | 2 — Working state completion | `completeWorkingState(ws, actions)` (private) |
    | 3 — Recordset extraction | `extractRecordset(ws)` → `constructSchema` + `generateRecords` |
    | 4 — Recordset transformation | `transformRecordset(recordset)` |

    **Strategies and options** (all configurable via `TableInterpreter.with*(…)`):

    | Option | Python type | Values |
    |---|---|---|
    | Action application strategy `Γ_row` / `Γ_col` | `ActionApplicationStrategy` | `ROW_FIRST` (default), `COLUMN_FIRST` |
    | Schema construction strategy `Γ_rec` / `Γ_pos` | `SchemaConstructionStrategy` | `RECORD_FIRST` (default), `POSITION_FIRST` |
    | Anonymous attribute template | `String` | Pattern with `%i` placeholder; default `"$a_%i"` |
    | Missing value handler `μ` | `MissingValueHandler` | Functional interface; default returns `null` |

    **Recordset transformations** (`RecordsetTransformation` sealed interface):

    | Transformation | Python class | RTL / API trigger |
    |---|---|---|
    | Schema reordering | `SchemaReordering` | explicit attribute list |
    | Delimited field split | `DelimitedFieldSplit` | `ActionSpec.rec(String delimiter, …)` — RTL `REC('s')` |
    | Field splitting | `FieldSplitting` | explicit split spec |
    | Whitespace normalisation | `WhitespaceNormalization` | `with_transformations(…)` |
    | Anchor attribute at position | `AnchorAttributeAtPosition` | `ActionSpec.rec(int pos, …)` — RTL `REC(n)` |

---

## End-to-end example

This section traces the full lifecycle of a concrete table through ITM.
The [Table patterns (ATP)](atp.md#end-to-end-example) page shows the ATP pattern and
the syntactic matching step that precede interpretation.

### Source table

Consider a 3 × 3 table `t₀` listing the number of airline departures from airports
in certain months.  Its first row contains an empty cell followed by two airline
codes; each remaining row contains an airport code followed by two cells each
containing a departure count and a month name separated by a space:

| (empty) | CA   | HU    |
|---------|------|-------|
| IKT     | 0 Jan | 8 Feb |
| SVO     | 31 Jan | 40 Feb |

Target schema: `S = ⟨ND, AIRLINE, AIRPORT, MON⟩`.

### Items derived from the table

After syntactic matching, the following cell-derived items (all VAL) are created and
added to `I_tbl^val` (subscripts denote row, column; superscripts denote index
within the cell):

| Cell | Items |
|------|-------|
| `c(0,1)` | `ι(0,1)` = `"CA"` |
| `c(0,2)` | `ι(0,2)` = `"HU"` |
| `c(1,0)` | `ι(1,0)` = `"IKT"` |
| `c(1,1)` | `ι(1,1)⁰` = `"0"`,  `ι(1,1)¹` = `"Jan"` |
| `c(1,2)` | `ι(1,2)⁰` = `"8"`,  `ι(1,2)¹` = `"Feb"` |
| `c(2,0)` | `ι(2,0)` = `"SVO"` |
| `c(2,1)` | `ι(2,1)⁰` = `"31"`, `ι(2,1)¹` = `"Jan"` |
| `c(2,2)` | `ι(2,2)⁰` = `"40"`, `ι(2,2)¹` = `"Feb"` |

The cell `c(0,0)` is matched by a skip cell pattern — no item is derived.

Four context-derived ATTR items are also created from string constants embedded in
the action specifications: `α(ND)`, `α(AIRLINE)`, `α(AIRPORT)`, `α(MON)`.

### Working state completion

**AVP-construction actions** (O_avp) assign an attribute to every VAL item:

| Item | avp |
|------|-----|
| `ι(0,1)` | `(AIRLINE, "CA")` |
| `ι(0,2)` | `(AIRLINE, "HU")` |
| `ι(1,0)` | `(AIRPORT, "IKT")` |
| `ι(2,0)` | `(AIRPORT, "SVO")` |
| `ι(1,1)⁰` | `(ND, "0")` |
| `ι(1,2)⁰` | `(ND, "8")` |
| `ι(2,1)⁰` | `(ND, "31")` |
| `ι(2,2)⁰` | `(ND, "40")` |
| `ι(1,1)¹` | `(MON, "Jan")` |
| `ι(1,2)¹` | `(MON, "Feb")` |
| `ι(2,1)¹` | `(MON, "Jan")` |
| `ι(2,2)¹` | `(MON, "Feb")` |

**Record-construction actions** (O_rec) build four item-based records.  The anchor
of each record is a departure-count item; the remaining fields are collected by
three providers: *same column* (airline), *same row* (airport), *same cell* (month):

| Anchor | rec(anchor) |
|--------|-------------|
| `ι(1,1)⁰` | `⟨ι(1,1)⁰, ι(0,1), ι(1,0), ι(1,1)¹⟩` → `⟨"0",  "CA", "IKT", "Jan"⟩` |
| `ι(1,2)⁰` | `⟨ι(1,2)⁰, ι(0,2), ι(1,0), ι(1,2)¹⟩` → `⟨"8",  "HU", "IKT", "Feb"⟩` |
| `ι(2,1)⁰` | `⟨ι(2,1)⁰, ι(0,1), ι(2,0), ι(2,1)¹⟩` → `⟨"31", "CA", "SVO", "Jan"⟩` |
| `ι(2,2)⁰` | `⟨ι(2,2)⁰, ι(0,2), ι(2,0), ι(2,2)¹⟩` → `⟨"40", "HU", "SVO", "Feb"⟩` |

### Extracted recordset

Schema construction collects the attributes in the order they appear across records
(record-first strategy), yielding `S = ⟨ND, AIRLINE, AIRPORT, MON⟩`.  Record
generation produces:

| ND | AIRLINE | AIRPORT | MON |
|----|---------|---------|-----|
| 0  | CA      | IKT     | Jan |
| 8  | HU      | IKT     | Feb |
| 31 | CA      | SVO     | Jan |
| 40 | HU      | SVO     | Feb |
