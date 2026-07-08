"""Embedded RTL — a Python DSL mirroring RTL syntax, building the same ATP objects
as :func:`pyregtab.RtlCompiler.compile`.

Import the vocabulary and read it as RTL::

    from pyregtab.dsl import *

    # RTL:  { [ [VAL : ST*->REC] [VAL]{2} []+ ]
    #         [ []               [VAL]{4} []+ ] }+
    p = table(
        subtable(
            row(cell(VAL, rec(ST.unbounded())), cell(VAL).exactly(2), skip().one_or_more()),
            row(skip(), cell(VAL).exactly(4), skip().one_or_more()),
        ).one_or_more())

The pattern ``p`` is an ordinary :class:`pyregtab.TablePattern`: match it with
:func:`pyregtab.AtpMatcher.match`, serialize it with
:func:`pyregtab.AtpToRtlSerializer.serialize`. For lambda-free patterns it is
byte-identical to the one the RTL compiler produces from the equivalent string.

Escape hatches into Python (no RTL analog): ``cell(where("desc", lambda c: ...), VAL)``
for cell match conditions and ``ST.where("desc", lambda a, c: ...)`` for provider
constraints. Patterns containing ``where(...)`` cannot be serialized back to RTL
(the serializable alternative is ``EXT('name')`` + :class:`pyregtab.Bindings` in the
string compiler).

The DSL adds no expressive power beyond ATP — everything it builds is a plain
``TablePattern``; the ATP API remains the documented low-level layer.
"""

from __future__ import annotations

from typing import Callable, Optional, Union

from pyregtab._core import (
    ActionSpec,
    AtomicContentSpec,
    CellMatchCondition,
    CellPattern,
    CellPredicate,
    ConditionalContentSpec,
    FilterTerm,
    ItemDerivationDirective,
    ItemFilterConditionSpec,
    ProviderSpec,
    Quantifier,
    RowPattern,
    StringExtractor,
    SubrowPattern,
    SubtablePattern,
    TablePattern,
    TraversalOrder,
    UNBOUNDED,
    WhitespaceNormalization,
    AnchorAttributeAtPosition,
    DelimitedFieldSplit,
)

__all__ = [
    # item derivation directives
    "VAL", "ATTR", "AUX", "SKIP",
    # string extractors
    "NORM", "TRIM", "UC", "LC", "repl", "substr", "chain",
    # recordset transformations
    "norm", "anch", "split",
    # level-scoped inherited actions
    "acts",
    # pattern levels
    "table", "subtable", "row", "subrow", "skip", "cell",
    # atomic content specs
    "val", "attr", "aux", "when",
    # cell match predicates
    "blank", "not_blank", "re", "not_re", "contains", "not_contains", "where",
    # providers: spatial / content constants
    "LT", "RT", "AV", "BW", "ROW", "COL", "SR", "SC", "ST", "NCL", "CL", "STR",
    # providers: positional constraints
    "C", "Crel", "CrelFrom", "R", "Rrel", "P", "Prel",
    # providers: content constraints
    "tag", "not_tag", "item_re", "item_not_re", "item_blank", "item_not_blank",
    "item_contains", "item_not_contains",
    # context providers
    "lit", "ctx_avp",
    # actions
    "rec", "rec_split", "avp", "join", "fill", "prefix", "suffix",
    # builder / helper types
    "Prov", "Acts",
]

# ============================================================ item derivation directives

VAL = ItemDerivationDirective.VAL
ATTR = ItemDerivationDirective.ATTR
AUX = ItemDerivationDirective.AUX
SKIP = ItemDerivationDirective.SKIP

# ============================================================ string extractors

NORM = StringExtractor.whitespace_normalized()
TRIM = StringExtractor.trimmed()
UC = StringExtractor.upper_case()
LC = StringExtractor.lower_case()


def repl(regex: str, replacement: str) -> StringExtractor:
    """RTL ``REPL("regex","replacement")``."""
    return StringExtractor.replaced(regex, replacement)


