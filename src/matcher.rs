//! Port of `ru.icc.regtab.atp.match` + `AtpMatcher`:
//! backtracking syntactic matching and semantic layer construction.

use crate::semantics::{ActionInst, CellItem, CtxItem, ItemId, OpInst, ProviderInst, SemanticsCore};
use crate::spec::*;
use crate::syntax::SyntaxCore;
use crate::util::{split_literal, CoreErr, CoreResult};
use std::sync::Arc;

// ---------------------------------------------------------------- match state

#[derive(Default)]
struct MatchState {
    pairs: Vec<(Arc<CellPattern>, (usize, usize))>,
    subtables: Vec<(usize, usize)>,          // (row_start, row_end)
    subrows: Vec<(usize, usize, usize)>,     // (row, col_start, col_end)
}

#[derive(Clone, Copy)]
struct Snapshot {
    pairs: usize,
    subtables: usize,
    subrows: usize,
}

impl MatchState {
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            pairs: self.pairs.len(),
            subtables: self.subtables.len(),
            subrows: self.subrows.len(),
        }
    }
    fn restore(&mut self, s: Snapshot) {
        self.pairs.truncate(s.pairs);
        self.subtables.truncate(s.subtables);
        self.subrows.truncate(s.subrows);
    }
}

/// Success => Some(next_index); failure => None.
type Outcome = Option<usize>;

// ---------------------------------------------------------------- generic algorithm

/// Port of the generic `matchPatterns` (backtracking over a pattern list).
fn match_patterns<P>(
    patterns: &[P],
    n_elements: usize,
    element_index: usize,
    state: &mut MatchState,
    quantifier_of: &dyn Fn(&P) -> Quantifier,
    dispatch: &mut dyn FnMut(&P, usize, &mut MatchState) -> CoreResult<Outcome>,
) -> CoreResult<Outcome> {
    let mut i = element_index;
    let n = n_elements;

    for j in 0..patterns.len() {
        let pattern = &patterns[j];
        let q = quantifier_of(pattern);
        let min = q.min();
        let max = q.max();
        let mut stack: Vec<(usize, Snapshot)> = Vec::new();

        while (stack.len() as i64) < max && i < n {
            let saved = state.snapshot();
            let dispatched = dispatch(pattern, i, state)?;
            match dispatched {
                Some(next) => {
                    stack.push((i, saved));
                    i = next;
                }
                None => {
                    state.restore(saved);
                    break;
                }
            }
        }

        if (stack.len() as i64) < min {
            return Ok(None);
        }

        if j < patterns.len() - 1 && !stack.is_empty() {
            loop {
                let next = match_patterns(
                    &patterns[j + 1..],
                    n_elements,
                    i,
                    state,
                    quantifier_of,
                    dispatch,
                )?;
                if next.is_some() {
                    return Ok(next);
                }
                if stack.len() as i64 <= min {
                    return Ok(None);
                }
                let (released_i, snap) = stack.pop().unwrap();
                i = released_i;
                state.restore(snap);
            }
        }
    }

    Ok(Some(i))
}

// ---------------------------------------------------------------- condition helpers

