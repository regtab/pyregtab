//! Port of `ru.icc.regtab.itm.semantics`: items, providers, working state.
//! Java identity-keyed maps become `ItemId`-keyed insertion-ordered maps.

use crate::spec::{CellKind, CtxKind, EvalEnv, FilterCond, ItemType, TraversalOrder, UNBOUNDED};
use crate::util::CoreResult;
use indexmap::IndexMap;
use std::collections::BTreeSet;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ItemId {
    Cell(usize),
    Ctx(usize),
}

/// Cell-derived item (s, tags, index) bound to its source cell (row, col).
#[derive(Clone, Debug)]
pub struct CellItem {
    pub s: String,
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

/// Context-derived item.
#[derive(Clone, Debug)]
pub struct CtxItem {
    pub s: String,
    pub ty: ItemType,
    pub const_value: Option<String>,
}

/// One instantiated interpretation action.
#[derive(Clone, Debug)]
pub struct ActionInst {
    pub anchor: ItemId,
    pub providers: Vec<ProviderInst>,
    pub op: OpInst,
}

#[derive(Clone, Debug)]
pub enum OpInst {
    Fill(String),
    Prefix(String),
    Suffix(String),
    Avp,
    Rec,
    Join(BTreeSet<i64>),
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
    },
    /// Context-derived provider over indices into `SemanticsCore::ctx_items`.
    Ctx { items: Vec<usize>, kind: CtxKind },
}

#[derive(Clone, Debug, Default)]
pub struct SemanticsCore {
    pub cell_items: Vec<CellItem>,
    pub ctx_items: Vec<CtxItem>,
    pub actions: Vec<ActionInst>,
}

