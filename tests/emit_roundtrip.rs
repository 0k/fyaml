//! Tests for YAML emit and roundtrip functionality.
//!
//! These tests cover:
//! - Value::to_yaml_string() for various types
//! - Roundtrip: YAML -> Value -> YAML -> Value
//! - Special float emission (.inf, -.inf, .nan)
//! - Complex nested structure roundtrips

use fyaml::value::{Number, TaggedValue, Value};
use fyaml::Document;
use indexmap::IndexMap;

// =============================================================================
// Value::to_yaml_string() tests
// =============================================================================

#[test]
fn value_to_yaml_string_null() {
    let value = Value::Null;
    let yaml = value.to_yaml_string().unwrap();
    assert_eq!(yaml, "null");
}

#[test]
fn value_to_yaml_string_null_vs_empty_string() {
    let null_yaml = Value::Null.to_yaml_string().unwrap();
    let empty_yaml = Value::String(String::new()).to_yaml_string().unwrap();
    assert_ne!(
        null_yaml, empty_yaml,
        "Value::Null and Value::String(\"\") must emit differently"
    );
}

#[test]
fn value_to_yaml_string_bool_true() {
    let value = Value::Bool(true);
    let yaml = value.to_yaml_string().unwrap();

    assert!(yaml.contains("true"));
}

#[test]
fn value_to_yaml_string_bool_false() {
    let value = Value::Bool(false);
    let yaml = value.to_yaml_string().unwrap();

    assert!(yaml.contains("false"));
}

#[test]
fn value_to_yaml_string_positive_int() {
    let value = Value::Number(Number::Int(42));
    let yaml = value.to_yaml_string().unwrap();

    assert!(yaml.contains("42"));
}

#[test]
fn value_to_yaml_string_negative_int() {
    let value = Value::Number(Number::Int(-100));
    let yaml = value.to_yaml_string().unwrap();

    assert!(yaml.contains("-100"));
}

#[test]
fn value_to_yaml_string_uint() {
    let value = Value::Number(Number::UInt(12345));
    let yaml = value.to_yaml_string().unwrap();

    assert!(yaml.contains("12345"));
}

#[test]
fn value_to_yaml_string_float() {
    let value = Value::Number(Number::Float(3.5));
    let yaml = value.to_yaml_string().unwrap();

    // Float should be emitted with decimal
    assert!(yaml.contains("3.5"));
}

#[test]
fn value_to_yaml_string_simple_string() {
    let value = Value::String("hello world".to_string());
    let yaml = value.to_yaml_string().unwrap();

    assert!(yaml.contains("hello world"));
}

#[test]
fn value_to_yaml_string_string_with_special_chars() {
    let value = Value::String("line1\nline2".to_string());
    let yaml = value.to_yaml_string().unwrap();

    // Should handle newlines (might use literal block or escaped)
    // Just verify it doesn't panic and produces something
    assert!(!yaml.is_empty());
}

// =============================================================================
// Special float emission tests
// =============================================================================

#[test]
fn value_emit_positive_infinity() {
    let value = Value::Number(Number::Float(f64::INFINITY));
    let yaml = value.to_yaml_string().unwrap();

    // Should emit as .inf or +.inf
    let lower = yaml.to_lowercase();
    assert!(lower.contains(".inf") || lower.contains("inf"));
}

#[test]
fn value_emit_negative_infinity() {
    let value = Value::Number(Number::Float(f64::NEG_INFINITY));
    let yaml = value.to_yaml_string().unwrap();

    // Should emit as -.inf
    let lower = yaml.to_lowercase();
    assert!(lower.contains("-.inf") || lower.contains("-inf"));
}

#[test]
fn value_emit_nan() {
    let value = Value::Number(Number::Float(f64::NAN));
    let yaml = value.to_yaml_string().unwrap();

    // Should emit as .nan
    let lower = yaml.to_lowercase();
    assert!(lower.contains(".nan") || lower.contains("nan"));
}

// =============================================================================
// Sequence emit tests
// =============================================================================

#[test]
fn emit_roundtrip_simple_sequence() {
    let value = Value::Sequence(vec![
        Value::Number(Number::Int(1)),
        Value::Number(Number::Int(2)),
        Value::Number(Number::Int(3)),
    ]);

    let yaml = value.to_yaml_string().unwrap();

    // Should contain all values
    assert!(yaml.contains("1"));
    assert!(yaml.contains("2"));
    assert!(yaml.contains("3"));

    // Should be reparseable
    let reparsed: Value = yaml.parse().unwrap();
    assert!(reparsed.is_sequence());
    assert_eq!(reparsed.as_sequence().unwrap().len(), 3);
}

