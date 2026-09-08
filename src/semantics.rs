//! Port of `ru.icc.regtab.itm.semantics`: items, providers, working state.
//! Java identity-keyed maps become `ItemId`-keyed insertion-ordered maps.
//!
//! Cell-derived providers do not scan the whole item set: a spatial index over
//! the items ([`ItemIndex`], port of `CellDerivedItemIndex`) and a candidate
//! scope derived statically from the filter specification ([`CandidateScope`],
//! port of `CandidateScope` / `CandidateScopes`) restrict the scan to the
//! anchor's row, column, cell, row range or subtable. The filter κ is still
//! applied to every candidate of the scope and the scan stops after k matches,
//! so the result of Υ^{J,k}_{τ,κ} is unchanged (pinned by the randomized
//! equivalence test at the end of this file).

use crate::spec::{
    CellKind, CtxKind, EvalEnv, FilterCond, FilterTerm, ItemType, RecordKey, TraversalOrder,
    UNBOUNDED,
};
use crate::util::CoreResult;
pub use crate::util::Text;
use std::collections::{HashMap, HashSet};

/// Map from items to values stored densely: one slot per cell-derived item
/// and one per context item, indexed by the item's index. The working state
/// performs a handful of lookups per cell of the table, and on a
/// million-cell table a vector slot beats hashing an `ItemId`.
#[derive(Clone, Debug)]
pub struct ItemMap<V> {
    cell: Vec<Option<V>>,
    ctx: Vec<Option<V>>,
}

impl<V> Default for ItemMap<V> {
    fn default() -> Self {
        ItemMap { cell: Vec::new(), ctx: Vec::new() }
    }
}

impl<V> ItemMap<V> {
    /// Pre-sizes the slots so that inserts never reallocate.
    pub fn reserve(&mut self, cells: usize, ctx: usize) {
        if self.cell.len() < cells {
            self.cell.resize_with(cells, || None);
        }
        if self.ctx.len() < ctx {
            self.ctx.resize_with(ctx, || None);
        }
    }

    #[inline]
    pub fn get(&self, id: &ItemId) -> Option<&V> {
        match *id {
            ItemId::Cell(i) => self.cell.get(i).and_then(|s| s.as_ref()),
            ItemId::Ctx(i) => self.ctx.get(i).and_then(|s| s.as_ref()),
        }
    }

    #[inline]
    pub fn contains_key(&self, id: &ItemId) -> bool {
        self.get(id).is_some()
    }

    /// Sets the value of an item; returns the previous value, if any.
    pub fn insert(&mut self, id: ItemId, value: V) -> Option<V> {
        let (slots, i) = match id {
            ItemId::Cell(i) => (&mut self.cell, i),
            ItemId::Ctx(i) => (&mut self.ctx, i),
        };
        if i >= slots.len() {
            slots.resize_with(i + 1, || None);
        }
        slots[i].replace(value)
    }
}

impl<V> std::ops::Index<&ItemId> for ItemMap<V> {
    type Output = V;
    fn index(&self, id: &ItemId) -> &V {
        self.get(id).expect("no entry for item")
    }
}

/// Insertion-ordered map from anchors (cell-item indices) to values, stored
/// densely by anchor index: the insertion order — the order of the records —
/// is kept in a separate list.
#[derive(Clone, Debug)]
pub struct AnchorMap<V> {
    slots: Vec<Option<V>>,
    order: Vec<usize>,
}

impl<V> Default for AnchorMap<V> {
    fn default() -> Self {
        AnchorMap { slots: Vec::new(), order: Vec::new() }
    }
}

impl<V> AnchorMap<V> {
    pub fn reserve(&mut self, anchors: usize) {
        if self.slots.len() < anchors {
            self.slots.resize_with(anchors, || None);
        }
    }

    #[inline]
    pub fn get(&self, anchor: &usize) -> Option<&V> {
        self.slots.get(*anchor).and_then(|s| s.as_ref())
    }

    #[inline]
    pub fn contains_key(&self, anchor: &usize) -> bool {
        self.get(anchor).is_some()
    }

    /// Sets the value of an anchor, appending a new anchor to the order.
    pub fn insert(&mut self, anchor: usize, value: V) -> Option<V> {
        if anchor >= self.slots.len() {
            self.slots.resize_with(anchor + 1, || None);
        }
        let previous = self.slots[anchor].replace(value);
        if previous.is_none() {
            self.order.push(anchor);
        }
        previous
    }

    /// Anchors in insertion order.
    pub fn keys(&self) -> impl Iterator<Item = &usize> + '_ {
        self.order.iter()
    }

    /// Entries in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&usize, &V)> + '_ {
        self.order
            .iter()
            .map(move |a| (a, self.slots[*a].as_ref().expect("ordered anchor has a value")))
    }

    pub fn len(&self) -> usize {
        self.order.len()
    }

    pub fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
}

impl<V> std::ops::Index<&usize> for AnchorMap<V> {
    type Output = V;
    fn index(&self, anchor: &usize) -> &V {
        self.get(anchor).expect("no entry for anchor")
    }
}

/// A set of anchors (cell-item indices) as a dense bit vector.
#[derive(Clone, Debug, Default)]
pub struct AnchorSet {
    flags: Vec<bool>,
    len: usize,
}

impl AnchorSet {
    #[inline]
    pub fn contains(&self, anchor: &usize) -> bool {
        self.flags.get(*anchor).copied().unwrap_or(false)
    }

    pub fn insert(&mut self, anchor: usize) -> bool {
        if anchor >= self.flags.len() {
            self.flags.resize(anchor + 1, false);
        }
        let added = !self.flags[anchor];
        self.flags[anchor] = true;
        self.len += added as usize;
        added
    }

    pub fn remove(&mut self, anchor: &usize) -> bool {
        let present = self.contains(anchor);
        if present {
            self.flags[*anchor] = false;
            self.len -= 1;
        }
        present
    }

    pub fn extend(&mut self, anchors: impl IntoIterator<Item = usize>) {
        for a in anchors {
            self.insert(a);
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// The anchors in ascending order.
    pub fn iter(&self) -> impl Iterator<Item = usize> + '_ {
        self.flags.iter().enumerate().filter(|(_, &f)| f).map(|(i, _)| i)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ItemId {
    Cell(usize),
    Ctx(usize),
}

/// Cell-derived item (s, tags, index) bound to its source cell (row, col).
#[derive(Clone, Debug)]
pub struct CellItem {
    pub s: Text,
    pub tags: Vec<String>,
    pub index: usize,
    pub row: usize,
    pub col: usize,
    pub ty: ItemType,
    /// Byte range of the item's source segment within the raw cell text
    /// (before extractors): atomic content spans the whole text, delimited
    /// and compound content spans the segment the item was derived from.
    pub span: (usize, usize),
}

impl CellItem {
    /// `CellDerivedItem.toString()` — used in diagnostics.
    pub fn describe(&self) -> String {
        format!(
            "CellDerivedItem[str=\"{}\", index={}, cell=({}, {}), type={:?}]",
            self.s, self.index, self.row, self.col, self.ty
        )
    }
}

/// Context-derived item.
#[derive(Clone, Debug)]
pub struct CtxItem {
    pub s: Text,
    pub ty: ItemType,
    pub const_value: Option<Text>,
}

/// One instantiated interpretation action: an anchor plus the part shared
/// by every instance of the same `ActionSpec` (the matcher instantiates one
/// action per matched cell, so a million-cell table holds a million actions
/// of a handful of specs).
#[derive(Clone, Debug)]
pub struct ActionInst {
    pub anchor: ItemId,
    pub template: std::sync::Arc<ActionTemplate>,
}

impl ActionInst {
    #[inline]
    pub fn providers(&self) -> &[ProviderInst] {
        &self.template.providers
    }
    #[inline]
    pub fn op(&self) -> &OpInst {
        &self.template.op
    }
    #[inline]
    pub fn inherited(&self) -> bool {
        self.template.inherited
    }
}

/// The anchor-independent part of an action.
#[derive(Clone, Debug)]
pub struct ActionTemplate {
    pub providers: Vec<ProviderInst>,
    pub op: OpInst,
    /// `true` if the action was inherited from the `actSpecs` of a
    /// table/subtable/row/subrow scope; affects diagnostics only.
    pub inherited: bool,
}

#[derive(Clone, Debug)]
pub enum OpInst {
    Fill(String),
    Prefix(String),
    Suffix(String),
    Avp,
    Rec,
    Concat(RecordKey),
    Join(RecordKey),
}

#[derive(Clone, Debug)]
pub enum ProviderInst {
    /// Cell-derived provider over the full item set of the semantics layer.
    Cell {
        cond: FilterCond,
        order: TraversalOrder,
        cardinality: i64,
        kind: CellKind,
        exclude_anchor: bool,
        lenient: bool,
        /// Static over-approximation of the support of `cond`.
        scope: CandidateScope,
    },
    /// Context-derived provider over indices into `SemanticsCore::ctx_items`.
    Ctx { items: Vec<usize>, kind: CtxKind },
}

#[derive(Clone, Debug, Default)]
pub struct SemanticsCore {
    pub cell_items: Vec<CellItem>,
    pub ctx_items: Vec<CtxItem>,
    pub actions: Vec<ActionInst>,
    /// Matcher scratch: action templates by `ActionSpec` address, valid only
    /// while the pattern being matched is alive; cleared when matching ends.
    #[doc(hidden)]
    pub action_templates: HashMap<usize, std::sync::Arc<ActionTemplate>>,
}

impl SemanticsCore {
    /// True if interpreting may call back into Python (Custom/External
    /// filter conditions inside instantiated providers).
    pub fn has_py_callbacks(&self) -> bool {
        self.actions.iter().any(|a| {
            a.providers().iter().any(|p| match p {
                ProviderInst::Cell { cond, .. } => cond.has_py(),
                ProviderInst::Ctx { .. } => false,
            })
        })
    }

    pub fn item_str(&self, id: ItemId) -> &str {
        match id {
            ItemId::Cell(i) => &self.cell_items[i].s,
            ItemId::Ctx(i) => &self.ctx_items[i].s,
        }
    }

    pub fn item_type(&self, id: ItemId) -> ItemType {
        match id {
            ItemId::Cell(i) => self.cell_items[i].ty,
            ItemId::Ctx(i) => self.ctx_items[i].ty,
        }
    }
}

// ---------------------------------------------------------------- CandidateScope

/// Inclusive interval of row or column indices; open ends use [`Interval::OPEN_LO`] /
/// [`Interval::OPEN_HI`]. When `relative` is set, both bounds are offsets from the
/// anchor's index.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Interval {
    pub lo: i64,
    pub hi: i64,
    pub relative: bool,
}

impl Interval {
    pub const OPEN_LO: i64 = i64::MIN;
    pub const OPEN_HI: i64 = i64::MAX;

    pub const fn absolute(lo: i64, hi: i64) -> Interval {
        Interval { lo, hi, relative: false }
    }
    pub const fn absolute_point(index: i64) -> Interval {
        Interval::absolute(index, index)
    }
    pub const fn relative(lo: i64, hi: i64) -> Interval {
        Interval { lo, hi, relative: true }
    }
    pub const fn relative_point(delta: i64) -> Interval {
        Interval::relative(delta, delta)
    }

    pub fn is_point(&self) -> bool {
        self.lo == self.hi
    }

    /// Resolved lower bound for the anchor index (unclamped).
    fn resolve_lo(&self, anchor_index: i64) -> i64 {
        if self.lo == Self::OPEN_LO {
            i64::MIN
        } else if self.relative {
            anchor_index.saturating_add(self.lo)
        } else {
            self.lo
        }
    }

    /// Resolved upper bound for the anchor index (unclamped).
    fn resolve_hi(&self, anchor_index: i64) -> i64 {
        if self.hi == Self::OPEN_HI {
            i64::MAX
        } else if self.relative {
            anchor_index.saturating_add(self.hi)
        } else {
            self.hi
        }
    }

    /// Conjunction with another interval: exact intersection when both are of
    /// the same kind, otherwise the more specific one (a point wins; ties keep
    /// `self`). The result always contains the intersection of the two.
    pub fn and(&self, other: Option<Interval>) -> Interval {
        let Some(other) = other else { return *self };
        if self.relative == other.relative {
            return Interval {
                lo: self.lo.max(other.lo),
                hi: self.hi.min(other.hi),
                relative: self.relative,
            };
        }
        if self.is_point() {
            return *self;
        }
        if other.is_point() {
            return other;
        }
        *self
    }
}

/// Candidate scope — a static over-approximation of the support of an item
/// filter κ: a row interval × column interval (each optional, absolute or
/// relative to the anchor) plus a `same_subtable` flag. It never changes the
/// result: κ is still applied to every candidate of the scope, so any scope
/// that contains the support of κ is correct, and [`CandidateScope::ALL`] is
/// always correct.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct CandidateScope {
    pub rows: Option<Interval>,
    pub cols: Option<Interval>,
    pub same_subtable: bool,
}

impl CandidateScope {
    /// No restriction: the whole target set.
    pub const ALL: CandidateScope = CandidateScope { rows: None, cols: None, same_subtable: false };

