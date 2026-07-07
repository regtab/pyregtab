//! Port of `rtl.internal.ATPBuilder` + `ProviderTemplateResolver` +
//! `StringExtractorFactory`: AST → ATP spec hierarchy.

use super::ast::*;
use super::{BindingsCore, RtlErr};
use crate::spec::*;
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::Arc;

pub struct ATPBuilder<'a> {
    bindings: &'a BindingsCore,
    inherited_stack: Vec<Vec<ActionSpec>>,
    cell_frags: HashMap<String, Option<CellBodyAst>>,
    row_frags: HashMap<String, RowBodyAst>,
    subrow_frags: HashMap<String, SubrowBodyAst>,
    subtable_frags: HashMap<String, SubtableBodyAst>,
}

fn quant(q: &Option<PQuant>) -> Result<Quantifier, RtlErr> {
    Ok(match q {
        None => Quantifier::ONE,
        Some(PQuant::ZeroOrOne) => Quantifier::ZERO_OR_ONE,
        Some(PQuant::ZeroOrMore) => Quantifier::ZERO_OR_MORE,
        Some(PQuant::OneOrMore) => Quantifier::ONE_OR_MORE,
        Some(PQuant::Exactly(n)) => Quantifier::exactly(*n).map_err(RtlErr::from)?,
    })
}

fn build_extractor(steps: &[PStep]) -> Extractor {
    let one = |s: &PStep| match s {
        PStep::Substr(begin, length) => Extractor::Substring(*begin, *begin + *length),
        PStep::Repl(a, b) => Extractor::Replaced(a.clone(), b.clone()),
        PStep::Norm => Extractor::WhitespaceNormalized,
        PStep::Uc => Extractor::UpperCase,
        PStep::Lc => Extractor::LowerCase,
        PStep::Trim => Extractor::Trimmed,
    };
    if steps.len() == 1 {
        one(&steps[0])
    } else {
        Extractor::Chain(steps.iter().map(one).collect())
    }
}

impl<'a> ATPBuilder<'a> {
    pub fn new(bindings: &'a BindingsCore) -> Self {
        ATPBuilder {
            bindings,
            inherited_stack: Vec::new(),
            cell_frags: HashMap::new(),
            row_frags: HashMap::new(),
            subrow_frags: HashMap::new(),
            subtable_frags: HashMap::new(),
        }
    }

    pub fn build(&mut self, tree: &TableAst) -> Result<TablePattern, RtlErr> {
        for (name, body) in &tree.fragments {
            match body {
                FragBodyAst::Cell(b) => {
                    self.cell_frags.insert(name.clone(), b.clone());
                }
                FragBodyAst::Row(b) => {
                    self.row_frags.insert(name.clone(), b.clone());
                }
                FragBodyAst::Subrow(b) => {
                    self.subrow_frags.insert(name.clone(), b.clone());
                }
                FragBodyAst::Subtable(b) => {
                    self.subtable_frags.insert(name.clone(), b.clone());
                }
            }
        }
        let cond = tree
            .cond
            .as_ref()
            .map(|c| self.build_cell_cond(c))
            .transpose()?;
        let local = match &tree.actions {
            Some(a) => self.build_act_specs(a, None)?,
            None => Vec::new(),
        };
        self.push_inherited(local);
        let result = (|| {
            let mut subtables = Vec::new();
            for st in &tree.subtables {
                subtables.push(Arc::new(self.build_subtable(st)?));
            }
            let mut transformations = Vec::new();
            if let Some(settings) = &tree.settings {
                for s in settings {
                    transformations.push(match s {
                        PSetting::Norm => Transformation::WhitespaceNormalization,
                        PSetting::Anch(n) => Transformation::AnchorAttributeAtPosition(*n),
                        PSetting::Split(d) => Transformation::DelimitedFieldSplit {
                            delimiter: d.clone(),
                            only_attributes: None,
                            template: "$a_%i".to_string(),
                        },
                    });
                }
            }
            TablePattern::new(cond, subtables, transformations).map_err(RtlErr::from)
        })();
        self.inherited_stack.pop();
        result
    }

