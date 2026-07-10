//! Port of `ru.icc.regtab.atp.spec` (+ `interpret` transformations):
//! the ATP specification hierarchy. Java sealed interfaces map to enums.

use crate::recordset::{RecordCore, RecordsetCore, Schema};
use crate::semantics::CellItem;
use crate::syntax::{CellData, SyntaxCore};
use crate::util::{
    full_match, java_is_blank, java_trim, norm_whitespace, replace_all, split_literal, split_regex,
    CoreResult,
};
use std::collections::BTreeSet;
use std::sync::Arc;

/// Sentinel for unbounded cardinality / open range ends
/// (Java: `Integer.MAX_VALUE`).
pub const UNBOUNDED: i64 = i64::MAX;

// ---------------------------------------------------------------- enums

#[pyo3::pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum ItemType {
    #[pyo3(name = "VALUE")]
    Value,
    #[pyo3(name = "ATTRIBUTE")]
    Attribute,
    #[pyo3(name = "AUXILIARY")]
    Auxiliary,
}

#[pyo3::pyclass(eq, eq_int, name = "ItemDerivationDirective")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Idd {
    #[pyo3(name = "VAL")]
    Val,
    #[pyo3(name = "ATTR")]
    Attr,
    #[pyo3(name = "AUX")]
    Aux,
    #[pyo3(name = "SKIP")]
    Skip,
}

impl Idd {
    pub fn to_item_type(self) -> CoreResult<ItemType> {
        match self {
            Idd::Val => Ok(ItemType::Value),
            Idd::Attr => Ok(ItemType::Attribute),
            Idd::Aux => Ok(ItemType::Auxiliary),
            Idd::Skip => Err("SKIP has no corresponding ItemType".into()),
        }
    }
}

#[pyo3::pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum OperationType {
    #[pyo3(name = "FILL")]
    Fill,
    #[pyo3(name = "PREFIX")]
    Prefix,
    #[pyo3(name = "SUFFIX")]
    Suffix,
    #[pyo3(name = "AVP")]
    Avp,
    #[pyo3(name = "REC")]
    Rec,
    #[pyo3(name = "JOIN")]
    Join,
}

#[pyo3::pyclass(eq, eq_int)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TraversalOrder {
    #[pyo3(name = "ROW_MAJOR")]
    RowMajor,
    #[pyo3(name = "REVERSE_ROW_MAJOR")]
    ReverseRowMajor,
    #[pyo3(name = "COLUMN_MAJOR")]
    ColumnMajor,
    #[pyo3(name = "REVERSE_COLUMN_MAJOR")]
    ReverseColumnMajor,
}

/// `CellDerivedProviderKind`.
#[pyo3::pyclass(eq, eq_int, name = "CellDerivedProviderKind")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CellKind {
    #[pyo3(name = "UNRESTRICTED")]
    Unrestricted,
    #[pyo3(name = "VAL")]
    Val,
    #[pyo3(name = "ATTR")]
    Attr,
    #[pyo3(name = "AUX")]
    Aux,
}

/// `ContextDerivedProviderKind`.
#[pyo3::pyclass(eq, eq_int, name = "ContextDerivedProviderKind")]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum CtxKind {
    #[pyo3(name = "UNRESTRICTED")]
    Unrestricted,
    #[pyo3(name = "VAL")]
    Val,
    #[pyo3(name = "ATTR")]
    Attr,
    #[pyo3(name = "AUX")]
    Aux,
}

// ---------------------------------------------------------------- quantifier

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QKind {
    ZeroOrOne,
    One,
    Exactly,
    OneOrMore,
    ZeroOrMore,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Quantifier {
    pub kind: QKind,
    pub n: i64,
}

impl Quantifier {
    pub const ONE: Quantifier = Quantifier { kind: QKind::One, n: 0 };
    pub const ZERO_OR_ONE: Quantifier = Quantifier { kind: QKind::ZeroOrOne, n: 0 };
    pub const ONE_OR_MORE: Quantifier = Quantifier { kind: QKind::OneOrMore, n: 0 };
    pub const ZERO_OR_MORE: Quantifier = Quantifier { kind: QKind::ZeroOrMore, n: 0 };

    pub fn exactly(n: i64) -> CoreResult<Quantifier> {
        if n < 2 {
            return Err(format!("EXACTLY requires n >= 2, got: {n}").into());
        }
        Ok(Quantifier { kind: QKind::Exactly, n })
    }

    pub fn min(&self) -> i64 {
        match self.kind {
            QKind::ZeroOrOne | QKind::ZeroOrMore => 0,
            QKind::One | QKind::OneOrMore => 1,
            QKind::Exactly => self.n,
        }
    }

    pub fn max(&self) -> i64 {
        match self.kind {
            QKind::ZeroOrOne | QKind::One => 1,
            QKind::Exactly => self.n,
            QKind::OneOrMore | QKind::ZeroOrMore => UNBOUNDED,
        }
    }
}

// ---------------------------------------------------------------- Python callbacks

/// A shared Python callable used by Custom/External predicates and extractors.
#[derive(Clone, Debug)]
pub struct PyFunc(pub Arc<pyo3::Py<pyo3::PyAny>>);

impl PartialEq for PyFunc {
    fn eq(&self, other: &Self) -> bool {
        self.0.as_ptr() == other.0.as_ptr()
    }
}

/// Evaluation environment threaded through predicate evaluation.
/// `py_table` (a `pyregtab._core.TableSyntax` handle) is required only when
/// External/Custom Python callbacks participate.
pub struct EvalEnv<'a> {
    pub syntax: &'a SyntaxCore,
    pub py_table: Option<&'a pyo3::Py<pyo3::PyAny>>,
}