    pub const fn rows(rows: Interval) -> CandidateScope {
        CandidateScope { rows: Some(rows), cols: None, same_subtable: false }
    }
    pub const fn cols(cols: Interval) -> CandidateScope {
        CandidateScope { rows: None, cols: Some(cols), same_subtable: false }
    }
    pub const fn subtable() -> CandidateScope {
        CandidateScope { rows: None, cols: None, same_subtable: true }
    }

    /// Conjunction of two scopes: each component only narrows.
    pub fn and(&self, other: &CandidateScope) -> CandidateScope {
        CandidateScope {
            rows: match self.rows {
                None => other.rows,
                Some(r) => Some(r.and(other.rows)),
            },
            cols: match self.cols {
                None => other.cols,
                Some(c) => Some(c.and(other.cols)),
            },
            same_subtable: self.same_subtable || other.same_subtable,
        }
    }

    pub fn is_all(&self) -> bool {
        self.rows.is_none() && self.cols.is_none() && !self.same_subtable
    }

    /// Port of `CandidateScopes.of(ItemFilterConditionSpec)`: a conjunction is
    /// the intersection of its terms' scopes, a disjunction or an opaque
    /// condition yields [`CandidateScope::ALL`].
    pub fn of_cond(cond: &FilterCond) -> CandidateScope {
        match cond {
            FilterCond::Bare(t) => CandidateScope::of_term(t),
            FilterCond::And(terms) => {
                let mut scope = CandidateScope::ALL;
                for t in terms {
                    scope = scope.and(&CandidateScope::of_term(t));
                }
                scope
            }
            FilterCond::Or(_) | FilterCond::Custom { .. } => CandidateScope::ALL,
        }
    }

    /// Port of `CandidateScopes.of(FilterTerm)`: for every term the scope
    /// contains every candidate for which the term can hold.
    pub fn of_term(term: &FilterTerm) -> CandidateScope {
        fn open_hi(hi: i64) -> i64 {
            if hi == UNBOUNDED {
                Interval::OPEN_HI
            } else {
                hi
            }
        }
        match term {
            // c.sameSubrow(a) && col < col(a): same row, columns to the left
            FilterTerm::LeftOf => CandidateScope {
                rows: Some(Interval::relative_point(0)),
                cols: Some(Interval::relative(Interval::OPEN_LO, -1)),
                same_subtable: false,
            },
            FilterTerm::RightOf => CandidateScope {
                rows: Some(Interval::relative_point(0)),
                cols: Some(Interval::relative(1, Interval::OPEN_HI)),
                same_subtable: false,
            },
            // c.sameSubcol(a) && row < row(a): same subtable, same column, rows above
            FilterTerm::Above => CandidateScope {
                rows: Some(Interval::relative(Interval::OPEN_LO, -1)),
                cols: Some(Interval::relative_point(0)),
                same_subtable: true,
            },
            FilterTerm::Below => CandidateScope {
                rows: Some(Interval::relative(1, Interval::OPEN_HI)),
                cols: Some(Interval::relative_point(0)),
                same_subtable: true,
            },
            FilterTerm::SameSubrow => CandidateScope::rows(Interval::relative_point(0)),
            FilterTerm::SameSubcol => CandidateScope {
                rows: None,
                cols: Some(Interval::relative_point(0)),
                same_subtable: true,
            },
            FilterTerm::SameSubtable => CandidateScope::subtable(),
            FilterTerm::SameRow => CandidateScope::rows(Interval::relative_point(0)),
            FilterTerm::SameCol => CandidateScope::cols(Interval::relative_point(0)),
            FilterTerm::SameCell => CandidateScope {
                rows: Some(Interval::relative_point(0)),
                cols: Some(Interval::relative_point(0)),
                same_subtable: false,
            },
            FilterTerm::ColExact(n) => CandidateScope::cols(Interval::absolute_point(*n)),
            FilterTerm::ColOffset(d) => CandidateScope::cols(Interval::relative_point(*d)),
            FilterTerm::ColRange(from, to) => {
                CandidateScope::cols(Interval::relative(*from, open_hi(*to)))
            }
            FilterTerm::ColAbsoluteRange(lo, hi) => {
                CandidateScope::cols(Interval::absolute(*lo, open_hi(*hi)))
            }
            FilterTerm::RowExact(n) => CandidateScope::rows(Interval::absolute_point(*n)),
            FilterTerm::RowOffset(d) => CandidateScope::rows(Interval::relative_point(*d)),
            FilterTerm::RowAbsoluteRange(lo, hi) => {
                CandidateScope::rows(Interval::absolute(*lo, open_hi(*hi)))
            }
            // position, content, sameStr, external / custom predicates and NCL
            // do not restrict the position
            _ => CandidateScope::ALL,
        }
    }

    fn clamp_lo(v: i64, n: usize) -> usize {
        if v < 0 {
            0
        } else if v > n as i64 {
            n
        } else {
            v as usize
        }
    }

    /// Clamped to `[-1, n - 1]`, returned as `i64`.
    fn clamp_hi(v: i64, n: usize) -> i64 {
        if v < -1 {
            -1
        } else if v > n as i64 - 1 {
            n as i64 - 1
        } else {
            v
        }
    }

    /// Resolved row bounds `(lo, hi)` for the anchor — `lo ∈ [0, num_rows]`,
    /// `hi ∈ [-1, num_rows - 1]`.
    fn row_bounds(&self, anchor_row: usize, num_rows: usize) -> (usize, i64) {
        match self.rows {
            None => (0, num_rows as i64 - 1),
            Some(r) => (
                Self::clamp_lo(r.resolve_lo(anchor_row as i64), num_rows),
                Self::clamp_hi(r.resolve_hi(anchor_row as i64), num_rows),
            ),
        }
    }

    fn col_bounds(&self, anchor_col: usize, num_cols: usize) -> (usize, i64) {
        match self.cols {
            None => (0, num_cols as i64 - 1),
            Some(c) => (
                Self::clamp_lo(c.resolve_lo(anchor_col as i64), num_cols),
                Self::clamp_hi(c.resolve_hi(anchor_col as i64), num_cols),
            ),
        }
    }
}

// ---------------------------------------------------------------- ItemIndex

/// Spatial index over the cell-derived items of one semantics layer (port of
/// `CellDerivedItemIndex`): two orderings of the items — row-major and
/// column-major, exactly as produced by the linearization Ω_τ — together with
/// row / column offsets, so that a [`CandidateScope`] resolves to a contiguous
/// slice of one of them: a row, a column, a cell (binary search inside a row),
/// a row range (a subtable is a row range), or the whole set. Every slice is
/// already in traversal order, so a provider needs neither a copy of the item
/// set nor a sort.
#[derive(Debug)]
pub struct ItemIndex {
    num_rows: usize,
    num_cols: usize,
    /// Item indices sorted by (row, col, index), stable.
    row_major: Vec<usize>,
    row_start: Vec<usize>,
    /// Item indices sorted by (col, row, index), stable.
    col_major: Vec<usize>,
    col_start: Vec<usize>,
    /// Column-major ordering of the items of each subtable (by subtable index),
    /// built only when some provider needs it.
    subtable_col_major: Vec<Vec<usize>>,
}

/// A contiguous slice of an ordered array, to be traversed forward or, when
/// `backward` is set, from the end — cell by cell, keeping the ascending index
/// order inside each cell (this is exactly the order of Ω_τ for the reverse
/// traversal orders).
pub struct Range<'a> {
    pub items: &'a [usize],
    pub backward: bool,
}