def substr(begin: int, end: int) -> StringExtractor:
    """RTL ``SUBSTR(begin,length)`` — here as begin/end offsets."""
    return StringExtractor.substring(begin, end)


def chain(*steps: StringExtractor) -> StringExtractor:
    """RTL extractor chain ``=REPL(...).TRIM``."""
    return StringExtractor.chain(*steps)


# ============================================================ recordset transformations


def norm() -> WhitespaceNormalization:
    """RTL setting ``NORM`` — use with ``table(...).with_transformations(norm(), ...)``."""
    return WhitespaceNormalization()


def anch(position: int) -> AnchorAttributeAtPosition:
    """RTL setting ``ANCH(n)``."""
    return AnchorAttributeAtPosition(position)


def split(delimiter: str) -> DelimitedFieldSplit:
    """RTL setting ``SPLIT("s")``."""
    return DelimitedFieldSplit(delimiter)


# ============================================================ wrapper AST
#
# The DSL builds a small Python wrapper tree rather than native ATP objects
# directly: level-scoped ``acts(...)`` must be merged down into the atoms of
# already-constructed children, and the native pattern objects do not expose
# child accessors. Native objects are materialized once, at ``table(...)``.


class _Content:
    """Base for content-spec wrappers (atomic / delimited / compound / conditional)."""

    def build(self):  # -> native ContentSpec
        raise NotImplementedError

    def _merge(self, inherited: list) -> "_Content":
        """Return a copy with inherited actions merged into every atom below."""
        raise NotImplementedError

    def then(self, delimiter: str, nxt: "_Content") -> "_Compound":
        return _Compound([("", self), (delimiter, nxt)])


class _Atom(_Content):
    def __init__(
        self,
        idd: ItemDerivationDirective,
        actions: tuple = (),
        extractor: Optional[StringExtractor] = None,
        tags: tuple = (),
    ):
        self.idd = idd
        self.actions = tuple(actions)
        self.extractor = extractor
        self.tags = tuple(tags)

    def tagged(self, *new_tags: str) -> "_Atom":
        return _Atom(self.idd, self.actions, self.extractor, self.tags + new_tags)

    def extract(self, extractor: StringExtractor) -> "_Atom":
        return _Atom(self.idd, self.actions, extractor, self.tags)

    def split_by(self, delimiter: str) -> "_Delimited":
        return _Delimited(delimiter, self)

    def _merge(self, inherited: list) -> "_Atom":
        return _Atom(self.idd, tuple(inherited) + self.actions, self.extractor, self.tags)

    def build(self):
        if self.idd == ItemDerivationDirective.SKIP:
            # A bare skip is `[]`; a skip that inherited actions (merged down from
            # a level-scoped `acts(...)`) keeps them, as the compiler does.
            if not self.actions and not self.tags:
                return AtomicContentSpec.skip()
            return AtomicContentSpec(
                ItemDerivationDirective.SKIP, self.extractor, list(self.tags), list(self.actions)
            )
        if self.idd == ItemDerivationDirective.VAL:
            factory = AtomicContentSpec.val
        elif self.idd == ItemDerivationDirective.ATTR:
            factory = AtomicContentSpec.attr
        else:
            factory = AtomicContentSpec.aux
        spec = factory(*self.actions, extractor=self.extractor)
        if self.tags:
            spec = spec.tagged(*self.tags)
        return spec


class _Delimited(_Content):
    def __init__(self, delimiter: str, atom: _Atom):
        self.delimiter = delimiter
        self.atom = atom

    def _merge(self, inherited: list) -> "_Delimited":
        return _Delimited(self.delimiter, self.atom._merge(inherited))

    def build(self):
        return self.atom.build().split_by(self.delimiter)


class _Compound(_Content):
    def __init__(self, segments: list):
        # segments: list of (leading_delimiter, _Content); first delimiter is ""
        self.segments = list(segments)

    def then(self, delimiter: str, nxt: _Content) -> "_Compound":
        return _Compound(self.segments + [(delimiter, nxt)])

    def _merge(self, inherited: list) -> "_Compound":
        return _Compound([(d, c._merge(inherited)) for d, c in self.segments])

    def build(self):
        acc = self.segments[0][1].build()
        for delimiter, content in self.segments[1:]:
            acc = acc.then(delimiter, content.build())
        return acc


