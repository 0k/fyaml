//! Tests asserting that emit() and to_yaml_string() do NOT add trailing newlines.
//!
//! libfyaml's C function `fy_emit_document_to_string` adds a trailing `\n`
//! (document-level convention), but `fy_emit_node_to_string` does not.
//!
//! fyaml uses node-level emission for both `NodeRef::emit()` and
//! `Value::to_yaml_string()` because they return value representations,
//! not complete YAML documents. Only `Document::emit()` uses document-level
//! emission and retains the trailing `\n`.
//!
//! See README.org "Differences from libfyaml C API" for rationale.

use fyaml::value::{Number, Value};
use fyaml::Document;
use indexmap::IndexMap;

// =============================================================================
// NodeRef::emit() — no trailing newline
// =============================================================================

#[test]
fn noderef_emit_scalar_no_trailing_newline() {
    let doc = Document::parse_str("key: hello").unwrap();
    let node = doc.root().unwrap().at_path("/key").unwrap();

    let emitted = node.emit().unwrap();
    assert_eq!(emitted, "hello", "scalar emit() should not end with \\n");
}

#[test]
fn noderef_emit_integer_no_trailing_newline() {
    let doc = Document::parse_str("key: 42").unwrap();
    let node = doc.root().unwrap().at_path("/key").unwrap();

    let emitted = node.emit().unwrap();
    assert_eq!(emitted, "42");
}

#[test]
fn noderef_emit_quoted_scalar_no_trailing_newline() {
    let doc = Document::parse_str("value: \"bar: wiz\"").unwrap();
    let node = doc.root().unwrap().at_path("/value").unwrap();

    let emitted = node.emit().unwrap();
    assert_eq!(emitted, "\"bar: wiz\"");
}

#[test]
fn noderef_emit_mapping_no_trailing_newline() {
    let doc = Document::parse_str("outer:\n  a: 1\n  b: 2").unwrap();
    let node = doc.root().unwrap().at_path("/outer").unwrap();

    let emitted = node.emit().unwrap();
    // Block mapping: structural newlines between entries, but no trailing \n
    assert!(
        !emitted.ends_with('\n'),
        "mapping emit() should not end with \\n, got: {:?}",
        emitted
    );
    assert!(emitted.contains("a: 1"));
    assert!(emitted.contains("b: 2"));
}

#[test]
fn noderef_emit_sequence_no_trailing_newline() {
    let doc = Document::parse_str("items:\n  - one\n  - two").unwrap();
    let node = doc.root().unwrap().at_path("/items").unwrap();

    let emitted = node.emit().unwrap();
    // Block sequence: structural newlines between items, but no trailing \n
    assert!(
        !emitted.ends_with('\n'),
        "sequence emit() should not end with \\n, got: {:?}",
        emitted
    );
    assert!(emitted.contains("- one"));
    assert!(emitted.contains("- two"));
}

#[test]
fn noderef_emit_root_mapping_no_trailing_newline() {
    let doc = Document::parse_str("foo: bar").unwrap();
    let root = doc.root().unwrap();

    let emitted = root.emit().unwrap();
    assert!(
        !emitted.ends_with('\n'),
        "root mapping emit() should not end with \\n, got: {:?}",
        emitted
    );
}

// =============================================================================
// Value::to_yaml_string() — no trailing newline
// =============================================================================

#[test]
fn value_to_yaml_string_scalar_no_trailing_newline() {
    let value = Value::Bool(true);
    let yaml = value.to_yaml_string().unwrap();
    assert_eq!(
        yaml, "true",
        "Bool to_yaml_string() should not end with \\n"
    );
}

#[test]
fn value_to_yaml_string_integer_no_trailing_newline() {
    let value = Value::Number(Number::Int(42));
    let yaml = value.to_yaml_string().unwrap();
    assert_eq!(yaml, "42");
}

#[test]
fn value_to_yaml_string_string_no_trailing_newline() {
    let value = Value::String("hello".to_string());
    let yaml = value.to_yaml_string().unwrap();
    assert!(
        !yaml.ends_with('\n'),
        "String to_yaml_string() should not end with \\n, got: {:?}",
        yaml
    );
    assert!(yaml.contains("hello"));
}

#[test]
fn value_to_yaml_string_mapping_no_trailing_newline() {
    let mut map = IndexMap::new();
    map.insert(Value::String("key".into()), Value::String("value".into()));
    let value = Value::Mapping(map);
    let yaml = value.to_yaml_string().unwrap();
    assert!(
        !yaml.ends_with('\n'),
        "Mapping to_yaml_string() should not end with \\n, got: {:?}",
        yaml
    );
}

#[test]
fn value_to_yaml_string_sequence_no_trailing_newline() {
    let value = Value::Sequence(vec![
        Value::Number(Number::Int(1)),
        Value::Number(Number::Int(2)),
    ]);
    let yaml = value.to_yaml_string().unwrap();
    assert!(
        !yaml.ends_with('\n'),
        "Sequence to_yaml_string() should not end with \\n, got: {:?}",
        yaml
    );
}

// =============================================================================
// Document::emit() — DOES keep trailing newline (document-level)
// =============================================================================

#[test]
fn document_emit_keeps_trailing_newline() {
    let doc = Document::parse_str("foo: bar").unwrap();
    let emitted = doc.emit().unwrap();
    assert!(
        emitted.ends_with('\n'),
        "Document::emit() SHOULD end with \\n (document-level), got: {:?}",
        emitted
    );
}

// =============================================================================
// Roundtrip still works after stripping trailing newline
// =============================================================================

#[test]
fn noderef_emit_roundtrip_still_works() {
    let doc = Document::parse_str("key: value\nlist:\n  - a\n  - b").unwrap();
    let root = doc.root().unwrap();

    let emitted = root.emit().unwrap();
    // Even without trailing \n, the emitted YAML should be re-parseable
    let reparsed = Document::parse_str(&emitted).unwrap();
    assert_eq!(
        reparsed.at_path("/key").unwrap().scalar_str().unwrap(),
        "value"
    );
    assert_eq!(
        reparsed.at_path("/list/0").unwrap().scalar_str().unwrap(),
        "a"
    );
}

#[test]
fn value_to_yaml_string_roundtrip_still_works() {
    let mut map = IndexMap::new();
    map.insert(Value::String("name".into()), Value::String("Alice".into()));
    map.insert(Value::String("age".into()), Value::Number(Number::Int(30)));
    let value = Value::Mapping(map);

    let yaml = value.to_yaml_string().unwrap();
    let reparsed: Value = yaml.parse().unwrap();
    assert_eq!(reparsed.get("name").unwrap().as_str(), Some("Alice"));
    assert_eq!(reparsed.get("age").unwrap().as_i64(), Some(30));
}