    // ---------------- subtable ----------------

    fn build_subtable(&mut self, ast: &SubtableAst) -> Result<SubtablePattern, RtlErr> {
        match ast {
            SubtableAst::Frag { name, quant: q } => {
                let body = self
                    .subtable_frags
                    .get(name)
                    .cloned()
                    .ok_or_else(|| RtlErr::new(format!("Unknown subtable fragment: ${name}")))?;
                self.build_subtable_body(&body, quant(q)?)
            }
            SubtableAst::Impl(rows) => {
                let mut out = Vec::new();
                for r in rows {
                    out.push(Arc::new(self.build_row(r)?));
                }
                SubtablePattern::new(None, Quantifier::ONE, out).map_err(RtlErr::from)
            }
            SubtableAst::Expl { body, quant: q } => self.build_subtable_body(body, quant(q)?),
        }
    }

    fn build_subtable_body(
        &mut self,
        body: &SubtableBodyAst,
        q: Quantifier,
    ) -> Result<SubtablePattern, RtlErr> {
        let cond = body
            .cond
            .as_ref()
            .map(|c| self.build_cell_cond(c))
            .transpose()?;
        let local = match &body.actions {
            Some(a) => self.build_act_specs(a, None)?,
            None => Vec::new(),
        };
        self.push_inherited(local);
        let result = (|| {
            let mut rows = Vec::new();
            for r in &body.rows {
                rows.push(Arc::new(self.build_row(r)?));
            }
            SubtablePattern::new(cond, q, rows).map_err(RtlErr::from)
        })();
        self.inherited_stack.pop();
        result
    }

    // ---------------- row ----------------

    fn build_row(&mut self, ast: &RowAst) -> Result<RowPattern, RtlErr> {
        match ast {
            RowAst::Frag { name, quant: q } => {
                let body = self
                    .row_frags
                    .get(name)
                    .cloned()
                    .ok_or_else(|| RtlErr::new(format!("Unknown row fragment: ${name}")))?;
                self.build_row_body(&body, quant(q)?)
            }
            RowAst::Body { body, quant: q } => self.build_row_body(body, quant(q)?),
        }
    }

    fn build_row_body(&mut self, body: &RowBodyAst, q: Quantifier) -> Result<RowPattern, RtlErr> {
        let cond = body
            .cond
            .as_ref()
            .map(|c| self.build_cell_cond(c))
            .transpose()?;
        let local = match &body.actions {
            Some(a) => self.build_act_specs(a, None)?,
            None => Vec::new(),
        };
        self.push_inherited(local);
        let result = (|| {
            let mut subrows = Vec::new();
            for sr in &body.subrows {
                subrows.push(Arc::new(self.build_subrow(sr)?));
            }
            RowPattern::new(cond, q, subrows).map_err(RtlErr::from)
        })();
        self.inherited_stack.pop();
        result
    }

    // ---------------- subrow ----------------

    fn build_subrow(&mut self, ast: &SubrowAst) -> Result<SubrowPattern, RtlErr> {
        match ast {
            SubrowAst::Frag { name, quant: q } => {
                let body = self
                    .subrow_frags
                    .get(name)
                    .cloned()
                    .ok_or_else(|| RtlErr::new(format!("Unknown subrow fragment: ${name}")))?;
                self.build_subrow_body(&body, quant(q)?)
            }
            SubrowAst::Impl(cells) => {
                let mut out = Vec::new();
                for c in cells {
                    out.push(Arc::new(self.build_cell(c)?));
                }
                SubrowPattern::new(None, Quantifier::ONE, out).map_err(RtlErr::from)
            }
            SubrowAst::Expl { body, quant: q } => self.build_subrow_body(body, quant(q)?),
        }
    }