class _Conditional(_Content):
    def __init__(self, condition: CellPredicate, positive: _Content, negative: _Content):
        self.condition = condition
        self.positive = positive
        self.negative = negative

    def then(self, delimiter: str, nxt: _Content):
        raise TypeError("conditional content spec cannot start a compound (`.then`)")

    def _merge(self, inherited: list) -> "_Conditional":
        return _Conditional(
            self.condition, self.positive._merge(inherited), self.negative._merge(inherited)
        )

    def build(self):
        return ConditionalContentSpec(
            CellMatchCondition(self.condition), self.positive.build(), self.negative.build()
        )


class _Cell:
    def __init__(
        self,
        condition: Optional[CellPredicate] = None,
        content: Optional[_Content] = None,
        quantifier: Optional[Quantifier] = None,
        is_skip: bool = False,
    ):
        self.condition = condition
        self.content = content
        self.quantifier = quantifier or Quantifier.one()
        self.is_skip = is_skip

    def _with_quantifier(self, q: Quantifier) -> "_Cell":
        return _Cell(self.condition, self.content, q, self.is_skip)

    def one_or_more(self) -> "_Cell":
        return self._with_quantifier(Quantifier.one_or_more())

    def zero_or_more(self) -> "_Cell":
        return self._with_quantifier(Quantifier.zero_or_more())

    def zero_or_one(self) -> "_Cell":
        return self._with_quantifier(Quantifier.zero_or_one())

    def exactly(self, n: int) -> "_Cell":
        return self._with_quantifier(Quantifier.exactly(n))

    def _merge(self, inherited: list) -> "_Cell":
        if self.content is None:
            return self
        return _Cell(self.condition, self.content._merge(inherited), self.quantifier, self.is_skip)

    def build(self) -> CellPattern:
        if self.is_skip:
            return CellPattern.skip(self.quantifier)
        cond = CellMatchCondition(self.condition) if self.condition is not None else None
        content = self.content.build() if self.content is not None else None
        return CellPattern(cond, self.quantifier, content)


class _Subrow:
    def __init__(
        self,
        cells: tuple,
        condition: Optional[CellPredicate] = None,
        quantifier: Optional[Quantifier] = None,
    ):
        self.cells = tuple(cells)
        self.condition = condition
        self.quantifier = quantifier or Quantifier.one()

    def _with_quantifier(self, q: Quantifier) -> "_Subrow":
        return _Subrow(self.cells, self.condition, q)

    def one_or_more(self) -> "_Subrow":
        return self._with_quantifier(Quantifier.one_or_more())

    def zero_or_more(self) -> "_Subrow":
        return self._with_quantifier(Quantifier.zero_or_more())

    def zero_or_one(self) -> "_Subrow":
        return self._with_quantifier(Quantifier.zero_or_one())

    def exactly(self, n: int) -> "_Subrow":
        return self._with_quantifier(Quantifier.exactly(n))

    def _merge(self, inherited: list) -> "_Subrow":
        return _Subrow(
            tuple(c._merge(inherited) for c in self.cells), self.condition, self.quantifier
        )

    def build(self) -> SubrowPattern:
        cond = CellMatchCondition(self.condition) if self.condition is not None else None
        return SubrowPattern(cond, self.quantifier, [c.build() for c in self.cells])


