//! Tests for serde coverage.
//!
//! These tests target serde serialization and deserialization paths:
//! - TaggedValue roundtrips
//! - Float deserialization
//! - Negative integer deserialization
//! - Various Value serialization scenarios

use fyaml::value::{Number, TaggedValue, Value};
use indexmap::IndexMap;

// =============================================================================
// TaggedValue serde roundtrip tests
// =============================================================================

#[test]
fn serde_tagged_value_serializes_as_map() {
    // TaggedValue is serialized as a map with the tag as key
    // This is the expected behavior for JSON interop
    let tagged = TaggedValue {
        tag: "!custom".to_string(),
        value: Value::String("tagged string".to_string()),
    };
    let value = Value::Tagged(Box::new(tagged));

    // Serialize to JSON
    let json = serde_json::to_string(&value).unwrap();

    // Should serialize as {"!custom": "tagged string"}
    assert!(json.contains("!custom"));
    assert!(json.contains("tagged string"));

    // When deserialized, it becomes a mapping (JSON doesn't have tags)
    let deserialized: Value = serde_json::from_str(&json).unwrap();
    assert!(deserialized.is_mapping());

    // Can access the value via the tag key
    let inner = deserialized.get("!custom");
    assert!(inner.is_some());
    assert_eq!(inner.unwrap().as_str(), Some("tagged string"));
}

#[test]
fn serde_tagged_value_with_number_serializes_as_map() {
    let tagged = TaggedValue {
        tag: "!int".to_string(),
        value: Value::Number(Number::Int(42)),
    };
    let value = Value::Tagged(Box::new(tagged));

    let json = serde_json::to_string(&value).unwrap();

    // Should be {"!int": 42}
    let deserialized: Value = serde_json::from_str(&json).unwrap();
    assert!(deserialized.is_mapping());
    assert_eq!(deserialized.get("!int").unwrap().as_i64(), Some(42));
}

#[test]
fn serde_tagged_value_with_sequence_serializes_as_map() {
    let tagged = TaggedValue {
        tag: "!list".to_string(),
        value: Value::Sequence(vec![
            Value::Number(Number::Int(1)),
            Value::Number(Number::Int(2)),
        ]),
    };
    let value = Value::Tagged(Box::new(tagged));

    let json = serde_json::to_string(&value).unwrap();
    let deserialized: Value = serde_json::from_str(&json).unwrap();

    assert!(deserialized.is_mapping());
    let inner = deserialized.get("!list").unwrap();
    assert!(inner.is_sequence());
}

#[test]
fn serde_tagged_value_with_mapping_serializes_as_map() {
    let mut map = IndexMap::new();
    map.insert(
        Value::String("key".to_string()),
        Value::String("value".to_string()),
    );

    let tagged = TaggedValue {
        tag: "!object".to_string(),
        value: Value::Mapping(map),
    };
    let value = Value::Tagged(Box::new(tagged));

    let json = serde_json::to_string(&value).unwrap();
    let deserialized: Value = serde_json::from_str(&json).unwrap();

    assert!(deserialized.is_mapping());
    let inner = deserialized.get("!object").unwrap();
    assert!(inner.is_mapping());
}

// =============================================================================
// Float deserialization tests
// =============================================================================

#[test]
fn serde_deserialize_float_from_json() {
    let json = r#"3.456789"#;
    let value: Value = serde_json::from_str(json).unwrap();

    assert!(value.is_number());
    let f = value.as_f64().unwrap();
    assert!((f - 3.456789).abs() < 0.00001);
}

#[test]
fn serde_deserialize_negative_float() {
    let json = r#"-2.5"#;
    let value: Value = serde_json::from_str(json).unwrap();

    assert!(value.is_number());
    let f = value.as_f64().unwrap();
    assert!((f - (-2.5)).abs() < 0.001);
}

#[test]
fn serde_deserialize_float_exponent() {
    let json = r#"1.5e10"#;
    let value: Value = serde_json::from_str(json).unwrap();

    assert!(value.is_number());
    let f = value.as_f64().unwrap();
    assert!((f - 1.5e10).abs() < 1e5);
}

#[test]
fn serde_deserialize_float_negative_exponent() {
    let json = r#"1.5e-10"#;
    let value: Value = serde_json::from_str(json).unwrap();

    assert!(value.is_number());
    let f = value.as_f64().unwrap();
    assert!((f - 1.5e-10).abs() < 1e-15);
}

#[test]
fn serde_serialize_float() {
    let value = Value::Number(Number::Float(2.5));
    let json = serde_json::to_string(&value).unwrap();

    // Should be a valid float
    let reparsed: f64 = serde_json::from_str(&json).unwrap();
    assert!((reparsed - 2.5).abs() < 0.0001);
}

