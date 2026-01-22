//! Serialize implementation for Value.

use super::{Number, TaggedValue, Value};
use serde::ser::{SerializeMap, SerializeSeq};
use serde::{Serialize, Serializer};

impl Serialize for Value {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(b) => serializer.serialize_bool(*b),
            Value::Number(n) => n.serialize(serializer),
            Value::String(s) => serializer.serialize_str(s),
            Value::Sequence(seq) => {
                let mut seq_ser = serializer.serialize_seq(Some(seq.len()))?;
                for item in seq {
                    seq_ser.serialize_element(item)?;
                }
                seq_ser.end()
            }
            Value::Mapping(map) => {
                let mut map_ser = serializer.serialize_map(Some(map.len()))?;
                for (k, v) in map {
                    map_ser.serialize_entry(k, v)?;
                }
                map_ser.end()
            }
            Value::Tagged(tagged) => tagged.serialize(serializer),
        }
    }
}

impl Serialize for Number {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Number::Int(n) => serializer.serialize_i64(*n),
            Number::UInt(n) => serializer.serialize_u64(*n),
            Number::Float(f) => serializer.serialize_f64(*f),
        }
    }
}

impl Serialize for TaggedValue {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as a single-key map with tag as key
        // This is a common pattern for representing tagged values in JSON
        let mut map = serializer.serialize_map(Some(1))?;
        map.serialize_entry(&self.tag, &self.value)?;
        map.end()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    #[test]
    fn test_serialize_null() {
        let value = Value::Null;
        // Just verify it doesn't panic - actual output depends on serializer
        let _ = serde_json::to_string(&value);
    }

    #[test]
    fn test_serialize_bool() {
        assert_eq!(serde_json::to_string(&Value::Bool(true)).unwrap(), "true");
        assert_eq!(serde_json::to_string(&Value::Bool(false)).unwrap(), "false");
    }

    #[test]
    fn test_serialize_number() {
        assert_eq!(
            serde_json::to_string(&Value::Number(Number::Int(42))).unwrap(),
            "42"
        );
        assert_eq!(
            serde_json::to_string(&Value::Number(Number::UInt(42))).unwrap(),
            "42"
        );
        assert_eq!(
            serde_json::to_string(&Value::Number(Number::Float(2.5))).unwrap(),
            "2.5"
        );
    }

    #[test]
    fn test_serialize_string() {
        assert_eq!(
            serde_json::to_string(&Value::String("hello".into())).unwrap(),
            "\"hello\""
        );
    }

    #[test]
    fn test_serialize_sequence() {
        let value = Value::Sequence(vec![
            Value::Number(Number::Int(1)),
            Value::Number(Number::Int(2)),
            Value::Number(Number::Int(3)),
        ]);
        assert_eq!(serde_json::to_string(&value).unwrap(), "[1,2,3]");
    }

    #[test]
    fn test_serialize_mapping() {
        let mut map = IndexMap::new();
        map.insert(Value::String("key".into()), Value::String("value".into()));
        let value = Value::Mapping(map);
        assert_eq!(
            serde_json::to_string(&value).unwrap(),
            "{\"key\":\"value\"}"
        );
    }
}
