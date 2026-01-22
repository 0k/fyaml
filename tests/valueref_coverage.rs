//! Tests for ValueRef coverage.
//!
//! These tests target ValueRef methods that may have lower coverage:
//! - `seq_len()` / `map_len()` on wrong types
//! - `as_bytes()` method
//! - Debug formatting for different value types
//! - Index out of bounds scenarios

use fyaml::Document;

// =============================================================================
// seq_len() and map_len() on wrong types
// =============================================================================

#[test]
fn valueref_seq_len_on_scalar_returns_none() {
    let doc = Document::parse_str("scalar: value").unwrap();
    let root = doc.root_value().unwrap();
    let scalar = root.get("scalar").unwrap();

    // seq_len on a scalar should return None
    assert!(scalar.seq_len().is_none());
}

#[test]
fn valueref_seq_len_on_mapping_returns_none() {
    let doc = Document::parse_str("mapping:\n  key: value").unwrap();
    let root = doc.root_value().unwrap();
    let mapping = root.get("mapping").unwrap();

    // seq_len on a mapping should return None
    assert!(mapping.seq_len().is_none());
}

#[test]
fn valueref_seq_len_on_sequence_returns_count() {
    let doc = Document::parse_str("seq:\n  - a\n  - b\n  - c").unwrap();
    let root = doc.root_value().unwrap();
    let seq = root.get("seq").unwrap();

    assert_eq!(seq.seq_len(), Some(3));
}

#[test]
fn valueref_map_len_on_scalar_returns_none() {
    let doc = Document::parse_str("scalar: value").unwrap();
    let root = doc.root_value().unwrap();
    let scalar = root.get("scalar").unwrap();

    // map_len on a scalar should return None
    assert!(scalar.map_len().is_none());
}

#[test]
fn valueref_map_len_on_sequence_returns_none() {
    let doc = Document::parse_str("seq:\n  - item1\n  - item2").unwrap();
    let root = doc.root_value().unwrap();
    let seq = root.get("seq").unwrap();

    // map_len on a sequence should return None
    assert!(seq.map_len().is_none());
}

#[test]
fn valueref_map_len_on_mapping_returns_count() {
    let doc = Document::parse_str("mapping:\n  a: 1\n  b: 2\n  c: 3").unwrap();
    let root = doc.root_value().unwrap();
    let mapping = root.get("mapping").unwrap();

    assert_eq!(mapping.map_len(), Some(3));
}

// =============================================================================
// as_bytes() tests
// =============================================================================

#[test]
fn valueref_as_bytes_returns_raw_bytes() {
    let doc = Document::parse_str("data: hello").unwrap();
    let root = doc.root_value().unwrap();
    let data = root.get("data").unwrap();

    let bytes = data.as_bytes();
    assert_eq!(bytes, Some(b"hello".as_slice()));
}

#[test]
fn valueref_as_bytes_on_sequence_returns_none() {
    let doc = Document::parse_str("seq:\n  - item").unwrap();
    let root = doc.root_value().unwrap();
    let seq = root.get("seq").unwrap();

    assert!(seq.as_bytes().is_none());
}

#[test]
fn valueref_as_bytes_on_mapping_returns_none() {
    let doc = Document::parse_str("map:\n  key: value").unwrap();
    let root = doc.root_value().unwrap();
    let map = root.get("map").unwrap();

    assert!(map.as_bytes().is_none());
}

#[test]
fn valueref_as_bytes_with_unicode() {
    let doc = Document::parse_str("unicode: \u{1F600}").unwrap();
    let root = doc.root_value().unwrap();
    let unicode = root.get("unicode").unwrap();

    let bytes = unicode.as_bytes().unwrap();
    // UTF-8 encoding of U+1F600 (grinning face) is 4 bytes: F0 9F 98 80
    assert_eq!(bytes, "\u{1F600}".as_bytes());
}

// =============================================================================
// Debug formatting tests
// =============================================================================

#[test]
fn valueref_debug_format_null() {
    let doc = Document::parse_str("value: null").unwrap();
    let root = doc.root_value().unwrap();
    let null_val = root.get("value").unwrap();

    let debug = format!("{:?}", null_val);
    assert!(debug.contains("ValueRef"));
    assert!(debug.contains("null"));
}