    fn build_subrow_body(
        &mut self,
        body: &SubrowBodyAst,
        q: Quantifier,
    ) -> Result<SubrowPattern, RtlErr> {
        let cond = body
            .cond
            .as_ref()
            .map(|c| self.build_cell_cond(c))
            .transpose()?;
        let local = match &body.actions {
            Some(a) => self.build_act_specs(a, None)?,
            None => Vec::new(),
        };
        self.push_inherited(local);
        let result = (|| {
            let mut cells = Vec::new();
            for c in &body.cells {
                cells.push(Arc::new(self.build_cell(c)?));
            }
            SubrowPattern::new(cond, q, cells).map_err(RtlErr::from)
        })();
        self.inherited_stack.pop();
        result
    }

    // ---------------- cell ----------------

    fn build_cell(&mut self, ast: &CellAst) -> Result<CellPattern, RtlErr> {
        match ast {
            CellAst::Frag { name, quant: q } => {
                let body = self
                    .cell_frags
                    .get(name)
                    .cloned()
                    .ok_or_else(|| RtlErr::new(format!("Unknown cell fragment: ${name}")))?;
                self.build_cell_body(body.as_ref(), quant(q)?)
            }
            CellAst::Body { body, quant: q } => self.build_cell_body(body.as_ref(), quant(q)?),
        }
    }

    fn build_cell_body(
        &mut self,
        body: Option<&CellBodyAst>,
        q: Quantifier,
    ) -> Result<CellPattern, RtlErr> {
        // [] and [BLANK?] and [SKIP] are all "skip" cells
        let is_skip_atom = matches!(
            body.and_then(|b| b.cont.as_ref()),
            Some(ContAst::Atom(a)) if a.idd == Idd::Skip
        );
        if body.is_none() || body.unwrap().cont.is_none() || is_skip_atom {
            let cond = match body.and_then(|b| b.cond.as_ref()) {
                Some(c) => Some(self.build_cell_cond(c)?),
                None => None,
            };
            return Ok(CellPattern { condition: cond, quantifier: q, content_spec: None });
        }
        let body = body.unwrap();
        let cond = body
            .cond
            .as_ref()
            .map(|c| self.build_cell_cond(c))
            .transpose()?;
        let local = match &body.actions {
            Some(a) => self.build_act_specs(a, None)?,
            None => Vec::new(),
        };
        self.push_inherited(local);
        let result = (|| {
            let cs = self.build_content_spec(body.cont.as_ref().unwrap())?;
            Ok(CellPattern { condition: cond, quantifier: q, content_spec: Some(cs) })
        })();
        self.inherited_stack.pop();
        result
    }

    // ---------------- content specs ----------------

    fn build_content_spec(&mut self, ast: &ContAst) -> Result<ContentSpec, RtlErr> {
        Ok(match ast {
            ContAst::Atom(a) => ContentSpec::Atomic(self.build_atomic(a)?),
            ContAst::Delim(d) => ContentSpec::Delimited(self.build_delimited(d)?),
            ContAst::Comp(c) => ContentSpec::Compound(self.build_compound(c)?),
            ContAst::Cond(c) => {
                let cond = self.build_cell_cond(&c.cond)?;
                let pos = self.build_x(&c.positive)?;
                let neg = self.build_x(&c.negative)?;
                ContentSpec::Conditional(Box::new(ConditionalSpec {
                    condition: cond,
                    positive: pos,
                    negative: neg,
                }))
            }
        })
    }

    fn build_x(&mut self, ast: &XSpecAst) -> Result<ContentSpec, RtlErr> {
        Ok(match ast {
            XSpecAst::Atom(a) => ContentSpec::Atomic(self.build_atomic(a)?),
            XSpecAst::Delim(d) => ContentSpec::Delimited(self.build_delimited(d)?),
            XSpecAst::Comp(c) => ContentSpec::Compound(self.build_compound(c)?),
        })
    }

