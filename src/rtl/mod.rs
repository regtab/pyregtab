//! Port of `ru.icc.regtab.rtl`: hand-written lexer + recursive-descent parser
//! (structurally following `grammar/RTL.g4`), ATP builder and serializer.

pub mod ast;
pub mod build;
pub mod lexer;
pub mod parser;
pub mod serialize;

use crate::spec::{PyFunc, TablePattern, Transformation};
use crate::util::CoreErr;
use indexmap::IndexMap;

/// Compile error with source position (line 1-based, column 0-based; -1 unknown).
#[derive(Debug)]
pub struct RtlErr {
    pub msg: String,
    pub line: i64,
    pub col: i64,
}

impl RtlErr {
    pub fn new(msg: impl Into<String>) -> Self {
        RtlErr { msg: msg.into(), line: -1, col: -1 }
    }
    pub fn at(msg: impl Into<String>, line: i64, col: i64) -> Self {
        RtlErr { msg: msg.into(), line, col }
    }
}

impl From<CoreErr> for RtlErr {
    fn from(e: CoreErr) -> Self {
        match e {
            CoreErr::Msg(m) => RtlErr::new(m),
            CoreErr::Py(e) => RtlErr::new(format!("{e}")),
        }
    }
}

/// Named Python predicates referenced from RTL via `EXT('name')`.
#[derive(Default, Clone)]
pub struct BindingsCore {
    pub cell: IndexMap<String, PyFunc>,
    pub filter: IndexMap<String, PyFunc>,
}

/// Port of `RtlCompiler.compile(rtl, bindings)`.
pub fn compile(rtl: &str, bindings: &BindingsCore) -> Result<TablePattern, RtlErr> {
    let tokens = lexer::lex(rtl)?;
    let tree = parser::parse(tokens)?;

    // Inline REC params (RtlCompiler.extractInlineRecParams).
    let (anchors, splits) = ast::collect_rec_params(&tree);
    let mut uniq_anchors = anchors.clone();
    uniq_anchors.dedup();
    uniq_anchors.sort_unstable();
    uniq_anchors.dedup();
    if uniq_anchors.len() > 1 {
        return Err(RtlErr::new(format!(
            "Conflicting REC(n) anchor positions: {anchors:?}"
        )));
    }
    let mut uniq_splits = splits.clone();
    uniq_splits.sort();
    uniq_splits.dedup();
    if uniq_splits.len() > 1 {
        return Err(RtlErr::new(format!(
            "Conflicting REC('s') split delimiters: {splits:?}"
        )));
    }
    let inline_anchor = anchors.first().copied();
    let inline_split = splits.first().cloned();

    // buildTransformations(settings, inline)
    let mut transforms: Vec<Transformation> = Vec::new();
    let mut setting_anchor: Option<i64> = None;
    let mut setting_split: Option<String> = None;
    if let Some(settings) = &tree.settings {
        for s in settings {
            match s {
                ast::PSetting::Norm => transforms.push(Transformation::WhitespaceNormalization),
                ast::PSetting::Anch(n) => {
                    setting_anchor = Some(*n);
                    transforms.push(Transformation::AnchorAttributeAtPosition(*n));
                }
                ast::PSetting::Split(d) => {
                    setting_split = Some(d.clone());
                    transforms.push(Transformation::DelimitedFieldSplit {
                        delimiter: d.clone(),
                        only_attributes: None,
                        template: "$a_%i".to_string(),
                    });
                }
            }
        }
    }
    if let Some(ia) = inline_anchor {
        if let Some(sa) = setting_anchor {
            if sa != ia {
                return Err(RtlErr::new(format!(
                    "Conflicting ANCH({sa}) and REC({ia})"
                )));
            }
        } else {
            transforms.push(Transformation::AnchorAttributeAtPosition(ia));
        }
    }
    if let Some(is) = &inline_split {
        if let Some(ss) = &setting_split {
            if ss != is {
                return Err(RtlErr::new(format!(
                    "Conflicting SPLIT(\"{ss}\") and REC('{is}')"
                )));
            }
        } else {
            transforms.push(Transformation::DelimitedFieldSplit {
                delimiter: is.clone(),
                only_attributes: None,
                template: "$a_%i".to_string(),
            });
        }
    }

    let pattern = build::ATPBuilder::new(bindings).build(&tree)?;
    if transforms.is_empty() {
        Ok(pattern)
    } else {
        Ok(TablePattern {
            condition: pattern.condition,
            subtable_patterns: pattern.subtable_patterns,
            transformations: transforms,
        })
    }
}