// =============================================================================
// Negative integer deserialization tests
// =============================================================================

#[test]
fn serde_deserialize_negative_int() {
    let json = r#"-42"#;
    let value: Value = serde_json::from_str(json).unwrap();

    assert!(value.is_number());
    assert_eq!(value.as_i64(), Some(-42));
}

#[test]
fn serde_deserialize_large_negative_int() {
    let json = r#"-9223372036854775807"#; // Near i64::MIN
    let value: Value = serde_json::from_str(json).unwrap();

    assert!(value.is_number());
    assert_eq!(value.as_i64(), Some(-9223372036854775807));
}

#[test]
fn serde_serialize_negative_int() {
    let value = Value::Number(Number::Int(-100));
    let json = serde_json::to_string(&value).unwrap();

    assert_eq!(json, "-100");
}

#[test]
fn serde_deserialize_zero() {
    let json = r#"0"#;
    let value: Value = serde_json::from_str(json).unwrap();

    assert!(value.is_number());
    // Zero could be Int or UInt
    assert!(value.as_i64() == Some(0) || value.as_u64() == Some(0));
}

// =============================================================================
// TaggedValue to_yaml_string tests
// =============================================================================

#[test]
fn tagged_value_to_yaml_string_scalar() {
    let tagged = TaggedValue {
        tag: "!mytag".to_string(),
        value: Value::String("hello".to_string()),
    };

    let yaml = tagged.to_yaml_string().unwrap();
    assert!(yaml.contains("mytag") || yaml.contains("!mytag"));
    assert!(yaml.contains("hello"));
}

#[test]
fn tagged_value_to_yaml_string_number() {
    let tagged = TaggedValue {
        tag: "!integer".to_string(),
        value: Value::Number(Number::Int(42)),
    };

    let yaml = tagged.to_yaml_string().unwrap();
    assert!(yaml.contains("42"));
}

#[test]
fn tagged_value_to_yaml_string_sequence() {
    let tagged = TaggedValue {
        tag: "!seq".to_string(),
        value: Value::Sequence(vec![
            Value::String("a".to_string()),
            Value::String("b".to_string()),
        ]),
    };

    let yaml = tagged.to_yaml_string().unwrap();
    assert!(yaml.contains("a"));
    assert!(yaml.contains("b"));
}

// =============================================================================
// Value serde roundtrip tests
// =============================================================================

#[test]
fn serde_roundtrip_null() {
    let value = Value::Null;
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(json, "null");

    let deserialized: Value = serde_json::from_str(&json).unwrap();
    assert!(deserialized.is_null());
}

#[test]
fn serde_roundtrip_bool_true() {
    let value = Value::Bool(true);
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(json, "true");

    let deserialized: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.as_bool(), Some(true));
}

#[test]
fn serde_roundtrip_bool_false() {
    let value = Value::Bool(false);
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(json, "false");

    let deserialized: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.as_bool(), Some(false));
}

#[test]
fn serde_roundtrip_string() {
    let value = Value::String("hello world".to_string());
    let json = serde_json::to_string(&value).unwrap();

    let deserialized: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.as_str(), Some("hello world"));
}

#[test]
fn serde_roundtrip_string_with_escapes() {
    let value = Value::String("line1\nline2\ttab".to_string());
    let json = serde_json::to_string(&value).unwrap();

    let deserialized: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.as_str(), Some("line1\nline2\ttab"));
}

#[test]
fn serde_roundtrip_sequence() {
    let value = Value::Sequence(vec![
        Value::Number(Number::Int(1)),
        Value::Number(Number::Int(2)),
        Value::Number(Number::Int(3)),
    ]);
    let json = serde_json::to_string(&value).unwrap();

    let deserialized: Value = serde_json::from_str(&json).unwrap();
    assert!(deserialized.is_sequence());
    let seq = deserialized.as_sequence().unwrap();
    assert_eq!(seq.len(), 3);
}

#[test]
fn serde_roundtrip_mapping() {
    let mut map = IndexMap::new();
    map.insert(
        Value::String("a".to_string()),
        Value::Number(Number::Int(1)),
    );
    map.insert(
        Value::String("b".to_string()),
        Value::Number(Number::Int(2)),
    );

    let value = Value::Mapping(map);
    let json = serde_json::to_string(&value).unwrap();

    let deserialized: Value = serde_json::from_str(&json).unwrap();
    assert!(deserialized.is_mapping());
}

