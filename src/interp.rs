//! Port of `ru.icc.regtab.interpret.TableInterpreter`: 4 interpretation phases.
//!
//! Working state completion applies the actions in operation-type order
//! `FILL/PREFIX/SUFFIX → AVP → REC → CONCAT → JOIN`: records are folded by
//! concatenation before they are multiplied by joins. Provider evaluation and
//! all working-state mutations remain sequential. Recordset extraction may fill
//! disjoint output slices in parallel.
//! Preconditions violated by `CONCAT`/`JOIN` leave the working state unchanged
//! and are reported through [`Diagnostic`]s (or fail under strict preconditions).

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
    pub num_threads: usize,
    pub jobs_per_thread: usize,
    pub parallel_record_generation: bool,
    pub parallel_item_index: bool,
    pub parallel_consistency: bool,
    pub parallel_action_sorting: bool,
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
            num_threads: 1,
            jobs_per_thread: 4,
            parallel_record_generation: true,
            parallel_item_index: true,
            parallel_consistency: true,
            parallel_action_sorting: true,
        }
    }
}

struct ScopedExecutor<'scope, 'env> {
    scope: &'scope std::thread::Scope<'scope, 'env>,
    pool: Option<rayon::ThreadPool>,
    num_threads: usize,
    pool_creation: f64,
}

impl<'scope, 'env> ScopedExecutor<'scope, 'env> {
    fn new(scope: &'scope std::thread::Scope<'scope, 'env>, num_threads: usize) -> Self {
        ScopedExecutor { scope, pool: None, num_threads, pool_creation: 0.0 }
    }

    fn pool(&mut self) -> CoreResult<&rayon::ThreadPool> {
        if self.pool.is_none() {
            let started = std::time::Instant::now();
            let scope = self.scope;
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(self.num_threads)
                .spawn_handler(move |thread| {
                    std::thread::Builder::new().spawn_scoped(scope, || thread.run())?;
                    Ok(())
                })
                .build()
                .map_err(|e| format!("Cannot create interpretation thread pool: {e}"))?;
            self.pool_creation = started.elapsed().as_secs_f64();
            self.pool = Some(pool);
        }
        Ok(self.pool.as_ref().expect("pool initialized"))
    }
}

/// The recordset of an interpretation together with the diagnostics of its
/// working state completion (empty when no action was skipped).
pub struct Interpretation {
    pub recordset: RecordsetCore,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Debug, Default)]
pub struct InterpretProfile {
    pub phases: [f64; 4],
    pub pool_creation: f64,
    pub action_grouping: f64,
    pub action_sorting: f64,
    pub item_index: f64,
    pub provider_evaluation: f64,
    pub action_application: f64,
    pub consistency: f64,
    pub schema: f64,
    pub record_preparation: f64,
    pub record_generation: f64,
    pub record_work: f64,
    pub record_initialization: f64,
    pub record_fill: f64,
    pub actions: u64,
    pub provider_items: u64,
    pub records: u64,
    pub record_items: u64,
    pub jobs: u64,
    pub parallel_action_sorting: bool,
    pub parallel_item_index: bool,
    pub parallel_consistency: bool,
    detailed: bool,
}

impl InterpretProfile {
    fn new(detailed: bool) -> Self {
        InterpretProfile { detailed, ..Default::default() }
    }
}

fn lap(profile: &mut Option<&mut InterpretProfile>, clock: &mut std::time::Instant, phase: usize) {
    if let Some(p) = profile.as_deref_mut() {
        p.phases[phase] = clock.elapsed().as_secs_f64();
        *clock = std::time::Instant::now();
    }
}

pub fn interpret(
    cfg: &InterpreterCfg,
    syntax: &SyntaxCore,
    sem: &SemanticsCore,
    py_table: Option<&crate::spec::PyTableHandle>,
) -> CoreResult<Interpretation> {
    interpret_entry(cfg, syntax, sem, py_table, None)
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
    let mut profile = InterpretProfile::new(false);
    let out = interpret_entry(cfg, syntax, sem, py_table, Some(&mut profile))?;
    Ok((out, profile.phases))
}