class _Row:
    def __init__(
        self,
        subrows: tuple,
        condition: Optional[CellPredicate] = None,
        quantifier: Optional[Quantifier] = None,
    ):
        self.subrows = tuple(subrows)
        self.condition = condition
        self.quantifier = quantifier or Quantifier.one()

    def _with_quantifier(self, q: Quantifier) -> "_Row":
        return _Row(self.subrows, self.condition, q)

    def one_or_more(self) -> "_Row":
        return self._with_quantifier(Quantifier.one_or_more())

    def zero_or_more(self) -> "_Row":
        return self._with_quantifier(Quantifier.zero_or_more())

    def zero_or_one(self) -> "_Row":
        return self._with_quantifier(Quantifier.zero_or_one())

    def exactly(self, n: int) -> "_Row":
        return self._with_quantifier(Quantifier.exactly(n))

    def _merge(self, inherited: list) -> "_Row":
        return _Row(
            tuple(sr._merge(inherited) for sr in self.subrows), self.condition, self.quantifier
        )

    def build(self) -> RowPattern:
        cond = CellMatchCondition(self.condition) if self.condition is not None else None
        return RowPattern(cond, self.quantifier, [sr.build() for sr in self.subrows])


class _Subtable:
    def __init__(
        self,
        rows: tuple,
        condition: Optional[CellPredicate] = None,
        quantifier: Optional[Quantifier] = None,
    ):
        self.rows = tuple(rows)
        self.condition = condition
        self.quantifier = quantifier or Quantifier.one()

    def _with_quantifier(self, q: Quantifier) -> "_Subtable":
        return _Subtable(self.rows, self.condition, q)

    def one_or_more(self) -> "_Subtable":
        return self._with_quantifier(Quantifier.one_or_more())

    def zero_or_more(self) -> "_Subtable":
        return self._with_quantifier(Quantifier.zero_or_more())

    def zero_or_one(self) -> "_Subtable":
        return self._with_quantifier(Quantifier.zero_or_one())

    def exactly(self, n: int) -> "_Subtable":
        return self._with_quantifier(Quantifier.exactly(n))

    def _merge(self, inherited: list) -> "_Subtable":
        return _Subtable(
            tuple(r._merge(inherited) for r in self.rows), self.condition, self.quantifier
        )

    def build(self) -> SubtablePattern:
        cond = CellMatchCondition(self.condition) if self.condition is not None else None
        return SubtablePattern(cond, self.quantifier, [r.build() for r in self.rows])


class Acts:
    """Level-scoped action specs (RTL ``acts`` written before the nested patterns).

    Merged down into every atom of the enclosed patterns with ``inherited=True``,
    exactly as the RTL compiler does: ``row(acts(rec(BW.unbounded())), subrow(...))``
    is RTL ``[ BW*->REC { ... } ]``.
    """

    def __init__(self, actions: tuple):
        self.actions = tuple(actions)

    def marked(self) -> list:
        return [a.as_inherited() for a in self.actions]


def acts(*actions: ActionSpec) -> Acts:
    """Level-scoped inherited action specs — see :class:`Acts`."""
    return Acts(actions)


# ============================================================ pattern levels


def table(*args) -> TablePattern:
    """Table pattern (RTL ``{ subtables }`` at the top level)."""
    condition, rest = _split_condition(args)
    inherited, rest = _split_acts(rest)
    subs = [s._merge(inherited) if inherited else s for s in rest]
    built = [s.build() for s in subs]
    if condition is not None:
        return TablePattern(CellMatchCondition(condition), built, ())
    return TablePattern.of(*built)


def subtable(*args) -> _Subtable:
    """Subtable pattern (RTL ``{ rows }``); quantify with postfix methods."""
    condition, rest = _split_condition(args)
    inherited, rest = _split_acts(rest)
    rows = tuple(r._merge(inherited) if inherited else r for r in rest)
    return _Subtable(rows, condition)


def row(*args) -> _Row:
    """Row pattern (RTL ``[ cells ]``).

    Bare cells are wrapped into one implicit subrow (as the compiler does); pass
    explicit ``subrow(...)`` patterns for multiple subrows.
    """
    condition, rest = _split_condition(args)
    inherited, rest = _split_acts(rest)
    if rest and isinstance(rest[0], _Subrow):
        subrows = tuple(sr._merge(inherited) if inherited else sr for sr in rest)
    else:
        cells = tuple(c._merge(inherited) if inherited else c for c in rest)
        subrows = (_Subrow(cells),)
    return _Row(subrows, condition)


