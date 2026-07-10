//! Port of `ru.icc.regtab.interpret.TableInterpreter`: 4 interpretation phases.

use crate::recordset::{RecordCore, RecordsetCore, Schema};
use crate::semantics::{ActionInst, ItemId, OpInst, SemanticsCore, WorkingState};
use crate::spec::{EvalEnv, ItemType, PyFunc, Transformation};
use crate::syntax::SyntaxCore;
use crate::util::CoreResult;
use std::collections::HashMap;

#[pyo3::pyclass(eq, eq_int, name = "SchemaConstructionStrategy")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum SchemaStrategy {
    #[pyo3(name = "RECORD_FIRST")]
    RecordFirst,
    #[pyo3(name = "POSITION_FIRST")]
    PositionFirst,
}

#[pyo3::pyclass(eq, eq_int, name = "ActionApplicationStrategy")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActionStrategy {
    #[pyo3(name = "ROW_FIRST")]
    RowFirst,
    #[pyo3(name = "COLUMN_FIRST")]
    ColumnFirst,
}

pub struct InterpreterCfg {
    pub strategy: SchemaStrategy,
    pub action_strategy: ActionStrategy,
    pub missing_value_handler: Option<PyFunc>,
    pub transformations: Vec<Transformation>,
    pub anonymous_attribute_template: String,
}

impl Default for InterpreterCfg {
    fn default() -> Self {
        InterpreterCfg {
            strategy: SchemaStrategy::RecordFirst,
            action_strategy: ActionStrategy::RowFirst,
            missing_value_handler: None,
            transformations: Vec::new(),
            anonymous_attribute_template: "$a_%i".to_string(),
        }
    }
}

pub fn interpret(
    cfg: &InterpreterCfg,
    syntax: &SyntaxCore,
    sem: &SemanticsCore,
    py_table: Option<&pyo3::Py<pyo3::PyAny>>,
) -> CoreResult<RecordsetCore> {
    let env = EvalEnv { syntax, py_table };

    // Phase 1: working state initialization
    let mut ws = WorkingState::default();
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

    // Phase 3: recordset extraction
    let recordset = extract_recordset(cfg, &mut ws, sem)?;

    // Phase 4: recordset transformation
    let mut rs = recordset;
    for t in &cfg.transformations {
        rs = t.with_template(&cfg.anonymous_attribute_template).apply(rs)?;
    }
    Ok(rs)
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

fn sort_actions(cfg: &InterpreterCfg, sem: &SemanticsCore, actions: &mut Vec<&ActionInst>) {
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
    let mut join_actions: Vec<&ActionInst> = Vec::new();

    for action in &sem.actions {
        match action.op {
            OpInst::Fill(_) | OpInst::Prefix(_) | OpInst::Suffix(_) => str_actions.push(action),
            OpInst::Avp => avp_actions.push(action),
            OpInst::Rec => rec_actions.push(action),
            OpInst::Join(_) => join_actions.push(action),
        }
    }

    sort_actions(cfg, sem, &mut str_actions);
    sort_actions(cfg, sem, &mut avp_actions);
    sort_actions(cfg, sem, &mut rec_actions);
    sort_actions(cfg, sem, &mut join_actions);

    for group in [str_actions, avp_actions, rec_actions, join_actions] {
        for action in group {
            apply_action(ws, sem, env, action)?;
        }
    }
    Ok(())
}

fn apply_action(
    ws: &mut WorkingState,
    sem: &SemanticsCore,
    env: &EvalEnv,
    action: &ActionInst,
) -> CoreResult<()> {
    let anchor = action.anchor;
    let mut items: Vec<ItemId> = Vec::new();
    for provider in &action.providers {
        items.extend(provider.provide(anchor, sem, env)?);
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
        OpInst::Join(kp) => {
            if !items.is_empty() {
                ws.apply_join(anchor, &items, kp)
            } else {
                Ok(())
            }
        }
    }
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

fn construct_schema(cfg: &InterpreterCfg, ws: &mut WorkingState) -> CoreResult<Schema> {
    let anchors: Vec<usize> = ws.rec.keys().copied().collect();

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

    // visit order of (anchor_index_in_list, position) pairs
    let pairs: Vec<(usize, usize)> = match cfg.strategy {
        SchemaStrategy::RecordFirst => {
            let mut out = Vec::new();
            for (a, &anchor) in anchors.iter().enumerate() {
                let len = ws.rec.get(&anchor).map(|s| s.len()).unwrap_or(0);
                for i in 1..len {
                    out.push((a, i));
                }
            }
            out
        }
        SchemaStrategy::PositionFirst => {
            let mut max_len = 0;
            for &anchor in &anchors {
                max_len = max_len.max(ws.rec.get(&anchor).map(|s| s.len()).unwrap_or(0));
            }
            let mut out = Vec::new();
            for i in 1..max_len {
                for a in 0..anchors.len() {
                    out.push((a, i));
                }
            }
            out
        }
    };

    let mut in_schema: Vec<String> = schema_attrs.clone();

    for (a, pos_idx) in pairs {
        let anchor = anchors[a];
        let sequence = ws.rec.get(&anchor).cloned().unwrap_or_default();
        if pos_idx >= sequence.len() {
            continue;
        }
        let item = sequence[pos_idx];
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
    let mut records = Vec::with_capacity(ws.rec.len());
    for sequence in ws.rec.values() {
        let mut values: Vec<Option<String>> = Vec::with_capacity(n);
        for attr in &schema.attributes {
            values.push(handle_missing(cfg, attr)?);
        }
        for &item in sequence {
            if let Some(a) = ws.assoc(item) {
                if let Some(idx) = schema.index_of(a) {
                    values[idx] = ws.val.get(&item).cloned();
                }
            }
        }
        records.push(RecordCore { values });
    }
    Ok(records)
}

fn handle_missing(cfg: &InterpreterCfg, attribute: &str) -> CoreResult<Option<String>> {
    match &cfg.missing_value_handler {
        None => Ok(None),
        Some(f) => crate::py::call_missing_handler(f, attribute),
    }
}