    fn build_atomic(&mut self, ast: &AtomAst) -> Result<AtomicSpec, RtlErr> {
        let tags: Vec<String> = ast.tags.iter().map(|t| format!("#{t}")).collect();
        let extractor = ast.extractor.as_ref().map(|s| build_extractor(s));
        let local = match &ast.actions {
            Some(a) => self.build_act_specs(a, Some(ast.idd))?,
            None => Vec::new(),
        };
        Ok(AtomicSpec {
            idd: ast.idd,
            extractor,
            tags,
            actions: self.merge_with_inherited(local),
        })
    }

    fn build_delimited(&mut self, ast: &DelimAst) -> Result<DelimitedSpec, RtlErr> {
        let atom = self.build_atomic(&ast.atom)?;
        DelimitedSpec::new(ast.separator.clone(), atom).map_err(RtlErr::from)
    }

    fn build_compound(&mut self, ast: &CompAst) -> Result<CompoundSpec, RtlErr> {
        let open = ast.open_delim.clone().unwrap_or_default();
        let close = ast.close_delim.clone().unwrap_or_default();
        let mut segments = Vec::new();
        segments.push(
            CompoundSegment::new(open, self.build_seg(&ast.first)?).map_err(RtlErr::from)?,
        );
        for (sep, seg) in &ast.rest {
            segments.push(
                CompoundSegment::new(sep.clone(), self.build_seg(seg)?).map_err(RtlErr::from)?,
            );
        }
        CompoundSpec::new(segments, close).map_err(RtlErr::from)
    }

    fn build_seg(&mut self, ast: &CompSegAst) -> Result<ContentSpec, RtlErr> {
        Ok(match ast {
            CompSegAst::Atom(a) => ContentSpec::Atomic(self.build_atomic(a)?),
            CompSegAst::Delim(d) => ContentSpec::Delimited(self.build_delimited(d)?),
        })
    }

    // ---------------- action specs ----------------

    fn build_act_specs(
        &mut self,
        specs: &[PActSpec],
        anchor_type: Option<Idd>,
    ) -> Result<Vec<ActionSpec>, RtlErr> {
        specs
            .iter()
            .map(|a| self.build_act_spec(a, anchor_type))
            .collect()
    }

    fn build_act_spec(
        &mut self,
        ast: &PActSpec,
        _anchor_type: Option<Idd>,
    ) -> Result<ActionSpec, RtlErr> {
        let mut providers = Vec::new();
        for p in &ast.providers {
            providers.push(self.build_prov_spec(p, &ast.op)?);
        }
        let (op, delim, anchor_pos, split_delim, keys): (
            OperationType,
            Option<String>,
            Option<i64>,
            Option<String>,
            BTreeSet<i64>,
        ) = match &ast.op {
            POp::Avp => (OperationType::Avp, None, None, None, BTreeSet::new()),
            POp::Rec { anchor, split } => (
                OperationType::Rec,
                None,
                *anchor,
                split.clone(),
                BTreeSet::new(),
            ),
            POp::Join(keys) => (
                OperationType::Join,
                None,
                None,
                None,
                keys.iter().copied().collect(),
            ),
            POp::Fill(d) => (
                OperationType::Fill,
                Some(d.clone().unwrap_or_default()),
                None,
                None,
                BTreeSet::new(),
            ),
            POp::Prefix(d) => (
                OperationType::Prefix,
                Some(d.clone().unwrap_or_default()),
                None,
                None,
                BTreeSet::new(),
            ),
            POp::Suffix(d) => (
                OperationType::Suffix,
                Some(d.clone().unwrap_or_default()),
                None,
                None,
                BTreeSet::new(),
            ),
        };
        ActionSpec::new(op, delim, providers, anchor_pos, split_delim, keys, false)
            .map_err(RtlErr::from)
    }

