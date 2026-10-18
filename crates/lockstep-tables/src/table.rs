// SPDX-License-Identifier: Apache-2.0
use lockstep_core::hash_of;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::collections::BTreeMap;

/// The most rows one table holds.
pub const MAXIMUM_ROWS: usize = 65_536;

/// The longest JSON text a table may be read from, in bytes. A swap lives in the recording, so it
/// is bounded.
pub const MAXIMUM_SWAP_BYTES: usize = 1 << 20;

/// What a row must satisfy beyond its shape. `check` runs on every row of every table before the
/// table exists, whether it came from JSON, from a swap or from a save.
pub trait TableRow: Clone + Serialize + DeserializeOwned {
    fn check(&self) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TableError {
    Json(String),
    /// A table of no rows.
    Empty,
    TooMany,
    TooLarge,
    DuplicateName(String),
    InvalidRow {
        name: String,
        reason: String,
    },
}

impl std::fmt::Display for TableError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for TableError {}

/// One row as written in JSON: its name beside the row's own fields.
#[derive(Deserialize)]
#[serde(bound = "Row: DeserializeOwned")]
struct Named<Row> {
    name: String,
    #[serde(flatten)]
    row: Row,
}

#[derive(Deserialize)]
#[serde(bound = "Row: DeserializeOwned")]
struct TableFile<Row> {
    rows: Vec<Named<Row>>,
}

/// Rows by name, in name order. The only ways to build one are `from_json`, `swap` and decoding,
/// and each checks every row, so a table in hand is a table that passed.
#[derive(Clone, Debug, PartialEq)]
pub struct Table<Row> {
    rows: BTreeMap<String, Row>,
}

/// What `Table::swap` did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Swapped {
    /// The new table has the same content as the old one: nothing changed.
    Unchanged,
    Replaced {
        from: u64,
        to: u64,
        rows: usize,
    },
}

impl<Row: TableRow> Table<Row> {
    /// Reads `{"rows": [{"name": "...", ...the row's fields}]}` and checks it.
    pub fn from_json(text: &str) -> Result<Self, TableError> {
        if text.len() > MAXIMUM_SWAP_BYTES {
            return Err(TableError::TooLarge);
        }
        let file: TableFile<Row> =
            serde_json::from_str(text).map_err(|error| TableError::Json(error.to_string()))?;
        if file.rows.len() > MAXIMUM_ROWS {
            return Err(TableError::TooMany);
        }
        let mut rows = BTreeMap::new();
        for named in file.rows {
            if rows.contains_key(&named.name) {
                return Err(TableError::DuplicateName(named.name));
            }
            rows.insert(named.name, named.row);
        }
        Self::from_rows(rows)
    }

    fn from_rows(rows: BTreeMap<String, Row>) -> Result<Self, TableError> {
        if rows.is_empty() {
            return Err(TableError::Empty);
        }
        if rows.len() > MAXIMUM_ROWS {
            return Err(TableError::TooMany);
        }
        for (name, row) in &rows {
            row.check().map_err(|reason| TableError::InvalidRow {
                name: name.clone(),
                reason,
            })?;
        }
        Ok(Table { rows })
    }

    /// Replaces this table with the one in `text`, all or nothing: on any error the table is
    /// untouched. A text with the same content as the table changes nothing.
    pub fn swap(&mut self, text: &str) -> Result<Swapped, TableError> {
        let next = Self::from_json(text)?;
        let (from, to) = (self.digest(), next.digest());
        if from == to {
            return Ok(Swapped::Unchanged);
        }
        let rows = next.rows.len();
        *self = next;
        Ok(Swapped::Replaced { from, to, rows })
    }

    pub fn get(&self, name: &str) -> Option<&Row> {
        self.rows.get(name)
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    /// Never true: a table has at least one row.
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Rows in name order, whatever order the JSON listed them.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &Row)> {
        self.rows.iter().map(|(name, row)| (name.as_str(), row))
    }

    /// A number that names the table's content: equal content, equal digest, on every platform.
    pub fn digest(&self) -> u64 {
        hash_of(&self.rows)
    }
}

impl<Row: Serialize> Serialize for Table<Row> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.rows.serialize(serializer)
    }
}

impl<'de, Row: TableRow> Deserialize<'de> for Table<Row> {
    /// Checks like `from_json`, so a damaged save cannot carry a table that never passed.
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let rows = BTreeMap::<String, Row>::deserialize(deserializer)?;
        Self::from_rows(rows).map_err(serde::de::Error::custom)
    }
}

/// The intent that replaces a table while a session runs: the table's name, for the simulation to
/// route, and the new JSON text. It carries the text itself, not a file name, so a recording holds
/// everything a replay needs.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TableSwap {
    pub table: String,
    pub json: String,
}