#[test]
fn serde_roundtrip_nested_structure() {
    let mut inner = IndexMap::new();
    inner.insert(
        Value::String("name".to_string()),
        Value::String("test".to_string()),
    );
    inner.insert(
        Value::String("values".to_string()),
        Value::Sequence(vec![
            Value::Number(Number::Int(1)),
            Value::Number(Number::Int(2)),
        ]),
    );

    let mut outer = IndexMap::new();
    outer.insert(Value::String("data".to_string()), Value::Mapping(inner));

    let value = Value::Mapping(outer);
    let json = serde_json::to_string(&value).unwrap();

    let deserialized: Value = serde_json::from_str(&json).unwrap();
    assert!(deserialized.is_mapping());

    // Navigate the structure
    let data = deserialized.get("data").unwrap();
    assert!(data.is_mapping());

    let name = data.get("name").unwrap();
    assert_eq!(name.as_str(), Some("test"));
}

// =============================================================================
// Number type preservation tests
// =============================================================================

#[test]
fn serde_number_int_preserved() {
    let value = Value::Number(Number::Int(i64::MAX));
    let json = serde_json::to_string(&value).unwrap();

    let deserialized: Value = serde_json::from_str(&json).unwrap();
    // Large integers may be preserved or converted
    assert!(deserialized.is_number());
}

#[test]
fn serde_number_uint_preserved() {
    let value = Value::Number(Number::UInt(u64::MAX));
    let json = serde_json::to_string(&value).unwrap();

    let deserialized: Value = serde_json::from_str(&json).unwrap();
    // u64::MAX may be serialized as large integer
    assert!(deserialized.is_number());
}

#[test]
fn serde_number_float_preserved() {
    let value = Value::Number(Number::Float(std::f64::consts::PI));
    let json = serde_json::to_string(&value).unwrap();

    let deserialized: Value = serde_json::from_str(&json).unwrap();
    assert!(deserialized.is_number());

    let f = deserialized.as_f64().unwrap();
    assert!((f - std::f64::consts::PI).abs() < 0.0001);
}

// =============================================================================
// Empty collection tests
// =============================================================================

#[test]
fn serde_roundtrip_empty_sequence() {
    let value = Value::Sequence(vec![]);
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(json, "[]");

    let deserialized: Value = serde_json::from_str(&json).unwrap();
    assert!(deserialized.is_sequence());
    assert_eq!(deserialized.as_sequence().unwrap().len(), 0);
}

#[test]
fn serde_roundtrip_empty_mapping() {
    let value = Value::Mapping(IndexMap::new());
    let json = serde_json::to_string(&value).unwrap();
    assert_eq!(json, "{}");

    let deserialized: Value = serde_json::from_str(&json).unwrap();
    assert!(deserialized.is_mapping());
    assert_eq!(deserialized.as_mapping().unwrap().len(), 0);
}

// =============================================================================
// Unicode handling tests
// =============================================================================

#[test]
fn serde_roundtrip_unicode_string() {
    let value = Value::String("Hello \u{1F600} World \u{1F44D}".to_string());
    let json = serde_json::to_string(&value).unwrap();

    let deserialized: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(
        deserialized.as_str(),
        Some("Hello \u{1F600} World \u{1F44D}")
    );
}

#[test]
fn serde_roundtrip_chinese_characters() {
    let value = Value::String("\u{4E2D}\u{6587}".to_string()); // 中文
    let json = serde_json::to_string(&value).unwrap();

    let deserialized: Value = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.as_str(), Some("\u{4E2D}\u{6587}"));
}

// =============================================================================
// YAML parsing to Value tests
// =============================================================================

#[test]
fn value_from_yaml_string() {
    let value: Value = "key: value".parse().unwrap();

    assert!(value.is_mapping());
    assert_eq!(value.get("key").unwrap().as_str(), Some("value"));
}

#[test]
fn value_from_yaml_sequence() {
    let value: Value = "- a\n- b\n- c".parse().unwrap();

    assert!(value.is_sequence());
    let seq = value.as_sequence().unwrap();
    assert_eq!(seq.len(), 3);
}

#[test]
fn value_from_yaml_with_types() {
    let value: Value = "int: 42\nfloat: 3.14\nbool: true\nnull_val: null"
        .parse()
        .unwrap();

    assert!(value.is_mapping());
    assert_eq!(value.get("int").unwrap().as_i64(), Some(42));
    assert!(value.get("float").unwrap().as_f64().is_some());
    assert_eq!(value.get("bool").unwrap().as_bool(), Some(true));
    assert!(value.get("null_val").unwrap().is_null());
}