    fn build_prov_spec(&mut self, ast: &PProvSpec, op: &POp) -> Result<ProviderSpec, RtlErr> {
        match ast {
            PProvSpec::Tbl(t) => self.resolve_tbl_provider(t, op),
            PProvSpec::CtxAvp(name, value) => {
                Ok(ProviderSpec::ctx_avp(name.clone(), value.clone()))
            }
            PProvSpec::Ctx(literal) => {
                let ty = if matches!(op, POp::Rec { .. } | POp::Join(_)) {
                    ItemType::Value
                } else {
                    ItemType::Attribute
                };
                Ok(ProviderSpec::ctx(literal.clone(), ty))
            }
        }
    }

    // ---------------- ProviderTemplateResolver ----------------

    fn resolve_tbl_provider(&mut self, ast: &PTblProv, op: &POp) -> Result<ProviderSpec, RtlErr> {
        let cardinality = match &ast.cardinality {
            None => 1,
            Some(PCard::Unbounded) => UNBOUNDED,
            Some(PCard::Exactly(n)) => *n,
        };
        let condition = self.build_provider_condition(&ast.body)?;
        let kind = match op {
            POp::Rec { .. } | POp::Join(_) => CellKind::Val,
            POp::Avp => CellKind::Attr,
            _ => CellKind::Unrestricted,
        };
        let actual_cardinality = if kind == CellKind::Attr { 1 } else { cardinality };
        ProviderSpec::new(
            actual_cardinality,
            ast.order,
            Some(condition),
            Some(kind),
            None,
        )
        .map_err(RtlErr::from)
    }

    fn build_provider_condition(&mut self, body: &PTblBody) -> Result<FilterCond, RtlErr> {
        match body {
            PTblBody::Spat(s) => {
                let term = self.spat_term(s)?;
                Ok(FilterCond::Bare(term))
            }
            PTblBody::Parens(c) => self.build_constraints(c),
            PTblBody::BareConj(spat, base) => {
                let spat_term = self.spat_term(spat)?;
                let mut distributed: Vec<Vec<FilterTerm>> = vec![vec![spat_term]];
                for bc in base {
                    let alts = self.expand_base_constr(bc)?;
                    let mut next = Vec::new();
                    for existing in &distributed {
                        for alt in &alts {
                            let mut combined = existing.clone();
                            combined.extend(alt.iter().cloned());
                            next.push(combined);
                        }
                    }
                    distributed = next;
                }
                Ok(to_spec(distributed))
            }
        }
    }

    fn build_constraints(&mut self, ast: &PConstraints) -> Result<FilterCond, RtlErr> {
        if ast.or_groups.len() == 1 {
            return self.build_or_group(&ast.or_groups[0]);
        }
        let mut all_ands: Vec<Vec<FilterTerm>> = Vec::new();
        for g in &ast.or_groups {
            match self.build_or_group(g)? {
                FilterCond::Bare(t) => all_ands.push(vec![t]),
                FilterCond::And(ts) => all_ands.push(ts),
                FilterCond::Or(gs) => all_ands.extend(gs),
                FilterCond::Custom { .. } => {
                    return Err(RtlErr::new("Unexpected spec type in OR"))
                }
            }
        }
        Ok(FilterCond::Or(all_ands))
    }

    fn build_or_group(&mut self, ast: &POrGroup) -> Result<FilterCond, RtlErr> {
        let mut distributed: Vec<Vec<FilterTerm>> = vec![Vec::new()];
        for bc in &ast.base {
            let alts = self.expand_base_constr(bc)?;
            let mut next = Vec::new();
            for existing in &distributed {
                for alt in &alts {
                    let mut combined = existing.clone();
                    combined.extend(alt.iter().cloned());
                    next.push(combined);
                }
            }
            distributed = next;
        }
        Ok(to_spec(distributed))
    }