def subrow(*args) -> _Subrow:
    """Explicit subrow pattern (RTL ``{ cells }`` inside a row)."""
    condition, rest = _split_condition(args)
    inherited, rest = _split_acts(rest)
    cells = tuple(c._merge(inherited) if inherited else c for c in rest)
    return _Subrow(cells, condition)


def skip() -> _Cell:
    """Skip cell (RTL ``[]``)."""
    return _Cell(is_skip=True)


def cell(*args) -> _Cell:
    """Cell pattern. Forms mirror RTL:

    - ``cell(VAL, rec(...))`` — atomic content ``[VAL : actions]``
    - ``cell(content_spec)`` — arbitrary content (compound/delimited/conditional/tagged)
    - ``cell(blank())`` — condition-only cell ``[BLANK]``
    - ``cell(not_blank(), VAL, ...)`` — guarded cell ``[cond ? VAL : actions]``
    - ``cell(cond, content_spec)`` / ``cell(acts(...), content_spec)`` / ``cell(cond, acts(...), cs)``
    """
    if not args:
        raise TypeError("cell() requires at least one argument")
    first = args[0]

    # leading cell-match condition
    condition: Optional[CellPredicate] = None
    if isinstance(first, CellPredicate):
        condition = first
        args = args[1:]
        if not args:
            # condition-only cell: [BLANK]
            return _Cell(condition=condition, content=None)
        first = args[0]

    # cell-level inherited actions
    inherited: list = []
    if isinstance(first, Acts):
        inherited = first.marked()
        args = args[1:]
        first = args[0] if args else None

    if isinstance(first, ItemDerivationDirective):
        content: _Content = _Atom(first, args[1:])
    elif isinstance(first, _Content):
        if len(args) != 1:
            raise TypeError("cell(content_spec) takes exactly one content spec")
        content = first
    else:
        raise TypeError(f"unexpected cell() argument: {first!r}")

    if inherited:
        content = content._merge(inherited)
    return _Cell(condition=condition, content=content)


def _split_condition(args):
    if args and isinstance(args[0], CellPredicate):
        return args[0], args[1:]
    return None, args


def _split_acts(args):
    if args and isinstance(args[0], Acts):
        return args[0].marked(), args[1:]
    return [], args


# ============================================================ atomic content specs


def val(*actions: ActionSpec) -> _Atom:
    """Atomic VAL (RTL ``VAL : actions``); chain ``.tagged/.extract/.split_by/.then``."""
    return _Atom(ItemDerivationDirective.VAL, actions)


def attr(*actions: ActionSpec) -> _Atom:
    """Atomic ATTR."""
    return _Atom(ItemDerivationDirective.ATTR, actions)


def aux(*actions: ActionSpec) -> _Atom:
    """Atomic AUX."""
    return _Atom(ItemDerivationDirective.AUX, actions)


def _to_content(x: Union[ItemDerivationDirective, _Content]) -> _Content:
    if isinstance(x, ItemDerivationDirective):
        return _Atom(x)
    if isinstance(x, _Content):
        return x
    raise TypeError(f"expected a directive or content spec, got {x!r}")


def when(
    condition: CellPredicate,
    positive: Union[ItemDerivationDirective, _Content],
    negative: Union[ItemDerivationDirective, _Content],
) -> _Conditional:
    """Conditional content spec (RTL ``cond ? S⁺ | S⁻``).

    Each branch may be a directive (``VAL``/``SKIP``/...) or a content spec.
    """
    return _Conditional(condition, _to_content(positive), _to_content(negative))


# ============================================================ cell match predicates


def blank() -> CellPredicate:
    """RTL ``BLANK``."""
    return CellPredicate.blank()


def not_blank() -> CellPredicate:
    """RTL ``!BLANK``."""
    return CellPredicate.not_blank()


def re(regex: str) -> CellPredicate:
    """RTL ``"regex"``."""
    return CellPredicate.regex_matched(regex)


