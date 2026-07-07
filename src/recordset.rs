//! Port of `ru.icc.regtab.recordset`: Schema, Record, Recordset.
//! Records store values positionally (aligned with the schema).

use crate::util::CoreResult;
use std::collections::HashSet;

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
    pub values: Vec<Option<String>>,
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
}