    fn expand_base_constr(&mut self, ast: &PBaseConstr) -> Result<Vec<Vec<FilterTerm>>, RtlErr> {
        match ast {
            PBaseConstr::Parens(c) => {
                let inner = self.build_constraints(c)?;
                Ok(match inner {
                    FilterCond::Bare(t) => vec![vec![t]],
                    FilterCond::And(ts) => vec![ts],
                    FilterCond::Or(gs) => gs,
                    FilterCond::Custom { .. } => {
                        return Err(RtlErr::new("Unexpected nested spec type"))
                    }
                })
            }
            PBaseConstr::Constr(c) => {
                let term = match c {
                    PConstr::Spat(s) => self.spat_term(s)?,
                    PConstr::Cont(cc) => self.cont_term(cc)?,
                };
                Ok(vec![vec![term]])
            }
        }
    }

    fn spat_term(&self, ast: &PSpat) -> Result<FilterTerm, RtlErr> {
        Ok(match ast {
            PSpat::Named(n) => match n {
                NamedSpat::LeftOf => FilterTerm::LeftOf,
                NamedSpat::RightOf => FilterTerm::RightOf,
                NamedSpat::Above => FilterTerm::Above,
                NamedSpat::Below => FilterTerm::Below,
                NamedSpat::SameRow => FilterTerm::SameRow,
                NamedSpat::SameCol => FilterTerm::SameCol,
                NamedSpat::SameSubrow => FilterTerm::SameSubrow,
                NamedSpat::SameSubcol => FilterTerm::SameSubcol,
                NamedSpat::SameSubtable => FilterTerm::SameSubtable,
                NamedSpat::NotSameCell => FilterTerm::NotSameCell,
                NamedSpat::SameCell => FilterTerm::SameCell,
            },
            PSpat::Col(p) => match p {
                PPosLike::Range { start, end } => {
                    let start_is_offset = matches!(start, PBound::Offset(_));
                    let lo = bound_value(start);
                    let hi = end.as_ref().map(bound_value).unwrap_or(UNBOUNDED);
                    if start_is_offset {
                        FilterTerm::ColRange(lo, hi)
                    } else {
                        FilterTerm::ColAbsoluteRange(lo, hi)
                    }
                }
                PPosLike::Offset(d) => FilterTerm::ColOffset(*d),
                PPosLike::Int(n) => FilterTerm::ColExact(*n),
            },
            PSpat::Row(p) => match p {
                PPosLike::Range { start, end } => {
                    let start_is_offset = matches!(start, PBound::Offset(_));
                    let lo = bound_value(start);
                    let hi = end.as_ref().map(bound_value).unwrap_or(UNBOUNDED);
                    if start_is_offset {
                        FilterTerm::RowOffset(lo) // R+n.. not yet needed (as in Java)
                    } else {
                        FilterTerm::RowAbsoluteRange(lo, hi)
                    }
                }
                PPosLike::Offset(d) => FilterTerm::RowOffset(*d),
                PPosLike::Int(n) => FilterTerm::RowExact(*n),
            },
            PSpat::Pos(p) => match p {
                PPosLike::Range { start, end } => {
                    let lo = bound_value(start);
                    let hi = end.as_ref().map(bound_value).unwrap_or(UNBOUNDED);
                    FilterTerm::PosRange(lo, hi)
                }
                PPosLike::Offset(d) => FilterTerm::PosOffset(*d),
                PPosLike::Int(n) => FilterTerm::PosExact(*n),
            },
        })
    }