def not_re(regex: str) -> CellPredicate:
    """RTL ``!"regex"``."""
    return CellPredicate.not_regex_matched(regex)


def contains(substring: str) -> CellPredicate:
    """RTL ``~"sub"``."""
    return CellPredicate.contains(substring)


def not_contains(substring: str) -> CellPredicate:
    """RTL ``!~"sub"``."""
    return CellPredicate.not_contains(substring)


def where(description: str, predicate: Callable[[object], bool]) -> CellPredicate:
    """Escape hatch: arbitrary Python cell predicate (no RTL analog)."""
    return CellPredicate.custom(description, predicate)


# ============================================================ provider builder


class Prov:
    """Cell-derived provider builder — the DSL mirror of an RTL ``tblProvSpec``.

    Immutable; starts from a spatial/content constant (``ST``, ``ROW``, ...) or
    factory (``C(n)``, ``tag("H")``, ...), then chains::

        ST.and_(C(2, 5)).unbounded().col_major()   # RTL ^ST&C2..5*

    Disjunction mirrors RTL ``|`` with the same distribution as the compiler:
    ``A.and_(B.or_(C))`` == ``A&(B|C)`` == ``(A&B)|(A&C)``.

    The provider kind (VAL/ATTR/UNRESTRICTED) is inferred from the action the
    provider is passed to, exactly as in the RTL compiler.
    """

    __slots__ = ("_or_groups", "_cardinality", "_order")

    def __init__(self, term: Optional[FilterTerm] = None, *, _or_groups=None, _cardinality=1, _order=None):
        if term is not None:
            self._or_groups = ((term,),)
        else:
            self._or_groups = tuple(_or_groups)
        self._cardinality = _cardinality
        self._order = _order if _order is not None else TraversalOrder.ROW_MAJOR

    def _copy(self, *, or_groups=None, cardinality=None, order=None) -> "Prov":
        return Prov(
            _or_groups=or_groups if or_groups is not None else self._or_groups,
            _cardinality=cardinality if cardinality is not None else self._cardinality,
            _order=order if order is not None else self._order,
        )

    def and_(self, other: "Prov") -> "Prov":
        """Conjunction (RTL ``&``); nested ORs distribute: ``A&(B|C) -> (A&B)|(A&C)``."""
        distributed = []
        for left in self._or_groups:
            for right in other._or_groups:
                distributed.append(tuple(left) + tuple(right))
        return self._copy(or_groups=tuple(distributed))

    def or_(self, other: "Prov") -> "Prov":
        """Disjunction (RTL ``|``)."""
        return self._copy(or_groups=self._or_groups + other._or_groups)

    def where(self, description: str, predicate: Callable[[object, object], bool]) -> "Prov":
        """Escape hatch: conjunction with an arbitrary Python item filter (no RTL analog)."""
        return self.and_(Prov(FilterTerm.custom(description, predicate)))

    def card(self, n: int) -> "Prov":
        """Cardinality ``{n}``."""
        return self._copy(cardinality=n)

    def unbounded(self) -> "Prov":
        """Cardinality ``*``."""
        return self._copy(cardinality=UNBOUNDED)

    def col_major(self) -> "Prov":
        """Traversal order ``^`` — column-major."""
        return self._copy(order=TraversalOrder.COLUMN_MAJOR)

    def reversed(self) -> "Prov":
        """Traversal order ``-`` — reverse row-major."""
        return self._copy(order=TraversalOrder.REVERSE_ROW_MAJOR)

    def reversed_col_major(self) -> "Prov":
        """Traversal order ``-^`` — reverse column-major."""
        return self._copy(order=TraversalOrder.REVERSE_COLUMN_MAJOR)

    def _condition(self) -> ItemFilterConditionSpec:
        if len(self._or_groups) == 1:
            terms = self._or_groups[0]
            if len(terms) == 1:
                return ItemFilterConditionSpec.bare(terms[0])
            return ItemFilterConditionSpec.and_(*terms)
        return ItemFilterConditionSpec.or_(
            *(ItemFilterConditionSpec.and_(*group) for group in self._or_groups)
        )

    def _spec(self, kind: str) -> ProviderSpec:
        cond = self._condition()
        if kind == "attr":
            return ProviderSpec.attr(cond, self._order)
        if kind == "any":
            return ProviderSpec.any(cond, self._cardinality, self._order)
        return ProviderSpec.val(cond, self._cardinality, self._order)