#[test]
fn valueref_debug_format_bool_true() {
    let doc = Document::parse_str("value: true").unwrap();
    let root = doc.root_value().unwrap();
    let bool_val = root.get("value").unwrap();

    let debug = format!("{:?}", bool_val);
    assert!(debug.contains("ValueRef"));
    assert!(debug.contains("true"));
}

#[test]
fn valueref_debug_format_bool_false() {
    let doc = Document::parse_str("value: false").unwrap();
    let root = doc.root_value().unwrap();
    let bool_val = root.get("value").unwrap();

    let debug = format!("{:?}", bool_val);
    assert!(debug.contains("ValueRef"));
    assert!(debug.contains("false"));
}

#[test]
fn valueref_debug_format_integer() {
    let doc = Document::parse_str("value: 42").unwrap();
    let root = doc.root_value().unwrap();
    let int_val = root.get("value").unwrap();

    let debug = format!("{:?}", int_val);
    assert!(debug.contains("ValueRef"));
    assert!(debug.contains("42"));
}

#[test]
fn valueref_debug_format_float() {
    let doc = Document::parse_str("value: 3.14").unwrap();
    let root = doc.root_value().unwrap();
    let float_val = root.get("value").unwrap();

    let debug = format!("{:?}", float_val);
    assert!(debug.contains("ValueRef"));
    // Float might be formatted slightly differently
    assert!(debug.contains("3.14") || debug.contains("3.1"));
}

#[test]
fn valueref_debug_format_string() {
    let doc = Document::parse_str("value: 'hello'").unwrap();
    let root = doc.root_value().unwrap();
    let str_val = root.get("value").unwrap();

    let debug = format!("{:?}", str_val);
    assert!(debug.contains("ValueRef"));
    assert!(debug.contains("hello"));
}

#[test]
fn valueref_debug_format_sequence() {
    let doc = Document::parse_str("seq:\n  - a\n  - b\n  - c").unwrap();
    let root = doc.root_value().unwrap();
    let seq = root.get("seq").unwrap();

    let debug = format!("{:?}", seq);
    assert!(debug.contains("ValueRef"));
    assert!(debug.contains("sequence"));
    assert!(debug.contains("3")); // Length
}

#[test]
fn valueref_debug_format_mapping() {
    let doc = Document::parse_str("map:\n  a: 1\n  b: 2").unwrap();
    let root = doc.root_value().unwrap();
    let map = root.get("map").unwrap();

    let debug = format!("{:?}", map);
    assert!(debug.contains("ValueRef"));
    assert!(debug.contains("mapping"));
    assert!(debug.contains("2")); // Length
}

#[test]
fn valueref_debug_format_empty_sequence() {
    let doc = Document::parse_str("seq: []").unwrap();
    let root = doc.root_value().unwrap();
    let seq = root.get("seq").unwrap();

    let debug = format!("{:?}", seq);
    assert!(debug.contains("ValueRef"));
    assert!(debug.contains("sequence"));
    assert!(debug.contains("0")); // Empty
}

#[test]
fn valueref_debug_format_empty_mapping() {
    let doc = Document::parse_str("map: {}").unwrap();
    let root = doc.root_value().unwrap();
    let map = root.get("map").unwrap();

    let debug = format!("{:?}", map);
    assert!(debug.contains("ValueRef"));
    assert!(debug.contains("mapping"));
    assert!(debug.contains("0")); // Empty
}

// =============================================================================
// Display formatting tests
// =============================================================================

#[test]
fn valueref_display_format_scalar() {
    let doc = Document::parse_str("value: hello").unwrap();
    let root = doc.root_value().unwrap();
    let val = root.get("value").unwrap();

    let display = format!("{}", val);
    assert!(display.contains("hello"));
}

#[test]
fn valueref_display_format_sequence() {
    let doc = Document::parse_str("seq:\n  - a\n  - b").unwrap();
    let root = doc.root_value().unwrap();
    let seq = root.get("seq").unwrap();

    let display = format!("{}", seq);
    assert!(display.contains("a"));
    assert!(display.contains("b"));
}

#[test]
fn valueref_display_format_mapping() {
    let doc = Document::parse_str("map:\n  key: value").unwrap();
    let root = doc.root_value().unwrap();
    let map = root.get("map").unwrap();

    let display = format!("{}", map);
    assert!(display.contains("key"));
    assert!(display.contains("value"));
}