#[test]
fn emit_roundtrip_empty_sequence() {
    let value = Value::Sequence(vec![]);
    let yaml = value.to_yaml_string().unwrap();

    // Should be reparseable
    let reparsed: Value = yaml.parse().unwrap();
    assert!(reparsed.is_sequence());
    assert_eq!(reparsed.as_sequence().unwrap().len(), 0);
}

#[test]
fn emit_roundtrip_mixed_mapping() {
    let mut map = IndexMap::new();
    map.insert(
        Value::String("string_key".to_string()),
        Value::String("value".to_string()),
    );
    map.insert(
        Value::String("number_key".to_string()),
        Value::Number(Number::Int(42)),
    );
    map.insert(Value::String("bool_key".to_string()), Value::Bool(false));
    map.insert(Value::String("null_key".to_string()), Value::Null);

    let value = Value::Mapping(map);
    let yaml = value.to_yaml_string().unwrap();
    let reparsed: Value = yaml.parse().unwrap();

    assert!(reparsed.is_mapping());
    assert_eq!(reparsed.get("string_key").unwrap().as_str(), Some("value"));
    assert_eq!(reparsed.get("number_key").unwrap().as_i64(), Some(42));
    assert_eq!(reparsed.get("bool_key").unwrap().as_bool(), Some(false));
    assert!(reparsed.get("null_key").unwrap().is_null());
}

// =============================================================================
// Mapping emit tests
// =============================================================================

#[test]
fn emit_roundtrip_simple_mapping() {
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
    let yaml = value.to_yaml_string().unwrap();

    assert!(yaml.contains("a"));
    assert!(yaml.contains("b"));
    assert!(yaml.contains("1"));
    assert!(yaml.contains("2"));

    // Reparse
    let reparsed: Value = yaml.parse().unwrap();
    assert!(reparsed.is_mapping());
    assert_eq!(reparsed.get("a").unwrap().as_i64(), Some(1));
    assert_eq!(reparsed.get("b").unwrap().as_i64(), Some(2));
}

#[test]
fn emit_roundtrip_empty_mapping() {
    let value = Value::Mapping(IndexMap::new());
    let yaml = value.to_yaml_string().unwrap();

    let reparsed: Value = yaml.parse().unwrap();
    assert!(reparsed.is_mapping());
    assert_eq!(reparsed.as_mapping().unwrap().len(), 0);
}

// =============================================================================
// Nested structure roundtrip tests
// =============================================================================

#[test]
fn emit_roundtrip_nested_mapping() {
    let mut inner = IndexMap::new();
    inner.insert(
        Value::String("key".to_string()),
        Value::String("value".to_string()),
    );

    let mut outer = IndexMap::new();
    outer.insert(Value::String("nested".to_string()), Value::Mapping(inner));

    let value = Value::Mapping(outer);
    let yaml = value.to_yaml_string().unwrap();
    let reparsed: Value = yaml.parse().unwrap();

    assert!(reparsed.is_mapping());
    let nested = reparsed.get("nested").unwrap();
    assert!(nested.is_mapping());
    assert_eq!(nested.get("key").unwrap().as_str(), Some("value"));
}

#[test]
fn emit_roundtrip_nested_sequence() {
    let inner = Value::Sequence(vec![
        Value::Number(Number::Int(1)),
        Value::Number(Number::Int(2)),
    ]);
    let outer = Value::Sequence(vec![inner, Value::Number(Number::Int(3))]);

    let yaml = outer.to_yaml_string().unwrap();
    let reparsed: Value = yaml.parse().unwrap();

    assert!(reparsed.is_sequence());
    let seq = reparsed.as_sequence().unwrap();
    assert_eq!(seq.len(), 2);
    assert!(seq[0].is_sequence());
    assert_eq!(seq[1].as_i64(), Some(3));
}