# ---- providers: named spatial / content constants ----

LT = Prov(FilterTerm.left_of())
RT = Prov(FilterTerm.right_of())
AV = Prov(FilterTerm.above())
BW = Prov(FilterTerm.below())
ROW = Prov(FilterTerm.same_row())
COL = Prov(FilterTerm.same_col())
SR = Prov(FilterTerm.same_subrow())
SC = Prov(FilterTerm.same_subcol())
ST = Prov(FilterTerm.same_subtable())
NCL = Prov(FilterTerm.not_same_cell())
CL = Prov(FilterTerm.same_cell())
STR = Prov(FilterTerm.same_str())


# ---- providers: positional constraints ----


def C(n: int, hi: Optional[int] = None) -> Prov:
    """RTL ``Cn`` (absolute column) or ``Ca..b`` (absolute column range)."""
    if hi is None:
        return Prov(FilterTerm.col_exact(n))
    return Prov(FilterTerm.col_absolute_range(n, hi))


def Crel(delta: int, hi: Optional[int] = None) -> Prov:
    """RTL ``C+n``/``C-n`` (column offset) or ``C+lo..hi`` (relative column range)."""
    if hi is None:
        return Prov(FilterTerm.col_offset(delta))
    return Prov(FilterTerm.col_range(delta, hi))


def CrelFrom(lo: int) -> Prov:
    """RTL ``C+lo..*`` — open-ended relative column range."""
    return Prov(FilterTerm.col_range(lo, UNBOUNDED))


def R(n: int, hi: Optional[int] = None) -> Prov:
    """RTL ``Rn`` (absolute row) or ``Ra..b`` (absolute row range)."""
    if hi is None:
        return Prov(FilterTerm.row_exact(n))
    return Prov(FilterTerm.row_absolute_range(n, hi))


def Rrel(delta: int) -> Prov:
    """RTL ``R+n``/``R-n`` — row offset from the anchor."""
    return Prov(FilterTerm.row_offset(delta))


def P(n: int, hi: Optional[int] = None) -> Prov:
    """RTL ``Pn`` (absolute position) or ``Pa..b`` (absolute position range)."""
    if hi is None:
        return Prov(FilterTerm.pos_exact(n))
    return Prov(FilterTerm.pos_range(n, hi))


def Prel(delta: int) -> Prov:
    """RTL ``P+n``/``P-n`` — position offset from the anchor."""
    return Prov(FilterTerm.pos_offset(delta))


# ---- providers: content constraints ----


def tag(t: str) -> Prov:
    """RTL ``#'tag'`` constraint."""
    return Prov(FilterTerm.tagged("#" + t))


def not_tag(t: str) -> Prov:
    """RTL ``!#'tag'`` constraint."""
    return Prov(FilterTerm.not_tagged("#" + t))


def item_re(regex: str) -> Prov:
    """RTL ``"regex"`` item constraint."""
    return Prov(FilterTerm.regex_matched(regex))


def item_not_re(regex: str) -> Prov:
    """RTL ``!"regex"`` item constraint."""
    return Prov(FilterTerm.not_regex_matched(regex))


def item_blank() -> Prov:
    """RTL ``BLANK`` item constraint."""
    return Prov(FilterTerm.blank())


def item_not_blank() -> Prov:
    """RTL ``!BLANK`` item constraint."""
    return Prov(FilterTerm.not_blank())


def item_contains(substring: str) -> Prov:
    """RTL ``~"sub"`` item constraint."""
    return Prov(FilterTerm.contains(substring))


def item_not_contains(substring: str) -> Prov:
    """RTL ``!~"sub"`` item constraint."""
    return Prov(FilterTerm.not_contains(substring))