    fn cont_term(&self, ast: &PContConstr) -> Result<FilterTerm, RtlErr> {
        Ok(match ast {
            PContConstr::Regex { neg: false, pattern } => FilterTerm::Regex(pattern.clone()),
            PContConstr::Regex { neg: true, pattern } => FilterTerm::NotRegex(pattern.clone()),
            PContConstr::Blank { neg: false } => FilterTerm::Blank,
            PContConstr::Blank { neg: true } => FilterTerm::NotBlank,
            PContConstr::Tag { neg, name } => {
                let tag = format!("#{name}");
                if *neg {
                    FilterTerm::NotTagged(tag)
                } else {
                    FilterTerm::Tagged(tag)
                }
            }
            PContConstr::SameStr => FilterTerm::SameStr,
            PContConstr::Contains { neg: false, substring } => {
                FilterTerm::Contains(substring.clone())
            }
            PContConstr::Contains { neg: true, substring } => {
                FilterTerm::NotContains(substring.clone())
            }
            PContConstr::Ext { name, line, col } => {
                if name.trim().is_empty() {
                    return Err(RtlErr::at(
                        "EXT binding name must not be blank",
                        *line,
                        *col,
                    ));
                }
                match self.bindings.filter.get(name) {
                    Some(f) => FilterTerm::External { name: name.clone(), func: f.clone() },
                    None => {
                        let hint = if self.bindings.cell.contains_key(name) {
                            format!(
                                " ('{name}' is bound as a cell predicate; a provider constraint needs Bindings.filter(...))"
                            )
                        } else {
                            String::new()
                        };
                        return Err(RtlErr::at(
                            format!("Unbound EXT item filter: '{name}'{hint}"),
                            *line,
                            *col,
                        ));
                    }
                }
            }
        })
    }

    // ---------------- cell match condition ----------------

    fn build_cell_cond(&mut self, ast: &PCellCond) -> Result<CellPredicate, RtlErr> {
        Ok(match ast {
            PCellCond::Regex { neg: false, pattern } => CellPredicate::Regex(pattern.clone()),
            PCellCond::Regex { neg: true, pattern } => CellPredicate::NotRegex(pattern.clone()),
            PCellCond::Blank { neg: false } => CellPredicate::Blank,
            PCellCond::Blank { neg: true } => CellPredicate::NotBlank,
            PCellCond::Contains { neg: false, substring } => {
                CellPredicate::Contains(substring.clone())
            }
            PCellCond::Contains { neg: true, substring } => {
                CellPredicate::NotContains(substring.clone())
            }
            PCellCond::Ext { name, line, col } => {
                if name.trim().is_empty() {
                    return Err(RtlErr::at(
                        "EXT binding name must not be blank",
                        *line,
                        *col,
                    ));
                }
                match self.bindings.cell.get(name) {
                    Some(f) => CellPredicate::External { name: name.clone(), func: f.clone() },
                    None => {
                        let hint = if self.bindings.filter.contains_key(name) {
                            format!(
                                " ('{name}' is bound as a filter; a cell match condition needs Bindings.cell(...))"
                            )
                        } else {
                            String::new()
                        };
                        return Err(RtlErr::at(
                            format!("Unbound EXT cell predicate: '{name}'{hint}"),
                            *line,
                            *col,
                        ));
                    }
                }
            }
        })
    }

    // ---------------- inherited actions stack ----------------

    fn current_inherited(&self) -> Vec<ActionSpec> {
        self.inherited_stack.last().cloned().unwrap_or_default()
    }

    fn push_inherited(&mut self, local: Vec<ActionSpec>) {
        if local.is_empty() {
            let cur = self.current_inherited();
            self.inherited_stack.push(cur);
        } else {
            let mut merged = self.current_inherited();
            merged.extend(local);
            self.inherited_stack.push(merged);
        }
    }

    fn merge_with_inherited(&self, local: Vec<ActionSpec>) -> Vec<ActionSpec> {
        let from_stack = self.current_inherited();
        if from_stack.is_empty() {
            return local;
        }
        let mut all: Vec<ActionSpec> = from_stack.iter().map(|a| a.as_inherited()).collect();
        all.extend(local);
        all
    }
}

fn bound_value(b: &PBound) -> i64 {
    match b {
        PBound::Offset(d) => *d,
        PBound::Int(n) => *n,
    }
}

fn to_spec(mut distributed: Vec<Vec<FilterTerm>>) -> FilterCond {
    if distributed.len() == 1 {
        let parts = distributed.pop().unwrap();
        if parts.len() == 1 {
            FilterCond::Bare(parts.into_iter().next().unwrap())
        } else {
            FilterCond::And(parts)
        }
    } else {
        FilterCond::Or(distributed)
    }
}