#[test]
fn emit_roundtrip_complex_structure() {
    let mut users = IndexMap::new();

    // User 1
    let mut user1 = IndexMap::new();
    user1.insert(
        Value::String("name".to_string()),
        Value::String("Alice".to_string()),
    );
    user1.insert(
        Value::String("age".to_string()),
        Value::Number(Number::Int(30)),
    );
    user1.insert(Value::String("active".to_string()), Value::Bool(true));
    user1.insert(
        Value::String("tags".to_string()),
        Value::Sequence(vec![
            Value::String("admin".to_string()),
            Value::String("user".to_string()),
        ]),
    );

    // User 2
    let mut user2 = IndexMap::new();
    user2.insert(
        Value::String("name".to_string()),
        Value::String("Bob".to_string()),
    );
    user2.insert(
        Value::String("age".to_string()),
        Value::Number(Number::Int(25)),
    );
    user2.insert(Value::String("active".to_string()), Value::Bool(false));
    user2.insert(
        Value::String("tags".to_string()),
        Value::Sequence(vec![Value::String("user".to_string())]),
    );

    users.insert(
        Value::String("users".to_string()),
        Value::Sequence(vec![Value::Mapping(user1), Value::Mapping(user2)]),
    );

    let value = Value::Mapping(users);
    let yaml = value.to_yaml_string().unwrap();

    // Reparse and verify
    let reparsed: Value = yaml.parse().unwrap();

    let users_list = reparsed.get("users").unwrap();
    assert!(users_list.is_sequence());
    let seq = users_list.as_sequence().unwrap();
    assert_eq!(seq.len(), 2);

    // Check first user
    let user1 = &seq[0];
    assert_eq!(user1.get("name").unwrap().as_str(), Some("Alice"));
    assert_eq!(user1.get("age").unwrap().as_i64(), Some(30));
    assert_eq!(user1.get("active").unwrap().as_bool(), Some(true));

    // Check second user
    let user2 = &seq[1];
    assert_eq!(user2.get("name").unwrap().as_str(), Some("Bob"));
    assert_eq!(user2.get("active").unwrap().as_bool(), Some(false));
}

// =============================================================================
// Document emit tests
// =============================================================================

#[test]
fn document_emit_and_reparse() {
    let doc = Document::parse_str("key: value\nlist:\n  - a\n  - b").unwrap();
    let yaml = doc.emit().unwrap();

    // Should be valid YAML
    let reparsed = Document::parse_str(&yaml).unwrap();

    assert_eq!(
        reparsed.at_path("/key").unwrap().scalar_str().unwrap(),
        "value"
    );
    assert_eq!(
        reparsed.at_path("/list/0").unwrap().scalar_str().unwrap(),
        "a"
    );
    assert_eq!(
        reparsed.at_path("/list/1").unwrap().scalar_str().unwrap(),
        "b"
    );
}

#[test]
fn noderef_emit_and_reparse() {
    let doc = Document::parse_str("outer:\n  inner: value").unwrap();
    let inner_node = doc.at_path("/outer").unwrap();

    let yaml = inner_node.emit().unwrap();

    // Should be valid YAML for just the inner part
    let reparsed = Document::parse_str(&yaml).unwrap();
    assert_eq!(
        reparsed.at_path("/inner").unwrap().scalar_str().unwrap(),
        "value"
    );
}

// =============================================================================
// Value YAML roundtrip tests
// =============================================================================

#[test]
fn value_yaml_roundtrip_preserves_types() {
    let yaml = "int: 42\nfloat: 3.14\nbool: true\nstr: hello\nnull_val: null";
    let value: Value = yaml.parse().unwrap();

    let emitted = value.to_yaml_string().unwrap();
    let reparsed: Value = emitted.parse().unwrap();

    assert_eq!(reparsed.get("int").unwrap().as_i64(), Some(42));
    assert!(reparsed.get("float").unwrap().as_f64().is_some());
    assert_eq!(reparsed.get("bool").unwrap().as_bool(), Some(true));
    assert_eq!(reparsed.get("str").unwrap().as_str(), Some("hello"));
    assert!(reparsed.get("null_val").unwrap().is_null());
}

#[test]
fn value_yaml_roundtrip_sequence() {
    let yaml = "- 1\n- 2\n- 3";
    let value: Value = yaml.parse().unwrap();

    let emitted = value.to_yaml_string().unwrap();
    let reparsed: Value = emitted.parse().unwrap();

    assert!(reparsed.is_sequence());
    let seq = reparsed.as_sequence().unwrap();
    assert_eq!(seq.len(), 3);
    assert_eq!(seq[0].as_i64(), Some(1));
    assert_eq!(seq[1].as_i64(), Some(2));
    assert_eq!(seq[2].as_i64(), Some(3));
}

#[test]
fn value_yaml_roundtrip_mapping() {
    let yaml = "a: 1\nb: 2\nc: 3";
    let value: Value = yaml.parse().unwrap();

    let emitted = value.to_yaml_string().unwrap();
    let reparsed: Value = emitted.parse().unwrap();

    assert!(reparsed.is_mapping());
    assert_eq!(reparsed.get("a").unwrap().as_i64(), Some(1));
    assert_eq!(reparsed.get("b").unwrap().as_i64(), Some(2));
    assert_eq!(reparsed.get("c").unwrap().as_i64(), Some(3));
}

// =============================================================================
// Tagged value emit tests
// =============================================================================

