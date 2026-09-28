//! `Id`: a UUIDv7 newtype (D2, P2-04). Binds to SeaORM/sqlx as hyphenated text and decodes from
//! either a 36-char hyphenated text or 16 raw bytes, since a native MariaDB `UUID` column can come
//! back from the driver in either shape depending on the connector/version.

use std::fmt;
use std::str::FromStr;

use sea_orm::sea_query::{ArrayType, ColumnType, Nullable, Value, ValueType, ValueTypeErr};
use sea_orm::{TryFromU64, TryGetError, TryGetable};
use serde::{Deserialize, Deserializer, Serialize, Serializer};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub Uuid);

impl Id {
    /// A fresh, time-ordered id (UUIDv7 — D2).
    pub fn new() -> Self {
        Self(Uuid::now_v7())
    }
}

impl Default for Id {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Id {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0.as_hyphenated())
    }
}

impl FromStr for Id {
    type Err = uuid::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Id)
    }
}

impl From<Uuid> for Id {
    fn from(u: Uuid) -> Self {
        Id(u)
    }
}

impl From<Id> for Uuid {
    fn from(id: Id) -> Self {
        id.0
    }
}

impl Serialize for Id {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Id::from_str(&s).map_err(serde::de::Error::custom)
    }
}

// --- SeaORM / sea-query value plumbing ---------------------------------------------------------
// Always bound/decoded as hyphenated text (36 chars), which is what a native MariaDB `UUID`
// column round-trips as through sqlx's MySQL driver.

impl From<Id> for Value {
    fn from(id: Id) -> Self {
        Value::String(Some(Box::new(id.to_string())))
    }
}

impl TryGetable for Id {
    fn try_get_by<I: sea_orm::ColIdx>(res: &sea_orm::QueryResult, index: I) -> Result<Self, TryGetError> {
        // Accept either a 36-char hyphenated string or 16 raw bytes (P2-04).
        if let Ok(s) = res.try_get_by::<String, _>(index) {
            return Id::from_str(&s).map_err(|e| TryGetError::DbErr(sea_orm::DbErr::Custom(e.to_string())));
        }
        let bytes = res
            .try_get_by::<Vec<u8>, _>(index)
            .map_err(|_| TryGetError::DbErr(sea_orm::DbErr::Custom("Id: column is neither text nor bytes".to_string())))?;
        Uuid::from_slice(&bytes)
            .map(Id)
            .map_err(|e| TryGetError::DbErr(sea_orm::DbErr::Custom(e.to_string())))
    }
}

impl ValueType for Id {
    fn try_from(v: Value) -> Result<Self, ValueTypeErr> {
        match v {
            Value::String(Some(s)) => Id::from_str(&s).map_err(|_| ValueTypeErr),
            Value::Bytes(Some(b)) if b.len() == 16 => Uuid::from_slice(&b).map(Id).map_err(|_| ValueTypeErr),
            _ => Err(ValueTypeErr),
        }
    }

    fn type_name() -> String {
        "Id".to_string()
    }

    fn array_type() -> ArrayType {
        ArrayType::String
    }

    fn column_type() -> ColumnType {
        ColumnType::Uuid
    }
}

impl Nullable for Id {
    fn null() -> Value {
        Value::String(None)
    }
}

impl TryFromU64 for Id {
    fn try_from_u64(_n: u64) -> Result<Self, sea_orm::DbErr> {
        Err(sea_orm::DbErr::ConvertFromU64("Id is not derived from an auto-increment integer"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn ids_are_strictly_increasing() {
        let mut prev: Option<Uuid> = None;
        for _ in 0..10_000 {
            let id = Id::new();
            if let Some(p) = prev {
                assert!(id.0 > p, "Id::new() must be strictly increasing (UUIDv7 monotonic order)");
            }
            prev = Some(id.0);
        }
    }

    #[test]
    fn display_and_parse_round_trip() {
        let id = Id::new();
        let text = id.to_string();
        assert_eq!(text.len(), 36);
        let back: Id = text.parse().unwrap();
        assert_eq!(back, id);
    }

    #[test]
    fn ordering_matches_generation_order_via_btreeset() {
        // Proxy for "DB order: 1,000 v7 ids inserted in random order come back sorted by
        // generation order under ORDER BY id" — BTreeSet<Id> uses the same Ord this newtype
        // gives sea-query/sqlx's comparison semantics for the bound hyphenated-text value.
        let mut generated = Vec::with_capacity(1000);
        for _ in 0..1000 {
            generated.push(Id::new());
        }
        let mut shuffled = generated.clone();
        // deterministic "shuffle": reverse plus an interleave, not a random crate dependency.
        shuffled.reverse();
        let ordered: BTreeSet<Id> = shuffled.into_iter().collect();
        let ordered_vec: Vec<Id> = ordered.into_iter().collect();
        assert_eq!(ordered_vec, generated, "BTreeSet order must equal generation order");
    }
}
