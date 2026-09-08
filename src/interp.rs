//! Port of `ru.icc.regtab.interpret.TableInterpreter`: 4 interpretation phases.
//!
//! Working state completion applies the actions in operation-type order
//! `FILL/PREFIX/SUFFIX → AVP → REC → CONCAT → JOIN`: records are folded by
//! concatenation before they are multiplied by joins. Preconditions violated
//! by `CONCAT`/`JOIN` leave the working state unchanged and are reported
//! through the returned [`Diagnostic`]s (or fail under strict preconditions).

use crate::recordset::{RecordCore, RecordsetCore, Schema};
use crate::semantics::{ActionInst, Diagnostic, ItemId, ItemIndex, OpInst, SemanticsCore, WorkingState};
use crate::spec::{EvalEnv, ItemType, PyFunc, Transformation};
use crate::syntax::SyntaxCore;
use crate::util::CoreResult;
use std::collections::HashMap;

#[cfg_attr(feature = "python", pyo3::pyclass(eq, eq_int, name = "SchemaConstructionStrategy", rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SchemaStrategy {
    RecordFirst,
    PositionFirst,
}

#[cfg_attr(feature = "python", pyo3::pyclass(eq, eq_int, name = "ActionApplicationStrategy", rename_all = "SCREAMING_SNAKE_CASE"))]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActionStrategy {
    RowFirst,
    ColumnFirst,
}

pub struct InterpreterCfg {
    pub strategy: SchemaStrategy,
    pub action_strategy: ActionStrategy,
    pub missing_value_handler: Option<PyFunc>,
    pub transformations: Vec<Transformation>,
    pub anonymous_attribute_template: String,
    /// A violated `CONCAT`/`JOIN` precondition fails the interpretation
    /// instead of having no effect (Java: `withStrictPreconditions(true)`).
    pub strict_preconditions: bool,
}

impl Default for InterpreterCfg {
    fn default() -> Self {
        InterpreterCfg {
            strategy: SchemaStrategy::RecordFirst,
            action_strategy: ActionStrategy::RowFirst,
            missing_value_handler: None,
            transformations: Vec::new(),
            anonymous_attribute_template: "$a_%i".to_string(),
            strict_preconditions: false,
        }
    }
}

