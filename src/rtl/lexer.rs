//! Hand-written lexer for RTL (mirrors the lexer rules of `RTL.g4`,
//! including `caseInsensitive = true` and the three string-literal forms).

use super::RtlErr;

#[derive(Clone, PartialEq, Debug)]
pub enum Tok {
    Plus,
    Minus,
    Caret,
    Mult,
    Amp,
    LParen,
    RParen,
    LCurly,
    RCurly,
    LSquare,
    RSquare,
    LAngle,
    RAngle,
    Colon,
    Comma,
    Question,
    VBar,
    Excl,
    Tilda,
    DoublePeriod,
    Dot,
    Assign,
    Arrow,
    Hash,
    At,
    Int(i64),
    /// Unquoted, unescaped string value (parseStringLiteral applied).
    Str(String),
    /// Fragment name without the leading `$` (case preserved).
    FragmentId(String),
    // keywords (case-insensitive)
    KwAttr,
    KwVal,
    KwAux,
    KwSkip,
    KwFill,
    KwPrefix,
    KwSuffix,
    KwAvp,
    KwRec,
    KwJoin,
    KwLt,
    KwRt,
    KwAv,
    KwBw,
    KwRow,
    KwCol,
    KwSr,
    KwSc,
    KwSt,
    KwNcl,
    KwCl,
    KwStr,
    KwExt,
    KwBlank,
    KwNorm,
    KwAnch,
    KwSplit,
    KwSubstr,
    KwRepl,
    KwUc,
    KwLc,
    KwTrim,
    KwR,
    KwC,
    KwP,
    Eof,
}

#[derive(Clone, Debug)]
pub struct Token {
    pub tok: Tok,
    pub line: i64,
    pub col: i64,
}

fn keyword(word: &str) -> Option<Tok> {
    Some(match word.to_ascii_uppercase().as_str() {
        "ATTR" => Tok::KwAttr,
        "VAL" => Tok::KwVal,
        "AUX" => Tok::KwAux,
        "SKIP" => Tok::KwSkip,
        "FILL" => Tok::KwFill,
        "PREFIX" => Tok::KwPrefix,
        "SUFFIX" => Tok::KwSuffix,
        "AVP" => Tok::KwAvp,
        "REC" => Tok::KwRec,
        "JOIN" => Tok::KwJoin,
        "LT" => Tok::KwLt,
        "RT" => Tok::KwRt,
        "AV" => Tok::KwAv,
        "BW" => Tok::KwBw,
        "ROW" => Tok::KwRow,
        "COL" => Tok::KwCol,
        "SR" => Tok::KwSr,
        "SC" => Tok::KwSc,
        "ST" => Tok::KwSt,
        "NCL" => Tok::KwNcl,
        "CL" => Tok::KwCl,
        "STR" => Tok::KwStr,
        "EXT" => Tok::KwExt,
        "BLANK" => Tok::KwBlank,
        "NORM" => Tok::KwNorm,
        "ANCH" => Tok::KwAnch,
        "SPLIT" => Tok::KwSplit,
        "SUBSTR" => Tok::KwSubstr,
        "REPL" => Tok::KwRepl,
        "UC" => Tok::KwUc,
        "LC" => Tok::KwLc,
        "TRIM" => Tok::KwTrim,
        "R" => Tok::KwR,
        "C" => Tok::KwC,
        "P" => Tok::KwP,
        _ => return None,
    })
}

struct Lexer<'a> {
    chars: Vec<char>,
    pos: usize,
    line: i64,
    col: i64,
    src: &'a str,
}

impl<'a> Lexer<'a> {
    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }
    fn peek2(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }
    fn bump(&mut self) -> Option<char> {
        let c = self.chars.get(self.pos).copied()?;
        self.pos += 1;
        if c == '\n' {
            self.line += 1;
            self.col = 0;
        } else {
            self.col += 1;
        }
        Some(c)
    }
}

/// Unescapes a quoted body per `parseStringLiteral`: only doubled quotes
/// are collapsed; backtick escapes stay literal.
fn unescape(body: &str, quote: char) -> String {
    let doubled: String = [quote, quote].iter().collect();
    body.replace(&doubled, &quote.to_string())
}

