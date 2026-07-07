//! Port of `AtpToRtlSerializer`.

use crate::spec::*;
use crate::util::CoreResult;

fn serialize_quantifier(q: &Quantifier) -> String {
    match q.kind {
        QKind::One => String::new(),
        QKind::ZeroOrOne => "?".to_string(),
        QKind::OneOrMore => "+".to_string(),
        QKind::ZeroOrMore => "*".to_string(),
        QKind::Exactly => format!("{{{}}}", q.n),
    }
}

fn esc_dq(s: &str) -> String {
    s.replace('"', "\"\"")
}
fn esc_sq(s: &str) -> String {
    s.replace('\'', "''")
}

pub fn serialize(pattern: &TablePattern) -> CoreResult<String> {
    let settings = serialize_settings(&pattern.transformations)?;
    let cond = match &pattern.condition {
        Some(c) => format!("{}? ", c.to_rtl()?),
        None => String::new(),
    };
    let subtables = &pattern.subtable_patterns;
    let body = if subtables.len() == 1 && is_implicit_subtable(&subtables[0]) {
        serialize_implicit_subtable(&subtables[0])?
    } else {
        let parts: CoreResult<Vec<String>> = subtables
            .iter()
            .map(|sp| serialize_explicit_subtable(sp))
            .collect();
        parts?.join(" ")
    };
    Ok(format!("{settings}{cond}{body}"))
}

fn serialize_settings(transformations: &[Transformation]) -> CoreResult<String> {
    if transformations.is_empty() {
        return Ok(String::new());
    }
    let mut parts = Vec::new();
    for t in transformations {
        parts.push(match t {
            Transformation::AnchorAttributeAtPosition(p) => format!("ANCH({p})"),
            Transformation::WhitespaceNormalization => "NORM".to_string(),
            Transformation::DelimitedFieldSplit { delimiter, .. } => {
                format!("SPLIT(\"{}\")", esc_dq(delimiter))
            }
            other => {
                return Err(format!("Cannot serialize transformation: {other:?}").into());
            }
        });
    }
    Ok(format!("<{}> ", parts.join(", ")))
}

fn is_implicit_subtable(sp: &SubtablePattern) -> bool {
    sp.condition.is_none() && sp.quantifier.kind == QKind::One
}

fn serialize_implicit_subtable(sp: &SubtablePattern) -> CoreResult<String> {
    let parts: CoreResult<Vec<String>> = sp.row_patterns.iter().map(|r| serialize_row(r)).collect();
    Ok(parts?.join(" "))
}

fn serialize_explicit_subtable(sp: &SubtablePattern) -> CoreResult<String> {
    let mut sb = String::from("{ ");
    if let Some(c) = &sp.condition {
        sb.push_str(&c.to_rtl()?);
        sb.push_str("? ");
    }
    for rp in &sp.row_patterns {
        sb.push_str(&serialize_row(rp)?);
        sb.push(' ');
    }
    sb.push('}');
    sb.push_str(&serialize_quantifier(&sp.quantifier));
    Ok(sb)
}

fn serialize_row(rp: &RowPattern) -> CoreResult<String> {
    let mut sb = String::from("[ ");
    if let Some(c) = &rp.condition {
        sb.push_str(&c.to_rtl()?);
        sb.push_str("? ");
    }
    for sr in &rp.subrow_patterns {
        sb.push_str(&serialize_subrow(sr)?);
        sb.push(' ');
    }
    sb.push(']');
    sb.push_str(&serialize_quantifier(&rp.quantifier));
    Ok(sb)
}

fn serialize_subrow(sr: &SubrowPattern) -> CoreResult<String> {
    if sr.condition.is_none() && sr.quantifier.kind == QKind::One {
        let parts: CoreResult<Vec<String>> =
            sr.cell_patterns.iter().map(|c| serialize_cell(c)).collect();
        return Ok(parts?.join(" "));
    }
    let mut sb = String::from("{ ");
    if let Some(c) = &sr.condition {
        sb.push_str(&c.to_rtl()?);
        sb.push_str("? ");
    }
    for cp in &sr.cell_patterns {
        sb.push_str(&serialize_cell(cp)?);
        sb.push(' ');
    }
    sb.push('}');
    sb.push_str(&serialize_quantifier(&sr.quantifier));
    Ok(sb)
}

fn serialize_cell(cp: &CellPattern) -> CoreResult<String> {
    let mut sb = String::from("[");
    let has_body = cp.condition.is_some() || cp.content_spec.is_some();
    if has_body {
        sb.push(' ');
        if let Some(c) = &cp.condition {
            sb.push_str(&c.to_rtl()?);
            if cp.content_spec.is_some() {
                sb.push_str("? ");
            } else {
                sb.push(' ');
            }
        }
        if let Some(cs) = &cp.content_spec {
            sb.push_str(&serialize_content_spec(cs)?);
            sb.push(' ');
        }
    }
    sb.push(']');
    sb.push_str(&serialize_quantifier(&cp.quantifier));
    Ok(sb)
}

fn serialize_content_spec(cs: &ContentSpec) -> CoreResult<String> {
    match cs {
        ContentSpec::Atomic(a) => serialize_atomic(a),
        ContentSpec::Delimited(d) => serialize_delimited(d),
        ContentSpec::Compound(c) => serialize_compound(c),
        ContentSpec::Conditional(c) => serialize_conditional(c),
    }
}