// ---------------------------------------------------------------- CellPredicate

#[derive(Clone, PartialEq, Debug)]
pub enum CellPredicate {
    Blank,
    NotBlank,
    Regex(String),
    NotRegex(String),
    Contains(String),
    NotContains(String),
    External { name: String, func: PyFunc },
    Custom { description: String, func: PyFunc },
}

impl CellPredicate {
    pub fn test(&self, cell: &CellData, env: &EvalEnv) -> CoreResult<bool> {
        match self {
            CellPredicate::Blank => Ok(cell.text_blank),
            CellPredicate::NotBlank => Ok(!cell.text_blank),
            CellPredicate::Regex(p) => full_match(p, &cell.text),
            CellPredicate::NotRegex(p) => Ok(!full_match(p, &cell.text)?),
            CellPredicate::Contains(s) => Ok(cell.text.contains(s.as_str())),
            CellPredicate::NotContains(s) => Ok(!cell.text.contains(s.as_str())),
            CellPredicate::External { func, .. } | CellPredicate::Custom { func, .. } => {
                crate::py::call_cell_predicate(func, env, cell.row, cell.col)
            }
        }
    }

    pub fn to_rtl(&self) -> CoreResult<String> {
        Ok(match self {
            CellPredicate::Blank => "BLANK".to_string(),
            CellPredicate::NotBlank => "!BLANK".to_string(),
            CellPredicate::Regex(p) => format!("\"{p}\""),
            CellPredicate::NotRegex(p) => format!("!\"{p}\""),
            CellPredicate::Contains(s) => format!("~\"{s}\""),
            CellPredicate::NotContains(s) => format!("!~\"{s}\""),
            CellPredicate::External { name, .. } => format!("EXT('{name}')"),
            CellPredicate::Custom { description, .. } => {
                return Err(format!("Custom CellPredicate has no RTL analog: {description}").into())
            }
        })
    }
}

// ---------------------------------------------------------------- FilterTerm

#[derive(Clone, PartialEq, Debug)]
pub enum FilterTerm {
    LeftOf,
    RightOf,
    Above,
    Below,
    SameSubrow,
    SameSubcol,
    SameSubtable,
    SameRow,
    SameCol,
    NotSameCell,
    SameCell,
    ColExact(i64),
    ColOffset(i64),
    ColRange(i64, i64),
    ColAbsoluteRange(i64, i64),
    RowExact(i64),
    RowOffset(i64),
    RowAbsoluteRange(i64, i64),
    PosExact(i64),
    PosOffset(i64),
    PosRange(i64, i64),
    Regex(String),
    NotRegex(String),
    Contains(String),
    NotContains(String),
    Blank,
    NotBlank,
    Tagged(String),
    NotTagged(String),
    SameStr,
    External { name: String, func: PyFunc },
    Custom { description: String, func: PyFunc },
}

fn same_subrow(a: &CellItem, c: &CellItem, env: &EvalEnv) -> bool {
    a.row == c.row && env.syntax.subrow_of(a.row, a.col) == env.syntax.subrow_of(c.row, c.col)
}

fn same_subtable(a: &CellItem, c: &CellItem, env: &EvalEnv) -> bool {
    match (env.syntax.subtable_of_row(c.row), env.syntax.subtable_of_row(a.row)) {
        (Some(x), Some(y)) => x == y,
        _ => false,
    }
}

fn same_subcol(a: &CellItem, c: &CellItem, env: &EvalEnv) -> bool {
    same_subtable(a, c, env) && a.col == c.col
}

fn same_cell(a: &CellItem, c: &CellItem) -> bool {
    a.row == c.row && a.col == c.col
}