# ============================================================ context providers


class _Ctx:
    __slots__ = ("text",)

    def __init__(self, text: str):
        self.text = text


class _CtxAvp:
    __slots__ = ("attribute", "value")

    def __init__(self, attribute: str, value: str):
        self.attribute = attribute
        self.value = value


def lit(text: str) -> _Ctx:
    """Context literal (RTL ``'EUR'``): VALUE under REC/JOIN, ATTRIBUTE otherwise."""
    return _Ctx(text)


def ctx_avp(attribute: str, value: str) -> _CtxAvp:
    """Constant attribute-value pair (RTL ``@'ATTR'='VALUE'``)."""
    return _CtxAvp(attribute, value)


# ============================================================ actions

_ProvArg = Union[Prov, _Ctx, _CtxAvp]


def _kind_for(op: str) -> str:
    if op in ("rec", "join"):
        return "val"
    if op == "avp":
        return "attr"
    return "any"


def _resolve(providers: tuple, op: str) -> list:
    kind = _kind_for(op)
    out = []
    for arg in providers:
        if isinstance(arg, Prov):
            out.append(arg._spec(kind))
        elif isinstance(arg, _Ctx):
            out.append(
                ProviderSpec.ctx_val(arg.text) if op in ("rec", "join")
                else ProviderSpec.ctx_attr(arg.text)
            )
        elif isinstance(arg, _CtxAvp):
            out.append(ProviderSpec.ctx_avp(arg.attribute, arg.value))
        else:
            raise TypeError(f"unexpected provider argument: {arg!r}")
    return out


def rec(*args: Union[int, _ProvArg]) -> ActionSpec:
    """RTL ``(...)->REC``; a leading int is the inline anchor position ``REC(n)``."""
    anchor_pos = None
    if args and isinstance(args[0], int):
        anchor_pos = args[0]
        args = args[1:]
    return ActionSpec.rec(*_resolve(args, "rec"), anchor_pos=anchor_pos)


def rec_split(split_delimiter: str, *args: _ProvArg) -> ActionSpec:
    """RTL ``(...)->REC('s')`` — with an inline split delimiter."""
    return ActionSpec.rec(*_resolve(args, "rec"), split_delimiter=split_delimiter)


def avp(provider: Union[Prov, str]) -> ActionSpec:
    """RTL ``prov->AVP`` (provider) or ``'NAME'->AVP`` (constant attribute name)."""
    if isinstance(provider, str):
        return ActionSpec.avp(provider)
    return ActionSpec.avp(provider._spec("attr"))


def join(*args: Union[int, set, _ProvArg]) -> ActionSpec:
    """RTL ``(...)->JOIN``; a leading int or set is the key position(s) ``JOIN(k)``."""
    key_positions = None
    if args and isinstance(args[0], (int, set)):
        key_positions = args[0]
        args = args[1:]
    return ActionSpec.join(*_resolve(args, "join"), key_positions=key_positions)


def fill(*args: Union[str, _ProvArg]) -> ActionSpec:
    """RTL ``(...)->FILL`` / ``(...)->FILL('d')`` (leading str is the delimiter)."""
    delimiter, providers = _split_delimiter(args)
    return ActionSpec.fill(delimiter, *_resolve(providers, "fill"))


def prefix(*args: Union[str, _ProvArg]) -> ActionSpec:
    """RTL ``(...)->PREFIX`` / ``(...)->PREFIX('d')``."""
    delimiter, providers = _split_delimiter(args)
    return ActionSpec.prefix(delimiter, *_resolve(providers, "prefix"))


def suffix(*args: Union[str, _ProvArg]) -> ActionSpec:
    """RTL ``(...)->SUFFIX`` / ``(...)->SUFFIX('d')``."""
    delimiter, providers = _split_delimiter(args)
    return ActionSpec.suffix(delimiter, *_resolve(providers, "suffix"))


def _split_delimiter(args):
    if args and isinstance(args[0], str):
        return args[0], args[1:]
    return "", args