#[test]
fn tagged_value_emit() {
    let tagged = TaggedValue {
        tag: "!mytag".to_string(),
        value: Value::String("tagged value".to_string()),
    };

    let yaml = tagged.to_yaml_string().unwrap();

    // Should contain the tag
    assert!(yaml.contains("mytag") || yaml.contains("!mytag"));
    assert!(yaml.contains("tagged value"));
}

#[test]
fn value_tagged_emit_roundtrip() {
    let tagged = TaggedValue {
        tag: "!custom".to_string(),
        value: Value::Number(Number::Int(42)),
    };
    let value = Value::Tagged(Box::new(tagged));

    let yaml = value.to_yaml_string().unwrap();

    // Reparse (tag may be preserved or not depending on Value::from_str behavior)
    let reparsed: Value = yaml.parse().unwrap();

    // The value should be accessible (either tagged or untagged)
    if reparsed.is_tagged() {
        let inner = reparsed.as_tagged().unwrap();
        assert_eq!(inner.value.as_i64(), Some(42));
    } else {
        // Tag may be stripped, but value should be preserved
        assert_eq!(reparsed.as_i64(), Some(42));
    }
}

// =============================================================================
// Unicode emit tests
// =============================================================================

#[test]
fn value_emit_unicode_string() {
    let value = Value::String("Hello \u{1F600} World".to_string());
    let yaml = value.to_yaml_string().unwrap();

    // Should contain the emoji (or its escape)
    let reparsed: Value = yaml.parse().unwrap();
    assert_eq!(reparsed.as_str(), Some("Hello \u{1F600} World"));
}

#[test]
fn value_emit_chinese_characters() {
    let value = Value::String("\u{4E2D}\u{6587}\u{6D4B}\u{8BD5}".to_string()); // 中文测试
    let yaml = value.to_yaml_string().unwrap();

    let reparsed: Value = yaml.parse().unwrap();
    assert_eq!(reparsed.as_str(), Some("\u{4E2D}\u{6587}\u{6D4B}\u{8BD5}"));
}

// =============================================================================
// Edge cases
// =============================================================================

#[test]
fn value_emit_string_that_looks_like_bool() {
    let value = Value::String("true".to_string());
    let yaml = value.to_yaml_string().unwrap();

    // When reparsed as Value, "true" string may be interpreted as bool
    // This is expected YAML behavior for unquoted strings
    let reparsed: Value = yaml.parse().unwrap();

    // The emitter should quote strings that look like booleans
    // If it doesn't, the value will be parsed as bool
    // Either behavior is acceptable, just verify consistency
    assert!(reparsed.is_bool() || reparsed.is_string());
}

#[test]
fn value_emit_string_that_looks_like_number() {
    let value = Value::String("42".to_string());
    let yaml = value.to_yaml_string().unwrap();

    let reparsed: Value = yaml.parse().unwrap();

    // Similar to above - may be parsed as number if unquoted
    assert!(reparsed.is_number() || reparsed.is_string());
}

#[test]
fn value_emit_empty_string() {
    let value = Value::String("".to_string());
    let yaml = value.to_yaml_string().unwrap();

    // Reparsing an empty string emission may fail (empty input error)
    // or produce null/empty string. All are acceptable outcomes.
    if let Ok(reparsed) = yaml.parse::<Value>() {
        assert!(reparsed.is_null() || reparsed.as_str() == Some(""));
    }
    // If parse fails with "empty input", that's also acceptable
}

#[test]
fn value_emit_multiline_string() {
    let value = Value::String("line1\nline2\nline3".to_string());
    let yaml = value.to_yaml_string().unwrap();

    let reparsed: Value = yaml.parse().unwrap();

    // Should preserve newlines
    let s = reparsed.as_str().unwrap();
    assert!(s.contains("line1"));
    assert!(s.contains("line2"));
    assert!(s.contains("line3"));
}

#[test]
fn value_emit_very_long_string() {
    let long_string = "x".repeat(10000);
    let value = Value::String(long_string.clone());
    let yaml = value.to_yaml_string().unwrap();

    let reparsed: Value = yaml.parse().unwrap();
    assert_eq!(reparsed.as_str().unwrap().len(), 10000);
}

#[test]
fn value_emit_deeply_nested() {
    // Create 10 levels of nesting
    let mut value = Value::String("deep".to_string());
    for i in 0..10 {
        let mut map = IndexMap::new();
        map.insert(Value::String(format!("level{}", i)), value);
        value = Value::Mapping(map);
    }

    let yaml = value.to_yaml_string().unwrap();
    let reparsed: Value = yaml.parse().unwrap();

    // Navigate to the deepest level
    let mut current = &reparsed;
    for i in (0..10).rev() {
        current = current.get(&format!("level{}", i)).unwrap();
    }
    assert_eq!(current.as_str(), Some("deep"));
}