impl FilterTerm {
    /// κ(anchor, candidate) — port of `FilterTerm.toCondition()` semantics.
    pub fn eval(&self, a: &CellItem, c: &CellItem, env: &EvalEnv) -> CoreResult<bool> {
        let (ar, ac) = (a.row as i64, a.col as i64);
        let (cr, cc) = (c.row as i64, c.col as i64);
        Ok(match self {
            FilterTerm::LeftOf => same_subrow(a, c, env) && cc < ac,
            FilterTerm::RightOf => same_subrow(a, c, env) && cc > ac,
            FilterTerm::Above => same_subcol(a, c, env) && cr < ar,
            FilterTerm::Below => same_subcol(a, c, env) && cr > ar,
            FilterTerm::SameSubrow => same_subrow(a, c, env) && !same_cell(a, c),
            FilterTerm::SameSubcol => same_subcol(a, c, env) && !same_cell(a, c),
            FilterTerm::SameSubtable => same_subtable(a, c, env) && !same_cell(a, c),
            FilterTerm::SameRow => cr == ar && !same_cell(a, c),
            FilterTerm::SameCol => cc == ac && !same_cell(a, c),
            FilterTerm::NotSameCell => !same_cell(a, c),
            FilterTerm::SameCell => same_cell(a, c),
            FilterTerm::ColExact(n) => cc == *n,
            FilterTerm::ColOffset(d) => cc == ac + d,
            FilterTerm::ColRange(from, to) => {
                let lo = ac + from;
                cc >= lo && (*to == UNBOUNDED || cc <= ac + to)
            }
            FilterTerm::ColAbsoluteRange(lo, hi) => cc >= *lo && (*hi == UNBOUNDED || cc <= *hi),
            FilterTerm::RowExact(n) => cr == *n,
            FilterTerm::RowOffset(d) => cr == ar + d,
            FilterTerm::RowAbsoluteRange(lo, hi) => cr >= *lo && (*hi == UNBOUNDED || cr <= *hi),
            FilterTerm::PosExact(n) => c.index as i64 == *n,
            FilterTerm::PosOffset(d) => c.index as i64 == a.index as i64 + d,
            FilterTerm::PosRange(lo, hi) => {
                let ci = c.index as i64;
                ci >= *lo && (*hi == UNBOUNDED || ci <= *hi)
            }
            FilterTerm::Regex(p) => full_match(p, &c.s)?,
            FilterTerm::NotRegex(p) => !full_match(p, &c.s)?,
            FilterTerm::Contains(s) => c.s.contains(s.as_str()),
            FilterTerm::NotContains(s) => !c.s.contains(s.as_str()),
            FilterTerm::Blank => java_is_blank(&c.s),
            FilterTerm::NotBlank => !java_is_blank(&c.s),
            FilterTerm::Tagged(t) => c.tags.iter().any(|x| x == t),
            FilterTerm::NotTagged(t) => !c.tags.iter().any(|x| x == t),
            FilterTerm::SameStr => c.s == a.s,
            FilterTerm::External { func, .. } | FilterTerm::Custom { func, .. } => {
                return crate::py::call_item_filter(func, env, a, c);
            }
        })
    }

    pub fn to_rtl(&self) -> CoreResult<String> {
        fn off(prefix: &str, d: i64) -> String {
            if d >= 0 {
                format!("{prefix}+{d}")
            } else {
                format!("{prefix}{d}")
            }
        }
        fn abs_range(prefix: &str, lo: i64, hi: i64) -> String {
            if hi == UNBOUNDED {
                format!("{prefix}{lo}..")
            } else {
                format!("{prefix}{lo}..{hi}")
            }
        }
        fn quoted_tag(tag: &str) -> String {
            let name = tag.strip_prefix('#').unwrap_or(tag);
            format!("#'{name}'")
        }
        Ok(match self {
            FilterTerm::LeftOf => "LT".into(),
            FilterTerm::RightOf => "RT".into(),
            FilterTerm::Above => "AV".into(),
            FilterTerm::Below => "BW".into(),
            FilterTerm::SameSubrow => "SR".into(),
            FilterTerm::SameSubcol => "SC".into(),
            FilterTerm::SameSubtable => "ST".into(),
            FilterTerm::SameRow => "ROW".into(),
            FilterTerm::SameCol => "COL".into(),
            FilterTerm::NotSameCell => "NCL".into(),
            FilterTerm::SameCell => "CL".into(),
            FilterTerm::ColExact(n) => format!("C{n}"),
            FilterTerm::ColOffset(d) => off("C", *d),
            FilterTerm::ColRange(from, to) => {
                let lo = off("C", *from);
                if *to == UNBOUNDED {
                    format!("{lo}..")
                } else {
                    format!("{lo}..{to}")
                }
            }
            FilterTerm::ColAbsoluteRange(lo, hi) => abs_range("C", *lo, *hi),
            FilterTerm::RowExact(n) => format!("R{n}"),
            FilterTerm::RowOffset(d) => off("R", *d),
            FilterTerm::RowAbsoluteRange(lo, hi) => abs_range("R", *lo, *hi),
            FilterTerm::PosExact(n) => format!("P{n}"),
            FilterTerm::PosOffset(d) => off("P", *d),
            FilterTerm::PosRange(lo, hi) => abs_range("P", *lo, *hi),
            FilterTerm::Regex(p) => format!("\"{p}\""),
            FilterTerm::NotRegex(p) => format!("!\"{p}\""),
            FilterTerm::Contains(s) => format!("~\"{s}\""),
            FilterTerm::NotContains(s) => format!("!~\"{s}\""),
            FilterTerm::Blank => "BLANK".into(),
            FilterTerm::NotBlank => "!BLANK".into(),
            FilterTerm::Tagged(t) => quoted_tag(t),
            FilterTerm::NotTagged(t) => format!("!{}", quoted_tag(t)),
            FilterTerm::SameStr => "STR".into(),
            FilterTerm::External { name, .. } => format!("EXT('{name}')"),
            FilterTerm::Custom { .. } => {
                return Err("Custom constraint has no RTL analog".into());
            }
        })
    }
}

// ---------------------------------------------------------------- ItemFilterConditionSpec

#[derive(Clone, PartialEq, Debug)]
pub enum FilterCond {
    Bare(FilterTerm),
    And(Vec<FilterTerm>),
    /// OR of AND-groups.
    Or(Vec<Vec<FilterTerm>>),
    Custom { description: String, func: PyFunc },
}