// =============================================================================
// index() out of bounds tests
// =============================================================================

#[test]
fn valueref_index_out_of_bounds_positive() {
    let doc = Document::parse_str("seq:\n  - a\n  - b").unwrap();
    let root = doc.root_value().unwrap();
    let seq = root.get("seq").unwrap();

    // Valid indices
    assert!(seq.index(0).is_some());
    assert!(seq.index(1).is_some());

    // Out of bounds
    assert!(seq.index(2).is_none());
    assert!(seq.index(100).is_none());
}

#[test]
fn valueref_index_out_of_bounds_negative() {
    let doc = Document::parse_str("seq:\n  - a\n  - b").unwrap();
    let root = doc.root_value().unwrap();
    let seq = root.get("seq").unwrap();

    // Valid negative indices
    assert!(seq.index(-1).is_some());
    assert!(seq.index(-2).is_some());

    // Out of bounds negative
    assert!(seq.index(-3).is_none());
    assert!(seq.index(-100).is_none());
}

#[test]
fn valueref_index_on_non_sequence() {
    let doc = Document::parse_str("scalar: value").unwrap();
    let root = doc.root_value().unwrap();
    let scalar = root.get("scalar").unwrap();

    // index on a scalar should return None
    assert!(scalar.index(0).is_none());
}

#[test]
fn valueref_index_on_mapping() {
    let doc = Document::parse_str("map:\n  key: value").unwrap();
    let root = doc.root_value().unwrap();
    let map = root.get("map").unwrap();

    // index on a mapping should return None
    assert!(map.index(0).is_none());
}

// =============================================================================
// get() edge cases
// =============================================================================

#[test]
fn valueref_get_on_scalar_returns_none() {
    let doc = Document::parse_str("scalar: value").unwrap();
    let root = doc.root_value().unwrap();
    let scalar = root.get("scalar").unwrap();

    // get on a scalar should return None
    assert!(scalar.get("anything").is_none());
}

#[test]
fn valueref_get_on_sequence_returns_none() {
    let doc = Document::parse_str("seq:\n  - item").unwrap();
    let root = doc.root_value().unwrap();
    let seq = root.get("seq").unwrap();

    // get on a sequence should return None
    assert!(seq.get("key").is_none());
}

#[test]
fn valueref_get_nonexistent_key() {
    let doc = Document::parse_str("map:\n  existing: value").unwrap();
    let root = doc.root_value().unwrap();
    let map = root.get("map").unwrap();

    assert!(map.get("existing").is_some());
    assert!(map.get("nonexistent").is_none());
}

// =============================================================================
// Type checking edge cases
// =============================================================================

#[test]
fn valueref_is_null_on_non_scalar() {
    let doc = Document::parse_str("seq: []\nmap: {}").unwrap();
    let root = doc.root_value().unwrap();

    // Sequences and mappings are not null
    assert!(!root.get("seq").unwrap().is_null());
    assert!(!root.get("map").unwrap().is_null());
}

#[test]
fn valueref_is_null_on_quoted_null() {
    let doc = Document::parse_str("quoted: 'null'\nunquoted: null").unwrap();
    let root = doc.root_value().unwrap();

    // Quoted 'null' is not null
    assert!(!root.get("quoted").unwrap().is_null());
    // Unquoted null is null
    assert!(root.get("unquoted").unwrap().is_null());
}

#[test]
fn valueref_as_bool_on_non_scalar() {
    let doc = Document::parse_str("seq: []\nmap: {}").unwrap();
    let root = doc.root_value().unwrap();

    // Sequences and mappings are not booleans
    assert!(root.get("seq").unwrap().as_bool().is_none());
    assert!(root.get("map").unwrap().as_bool().is_none());
}

#[test]
fn valueref_as_i64_on_non_scalar() {
    let doc = Document::parse_str("seq: [1]\nmap: {a: 1}").unwrap();
    let root = doc.root_value().unwrap();

    // Sequences and mappings are not integers
    assert!(root.get("seq").unwrap().as_i64().is_none());
    assert!(root.get("map").unwrap().as_i64().is_none());
}

