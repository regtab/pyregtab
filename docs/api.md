# API reference

The public Python API of pyRegTab mirrors the jRegTab (Java) API 1:1 in classes
and semantics; method names follow PEP 8 (`snake_case`). Everything is exported
from the top-level `pyregtab` package; the implementation lives in the native
core `pyregtab._core` (Rust). Full signatures are in the bundled type stubs
(`pyregtab/_core.pyi`).

## Pipeline entry points

| Class / function | Role |
|---|---|
| `RtlCompiler.compile(rtl, bindings=None)` / `pyregtab.compile(...)` | RTL string → `TablePattern`; raises `RtlCompileError` with source position (`.line` 1-based, `.col` 0-based, both `None` when unknown) |
| `AtpMatcher.match(pattern, syntax, context_items=None)` | `TablePattern` × `TableSyntax` → `InterpretableTable \| None` |
| `AtpMatcher.match_many(pattern, syntaxes, context_items=None)` | batch form of `match`: `list[InterpretableTable \| None]` in input order, matched in parallel on an internal thread pool (GIL released) |
| `TableInterpreter().interpret(itm)` | `InterpretableTable` → `Recordset` |
| `AtpToRtlSerializer.serialize(pattern)` | `TablePattern` → canonical RTL string |
| `TablePattern.transform(rs)` | applies the pattern's recordset transformations |

`TableInterpreter` is configured fluently:

```python
rs = (
    TableInterpreter()
    .with_strategy(SchemaConstructionStrategy.RECORD_FIRST)      # or POSITION_FIRST
    .with_action_application_strategy(ActionApplicationStrategy.ROW_FIRST)
    .with_missing_value_handler(lambda attr: "")                 # str -> str | None
    .with_anonymous_attribute_template("A%i")                    # default "$a_%i"
    .interpret(itm)
)
```

When the pattern contains no Python callbacks (no `EXT`/custom predicates),
`AtpMatcher.match` and `TableInterpreter.interpret` release the GIL — batch
processing parallelizes with a plain `ThreadPoolExecutor`, or use
`AtpMatcher.match_many`, which fans the tables out over an internal thread
pool itself. With Python callbacks present, `match_many` degrades to matching
sequentially under the GIL, like `match`.

## ITM: syntactic layer

| Class | Notes |
|---|---|
| `TableSyntax(num_rows, num_cols)` | grid of pre-created cells; `cell(r, c)`, `row(i)`, `rows()`, `subtables()`, `all_cells()`, `define_subtables(*starts)`, `define_subrow(row, c0, c1)` |
| `Cell` | properties: `text` (get/set, plus `set_text()`), `text_blank`, `text_multiline`, `text_indent`, `row`, `col`, `pos`, `bbox`, `merged`, formatting (`font_*`, `horz_align`, `vert_align`, borders, `bg_color`, `fg_color`, `rotation`), structure (`parent_row`, `subtable`, `subrow`) |
| `Row`, `Subrow`, `Subtable` | structural handles (`subrows()`, `cells()`, `rows()`) |
| `GridPosition`, `BoundingBox`, `CellColor` | value types; enums `FontFamily`, `HorizontalAlignment`, `VerticalAlignment` |

## ITM: semantic layer and result