impl FilterCond {
    pub fn eval(&self, a: &CellItem, c: &CellItem, env: &EvalEnv) -> CoreResult<bool> {
        match self {
            FilterCond::Bare(t) => t.eval(a, c, env),
            FilterCond::And(terms) => {
                for t in terms {
                    if !t.eval(a, c, env)? {
                        return Ok(false);
                    }
                }
                Ok(true)
            }
            FilterCond::Or(groups) => {
                for g in groups {
                    let mut all = true;
                    for t in g {
                        if !t.eval(a, c, env)? {
                            all = false;
                            break;
                        }
                    }
                    if all {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            FilterCond::Custom { func, .. } => crate::py::call_item_filter(func, env, a, c),
        }
    }

    pub fn to_rtl(&self) -> CoreResult<String> {
        match self {
            FilterCond::Bare(t) => t.to_rtl(),
            FilterCond::And(terms) => {
                let parts: CoreResult<Vec<String>> = terms.iter().map(|t| t.to_rtl()).collect();
                Ok(format!("({})", parts?.join(" & ")))
            }
            FilterCond::Or(groups) => {
                let mut parts = Vec::new();
                for g in groups {
                    let inner: CoreResult<Vec<String>> = g.iter().map(|t| t.to_rtl()).collect();
                    parts.push(format!("({})", inner?.join(" & ")));
                }
                Ok(format!("({})", parts.join(" | ")))
            }
            FilterCond::Custom { description, .. } => Err(format!(
                "Custom ItemFilterConditionSpec has no RTL analog: {description}"
            )
            .into()),
        }
    }
}

// ---------------------------------------------------------------- StringExtractor

#[derive(Clone, PartialEq, Debug)]
pub enum Extractor {
    Verbatim,
    Replaced(String, String),
    WhitespaceNormalized,
    Trimmed,
    /// begin/end in code points (Java uses UTF-16 units; identical on BMP text).
    Substring(i64, i64),
    UpperCase,
    LowerCase,
    Chain(Vec<Extractor>),
    Custom { description: String, func: PyFunc },
}

impl Extractor {
    pub fn apply(&self, input: &str) -> CoreResult<String> {
        Ok(match self {
            Extractor::Verbatim => input.to_string(),
            Extractor::Replaced(rx, rep) => replace_all(rx, rep, input)?,
            Extractor::WhitespaceNormalized => norm_whitespace(input),
            Extractor::Trimmed => java_trim(input).to_string(),
            Extractor::Substring(begin, end) => {
                let chars: Vec<char> = input.chars().collect();
                let len = chars.len() as i64;
                let b = (*begin).clamp(0, len) as usize;
                let e = (*end).min(len).max(*begin) as usize;
                chars[b..e].iter().collect()
            }
            Extractor::UpperCase => input.to_uppercase(),
            Extractor::LowerCase => input.to_lowercase(),
            Extractor::Chain(steps) => {
                let mut cur = input.to_string();
                for s in steps {
                    cur = s.apply(&cur)?;
                }
                cur
            }
            Extractor::Custom { func, .. } => crate::py::call_extractor(func, input)?,
        })
    }

    pub fn to_rtl(&self) -> CoreResult<String> {
        Ok(match self {
            Extractor::Verbatim => String::new(),
            Extractor::Replaced(rx, rep) => format!(
                "REPL(\"{}\",\"{}\")",
                rx.replace('"', "\"\""),
                rep.replace('"', "\"\"")
            ),
            Extractor::WhitespaceNormalized => "NORM".into(),
            Extractor::Trimmed => "TRIM".into(),
            Extractor::Substring(b, e) => format!("SUBSTR({},{})", b, e - b),
            Extractor::UpperCase => "UC".into(),
            Extractor::LowerCase => "LC".into(),
            Extractor::Chain(steps) => {
                let parts: CoreResult<Vec<String>> = steps.iter().map(|s| s.to_rtl()).collect();
                parts?.join(".")
            }
            Extractor::Custom { description, .. } => {
                return Err(format!("Custom StringExtractor has no RTL analog: {description}").into())
            }
        })
    }
}

// ---------------------------------------------------------------- ProviderSpec

#[derive(Clone, PartialEq, Debug)]
pub struct CtxLiteral {
    pub text: String,
    pub ty: ItemType,
    pub const_value: Option<String>,
}

impl CtxLiteral {
    pub fn kind(&self) -> CtxKind {
        match self.ty {
            ItemType::Value => CtxKind::Val,
            ItemType::Attribute => CtxKind::Attr,
            ItemType::Auxiliary => CtxKind::Aux,
        }
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct ProviderSpec {
    pub cardinality: i64,
    pub traversal_order: TraversalOrder,
    pub filter_condition: Option<FilterCond>,
    pub target_item_kind: Option<CellKind>,
    pub context_literal: Option<CtxLiteral>,
}

impl ProviderSpec {
    pub fn new(
        cardinality: i64,
        traversal_order: TraversalOrder,
        filter_condition: Option<FilterCond>,
        target_item_kind: Option<CellKind>,
        context_literal: Option<CtxLiteral>,
    ) -> CoreResult<Self> {
        if cardinality < 0 {
            return Err(format!("cardinality must be non-negative: {cardinality}").into());
        }
        if context_literal.is_none() {
            if filter_condition.is_none() {
                return Err("filterCondition".into());
            }
            let kind = target_item_kind.ok_or("targetItemKind")?;
            if kind == CellKind::Attr && cardinality != 1 {
                return Err(
                    format!("ATTR provider requires cardinality = 1, got: {cardinality}").into(),
                );
            }
        }
        Ok(ProviderSpec {
            cardinality,
            traversal_order,
            filter_condition,
            target_item_kind,
            context_literal,
        })
    }

    pub fn is_context_literal(&self) -> bool {
        self.context_literal.is_some()
    }

    pub fn cell(
        kind: CellKind,
        cardinality: i64,
        order: TraversalOrder,
        cond: FilterCond,
    ) -> CoreResult<Self> {
        ProviderSpec::new(cardinality, order, Some(cond), Some(kind), None)
    }

    pub fn ctx(text: String, ty: ItemType) -> Self {
        ProviderSpec {
            cardinality: 1,
            traversal_order: TraversalOrder::RowMajor,
            filter_condition: None,
            target_item_kind: None,
            context_literal: Some(CtxLiteral { text, ty, const_value: None }),
        }
    }

    pub fn ctx_avp(attr_name: String, value: String) -> Self {
        ProviderSpec {
            cardinality: 1,
            traversal_order: TraversalOrder::RowMajor,
            filter_condition: None,
            target_item_kind: None,
            context_literal: Some(CtxLiteral {
                text: attr_name,
                ty: ItemType::Attribute,
                const_value: Some(value),
            }),
        }
    }
}

// ---------------------------------------------------------------- ActionSpec

#[derive(Clone, PartialEq, Debug)]
pub struct ActionSpec {
    pub operation_type: OperationType,
    pub delimiter: Option<String>,
    pub providers: Vec<ProviderSpec>,
    pub anchor_pos: Option<i64>,
    pub split_delimiter: Option<String>,
    pub key_positions: BTreeSet<i64>,
    pub inherited: bool,
}

impl ActionSpec {
    pub fn new(
        operation_type: OperationType,
        delimiter: Option<String>,
        providers: Vec<ProviderSpec>,
        anchor_pos: Option<i64>,
        split_delimiter: Option<String>,
        key_positions: BTreeSet<i64>,
        inherited: bool,
    ) -> CoreResult<Self> {
        for p in &providers {
            if let Some(ctx) = &p.context_literal {
                let is_const_avp = ctx.const_value.is_some();
                if operation_type == OperationType::Join {
                    return Err("JOIN action does not allow context literals".into());
                }
                if operation_type == OperationType::Rec && !is_const_avp && ctx.ty != ItemType::Value {
                    return Err(format!(
                        "REC action requires a VALUE context literal, got {:?}",
                        ctx.ty
                    )
                    .into());
                }
                if operation_type == OperationType::Avp && ctx.ty != ItemType::Attribute {
                    return Err(format!(
                        "AVP action requires an ATTRIBUTE context literal, got {:?}",
                        ctx.ty
                    )
                    .into());
                }
            } else {
                let kind = p.target_item_kind.unwrap_or(CellKind::Unrestricted);
                if (operation_type == OperationType::Rec || operation_type == OperationType::Join)
                    && kind != CellKind::Val
                {
                    return Err(format!(
                        "{operation_type:?} action requires a VAL provider, got {kind:?}"
                    )
                    .into());
                }
                if operation_type == OperationType::Avp && kind != CellKind::Attr {
                    return Err(
                        format!("AVP action requires an ATTR provider, got {kind:?}").into()
                    );
                }
            }
        }
        Ok(ActionSpec {
            operation_type,
            delimiter,
            providers,
            anchor_pos,
            split_delimiter,
            key_positions,
            inherited,
        })
    }

    pub fn as_inherited(&self) -> ActionSpec {
        if self.inherited {
            self.clone()
        } else {
            let mut c = self.clone();
            c.inherited = true;
            c
        }
    }
}

// ---------------------------------------------------------------- ContentSpec

#[derive(Clone, PartialEq, Debug)]
pub struct AtomicSpec {
    pub idd: Idd,
    pub extractor: Option<Extractor>,
    pub tags: Vec<String>,
    pub actions: Vec<ActionSpec>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct DelimitedSpec {
    pub delimiter: String,
    pub atom: AtomicSpec,
}

impl DelimitedSpec {
    pub fn new(delimiter: String, atom: AtomicSpec) -> CoreResult<Self> {
        if delimiter.is_empty() {
            return Err("delimiter must be non-empty".into());
        }
        Ok(DelimitedSpec { delimiter, atom })
    }
}

/// One (δᵢ, S_xⁱ) pair; spec is Atomic or Delimited (enforced by builders).
#[derive(Clone, PartialEq, Debug)]
pub struct CompoundSegment {
    pub leading_delimiter: String,
    pub spec: ContentSpec,
}

impl CompoundSegment {
    pub fn new(leading_delimiter: String, spec: ContentSpec) -> CoreResult<Self> {
        match &spec {
            ContentSpec::Atomic(_) | ContentSpec::Delimited(_) => {}
            _ => {
                return Err(
                    "Compound segment spec must be AtomicContentSpec or DelimitedContentSpec".into(),
                )
            }
        }
        Ok(CompoundSegment { leading_delimiter, spec })
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct CompoundSpec {
    pub segments: Vec<CompoundSegment>,
    pub trailing_delimiter: String,
}

impl CompoundSpec {
    pub fn new(segments: Vec<CompoundSegment>, trailing_delimiter: String) -> CoreResult<Self> {
        if segments.is_empty() {
            return Err("At least one segment is required".into());
        }
        Ok(CompoundSpec { segments, trailing_delimiter })
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct ConditionalSpec {
    pub condition: CellPredicate,
    pub positive: ContentSpec,
    pub negative: ContentSpec,
}

#[derive(Clone, PartialEq, Debug)]
pub enum ContentSpec {
    Atomic(AtomicSpec),
    Delimited(DelimitedSpec),
    Compound(CompoundSpec),
    Conditional(Box<ConditionalSpec>),
}

// ---------------------------------------------------------------- patterns

#[derive(Clone, PartialEq, Debug)]
pub struct CellPattern {
    pub condition: Option<CellPredicate>,
    pub quantifier: Quantifier,
    pub content_spec: Option<ContentSpec>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct SubrowPattern {
    pub condition: Option<CellPredicate>,
    pub quantifier: Quantifier,
    pub cell_patterns: Vec<Arc<CellPattern>>,
}

impl SubrowPattern {
    pub fn new(
        condition: Option<CellPredicate>,
        quantifier: Quantifier,
        cell_patterns: Vec<Arc<CellPattern>>,
    ) -> CoreResult<Self> {
        if cell_patterns.is_empty() {
            return Err("At least one cell pattern is required".into());
        }
        Ok(SubrowPattern { condition, quantifier, cell_patterns })
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct RowPattern {
    pub condition: Option<CellPredicate>,
    pub quantifier: Quantifier,
    pub subrow_patterns: Vec<Arc<SubrowPattern>>,
}

impl RowPattern {
    pub fn new(
        condition: Option<CellPredicate>,
        quantifier: Quantifier,
        subrow_patterns: Vec<Arc<SubrowPattern>>,
    ) -> CoreResult<Self> {
        if subrow_patterns.is_empty() {
            return Err("At least one subrow pattern is required".into());
        }
        Ok(RowPattern { condition, quantifier, subrow_patterns })
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct SubtablePattern {
    pub condition: Option<CellPredicate>,
    pub quantifier: Quantifier,
    pub row_patterns: Vec<Arc<RowPattern>>,
}

impl SubtablePattern {
    pub fn new(
        condition: Option<CellPredicate>,
        quantifier: Quantifier,
        row_patterns: Vec<Arc<RowPattern>>,
    ) -> CoreResult<Self> {
        if row_patterns.is_empty() {
            return Err("At least one row pattern is required".into());
        }
        Ok(SubtablePattern { condition, quantifier, row_patterns })
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct TablePattern {
    pub condition: Option<CellPredicate>,
    pub subtable_patterns: Vec<Arc<SubtablePattern>>,
    pub transformations: Vec<Transformation>,
}

impl TablePattern {
    pub fn new(
        condition: Option<CellPredicate>,
        subtable_patterns: Vec<Arc<SubtablePattern>>,
        transformations: Vec<Transformation>,
    ) -> CoreResult<Self> {
        if subtable_patterns.is_empty() {
            return Err("At least one subtable pattern is required".into());
        }
        Ok(TablePattern { condition, subtable_patterns, transformations })
    }

    /// `TablePattern.of(...)`: collects inline REC params into transformations.
    pub fn of(subtables: Vec<Arc<SubtablePattern>>) -> CoreResult<Self> {
        let transforms = extract_inline_transformations(&subtables)?;
        TablePattern::new(None, subtables, transforms)
    }

    /// Applies all transformations in order (each with the `$a_%i` template).
    pub fn transform(&self, mut rs: RecordsetCore) -> CoreResult<RecordsetCore> {
        for t in &self.transformations {
            rs = t.with_template("$a_%i").apply(rs)?;
        }
        Ok(rs)
    }
}

// ------------------------------------------------ Python-callback detection

impl CellPredicate {
    pub fn has_py(&self) -> bool {
        matches!(self, CellPredicate::External { .. } | CellPredicate::Custom { .. })
    }
}

impl FilterCond {
    pub fn has_py(&self) -> bool {
        let term = |t: &FilterTerm| {
            matches!(t, FilterTerm::External { .. } | FilterTerm::Custom { .. })
        };
        match self {
            FilterCond::Bare(t) => term(t),
            FilterCond::And(ts) => ts.iter().any(term),
            FilterCond::Or(gs) => gs.iter().any(|g| g.iter().any(term)),
            FilterCond::Custom { .. } => true,
        }
    }
}

impl Extractor {
    pub fn has_py(&self) -> bool {
        match self {
            Extractor::Custom { .. } => true,
            Extractor::Chain(steps) => steps.iter().any(|s| s.has_py()),
            _ => false,
        }
    }
}

impl ActionSpec {
    pub fn has_py(&self) -> bool {
        self.providers.iter().any(|p| {
            p.filter_condition.as_ref().map(|c| c.has_py()).unwrap_or(false)
        })
    }
}

impl AtomicSpec {
    pub fn has_py(&self) -> bool {
        self.extractor.as_ref().map(|x| x.has_py()).unwrap_or(false)
            || self.actions.iter().any(|a| a.has_py())
    }
}

impl ContentSpec {
    pub fn has_py(&self) -> bool {
        match self {
            ContentSpec::Atomic(a) => a.has_py(),
            ContentSpec::Delimited(d) => d.atom.has_py(),
            ContentSpec::Compound(c) => c.segments.iter().any(|s| s.spec.has_py()),
            ContentSpec::Conditional(c) => {
                c.condition.has_py() || c.positive.has_py() || c.negative.has_py()
            }
        }
    }
}

impl TablePattern {
    /// True if evaluating this pattern may call back into Python
    /// (External/Custom predicates or extractors) — in that case the GIL
    /// must be held during matching.
    pub fn has_py_callbacks(&self) -> bool {
        let cond = |c: &Option<CellPredicate>| c.as_ref().map(|c| c.has_py()).unwrap_or(false);
        if cond(&self.condition) {
            return true;
        }
        for st in &self.subtable_patterns {
            if cond(&st.condition) {
                return true;
            }
            for row in &st.row_patterns {
                if cond(&row.condition) {
                    return true;
                }
                for sr in &row.subrow_patterns {
                    if cond(&sr.condition) {
                        return true;
                    }
                    for cell in &sr.cell_patterns {
                        if cond(&cell.condition) {
                            return true;
                        }
                        if cell.content_spec.as_ref().map(|c| c.has_py()).unwrap_or(false) {
                            return true;
                        }
                    }
                }
            }
        }
        false
    }
}

fn actions_of(cs: &ContentSpec, out: &mut Vec<ActionSpec>) {
    match cs {
        ContentSpec::Atomic(a) => out.extend(a.actions.iter().cloned()),
        ContentSpec::Delimited(d) => out.extend(d.atom.actions.iter().cloned()),
        ContentSpec::Conditional(c) => {
            actions_of(&c.positive, out);
            actions_of(&c.negative, out);
        }
        ContentSpec::Compound(c) => {
            for seg in &c.segments {
                actions_of(&seg.spec, out);
            }
        }
    }
}

pub fn extract_inline_transformations(
    subtables: &[Arc<SubtablePattern>],
) -> CoreResult<Vec<Transformation>> {
    let mut anchor_pos: Option<i64> = None;
    let mut split_delimiter: Option<String> = None;
    for st in subtables {
        for row in &st.row_patterns {
            for subrow in &row.subrow_patterns {
                for cell in &subrow.cell_patterns {
                    let Some(cs) = &cell.content_spec else { continue };
                    let mut actions = Vec::new();
                    actions_of(cs, &mut actions);
                    for a in &actions {
                        if let Some(ap) = a.anchor_pos {
                            if let Some(prev) = anchor_pos {
                                if prev != ap {
                                    return Err(format!(
                                        "Conflicting inline anchorPos: {prev} vs {ap}"
                                    )
                                    .into());
                                }
                            }
                            anchor_pos = Some(ap);
                        }
                        if let Some(sd) = &a.split_delimiter {
                            if let Some(prev) = &split_delimiter {
                                if prev != sd {
                                    return Err(format!(
                                        "Conflicting inline splitDelimiter: \"{prev}\" vs \"{sd}\""
                                    )
                                    .into());
                                }
                            }
                            split_delimiter = Some(sd.clone());
                        }
                    }
                }
            }
        }
    }
    let mut result = Vec::new();
    if let Some(p) = anchor_pos {
        result.push(Transformation::AnchorAttributeAtPosition(p));
    }
    if let Some(d) = split_delimiter {
        result.push(Transformation::DelimitedFieldSplit {
            delimiter: d,
            only_attributes: None,
            template: "$a_%i".to_string(),
        });
    }
    Ok(result)
}

// ---------------------------------------------------------------- transformations

#[derive(Clone, PartialEq, Debug)]
pub enum Transformation {
    AnchorAttributeAtPosition(i64),
    WhitespaceNormalization,
    DelimitedFieldSplit {
        delimiter: String,
        only_attributes: Option<BTreeSet<String>>,
        template: String,
    },
    FieldSplitting {
        attribute: String,
        delimiter: String,
        part_attribute_names: Vec<String>,
    },
    SchemaReordering(Vec<String>),
}

impl Transformation {
    pub fn with_template(&self, template: &str) -> Transformation {
        match self {
            Transformation::DelimitedFieldSplit { delimiter, only_attributes, .. } => {
                Transformation::DelimitedFieldSplit {
                    delimiter: delimiter.clone(),
                    only_attributes: only_attributes.clone(),
                    template: template.to_string(),
                }
            }
            other => other.clone(),
        }
    }

    pub fn apply(&self, rs: RecordsetCore) -> CoreResult<RecordsetCore> {
        match self {
            Transformation::AnchorAttributeAtPosition(position) => {
                apply_anchor_at_position(rs, *position)
            }
            Transformation::WhitespaceNormalization => {
                let records = rs
                    .records
                    .into_iter()
                    .map(|r| RecordCore {
                        values: r
                            .values
                            .into_iter()
                            .map(|v| v.map(|s| norm_whitespace(&s)))
                            .collect(),
                    })
                    .collect();
                Ok(RecordsetCore { schema: rs.schema, records })
            }
            Transformation::DelimitedFieldSplit { delimiter, only_attributes, template } => {
                apply_delimited_field_split(rs, delimiter, only_attributes.as_ref(), template)
            }
            Transformation::FieldSplitting { attribute, delimiter, part_attribute_names } => {
                apply_field_splitting(rs, attribute, delimiter, part_attribute_names)
            }
            Transformation::SchemaReordering(order) => apply_schema_reordering(rs, order),
        }
    }
}

fn apply_anchor_at_position(rs: RecordsetCore, position: i64) -> CoreResult<RecordsetCore> {
    if position < 0 {
        return Err(format!("position must be non-negative: {position}").into());
    }
    let attrs = &rs.schema.attributes;
    let n = attrs.len();
    let position = position as usize;
    if n <= 1 || position >= n || position == 0 {
        return Ok(rs);
    }
    // reordered = rest[0..position] + anchor + rest[position..]
    let mut reordered: Vec<usize> = Vec::with_capacity(n);
    for i in 0..position {
        reordered.push(i + 1);
    }
    reordered.push(0);
    for i in position..(n - 1) {
        reordered.push(i + 1);
    }
    let records = rs
        .records
        .iter()
        .map(|r| RecordCore {
            values: reordered.iter().map(|&src| r.values[src].clone()).collect(),
        })
        .collect();
    Ok(RecordsetCore { schema: rs.schema, records })
}

fn anonymous_attribute(template: &str, index: usize) -> String {
    template.replace("%i", &index.to_string())
}

fn apply_delimited_field_split(
    rs: RecordsetCore,
    delimiter: &str,
    only: Option<&BTreeSet<String>>,
    template: &str,
) -> CoreResult<RecordsetCore> {
    if delimiter.is_empty() {
        return Err("delimiter must be non-empty".into());
    }
    if !template.contains("%i") {
        return Err(format!(
            "Anonymous attribute template must contain the placeholder %i: {template}"
        )
        .into());
    }
    let only = only.filter(|s| !s.is_empty());
    let attrs = rs.schema.attributes.clone();
    if attrs.is_empty() {
        return Ok(rs);
    }
    let n = attrs.len();
    let mut width = vec![1usize; n];
    let mut any_split = false;
    for i in 0..n {
        if let Some(set) = only {
            if !set.contains(&attrs[i]) {
                continue;
            }
        }
        let mut max_parts = 1;
        for r in &rs.records {
            let val = r.values[i].as_deref().unwrap_or("");
            max_parts = max_parts.max(split_literal(delimiter, val).len());
        }
        width[i] = max_parts;
        if max_parts > 1 {
            any_split = true;
        }
    }
    if !any_split {
        return Ok(rs);
    }
    let total: usize = width.iter().sum();
    let new_attrs: Vec<String> = (0..total)
        .map(|j| anonymous_attribute(template, j + 1))
        .collect();
    let schema = Schema::new(new_attrs)?;
    let mut out = Vec::with_capacity(rs.records.len());
    for r in &rs.records {
        let mut cells: Vec<Option<String>> = Vec::with_capacity(total);
        for (i, &w) in width.iter().enumerate().take(n) {
            let val = r.values[i].clone().unwrap_or_default();
            if w == 1 {
                cells.push(Some(val));
            } else {
                let parts = split_literal(delimiter, &val);
                for p in 0..w {
                    cells.push(Some(parts.get(p).cloned().unwrap_or_default()));
                }
            }
        }
        out.push(RecordCore { values: cells });
    }
    Ok(RecordsetCore { schema, records: out })
}

fn apply_field_splitting(
    rs: RecordsetCore,
    attribute: &str,
    delimiter: &str,
    part_names_cfg: &[String],
) -> CoreResult<RecordsetCore> {
    let Some(idx) = rs.schema.index_of(attribute) else {
        return Ok(rs);
    };
    let mut max_parts = 1;
    for r in &rs.records {
        if let Some(val) = &r.values[idx] {
            max_parts = max_parts.max(split_regex(delimiter, val)?.len());
        }
    }
    if max_parts <= 1 {
        return Ok(rs);
    }
    if !part_names_cfg.is_empty() && part_names_cfg.len() != max_parts {
        return Err(format!(
            "partAttributeNames size ({}) must equal split part count ({})",
            part_names_cfg.len(),
            max_parts
        )
        .into());
    }
    let part_names: Vec<String> = (0..max_parts)
        .map(|p| {
            if !part_names_cfg.is_empty() {
                part_names_cfg[p].clone()
            } else {
                format!("{}_{}", attribute, p + 1)
            }
        })
        .collect();
    let mut new_attrs = Vec::new();
    for (i, a) in rs.schema.attributes.iter().enumerate() {
        if i == idx {
            new_attrs.extend(part_names.iter().cloned());
        } else {
            new_attrs.push(a.clone());
        }
    }
    let schema = Schema::new(new_attrs)?;
    let mut records = Vec::with_capacity(rs.records.len());
    for r in &rs.records {
        let mut values = Vec::with_capacity(schema.attributes.len());
        for (i, v) in r.values.iter().enumerate() {
            if i == idx {
                let parts: Vec<String> = match v {
                    Some(val) => split_regex(delimiter, val)?,
                    None => Vec::new(),
                };
                for p in 0..max_parts {
                    values.push(parts.get(p).cloned());
                }
            } else {
                values.push(v.clone());
            }
        }
        records.push(RecordCore { values });
    }
    Ok(RecordsetCore { schema, records })
}

fn apply_schema_reordering(rs: RecordsetCore, order: &[String]) -> CoreResult<RecordsetCore> {
    let mut new_attrs: Vec<String> = Vec::new();
    for a in order {
        if rs.schema.contains(a) && !new_attrs.contains(a) {
            new_attrs.push(a.clone());
        }
    }
    for a in &rs.schema.attributes {
        if !new_attrs.contains(a) {
            new_attrs.push(a.clone());
        }
    }
    let indices: Vec<usize> = new_attrs
        .iter()
        .map(|a| rs.schema.index_of(a).unwrap())
        .collect();
    let schema = Schema::new(new_attrs)?;
    let records = rs
        .records
        .iter()
        .map(|r| RecordCore {
            values: indices.iter().map(|&i| r.values[i].clone()).collect(),
        })
        .collect();
    Ok(RecordsetCore { schema, records })
}