pub fn lex(src: &str) -> Result<Vec<Token>, RtlErr> {
    let mut lx = Lexer {
        chars: src.chars().collect(),
        pos: 0,
        line: 1,
        col: 0,
        src,
    };
    let _ = lx.src;
    let mut out = Vec::new();

    loop {
        // skip whitespace, BOM, comments
        loop {
            match lx.peek() {
                Some(c) if c == ' ' || c == '\r' || c == '\t' || c == '\n' || c == '\u{FEFF}' => {
                    lx.bump();
                }
                Some('/') if lx.peek2() == Some('/') => {
                    while let Some(c) = lx.peek() {
                        if c == '\r' || c == '\n' {
                            break;
                        }
                        lx.bump();
                    }
                }
                _ => break,
            }
        }
        let (line, col) = (lx.line, lx.col);
        let Some(c) = lx.peek() else {
            out.push(Token { tok: Tok::Eof, line, col });
            return Ok(out);
        };

        let tok = match c {
            '+' => {
                lx.bump();
                Tok::Plus
            }
            '-' => {
                lx.bump();
                if lx.peek() == Some('>') {
                    lx.bump();
                    Tok::Arrow
                } else {
                    Tok::Minus
                }
            }
            '^' => {
                lx.bump();
                Tok::Caret
            }
            '*' => {
                lx.bump();
                Tok::Mult
            }
            '&' => {
                lx.bump();
                Tok::Amp
            }
            '(' => {
                lx.bump();
                Tok::LParen
            }
            ')' => {
                lx.bump();
                Tok::RParen
            }
            '{' => {
                lx.bump();
                Tok::LCurly
            }
            '}' => {
                lx.bump();
                Tok::RCurly
            }
            '[' => {
                lx.bump();
                Tok::LSquare
            }
            ']' => {
                lx.bump();
                Tok::RSquare
            }
            '<' => {
                lx.bump();
                Tok::LAngle
            }
            '>' => {
                lx.bump();
                Tok::RAngle
            }
            ':' => {
                lx.bump();
                Tok::Colon
            }
            ',' => {
                lx.bump();
                Tok::Comma
            }
            '?' => {
                lx.bump();
                Tok::Question
            }
            '|' => {
                lx.bump();
                Tok::VBar
            }
            '!' => {
                lx.bump();
                Tok::Excl
            }
            '~' => {
                lx.bump();
                Tok::Tilda
            }
            '=' => {
                lx.bump();
                Tok::Assign
            }
            '#' => {
                lx.bump();
                Tok::Hash
            }
            '@' => {
                lx.bump();
                Tok::At
            }
            '.' => {
                lx.bump();
                if lx.peek() == Some('.') {
                    lx.bump();
                    Tok::DoublePeriod
                } else {
                    Tok::Dot
                }
            }
            '_' => {
                lx.bump();
                Tok::KwSkip
            }
            '$' => {
                lx.bump();
                let mut name = String::new();
                match lx.peek() {
                    Some(c) if c.is_ascii_alphabetic() => {
                        name.push(c);
                        lx.bump();
                    }
                    _ => {
                        return Err(RtlErr::at(
                            "token recognition error at: '$'",
                            line,
                            col,
                        ))
                    }
                }
                while let Some(c) = lx.peek() {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        name.push(c);
                        lx.bump();
                    } else {
                        break;
                    }
                }
                Tok::FragmentId(name)
            }
            '0'..='9' => {
                let mut n: i64 = 0;
                while let Some(c) = lx.peek() {
                    if let Some(d) = c.to_digit(10) {
                        n = n * 10 + d as i64;
                        lx.bump();
                    } else {
                        break;
                    }
                }
                Tok::Int(n)
            }
            '"' | '\'' => {
                lx.bump();
                let quote = c;
                let mut body = String::new();
                loop {
                    match lx.peek() {
                        None => {
                            return Err(RtlErr::at(
                                format!("unclosed string literal starting at {line}:{col}"),
                                line,
                                col,
                            ))
                        }
                        Some('`') if matches!(lx.peek2(), Some('\'') | Some('"')) => {
                            // backtick escape: kept literally (as in Java)
                            body.push('`');
                            lx.bump();
                            body.push(lx.peek().unwrap());
                            lx.bump();
                        }
                        Some(ch) if ch == quote => {
                            if lx.peek2() == Some(quote) {
                                body.push(quote);
                                body.push(quote);
                                lx.bump();
                                lx.bump();
                            } else {
                                lx.bump();
                                break;
                            }
                        }
                        Some(ch) => {
                            body.push(ch);
                            lx.bump();
                        }
                    }
                }
                Tok::Str(unescape(&body, quote))
            }
            '\u{201C}' => {
                // smart quote: non-greedy up to '”' or '″'
                lx.bump();
                let mut body = String::new();
                loop {
                    match lx.peek() {
                        None => {
                            return Err(RtlErr::at(
                                format!("unclosed string literal starting at {line}:{col}"),
                                line,
                                col,
                            ))
                        }
                        Some('\u{201D}') | Some('\u{2033}') => {
                            lx.bump();
                            break;
                        }
                        Some(ch) => {
                            body.push(ch);
                            lx.bump();
                        }
                    }
                }
                Tok::Str(body)
            }
            c if c.is_ascii_alphabetic() => {
                let mut word = String::new();
                while let Some(ch) = lx.peek() {
                    if ch.is_ascii_alphabetic() {
                        word.push(ch);
                        lx.bump();
                    } else {
                        break;
                    }
                }
                match keyword(&word) {
                    Some(t) => t,
                    None => {
                        return Err(RtlErr::at(
                            format!("token recognition error at: '{word}'"),
                            line,
                            col,
                        ))
                    }
                }
            }
            other => {
                return Err(RtlErr::at(
                    format!("token recognition error at: '{other}'"),
                    line,
                    col,
                ))
            }
        };
        out.push(Token { tok, line, col });
    }
}