#[test]
fn valueref_as_f64_on_non_scalar() {
    let doc = Document::parse_str("seq: [1.0]\nmap: {a: 1.0}").unwrap();
    let root = doc.root_value().unwrap();

    // Sequences and mappings are not floats
    assert!(root.get("seq").unwrap().as_f64().is_none());
    assert!(root.get("map").unwrap().as_f64().is_none());
}

#[test]
fn valueref_as_str_on_non_scalar() {
    let doc = Document::parse_str("seq: []\nmap: {}").unwrap();
    let root = doc.root_value().unwrap();

    // Sequences and mappings don't have string representation via as_str
    assert!(root.get("seq").unwrap().as_str().is_none());
    assert!(root.get("map").unwrap().as_str().is_none());
}

// =============================================================================
// Navigation tests
// =============================================================================

#[test]
fn valueref_at_path_deep_nesting() {
    let doc = Document::parse_str("a:\n  b:\n    c:\n      d: value").unwrap();
    let root = doc.root_value().unwrap();

    let deep = root.at_path("/a/b/c/d").unwrap();
    assert_eq!(deep.as_str(), Some("value"));
}

#[test]
fn valueref_at_path_through_sequence() {
    let doc = Document::parse_str("list:\n  - name: first\n  - name: second").unwrap();
    let root = doc.root_value().unwrap();

    let first = root.at_path("/list/0/name").unwrap();
    assert_eq!(first.as_str(), Some("first"));

    let second = root.at_path("/list/1/name").unwrap();
    assert_eq!(second.as_str(), Some("second"));
}

#[test]
fn valueref_at_path_invalid_returns_none() {
    let doc = Document::parse_str("key: value").unwrap();
    let root = doc.root_value().unwrap();

    assert!(root.at_path("/nonexistent").is_none());
    assert!(root.at_path("/key/nested").is_none()); // key is scalar
}

// =============================================================================
// Iteration tests
// =============================================================================

#[test]
fn valueref_seq_iter_on_non_sequence_is_empty() {
    let doc = Document::parse_str("key: value").unwrap();
    let root = doc.root_value().unwrap();

    // seq_iter on root (a mapping) should be empty
    let count = root.seq_iter().count();
    assert_eq!(count, 0);
}

#[test]
fn valueref_map_iter_on_non_mapping_is_empty() {
    let doc = Document::parse_str("- item1\n- item2").unwrap();
    let root = doc.root_value().unwrap();

    // map_iter on a sequence should be empty
    let count = root.map_iter().count();
    assert_eq!(count, 0);
}

#[test]
fn valueref_seq_iter_yields_values() {
    let doc = Document::parse_str("- 1\n- 2\n- 3").unwrap();
    let root = doc.root_value().unwrap();

    let values: Vec<i64> = root.seq_iter().filter_map(|v| v.as_i64()).collect();

    assert_eq!(values, vec![1, 2, 3]);
}

#[test]
fn valueref_map_iter_yields_key_value_pairs() {
    let doc = Document::parse_str("a: 1\nb: 2\nc: 3").unwrap();
    let root = doc.root_value().unwrap();

    let pairs: Vec<(&str, i64)> = root
        .map_iter()
        .filter_map(|(k, v)| Some((k.as_str()?, v.as_i64()?)))
        .collect();

    assert_eq!(pairs, vec![("a", 1), ("b", 2), ("c", 3)]);
}

// =============================================================================
// Tag access tests
// =============================================================================

#[test]
fn valueref_tag_returns_none_for_untagged() {
    let doc = Document::parse_str("value: untagged").unwrap();
    let root = doc.root_value().unwrap();
    let val = root.get("value").unwrap();

    assert!(val.tag().is_none());
}

#[test]
fn valueref_tag_returns_tag_string() {
    let doc = Document::parse_str("value: !custom tagged").unwrap();
    let root = doc.root_value().unwrap();
    let val = root.get("value").unwrap();

    let tag = val.tag();
    assert!(tag.is_some());
    assert!(tag.unwrap().contains("custom"));
}

// =============================================================================
// as_node() tests
// =============================================================================

#[test]
fn valueref_as_node_returns_underlying_noderef() {
    let doc = Document::parse_str("key: value").unwrap();
    let root = doc.root_value().unwrap();

    let node = root.as_node();
    assert!(node.is_mapping());

    // Can use NodeRef methods
    let key_node = node.at_path("/key").unwrap();
    assert_eq!(key_node.scalar_str().unwrap(), "value");
}