/// [`interpret`] with detailed phase counters for performance experiments.
pub fn interpret_profile(
    cfg: &InterpreterCfg,
    syntax: &SyntaxCore,
    sem: &SemanticsCore,
    py_table: Option<&crate::spec::PyTableHandle>,
) -> CoreResult<(Interpretation, InterpretProfile)> {
    let mut profile = InterpretProfile::new(true);
    let out = interpret_entry(cfg, syntax, sem, py_table, Some(&mut profile))?;
    Ok((out, profile))
}

fn interpret_entry(
    cfg: &InterpreterCfg,
    syntax: &SyntaxCore,
    sem: &SemanticsCore,
    py_table: Option<&crate::spec::PyTableHandle>,
    profile: Option<&mut InterpretProfile>,
) -> CoreResult<Interpretation> {
    if cfg.num_threads == 0 || cfg.jobs_per_thread == 0 {
        return Err("num_threads and jobs_per_thread must be positive".into());
    }
    if cfg.num_threads == 1 {
        return interpret_impl(cfg, syntax, sem, py_table, None, profile);
    }
    std::thread::scope(|scope| {
        let mut executor = ScopedExecutor::new(scope, cfg.num_threads);
        let result = interpret_impl(cfg, syntax, sem, py_table, Some(&mut executor), profile);
        drop(executor);
        result
    })
}

fn interpret_impl(
    cfg: &InterpreterCfg,
    syntax: &SyntaxCore,
    sem: &SemanticsCore,
    py_table: Option<&crate::spec::PyTableHandle>,
    mut executor: Option<&mut ScopedExecutor>,
    mut profile: Option<&mut InterpretProfile>,
) -> CoreResult<Interpretation> {
    let env = EvalEnv { syntax, py_table };
    let mut clock = std::time::Instant::now();

    // Phase 1: working state initialization — val(ι) = s(ι) for value items
    // and attr(ι) = s(ι) for attribute items hold by default (the state
    // stores overrides only), so only the storage is sized here.
    let mut ws = WorkingState::new(cfg.strict_preconditions);
    ws.reserve(sem.cell_items.len(), sem.ctx_items.len());
    lap(&mut profile, &mut clock, 0);

    // Phase 2: working state completion
    complete_working_state(cfg, &mut ws, sem, &env, executor.as_deref_mut(), profile.as_deref_mut())?;
    let diagnostics = ws.take_diagnostics();
    lap(&mut profile, &mut clock, 1);

    // Phase 3: recordset extraction
    let recordset = extract_recordset(cfg, &mut ws, sem, executor.as_deref_mut(), profile.as_deref_mut())?;
    if let Some(p) = profile.as_deref_mut() {
        p.records = recordset.len() as u64;
        p.pool_creation = executor.as_ref().map_or(0.0, |e| e.pool_creation);
    }
    lap(&mut profile, &mut clock, 2);

    // Phase 4: recordset transformation
    let mut rs = recordset;
    drop(ws);
    for t in &cfg.transformations {
        rs = t.with_template(&cfg.anonymous_attribute_template).apply(rs)?;
    }
    lap(&mut profile, &mut clock, 3);
    Ok(Interpretation { recordset: rs, diagnostics })
}

