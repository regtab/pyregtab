//! Port of `ru.icc.regtab.interpret.TableInterpreter`: 4 interpretation phases.
//!
//! Working state completion applies the actions in operation-type order
//! `FILL/PREFIX/SUFFIX → AVP → REC → CONCAT → JOIN`: records are folded by
//! concatenation before they are multiplied by joins. Preconditions violated
//! by `CONCAT`/`JOIN` leave the working state unchanged and are reported
//! through the returned [`Diagnostic`]s (or fail under strict preconditions).

use crate::recordset::{RecordsetCore, Schema};
use crate::semantics::{ActionInst, Diagnostic, ItemId, ItemIndex, OpInst, SemanticsCore, WorkingState};
use crate::spec::{EvalEnv, PyFunc, Transformation};
use crate::syntax::SyntaxCore;
use crate::util::{CoreResult, Text};
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
    interpret_timed(cfg, syntax, sem, py_table).map(|(i, _)| i)
}

/// [`interpret`] that also reports the wall time (seconds) of its four
/// phases — initialization, completion, extraction, transformation — for
/// profiling (`examples/perf_runner.rs`).
pub fn interpret_timed(
    cfg: &InterpreterCfg,
    syntax: &SyntaxCore,
    sem: &SemanticsCore,
    py_table: Option<&crate::spec::PyTableHandle>,
) -> CoreResult<(Interpretation, [f64; 4])> {
    let env = EvalEnv { syntax, py_table };
    let mut phases = [0.0f64; 4];
    let mut clock = std::time::Instant::now();
    let mut lap = |slot: &mut f64| {
        *slot = clock.elapsed().as_secs_f64();
        clock = std::time::Instant::now();
    };

    // Phase 1: working state initialization — val(ι) = s(ι) for value items
    // and attr(ι) = s(ι) for attribute items hold by default (the state
    // stores overrides only), so only the storage is sized here.
    let mut ws = WorkingState::new(cfg.strict_preconditions);
    ws.reserve(sem.cell_items.len(), sem.ctx_items.len());
    lap(&mut phases[0]);

    // Phase 2: working state completion
    complete_working_state(cfg, &mut ws, sem, &env)?;
    let diagnostics = ws.take_diagnostics();
    lap(&mut phases[1]);

    // Phase 3: recordset extraction
    let recordset = extract_recordset(cfg, &mut ws, sem)?;
    lap(&mut phases[2]);

    // Phase 4: recordset transformation
    let mut rs = recordset;
    for t in &cfg.transformations {
        rs = t.with_template(&cfg.anonymous_attribute_template).apply(rs)?;
    }
    lap(&mut phases[3]);
    Ok((Interpretation { recordset: rs, diagnostics }, phases))
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

fn sort_actions(cfg: &InterpreterCfg, sem: &SemanticsCore, actions: &mut [u32]) {
    actions.sort_by(|&a, &b| {
        let pa = anchor_pos(sem, &sem.actions[a as usize]);
        let pb = anchor_pos(sem, &sem.actions[b as usize]);
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
    // Action indices by operation group (a million actions: indices, not refs).
    let mut groups: [Vec<u32>; 5] = Default::default();
    for (i, action) in sem.actions.iter().enumerate() {
        let g = match action.op() {
            OpInst::Fill(_) | OpInst::Prefix(_) | OpInst::Suffix(_) => 0,
            OpInst::Avp => 1,
            OpInst::Rec => 2,
            OpInst::Concat(_) => 3,
            OpInst::Join(_) => 4,
        };
        groups[g].push(i as u32);
    }
    for group in groups.iter_mut() {
        sort_actions(cfg, sem, group);
    }

    // One spatial index per interpretation, shared by all providers.
    let index = ItemIndex::build(sem, env.syntax);

    // One provider buffer for all actions.
    let mut items: Vec<ItemId> = Vec::new();
    for group in groups.iter() {
        for &i in group {
            apply_action(ws, sem, env, &index, &sem.actions[i as usize], &mut items)?;
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
    items: &mut Vec<ItemId>,
) -> CoreResult<()> {
    let anchor = action.anchor;
    items.clear();
    for provider in action.providers() {
        provider.provide_into(anchor, sem, env, index, items)?;
    }
    let items: &[ItemId] = items;
    match action.op() {
        OpInst::Fill(d) => ws.apply_fill(sem, anchor, items, d),
        OpInst::Prefix(d) => ws.apply_prefix(sem, anchor, items, d),
        OpInst::Suffix(d) => ws.apply_suffix(sem, anchor, items, d),
        // Empty items (e.g. lenient inherited provider) → skip
        OpInst::Avp => {
            if !items.is_empty() {
                ws.apply_avp(sem, anchor, items)
            } else {
                Ok(())
            }
        }
        OpInst::Rec => ws.apply_rec(sem, anchor, items),
        OpInst::Concat(key) => {
            if items.is_empty() {
                return Ok(());
            }
            check_records(ws, sem, action, items, "CONCAT")?;
            ws.apply_concat(sem, anchor, items, key)
        }
        OpInst::Join(key) => {
            if items.is_empty() {
                return Ok(());
            }
            check_records(ws, sem, action, items, "JOIN")?;
            ws.apply_join(sem, anchor, items, key)
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
    if action.inherited() {
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
    let anchors: Vec<usize> = ws.live_anchors();
    let schema = construct_schema(cfg, ws, sem, &anchors)?;
    generate_records(cfg, ws, sem, &anchors, schema)
}

/// Visits the `(anchor, record, position)` triples of schema construction
/// (port of `SchemaConstructionStrategy.buildVisitOrder`): the records of an
/// anchor are always visited in their sequence order. The visitor may name
/// items (which does not change the records), so the working state is
/// re-borrowed for every triple instead of materializing them.
fn visit_schema(
    strategy: SchemaStrategy,
    anchors: &[usize],
    ws: &mut WorkingState,
    mut visit: impl FnMut(&mut WorkingState, usize, usize, usize),
) {
    let n_records = |ws: &WorkingState, anchor: usize| ws.rec(anchor).map(|r| r.len()).unwrap_or(0);
    let rec_len = |ws: &WorkingState, anchor: usize, r: usize| {
        ws.rec(anchor).and_then(|rs| rs.get(r)).map(|seq| seq.len()).unwrap_or(0)
    };
    match strategy {
        SchemaStrategy::RecordFirst => {
            for (a, &anchor) in anchors.iter().enumerate() {
                for r in 0..n_records(ws, anchor) {
                    for i in 1..rec_len(ws, anchor, r) {
                        visit(ws, a, r, i);
                    }
                }
            }
        }
        SchemaStrategy::PositionFirst => {
            let mut max_len = 0;
            for &anchor in anchors {
                for r in 0..n_records(ws, anchor) {
                    max_len = max_len.max(rec_len(ws, anchor, r));
                }
            }
            for i in 1..max_len {
                for (a, &anchor) in anchors.iter().enumerate() {
                    for r in 0..n_records(ws, anchor) {
                        visit(ws, a, r, i);
                    }
                }
            }
        }
    }
}

fn construct_schema(
    cfg: &InterpreterCfg,
    ws: &mut WorkingState,
    sem: &SemanticsCore,
    anchors: &[usize],
) -> CoreResult<Schema> {
    let mut schema_attrs: Vec<String> = Vec::new();
    // Anonymous attribute (by position) → its interned id.
    let mut anon_map: HashMap<usize, u32> = HashMap::new();
    // Attributes already in the schema, by interned id.
    let mut in_schema: Vec<bool> = Vec::new();
    fn mark(in_schema: &mut Vec<bool>, id: u32) {
        let i = id as usize;
        if i >= in_schema.len() {
            in_schema.resize(i + 1, false);
        }
        in_schema[i] = true;
    }
    fn marked(in_schema: &[bool], id: u32) -> bool {
        in_schema.get(id as usize).copied().unwrap_or(false)
    }

    let mut a1: Option<String> = None;
    for &anchor in anchors {
        if let Some(a) = ws.assoc(ItemId::Cell(anchor)) {
            a1 = Some(a.to_string());
            break;
        }
    }
    let a1 = match a1 {
        Some(a) => a,
        None => {
            let a1 = anonymous_attribute(cfg, 1);
            let id = ws.intern_attr(&a1);
            for &anchor in anchors {
                if ws.val(sem, ItemId::Cell(anchor)).is_some() {
                    ws.set_avp_id(ItemId::Cell(anchor), id);
                }
            }
            a1
        }
    };
    mark(&mut in_schema, ws.intern_attr(&a1));
    schema_attrs.push(a1);

    visit_schema(cfg.strategy, anchors, ws, |ws, a, rec_idx, pos_idx| {
        let anchor = anchors[a];
        let item = match ws.rec(anchor).and_then(|rs| rs.get(rec_idx)) {
            Some(seq) if pos_idx < seq.len() => seq[pos_idx],
            _ => return,
        };
        match ws.attr_id(item) {
            Some(id) => {
                if !marked(&in_schema, id) {
                    mark(&mut in_schema, id);
                    schema_attrs.push(ws.attr_name(id).to_string());
                }
            }
            None => {
                let id = match anon_map.entry(pos_idx) {
                    std::collections::hash_map::Entry::Vacant(e) => {
                        let anon = anonymous_attribute(cfg, pos_idx + 1);
                        let id = ws.intern_attr(&anon);
                        e.insert(id);
                        schema_attrs.push(anon);
                        mark(&mut in_schema, id);
                        id
                    }
                    std::collections::hash_map::Entry::Occupied(e) => *e.get(),
                };
                if ws.val(sem, item).is_some() {
                    ws.set_avp_id(item, id);
                }
            }
        }
    });

    Schema::new(schema_attrs)
}

fn generate_records(
    cfg: &InterpreterCfg,
    ws: &WorkingState,
    sem: &SemanticsCore,
    anchors: &[usize],
    schema: Schema,
) -> CoreResult<RecordsetCore> {
    let n = schema.attributes.len();
    // Interned attribute id → position in the schema.
    let mut index: Vec<Option<usize>> = vec![None; ws.attr_count()];
    for (i, a) in schema.attributes.iter().enumerate() {
        if let Some(id) = ws.attr_id_of(a) {
            index[id as usize] = Some(i);
        }
    }
    // The missing values of a record depend on the schema only.
    let missing: Vec<Option<Text>> = match &cfg.missing_value_handler {
        None => vec![None; n],
        Some(_) => {
            let mut m = Vec::with_capacity(n);
            for attr in &schema.attributes {
                m.push(handle_missing(cfg, attr)?.map(Text::from));
            }
            m
        }
    };
    let mut rs = RecordsetCore::with_capacity(schema, anchors.len());
    for &anchor in anchors {
        let Some(recs) = ws.rec(anchor) else { continue };
        for sequence in recs.iter() {
            let r = rs.push_slice(&missing);
            let values = rs.record_mut(r);
            for &item in sequence {
                if let Some(id) = ws.attr_id(item) {
                    if let Some(idx) = index[id as usize] {
                        values[idx] = ws.val(sem, item).cloned();
                    }
                }
            }
        }
    }
    Ok(rs)
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