impl SemanticsCore {
    /// True if interpreting may call back into Python (Custom/External
    /// filter conditions inside instantiated providers).
    pub fn has_py_callbacks(&self) -> bool {
        self.actions.iter().any(|a| {
            a.providers.iter().any(|p| match p {
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

impl ProviderInst {
    /// Port of `ItemProvider.provide(anchor)`.
    pub fn provide(
        &self,
        anchor: ItemId,
        sem: &SemanticsCore,
        env: &EvalEnv,
    ) -> CoreResult<Vec<ItemId>> {
        match self {
            ProviderInst::Ctx { items, kind } => {
                self.provide_ctx(anchor, sem, items, *kind)
            }
            ProviderInst::Cell { cond, order, cardinality, kind, exclude_anchor, lenient } => {
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
                        return Ok(Vec::new());
                    }
                    return Err(format!(
                        "Υ_tbl^val and Υ_tbl^attr require a value-associated anchor, got: {:?}",
                        anch.ty
                    )
                    .into());
                }
                // Candidates: all cell items, minus anchor, restricted by kind.
                let mut filtered: Vec<usize> = Vec::new();
                for (i, cand) in sem.cell_items.iter().enumerate() {
                    if *exclude_anchor && i == anchor_idx {
                        continue;
                    }
                    let type_ok = match kind {
                        CellKind::Unrestricted | CellKind::Aux => true,
                        CellKind::Val => cand.ty == ItemType::Value,
                        CellKind::Attr => cand.ty == ItemType::Attribute,
                    };
                    if !type_ok {
                        continue;
                    }
                    if cond.eval(anch, cand, env)? {
                        filtered.push(i);
                    }
                }
                // Linearization Ω_τ.
                filtered.sort_by(|&x, &y| {
                    let a = &sem.cell_items[x];
                    let b = &sem.cell_items[y];
                    if a.row == b.row && a.col == b.col {
                        return a.index.cmp(&b.index);
                    }
                    match order {
                        TraversalOrder::RowMajor => {
                            a.row.cmp(&b.row).then(a.col.cmp(&b.col))
                        }
                        TraversalOrder::ReverseRowMajor => {
                            b.row.cmp(&a.row).then(b.col.cmp(&a.col))
                        }
                        TraversalOrder::ColumnMajor => {
                            a.col.cmp(&b.col).then(a.row.cmp(&b.row))
                        }
                        TraversalOrder::ReverseColumnMajor => {
                            b.col.cmp(&a.col).then(b.row.cmp(&a.row))
                        }
                    }
                });
                if *cardinality != UNBOUNDED && filtered.len() as i64 > *cardinality {
                    filtered.truncate(*cardinality as usize);
                }
                Ok(filtered.into_iter().map(ItemId::Cell).collect())
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

// ---------------------------------------------------------------- WorkingState

/// Port of `WorkingState`: insertion-ordered maps keyed by item identity.
#[derive(Default, Debug)]
pub struct WorkingState {
    pub val: IndexMap<ItemId, String>,
    pub attr: IndexMap<ItemId, String>,
    pub avp: IndexMap<ItemId, (String, String)>,
    pub rec: IndexMap<usize, Vec<ItemId>>, // keyed by cell-item index
}

impl WorkingState {
    pub fn assoc(&self, item: ItemId) -> Option<&str> {
        self.avp.get(&item).map(|(a, _)| a.as_str())
    }

    fn get_val_or_attr(&self, sem: &SemanticsCore, anchor: ItemId) -> CoreResult<String> {
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
                self.val.insert(anchor, value);
                Ok(())
            }
            ItemType::Attribute => {
                self.attr.insert(anchor, value);
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
        self.avp.insert(anchor, (a, v));
        Ok(())
    }

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
                    self.avp.insert(item, (ctx.s.clone(), cv.clone()));
                }
            }
            sequence.push(item);
        }
        self.rec.insert(anchor_idx, sequence);
        Ok(())
    }

    pub fn apply_join(
        &mut self,
        anchor: ItemId,
        items: &[ItemId],
        key_positions: &BTreeSet<i64>,
    ) -> CoreResult<()> {
        let ItemId::Cell(anchor_idx) = anchor else {
            return Err("O_join requires a cell-derived anchor".into());
        };
        let Some(anchor_rec) = self.rec.get(&anchor_idx).cloned() else {
            return Ok(());
        };
        if items.is_empty() {
            return Ok(());
        }
        let mut result = anchor_rec;
        for &item in items {
            let ItemId::Cell(ci) = item else { continue };
            let Some(other_rec) = self.rec.get(&ci).cloned() else {
                continue;
            };
            for (pos, it) in other_rec.iter().enumerate() {
                if !key_positions.contains(&(pos as i64)) {
                    result.push(*it);
                }
            }
            self.rec.shift_remove(&ci);
        }
        // dedup by named attribute: keep first occurrence.
        let mut seen: Vec<String> = Vec::new();
        let mut deduped = Vec::with_capacity(result.len());
        for it in result {
            match self.assoc(it) {
                Some(a) => {
                    if !seen.iter().any(|x| x == a) {
                        seen.push(a.to_string());
                        deduped.push(it);
                    }
                }
                None => deduped.push(it),
            }
        }
        self.rec.insert(anchor_idx, deduped);
        Ok(())
    }

    pub fn set_avp(&mut self, item: ItemId, attribute: String, value: String) {
        self.avp.insert(item, (attribute, value));
    }

    // --- consistency checks ---

    pub fn is_anchor_attribute_uniform(&self) -> bool {
        let mut common: Option<&str> = None;
        for &anchor_idx in self.rec.keys() {
            if let Some(a) = self.assoc(ItemId::Cell(anchor_idx)) {
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
        for sequence in self.rec.values() {
            let mut seen: Vec<&str> = Vec::new();
            for &item in sequence {
                if let Some(a) = self.assoc(item) {
                    if seen.contains(&a) {
                        return false;
                    }
                    seen.push(a);
                }
            }
        }
        true
    }

    pub fn is_recordset_consistent(&self) -> bool {
        self.is_anchor_attribute_uniform() && self.is_record_attributes_distinct()
    }
}
