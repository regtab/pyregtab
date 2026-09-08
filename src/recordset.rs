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

/// A recordset: the schema and the values of all records in one flat
/// vector, record after record (`width` = number of attributes). `None`
/// corresponds to Java's `null` (missing value). One allocation for a
/// million records instead of a vector per record.
#[derive(Clone, PartialEq, Debug)]
pub struct RecordsetCore {
    pub schema: Schema,
    width: usize,
    len: usize,
    values: Vec<Option<Text>>,
}

impl RecordsetCore {
    /// An empty recordset over the schema.
    pub fn new(schema: Schema) -> Self {
        let width = schema.attributes.len();
        RecordsetCore { schema, width, len: 0, values: Vec::new() }
    }

    /// An empty recordset with room for `records` records.
    pub fn with_capacity(schema: Schema, records: usize) -> Self {
        let mut rs = Self::new(schema);
        rs.values.reserve(records * rs.width);
        rs
    }

    /// A recordset from rows of values (each row as wide as the schema).
    pub fn from_rows<I: IntoIterator<Item = Option<Text>>>(
        schema: Schema,
        rows: impl IntoIterator<Item = I>,
    ) -> Self {
        let mut rs = Self::new(schema);
        for row in rows {
            rs.push(row);
        }
        rs
    }

    /// Number of records.
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Number of attributes (values per record).
    pub fn width(&self) -> usize {
        self.width
    }

    /// The values of the `i`-th record, aligned with the schema.
    #[inline]
    pub fn record(&self, i: usize) -> &[Option<Text>] {
        assert!(i < self.len, "record index {i} out of range ({})", self.len);
        &self.values[i * self.width..(i + 1) * self.width]
    }

    /// Mutable values of the `i`-th record.
    #[inline]
    pub fn record_mut(&mut self, i: usize) -> &mut [Option<Text>] {
        assert!(i < self.len, "record index {i} out of range ({})", self.len);
        &mut self.values[i * self.width..(i + 1) * self.width]
    }

    /// The records in order.
    pub fn records(&self) -> impl Iterator<Item = &[Option<Text>]> + '_ {
        (0..self.len).map(move |i| self.record(i))
    }

    /// Appends a record; returns its index. The row must be exactly as wide
    /// as the schema.
    pub fn push<I: IntoIterator<Item = Option<Text>>>(&mut self, values: I) -> usize {
        let before = self.values.len();
        self.values.extend(values);
        let added = self.values.len() - before;
        assert_eq!(added, self.width, "a record must have one value per attribute");
        self.len += 1;
        self.len - 1
    }

    /// Appends a copy of the given values as a record; returns its index.
    pub fn push_slice(&mut self, values: &[Option<Text>]) -> usize {
        assert_eq!(values.len(), self.width, "a record must have one value per attribute");
        self.values.extend_from_slice(values);
        self.len += 1;
        self.len - 1
    }

    /// The same records with every value mapped.
    pub fn map_values(self, f: impl FnMut(Option<Text>) -> Option<Text>) -> Self {
        RecordsetCore {
            schema: self.schema,
            width: self.width,
            len: self.len,
            values: self.values.into_iter().map(f).collect(),
        }
    }

    pub fn get(&self, record: usize, attribute: &str) -> Option<&str> {
        let idx = self.schema.index_of(attribute)?;
        self.record(record)[idx].as_deref()
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
        for r in self.records() {
            for (i, v) in r.iter().enumerate() {
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
