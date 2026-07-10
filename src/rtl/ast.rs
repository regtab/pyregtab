//! AST mirroring the parse-rule structure of `RTL.g4`.

use crate::spec::{Idd, TraversalOrder};

#[derive(Clone, Debug)]
pub enum PCellCond {
    Regex { neg: bool, pattern: String },
    Blank { neg: bool },
    Contains { neg: bool, substring: String },
    Ext { name: String, line: i64, col: i64 },
}

#[derive(Clone, Debug)]
pub enum PSetting {
    Norm,
    Anch(i64),
    Split(String),
}

#[derive(Clone, Copy, Debug)]
pub enum PQuant {
    ZeroOrOne,
    ZeroOrMore,
    OneOrMore,
    Exactly(i64),
}

#[derive(Clone, Debug)]
pub enum PBound {
    Offset(i64),
    Int(i64),
}

#[derive(Clone, Debug)]
pub enum PPosLike {
    Range { start: PBound, end: Option<PBound> },
    Offset(i64),
    Int(i64),
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum NamedSpat {
    LeftOf,
    RightOf,
    Above,
    Below,
    SameRow,
    SameCol,
    SameSubrow,
    SameSubcol,
    SameSubtable,
    NotSameCell,
    SameCell,
}

#[derive(Clone, Debug)]
pub enum PSpat {
    Named(NamedSpat),
    Col(PPosLike),
    Row(PPosLike),
    Pos(PPosLike),
}

#[derive(Clone, Debug)]
pub enum PContConstr {
    Regex { neg: bool, pattern: String },
    Blank { neg: bool },
    Tag { neg: bool, name: String },
    SameStr,
    Contains { neg: bool, substring: String },
    Ext { name: String, line: i64, col: i64 },
}

#[derive(Clone, Debug)]
pub enum PConstr {
    Spat(PSpat),
    Cont(PContConstr),
}

#[derive(Clone, Debug)]
pub enum PBaseConstr {
    Constr(PConstr),
    Parens(PConstraints),
}

#[derive(Clone, Debug)]
pub struct POrGroup {
    pub base: Vec<PBaseConstr>,
}

#[derive(Clone, Debug)]
pub struct PConstraints {
    pub or_groups: Vec<POrGroup>,
}

#[derive(Clone, Debug)]
pub enum PTblBody {
    Spat(PSpat),
    Parens(PConstraints),
    BareConj(PSpat, Vec<PBaseConstr>),
}

#[derive(Clone, Debug)]
pub enum PCard {
    Exactly(i64),
    Unbounded,
}

#[derive(Clone, Debug)]
pub struct PTblProv {
    pub order: TraversalOrder,
    pub body: PTblBody,
    pub cardinality: Option<PCard>,
}

#[derive(Clone, Debug)]
pub enum PProvSpec {
    Tbl(PTblProv),
    Ctx(String),
    CtxAvp(String, String),
}

#[derive(Clone, Debug)]
pub enum POp {
    Avp,
    Rec { anchor: Option<i64>, split: Option<String> },
    Join(Vec<i64>),
    Fill(Option<String>),
    Prefix(Option<String>),
    Suffix(Option<String>),
}

#[derive(Clone, Debug)]
pub struct PActSpec {
    pub providers: Vec<PProvSpec>,
    pub op: POp,
}

#[derive(Clone, Debug)]
pub enum PStep {
    Substr(i64, i64),
    Repl(String, String),
    Norm,
    Uc,
    Lc,
    Trim,
}

#[derive(Clone, Debug)]
pub struct AtomAst {
    pub idd: Idd,
    pub tags: Vec<String>,
    pub extractor: Option<Vec<PStep>>,
    pub actions: Option<Vec<PActSpec>>,
}

#[derive(Clone, Debug)]
pub struct DelimAst {
    pub atom: AtomAst,
    pub separator: String,
}

#[derive(Clone, Debug)]
pub enum CompSegAst {
    Atom(AtomAst),
    Delim(DelimAst),
}

#[derive(Clone, Debug)]
pub struct CompAst {
    pub open_delim: Option<String>,
    /// first segment + (separator, segment)*
    pub first: CompSegAst,
    pub rest: Vec<(String, CompSegAst)>,
    pub close_delim: Option<String>,
}

#[derive(Clone, Debug)]
pub enum XSpecAst {
    Atom(AtomAst),
    Delim(DelimAst),
    Comp(CompAst),
}

#[derive(Clone, Debug)]
pub struct CondAst {
    pub cond: PCellCond,
    pub positive: XSpecAst,
    pub negative: XSpecAst,
}

#[derive(Clone, Debug)]
pub enum ContAst {
    Atom(AtomAst),
    Delim(DelimAst),
    Comp(CompAst),
    Cond(Box<CondAst>),
}

#[derive(Clone, Debug)]
pub struct CellBodyAst {
    pub cond: Option<PCellCond>,
    pub actions: Option<Vec<PActSpec>>,
    pub cont: Option<ContAst>,
}

#[derive(Clone, Debug)]
// The AST is built once per compiled pattern and immediately lowered; the
// per-variant size imbalance has no measurable cost there.
#[allow(clippy::large_enum_variant)]
pub enum CellAst {
    Body { body: Option<CellBodyAst>, quant: Option<PQuant> },
    Frag { name: String, quant: Option<PQuant> },
}

#[derive(Clone, Debug)]
pub struct SubrowBodyAst {
    pub cond: Option<PCellCond>,
    pub actions: Option<Vec<PActSpec>>,
    pub cells: Vec<CellAst>,
}

#[derive(Clone, Debug)]
pub enum SubrowAst {
    Impl(Vec<CellAst>),
    Expl { body: SubrowBodyAst, quant: Option<PQuant> },
    Frag { name: String, quant: Option<PQuant> },
}

#[derive(Clone, Debug)]
pub struct RowBodyAst {
    pub cond: Option<PCellCond>,
    pub actions: Option<Vec<PActSpec>>,
    pub subrows: Vec<SubrowAst>,
}

#[derive(Clone, Debug)]
pub enum RowAst {
    Body { body: RowBodyAst, quant: Option<PQuant> },
    Frag { name: String, quant: Option<PQuant> },
}

#[derive(Clone, Debug)]
pub struct SubtableBodyAst {
    pub cond: Option<PCellCond>,
    pub actions: Option<Vec<PActSpec>>,
    pub rows: Vec<RowAst>,
}

#[derive(Clone, Debug)]
pub enum SubtableAst {
    Impl(Vec<RowAst>),
    Expl { body: SubtableBodyAst, quant: Option<PQuant> },
    Frag { name: String, quant: Option<PQuant> },
}

#[derive(Clone, Debug)]
pub enum FragBodyAst {
    Cell(Option<CellBodyAst>),
    Row(RowBodyAst),
    Subrow(SubrowBodyAst),
    Subtable(SubtableBodyAst),
}

#[derive(Clone, Debug)]
pub struct TableAst {
    pub fragments: Vec<(String, FragBodyAst)>,
    pub cond: Option<PCellCond>,
    pub settings: Option<Vec<PSetting>>,
    pub actions: Option<Vec<PActSpec>>,
    pub subtables: Vec<SubtableAst>,
}

// ------------------------------------------------ REC-param collection

pub fn collect_rec_params(tree: &TableAst) -> (Vec<i64>, Vec<String>) {
    let mut anchors = Vec::new();
    let mut splits = Vec::new();

    fn walk_atom(a: &AtomAst, anchors: &mut Vec<i64>, splits: &mut Vec<String>) {
        if let Some(acts) = &a.actions {
            for act in acts {
                if let POp::Rec { anchor, split } = &act.op {
                    if let Some(n) = anchor {
                        anchors.push(*n);
                    }
                    if let Some(s) = split {
                        splits.push(s.clone());
                    }
                }
            }
        }
    }
    fn walk_seg(s: &CompSegAst, anchors: &mut Vec<i64>, splits: &mut Vec<String>) {
        match s {
            CompSegAst::Atom(a) => walk_atom(a, anchors, splits),
            CompSegAst::Delim(d) => walk_atom(&d.atom, anchors, splits),
        }
    }
    fn walk_x(x: &XSpecAst, anchors: &mut Vec<i64>, splits: &mut Vec<String>) {
        match x {
            XSpecAst::Atom(a) => walk_atom(a, anchors, splits),
            XSpecAst::Delim(d) => walk_atom(&d.atom, anchors, splits),
            XSpecAst::Comp(c) => walk_comp(c, anchors, splits),
        }
    }
    fn walk_comp(c: &CompAst, anchors: &mut Vec<i64>, splits: &mut Vec<String>) {
        walk_seg(&c.first, anchors, splits);
        for (_, s) in &c.rest {
            walk_seg(s, anchors, splits);
        }
    }
    fn walk_cont(c: &ContAst, anchors: &mut Vec<i64>, splits: &mut Vec<String>) {
        match c {
            ContAst::Atom(a) => walk_atom(a, anchors, splits),
            ContAst::Delim(d) => walk_atom(&d.atom, anchors, splits),
            ContAst::Comp(comp) => walk_comp(comp, anchors, splits),
            ContAst::Cond(cond) => {
                walk_x(&cond.positive, anchors, splits);
                walk_x(&cond.negative, anchors, splits);
            }
        }
    }
    fn walk_acts_vec(
        acts: &Option<Vec<PActSpec>>,
        anchors: &mut Vec<i64>,
        splits: &mut Vec<String>,
    ) {
        if let Some(acts) = acts {
            for a in acts {
                if let POp::Rec { anchor, split } = &a.op {
                    if let Some(n) = anchor {
                        anchors.push(*n);
                    }
                    if let Some(s) = split {
                        splits.push(s.clone());
                    }
                }
            }
        }
    }
    fn walk_cell_body(
        b: &Option<CellBodyAst>,
        anchors: &mut Vec<i64>,
        splits: &mut Vec<String>,
    ) {
        if let Some(b) = b {
            walk_acts_vec(&b.actions, anchors, splits);
            if let Some(c) = &b.cont {
                walk_cont(c, anchors, splits);
            }
        }
    }
    fn walk_cell(c: &CellAst, anchors: &mut Vec<i64>, splits: &mut Vec<String>) {
        if let CellAst::Body { body, .. } = c {
            walk_cell_body(body, anchors, splits);
        }
    }
    fn walk_subrow_body(
        b: &SubrowBodyAst,
        anchors: &mut Vec<i64>,
        splits: &mut Vec<String>,
    ) {
        walk_acts_vec(&b.actions, anchors, splits);
        for c in &b.cells {
            walk_cell(c, anchors, splits);
        }
    }
    fn walk_subrow(s: &SubrowAst, anchors: &mut Vec<i64>, splits: &mut Vec<String>) {
        match s {
            SubrowAst::Impl(cells) => {
                for c in cells {
                    walk_cell(c, anchors, splits);
                }
            }
            SubrowAst::Expl { body, .. } => walk_subrow_body(body, anchors, splits),
            SubrowAst::Frag { .. } => {}
        }
    }
    fn walk_row_body(b: &RowBodyAst, anchors: &mut Vec<i64>, splits: &mut Vec<String>) {
        walk_acts_vec(&b.actions, anchors, splits);
        for s in &b.subrows {
            walk_subrow(s, anchors, splits);
        }
    }
    fn walk_row(r: &RowAst, anchors: &mut Vec<i64>, splits: &mut Vec<String>) {
        if let RowAst::Body { body, .. } = r {
            walk_row_body(body, anchors, splits);
        }
    }
    fn walk_subtable_body(
        b: &SubtableBodyAst,
        anchors: &mut Vec<i64>,
        splits: &mut Vec<String>,
    ) {
        walk_acts_vec(&b.actions, anchors, splits);
        for r in &b.rows {
            walk_row(r, anchors, splits);
        }
    }
    fn walk_subtable(s: &SubtableAst, anchors: &mut Vec<i64>, splits: &mut Vec<String>) {
        match s {
            SubtableAst::Impl(rows) => {
                for r in rows {
                    walk_row(r, anchors, splits);
                }
            }
            SubtableAst::Expl { body, .. } => walk_subtable_body(body, anchors, splits),
            SubtableAst::Frag { .. } => {}
        }
    }

    walk_acts_vec(&tree.actions, &mut anchors, &mut splits);
    for (_, frag) in &tree.fragments {
        match frag {
            FragBodyAst::Cell(b) => walk_cell_body(b, &mut anchors, &mut splits),
            FragBodyAst::Row(b) => walk_row_body(b, &mut anchors, &mut splits),
            FragBodyAst::Subrow(b) => walk_subrow_body(b, &mut anchors, &mut splits),
            FragBodyAst::Subtable(b) => walk_subtable_body(b, &mut anchors, &mut splits),
        }
    }
    for st in &tree.subtables {
        walk_subtable(st, &mut anchors, &mut splits);
    }
    (anchors, splits)
}