/// The recordset of an interpretation together with the diagnostics of its
/// working state completion (empty when no action was skipped).
pub struct Interpretation {
    pub recordset: RecordsetCore,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn interpret(
    cfg: &InterpreterCfg,
    syntax: &SyntaxCore,
    sem: &SemanticsCore,
    py_table: Option<&crate::spec::PyTableHandle>,
) -> CoreResult<Interpretation> {
    let env = EvalEnv { syntax, py_table };

    // Phase 1: working state initialization
    let mut ws = WorkingState::new(cfg.strict_preconditions);
    for (i, item) in sem.cell_items.iter().enumerate() {
        match item.ty {
            ItemType::Value => {
                ws.val.insert(ItemId::Cell(i), item.s.clone());
            }
            ItemType::Attribute => {
                ws.attr.insert(ItemId::Cell(i), item.s.clone());
            }
            ItemType::Auxiliary => {}
        }
    }
    for (i, item) in sem.ctx_items.iter().enumerate() {
        match item.ty {
            ItemType::Value => {
                ws.val.insert(ItemId::Ctx(i), item.s.clone());
            }
            ItemType::Attribute => {
                ws.attr.insert(ItemId::Ctx(i), item.s.clone());
            }
            ItemType::Auxiliary => {}
        }
    }

    // Phase 2: working state completion
    complete_working_state(cfg, &mut ws, sem, &env)?;
    let diagnostics = ws.take_diagnostics();

    // Phase 3: recordset extraction
    let recordset = extract_recordset(cfg, &mut ws, sem)?;

    // Phase 4: recordset transformation
    let mut rs = recordset;
    for t in &cfg.transformations {
        rs = t.with_template(&cfg.anonymous_attribute_template).apply(rs)?;
    }
    Ok(Interpretation { recordset: rs, diagnostics })
}

fn anchor_pos(sem: &SemanticsCore, action: &ActionInst) -> Option<(usize, usize)> {
    match action.anchor {
        ItemId::Cell(i) => {
            let it = &sem.cell_items[i];
            Some((it.row, it.col))
        }
        ItemId::Ctx(_) => None,
    }
}

fn sort_actions(cfg: &InterpreterCfg, sem: &SemanticsCore, actions: &mut [&ActionInst]) {
    actions.sort_by(|a, b| {
        let pa = anchor_pos(sem, a);
        let pb = anchor_pos(sem, b);
        match (pa, pb) {
            (Some((r1, c1)), Some((r2, c2))) => match cfg.action_strategy {
                ActionStrategy::RowFirst => r1.cmp(&r2).then(c1.cmp(&c2)),
                ActionStrategy::ColumnFirst => c1.cmp(&c2).then(r1.cmp(&r2)),
            },
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        }
    });
}

fn complete_working_state(
    cfg: &InterpreterCfg,
    ws: &mut WorkingState,
    sem: &SemanticsCore,
    env: &EvalEnv,
) -> CoreResult<()> {
    let mut str_actions: Vec<&ActionInst> = Vec::new();
    let mut avp_actions: Vec<&ActionInst> = Vec::new();
    let mut rec_actions: Vec<&ActionInst> = Vec::new();
    let mut concat_actions: Vec<&ActionInst> = Vec::new();
    let mut join_actions: Vec<&ActionInst> = Vec::new();

    for action in &sem.actions {
        match action.op {
            OpInst::Fill(_) | OpInst::Prefix(_) | OpInst::Suffix(_) => str_actions.push(action),
            OpInst::Avp => avp_actions.push(action),
            OpInst::Rec => rec_actions.push(action),
            OpInst::Concat(_) => concat_actions.push(action),
            OpInst::Join(_) => join_actions.push(action),
        }
    }

    sort_actions(cfg, sem, &mut str_actions);
    sort_actions(cfg, sem, &mut avp_actions);
    sort_actions(cfg, sem, &mut rec_actions);
    sort_actions(cfg, sem, &mut concat_actions);
    sort_actions(cfg, sem, &mut join_actions);

    // One spatial index per interpretation, shared by all providers.
    let index = ItemIndex::build(sem, env.syntax);

    for group in [str_actions, avp_actions, rec_actions, concat_actions, join_actions] {
        for action in group {
            apply_action(ws, sem, env, &index, action)?;
        }
    }
    Ok(())
}

fn apply_action(
    ws: &mut WorkingState,
    sem: &SemanticsCore,
    env: &EvalEnv,
    index: &ItemIndex,
    action: &ActionInst,
) -> CoreResult<()> {
    let anchor = action.anchor;
    let mut items: Vec<ItemId> = Vec::new();
    for provider in &action.providers {
        items.extend(provider.provide(anchor, sem, env, index)?);
    }
    match &action.op {
        OpInst::Fill(d) => ws.apply_fill(sem, anchor, &items, d),
        OpInst::Prefix(d) => ws.apply_prefix(sem, anchor, &items, d),
        OpInst::Suffix(d) => ws.apply_suffix(sem, anchor, &items, d),
        // Empty items (e.g. lenient inherited provider) → skip
        OpInst::Avp => {
            if !items.is_empty() {
                ws.apply_avp(anchor, &items)
            } else {
                Ok(())
            }
        }
        OpInst::Rec => ws.apply_rec(sem, anchor, &items),
        OpInst::Concat(key) => {
            if items.is_empty() {
                return Ok(());
            }
            check_records(ws, sem, action, &items, "CONCAT")?;
            ws.apply_concat(sem, anchor, &items, key)
        }
        OpInst::Join(key) => {
            if items.is_empty() {
                return Ok(());
            }
            check_records(ws, sem, action, &items, "JOIN")?;
            ws.apply_join(sem, anchor, &items, key)
        }
    }
}

/// Makes the silent "not applicable" cases of `CONCAT`/`JOIN` visible for
/// *explicit* actions: an anchor without a record, or provided items none of
/// which has a record — both usually a forgotten `REC`. Inherited actions
/// reach anchors that were never meant to carry records, so for them these
/// cases are routine and not reported. An anchor whose record was folded away
/// by an earlier `CONCAT` (ι ∈ C) is routine as well: its own `CONCAT` is
/// applied after the one that consumed it.
fn check_records(
    ws: &mut WorkingState,
    sem: &SemanticsCore,
    action: &ActionInst,
    items: &[ItemId],
    operation: &str,
) -> CoreResult<()> {
    if action.inherited {
        return Ok(());
    }
    let ItemId::Cell(anchor) = action.anchor else {
        return Ok(());
    };
    if !ws.has_rec(anchor) {
        if !ws.is_concatenated(anchor) {
            ws.report(sem, anchor, operation, "anchor has no record — REC missing?")?;
        }
        return Ok(());
    }
    for &item in items {
        if let ItemId::Cell(c) = item {
            if c != anchor && (ws.has_rec(c) || ws.is_concatenated(c)) {
                return Ok(());
            }
        }
    }
    ws.report(
        sem,
        anchor,
        operation,
        "none of the provided items has a record — REC missing on the provider side?",
    )
}

fn anonymous_attribute(cfg: &InterpreterCfg, index: usize) -> String {
    cfg.anonymous_attribute_template.replace("%i", &index.to_string())
}

fn extract_recordset(
    cfg: &InterpreterCfg,
    ws: &mut WorkingState,
    sem: &SemanticsCore,
) -> CoreResult<RecordsetCore> {
    if !ws.is_recordset_consistent() {
        return Err("Working state is not recordset-consistent".into());
    }
    let schema = construct_schema(cfg, ws)?;
    let records = generate_records(cfg, ws, sem, &schema)?;
    Ok(RecordsetCore { schema, records })
}

/// Visit order of `(anchor, record, position)` triples for schema construction
/// (port of `SchemaConstructionStrategy.buildVisitOrder`): the records of an
/// anchor are always visited in their sequence order.
fn visit_order(
    strategy: SchemaStrategy,
    anchors: &[usize],
    ws: &WorkingState,
) -> Vec<(usize, usize, usize)> {
    let mut out = Vec::new();
    match strategy {
        SchemaStrategy::RecordFirst => {
            for (a, &anchor) in anchors.iter().enumerate() {
                for (r, seq) in ws.rec(anchor).into_iter().flatten().enumerate() {
                    for i in 1..seq.len() {
                        out.push((a, r, i));
                    }
                }
            }
        }
        SchemaStrategy::PositionFirst => {
            let mut max_len = 0;
            for &anchor in anchors {
                for seq in ws.rec(anchor).into_iter().flatten() {
                    max_len = max_len.max(seq.len());
                }
            }
            for i in 1..max_len {
                for (a, &anchor) in anchors.iter().enumerate() {
                    let n = ws.rec(anchor).map(|rs| rs.len()).unwrap_or(0);
                    for r in 0..n {
                        out.push((a, r, i));
                    }
                }
            }
        }
    }
    out
}

fn construct_schema(cfg: &InterpreterCfg, ws: &mut WorkingState) -> CoreResult<Schema> {
    let anchors: Vec<usize> = ws.live_anchors();

    let mut schema_attrs: Vec<String> = Vec::new();
    let mut anon_map: HashMap<usize, String> = HashMap::new();

    let mut a1: Option<String> = None;
    for &anchor in &anchors {
        if let Some(a) = ws.assoc(ItemId::Cell(anchor)) {
            a1 = Some(a.to_string());
            break;
        }
    }
    let a1 = match a1 {
        Some(a) => a,
        None => {
            let a1 = anonymous_attribute(cfg, 1);
            for &anchor in &anchors {
                if let Some(v) = ws.val.get(&ItemId::Cell(anchor)).cloned() {
                    ws.set_avp(ItemId::Cell(anchor), a1.clone(), v);
                }
            }
            a1
        }
    };
    schema_attrs.push(a1);

    let triples = visit_order(cfg.strategy, &anchors, ws);
    let mut in_schema: Vec<String> = schema_attrs.clone();

    for (a, rec_idx, pos_idx) in triples {
        let anchor = anchors[a];
        let item = match ws.rec(anchor).and_then(|rs| rs.get(rec_idx)) {
            Some(seq) if pos_idx < seq.len() => seq[pos_idx],
            _ => continue,
        };
        match ws.assoc(item).map(|s| s.to_string()) {
            Some(attr) => {
                if !in_schema.contains(&attr) {
                    in_schema.push(attr.clone());
                    schema_attrs.push(attr);
                }
            }
            None => {
                if let std::collections::hash_map::Entry::Vacant(e) = anon_map.entry(pos_idx) {
                    let anon = anonymous_attribute(cfg, pos_idx + 1);
                    e.insert(anon.clone());
                    schema_attrs.push(anon.clone());
                    in_schema.push(anon);
                }
                if let Some(v) = ws.val.get(&item).cloned() {
                    let anon = anon_map.get(&pos_idx).unwrap().clone();
                    ws.set_avp(item, anon, v);
                }
            }
        }
    }

    Schema::new(schema_attrs)
}

fn generate_records(
    cfg: &InterpreterCfg,
    ws: &WorkingState,
    _sem: &SemanticsCore,
    schema: &Schema,
) -> CoreResult<Vec<RecordCore>> {
    let n = schema.attributes.len();
    let anchors = ws.live_anchors();
    let index: HashMap<&str, usize> = schema
        .attributes
        .iter()
        .enumerate()
        .map(|(i, a)| (a.as_str(), i))
        .collect();
    // The missing values of a record depend on the schema only.
    let missing: Vec<Option<String>> = match &cfg.missing_value_handler {
        None => vec![None; n],
        Some(_) => {
            let mut m = Vec::with_capacity(n);
            for attr in &schema.attributes {
                m.push(handle_missing(cfg, attr)?);
            }
            m
        }
    };
    let mut records = Vec::with_capacity(anchors.len());
    for anchor in anchors {
        for sequence in ws.rec(anchor).into_iter().flatten() {
            let mut values = missing.clone();
            for &item in sequence {
                if let Some(a) = ws.assoc(item) {
                    if let Some(&idx) = index.get(a) {
                        values[idx] = ws.val.get(&item).cloned();
                    }
                }
            }
            records.push(RecordCore { values });
        }
    }
    Ok(records)
}

fn handle_missing(cfg: &InterpreterCfg, attribute: &str) -> CoreResult<Option<String>> {
    match &cfg.missing_value_handler {
        None => Ok(None),
        #[cfg(feature = "python")]
        Some(f) => crate::py::call_missing_handler(f, attribute),
        #[cfg(not(feature = "python"))]
        Some(f) => {
            let _ = attribute;
            match *f {}
        }
    }
}
