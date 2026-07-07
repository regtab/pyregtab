//! Recursive-descent parser structurally following `RTL.g4`.
//! Ambiguities that ANTLR resolves with adaptive prediction are handled with
//! bounded backtracking (save/restore of the token cursor).

use super::ast::*;
use super::lexer::{Tok, Token};
use super::RtlErr;
use crate::spec::{Idd, TraversalOrder};

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

pub fn parse(toks: Vec<Token>) -> Result<TableAst, RtlErr> {
    let mut p = Parser { toks, pos: 0 };
    let tree = p.table_pattern()?;
    p.expect_eof()?;
    Ok(tree)
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos].tok
    }
    fn peek_at(&self, off: usize) -> &Tok {
        let i = (self.pos + off).min(self.toks.len() - 1);
        &self.toks[i].tok
    }
    fn cur(&self) -> &Token {
        &self.toks[self.pos]
    }
    fn bump(&mut self) -> Token {
        let t = self.toks[self.pos].clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }
    fn save(&self) -> usize {
        self.pos
    }
    fn restore(&mut self, p: usize) {
        self.pos = p;
    }
    fn err<T>(&self, msg: impl Into<String>) -> Result<T, RtlErr> {
        let t = self.cur();
        Err(RtlErr::at(msg, t.line, t.col))
    }
    fn expect(&mut self, tok: Tok, what: &str) -> Result<Token, RtlErr> {
        if *self.peek() == tok {
            Ok(self.bump())
        } else {
            self.err(format!("expected {what}, found {:?}", self.peek()))
        }
    }
    fn eat(&mut self, tok: Tok) -> bool {
        if *self.peek() == tok {
            self.bump();
            true
        } else {
            false
        }
    }
    fn expect_eof(&mut self) -> Result<(), RtlErr> {
        if *self.peek() == Tok::Eof {
            Ok(())
        } else {
            self.err(format!("unexpected trailing input: {:?}", self.peek()))
        }
    }

    fn expect_int(&mut self) -> Result<i64, RtlErr> {
        match self.peek().clone() {
            Tok::Int(n) => {
                self.bump();
                Ok(n)
            }
            other => self.err(format!("expected INT, found {other:?}")),
        }
    }

    fn expect_string(&mut self) -> Result<String, RtlErr> {
        match self.peek().clone() {
            Tok::Str(s) => {
                self.bump();
                Ok(s)
            }
            other => self.err(format!("expected STRING, found {other:?}")),
        }
    }

    // ---------------- quantifier ----------------

    fn try_quantifier(&mut self) -> Result<Option<PQuant>, RtlErr> {
        match self.peek() {
            Tok::Question => {
                self.bump();
                Ok(Some(PQuant::ZeroOrOne))
            }
            Tok::Mult => {
                self.bump();
                Ok(Some(PQuant::ZeroOrMore))
            }
            Tok::Plus => {
                self.bump();
                Ok(Some(PQuant::OneOrMore))
            }
            Tok::LCurly => {
                // only when {INT}
                if matches!(self.peek_at(1), Tok::Int(_)) && *self.peek_at(2) == Tok::RCurly {
                    self.bump();
                    let n = self.expect_int()?;
                    self.expect(Tok::RCurly, "'}'")?;
                    Ok(Some(PQuant::Exactly(n)))
                } else {
                    Ok(None)
                }
            }
            _ => Ok(None),
        }
    }

    // ---------------- tablePattern ----------------

    pub fn table_pattern(&mut self) -> Result<TableAst, RtlErr> {
        let mut fragments = Vec::new();
        while matches!(self.peek(), Tok::FragmentId(_)) && *self.peek_at(1) == Tok::Assign {
            fragments.push(self.fragment_def()?);
        }

        // (cellMatchCond QUESTION)?
        let mut cond = None;
        {
            let save = self.save();
            if let Ok(c) = self.cell_match_cond() {
                if self.eat(Tok::Question) {
                    cond = Some(c);
                } else {
                    self.restore(save);
                }
            } else {
                self.restore(save);
            }
        }

        // settings?
        let settings = if *self.peek() == Tok::LAngle {
            Some(self.settings()?)
        } else {
            None
        };

        // actSpecs?
        let mut actions = None;
        {
            let save = self.save();
            if !matches!(self.peek(), Tok::LSquare | Tok::LCurly) {
                match self.act_specs() {
                    Ok(a) => actions = Some(a),
                    Err(_) => self.restore(save),
                }
            }
        }

        // subtablePattern+
        let mut subtables = Vec::new();
        subtables.push(self.subtable_pattern()?);
        while matches!(self.peek(), Tok::LSquare | Tok::LCurly) {
            subtables.push(self.subtable_pattern()?);
        }

        Ok(TableAst { fragments, cond, settings, actions, subtables })
    }

    fn fragment_def(&mut self) -> Result<(String, FragBodyAst), RtlErr> {
        let name = match self.bump().tok {
            Tok::FragmentId(n) => n,
            _ => unreachable!(),
        };
        self.expect(Tok::Assign, "'='")?;
        match self.peek() {
            Tok::LSquare => {
                self.bump();
                // try cell body first, then row body
                let save = self.save();
                if *self.peek() == Tok::RSquare {
                    self.bump();
                    return Ok((name, FragBodyAst::Cell(None)));
                }
                if let Ok(body) = self.cell_pattern_body() {
                    if self.eat(Tok::RSquare) {
                        return Ok((name, FragBodyAst::Cell(Some(body))));
                    }
                }
                self.restore(save);
                let body = self.row_pattern_body()?;
                self.expect(Tok::RSquare, "']'")?;
                Ok((name, FragBodyAst::Row(body)))
            }
            Tok::LCurly => {
                self.bump();
                // try subrow body first, then subtable body
                let save = self.save();
                if let Ok(body) = self.subrow_pattern_body() {
                    if self.eat(Tok::RCurly) {
                        return Ok((name, FragBodyAst::Subrow(body)));
                    }
                }
                self.restore(save);
                let body = self.subtable_pattern_body()?;
                self.expect(Tok::RCurly, "'}'")?;
                Ok((name, FragBodyAst::Subtable(body)))
            }
            _ => self.err("expected '[' or '{' after fragment '='"),
        }
    }

    fn settings(&mut self) -> Result<Vec<PSetting>, RtlErr> {
        self.expect(Tok::LAngle, "'<'")?;
        let mut out = vec![self.setting()?];
        while self.eat(Tok::Comma) {
            out.push(self.setting()?);
        }
        self.expect(Tok::RAngle, "'>'")?;
        Ok(out)
    }

    fn setting(&mut self) -> Result<PSetting, RtlErr> {
        match self.peek() {
            Tok::KwNorm => {
                self.bump();
                Ok(PSetting::Norm)
            }
            Tok::KwAnch => {
                self.bump();
                self.expect(Tok::LParen, "'('")?;
                let n = self.expect_int()?;
                self.expect(Tok::RParen, "')'")?;
                Ok(PSetting::Anch(n))
            }
            Tok::KwSplit => {
                self.bump();
                self.expect(Tok::LParen, "'('")?;
                let s = self.expect_string()?;
                self.expect(Tok::RParen, "')'")?;
                Ok(PSetting::Split(s))
            }
            other => self.err(format!("unknown setting: {other:?}")),
        }
    }

    // ---------------- subtable ----------------

    fn subtable_pattern(&mut self) -> Result<SubtableAst, RtlErr> {
        match self.peek() {
            Tok::LCurly => {
                // fragment ref {$N} or explicit subtable
                if matches!(self.peek_at(1), Tok::FragmentId(_)) && *self.peek_at(2) == Tok::RCurly
                {
                    self.bump();
                    let name = match self.bump().tok {
                        Tok::FragmentId(n) => n,
                        _ => unreachable!(),
                    };
                    self.bump(); // RCurly
                    let quant = self.try_quantifier()?;
                    Ok(SubtableAst::Frag { name, quant })
                } else {
                    self.bump();
                    let body = self.subtable_pattern_body()?;
                    self.expect(Tok::RCurly, "'}'")?;
                    let quant = self.try_quantifier()?;
                    Ok(SubtableAst::Expl { body, quant })
                }
            }
            Tok::LSquare => {
                // implicit subtable: rowPattern+
                let mut rows = vec![self.row_pattern()?];
                while *self.peek() == Tok::LSquare {
                    rows.push(self.row_pattern()?);
                }
                Ok(SubtableAst::Impl(rows))
            }
            other => self.err(format!("expected subtable pattern, found {other:?}")),
        }
    }

    fn subtable_pattern_body(&mut self) -> Result<SubtableBodyAst, RtlErr> {
        let (cond, actions) = self.cond_and_acts()?;
        let mut rows = vec![self.row_pattern()?];
        while *self.peek() == Tok::LSquare {
            rows.push(self.row_pattern()?);
        }
        Ok(SubtableBodyAst { cond, actions, rows })
    }

    /// `(cellMatchCond QUESTION)? actSpecs?` — shared prefix of the body rules.
    fn cond_and_acts(
        &mut self,
    ) -> Result<(Option<PCellCond>, Option<Vec<PActSpec>>), RtlErr> {
        let mut cond = None;
        {
            let save = self.save();
            if let Ok(c) = self.cell_match_cond() {
                if self.eat(Tok::Question) {
                    cond = Some(c);
                } else {
                    self.restore(save);
                }
            } else {
                self.restore(save);
            }
        }
        let mut actions = None;
        if !matches!(self.peek(), Tok::LSquare | Tok::LCurly) {
            let save = self.save();
            match self.act_specs() {
                Ok(a) => actions = Some(a),
                Err(_) => self.restore(save),
            }
        }
        Ok((cond, actions))
    }

    // ---------------- row ----------------

    fn row_pattern(&mut self) -> Result<RowAst, RtlErr> {
        self.expect(Tok::LSquare, "'['")?;
        if matches!(self.peek(), Tok::FragmentId(_)) && *self.peek_at(1) == Tok::RSquare {
            let name = match self.bump().tok {
                Tok::FragmentId(n) => n,
                _ => unreachable!(),
            };
            self.bump(); // RSquare
            let quant = self.try_quantifier()?;
            return Ok(RowAst::Frag { name, quant });
        }
        let body = self.row_pattern_body()?;
        self.expect(Tok::RSquare, "']'")?;
        let quant = self.try_quantifier()?;
        Ok(RowAst::Body { body, quant })
    }

    fn row_pattern_body(&mut self) -> Result<RowBodyAst, RtlErr> {
        let (cond, actions) = self.cond_and_acts()?;
        let mut subrows = vec![self.subrow_pattern()?];
        while matches!(self.peek(), Tok::LSquare | Tok::LCurly) {
            subrows.push(self.subrow_pattern()?);
        }
        Ok(RowBodyAst { cond, actions, subrows })
    }

    // ---------------- subrow ----------------

    fn subrow_pattern(&mut self) -> Result<SubrowAst, RtlErr> {
        match self.peek() {
            Tok::LCurly => {
                if matches!(self.peek_at(1), Tok::FragmentId(_)) && *self.peek_at(2) == Tok::RCurly
                {
                    self.bump();
                    let name = match self.bump().tok {
                        Tok::FragmentId(n) => n,
                        _ => unreachable!(),
                    };
                    self.bump();
                    let quant = self.try_quantifier()?;
                    Ok(SubrowAst::Frag { name, quant })
                } else {
                    self.bump();
                    let body = self.subrow_pattern_body()?;
                    self.expect(Tok::RCurly, "'}'")?;
                    let quant = self.try_quantifier()?;
                    Ok(SubrowAst::Expl { body, quant })
                }
            }
            Tok::LSquare => {
                let mut cells = vec![self.cell_pattern()?];
                while *self.peek() == Tok::LSquare {
                    cells.push(self.cell_pattern()?);
                }
                Ok(SubrowAst::Impl(cells))
            }
            other => self.err(format!("expected subrow pattern, found {other:?}")),
        }
    }

    fn subrow_pattern_body(&mut self) -> Result<SubrowBodyAst, RtlErr> {
        let (cond, actions) = self.cond_and_acts()?;
        let mut cells = vec![self.cell_pattern()?];
        while *self.peek() == Tok::LSquare {
            cells.push(self.cell_pattern()?);
        }
        Ok(SubrowBodyAst { cond, actions, cells })
    }

    // ---------------- cell ----------------

    fn cell_pattern(&mut self) -> Result<CellAst, RtlErr> {
        self.expect(Tok::LSquare, "'['")?;
        if matches!(self.peek(), Tok::FragmentId(_)) && *self.peek_at(1) == Tok::RSquare {
            let name = match self.bump().tok {
                Tok::FragmentId(n) => n,
                _ => unreachable!(),
            };
            self.bump();
            let quant = self.try_quantifier()?;
            return Ok(CellAst::Frag { name, quant });
        }
        if self.eat(Tok::RSquare) {
            let quant = self.try_quantifier()?;
            return Ok(CellAst::Body { body: None, quant });
        }
        let body = self.cell_pattern_body()?;
        self.expect(Tok::RSquare, "']'")?;
        let quant = self.try_quantifier()?;
        Ok(CellAst::Body { body: Some(body), quant })
    }

    /// cellPatternBody : cellMatchCond QUESTION actSpecs? contSpec
    ///                 | cellMatchCond
    ///                 | actSpecs? contSpec
    fn cell_pattern_body(&mut self) -> Result<CellBodyAst, RtlErr> {
        // Alt 1: cellMatchCond QUESTION actSpecs? contSpec
        {
            let save = self.save();
            if let Ok(cond) = self.cell_match_cond() {
                if self.eat(Tok::Question) {
                    let alt_save = self.save();
                    let mut actions = None;
                    if let Ok(a) = self.act_specs() {
                        actions = Some(a);
                    } else {
                        self.restore(alt_save);
                    }
                    match self.cont_spec(&[Tok::RSquare]) {
                        Ok(cont) if *self.peek() == Tok::RSquare => {
                            return Ok(CellBodyAst {
                                cond: Some(cond),
                                actions,
                                cont: Some(cont),
                            });
                        }
                        _ => {
                            self.restore(save);
                        }
                    }
                } else {
                    self.restore(save);
                }
            } else {
                self.restore(save);
            }
        }
        // Alt 2: cellMatchCond alone
        {
            let save = self.save();
            if let Ok(cond) = self.cell_match_cond() {
                if *self.peek() == Tok::RSquare {
                    return Ok(CellBodyAst { cond: Some(cond), actions: None, cont: None });
                }
            }
            self.restore(save);
        }
        // Alt 3: actSpecs? contSpec
        let save = self.save();
        let mut actions = None;
        if let Ok(a) = self.act_specs() {
            // actSpecs? must be followed by a contSpec (atomic/delimited/
            // compound start OR a conditional's cellMatchCond start)
            if matches!(
                self.peek(),
                Tok::KwVal
                    | Tok::KwAttr
                    | Tok::KwAux
                    | Tok::KwSkip
                    | Tok::Str(_)
                    | Tok::LParen
                    | Tok::Excl
                    | Tok::Tilda
                    | Tok::KwBlank
                    | Tok::KwExt
            ) {
                actions = Some(a);
            } else {
                self.restore(save);
            }
        } else {
            self.restore(save);
        }
        let cont = self.cont_spec(&[Tok::RSquare])?;
        if *self.peek() != Tok::RSquare {
            return self.err(format!("expected ']', found {:?}", self.peek()));
        }
        Ok(CellBodyAst { cond: None, actions, cont: Some(cont) })
    }

    // ---------------- content spec ----------------

    /// contSpec : atomContSpec | delimContSpec | compContSpec | condContSpec
    /// `followers` — tokens that may legally follow the content spec.
    fn cont_spec(&mut self, followers: &[Tok]) -> Result<ContAst, RtlErr> {
        // atomContSpec
        {
            let save = self.save();
            if let Ok(atom) = self.atom_cont_spec() {
                if followers.contains(self.peek()) {
                    return Ok(ContAst::Atom(atom));
                }
            }
            self.restore(save);
        }
        // delimContSpec
        {
            let save = self.save();
            if let Ok(d) = self.delim_cont_spec() {
                if followers.contains(self.peek()) {
                    return Ok(ContAst::Delim(d));
                }
            }
            self.restore(save);
        }
        // compContSpec
        {
            let save = self.save();
            if let Ok(c) = self.comp_cont_spec(followers) {
                if followers.contains(self.peek()) {
                    return Ok(ContAst::Comp(c));
                }
            }
            self.restore(save);
        }
        // condContSpec
        let cond = self.cell_match_cond()?;
        self.expect(Tok::Question, "'?'")?;
        let positive = self.x_cont_spec(&[Tok::VBar])?;
        self.expect(Tok::VBar, "'|'")?;
        let negative = self.x_cont_spec(followers)?;
        Ok(ContAst::Cond(Box::new(CondAst { cond, positive, negative })))
    }

    fn x_cont_spec(&mut self, followers: &[Tok]) -> Result<XSpecAst, RtlErr> {
        {
            let save = self.save();
            if let Ok(atom) = self.atom_cont_spec() {
                if followers.contains(self.peek()) {
                    return Ok(XSpecAst::Atom(atom));
                }
            }
            self.restore(save);
        }
        {
            let save = self.save();
            if let Ok(d) = self.delim_cont_spec() {
                if followers.contains(self.peek()) {
                    return Ok(XSpecAst::Delim(d));
                }
            }
            self.restore(save);
        }
        let c = self.comp_cont_spec(followers)?;
        Ok(XSpecAst::Comp(c))
    }

    /// atomContSpec : itemDerivDir tags? (ASSIGN strExtr)? (COLON actSpecs)?
    fn atom_cont_spec(&mut self) -> Result<AtomAst, RtlErr> {
        let idd = match self.peek() {
            Tok::KwVal => Idd::Val,
            Tok::KwAttr => Idd::Attr,
            Tok::KwAux => Idd::Aux,
            Tok::KwSkip => Idd::Skip,
            other => return self.err(format!("expected item derivation directive, found {other:?}")),
        };
        self.bump();
        let mut tags = Vec::new();
        while *self.peek() == Tok::Hash {
            self.bump();
            tags.push(self.expect_string()?);
        }
        let extractor = if self.eat(Tok::Assign) {
            Some(self.str_extr()?)
        } else {
            None
        };
        let actions = if self.eat(Tok::Colon) {
            Some(self.act_specs()?)
        } else {
            None
        };
        Ok(AtomAst { idd, tags, extractor, actions })
    }

    /// delimContSpec : LPAREN atomContSpec RPAREN LCURLY separator RCURLY
    fn delim_cont_spec(&mut self) -> Result<DelimAst, RtlErr> {
        self.expect(Tok::LParen, "'('")?;
        let atom = self.atom_cont_spec()?;
        self.expect(Tok::RParen, "')'")?;
        self.expect(Tok::LCurly, "'{'")?;
        let separator = self.expect_string()?;
        self.expect(Tok::RCurly, "'}'")?;
        Ok(DelimAst { atom, separator })
    }

    /// compContSpec : openDelim? compSeg (separator compSeg)* closeDelim?
    fn comp_cont_spec(&mut self, followers: &[Tok]) -> Result<CompAst, RtlErr> {
        let mut open_delim = None;
        if let Tok::Str(_) = self.peek() {
            open_delim = Some(self.expect_string()?);
        }
        let first = self.comp_seg()?;
        let mut rest = Vec::new();
        let mut close_delim = None;
        while let Tok::Str(_) = self.peek() {
            let s = self.expect_string()?;
            // a following compSeg? else it's the close delimiter
            let save = self.save();
            match self.comp_seg() {
                Ok(seg) => rest.push((s, seg)),
                Err(_) => {
                    self.restore(save);
                    close_delim = Some(s);
                    break;
                }
            }
        }
        // reject if not a "real" compound (single atom, no delims) — the
        // atom/delim alternatives are tried first by the caller anyway.
        if open_delim.is_none() && rest.is_empty() && close_delim.is_none() {
            if let CompSegAst::Atom(_) = first {
                if followers.contains(self.peek()) {
                    // единственный атом — не compound; вызывающий уже пробовал atom
                    return self.err("not a compound spec");
                }
            }
        }
        Ok(CompAst { open_delim, first, rest, close_delim })
    }

    fn comp_seg(&mut self) -> Result<CompSegAst, RtlErr> {
        if *self.peek() == Tok::LParen {
            Ok(CompSegAst::Delim(self.delim_cont_spec()?))
        } else {
            Ok(CompSegAst::Atom(self.atom_cont_spec()?))
        }
    }

    // ---------------- string extractor ----------------

    fn str_extr(&mut self) -> Result<Vec<PStep>, RtlErr> {
        let mut steps = vec![self.str_extr_step()?];
        while self.eat(Tok::Dot) {
            steps.push(self.str_extr_step()?);
        }
        Ok(steps)
    }

    fn str_extr_step(&mut self) -> Result<PStep, RtlErr> {
        match self.peek() {
            Tok::KwSubstr => {
                self.bump();
                self.expect(Tok::LParen, "'('")?;
                let a = self.expect_int()?;
                self.expect(Tok::Comma, "','")?;
                let b = self.expect_int()?;
                self.expect(Tok::RParen, "')'")?;
                Ok(PStep::Substr(a, b))
            }
            Tok::KwRepl => {
                self.bump();
                self.expect(Tok::LParen, "'('")?;
                let a = self.expect_string()?;
                self.expect(Tok::Comma, "','")?;
                let b = self.expect_string()?;
                self.expect(Tok::RParen, "')'")?;
                Ok(PStep::Repl(a, b))
            }
            Tok::KwNorm => {
                self.bump();
                Ok(PStep::Norm)
            }
            Tok::KwUc => {
                self.bump();
                Ok(PStep::Uc)
            }
            Tok::KwLc => {
                self.bump();
                Ok(PStep::Lc)
            }
            Tok::KwTrim => {
                self.bump();
                Ok(PStep::Trim)
            }
            other => self.err(format!("expected string extractor step, found {other:?}")),
        }
    }

    // ---------------- action specs ----------------

    fn act_specs(&mut self) -> Result<Vec<PActSpec>, RtlErr> {
        let mut out = vec![self.act_spec()?];
        while *self.peek() == Tok::Comma {
            let save = self.save();
            self.bump();
            match self.act_spec() {
                Ok(a) => out.push(a),
                Err(_) => {
                    self.restore(save);
                    break;
                }
            }
        }
        Ok(out)
    }

    fn act_spec(&mut self) -> Result<PActSpec, RtlErr> {
        let providers = self.prov_specs()?;
        self.expect(Tok::Arrow, "'->'")?;
        let op = self.op()?;
        Ok(PActSpec { providers, op })
    }

    fn op(&mut self) -> Result<POp, RtlErr> {
        match self.peek() {
            Tok::KwAvp => {
                self.bump();
                Ok(POp::Avp)
            }
            Tok::KwRec => {
                self.bump();
                let mut anchor = None;
                let mut split = None;
                if self.eat(Tok::LParen) {
                    match self.peek().clone() {
                        Tok::Int(n) => {
                            self.bump();
                            anchor = Some(n);
                        }
                        Tok::Str(s) => {
                            self.bump();
                            split = Some(s);
                        }
                        other => return self.err(format!("expected INT or STRING in REC(...), found {other:?}")),
                    }
                    self.expect(Tok::RParen, "')'")?;
                }
                Ok(POp::Rec { anchor, split })
            }
            Tok::KwJoin => {
                self.bump();
                let mut keys = Vec::new();
                if self.eat(Tok::LParen) {
                    keys.push(self.expect_int()?);
                    while self.eat(Tok::Comma) {
                        keys.push(self.expect_int()?);
                    }
                    self.expect(Tok::RParen, "')'")?;
                }
                Ok(POp::Join(keys))
            }
            Tok::KwFill => {
                self.bump();
                Ok(POp::Fill(self.op_string_arg()?))
            }
            Tok::KwPrefix => {
                self.bump();
                Ok(POp::Prefix(self.op_string_arg()?))
            }
            Tok::KwSuffix => {
                self.bump();
                Ok(POp::Suffix(self.op_string_arg()?))
            }
            other => self.err(format!("expected operation, found {other:?}")),
        }
    }

    fn op_string_arg(&mut self) -> Result<Option<String>, RtlErr> {
        if self.eat(Tok::LParen) {
            let s = self.expect_string()?;
            self.expect(Tok::RParen, "')'")?;
            Ok(Some(s))
        } else {
            Ok(None)
        }
    }

    /// provSpecs : provSpec | LPAREN provSpec (COMMA provSpec)* RPAREN | LPAREN RPAREN
    fn prov_specs(&mut self) -> Result<Vec<PProvSpec>, RtlErr> {
        if *self.peek() == Tok::LParen {
            // could still be a single tblProvSpec with parenthesized constraints
            let save = self.save();
            if let Ok(p) = self.prov_spec() {
                if *self.peek() == Tok::Arrow {
                    return Ok(vec![p]);
                }
            }
            self.restore(save);
            self.expect(Tok::LParen, "'('")?;
            if self.eat(Tok::RParen) {
                return Ok(Vec::new());
            }
            let mut out = vec![self.prov_spec()?];
            while self.eat(Tok::Comma) {
                out.push(self.prov_spec()?);
            }
            self.expect(Tok::RParen, "')'")?;
            Ok(out)
        } else {
            Ok(vec![self.prov_spec()?])
        }
    }

    /// provSpec : tblProvSpec | ctxProvSpec | ctxAvpSpec
    fn prov_spec(&mut self) -> Result<PProvSpec, RtlErr> {
        match self.peek().clone() {
            Tok::Str(s) => {
                self.bump();
                Ok(PProvSpec::Ctx(s))
            }
            Tok::At => {
                self.bump();
                let name = self.expect_string()?;
                self.expect(Tok::Assign, "'='")?;
                let value = self.expect_string()?;
                Ok(PProvSpec::CtxAvp(name, value))
            }
            _ => Ok(PProvSpec::Tbl(self.tbl_prov_spec()?)),
        }
    }

    /// tblProvSpec : traversalOrderMark? (spatConstr | LPAREN constraints RPAREN | bareConjConstraints) cardinality?
    fn tbl_prov_spec(&mut self) -> Result<PTblProv, RtlErr> {
        let order = match self.peek() {
            Tok::Minus => {
                self.bump();
                if self.eat(Tok::Caret) {
                    TraversalOrder::ReverseColumnMajor
                } else {
                    TraversalOrder::ReverseRowMajor
                }
            }
            Tok::Caret => {
                self.bump();
                TraversalOrder::ColumnMajor
            }
            _ => TraversalOrder::RowMajor,
        };
        let body = if *self.peek() == Tok::LParen {
            self.bump();
            let c = self.constraints()?;
            self.expect(Tok::RParen, "')'")?;
            PTblBody::Parens(c)
        } else {
            let spat = self.spat_constr()?;
            if *self.peek() == Tok::Amp {
                let mut base = Vec::new();
                while self.eat(Tok::Amp) {
                    base.push(self.base_constr()?);
                }
                PTblBody::BareConj(spat, base)
            } else {
                PTblBody::Spat(spat)
            }
        };
        let cardinality = match self.peek() {
            Tok::LCurly => {
                if matches!(self.peek_at(1), Tok::Int(_)) && *self.peek_at(2) == Tok::RCurly {
                    self.bump();
                    let n = self.expect_int()?;
                    self.expect(Tok::RCurly, "'}'")?;
                    Some(PCard::Exactly(n))
                } else {
                    None
                }
            }
            Tok::Mult => {
                self.bump();
                Some(PCard::Unbounded)
            }
            _ => None,
        };
        Ok(PTblProv { order, body, cardinality })
    }

    // ---------------- constraints ----------------

    fn constraints(&mut self) -> Result<PConstraints, RtlErr> {
        let mut or_groups = vec![self.or_group()?];
        while self.eat(Tok::VBar) {
            or_groups.push(self.or_group()?);
        }
        Ok(PConstraints { or_groups })
    }

    fn or_group(&mut self) -> Result<POrGroup, RtlErr> {
        let mut base = vec![self.base_constr()?];
        while self.eat(Tok::Amp) {
            base.push(self.base_constr()?);
        }
        Ok(POrGroup { base })
    }

    fn base_constr(&mut self) -> Result<PBaseConstr, RtlErr> {
        if self.eat(Tok::LParen) {
            let c = self.constraints()?;
            self.expect(Tok::RParen, "')'")?;
            Ok(PBaseConstr::Parens(c))
        } else {
            Ok(PBaseConstr::Constr(self.constr()?))
        }
    }

    fn constr(&mut self) -> Result<PConstr, RtlErr> {
        // spatConstr first (keywords), then contConstr
        let save = self.save();
        match self.spat_constr() {
            Ok(s) => Ok(PConstr::Spat(s)),
            Err(_) => {
                self.restore(save);
                Ok(PConstr::Cont(self.cont_constr()?))
            }
        }
    }

    fn spat_constr(&mut self) -> Result<PSpat, RtlErr> {
        let named = match self.peek() {
            Tok::KwLt => Some(NamedSpat::LeftOf),
            Tok::KwRt => Some(NamedSpat::RightOf),
            Tok::KwAv => Some(NamedSpat::Above),
            Tok::KwBw => Some(NamedSpat::Below),
            Tok::KwRow => Some(NamedSpat::SameRow),
            Tok::KwCol => Some(NamedSpat::SameCol),
            Tok::KwSr => Some(NamedSpat::SameSubrow),
            Tok::KwSc => Some(NamedSpat::SameSubcol),
            Tok::KwSt => Some(NamedSpat::SameSubtable),
            Tok::KwNcl => Some(NamedSpat::NotSameCell),
            Tok::KwCl => Some(NamedSpat::SameCell),
            _ => None,
        };
        if let Some(n) = named {
            self.bump();
            return Ok(PSpat::Named(n));
        }
        match self.peek() {
            Tok::KwC => {
                self.bump();
                Ok(PSpat::Col(self.pos_like()?))
            }
            Tok::KwR => {
                self.bump();
                Ok(PSpat::Row(self.pos_like()?))
            }
            Tok::KwP => {
                self.bump();
                Ok(PSpat::Pos(self.pos_like()?))
            }
            other => self.err(format!("expected spatial constraint, found {other:?}")),
        }
    }

    /// (range | offset | INT) — range : start '..' end?
    fn pos_like(&mut self) -> Result<PPosLike, RtlErr> {
        let start = self.bound()?;
        if self.eat(Tok::DoublePeriod) {
            let end = {
                let save = self.save();
                match self.bound() {
                    Ok(b) => Some(b),
                    Err(_) => {
                        self.restore(save);
                        None
                    }
                }
            };
            return Ok(PPosLike::Range { start, end });
        }
        Ok(match start {
            PBound::Offset(d) => PPosLike::Offset(d),
            PBound::Int(n) => PPosLike::Int(n),
        })
    }

    fn bound(&mut self) -> Result<PBound, RtlErr> {
        match self.peek().clone() {
            Tok::Minus => {
                self.bump();
                let n = self.expect_int()?;
                Ok(PBound::Offset(-n))
            }
            Tok::Plus => {
                self.bump();
                let n = self.expect_int()?;
                Ok(PBound::Offset(n))
            }
            Tok::Int(n) => {
                self.bump();
                Ok(PBound::Int(n))
            }
            other => self.err(format!("expected offset or INT, found {other:?}")),
        }
    }

    fn cont_constr(&mut self) -> Result<PContConstr, RtlErr> {
        match self.peek().clone() {
            Tok::KwStr => {
                self.bump();
                Ok(PContConstr::SameStr)
            }
            Tok::KwExt => {
                let t = self.cur().clone();
                self.bump();
                self.expect(Tok::LParen, "'('")?;
                let name = self.expect_string()?;
                self.expect(Tok::RParen, "')'")?;
                Ok(PContConstr::Ext { name, line: t.line, col: t.col })
            }
            Tok::KwBlank => {
                self.bump();
                Ok(PContConstr::Blank { neg: false })
            }
            Tok::Str(s) => {
                self.bump();
                Ok(PContConstr::Regex { neg: false, pattern: s })
            }
            Tok::Tilda => {
                self.bump();
                let s = self.expect_string()?;
                Ok(PContConstr::Contains { neg: false, substring: s })
            }
            Tok::Hash => {
                self.bump();
                let s = self.expect_string()?;
                Ok(PContConstr::Tag { neg: false, name: s })
            }
            Tok::Excl => {
                self.bump();
                match self.peek().clone() {
                    Tok::KwBlank => {
                        self.bump();
                        Ok(PContConstr::Blank { neg: true })
                    }
                    Tok::Str(s) => {
                        self.bump();
                        Ok(PContConstr::Regex { neg: true, pattern: s })
                    }
                    Tok::Tilda => {
                        self.bump();
                        let s = self.expect_string()?;
                        Ok(PContConstr::Contains { neg: true, substring: s })
                    }
                    Tok::Hash => {
                        self.bump();
                        let s = self.expect_string()?;
                        Ok(PContConstr::Tag { neg: true, name: s })
                    }
                    other => self.err(format!("expected content constraint after '!', found {other:?}")),
                }
            }
            other => self.err(format!("expected content constraint, found {other:?}")),
        }
    }

    // ---------------- cell match condition ----------------

    /// cellMatchConstr : regex | blank | contains | ext
    fn cell_match_cond(&mut self) -> Result<PCellCond, RtlErr> {
        match self.peek().clone() {
            Tok::KwExt => {
                let t = self.cur().clone();
                self.bump();
                self.expect(Tok::LParen, "'('")?;
                let name = self.expect_string()?;
                self.expect(Tok::RParen, "')'")?;
                Ok(PCellCond::Ext { name, line: t.line, col: t.col })
            }
            Tok::KwBlank => {
                self.bump();
                Ok(PCellCond::Blank { neg: false })
            }
            Tok::Str(s) => {
                self.bump();
                Ok(PCellCond::Regex { neg: false, pattern: s })
            }
            Tok::Tilda => {
                self.bump();
                let s = self.expect_string()?;
                Ok(PCellCond::Contains { neg: false, substring: s })
            }
            Tok::Excl => {
                self.bump();
                match self.peek().clone() {
                    Tok::KwBlank => {
                        self.bump();
                        Ok(PCellCond::Blank { neg: true })
                    }
                    Tok::Str(s) => {
                        self.bump();
                        Ok(PCellCond::Regex { neg: true, pattern: s })
                    }
                    Tok::Tilda => {
                        self.bump();
                        let s = self.expect_string()?;
                        Ok(PCellCond::Contains { neg: true, substring: s })
                    }
                    other => {
                        self.err(format!("expected cell match constraint after '!', found {other:?}"))
                    }
                }
            }
            other => self.err(format!("expected cell match constraint, found {other:?}")),
        }
    }
}