fn anchor_pos(sem: &SemanticsCore, action: &ActionInst) -> Option<(usize, usize)> {
    match action.anchor() {
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

fn apply_sequential_chunk(
    ws: &mut WorkingState,
    sem: &SemanticsCore,
    env: &EvalEnv,
    index: &ItemIndex,
    group: &[u32],
    items: &mut Vec<ItemId>,
    mut profile: Option<&mut InterpretProfile>,
) -> CoreResult<()> {
    let detailed = profile.as_ref().is_some_and(|p| p.detailed);
    for &i in group {
        let action = &sem.actions[i as usize];
        if detailed {
            let p = profile.as_deref_mut().unwrap();
            let mut clock = std::time::Instant::now();
            let result = evaluate_action_items(sem, env, index, action, items);
            p.provider_evaluation += clock.elapsed().as_secs_f64();
            p.provider_items += items.len() as u64;
            result?;
            clock = std::time::Instant::now();
            let result = apply_action_items(ws, sem, action, items);
            p.action_application += clock.elapsed().as_secs_f64();
            p.actions += 1;
            result?;
        } else {
            evaluate_action_items(sem, env, index, action, items)?;
            apply_action_items(ws, sem, action, items)?;
        }
    }
    Ok(())
}

fn complete_working_state(
    cfg: &InterpreterCfg,
    ws: &mut WorkingState,
    sem: &SemanticsCore,
    env: &EvalEnv,
    mut executor: Option<&mut ScopedExecutor>,
    mut profile: Option<&mut InterpretProfile>,
) -> CoreResult<()> {
    let detailed = profile.as_ref().is_some_and(|p| p.detailed);
    let mut clock = std::time::Instant::now();

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
    if detailed {
        profile.as_deref_mut().unwrap().action_grouping = clock.elapsed().as_secs_f64();
        clock = std::time::Instant::now();
    }
    if cfg.parallel_action_sorting && sem.actions.len() >= PARALLEL_STRUCTURE_MIN && executor.is_some() {
        let pool = match executor.as_mut() {
                Some(executor) => executor.pool()?,
                None => unreachable!("parallel path requires an executor"),
            };
        pool.install(|| {
            use rayon::prelude::*;
            groups.par_iter_mut().for_each(|group| sort_actions(cfg, sem, group));
        });
    } else {
        for group in groups.iter_mut() {
            sort_actions(cfg, sem, group);
        }
    }
    if detailed {
        let p = profile.as_deref_mut().unwrap();
        p.action_sorting = clock.elapsed().as_secs_f64();
        p.parallel_action_sorting = cfg.parallel_action_sorting && sem.actions.len() >= PARALLEL_STRUCTURE_MIN && executor.is_some();
        clock = std::time::Instant::now();
    }

    // One spatial index per interpretation, shared by all providers.
    let index = if cfg.parallel_item_index && sem.cell_items.len() >= PARALLEL_STRUCTURE_MIN && executor.is_some() {
        let pool = match executor.as_mut() {
                Some(executor) => executor.pool()?,
                None => unreachable!("parallel path requires an executor"),
            };
        pool.install(|| ItemIndex::build_parallel(sem, env.syntax))
    } else {
        ItemIndex::build(sem, env.syntax)
    };
    if detailed {
        let p = profile.as_deref_mut().unwrap();
        p.item_index = clock.elapsed().as_secs_f64();
        p.parallel_item_index = cfg.parallel_item_index && sem.cell_items.len() >= PARALLEL_STRUCTURE_MIN && executor.is_some();
    }

    let mut items: Vec<ItemId> = Vec::new();
    for group in groups.iter() {
        apply_sequential_chunk(ws, sem, env, &index, group, &mut items, profile.as_deref_mut())?;
    }
    Ok(())
}

fn evaluate_action_items(
    sem: &SemanticsCore,
    env: &EvalEnv,
    index: &ItemIndex,
    action: &ActionInst,
    items: &mut Vec<ItemId>,
) -> CoreResult<()> {
    let anchor = action.anchor();
    items.clear();
    for provider in action.providers() {
        provider.provide_into(anchor, sem, env, index, items)?;
    }
    Ok(())
}

fn apply_action_items(ws: &mut WorkingState, sem: &SemanticsCore, action: &ActionInst, items: &[ItemId]) -> CoreResult<()> {
    let anchor = action.anchor();
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
    let ItemId::Cell(anchor) = action.anchor() else {
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

fn records_attributes_distinct_parallel(ws: &WorkingState, anchors: &[usize], pool: &rayon::ThreadPool) -> bool {
    pool.install(|| {
        use rayon::prelude::*;
        anchors
            .par_iter()
            .map_init(Vec::<u32>::new, |seen, &anchor| {
                let Some(recs) = ws.rec(anchor) else { return true };
                for sequence in recs.iter() {
                    seen.clear();
                    for &item in sequence {
                        if let Some(attr) = ws.attr_id(item) {
                            if seen.contains(&attr) {
                                return false;
                            }
                            seen.push(attr);
                        }
                    }
                }
                true
            })
            .all(|valid| valid)
    })
}

fn extract_recordset(
    cfg: &InterpreterCfg,
    ws: &mut WorkingState,
    sem: &SemanticsCore,
    mut executor: Option<&mut ScopedExecutor>,
    mut profile: Option<&mut InterpretProfile>,
) -> CoreResult<RecordsetCore> {
    let detailed = profile.as_ref().is_some_and(|p| p.detailed);
    let mut clock = std::time::Instant::now();
    let anchors: Vec<usize> = ws.live_anchors();
    let consistent = ws.is_anchor_attribute_uniform()
        && if cfg.parallel_consistency && anchors.len() >= PARALLEL_STRUCTURE_MIN && executor.is_some() {
            let pool = match executor.as_mut() {
                Some(executor) => executor.pool()?,
                None => unreachable!("parallel path requires an executor"),
            };
            records_attributes_distinct_parallel(ws, &anchors, pool)
        } else {
            ws.is_record_attributes_distinct()
        };
    if detailed {
        let p = profile.as_deref_mut().unwrap();
        p.consistency = clock.elapsed().as_secs_f64();
        p.parallel_consistency = cfg.parallel_consistency && anchors.len() >= PARALLEL_STRUCTURE_MIN && executor.is_some();
        clock = std::time::Instant::now();
    }
    if !consistent {
        return Err("Working state is not recordset-consistent".into());
    }
    if detailed {
        profile.as_deref_mut().unwrap().record_preparation = clock.elapsed().as_secs_f64();
        clock = std::time::Instant::now();
    }
    let schema = construct_schema(cfg, ws, sem, &anchors)?;
    if detailed {
        profile.as_deref_mut().unwrap().schema = clock.elapsed().as_secs_f64();
        clock = std::time::Instant::now();
    }
    let rs = generate_records(cfg, ws, sem, &anchors, schema, executor, profile.as_deref_mut())?;
    if detailed {
        profile.as_mut().unwrap().record_generation = clock.elapsed().as_secs_f64();
    }
    Ok(rs)
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

const PARALLEL_STRUCTURE_MIN: usize = 262_144;
const PARALLEL_RECORD_MIN: usize = 262_144;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RecordCursor {
    anchor: usize,
    record: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct RecordJob {
    start: RecordCursor,
    end: RecordCursor,
    records: usize,
}

#[derive(Debug, Default, PartialEq, Eq)]
struct RecordWork {
    records: usize,
    item_ids: u64,
    weight: u64,
}

fn measure_records(ws: &WorkingState, anchors: &[usize], width: usize) -> CoreResult<RecordWork> {
    let mut work = RecordWork::default();
    for &anchor in anchors {
        let Some(recs) = ws.rec(anchor) else { continue };
        work.records = work.records.checked_add(recs.len()).ok_or("Record count overflow")?;
        for sequence in recs.iter() {
            work.item_ids = work.item_ids.saturating_add(sequence.len() as u64);
            work.weight = work.weight.saturating_add(sequence.len() as u64 + width as u64);
        }
    }
    Ok(work)
}

fn build_record_jobs(
    ws: &WorkingState,
    anchors: &[usize],
    width: usize,
    work: &RecordWork,
    num_threads: usize,
    jobs_per_thread: usize,
) -> Vec<RecordJob> {
    let target_jobs = work.records.min(num_threads.saturating_mul(jobs_per_thread)).max(1);
    let target_weight = work.weight.div_ceil(target_jobs as u64).max(1);
    let mut jobs = Vec::with_capacity(target_jobs);
    let mut start = RecordCursor { anchor: 0, record: 0 };
    let mut records = 0usize;
    let mut weight = 0u64;
    for (a, &anchor) in anchors.iter().enumerate() {
        let Some(recs) = ws.rec(anchor) else { continue };
        for (r, sequence) in recs.iter().enumerate() {
            records += 1;
            weight = weight.saturating_add(sequence.len() as u64 + width as u64);
            let next = if r + 1 < recs.len() {
                RecordCursor { anchor: a, record: r + 1 }
            } else {
                RecordCursor { anchor: a + 1, record: 0 }
            };
            if weight >= target_weight && jobs.len() + 1 < target_jobs {
                jobs.push(RecordJob { start, end: next, records });
                start = next;
                records = 0;
                weight = 0;
            }
        }
    }
    if records > 0 {
        jobs.push(RecordJob { start, end: RecordCursor { anchor: anchors.len(), record: 0 }, records });
    }
    jobs
}

fn fill_record_job(
    ws: &WorkingState,
    sem: &SemanticsCore,
    anchors: &[usize],
    index: &[Option<usize>],
    missing: &[Option<Text>],
    job: RecordJob,
    values: &mut [Option<Text>],
) {
    let width = missing.len();
    let has_missing = missing.iter().any(Option::is_some);
    let mut seen = vec![false; width];
    let mut touched = Vec::new();
    let mut cursor = job.start;
    let mut out = 0usize;
    while cursor.anchor < job.end.anchor || (cursor.anchor == job.end.anchor && cursor.record < job.end.record) {
        let recs = ws.rec(anchors[cursor.anchor]).expect("live anchor has records");
        let sequence = recs.get(cursor.record).expect("job record is in range");
        let row = &mut values[out..out + width];
        for &item in sequence {
            if let Some(id) = ws.attr_id(item) {
                if let Some(idx) = index[id as usize] {
                    if !seen[idx] {
                        seen[idx] = true;
                        touched.push(idx);
                    }
                    row[idx] = ws.val(sem, item).cloned();
                }
            }
        }
        if has_missing {
            for idx in 0..width {
                if !seen[idx] {
                    row[idx].clone_from(&missing[idx]);
                }
            }
        }
        for idx in touched.drain(..) {
            seen[idx] = false;
        }
        out += width;
        cursor = if cursor.record + 1 < recs.len() {
            RecordCursor { anchor: cursor.anchor, record: cursor.record + 1 }
        } else {
            RecordCursor { anchor: cursor.anchor + 1, record: 0 }
        };
    }
    debug_assert_eq!(out, values.len());
}

fn generate_records(
    cfg: &InterpreterCfg,
    ws: &WorkingState,
    sem: &SemanticsCore,
    anchors: &[usize],
    schema: Schema,
    executor: Option<&mut ScopedExecutor>,
    mut profile: Option<&mut InterpretProfile>,
) -> CoreResult<RecordsetCore> {
    let width = schema.attributes.len();
    // Interned attribute id → position in the schema.
    let mut index: Vec<Option<usize>> = vec![None; ws.attr_count()];
    for (i, a) in schema.attributes.iter().enumerate() {
        if let Some(id) = ws.attr_id_of(a) {
            index[id as usize] = Some(i);
        }
    }
    // The missing values of a record depend on the schema only.
    let missing: Vec<Option<Text>> = match &cfg.missing_value_handler {
        None => vec![None; width],
        Some(_) => {
            let mut m = Vec::with_capacity(width);
            for attr in &schema.attributes {
                m.push(handle_missing(cfg, attr)?.map(Text::from));
            }
            m
        }
    };
    let detailed = profile.as_ref().is_some_and(|p| p.detailed);
    if cfg.parallel_record_generation && width > 0 && executor.is_some() {
        let started = std::time::Instant::now();
        let work = measure_records(ws, anchors, width)?;
        let total_values = work.records.checked_mul(width).ok_or("Recordset size overflow")?;
        if work.records >= PARALLEL_RECORD_MIN {
            let jobs = build_record_jobs(ws, anchors, width, &work, cfg.num_threads, cfg.jobs_per_thread);
            if detailed {
                let p = profile.as_deref_mut().unwrap();
                p.record_work = started.elapsed().as_secs_f64();
                p.record_items = work.item_ids;
                p.jobs = jobs.len() as u64;
            }
            if let Some(executor) = executor {
                let pool = executor.pool()?;
                let started = std::time::Instant::now();
                let mut values = vec![None; total_values];
                let mut slices = Vec::with_capacity(jobs.len());
                let mut rest = values.as_mut_slice();
                for job in jobs.iter().copied() {
                    let (head, tail) = rest.split_at_mut(job.records * width);
                    slices.push((job, head));
                    rest = tail;
                }
                if detailed {
                    profile.as_deref_mut().unwrap().record_initialization = started.elapsed().as_secs_f64();
                }
                let started = std::time::Instant::now();
                pool.install(|| {
                    use rayon::prelude::*;
                    slices.into_par_iter().for_each(|(job, values)| {
                        fill_record_job(ws, sem, anchors, &index, &missing, job, values);
                    });
                });
                if detailed {
                    profile.as_mut().unwrap().record_fill = started.elapsed().as_secs_f64();
                }
                return RecordsetCore::from_flat_values(schema, work.records, values);
            }
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matcher::match_atp;
    use crate::rtl::{compile, BindingsCore};
    use crate::semantics::{CellItem, CtxItem};
    use crate::spec::{ItemType, RecordKey};

    fn semantics_with_cells(n: usize) -> SemanticsCore {
        SemanticsCore {
            cell_items: (0..n)
                .map(|i| CellItem {
                    s: Text::from(format!("v{i}")),
                    tags: Vec::new(),
                    index: 0,
                    row: i,
                    col: 0,
                    ty: ItemType::Value,
                    span: (0, 1),
                })
                .collect(),
            ctx_items: vec![CtxItem { s: Text::from("a"), ty: ItemType::Attribute, const_value: None }],
            actions: Vec::new(),
            action_templates: HashMap::new(),
        }
    }

    #[test]
    fn record_jobs_split_a_joined_anchor_without_gaps() {
        let mut sem = semantics_with_cells(5);
        let mut ws = WorkingState::new(false);
        ws.reserve(sem.cell_items.len(), sem.ctx_items.len());
        for anchor in 0..5 {
            ws.apply_rec(&sem, ItemId::Cell(anchor), &[]).unwrap();
        }
        ws.apply_join(&sem, ItemId::Cell(1), &[ItemId::Cell(2), ItemId::Cell(3)], &RecordKey::empty()).unwrap();
        ws.apply_join(&sem, ItemId::Cell(0), &[ItemId::Cell(1)], &RecordKey::empty()).unwrap();
        sem.actions.clear();
        let anchors = ws.live_anchors();
        let work = measure_records(&ws, &anchors, 2).unwrap();
        let jobs = build_record_jobs(&ws, &anchors, 2, &work, 4, 4);

        assert_eq!(
            work.records,
            3,
            "anchors={anchors:?} rec0={:?} rec1={:?} rec2={:?} rec3={:?}",
            ws.rec(0).map(|r| r.len()),
            ws.rec(1).map(|r| r.len()),
            ws.rec(2).map(|r| r.len()),
            ws.rec(3).map(|r| r.len()),
        );
        assert_eq!(jobs.len(), 3);
        assert_eq!(jobs.iter().map(|job| job.records).sum::<usize>(), work.records);
        assert_eq!(jobs[0].start, RecordCursor { anchor: 0, record: 0 });
        assert_eq!(jobs[0].end, jobs[1].start);
        assert_eq!(jobs[1].end, jobs[2].start);
        assert_eq!(jobs[0].start.anchor, 0);
        assert_eq!(jobs[1].start.anchor, 0, "the joined anchor is split between jobs");
        assert_eq!(jobs.last().unwrap().end, RecordCursor { anchor: anchors.len(), record: 0 });
    }

    #[test]
    fn flat_recordset_constructor_validates_size_and_zero_width() {
        let empty = RecordsetCore::from_flat_values(Schema::new(Vec::new()).unwrap(), 3, Vec::new()).unwrap();
        assert_eq!(empty.len(), 3);
        assert_eq!(empty.width(), 0);
        assert!(empty.record(2).is_empty());

        let schema = Schema::new(vec!["a".into()]).unwrap();
        assert!(RecordsetCore::from_flat_values(schema.clone(), 2, vec![None]).is_err());
        assert!(RecordsetCore::from_flat_values(schema, usize::MAX, Vec::new()).is_err());
    }

    #[test]
    fn join_products_compose_before_extraction() {
        let n = 10;
        let sem = semantics_with_cells(3 + 2 * n);
        let mut ws = WorkingState::new(false);
        ws.reserve(sem.cell_items.len(), sem.ctx_items.len());
        for anchor in 0..3 + 2 * n {
            ws.apply_rec(&sem, ItemId::Cell(anchor), &[]).unwrap();
        }
        let left: Vec<ItemId> = (3..n + 3).map(ItemId::Cell).collect();
        let right: Vec<ItemId> = (n + 3..2 * n + 3).map(ItemId::Cell).collect();
        ws.apply_join(&sem, ItemId::Cell(0), &left, &RecordKey::empty()).unwrap();
        ws.apply_join(&sem, ItemId::Cell(1), &right, &RecordKey::empty()).unwrap();
        ws.apply_join(&sem, ItemId::Cell(2), &[ItemId::Cell(0)], &RecordKey::empty()).unwrap();
        ws.apply_join(&sem, ItemId::Cell(2), &[ItemId::Cell(1)], &RecordKey::empty()).unwrap();
        assert_eq!(ws.rec(0).map(|recs| recs.len()), Some(n));
        assert_eq!(ws.rec(1).map(|recs| recs.len()), Some(n));
        assert_eq!(ws.rec(2).map(|recs| recs.len()), Some(n * n));
        assert_eq!(ws.live_anchors(), vec![2]);
    }

    #[test]
    fn missing_values_do_not_overwrite_an_explicit_none() {
        let mut sem = semantics_with_cells(2);
        sem.cell_items[1].ty = ItemType::Auxiliary;
        let mut ws = WorkingState::new(false);
        ws.reserve(sem.cell_items.len(), sem.ctx_items.len());
        ws.apply_rec(&sem, ItemId::Cell(0), &[ItemId::Cell(1)]).unwrap();
        ws.set_avp(ItemId::Cell(0), "a");
        let b = ws.intern_attr("b");
        ws.set_avp_id(ItemId::Cell(1), b);
        let anchors = ws.live_anchors();
        let missing = vec![Some(Text::from("missing-a")), Some(Text::from("missing-b")), Some(Text::from("missing-c"))];
        let mut values = vec![None; 3];
        fill_record_job(
            &ws,
            &sem,
            &anchors,
            &[Some(0), Some(1)],
            &missing,
            RecordJob {
                start: RecordCursor { anchor: 0, record: 0 },
                end: RecordCursor { anchor: 1, record: 0 },
                records: 1,
            },
            &mut values,
        );
        assert_eq!(values[0].as_deref(), Some("v0"));
        assert!(values[1].is_none(), "an explicit None is not a missing field");
        assert_eq!(values[2].as_deref(), Some("missing-c"));
    }

    #[test]
    fn all_parallel_paths_match_sequential_and_are_exercised() {
        let rows: Vec<Vec<String>> = (0..270_000).map(|i| vec![format!("v{i}")]).collect();
        let pattern = compile("[ [VAL: 'v'->AVP, ()->REC] ]+", &BindingsCore::default()).unwrap();
        let mut sequential_syntax = SyntaxCore::from_rows(rows, None).unwrap();
        let mut parallel_syntax = sequential_syntax.clone();
        let sequential_sem = match_atp(&pattern, &mut sequential_syntax, Vec::new()).unwrap().unwrap();
        let parallel_sem = match_atp(&pattern, &mut parallel_syntax, Vec::new()).unwrap().unwrap();

        let sequential = interpret(&InterpreterCfg::default(), &sequential_syntax, &sequential_sem, None).unwrap();
        let cfg = InterpreterCfg { num_threads: 4, ..Default::default() };
        let (parallel, profile) = interpret_profile(&cfg, &parallel_syntax, &parallel_sem, None).unwrap();

        assert_eq!(sequential.recordset, parallel.recordset);
        assert_eq!(sequential.diagnostics, parallel.diagnostics);
        assert!(profile.parallel_action_sorting);
        assert!(profile.parallel_item_index);
        assert!(profile.parallel_consistency);
        assert!(profile.jobs > 1);
    }
}
