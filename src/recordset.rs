//! Port of `ru.icc.regtab.recordset`: Schema, Record, Recordset.
//! Records store values positionally (aligned with the schema).

use crate::util::{CoreResult, Text};
use std::collections::HashSet;
use std::io::{self, Write};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Schema {
    pub attributes: Vec<String>,
}

impl Schema {
    pub fn new(attributes: Vec<String>) -> CoreResult<Self> {
        let mut seen = HashSet::new();
        for a in &attributes {
            if !seen.insert(a.clone()) {
                return Err(format!("Duplicate attribute: {a}").into());
            }
        }
        Ok(Schema { attributes })
    }

    pub fn index_of(&self, attribute: &str) -> Option<usize> {
        self.attributes.iter().position(|a| a == attribute)
    }

    pub fn contains(&self, attribute: &str) -> bool {
        self.index_of(attribute).is_some()
    }
}

/// One record: values aligned positionally with the schema attributes.
/// `None` corresponds to Java's `null` (missing value).
#[derive(Clone, PartialEq, Debug)]
pub struct RecordCore {
    pub values: Vec<Option<Text>>,
}

#[derive(Clone, PartialEq, Debug)]
pub struct RecordsetCore {
    pub schema: Schema,
    pub records: Vec<RecordCore>,
}

impl RecordsetCore {
    pub fn get(&self, record: usize, attribute: &str) -> Option<&str> {
        let idx = self.schema.index_of(attribute)?;
        self.records[record].values[idx].as_deref()
    }

    /// Writes the recordset as CSV (RFC 4180 quoting) record by record: the
    /// schema as the header row, a missing value as `missing`; `quote_all`
    /// double-quotes every field, `newline` terminates every row. Nothing is
    /// buffered beyond what `w` buffers, so a 160 MB output never exists as
    /// one string.
    pub fn write_csv<W: Write>(
        &self,
        w: &mut W,
        sep: &str,
        missing: &str,
        quote_all: bool,
        newline: &str,
    ) -> io::Result<()> {
        fn field<W: Write>(w: &mut W, s: &str, sep: &str, quote_all: bool) -> io::Result<()> {
            if quote_all || s.contains(sep) || s.contains('"') || s.contains('\n') || s.contains('\r') {
                w.write_all(b"\"")?;
                let mut rest = s;
                while let Some(i) = rest.find('"') {
                    w.write_all(&rest.as_bytes()[..i])?;
                    w.write_all(b"\"\"")?;
                    rest = &rest[i + 1..];
                }
                w.write_all(rest.as_bytes())?;
                w.write_all(b"\"")
            } else {
                w.write_all(s.as_bytes())
            }
        }
        for (i, a) in self.schema.attributes.iter().enumerate() {
            if i > 0 {
                w.write_all(sep.as_bytes())?;
            }
            field(w, a, sep, quote_all)?;
        }
        w.write_all(newline.as_bytes())?;
        for r in &self.records {
            for (i, v) in r.values.iter().enumerate() {
                if i > 0 {
                    w.write_all(sep.as_bytes())?;
                }
                field(w, v.as_deref().unwrap_or(missing), sep, quote_all)?;
            }
            w.write_all(newline.as_bytes())?;
        }
        Ok(())
    }

    /// [`Self::write_csv`] into a string.
    pub fn to_csv_string(&self, sep: &str, missing: &str, quote_all: bool, newline: &str) -> String {
        let mut buf: Vec<u8> = Vec::new();
        self.write_csv(&mut buf, sep, missing, quote_all, newline)
            .expect("writing to a Vec cannot fail");
        String::from_utf8(buf).expect("CSV of UTF-8 strings is UTF-8")
    }
}
