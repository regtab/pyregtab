//! CSV parsing for the CLI runner: an exact port of `RtlRunner.parseCsv`
//! (regtab-eval-on-atbench) and of the former pure-Python
//! `pyregtab.runner.parse_csv`.
//!
//! RFC 4180: a quoted field may contain commas, doubled quotes and line
//! breaks; CRLF, CR and LF all end a row; empty lines are skipped, but a line
//! holding only `""` is a row with one empty field. Rows are returned ragged
//! (as parsed); padding to the widest row is done by
//! [`crate::syntax::SyntaxCore::from_rows`].

use std::borrow::Cow;

/// Parses CSV text into ragged rows of fields.
///
/// The scanner works on bytes: every character it dispatches on (`"`, `,`,
/// `\r`, `\n`) is ASCII and never occurs inside a multi-byte UTF-8 sequence,
/// so the field boundaries found on bytes are exactly those the Java/Python
/// char loops find, and every field is a valid UTF-8 slice.
pub fn parse_csv<T: for<'a> From<&'a str>>(text: &str) -> Vec<Vec<T>> {
    let bytes = text.as_bytes();
    let n = bytes.len();
    let mut rows: Vec<Vec<T>> = Vec::new();
    let mut fields: Vec<T> = Vec::new();
    // Accumulator of the current field (reused; a field is emitted as a `T`).
    let mut cur = String::new();
    let mut in_quotes = false;
    // a quote was seen on this line: "" is a field, not an empty line
    let mut quoted = false;
    let mut i = 0;
    // Start of the pending run of plain bytes (copied into `cur` lazily).
    let mut run = 0;

    macro_rules! flush_run {
        ($end:expr) => {
            if run < $end {
                cur.push_str(&text[run..$end]);
            }
        };
    }

    while i < n {
        let ch = bytes[i];
        if in_quotes {
            if ch != b'"' {
                i += 1;
                continue;
            }
            flush_run!(i);
            if i + 1 < n && bytes[i + 1] == b'"' {
                cur.push('"');
                i += 2;
            } else {
                in_quotes = false;
                i += 1;
            }
            run = i;
        } else if ch == b'"' {
            flush_run!(i);
            in_quotes = true;
            quoted = true;
            i += 1;
            run = i;
        } else if ch == b',' {
            flush_run!(i);
            fields.push(T::from(cur.as_str()));
            cur.clear();
            i += 1;
            run = i;
        } else if ch == b'\r' || ch == b'\n' {
            flush_run!(i);
            if ch == b'\r' && i + 1 < n && bytes[i + 1] == b'\n' {
                i += 1; // CRLF is one line break
            }
            if !fields.is_empty() || !cur.is_empty() || quoted {
                fields.push(T::from(cur.as_str()));
                rows.push(std::mem::take(&mut fields));
            }
            cur.clear();
            quoted = false;
            i += 1;
            run = i;
        } else {
            i += 1;
        }
    }
    flush_run!(n);
    if !fields.is_empty() || !cur.is_empty() || quoted {
        fields.push(T::from(cur.as_str()));
        rows.push(fields);
    }
    rows
}

/// Python's universal-newlines translation (`Path.read_text`): `\r\n` and a
/// lone `\r` become `\n`. Borrows the input when it holds no `\r`.
pub fn universal_newlines(text: &str) -> Cow<'_, str> {
    if !text.as_bytes().contains(&b'\r') {
        return Cow::Borrowed(text);
    }
    let mut out = String::with_capacity(text.len());
    let bytes = text.as_bytes();
    let mut i = 0;
    let mut run = 0;
    while i < bytes.len() {
        if bytes[i] == b'\r' {
            out.push_str(&text[run..i]);
            out.push('\n');
            if i + 1 < bytes.len() && bytes[i + 1] == b'\n' {
                i += 1;
            }
            run = i + 1;
        }
        i += 1;
    }
    out.push_str(&text[run..]);
    Cow::Owned(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(s: &str) -> Vec<Vec<String>> {
        parse_csv::<String>(s)
    }

    fn v(rows: &[&[&str]]) -> Vec<Vec<String>> {
        rows.iter().map(|r| r.iter().map(|f| f.to_string()).collect()).collect()
    }

    #[test]
    fn plain_and_quoted() {
        assert_eq!(rows("a,b\n1,2\n"), v(&[&["a", "b"], &["1", "2"]]));
        assert_eq!(rows("\"a,b\",\"c\"\"d\"\n"), v(&[&["a,b", "c\"d"]]));
        assert_eq!(rows("\"x\ny\",z"), vec![vec!["x\ny", "z"]]);
        assert_eq!(rows("\"x\r\ny\",z"), vec![vec!["x\r\ny", "z"]]);
    }

    #[test]
    fn line_breaks_and_empty_lines() {
        assert_eq!(rows("a\r\nb\rc\nd"), v(&[&["a"], &["b"], &["c"], &["d"]]));
        assert_eq!(rows("a\n\n\nb\n\n"), v(&[&["a"], &["b"]]));
        assert_eq!(rows("\"\"\n"), v(&[&[""]]));
        assert_eq!(rows(",\n"), v(&[&["", ""]]));
        assert_eq!(rows(""), Vec::<Vec<String>>::new());
        assert_eq!(rows("\n"), Vec::<Vec<String>>::new());
        assert_eq!(rows("a,"), v(&[&["a", ""]]));
        assert_eq!(rows("a\"b\"c,d"), v(&[&["abc", "d"]]));
        assert_eq!(rows("\"unterminated,x"), v(&[&["unterminated,x"]]));
    }

    #[test]
    fn unicode_passthrough() {
        assert_eq!(rows("тест,\"日本,語\"\n"), v(&[&["тест", "日本,語"]]));
    }

    #[test]
    fn universal_newlines_translation() {
        assert!(matches!(universal_newlines("a\nb"), Cow::Borrowed(_)));
        assert_eq!(universal_newlines("a\r\nb\rc\n\r"), "a\nb\nc\n\n");
        assert_eq!(universal_newlines("\r\n\r\n"), "\n\n");
    }
}