fn row_satisfies(
    syntax: &SyntaxCore,
    env: &EvalEnv,
    row: usize,
    cond: Option<&CellPredicate>,
) -> CoreResult<bool> {
    let Some(cond) = cond else { return Ok(true) };
    for (r, c) in syntax.cells_of_row(row) {
        if !cond.test(syntax.cell(r, c), env)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn rows_satisfy(
    syntax: &SyntaxCore,
    env: &EvalEnv,
    from: usize,
    to: usize,
    cond: Option<&CellPredicate>,
) -> CoreResult<bool> {
    if cond.is_none() {
        return Ok(true);
    }
    for r in from..to {
        if !row_satisfies(syntax, env, r, cond)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn cells_satisfy(
    syntax: &SyntaxCore,
    env: &EvalEnv,
    cells: &[(usize, usize)],
    from: usize,
    to: usize,
    cond: Option<&CellPredicate>,
) -> CoreResult<bool> {
    let Some(cond) = cond else { return Ok(true) };
    for &(r, c) in &cells[from..to] {
        if !cond.test(syntax.cell(r, c), env)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn resolve_idd(
    spec: &ContentSpec,
    syntax: &SyntaxCore,
    env: &EvalEnv,
    row: usize,
    col: usize,
) -> CoreResult<Idd> {
    match spec {
        ContentSpec::Atomic(a) => Ok(a.idd),
        ContentSpec::Delimited(d) => Ok(d.atom.idd),
        ContentSpec::Compound(_) => Ok(Idd::Val),
        ContentSpec::Conditional(c) => {
            let branch = if c.condition.test(syntax.cell(row, col), env)? {
                &c.positive
            } else {
                &c.negative
            };
            resolve_idd(branch, syntax, env, row, col)
        }
    }
}

// ---------------------------------------------------------------- dispatchers

fn dispatch_cell(
    pattern: &Arc<CellPattern>,
    cells: &[(usize, usize)],
    cell_index: usize,
    syntax: &SyntaxCore,
    env: &EvalEnv,
    state: &mut MatchState,
) -> CoreResult<Outcome> {
    if cell_index >= cells.len() {
        return Ok(None);
    }
    let (r, c) = cells[cell_index];
    if let Some(cond) = &pattern.condition {
        if !cond.test(syntax.cell(r, c), env)? {
            return Ok(None);
        }
    }
    if let Some(cs) = &pattern.content_spec {
        if resolve_idd(cs, syntax, env, r, c)? != Idd::Skip {
            state.pairs.push((pattern.clone(), (r, c)));
        }
    }
    Ok(Some(cell_index + 1))
}

fn dispatch_subrow(
    pattern: &SubrowPattern,
    cells: &[(usize, usize)],
    cell_index: usize,
    row_index: usize,
    syntax: &SyntaxCore,
    env: &EvalEnv,
    state: &mut MatchState,
) -> CoreResult<Outcome> {
    let saved = state.snapshot();
    let inner = match_patterns(
        &pattern.cell_patterns,
        cells.len(),
        cell_index,
        state,
        &|p: &Arc<CellPattern>| p.quantifier,
        &mut |p, i, st| dispatch_cell(p, cells, i, syntax, env, st),
    )?;
    let Some(next) = inner else {
        state.restore(saved);
        return Ok(None);
    };
    if !cells_satisfy(syntax, env, cells, cell_index, next, pattern.condition.as_ref())? {
        state.restore(saved);
        return Ok(None);
    }
    state.subrows.push((
        row_index,
        cells[cell_index].1,
        cells[next - 1].1,
    ));
    Ok(Some(next))
}

fn dispatch_row(
    pattern: &RowPattern,
    row_index: usize,
    syntax: &SyntaxCore,
    env: &EvalEnv,
    state: &mut MatchState,
) -> CoreResult<Outcome> {
    if row_index >= syntax.num_rows {
        return Ok(None);
    }
    if !row_satisfies(syntax, env, row_index, pattern.condition.as_ref())? {
        return Ok(None);
    }
    let cells = syntax.cells_of_row(row_index);
    let saved = state.snapshot();
    let inner = match_patterns(
        &pattern.subrow_patterns,
        cells.len(),
        0,
        state,
        &|p: &Arc<SubrowPattern>| p.quantifier,
        &mut |p, i, st| dispatch_subrow(p, &cells, i, row_index, syntax, env, st),
    )?;
    match inner {
        Some(next) if next == cells.len() => Ok(Some(row_index + 1)),
        _ => {
            state.restore(saved);
            Ok(None)
        }
    }
}

fn dispatch_subtable(
    pattern: &SubtablePattern,
    row_index: usize,
    syntax: &SyntaxCore,
    env: &EvalEnv,
    state: &mut MatchState,
) -> CoreResult<Outcome> {
    let saved = state.snapshot();
    let inner = match_patterns(
        &pattern.row_patterns,
        syntax.num_rows,
        row_index,
        state,
        &|p: &Arc<RowPattern>| p.quantifier,
        &mut |p, i, st| dispatch_row(p, i, syntax, env, st),
    )?;
    let Some(next) = inner else {
        state.restore(saved);
        return Ok(None);
    };
    if !rows_satisfy(syntax, env, row_index, next, pattern.condition.as_ref())? {
        state.restore(saved);
        return Ok(None);
    }
    state.subtables.push((row_index, next - 1));
    Ok(Some(next))
}

// ---------------------------------------------------------------- SyntaxMatcher

pub struct SyntaxMatch {
    pub pairs: Vec<(Arc<CellPattern>, (usize, usize))>,
    pub subtables: Vec<(usize, usize)>,
    pub subrows: Vec<(usize, usize, usize)>,
}

pub fn syntax_match(
    atp: &TablePattern,
    syntax: &SyntaxCore,
    env: &EvalEnv,
) -> CoreResult<Option<SyntaxMatch>> {
    if !rows_satisfy(syntax, env, 0, syntax.num_rows, atp.condition.as_ref())? {
        return Ok(None);
    }
    let mut state = MatchState::default();
    let outcome = match_patterns(
        &atp.subtable_patterns,
        syntax.num_rows,
        0,
        &mut state,
        &|p: &Arc<SubtablePattern>| p.quantifier,
        &mut |p, i, st| dispatch_subtable(p, i, syntax, env, st),
    )?;
    match outcome {
        Some(next) if next == syntax.num_rows => Ok(Some(SyntaxMatch {
            pairs: state.pairs,
            subtables: state.subtables,
            subrows: state.subrows,
        })),
        _ => Ok(None),
    }
}

// ---------------------------------------------------------------- SemanticConstructor

/// A soft failure of semantic construction (Java `MatchException`):
/// `AtpMatcher.match` maps it to an empty result.
pub enum SemErr {
    Match(#[allow(dead_code)] String),
    Other(CoreErr),
}

impl From<CoreErr> for SemErr {
    fn from(e: CoreErr) -> Self {
        SemErr::Other(e)
    }
}

fn process_atomic(
    atomic: &AtomicSpec,
    row: usize,
    col: usize,
    input_text: &str,
    item_index: usize,
    span: (usize, usize),
    sem: &mut SemanticsCore,
) -> Result<(), SemErr> {
    if atomic.idd == Idd::Skip {
        return Ok(());
    }
    let s = match &atomic.extractor {
        Some(x) => x.apply(input_text).map_err(SemErr::Other)?,
        None => input_text.to_string(),
    };
    let ty = atomic.idd.to_item_type().map_err(SemErr::Other)?;
    let item_id = sem.cell_items.len();
    sem.cell_items.push(CellItem {
        s,
        tags: atomic.tags.clone(),
        index: item_index,
        row,
        col,
        ty,
        span,
    });
    for action_spec in &atomic.actions {
        let action = instantiate_action(ItemId::Cell(item_id), action_spec, sem)?;
        sem.actions.push(action);
    }
    Ok(())
}

/// Parts of a literal split, verbatim, as (part position, part text, byte span
/// in the original cell text); `base` is the offset of `text` within that cell
/// text. Per `def:delimited-content-spec` parts are passed on untrimmed and empty
/// parts are kept, so `n` parts always yield indices `0..n-1`; whitespace removal
/// is opt-in via the atom's extractor (`=TRIM` / `=NORM`).
fn split_with_spans(delim: &str, text: &str, base: usize) -> Vec<(usize, String, (usize, usize))> {
    let mut out = Vec::new();
    let mut start = 0usize;
    for (i, part) in split_literal(delim, text).into_iter().enumerate() {
        let from = base + start;
        let to = from + part.len();
        start += part.len() + delim.len();
        out.push((i, part, (from, to)));
    }
    out
}

fn process_delimited(
    d: &DelimitedSpec,
    row: usize,
    col: usize,
    text: &str,
    sem: &mut SemanticsCore,
) -> Result<(), SemErr> {
    for (i, part, span) in split_with_spans(&d.delimiter, text, 0) {
        process_atomic(&d.atom, row, col, &part, i, span, sem)?;
    }
    Ok(())
}

fn process_compound(
    comp: &CompoundSpec,
    row: usize,
    col: usize,
    text: &str,
    sem: &mut SemanticsCore,
) -> Result<(), SemErr> {
    let mut pos: usize = 0; // byte offset
    let mut item_index: usize = 0;
    let segments = &comp.segments;

    for i in 0..segments.len() {
        let seg = &segments[i];
        if !seg.leading_delimiter.is_empty() {
            match text[pos..].find(&seg.leading_delimiter) {
                Some(rel) => pos = pos + rel + seg.leading_delimiter.len(),
                None => {
                    return Err(SemErr::Match(format!(
                        "Expected delimiter '{}' not found in cell text at pos {}: '{}'",
                        seg.leading_delimiter, pos, text
                    )))
                }
            }
        }

        let next_delim: &str = if i < segments.len() - 1 {
            &segments[i + 1].leading_delimiter
        } else {
            &comp.trailing_delimiter
        };

        let end_pos = if !next_delim.is_empty() {
            match text[pos..].find(next_delim) {
                Some(rel) => pos + rel,
                None => text.len(),
            }
        } else {
            text.len()
        };

        let substring = &text[pos..end_pos];
        match &seg.spec {
            ContentSpec::Atomic(a) => {
                process_atomic(a, row, col, substring, item_index, (pos, end_pos), sem)?;
                item_index += 1;
            }
            ContentSpec::Delimited(d) => {
                for (_, part, span) in split_with_spans(&d.delimiter, substring, pos) {
                    process_atomic(&d.atom, row, col, &part, item_index, span, sem)?;
                    item_index += 1;
                }
            }
            _ => unreachable!("compound segment restricted to atomic/delimited"),
        }
        if !next_delim.is_empty() {
            pos = end_pos;
        }
    }
    Ok(())
}

fn process_content_spec(
    cs: &ContentSpec,
    row: usize,
    col: usize,
    syntax: &SyntaxCore,
    env: &EvalEnv,
    sem: &mut SemanticsCore,
) -> Result<(), SemErr> {
    match cs {
        ContentSpec::Atomic(a) => {
            let text = syntax.cell(row, col).text.clone();
            let span = (0, text.len());
            process_atomic(a, row, col, &text, 0, span, sem)
        }
        ContentSpec::Delimited(d) => {
            let text = syntax.cell(row, col).text.clone();
            process_delimited(d, row, col, &text, sem)
        }
        ContentSpec::Compound(c) => {
            let text = syntax.cell(row, col).text.clone();
            process_compound(c, row, col, &text, sem)
        }
        ContentSpec::Conditional(c) => {
            let take_pos = c
                .condition
                .test(syntax.cell(row, col), env)
                .map_err(SemErr::Other)?;
            let branch = if take_pos { &c.positive } else { &c.negative };
            process_content_spec(branch, row, col, syntax, env, sem)
        }
    }
}

fn get_or_create_context_item(sem: &mut SemanticsCore, lit: &CtxLiteral) -> usize {
    for (i, item) in sem.ctx_items.iter().enumerate() {
        if item.s == lit.text && item.ty == lit.ty {
            return i;
        }
    }
    sem.ctx_items.push(CtxItem {
        s: lit.text.clone(),
        ty: lit.ty,
        const_value: None,
    });
    sem.ctx_items.len() - 1
}

fn to_provider_inst(
    spec: &ProviderSpec,
    sem: &mut SemanticsCore,
    lenient: bool,
) -> Result<ProviderInst, SemErr> {
    if let Some(lit) = &spec.context_literal {
        if lit.const_value.is_some() {
            // Fresh (identity-distinct) context item per provider, like Java.
            sem.ctx_items.push(CtxItem {
                s: lit.text.clone(),
                ty: ItemType::Attribute,
                const_value: lit.const_value.clone(),
            });
            return Ok(ProviderInst::Ctx {
                items: vec![sem.ctx_items.len() - 1],
                kind: CtxKind::Unrestricted,
            });
        }
        let idx = get_or_create_context_item(sem, lit);
        return Ok(ProviderInst::Ctx {
            items: vec![idx],
            kind: lit.kind(),
        });
    }
    Ok(ProviderInst::Cell {
        cond: spec
            .filter_condition
            .clone()
            .ok_or_else(|| SemErr::Other("filterCondition".into()))?,
        order: spec.traversal_order,
        cardinality: spec.cardinality,
        kind: spec.target_item_kind.unwrap_or(CellKind::Unrestricted),
        exclude_anchor: true,
        lenient,
    })
}

fn instantiate_action(
    anchor: ItemId,
    action_spec: &ActionSpec,
    sem: &mut SemanticsCore,
) -> Result<ActionInst, SemErr> {
    let delim = action_spec.delimiter.clone().unwrap_or_default();
    let op = match action_spec.operation_type {
        OperationType::Fill => OpInst::Fill(delim),
        OperationType::Prefix => OpInst::Prefix(delim),
        OperationType::Suffix => OpInst::Suffix(delim),
        OperationType::Avp => OpInst::Avp,
        OperationType::Rec => OpInst::Rec,
        OperationType::Join => OpInst::Join(action_spec.key_positions.clone()),
    };
    let mut providers = Vec::with_capacity(action_spec.providers.len());
    for ps in &action_spec.providers {
        providers.push(to_provider_inst(ps, sem, action_spec.inherited)?);
    }
    Ok(ActionInst { anchor, providers, op })
}

pub fn construct_semantics(
    pairs: &[(Arc<CellPattern>, (usize, usize))],
    context_items: Vec<CtxItem>,
    syntax: &SyntaxCore,
    env: &EvalEnv,
) -> Result<SemanticsCore, SemErr> {
    let mut sem = SemanticsCore {
        cell_items: Vec::new(),
        ctx_items: context_items,
        actions: Vec::new(),
    };
    for (pattern, (r, c)) in pairs {
        let Some(cs) = &pattern.content_spec else { continue };
        process_content_spec(cs, *r, *c, syntax, env, &mut sem)?;
    }
    Ok(sem)
}

// ---------------------------------------------------------------- AtpMatcher

/// `AtpMatcher.applyMatchedStructure`.
pub fn apply_structure(syntax: &mut SyntaxCore, result: &SyntaxMatch) -> CoreResult<()> {
    let mut starts: Vec<usize> = result.subtables.iter().map(|&(s, _)| s).collect();
    starts.sort_unstable();
    starts.dedup();
    if !starts.is_empty() {
        syntax.define_subtables(&starts)?;
    }
    let mut subrows = result.subrows.clone();
    subrows.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    for (row, cs, ce) in subrows {
        syntax.define_subrow(row, cs, ce)?;
    }
    Ok(())
}

/// Port of `AtpMatcher.match` over an owned syntax (used by `cargo test`).
#[allow(dead_code)]
pub fn match_atp(
    atp: &TablePattern,
    syntax: &mut SyntaxCore,
    context_items: Vec<CtxItem>,
) -> CoreResult<Option<SemanticsCore>> {
    let result = {
        let env = EvalEnv { syntax, py_table: None };
        syntax_match(atp, syntax, &env)?
    };
    let Some(result) = result else {
        return Ok(None);
    };
    apply_structure(syntax, &result)?;
    let env = EvalEnv { syntax, py_table: None };
    match construct_semantics(&result.pairs, context_items, syntax, &env) {
        Ok(sem) => Ok(Some(sem)),
        Err(SemErr::Match(_)) => Ok(None),
        Err(SemErr::Other(e)) => Err(e),
    }
}

// ---------------------------------------------------------------- unit tests

/// `S_delim` (`def:delimited-content-spec`) decomposes the text into substrings
/// `s_k` in `Sigma*` and applies `S_atom` to each one verbatim: no trimming, no
/// dropping of empty parts. Whitespace removal is opt-in via the atom's string
/// extractor. Pinned normatively by `conformance/semantic/`.
#[cfg(test)]
mod tests {
    use super::*;
    use crate::rtl::{compile, BindingsCore};

    // -------------------------------------------------------- split_with_spans

    /// One expected `split_with_spans` triple.
    fn part(index: usize, s: &str, from: usize, to: usize) -> (usize, String, (usize, usize)) {
        (index, s.to_string(), (from, to))
    }

    #[test]
    fn split_keeps_token_whitespace() {
        // "a, b" -> "a", " b": the leading space of the second token survives,
        // and its span covers the raw token, not a trimmed one.
        assert_eq!(
            split_with_spans(",", "a, b", 0),
            vec![part(0, "a", 0, 1), part(1, " b", 2, 4)]
        );
        // Trailing whitespace survives just the same.
        assert_eq!(
            split_with_spans(",", "c ,d", 0),
            vec![part(0, "c ", 0, 2), part(1, "d", 3, 4)]
        );
    }

    #[test]
    fn split_keeps_empty_tokens() {
        // "a,,b" -> three parts; the middle one is empty with a zero-width span.
        assert_eq!(
            split_with_spans(",", "a,,b", 0),
            vec![part(0, "a", 0, 1), part(1, "", 2, 2), part(2, "b", 3, 4)]
        );
    }

    #[test]
    fn split_keeps_edge_empty_tokens() {
        // A trailing delimiter yields a trailing empty part (Java `split(_, -1)`).
        assert_eq!(
            split_with_spans(",", "a,b,", 0),
            vec![part(0, "a", 0, 1), part(1, "b", 2, 3), part(2, "", 4, 4)]
        );
        // Symmetrically for a leading one.
        assert_eq!(
            split_with_spans(",", ",a", 0),
            vec![part(0, "", 0, 0), part(1, "a", 1, 2)]
        );
    }

    #[test]
    fn split_indices_are_contiguous() {
        // n parts always derive n items numbered 0..n-1, whatever they contain:
        // blank and empty parts no longer punch holes in the numbering.
        let parts = split_with_spans(",", " ,a,,  ,b, ", 0);
        assert_eq!(parts.len(), 6);
        assert_eq!(
            parts.iter().map(|(i, _, _)| *i).collect::<Vec<_>>(),
            vec![0, 1, 2, 3, 4, 5]
        );
    }

    #[test]
    fn split_spans_are_shifted_by_base() {
        // The compound path passes `base = pos`, so spans stay in whole-cell
        // coordinates. Here " b1, c1" sits at offset 3 of "a1, b1, c1".
        let cell = "a1, b1, c1";
        let parts = split_with_spans(",", &cell[3..], 3);
        assert_eq!(parts, vec![part(0, " b1", 3, 6), part(1, " c1", 7, 10)]);
        // Every span indexes back into the original cell text.
        for (_, s, (from, to)) in parts {
            assert_eq!(cell[from..to], s);
        }
    }

    #[test]
    fn split_spans_are_byte_exact_on_multibyte_text() {
        // "\u{43f}\u{440}, \u{431}" — two bytes per Cyrillic letter.
        let cell = "\u{43f}\u{440}, \u{431}";
        let parts = split_with_spans(",", cell, 0);
        assert_eq!(
            parts,
            vec![part(0, "\u{43f}\u{440}", 0, 4), part(1, " \u{431}", 5, 8)]
        );
        for (_, s, (from, to)) in parts {
            assert_eq!(cell[from..to], s);
        }
    }

    // ------------------------------------------------------- end to end (ATP)

    /// Items derived from the single cell of a 1x1 table, as (s, span, index).
    fn items_of(rtl: &str, text: &str) -> Vec<(String, (usize, usize), usize)> {
        let mut syntax = SyntaxCore::new(1, 1).unwrap();
        syntax.cell_mut(0, 0).set_text(text.to_string());
        let pattern = compile(rtl, &BindingsCore::default()).expect("compile");
        let sem = match_atp(&pattern, &mut syntax, Vec::new())
            .expect("match")
            .expect("pattern must match");
        sem.cell_items
            .iter()
            .map(|it| (it.s.clone(), it.span, it.index))
            .collect()
    }

    /// One expected `items_of` triple.
    fn item(s: &str, from: usize, to: usize, index: usize) -> (String, (usize, usize), usize) {
        (s.to_string(), (from, to), index)
    }

    #[test]
    fn delimited_cell_derives_raw_items() {
        assert_eq!(
            items_of("[ [(VAL : CL*->REC){','}] ]", "a, b"),
            vec![item("a", 0, 1, 0), item(" b", 2, 4, 1)]
        );
    }

    #[test]
    fn delimited_cell_derives_an_item_per_empty_token() {
        assert_eq!(
            items_of("[ [(VAL : CL*->REC){','}] ]", "a,,b"),
            vec![item("a", 0, 1, 0), item("", 2, 2, 1), item("b", 3, 4, 2)]
        );
    }

    #[test]
    fn trim_extractor_opts_back_into_trimming() {
        // `=TRIM` applies to each substring separately, restoring the old values.
        // Spans stay raw: `CellItem.span` is deliberately pre-extractor.
        assert_eq!(
            items_of("[ [(VAL=TRIM : CL*->REC){','}] ]", "a, b"),
            vec![item("a", 0, 1, 0), item("b", 2, 4, 1)]
        );
        // …but it does not resurrect the dropping of empty tokens.
        assert_eq!(
            items_of("[ [(VAL=TRIM : CL*->REC){','}] ]", "a, ,b"),
            vec![item("a", 0, 1, 0), item("", 2, 3, 1), item("b", 4, 5, 2)]
        );
    }

    #[test]
    fn delimited_nested_in_compound_derives_raw_items() {
        // Segment 1 is atomic ("a1"), the remainder is a delimited segment. The
        // compound path numbers items with its own running counter, which stays
        // contiguous, and spans stay in whole-cell coordinates.
        let cell = "a1, b1, c1";
        let items = items_of("[ [VAL: CL*->REC ',' (VAL){','}] ]", cell);
        assert_eq!(
            items,
            vec![
                item("a1", 0, 2, 0),
                item(" b1", 3, 6, 1),
                item(" c1", 7, 10, 2),
            ]
        );
        for (s, (from, to), _) in items {
            assert_eq!(cell[from..to], s);
        }
    }
}