| Class | Notes |
|---|---|
| `InterpretableTable` | `.syntax`, `.semantics` |
| `TableSemantics` | `cell_derived_items()`, `context_derived_items()` |
| `CellDerivedItem` | `.str`, `.tags`, `.index`, `.cell`, `.span` (byte range of the item's source segment within the raw cell text, before extractors); passed to filter callbacks |
| `ContextDerivedItem(s, ItemType, const_value=None)` | external context items for `AtpMatcher.match` |
| `Recordset` | `.schema`, `.records`, `len(rs)`, `rs[i]`, `to_pandas()`, `to_csv(path=None, sep=",", missing="")` (RFC 4180; returns the CSV text when `path` is None) |
| `Record` | `rec[attr]`, `rec[i]`, `.get(...)`, `.values()`; missing value → `None` |
| `Schema` | `.attributes`, `index_of`, `contains` |

## ATP: pattern specification

Pattern hierarchy — each level has `of(...)` factories accepting an optional
leading `CellMatchCondition` and/or `Quantifier`, canonical constructors, and
postfix quantifier copies (`one_or_more()`, `zero_or_more()`, `zero_or_one()`,
`exactly(n)`):

`TablePattern.of(*subtables)` → `SubtablePattern.of(*rows)` →
`RowPattern.of(*cells_or_subrows)` → `SubrowPattern.of(*cells)` →
`CellPattern.of(content_spec)` / `CellPattern.skip()`.

Content specifications:

| Class | RTL analog |
|---|---|
| `AtomicContentSpec.val/attr/aux(*actions, extractor=None)`, `.skip()`, `.val_tagged(tag, *actions)` | `VAL : …` |
| `DelimitedContentSpec(delim, atom)` / `atom.split_by(delim)` | `(VAL){","}` |
| `CompoundContentSpec.of(first, (delim, spec), ...)` / `spec.then(delim, next)` | `VAL " " VAL` |
| `ConditionalContentSpec(cond, positive, negative)` | `BLANK? _ \| VAL` |

Actions and providers:

| Factory | RTL analog |
|---|---|
| `ActionSpec.rec(*providers, anchor_pos=None, split_delimiter=None)` | `…->REC`, `REC(n)`, `REC('/')` |
| `ActionSpec.avp(provider_or_str)` | `…->AVP`, `'NAME'->AVP` |
| `ActionSpec.join(*providers, key_positions=None)` | `…->JOIN(k…)` |
| `ActionSpec.fill/prefix/suffix(delimiter, *providers)` | `…->FILL("d")` |
| `ProviderSpec.val/attr/aux/any(condition, cardinality=1, traversal_order=None)` | `ST*`, `-AV`, `^COL{2}` |
| `ProviderSpec.ctx_attr/ctx_val/ctx_aux(text)`, `ctx_avp(name, value)` | `'NAME'`, `@'A'='V'` |

Conditions and predicates:

| Factory | RTL analog |
|---|---|
| `CellMatchCondition(CellPredicate.blank() / not_blank() / regex_matched(p) / contains(s) / external(name, fn) / custom(desc, fn))` | `BLANK?`, `"re"?`, `~"s"?`, `EXT('name')?` |
| `ItemFilterConditionSpec.bare(term)`, `.and_(*terms)`, `.or_(*groups)`, shorthands `same_subtable()` … `left_of()` | `(LT & !BLANK)` |
| `FilterTerm.*` — spatial (`left_of` … `same_cell`, `col_exact`, `row_offset`, `pos_range`, …) and content (`regex_matched`, `contains`, `blank`, `tagged`, `same_str`, `external`, `custom`) | `spatConstr` / `contConstr` |
| `StringExtractor.whitespace_normalized() / trimmed() / upper_case() / lower_case() / substring(b, e) / replaced(rx, rep) / chain(*steps) / custom(desc, fn)` | `=NORM`, `=TRIM`, … |
| `Quantifier.one() / zero_or_one() / one_or_more() / zero_or_more() / exactly(n)` | `?`, `+`, `*`, `{n}` |

Custom (`custom`) and external (`EXT`) predicates take Python callables:
cell predicates receive a `Cell`, item filters receive
`(anchor: CellDerivedItem, candidate: CellDerivedItem)`. Patterns with custom
predicates cannot be serialized back to RTL (as in Java) and disable the
GIL-release fast path.

## Embedded RTL DSL

`pyregtab.dsl` is a fluent, RTL-like layer over the ATP factories above:
`table/subtable/row/subrow/cell`, atoms `val/attr/aux`, actions `rec/avp/join/
fill/prefix/suffix`, provider constants (`ST`, `COL`, `C(n)`, …) with
`.and_()/.or_()/.card()/.unbounded()` and traversal methods, and `where(...)`
Python-callable escape hatches. It builds ordinary `TablePattern` objects,
byte-identical to `RtlCompiler.compile` for lambda-free patterns. See the
[Embedded RTL guide](embedded-rtl.md).

## Recordset transformations

`WhitespaceNormalization()`, `AnchorAttributeAtPosition(pos)`,
`DelimitedFieldSplit(delim, only_attributes=None, anonymous_attribute_template="$a_%i")`,
`FieldSplitting(attribute, delimiter, part_attribute_names=())`,
`SchemaReordering(order)` — each has `.apply(recordset)`; attach to a pattern
with `pattern.with_transformations(...)` or to an interpreter with
`.with_transformations([...])`. RTL settings `<NORM, ANCH(n), SPLIT("s")>` and
inline `REC(n)` / `REC('s')` parameters compile to these transformations.

## RTL bindings

```python
bindings = (
    Bindings.of()
    .cell("isTotal", lambda cell: cell.text.startswith("Total"))   # EXT in cell-condition position
    .filter("near", lambda a, c: abs(a.cell.row - c.cell.row) <= 2)  # EXT in provider position
)
pattern = RtlCompiler.compile(rtl, bindings)
```

Unbound `EXT('name')` references raise `RtlCompileError` at compile time with
the position of the constraint.