impl ItemIndex {
    /// Builds the index; `syntax` supplies the subtable partition used by
    /// subtable-scoped column-major providers.
    pub fn build(sem: &SemanticsCore, syntax: &crate::syntax::SyntaxCore) -> ItemIndex {
        let items = &sem.cell_items;
        let n = items.len();
        let num_rows = items.iter().map(|it| it.row + 1).max().unwrap_or(0);
        let num_cols = items.iter().map(|it| it.col + 1).max().unwrap_or(0);

        let mut row_major: Vec<usize> = (0..n).collect();
        row_major.sort_by_key(|&i| (items[i].row, items[i].col, items[i].index));
        let mut col_major: Vec<usize> = (0..n).collect();
        col_major.sort_by_key(|&i| (items[i].col, items[i].row, items[i].index));

        let mut row_start = vec![0usize; num_rows + 1];
        let mut idx = 0;
        for (r, start) in row_start.iter_mut().enumerate().take(num_rows) {
            *start = idx;
            while idx < n && items[row_major[idx]].row == r {
                idx += 1;
            }
        }
        row_start[num_rows] = n;

        let mut col_start = vec![0usize; num_cols + 1];
        idx = 0;
        for (c, start) in col_start.iter_mut().enumerate().take(num_cols) {
            *start = idx;
            while idx < n && items[col_major[idx]].col == c {
                idx += 1;
            }
        }
        col_start[num_cols] = n;

        let needs_subtable_col_major = sem.actions.iter().any(|a| {
            a.providers().iter().any(|p| match p {
                ProviderInst::Cell { order, scope, .. } => {
                    scope.same_subtable
                        && matches!(
                            order,
                            TraversalOrder::ColumnMajor | TraversalOrder::ReverseColumnMajor
                        )
                }
                ProviderInst::Ctx { .. } => false,
            })
        });
        let mut subtable_col_major = Vec::new();
        if needs_subtable_col_major {
            for st in &syntax.subtables {
                let lo = st.row_start.min(num_rows);
                let hi = (st.row_end + 1).min(num_rows);
                let mut a: Vec<usize> = if lo < hi {
                    row_major[row_start[lo]..row_start[hi]].to_vec()
                } else {
                    Vec::new()
                };
                a.sort_by_key(|&i| (items[i].col, items[i].row, items[i].index));
                subtable_col_major.push(a);
            }
        }

        ItemIndex { num_rows, num_cols, row_major, row_start, col_major, col_start, subtable_col_major }
    }

    /// Resolves the scope against the anchor and returns the candidate slice in
    /// traversal order τ. The slice is a superset of the items lying in
    /// `scope(anchor)`.
    pub fn lookup(
        &self,
        scope: &CandidateScope,
        anchor: &CellItem,
        order: TraversalOrder,
        env: &EvalEnv,
        sem: &SemanticsCore,
    ) -> Range<'_> {
        let backward = matches!(
            order,
            TraversalOrder::ReverseRowMajor | TraversalOrder::ReverseColumnMajor
        );
        let column_major = matches!(
            order,
            TraversalOrder::ColumnMajor | TraversalOrder::ReverseColumnMajor
        );
        let empty = Range { items: &self.row_major[0..0], backward };
        if self.row_major.is_empty() {
            return empty;
        }
        if scope.is_all() {
            let a = if column_major { &self.col_major } else { &self.row_major };
            return Range { items: a, backward };
        }

        let (mut r_lo, mut r_hi) = scope.row_bounds(anchor.row, self.num_rows);
        let (c_lo, c_hi) = scope.col_bounds(anchor.col, self.num_cols);
        let subtable = env.syntax.subtable_of_row(anchor.row);
        if scope.same_subtable {
            let Some(st_idx) = subtable else { return empty };
            let st = &env.syntax.subtables[st_idx];
            r_lo = r_lo.max(st.row_start);
            r_hi = r_hi.min(st.row_end as i64);
        }
        if r_lo as i64 > r_hi || c_lo as i64 > c_hi {
            return empty;
        }
        let (r_hi, c_hi) = (r_hi as usize, c_hi as usize);
        let items = &sem.cell_items;

        if r_lo == r_hi {
            // one row: row-major slice sorted by (col, index); narrow by columns
            let (mut from, mut to) = (self.row_start[r_lo], self.row_start[r_lo + 1]);
            if scope.cols.is_some() {
                from = lower_bound(&self.row_major, from, to, |i| items[i].col < c_lo);
                to = lower_bound(&self.row_major, from, to, |i| items[i].col <= c_hi);
            }
            return Range { items: &self.row_major[from..to], backward };
        }
        if c_lo == c_hi {
            // one column: column-major slice sorted by (row, index); narrow by rows
            let (mut from, mut to) = (self.col_start[c_lo], self.col_start[c_lo + 1]);
            if scope.rows.is_some() || scope.same_subtable {
                from = lower_bound(&self.col_major, from, to, |i| items[i].row < r_lo);
                to = lower_bound(&self.col_major, from, to, |i| items[i].row <= r_hi);
            }
            return Range { items: &self.col_major[from..to], backward };
        }
        if !column_major {
            // row range (a subtable is a row range) in row-major order
            return Range {
                items: &self.row_major[self.row_start[r_lo]..self.row_start[r_hi + 1]],
                backward,
            };
        }
        if scope.same_subtable {
            let a = &self.subtable_col_major[subtable.expect("checked above")];
            let from = lower_bound(a, 0, a.len(), |i| items[i].col < c_lo);
            let to = lower_bound(a, from, a.len(), |i| items[i].col <= c_hi);
            return Range { items: &a[from..to], backward };
        }
        Range {
            items: &self.col_major[self.col_start[c_lo]..self.col_start[c_hi + 1]],
            backward,
        }
    }
}