fn serialize_atomic(a: &AtomicSpec) -> CoreResult<String> {
    let mut sb = String::from(match a.idd {
        Idd::Val => "VAL",
        Idd::Attr => "ATTR",
        Idd::Aux => "AUX",
        Idd::Skip => "SKIP",
    });
    if !a.tags.is_empty() {
        let tags: Vec<String> = a
            .tags
            .iter()
            .map(|t| {
                let name = t.strip_prefix('#').unwrap_or(t);
                format!("#'{}'", esc_sq(name))
            })
            .collect();
        sb.push(' ');
        sb.push_str(&tags.join(" "));
    }
    if let Some(x) = &a.extractor {
        if !matches!(x, Extractor::Verbatim) {
            sb.push_str(" = ");
            sb.push_str(&x.to_rtl()?);
        }
    }
    if !a.actions.is_empty() {
        sb.push_str(" : ");
        sb.push_str(&serialize_act_specs(&a.actions)?);
    }
    Ok(sb)
}

fn serialize_delimited(d: &DelimitedSpec) -> CoreResult<String> {
    Ok(format!(
        "({}){{\"{}\"}}",
        serialize_atomic(&d.atom)?,
        esc_dq(&d.delimiter)
    ))
}

fn serialize_compound(c: &CompoundSpec) -> CoreResult<String> {
    let mut sb = String::new();
    let segs = &c.segments;
    let open = &segs[0].leading_delimiter;
    if !open.is_empty() {
        sb.push_str(&format!("\"{}\" ", esc_dq(open)));
    }
    sb.push_str(&serialize_seg_spec(&segs[0].spec)?);
    for seg in &segs[1..] {
        sb.push_str(&format!(" \"{}\" ", esc_dq(&seg.leading_delimiter)));
        sb.push_str(&serialize_seg_spec(&seg.spec)?);
    }
    if !c.trailing_delimiter.is_empty() {
        sb.push_str(&format!(" \"{}\"", esc_dq(&c.trailing_delimiter)));
    }
    Ok(sb)
}

fn serialize_seg_spec(cs: &ContentSpec) -> CoreResult<String> {
    match cs {
        ContentSpec::Atomic(a) => serialize_atomic(a),
        ContentSpec::Delimited(d) => serialize_delimited(d),
        _ => Err("Unsupported compound segment type".into()),
    }
}

fn serialize_conditional(c: &ConditionalSpec) -> CoreResult<String> {
    Ok(format!(
        "{}? {} | {}",
        c.condition.to_rtl()?,
        serialize_x(&c.positive)?,
        serialize_x(&c.negative)?
    ))
}

fn serialize_x(cs: &ContentSpec) -> CoreResult<String> {
    match cs {
        ContentSpec::Atomic(a) => serialize_atomic(a),
        ContentSpec::Delimited(d) => serialize_delimited(d),
        ContentSpec::Compound(c) => serialize_compound(c),
        _ => Err("Unsupported xContSpec type".into()),
    }
}

fn serialize_act_specs(actions: &[ActionSpec]) -> CoreResult<String> {
    let parts: CoreResult<Vec<String>> = actions.iter().map(serialize_act_spec).collect();
    Ok(parts?.join(", "))
}

fn serialize_act_spec(a: &ActionSpec) -> CoreResult<String> {
    Ok(format!(
        "{}->{}",
        serialize_prov_specs(&a.providers)?,
        serialize_op(a)?
    ))
}

fn serialize_prov_specs(providers: &[ProviderSpec]) -> CoreResult<String> {
    if providers.len() == 1 {
        return serialize_prov_spec(&providers[0]);
    }
    let parts: CoreResult<Vec<String>> = providers.iter().map(serialize_prov_spec).collect();
    Ok(format!("({})", parts?.join(", ")))
}

fn serialize_prov_spec(ps: &ProviderSpec) -> CoreResult<String> {
    if let Some(lit) = &ps.context_literal {
        if let Some(cv) = &lit.const_value {
            return Ok(format!("@'{}'='{}'", esc_sq(&lit.text), esc_sq(cv)));
        }
        return Ok(format!("'{}'", esc_sq(&lit.text)));
    }
    let order = match ps.traversal_order {
        TraversalOrder::RowMajor => "",
        TraversalOrder::ReverseRowMajor => "-",
        TraversalOrder::ColumnMajor => "^",
        TraversalOrder::ReverseColumnMajor => "-^",
    };
    let cond = ps
        .filter_condition
        .as_ref()
        .ok_or("missing filter condition")?
        .to_rtl()?;
    let card = if ps.cardinality == 1 {
        String::new()
    } else if ps.cardinality == UNBOUNDED {
        "*".to_string()
    } else {
        format!("{{{}}}", ps.cardinality)
    };
    Ok(format!("{order}{cond}{card}"))
}

fn serialize_op(a: &ActionSpec) -> CoreResult<String> {
    Ok(match a.operation_type {
        OperationType::Avp => "AVP".to_string(),
        OperationType::Rec => {
            if let Some(p) = a.anchor_pos {
                format!("REC({p})")
            } else if let Some(s) = &a.split_delimiter {
                format!("REC('{}')", esc_dq(s))
            } else {
                "REC".to_string()
            }
        }
        OperationType::Join => {
            if a.key_positions.is_empty() {
                "JOIN".to_string()
            } else {
                let args: Vec<String> = a.key_positions.iter().map(|k| k.to_string()).collect();
                format!("JOIN({})", args.join(", "))
            }
        }
        OperationType::Fill => op_with_delim("FILL", a),
        OperationType::Prefix => op_with_delim("PREFIX", a),
        OperationType::Suffix => op_with_delim("SUFFIX", a),
    })
}

fn op_with_delim(name: &str, a: &ActionSpec) -> String {
    match &a.delimiter {
        Some(d) if !d.is_empty() => format!("{name}(\"{}\")", esc_dq(d)),
        _ => name.to_string(),
    }
}
