//! Java-compatible string helpers and a process-wide regex cache.
//!
//! jRegTab relies on `java.util.regex` and Java string semantics
//! (`String.trim`, `String.strip`, `String.isBlank`, `\s` = ASCII whitespace).
//! These helpers replicate those semantics so the port stays behaviorally
//! identical on the reference fixtures.

use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Errors surfaced by the native core (message + optional source position).
#[derive(Debug)]
pub enum CoreErr {
    Msg(String),
    Py(pyo3::PyErr),
}

impl From<String> for CoreErr {
    fn from(s: String) -> Self {
        CoreErr::Msg(s)
    }
}
impl From<&str> for CoreErr {
    fn from(s: &str) -> Self {
        CoreErr::Msg(s.to_string())
    }
}
impl From<pyo3::PyErr> for CoreErr {
    fn from(e: pyo3::PyErr) -> Self {
        CoreErr::Py(e)
    }
}

pub type CoreResult<T> = Result<T, CoreErr>;

fn cache() -> &'static Mutex<HashMap<String, Arc<Regex>>> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<Regex>>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

pub fn cached_regex(pattern: &str) -> CoreResult<Arc<Regex>> {
    let mut map = cache().lock().unwrap();
    if let Some(re) = map.get(pattern) {
        return Ok(re.clone());
    }
    let re = Regex::new(pattern).map_err(|e| format!("Invalid regex '{pattern}': {e}"))?;
    let re = Arc::new(re);
    map.insert(pattern.to_string(), re.clone());
    Ok(re)
}

/// Java `String.matches(pattern)`: the whole string must match.
pub fn full_match(pattern: &str, text: &str) -> CoreResult<bool> {
    let anchored = format!("^(?:{pattern})$");
    let re = cached_regex(&anchored)?;
    Ok(re.is_match(text))
}

/// Java `String.replaceAll(regex, replacement)`.
/// Java's `$1` group references are converted to Rust's `${1}` form.
pub fn replace_all(pattern: &str, replacement: &str, text: &str) -> CoreResult<String> {
    let re = cached_regex(pattern)?;
    let repl = convert_java_replacement(replacement);
    Ok(re.replace_all(text, repl.as_str()).into_owned())
}

/// Converts Java replacement syntax (`$1`, `\$`, `\\`) to the regex crate's
/// (`${1}`, `$$`, `\`).
fn convert_java_replacement(repl: &str) -> String {
    let mut out = String::with_capacity(repl.len());
    let mut chars = repl.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                // Java: backslash escapes the next character literally.
                if let Some(&n) = chars.peek() {
                    chars.next();
                    if n == '$' {
                        out.push_str("$$");
                    } else {
                        out.push(n);
                    }
                }
            }
            '$' => {
                if let Some(&n) = chars.peek() {
                    if n.is_ascii_digit() {
                        let mut num = String::new();
                        while let Some(&d) = chars.peek() {
                            if d.is_ascii_digit() {
                                num.push(d);
                                chars.next();
                            } else {
                                break;
                            }
                        }
                        out.push_str("${");
                        out.push_str(&num);
                        out.push('}');
                    } else {
                        out.push_str("$$");
                    }
                } else {
                    out.push_str("$$");
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// Java `String.split(regex, -1)`: split on regex keeping trailing empties.
pub fn split_regex(pattern: &str, text: &str) -> CoreResult<Vec<String>> {
    let re = cached_regex(pattern)?;
    Ok(re.split(text).map(|s| s.to_string()).collect())
}

/// `Character.isWhitespace(c)` — NOT the same as Unicode White_Space:
/// U+00A0, U+2007 and U+202F are excluded; U+001C..U+001F are included.
pub fn java_is_whitespace(c: char) -> bool {
    match c {
        '\t' | '\n' | '\x0B' | '\x0C' | '\r' => true,
        '\x1C'..='\x1F' => true,
        '\u{00A0}' | '\u{2007}' | '\u{202F}' => false,
        _ => c.is_whitespace(),
    }
}

/// Java `String.isBlank()`.
pub fn java_is_blank(s: &str) -> bool {
    s.chars().all(java_is_whitespace)
}

/// Java `String.trim()`: removes leading/trailing chars <= U+0020.
pub fn java_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c <= ' ')
}

/// Java `String.strip()`: removes leading/trailing `Character.isWhitespace` chars.
pub fn java_strip(s: &str) -> &str {
    s.trim_matches(java_is_whitespace)
}

/// Is `c` in Java's regex `\s` class: `[ \t\n\x0B\f\r]`.
pub fn java_regex_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\x0B' | '\x0C' | '\r')
}

/// `input.strip().replaceAll("\\s+", " ")` — the NORM extractor and
/// WhitespaceNormalization transformation.
pub fn norm_whitespace(s: &str) -> String {
    let stripped = java_strip(s);
    let mut out = String::with_capacity(stripped.len());
    let mut in_run = false;
    for c in stripped.chars() {
        if java_regex_space(c) {
            if !in_run {
                out.push(' ');
                in_run = true;
            }
        } else {
            out.push(c);
            in_run = false;
        }
    }
    out
}

/// Java `text.split(Pattern.quote(delim), -1)`: literal split keeping empties.
pub fn split_literal(delim: &str, text: &str) -> Vec<String> {
    text.split(delim).map(|s| s.to_string()).collect()
}