/// First position in `a[from..to)` for which `less` is false (the array is
/// partitioned by `less`).
fn lower_bound(a: &[usize], from: usize, to: usize, less: impl Fn(usize) -> bool) -> usize {
    let (mut lo, mut hi) = (from, to);
    while lo < hi {
        let mid = (lo + hi) / 2;
        if less(a[mid]) {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    lo
}

// ---------------------------------------------------------------- providers

impl ProviderInst {
    /// Port of `ItemProvider.provide(anchor)`.
    pub fn provide(
        &self,
        anchor: ItemId,
        sem: &SemanticsCore,
        env: &EvalEnv,
        index: &ItemIndex,
    ) -> CoreResult<Vec<ItemId>> {
        let mut out = Vec::new();
        self.provide_into(anchor, sem, env, index, &mut out)?;
        Ok(out)
    }

    /// [`Self::provide`] appending to a caller-owned buffer (reused across
    /// the millions of actions of a large table).
    pub fn provide_into(
        &self,
        anchor: ItemId,
        sem: &SemanticsCore,
        env: &EvalEnv,
        index: &ItemIndex,
        result: &mut Vec<ItemId>,
    ) -> CoreResult<()> {
        match self {
            ProviderInst::Ctx { items, kind } => {
                result.extend(self.provide_ctx(anchor, sem, items, *kind)?);
                Ok(())
            }
            ProviderInst::Cell { cond, order, cardinality, kind, exclude_anchor, lenient, scope } => {
                let ItemId::Cell(anchor_idx) = anchor else {
                    return Err("CellDerivedItemProvider requires a cell-derived anchor".into());
                };
                let anch = &sem.cell_items[anchor_idx];
                let compatible = match kind {
                    CellKind::Unrestricted => true,
                    CellKind::Val | CellKind::Attr => anch.ty == ItemType::Value,
                    CellKind::Aux => {
                        anch.ty == ItemType::Value || anch.ty == ItemType::Attribute
                    }
                };
                if !compatible {
                    if *lenient {
                        return Ok(());
                    }
                    return Err(format!(
                        "Υ_tbl^val and Υ_tbl^attr require a value-associated anchor, got: {:?}",
                        anch.ty
                    )
                    .into());
                }
                if *cardinality == 0 {
                    return Ok(());
                }
                let start = result.len();
                let range = index.lookup(scope, anch, *order, env, sem);
                let a = range.items;
                // J \ {anchor}, the kind restriction on J and κ; stops once k
                // items are collected.
                let accept = |i: usize, result: &mut Vec<ItemId>| -> CoreResult<bool> {
                    if *exclude_anchor && i == anchor_idx {
                        return Ok(false);
                    }
                    let cand = &sem.cell_items[i];
                    let type_ok = match kind {
                        CellKind::Unrestricted | CellKind::Aux => true,
                        CellKind::Val => cand.ty == ItemType::Value,
                        CellKind::Attr => cand.ty == ItemType::Attribute,
                    };
                    if !type_ok || !cond.eval(anch, cand, env)? {
                        return Ok(false);
                    }
                    result.push(ItemId::Cell(i));
                    Ok(*cardinality != UNBOUNDED && (result.len() - start) as i64 >= *cardinality)
                };
                if !range.backward {
                    for &i in a {
                        if accept(i, result)? {
                            return Ok(());
                        }
                    }
                    return Ok(());
                }
                // Reverse traversal: cells in reverse order, items inside a cell
                // still by ascending index.
                let mut i = a.len();
                while i > 0 {
                    let last = i - 1;
                    let (r, c) = (sem.cell_items[a[last]].row, sem.cell_items[a[last]].col);
                    let mut j = last;
                    while j > 0
                        && sem.cell_items[a[j - 1]].row == r
                        && sem.cell_items[a[j - 1]].col == c
                    {
                        j -= 1;
                    }
                    for &k in &a[j..=last] {
                        if accept(k, result)? {
                            return Ok(());
                        }
                    }
                    i = j;
                }
                Ok(())
            }
        }
    }

    fn provide_ctx(
        &self,
        anchor: ItemId,
        sem: &SemanticsCore,
        items: &[usize],
        kind: CtxKind,
    ) -> CoreResult<Vec<ItemId>> {
        let result: Vec<ItemId> = items.iter().map(|&i| ItemId::Ctx(i)).collect();
        match kind {
            CtxKind::Unrestricted => {}
            CtxKind::Val => {
                if !items.iter().all(|&i| sem.ctx_items[i].ty == ItemType::Value) {
                    return Err("Υ_ctx^val requires context value items only".into());
                }
                let ok = matches!(anchor, ItemId::Cell(i) if sem.cell_items[i].ty == ItemType::Value);
                if !ok {
                    return Err("Υ_ctx^val requires a table value anchor".into());
                }
            }
            CtxKind::Attr => {
                if items.len() != 1 || sem.ctx_items[items[0]].ty != ItemType::Attribute {
                    return Err("Υ_ctx^attr requires exactly one attribute context item".into());
                }
                let ok = match anchor {
                    ItemId::Cell(i) => sem.cell_items[i].ty == ItemType::Value,
                    ItemId::Ctx(i) => sem.ctx_items[i].ty == ItemType::Value,
                };
                if !ok {
                    return Err(
                        "Υ_ctx^attr requires anchor ∈ I_tbl^val ∪ I_ctx^val".into()
                    );
                }
            }
            CtxKind::Aux => {
                if !items.iter().all(|&i| sem.ctx_items[i].ty == ItemType::Auxiliary) {
                    return Err("Υ_ctx^aux requires context auxiliary items only".into());
                }
                let ok = matches!(anchor, ItemId::Cell(i)
                    if sem.cell_items[i].ty == ItemType::Value
                        || sem.cell_items[i].ty == ItemType::Attribute);
                if !ok {
                    return Err(
                        "Υ_ctx^aux requires a table value or table attribute anchor".into()
                    );
                }
            }
        }
        Ok(result)
    }
}

// ---------------------------------------------------------------- Diagnostic

/// A diagnostic recorded during working state completion when an operation is
/// not applicable to its anchor (a violated precondition, a missing record).
/// By the formal model the operation then has no effect; the diagnostic makes
/// the silent no-op visible to the pattern author.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    /// Index of the anchor into `SemanticsCore::cell_items`.
    pub anchor: usize,
    /// The operation name (`CONCAT`, `JOIN`, …).
    pub operation: String,
    /// What precondition was violated and why.
    pub message: String,
}

impl Diagnostic {
    /// `Diagnostic.toString()`: `<op> skipped at <anchor>: <message>`.
    pub fn describe(&self, sem: &SemanticsCore) -> String {
        format!(
            "{} skipped at {}: {}",
            self.operation,
            sem.cell_items[self.anchor].describe(),
            self.message
        )
    }
}

// ---------------------------------------------------------------- WorkingState

/// The records of one anchor: one after `O_rec` / `O_concat` (no vector of
/// vectors for the common case), several after `O_join`.
#[derive(Clone, Debug, PartialEq)]
pub enum Records {
    One(Vec<ItemId>),
    Many(Vec<Vec<ItemId>>),
}

impl Records {
    #[inline]
    pub fn as_slice(&self) -> &[Vec<ItemId>] {
        match self {
            Records::One(r) => std::slice::from_ref(r),
            Records::Many(rs) => rs,
        }
    }
}

/// Port of `WorkingState`: `ws = (V, A, val, attr, avp, rec, J)`, insertion-
/// ordered maps keyed by item identity.
///
/// `rec` maps an anchor (cell-item index) to a *non-empty sequence* of
/// item-based records: a single record after `O_rec` / `O_concat`, several
/// after `O_join` (the record product). `J` — the *joined-away anchors* — are
/// items whose records have been consumed by a join; they stay in `rec` (a
/// later join may consume the same records again, irrespective of action
/// order) but are excluded from recordset extraction. `C` — the
/// *concatenated-away anchors* — are items whose records were folded into
/// another anchor's record by a concatenation and left `dom(rec)`; the set is
/// kept so that a later action on such an anchor can be told apart from an
/// anchor that never had a record.
#[derive(Default, Debug)]
pub struct WorkingState {
    pub val: ItemMap<Text>,
    pub attr: ItemMap<Text>,
    /// avp(ι) = (attribute, value); the attribute is an index into
    /// `attr_names` (interned: a million records share one name).
    avp: ItemMap<(u32, Text)>,
    attr_names: Vec<Text>,
    attr_ids: HashMap<Text, u32>,
    /// Keyed by cell-item index; insertion order defines the order of records.
    /// Concatenated-away anchors keep their (stale) entry and are masked by `C`.
    rec: AnchorMap<Records>,
    /// J: joined-away anchors.
    joined: AnchorSet,
    /// C: concatenated-away anchors (removed from `dom(rec)`).
    concatenated: AnchorSet,
    /// Preconditions violated during completion; the operations had no effect.
    diagnostics: Vec<Diagnostic>,
    /// If set, a violated precondition raises instead of being recorded.
    strict_preconditions: bool,
}

impl WorkingState {
    pub fn new(strict_preconditions: bool) -> WorkingState {
        WorkingState { strict_preconditions, ..Default::default() }
    }

    /// Pre-sizes the per-item storage for a semantics layer.
    pub fn reserve(&mut self, cell_items: usize, ctx_items: usize) {
        self.val.reserve(cell_items, ctx_items);
        self.attr.reserve(cell_items, ctx_items);
        self.avp.reserve(cell_items, ctx_items);
        self.rec.reserve(cell_items);
    }

    // --- attribute names (interned) ---

    /// The id of an attribute name, interning it on first use.
    pub fn intern_attr(&mut self, name: &str) -> u32 {
        if let Some(&id) = self.attr_ids.get(name) {
            return id;
        }
        let id = self.attr_names.len() as u32;
        let text: Text = Text::from(name);
        self.attr_names.push(text.clone());
        self.attr_ids.insert(text, id);
        id
    }

    /// The id of an already interned attribute name.
    pub fn attr_id_of(&self, name: &str) -> Option<u32> {
        self.attr_ids.get(name).copied()
    }

    pub fn attr_name(&self, id: u32) -> &str {
        &self.attr_names[id as usize]
    }

    /// Number of interned attribute names (ids are `0..attr_count()`).
    pub fn attr_count(&self) -> usize {
        self.attr_names.len()
    }

    /// The attribute id of the item's avp, if any.
    #[inline]
    pub fn attr_id(&self, item: ItemId) -> Option<u32> {
        self.avp.get(&item).map(|(a, _)| *a)
    }

    /// The attribute name of the item's avp, if any.
    pub fn assoc(&self, item: ItemId) -> Option<&str> {
        self.avp.get(&item).map(|(a, _)| self.attr_name(*a))
    }

    /// The avp of the item as `(attribute, value)`, if any.
    pub fn avp(&self, item: ItemId) -> Option<(&str, &str)> {
        self.avp.get(&item).map(|(a, v)| (self.attr_name(*a), &**v))
    }

    pub fn has_avp(&self, item: ItemId) -> bool {
        self.avp.contains_key(&item)
    }

    // --- rec accessors ---

    /// ι ∈ dom(rec).
    pub fn has_rec(&self, anchor: usize) -> bool {
        self.rec.contains_key(&anchor) && !self.concatenated.contains(&anchor)
    }

    /// rec(ι): the records of the anchor, also for joined-away anchors;
    /// `None` if ι ∉ dom(rec).
    pub fn rec(&self, anchor: usize) -> Option<&[Vec<ItemId>]> {
        if self.concatenated.contains(&anchor) {
            return None;
        }
        self.rec.get(&anchor).map(Records::as_slice)
    }

    /// ι ∈ J.
    pub fn is_joined(&self, anchor: usize) -> bool {
        self.joined.contains(&anchor)
    }

    /// ι ∈ C.
    pub fn is_concatenated(&self, anchor: usize) -> bool {
        self.concatenated.contains(&anchor)
    }

    /// The *live* anchors `dom(rec) \ J` in insertion order — exactly what
    /// recordset extraction sees.
    pub fn live_anchors(&self) -> Vec<usize> {
        self.rec
            .keys()
            .copied()
            .filter(|a| !self.joined.contains(a) && !self.concatenated.contains(a))
            .collect()
    }

    pub fn all_joined(&self) -> &AnchorSet {
        &self.joined
    }

    pub fn all_concatenated(&self) -> &AnchorSet {
        &self.concatenated
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn take_diagnostics(&mut self) -> Vec<Diagnostic> {
        std::mem::take(&mut self.diagnostics)
    }

    /// Records that an operation is not applicable to its anchor and has no
    /// effect: adds a [`Diagnostic`]; under strict preconditions returns an
    /// error with the same text instead (Java: `IllegalStateException`).
    pub fn report(
        &mut self,
        sem: &SemanticsCore,
        anchor: usize,
        operation: &str,
        message: impl Into<String>,
    ) -> CoreResult<()> {
        let d = Diagnostic { anchor, operation: operation.to_string(), message: message.into() };
        let text = d.describe(sem);
        self.diagnostics.push(d);
        if self.strict_preconditions {
            return Err(text.into());
        }
        Ok(())
    }

    // --- string helpers ---

    fn get_val_or_attr(&self, sem: &SemanticsCore, anchor: ItemId) -> CoreResult<Text> {
        match sem.item_type(anchor) {
            ItemType::Value => self
                .val
                .get(&anchor)
                .cloned()
                .ok_or_else(|| format!("No value for: {anchor:?}").into()),
            ItemType::Attribute => self
                .attr
                .get(&anchor)
                .cloned()
                .ok_or_else(|| format!("No attribute for: {anchor:?}").into()),
            ItemType::Auxiliary => {
                Err("String ops require VALUE or ATTRIBUTE anchor, got: AUXILIARY".into())
            }
        }
    }

    fn set_val_or_attr(&mut self, sem: &SemanticsCore, anchor: ItemId, value: String) -> CoreResult<()> {
        match sem.item_type(anchor) {
            ItemType::Value => {
                self.val.insert(anchor, Text::from(value));
                Ok(())
            }
            ItemType::Attribute => {
                self.attr.insert(anchor, Text::from(value));
                Ok(())
            }
            ItemType::Auxiliary => {
                Err("String ops require VALUE or ATTRIBUTE anchor, got: AUXILIARY".into())
            }
        }
    }

    fn join_strings(sem: &SemanticsCore, items: &[ItemId], delimiter: &str) -> String {
        items
            .iter()
            .map(|&i| sem.item_str(i))
            .collect::<Vec<_>>()
            .join(delimiter)
    }

    // --- O_fill / O_prefix / O_suffix ---

    pub fn apply_fill(
        &mut self,
        sem: &SemanticsCore,
        anchor: ItemId,
        items: &[ItemId],
        delimiter: &str,
    ) -> CoreResult<()> {
        let joined = Self::join_strings(sem, items, delimiter);
        self.set_val_or_attr(sem, anchor, joined)
    }

    pub fn apply_prefix(
        &mut self,
        sem: &SemanticsCore,
        anchor: ItemId,
        items: &[ItemId],
        delimiter: &str,
    ) -> CoreResult<()> {
        if items.is_empty() {
            return Ok(());
        }
        let current = self.get_val_or_attr(sem, anchor)?;
        let prefix = Self::join_strings(sem, items, delimiter);
        self.set_val_or_attr(sem, anchor, format!("{prefix}{delimiter}{current}"))
    }

    pub fn apply_suffix(
        &mut self,
        sem: &SemanticsCore,
        anchor: ItemId,
        items: &[ItemId],
        delimiter: &str,
    ) -> CoreResult<()> {
        if items.is_empty() {
            return Ok(());
        }
        let current = self.get_val_or_attr(sem, anchor)?;
        let suffix = Self::join_strings(sem, items, delimiter);
        self.set_val_or_attr(sem, anchor, format!("{current}{delimiter}{suffix}"))
    }

    // --- O_avp ---

    pub fn apply_avp(&mut self, anchor: ItemId, items: &[ItemId]) -> CoreResult<()> {
        if self.avp.contains_key(&anchor) {
            return Ok(());
        }
        if items.len() != 1 {
            return Err(format!("O_avp requires exactly 1 item, got: {}", items.len()).into());
        }
        let attr_item = items[0];
        let a = self
            .attr
            .get(&attr_item)
            .cloned()
            .ok_or_else(|| format!("No attribute for item: {attr_item:?}"))?;
        let v = self
            .val
            .get(&anchor)
            .cloned()
            .ok_or_else(|| format!("No value for anchor: {anchor:?}"))?;
        let id = self.intern_attr(&a);
        self.avp.insert(anchor, (id, v));
        Ok(())
    }

    // --- O_rec: rec(anchor) := <<anchor, i1, ..., in>> ---

    pub fn apply_rec(&mut self, sem: &SemanticsCore, anchor: ItemId, items: &[ItemId]) -> CoreResult<()> {
        let ItemId::Cell(anchor_idx) = anchor else {
            return Err("O_rec requires a cell-derived anchor".into());
        };
        if !self.val.contains_key(&anchor) {
            return Ok(());
        }
        if self.rec.contains_key(&anchor_idx) {
            return Ok(());
        }
        let mut sequence = Vec::with_capacity(items.len() + 1);
        sequence.push(anchor);
        for &item in items {
            if let ItemId::Ctx(ci) = item {
                let ctx = &sem.ctx_items[ci];
                if let Some(cv) = &ctx.const_value {
                    self.val.insert(item, cv.clone());
                    let id = self.intern_attr(&ctx.s);
                    self.avp.insert(item, (id, cv.clone()));
                }
            }
            sequence.push(item);
        }
        self.rec.insert(anchor_idx, Records::One(sequence));
        Ok(())
    }

    /// The provided cell-derived items other than the anchor that have a
    /// record, without duplicates, in provider order.
    fn others_with_records(&self, anchor_idx: usize, items: &[ItemId]) -> Vec<usize> {
        let mut others: Vec<usize> = Vec::new();
        for &item in items {
            if let ItemId::Cell(ci) = item {
                if ci != anchor_idx && self.has_rec(ci) && !others.contains(&ci) {
                    others.push(ci);
                }
            }
        }
        others
    }

    // --- O_concat^K: rec(anchor) := <rho_anch · drop_K(rho_1) · ... · drop_K(rho_n)>; rec.remove(i_k) ---

    /// Concatenates the records of the provided anchors to the anchor's record
    /// (one wide record) and removes them from dom(rec). Applicable iff (i) the
    /// anchor and at least one provided item have records, (ii) all records
    /// agree on the key K (key positions and/or key attribute names), and (iii)
    /// apart from the key no named attribute occurs in more than one of the
    /// concatenated records; otherwise the operation has no effect and a
    /// [`Diagnostic`] is recorded.
    pub fn apply_concat(
        &mut self,
        sem: &SemanticsCore,
        anchor: ItemId,
        items: &[ItemId],
        key: &RecordKey,
    ) -> CoreResult<()> {
        let ItemId::Cell(anchor_idx) = anchor else {
            return Err("O_concat requires a cell-derived anchor".into());
        };
        let Some(anchor_recs) = self.rec(anchor_idx) else {
            return Ok(());
        };
        if items.is_empty() {
            return Ok(());
        }
        let anchor_rec = anchor_recs[0].clone();
        let others = self.others_with_records(anchor_idx, items);
        if others.is_empty() {
            return Ok(()); // (i)
        }
        let mut result = anchor_rec.clone();
        for &other in &others {
            let other_rec = &self.rec[&other].as_slice()[0];
            if let Some(problem) = self.key_mismatch(&anchor_rec, other_rec, key) {
                return self.report(sem, anchor_idx, "CONCAT", problem); // (ii)
            }
            result.extend(self.drop_k(other_rec, key));
        }
        if let Some(duplicate) = self.duplicate_attribute(&result) {
            return self.report(
                sem,
                anchor_idx,
                "CONCAT",
                format!(
                    "named attribute '{duplicate}' occurs in more than one of the concatenated records"
                ),
            ); // (iii)
        }
        self.rec.insert(anchor_idx, Records::One(result));
        for other in others {
            self.joined.remove(&other);
            self.concatenated.insert(other);
        }
        Ok(())
    }

    // --- O_join^K: rec(anchor) := <dedup(rho_i · drop_K(rho'_j)) : compat_K ∧ agree>; J := J ∪ {i_k} ---

    /// Multiplies every record of the anchor by every record of the provided
    /// anchors (a cross product for K = ∅, an equi-join on the key K otherwise;
    /// a named attribute shared by two records acts as a natural-join
    /// condition) and marks the provided anchors as joined-away. Pairs whose
    /// keys differ or whose shared attributes disagree are dropped; if no pair
    /// survives, the anchor keeps its records (left outer join).
    pub fn apply_join(
        &mut self,
        sem: &SemanticsCore,
        anchor: ItemId,
        items: &[ItemId],
        key: &RecordKey,
    ) -> CoreResult<()> {
        let ItemId::Cell(anchor_idx) = anchor else {
            return Err("O_join requires a cell-derived anchor".into());
        };
        let Some(anchor_recs) = self.rec(anchor_idx) else {
            return Ok(());
        };
        if items.is_empty() {
            return Ok(());
        }
        let anchor_recs = anchor_recs.to_vec();
        let others = self.others_with_records(anchor_idx, items);
        if others.is_empty() {
            return Ok(());
        }
        let mut joined_recs: Vec<Vec<ItemId>> = Vec::new();
        for &other in &others {
            joined_recs.extend(self.rec[&other].as_slice().iter().cloned());
        }

        let mut result: Vec<Vec<ItemId>> = Vec::new();
        let mut dropped = 0usize;
        for rho in &anchor_recs {
            for rho2 in &joined_recs {
                if self.key_mismatch(rho, rho2, key).is_some() || !self.agree(rho, rho2) {
                    dropped += 1;
                    continue;
                }
                let mut combined = rho.clone();
                combined.extend(self.drop_k(rho2, key));
                result.push(self.dedup(&combined));
            }
        }
        if result.is_empty() {
            self.report(
                sem,
                anchor_idx,
                "JOIN",
                format!(
                    "none of the {dropped} record pairs satisfies the key/attribute conditions; the anchor keeps its records"
                ),
            )?;
        } else {
            self.rec.insert(anchor_idx, Records::Many(result));
        }
        self.joined.extend(others);
        Ok(())
    }

    /// drop_K(ρ̄): the sequence with the key items removed — the items at the
    /// key positions (0-based) and the items carrying a key attribute name;
    /// the latter are resolved per record.
    fn drop_k(&self, sequence: &[ItemId], key: &RecordKey) -> Vec<ItemId> {
        if key.is_empty() {
            return sequence.to_vec();
        }
        let mut result = Vec::with_capacity(sequence.len());
        for (i, &item) in sequence.iter().enumerate() {
            if key.positions.contains(&(i as i64)) {
                continue;
            }
            if let Some(a) = self.assoc(item) {
                if key.names.contains(a) {
                    continue;
                }
            }
            result.push(item);
        }
        result
    }

    /// The index of the first item of the record carrying the named attribute.
    fn index_of_attribute(&self, sequence: &[ItemId], attribute: &str) -> Option<usize> {
        sequence.iter().position(|&it| self.assoc(it) == Some(attribute))
    }

    /// dedup(ρ̄): keeps the first occurrence of each named attribute; items
    /// without avp are always kept.
    fn dedup(&self, sequence: &[ItemId]) -> Vec<ItemId> {
        let mut seen: HashSet<&str> = HashSet::new();
        let mut result = Vec::with_capacity(sequence.len());
        for &item in sequence {
            match self.assoc(item) {
                Some(a) => {
                    if seen.insert(a) {
                        result.push(item);
                    }
                }
                None => result.push(item),
            }
        }
        result
    }

    fn pair_text(&self, pair: &(u32, Text)) -> String {
        format!("AttributeValuePair[attribute={}, value={}]", self.attr_name(pair.0), pair.1)
    }

    /// compat_K(ρ, ρ'): `None` if for every key position k both records are
    /// long enough and the items at k are compatible (both with the same
    /// attribute-value pair, or both unnamed with the same value), and for
    /// every key attribute name both records carry the name with the same
    /// attribute-value pair; otherwise a description of the first mismatch.
    fn key_mismatch(&self, rho: &[ItemId], rho2: &[ItemId], key: &RecordKey) -> Option<String> {
        for &k in &key.positions {
            let k = k as usize;
            if k >= rho.len() || k >= rho2.len() {
                return Some(format!("key position {k} is beyond the end of a record"));
            }
            let (a, b) = (rho[k], rho2[k]);
            match (self.avp.get(&a), self.avp.get(&b)) {
                (Some(pa), Some(pb)) => {
                    if pa != pb {
                        return Some(format!(
                            "key position {k} differs: {} vs {}",
                            self.pair_text(pa),
                            self.pair_text(pb)
                        ));
                    }
                }
                (None, None) => {
                    if self.val.get(&a) != self.val.get(&b) {
                        return Some(format!(
                            "key position {k} differs: '{}' vs '{}'",
                            self.val.get(&a).map(|s| &**s).unwrap_or("null"),
                            self.val.get(&b).map(|s| &**s).unwrap_or("null")
                        ));
                    }
                }
                _ => return Some(format!("key position {k} mixes a named and an unnamed item")),
            }
        }
        for name in &key.names {
            let (i, j) = (
                self.index_of_attribute(rho, name),
                self.index_of_attribute(rho2, name),
            );
            let (Some(i), Some(j)) = (i, j) else {
                return Some(format!("key attribute '{name}' is missing in a record"));
            };
            let (pa, pb) = (&self.avp[&rho[i]], &self.avp[&rho2[j]]);
            if pa != pb {
                return Some(format!(
                    "key attribute '{name}' differs: {} vs {}",
                    self.pair_text(pa),
                    self.pair_text(pb)
                ));
            }
        }
        None
    }

    /// agree(ρ, ρ'): every named attribute appearing in both records carries
    /// the same value.
    fn agree(&self, rho: &[ItemId], rho2: &[ItemId]) -> bool {
        let mut named: HashMap<u32, &Text> = HashMap::new();
        for &item in rho {
            if let Some((a, v)) = self.avp.get(&item) {
                named.entry(*a).or_insert(v);
            }
        }
        for &item in rho2 {
            if let Some((a, v)) = self.avp.get(&item) {
                if let Some(&existing) = named.get(a) {
                    if existing != v {
                        return false;
                    }
                }
            }
        }
        true
    }

    /// The first named attribute occurring twice in the sequence.
    fn duplicate_attribute(&self, sequence: &[ItemId]) -> Option<String> {
        let mut seen: Vec<u32> = Vec::with_capacity(sequence.len());
        self.duplicate_attribute_in(sequence, &mut seen).map(|a| a.to_string())
    }

    /// [`Self::duplicate_attribute`] with a caller-provided scratch buffer
    /// (records are short, so a linear scan beats a hash set and the buffer
    /// is reused across the millions of records of a large table).
    fn duplicate_attribute_in(&self, sequence: &[ItemId], seen: &mut Vec<u32>) -> Option<&str> {
        seen.clear();
        for &item in sequence {
            if let Some(a) = self.attr_id(item) {
                if seen.contains(&a) {
                    return Some(self.attr_name(a));
                }
                seen.push(a);
            }
        }
        None
    }

    pub fn set_avp(&mut self, item: ItemId, attribute: String, value: Text) {
        let id = self.intern_attr(&attribute);
        self.avp.insert(item, (id, value));
    }

    /// [`Self::set_avp`] with an already interned attribute name.
    pub fn set_avp_id(&mut self, item: ItemId, attribute: u32, value: Text) {
        self.avp.insert(item, (attribute, value));
    }

    // --- consistency checks (over the live anchors) ---

    pub fn is_anchor_attribute_uniform(&self) -> bool {
        let mut common: Option<u32> = None;
        for anchor_idx in self.rec.keys().copied() {
            if self.joined.contains(&anchor_idx) || self.concatenated.contains(&anchor_idx) {
                continue;
            }
            if let Some(a) = self.attr_id(ItemId::Cell(anchor_idx)) {
                match common {
                    None => common = Some(a),
                    Some(c) => {
                        if c != a {
                            return false;
                        }
                    }
                }
            }
        }
        true
    }

    pub fn is_record_attributes_distinct(&self) -> bool {
        let mut seen: Vec<u32> = Vec::new();
        for (anchor_idx, records) in self.rec.iter() {
            if self.joined.contains(anchor_idx) || self.concatenated.contains(anchor_idx) {
                continue;
            }
            for sequence in records.as_slice() {
                if self.duplicate_attribute_in(sequence, &mut seen).is_some() {
                    return false;
                }
            }
        }
        true
    }

    pub fn is_recordset_consistent(&self) -> bool {
        self.is_anchor_attribute_uniform() && self.is_record_attributes_distinct()
    }
}

// ---------------------------------------------------------------- unit tests

#[cfg(test)]
mod tests {
    use super::*;
    use crate::syntax::SyntaxCore;

    // ------------------------------------------------ working state fixtures

    /// A semantics layer of VALUE items placed at the given cells (index 0
    /// unless a `(text, row, col, index)` quadruple says otherwise).
    fn sem_of(items: &[(&str, usize, usize, usize)]) -> SemanticsCore {
        SemanticsCore {
            cell_items: items
                .iter()
                .map(|&(s, r, c, i)| CellItem {
                    s: Text::from(s),
                    tags: Vec::new(),
                    index: i,
                    row: r,
                    col: c,
                    ty: ItemType::Value,
                    span: (0, s.len()),
                })
                .collect(),
            ctx_items: Vec::new(),
            actions: Vec::new(),
            action_templates: HashMap::new(),
        }
    }

    fn init(sem: &SemanticsCore, strict: bool) -> WorkingState {
        let mut ws = WorkingState::new(strict);
        for (i, it) in sem.cell_items.iter().enumerate() {
            ws.val.insert(ItemId::Cell(i), it.s.clone());
        }
        ws
    }

    fn name(ws: &mut WorkingState, sem: &SemanticsCore, item: usize, attribute: &str) {
        ws.set_avp(ItemId::Cell(item), attribute.to_string(), sem.cell_items[item].s.clone());
    }

    fn cells(ids: &[usize]) -> Vec<ItemId> {
        ids.iter().map(|&i| ItemId::Cell(i)).collect()
    }

    fn key_pos(k: &[i64]) -> RecordKey {
        RecordKey::positions(k.iter().copied()).unwrap()
    }

    fn key_names(a: &[&str]) -> RecordKey {
        RecordKey::names(a.iter().map(|s| s.to_string())).unwrap()
    }

    // ------------------------------------------------ O_concat (port of WorkingStateConcatTest)

    #[test]
    fn concat_folds_records_with_distinct_attributes_key_not_repeated() {
        // T-1 | D16   rec(0) = <ID:T-1, REF_TP:D16>
        // T-1 | 001   rec(2) = <ID:T-1, REF_SN:001>
        let sem = sem_of(&[("T-1", 1, 0, 0), ("D16", 1, 1, 0), ("T-1", 2, 0, 0), ("001", 2, 1, 0)]);
        let mut ws = init(&sem, false);
        name(&mut ws, &sem, 0, "ID");
        name(&mut ws, &sem, 1, "REF_TP");
        name(&mut ws, &sem, 2, "ID");
        name(&mut ws, &sem, 3, "REF_SN");
        ws.apply_rec(&sem, ItemId::Cell(0), &cells(&[1])).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(2), &cells(&[3])).unwrap();

        ws.apply_concat(&sem, ItemId::Cell(0), &cells(&[2]), &key_pos(&[0])).unwrap();

        assert_eq!(ws.rec(0), Some(&*vec![cells(&[0, 1, 3])]));
        assert!(!ws.has_rec(2), "the concatenated anchor is removed from dom(rec)");
        assert!(ws.is_concatenated(2));
        assert_eq!(ws.live_anchors(), vec![0]);
        assert!(ws.diagnostics().is_empty());
    }

    #[test]
    fn concat_folds_unnamed_records_task016_shape() {
        // book | 5 ; book | 6 ; book | 7  ->  book,5,6,7
        let sem = sem_of(&[
            ("book", 1, 0, 0), ("5", 1, 1, 0),
            ("book", 2, 0, 0), ("6", 2, 1, 0),
            ("book", 3, 0, 0), ("7", 3, 1, 0),
        ]);
        let mut ws = init(&sem, false);
        ws.apply_rec(&sem, ItemId::Cell(0), &cells(&[1])).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(2), &cells(&[3])).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(4), &cells(&[5])).unwrap();

        ws.apply_concat(&sem, ItemId::Cell(0), &cells(&[2, 4]), &key_pos(&[0])).unwrap();

        assert_eq!(ws.rec(0), Some(&*vec![cells(&[0, 1, 3, 5])]));
        assert!(!ws.has_rec(2) && !ws.has_rec(4));
        assert!(ws.diagnostics().is_empty());
    }

    #[test]
    fn concat_shared_named_attribute_no_effect_and_diagnostic() {
        // A | 5   rec = <ID:A, Qty:5>
        // A | 7   rec = <ID:A, Qty:7>      -- Qty occurs in both: not a key, a conflict
        let sem = sem_of(&[("A", 1, 0, 0), ("5", 1, 1, 0), ("A", 2, 0, 0), ("7", 2, 1, 0)]);
        let mut ws = init(&sem, false);
        name(&mut ws, &sem, 0, "ID");
        name(&mut ws, &sem, 1, "Qty");
        name(&mut ws, &sem, 2, "ID");
        name(&mut ws, &sem, 3, "Qty");
        ws.apply_rec(&sem, ItemId::Cell(0), &cells(&[1])).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(2), &cells(&[3])).unwrap();

        ws.apply_concat(&sem, ItemId::Cell(0), &cells(&[2]), &key_pos(&[0])).unwrap();

        assert_eq!(ws.rec(0), Some(&*vec![cells(&[0, 1])]), "anchor record unchanged");
        assert_eq!(ws.rec(2), Some(&*vec![cells(&[2, 3])]), "the other record is kept: both survive");
        assert_eq!(ws.live_anchors().len(), 2);
        assert_eq!(ws.diagnostics().len(), 1);
        let d = &ws.diagnostics()[0];
        assert_eq!((d.anchor, d.operation.as_str()), (0, "CONCAT"));
        assert!(d.message.contains("Qty"), "{}", d.message);
    }

    #[test]
    fn concat_key_position_mismatch_no_effect_and_diagnostic() {
        let sem = sem_of(&[("X", 1, 0, 0), ("5", 1, 1, 0), ("Y", 2, 0, 0), ("7", 2, 1, 0)]);
        let mut ws = init(&sem, false);
        ws.apply_rec(&sem, ItemId::Cell(0), &cells(&[1])).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(2), &cells(&[3])).unwrap();

        ws.apply_concat(&sem, ItemId::Cell(0), &cells(&[2]), &key_pos(&[0])).unwrap();

        assert_eq!(ws.rec(0), Some(&*vec![cells(&[0, 1])]));
        assert!(ws.has_rec(2));
        assert_eq!(ws.diagnostics().len(), 1);
        assert!(ws.diagnostics()[0].message.contains("key position 0"));
    }

    #[test]
    fn concat_strict_preconditions_fail_with_the_same_message() {
        let sem = sem_of(&[("A", 1, 0, 0), ("5", 1, 1, 0), ("A", 2, 0, 0), ("7", 2, 1, 0)]);
        let mut ws = init(&sem, true);
        name(&mut ws, &sem, 0, "ID");
        name(&mut ws, &sem, 1, "Qty");
        name(&mut ws, &sem, 2, "ID");
        name(&mut ws, &sem, 3, "Qty");
        ws.apply_rec(&sem, ItemId::Cell(0), &cells(&[1])).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(2), &cells(&[3])).unwrap();

        let err = ws
            .apply_concat(&sem, ItemId::Cell(0), &cells(&[2]), &key_pos(&[0]))
            .unwrap_err();
        let text = err.text();
        assert!(text.contains("Qty") && text.starts_with("CONCAT skipped at"), "{text}");
        assert_eq!(ws.diagnostics().len(), 1, "the diagnostic is recorded before failing");
    }

    #[test]
    fn concat_items_without_records_are_ignored() {
        let sem = sem_of(&[("book", 1, 0, 0), ("5", 1, 1, 0), ("book", 2, 0, 0)]);
        let mut ws = init(&sem, false);
        ws.apply_rec(&sem, ItemId::Cell(0), &cells(&[1])).unwrap();
        let before = ws.rec(0).map(<[_]>::to_vec);

        ws.apply_concat(&sem, ItemId::Cell(0), &cells(&[2]), &key_pos(&[0])).unwrap();

        assert_eq!(ws.rec(0).map(<[_]>::to_vec), before);
        assert!(ws.diagnostics().is_empty(), "no record to concatenate is not a violation");
    }

    #[test]
    fn concat_named_key_resolved_per_record_at_different_positions() {
        // rec(0) = <k, c1, a1:A>      A at position 2
        // rec(3) = <k, a1:A, c2>      A at position 1
        let sem = sem_of(&[
            ("k", 1, 0, 0), ("c1", 1, 1, 0), ("a1", 1, 2, 0),
            ("k", 2, 0, 0), ("a1", 2, 1, 0), ("c2", 2, 2, 0),
        ]);
        let mut ws = init(&sem, false);
        name(&mut ws, &sem, 2, "A");
        name(&mut ws, &sem, 4, "A");
        ws.apply_rec(&sem, ItemId::Cell(0), &cells(&[1, 2])).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(3), &cells(&[4, 5])).unwrap();

        let key = RecordKey::new([0].into(), ["A".to_string()].into()).unwrap();
        ws.apply_concat(&sem, ItemId::Cell(0), &cells(&[3]), &key).unwrap();

        assert_eq!(
            ws.rec(0),
            Some(&*vec![cells(&[0, 1, 2, 5])]),
            "the anchor keeps its key; A is dropped from the concatenated record at its own position"
        );
        assert!(!ws.has_rec(3));
        assert!(ws.diagnostics().is_empty());
    }

    #[test]
    fn concat_named_key_missing_in_a_record_no_effect_and_diagnostic() {
        let sem = sem_of(&[("k", 1, 0, 0), ("a1", 1, 1, 0), ("k", 2, 0, 0), ("c2", 2, 1, 0)]);
        let mut ws = init(&sem, false);
        name(&mut ws, &sem, 1, "A");
        ws.apply_rec(&sem, ItemId::Cell(0), &cells(&[1])).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(2), &cells(&[3])).unwrap();

        ws.apply_concat(&sem, ItemId::Cell(0), &cells(&[2]), &key_names(&["A"])).unwrap();

        assert_eq!(ws.rec(0), Some(&*vec![cells(&[0, 1])]));
        assert!(ws.has_rec(2), "no effect: both records remain");
        assert_eq!(ws.diagnostics().len(), 1);
        assert!(ws.diagnostics()[0].message.contains("key attribute 'A' is missing"));
    }

    #[test]
    fn concat_named_key_values_differ_no_effect_and_diagnostic() {
        let sem = sem_of(&[("k", 1, 0, 0), ("a1", 1, 1, 0), ("k", 2, 0, 0), ("a2", 2, 1, 0)]);
        let mut ws = init(&sem, false);
        name(&mut ws, &sem, 1, "A");
        name(&mut ws, &sem, 3, "A");
        ws.apply_rec(&sem, ItemId::Cell(0), &cells(&[1])).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(2), &cells(&[3])).unwrap();

        let key = RecordKey::new([0].into(), ["A".to_string()].into()).unwrap();
        ws.apply_concat(&sem, ItemId::Cell(0), &cells(&[2]), &key).unwrap();

        assert!(ws.has_rec(2));
        assert_eq!(ws.diagnostics().len(), 1);
        assert!(ws.diagnostics()[0].message.contains("key attribute 'A' differs"));
    }

    #[test]
    fn concat_mixed_key_is_equivalent_to_the_positional_key_task098_shape() {
        // k1 | k11 | a1:A | b1:B | c1        CONCAT(0,1,2,3) == CONCAT(0,1,'A','B')
        // k1 | k11 | a1:A | b1:B | c2
        let mixed = RecordKey::new([0, 1].into(), ["A".to_string(), "B".to_string()].into()).unwrap();
        for key in [key_pos(&[0, 1, 2, 3]), mixed] {
            let sem = sem_of(&[
                ("k1", 1, 0, 0), ("k11", 1, 1, 0), ("a1", 1, 2, 0), ("b1", 1, 3, 0), ("c1", 1, 4, 0),
                ("k1", 2, 0, 0), ("k11", 2, 1, 0), ("a1", 2, 2, 0), ("b1", 2, 3, 0), ("c2", 2, 4, 0),
            ]);
            let mut ws = init(&sem, false);
            name(&mut ws, &sem, 2, "A");
            name(&mut ws, &sem, 3, "B");
            name(&mut ws, &sem, 7, "A");
            name(&mut ws, &sem, 8, "B");
            ws.apply_rec(&sem, ItemId::Cell(0), &cells(&[1, 2, 3, 4])).unwrap();
            ws.apply_rec(&sem, ItemId::Cell(5), &cells(&[6, 7, 8, 9])).unwrap();

            ws.apply_concat(&sem, ItemId::Cell(0), &cells(&[5]), &key).unwrap();

            assert_eq!(ws.rec(0), Some(&*vec![cells(&[0, 1, 2, 3, 4, 9])]));
            assert!(ws.diagnostics().is_empty());
        }
    }

    // ------------------------------------------------ O_join (port of WorkingStateJoinTest)

    // id  | x | y      tokens a (0), b (1) of one cell; rec(2) = <value:1, var:x>, rec(3) = <value:2, var:y>
    // a;b | 1 | 2
    fn explode_stack() -> (SemanticsCore, WorkingState) {
        let sem = sem_of(&[
            ("a", 1, 0, 0), ("b", 1, 0, 1),
            ("1", 1, 1, 0), ("2", 1, 2, 0),
            ("x", 0, 1, 0), ("y", 0, 2, 0),
        ]);
        let mut ws = init(&sem, false);
        name(&mut ws, &sem, 0, "id");
        name(&mut ws, &sem, 1, "id");
        name(&mut ws, &sem, 2, "value");
        name(&mut ws, &sem, 3, "value");
        name(&mut ws, &sem, 4, "var");
        name(&mut ws, &sem, 5, "var");
        ws.apply_rec(&sem, ItemId::Cell(0), &[]).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(1), &[]).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(2), &cells(&[4])).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(3), &cells(&[5])).unwrap();
        (sem, ws)
    }

    #[test]
    fn join_cross_product_one_record_per_joined_record_in_nested_loop_order() {
        let (sem, mut ws) = explode_stack();

        ws.apply_join(&sem, ItemId::Cell(0), &cells(&[2, 3]), &RecordKey::empty()).unwrap();

        assert_eq!(ws.rec(0), Some(&*vec![cells(&[0, 2, 4]), cells(&[0, 3, 5])]));
        assert!(ws.is_joined(2) && ws.is_joined(3));
        assert_eq!(ws.all_joined().iter().collect::<Vec<_>>(), vec![2, 3]);
        assert!(ws.rec(2).is_some(), "a joined-away anchor keeps its records");
        assert_eq!(ws.live_anchors(), vec![0, 1], "live anchors only");
        assert!(ws.diagnostics().is_empty());
    }

    #[test]
    fn join_lazy_consumption_second_anchor_joins_the_same_records() {
        let (sem, mut ws) = explode_stack();

        ws.apply_join(&sem, ItemId::Cell(0), &cells(&[2, 3]), &RecordKey::empty()).unwrap();
        ws.apply_join(&sem, ItemId::Cell(1), &cells(&[2, 3]), &RecordKey::empty()).unwrap();

        assert_eq!(ws.rec(0).unwrap().len(), 2);
        assert_eq!(ws.rec(1), Some(&*vec![cells(&[1, 2, 4]), cells(&[1, 3, 5])]));
        assert!(
            ws.is_recordset_consistent(),
            "the 'value' attribute of the joined-away anchors does not break uniformity"
        );
    }

    #[test]
    fn join_composition_second_join_multiplies_further() {
        let (mut sem, mut ws) = explode_stack();
        for (s, r, c) in [("p", 2, 3), ("q", 2, 4)] {
            sem.cell_items.push(CellItem {
                s: Text::from(s),
                tags: Vec::new(),
                index: 0,
                row: r,
                col: c,
                ty: ItemType::Value,
                span: (0, 1),
            });
        }
        ws.val.insert(ItemId::Cell(6), "p".into());
        ws.val.insert(ItemId::Cell(7), "q".into());
        name(&mut ws, &sem, 6, "w");
        name(&mut ws, &sem, 7, "w");
        ws.apply_rec(&sem, ItemId::Cell(6), &[]).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(7), &[]).unwrap();

        ws.apply_join(&sem, ItemId::Cell(0), &cells(&[2, 3]), &RecordKey::empty()).unwrap();
        ws.apply_join(&sem, ItemId::Cell(0), &cells(&[6, 7]), &RecordKey::empty()).unwrap();

        assert_eq!(
            ws.rec(0),
            Some(&*vec![
                cells(&[0, 2, 4, 6]), cells(&[0, 2, 4, 7]),
                cells(&[0, 3, 5, 6]), cells(&[0, 3, 5, 7]),
            ])
        );
    }

    #[test]
    fn join_equi_join_on_key_position_drops_mismatched_pairs_and_the_joined_key() {
        // rec(p1) = <X, Qty:5>   rec(q1) = <X, Unit:kg>   rec(q2) = <Z, Unit:pc>
        let sem = sem_of(&[
            ("X", 1, 0, 0), ("5", 1, 1, 0),
            ("X", 1, 2, 0), ("kg", 1, 3, 0),
            ("Z", 2, 2, 0), ("pc", 2, 3, 0),
        ]);
        let mut ws = init(&sem, false);
        name(&mut ws, &sem, 1, "Qty");
        name(&mut ws, &sem, 3, "Unit");
        name(&mut ws, &sem, 5, "Unit");
        ws.apply_rec(&sem, ItemId::Cell(0), &cells(&[1])).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(2), &cells(&[3])).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(4), &cells(&[5])).unwrap();

        ws.apply_join(&sem, ItemId::Cell(0), &cells(&[2, 4]), &key_pos(&[0])).unwrap();

        assert_eq!(ws.rec(0), Some(&*vec![cells(&[0, 1, 3])]));
        assert!(ws.diagnostics().is_empty());
    }

    #[test]
    fn join_no_surviving_pair_anchor_keeps_its_records_left_outer() {
        let sem = sem_of(&[("Y", 1, 0, 0), ("8", 1, 1, 0), ("X", 1, 2, 0), ("kg", 1, 3, 0)]);
        let mut ws = init(&sem, false);
        name(&mut ws, &sem, 1, "Qty");
        name(&mut ws, &sem, 3, "Unit");
        ws.apply_rec(&sem, ItemId::Cell(0), &cells(&[1])).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(2), &cells(&[3])).unwrap();

        ws.apply_join(&sem, ItemId::Cell(0), &cells(&[2]), &key_pos(&[0])).unwrap();

        assert_eq!(ws.rec(0), Some(&*vec![cells(&[0, 1])]));
        assert!(ws.is_joined(2), "J is still extended");
        assert_eq!(ws.diagnostics().len(), 1);
        assert_eq!(ws.diagnostics()[0].operation, "JOIN");
    }

    #[test]
    fn join_shared_named_attribute_is_a_natural_join_condition() {
        // rec(r1) = <Store:S1, Year:2024>; rec(s1) = <Year:2024, Sales:10>; rec(s2) = <Year:2025, Sales:12>
        let sem = sem_of(&[
            ("S1", 1, 0, 0), ("2024", 1, 1, 0),
            ("2024", 2, 0, 0), ("10", 2, 1, 0),
            ("2025", 3, 0, 0), ("12", 3, 1, 0),
        ]);
        let mut ws = init(&sem, false);
        name(&mut ws, &sem, 0, "Store");
        name(&mut ws, &sem, 1, "Year");
        name(&mut ws, &sem, 2, "Year");
        name(&mut ws, &sem, 3, "Sales");
        name(&mut ws, &sem, 4, "Year");
        name(&mut ws, &sem, 5, "Sales");
        ws.apply_rec(&sem, ItemId::Cell(0), &cells(&[1])).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(2), &cells(&[3])).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(4), &cells(&[5])).unwrap();

        ws.apply_join(&sem, ItemId::Cell(0), &cells(&[2, 4]), &RecordKey::empty()).unwrap();

        let records = ws.rec(0).unwrap();
        assert_eq!(records.len(), 1, "the 2025 pair disagrees on Year and is dropped");
        assert_eq!(records[0], cells(&[0, 1, 3]), "Year occurs once (dedup)");
        assert!(ws.diagnostics().is_empty());
    }

    #[test]
    fn join_items_without_records_are_ignored() {
        let (mut sem, mut ws) = explode_stack();
        sem.cell_items.push(CellItem {
            s: "z".into(),
            tags: Vec::new(),
            index: 0,
            row: 3,
            col: 0,
            ty: ItemType::Value,
            span: (0, 1),
        });
        ws.val.insert(ItemId::Cell(6), "z".into());

        ws.apply_join(&sem, ItemId::Cell(0), &cells(&[6]), &RecordKey::empty()).unwrap();

        assert_eq!(ws.rec(0), Some(&*vec![cells(&[0])]));
        assert!(ws.all_joined().is_empty());
        assert!(ws.diagnostics().is_empty());
    }

    #[test]
    fn join_named_key_pair_without_the_attribute_is_dropped_unlike_the_bare_join() {
        // rec(p) = <x, 2024:Year>; rec(s1) = <y, 2024:Year, 10:Sales>; rec(s2) = <z, 12:Sales> (no Year)
        let sem = sem_of(&[
            ("x", 1, 0, 0), ("2024", 1, 1, 0),
            ("y", 2, 0, 0), ("2024", 2, 1, 0), ("10", 2, 2, 0),
            ("z", 3, 0, 0), ("12", 3, 2, 0),
        ]);
        let mut ws = init(&sem, false);
        name(&mut ws, &sem, 1, "Year");
        name(&mut ws, &sem, 3, "Year");
        name(&mut ws, &sem, 4, "Sales");
        name(&mut ws, &sem, 6, "Sales");
        ws.apply_rec(&sem, ItemId::Cell(0), &cells(&[1])).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(2), &cells(&[3, 4])).unwrap();
        ws.apply_rec(&sem, ItemId::Cell(5), &cells(&[6])).unwrap();

        ws.apply_join(&sem, ItemId::Cell(0), &cells(&[2, 5]), &key_names(&["Year"])).unwrap();

        assert_eq!(
            ws.rec(0),
            Some(&*vec![cells(&[0, 1, 2, 4])]),
            "only the pair carrying Year on both sides survives; the joined key is not repeated"
        );
        assert_eq!(ws.all_joined().iter().collect::<Vec<_>>(), vec![2, 5]);
    }

    // ------------------------------------------------ provider index equivalence

    /// Reference definition Υ^{J,k}_{τ,κ}(anchor) = Ω_τ(Φ_κ(anchor, J \ {anchor}))[:k]
    /// — full scan and sort (the implementation up to 0.5.1).
    #[allow(clippy::too_many_arguments)]
    fn provide_reference(
        cond: &FilterCond,
        order: TraversalOrder,
        cardinality: i64,
        kind: CellKind,
        exclude_anchor: bool,
        anchor_idx: usize,
        sem: &SemanticsCore,
        env: &EvalEnv,
    ) -> Vec<ItemId> {
        let anch = &sem.cell_items[anchor_idx];
        let mut filtered: Vec<usize> = Vec::new();
        for (i, cand) in sem.cell_items.iter().enumerate() {
            if exclude_anchor && i == anchor_idx {
                continue;
            }
            let type_ok = match kind {
                CellKind::Unrestricted | CellKind::Aux => true,
                CellKind::Val => cand.ty == ItemType::Value,
                CellKind::Attr => cand.ty == ItemType::Attribute,
            };
            if type_ok && cond.eval(anch, cand, env).unwrap() {
                filtered.push(i);
            }
        }
        filtered.sort_by(|&x, &y| {
            let a = &sem.cell_items[x];
            let b = &sem.cell_items[y];
            if a.row == b.row && a.col == b.col {
                return a.index.cmp(&b.index);
            }
            match order {
                TraversalOrder::RowMajor => a.row.cmp(&b.row).then(a.col.cmp(&b.col)),
                TraversalOrder::ReverseRowMajor => b.row.cmp(&a.row).then(b.col.cmp(&a.col)),
                TraversalOrder::ColumnMajor => a.col.cmp(&b.col).then(a.row.cmp(&b.row)),
                TraversalOrder::ReverseColumnMajor => b.col.cmp(&a.col).then(b.row.cmp(&a.row)),
            }
        });
        if cardinality != UNBOUNDED && filtered.len() as i64 > cardinality {
            filtered.truncate(cardinality as usize);
        }
        filtered.into_iter().map(ItemId::Cell).collect()
    }

    /// Tiny deterministic PRNG (xorshift) — no dev-dependency needed.
    struct Rng(u64);
    impl Rng {
        fn next(&mut self) -> u64 {
            let mut x = self.0;
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            self.0 = x;
            x
        }
        fn below(&mut self, n: usize) -> usize {
            (self.next() % n as u64) as usize
        }
    }

    /// Every filter term the scope derivation distinguishes, plus position,
    /// content and tag terms (scope ALL), in random combinations.
    fn random_cond(rng: &mut Rng) -> FilterCond {
        fn term(rng: &mut Rng) -> FilterTerm {
            let d = rng.below(5) as i64 - 2;
            match rng.below(24) {
                0 => FilterTerm::LeftOf,
                1 => FilterTerm::RightOf,
                2 => FilterTerm::Above,
                3 => FilterTerm::Below,
                4 => FilterTerm::SameSubrow,
                5 => FilterTerm::SameSubcol,
                6 => FilterTerm::SameSubtable,
                7 => FilterTerm::SameRow,
                8 => FilterTerm::SameCol,
                9 => FilterTerm::NotSameCell,
                10 => FilterTerm::SameCell,
                11 => FilterTerm::ColExact(rng.below(6) as i64),
                12 => FilterTerm::ColOffset(d),
                13 => FilterTerm::ColRange(d, if rng.below(2) == 0 { UNBOUNDED } else { d + rng.below(3) as i64 }),
                14 => FilterTerm::ColAbsoluteRange(rng.below(4) as i64, if rng.below(2) == 0 { UNBOUNDED } else { rng.below(6) as i64 }),
                15 => FilterTerm::RowExact(rng.below(6) as i64),
                16 => FilterTerm::RowOffset(d),
                17 => FilterTerm::RowAbsoluteRange(rng.below(4) as i64, if rng.below(2) == 0 { UNBOUNDED } else { rng.below(6) as i64 }),
                18 => FilterTerm::PosExact(rng.below(3) as i64),
                19 => FilterTerm::PosOffset(d),
                20 => FilterTerm::Blank,
                21 => FilterTerm::NotBlank,
                22 => FilterTerm::SameStr,
                _ => FilterTerm::Tagged("#t".into()),
            }
        }
        match rng.below(4) {
            0 => FilterCond::Bare(term(rng)),
            1 => FilterCond::Or(vec![vec![term(rng)], vec![term(rng), term(rng)]]),
            _ => FilterCond::And((0..1 + rng.below(3)).map(|_| term(rng)).collect()),
        }
    }

    #[test]
    fn indexed_provider_equals_the_reference_definition_on_random_tables() {
        let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
        let orders = [
            TraversalOrder::RowMajor,
            TraversalOrder::ReverseRowMajor,
            TraversalOrder::ColumnMajor,
            TraversalOrder::ReverseColumnMajor,
        ];
        let kinds = [CellKind::Unrestricted, CellKind::Val, CellKind::Attr, CellKind::Aux];
        let mut checked = 0usize;
        for _ in 0..25 {
            let num_rows = 2 + rng.below(6);
            let num_cols = 2 + rng.below(6);
            let mut syntax = SyntaxCore::new(num_rows, num_cols).unwrap();
            // random subtable boundaries and subrows
            let mut starts = vec![0];
            for r in 1..num_rows {
                if rng.below(3) == 0 {
                    starts.push(r);
                }
            }
            syntax.define_subtables(&starts).unwrap();
            for r in 0..num_rows {
                if num_cols > 1 && rng.below(2) == 0 {
                    let split = 1 + rng.below(num_cols - 1);
                    syntax.define_subrow(r, 0, split - 1).unwrap();
                    syntax.define_subrow(r, split, num_cols - 1).unwrap();
                }
            }
            // 0–3 items per cell, random types, some blank / tagged / repeated texts
            let mut sem = SemanticsCore::default();
            let texts = ["a", "b", "", " ", "c"];
            for r in 0..num_rows {
                for c in 0..num_cols {
                    let n = rng.below(4);
                    for i in 0..n {
                        let t = texts[rng.below(texts.len())];
                        syntax.cell_mut(r, c).set_text(t.to_string());
                        sem.cell_items.push(CellItem {
                            s: Text::from(t),
                            tags: if rng.below(3) == 0 { vec!["#t".into()] } else { Vec::new() },
                            index: i,
                            row: r,
                            col: c,
                            ty: [ItemType::Value, ItemType::Attribute, ItemType::Auxiliary][rng.below(3)],
                            span: (0, t.len()),
                        });
                    }
                }
            }
            if sem.cell_items.is_empty() {
                continue;
            }
            let env = EvalEnv { syntax: &syntax, py_table: None };
            // the index must serve subtable-scoped column-major providers too
            sem.actions.push(ActionInst {
                anchor: ItemId::Cell(0),
                template: std::sync::Arc::new(ActionTemplate {
                    providers: vec![ProviderInst::Cell {
                        cond: FilterCond::Bare(FilterTerm::SameSubtable),
                        order: TraversalOrder::ColumnMajor,
                        cardinality: UNBOUNDED,
                        kind: CellKind::Unrestricted,
                        exclude_anchor: true,
                        lenient: true,
                        scope: CandidateScope::subtable(),
                    }],
                    op: OpInst::Rec,
                    inherited: false,
                }),
            });
            let index = ItemIndex::build(&sem, &syntax);
            for _ in 0..60 {
                let cond = random_cond(&mut rng);
                let scope = CandidateScope::of_cond(&cond);
                for &order in &orders {
                    for &cardinality in &[UNBOUNDED, 1, 2, 3, 0] {
                        for &kind in &kinds {
                            for exclude_anchor in [true, false] {
                                let anchor_idx = rng.below(sem.cell_items.len());
                                let anch = &sem.cell_items[anchor_idx];
                                let compatible = match kind {
                                    CellKind::Unrestricted => true,
                                    CellKind::Val | CellKind::Attr => anch.ty == ItemType::Value,
                                    CellKind::Aux => anch.ty != ItemType::Auxiliary,
                                };
                                if !compatible {
                                    continue;
                                }
                                let provider = ProviderInst::Cell {
                                    cond: cond.clone(),
                                    order,
                                    cardinality,
                                    kind,
                                    exclude_anchor,
                                    lenient: false,
                                    scope,
                                };
                                let got = provider
                                    .provide(ItemId::Cell(anchor_idx), &sem, &env, &index)
                                    .unwrap();
                                let want = provide_reference(
                                    &cond, order, cardinality, kind, exclude_anchor, anchor_idx, &sem, &env,
                                );
                                assert_eq!(got, want, "cond={cond:?} order={order:?} k={cardinality} kind={kind:?} anchor={anchor_idx}");
                                checked += 1;
                            }
                        }
                    }
                }
            }
        }
        assert!(checked > 10_000, "checked only {checked} provider calls");
    }
}
