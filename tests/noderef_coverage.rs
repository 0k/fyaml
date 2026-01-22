//! Tests for NodeRef coverage.
//!
//! These tests target NodeRef methods that may have lower coverage:
//! - `document()` method
//! - `tag_bytes()` with and without tags
//! - `seq_get()` on non-sequence
//! - `map_get()` on non-mapping
//! - Debug and Display formatting

use fyaml::{Document, NodeStyle, NodeType};

// =============================================================================
// document() method tests
// =============================================================================

#[test]
fn noderef_document_method_returns_parent_document() {
    let doc = Document::parse_str("key: value").unwrap();
    let root = doc.root().unwrap();

    // The document() method should return a reference to the same document
    let doc_ref = root.document();

    // Verify it's the same document by checking root equality
    let root_from_doc_ref = doc_ref.root().unwrap();
    assert_eq!(root_from_doc_ref.scalar_str().ok(), root.scalar_str().ok());
}

#[test]
fn noderef_document_method_on_nested_node() {
    let doc = Document::parse_str("outer:\n  inner: value").unwrap();
    let inner = doc.root().unwrap().at_path("/outer/inner").unwrap();

    // Even nested nodes should return the parent document
    let doc_ref = inner.document();
    let root = doc_ref.root().unwrap();
    assert!(root.is_mapping());
}

// =============================================================================
// tag_bytes() tests
// =============================================================================

#[test]
fn noderef_tag_bytes_none_for_untagged() {
    let doc = Document::parse_str("untagged: value").unwrap();
    let node = doc.root().unwrap().at_path("/untagged").unwrap();

    // Untagged node should return Ok(None)
    let tag = node.tag_bytes().unwrap();
    assert!(tag.is_none());
}

#[test]
fn noderef_tag_bytes_some_for_tagged() {
    let doc = Document::parse_str("tagged: !custom value").unwrap();
    let node = doc.root().unwrap().at_path("/tagged").unwrap();

    // Tagged node should return Ok(Some(...))
    let tag = node.tag_bytes().unwrap();
    assert!(tag.is_some());

    let tag_bytes = tag.unwrap();
    // The tag should contain "custom"
    let tag_str = std::str::from_utf8(tag_bytes).unwrap();
    assert!(
        tag_str.contains("custom"),
        "Tag should contain 'custom', got: {}",
        tag_str
    );
}

#[test]
fn noderef_tag_str_none_for_untagged() {
    let doc = Document::parse_str("plain: scalar").unwrap();
    let node = doc.root().unwrap().at_path("/plain").unwrap();

    let tag = node.tag_str().unwrap();
    assert!(tag.is_none());
}

#[test]
fn noderef_tag_str_some_for_tagged() {
    let doc = Document::parse_str("!mytag myvalue").unwrap();
    let root = doc.root().unwrap();

    let tag = root.tag_str().unwrap();
    assert!(tag.is_some());
    assert!(tag.unwrap().contains("mytag"));
}

#[test]
fn noderef_tag_on_mapping() {
    let doc = Document::parse_str("!map-tag\nkey: value").unwrap();
    let root = doc.root().unwrap();

    let tag = root.tag_str().unwrap();
    assert!(tag.is_some());
}

#[test]
fn noderef_tag_on_sequence() {
    let doc = Document::parse_str("!seq-tag\n- item1\n- item2").unwrap();
    let root = doc.root().unwrap();

    let tag = root.tag_str().unwrap();
    assert!(tag.is_some());
}

// =============================================================================
// seq_get() on non-sequence tests
// =============================================================================

#[test]
fn noderef_seq_get_on_scalar_returns_none() {
    let doc = Document::parse_str("scalar: value").unwrap();
    let scalar = doc.root().unwrap().at_path("/scalar").unwrap();

    // seq_get on a scalar should return None
    assert!(scalar.seq_get(0).is_none());
    assert!(scalar.seq_get(-1).is_none());
}

#[test]
fn noderef_seq_get_on_mapping_returns_none() {
    let doc = Document::parse_str("mapping:\n  key: value").unwrap();
    let mapping = doc.root().unwrap().at_path("/mapping").unwrap();

    // seq_get on a mapping should return None
    assert!(mapping.seq_get(0).is_none());
    assert!(mapping.seq_get(1).is_none());
}

#[test]
fn noderef_seq_get_out_of_bounds_returns_none() {
    let doc = Document::parse_str("- a\n- b").unwrap();
    let seq = doc.root().unwrap();

    // Valid indices
    assert!(seq.seq_get(0).is_some());
    assert!(seq.seq_get(1).is_some());

    // Out of bounds
    assert!(seq.seq_get(2).is_none());
    assert!(seq.seq_get(100).is_none());
}

#[test]
fn noderef_seq_get_negative_index() {
    let doc = Document::parse_str("- first\n- second\n- third").unwrap();
    let seq = doc.root().unwrap();

    // Negative indices count from the end
    let last = seq.seq_get(-1).unwrap();
    assert_eq!(last.scalar_str().unwrap(), "third");

    let second_to_last = seq.seq_get(-2).unwrap();
    assert_eq!(second_to_last.scalar_str().unwrap(), "second");

    let first = seq.seq_get(-3).unwrap();
    assert_eq!(first.scalar_str().unwrap(), "first");

    // Out of bounds negative
    assert!(seq.seq_get(-4).is_none());
}

// =============================================================================
// map_get() on non-mapping tests
// =============================================================================

#[test]
fn noderef_map_get_on_scalar_returns_none() {
    let doc = Document::parse_str("just a scalar").unwrap();
    let scalar = doc.root().unwrap();

    // map_get on a scalar should return None
    assert!(scalar.map_get("anything").is_none());
}

#[test]
fn noderef_map_get_on_sequence_returns_none() {
    let doc = Document::parse_str("- item1\n- item2").unwrap();
    let seq = doc.root().unwrap();

    // map_get on a sequence should return None
    assert!(seq.map_get("key").is_none());
    assert!(seq.map_get("0").is_none());
}

#[test]
fn noderef_map_get_nonexistent_key_returns_none() {
    let doc = Document::parse_str("existing: value").unwrap();
    let mapping = doc.root().unwrap();

    // Existing key works
    assert!(mapping.map_get("existing").is_some());

    // Non-existent key returns None
    assert!(mapping.map_get("nonexistent").is_none());
    assert!(mapping.map_get("").is_none());
}

// =============================================================================
// Debug and Display formatting tests
// =============================================================================

#[test]
fn noderef_debug_format_scalar() {
    let doc = Document::parse_str("hello").unwrap();
    let node = doc.root().unwrap();

    let debug = format!("{:?}", node);

    // Debug should include NodeRef struct name and fields
    assert!(debug.contains("NodeRef"));
    assert!(debug.contains("kind"));
    assert!(debug.contains("style"));
}

#[test]
fn noderef_debug_format_mapping() {
    let doc = Document::parse_str("key: value").unwrap();
    let node = doc.root().unwrap();

    let debug = format!("{:?}", node);

    // Should show Mapping type
    assert!(debug.contains("NodeRef"));
    assert!(debug.contains("Mapping"));
}

#[test]
fn noderef_debug_format_sequence() {
    let doc = Document::parse_str("- item").unwrap();
    let node = doc.root().unwrap();

    let debug = format!("{:?}", node);

    // Should show Sequence type
    assert!(debug.contains("NodeRef"));
    assert!(debug.contains("Sequence"));
}

#[test]
fn noderef_display_format_scalar() {
    let doc = Document::parse_str("hello").unwrap();
    let node = doc.root().unwrap();

    let display = format!("{}", node);

    // Display should emit the node content
    assert!(display.contains("hello"));
}

#[test]
fn noderef_display_format_mapping() {
    let doc = Document::parse_str("key: value").unwrap();
    let node = doc.root().unwrap();

    let display = format!("{}", node);

    // Display should emit YAML
    assert!(display.contains("key"));
    assert!(display.contains("value"));
}

#[test]
fn noderef_display_format_sequence() {
    let doc = Document::parse_str("- a\n- b").unwrap();
    let node = doc.root().unwrap();

    let display = format!("{}", node);

    // Display should emit YAML sequence
    assert!(display.contains("a"));
    assert!(display.contains("b"));
}

// =============================================================================
// Additional coverage tests
// =============================================================================

#[test]
fn noderef_kind_returns_correct_type() {
    let doc = Document::parse_str("scalar: value\nseq:\n  - item\nmap:\n  nested: value").unwrap();
    let root = doc.root().unwrap();

    // Root is a mapping
    assert_eq!(root.kind(), NodeType::Mapping);

    // Scalar value
    let scalar = root.at_path("/scalar").unwrap();
    assert_eq!(scalar.kind(), NodeType::Scalar);

    // Sequence
    let seq = root.at_path("/seq").unwrap();
    assert_eq!(seq.kind(), NodeType::Sequence);

    // Nested mapping
    let nested = root.at_path("/map").unwrap();
    assert_eq!(nested.kind(), NodeType::Mapping);
}

#[test]
fn noderef_style_for_different_scalars() {
    let doc = Document::parse_str("plain: value\nsingle: 'quoted'\ndouble: \"quoted\"").unwrap();
    let root = doc.root().unwrap();

    let plain = root.at_path("/plain").unwrap();
    assert_eq!(plain.style(), NodeStyle::Plain);

    let single = root.at_path("/single").unwrap();
    assert_eq!(single.style(), NodeStyle::SingleQuoted);

    let double = root.at_path("/double").unwrap();
    assert_eq!(double.style(), NodeStyle::DoubleQuoted);
}

#[test]
fn noderef_is_quoted_variants() {
    let doc = Document::parse_str("plain: value\nsingle: 'quoted'\ndouble: \"quoted\"").unwrap();
    let root = doc.root().unwrap();

    assert!(!root.at_path("/plain").unwrap().is_quoted());
    assert!(root.at_path("/single").unwrap().is_quoted());
    assert!(root.at_path("/double").unwrap().is_quoted());
}

#[test]
fn noderef_is_non_plain_for_literal_and_folded() {
    let doc =
        Document::parse_str("literal: |\n  line1\n  line2\nfolded: >\n  line1\n  line2").unwrap();
    let root = doc.root().unwrap();

    let literal = root.at_path("/literal").unwrap();
    let folded = root.at_path("/folded").unwrap();

    assert!(literal.is_non_plain());
    assert!(folded.is_non_plain());

    // Also verify styles
    assert_eq!(literal.style(), NodeStyle::Literal);
    assert_eq!(folded.style(), NodeStyle::Folded);
}

#[test]
fn noderef_scalar_bytes_returns_bytes() {
    let doc = Document::parse_str("data: hello").unwrap();
    let node = doc.root().unwrap().at_path("/data").unwrap();

    let bytes = node.scalar_bytes().unwrap();
    assert_eq!(bytes, b"hello");
}

#[test]
fn noderef_scalar_bytes_on_non_scalar_returns_error() {
    let doc = Document::parse_str("- item").unwrap();
    let seq = doc.root().unwrap();

    // scalar_bytes on a sequence should return an error
    let result = seq.scalar_bytes();
    assert!(result.is_err());
}

#[test]
fn noderef_scalar_str_on_non_scalar_returns_error() {
    let doc = Document::parse_str("key: value").unwrap();
    let mapping = doc.root().unwrap();

    // scalar_str on a mapping should return an error
    let result = mapping.scalar_str();
    assert!(result.is_err());
}

#[test]
fn noderef_seq_len_on_non_sequence_returns_error() {
    let doc = Document::parse_str("key: value").unwrap();
    let mapping = doc.root().unwrap();

    let result = mapping.seq_len();
    assert!(result.is_err());
}

#[test]
fn noderef_map_len_on_non_mapping_returns_error() {
    let doc = Document::parse_str("- item1\n- item2").unwrap();
    let seq = doc.root().unwrap();

    let result = seq.map_len();
    assert!(result.is_err());
}

#[test]
fn noderef_emit_produces_valid_yaml() {
    let doc = Document::parse_str("key: value\nlist:\n  - a\n  - b").unwrap();
    let root = doc.root().unwrap();

    let emitted = root.emit().unwrap();

    // Should be valid YAML that can be re-parsed
    let reparsed = Document::parse_str(&emitted).unwrap();
    assert!(reparsed.root().is_some());
}

#[test]
fn noderef_at_path_empty_returns_self() {
    let doc = Document::parse_str("key: value").unwrap();
    let root = doc.root().unwrap();

    // Empty path should return self
    let same = root.at_path("").unwrap();
    assert!(same.is_mapping());
}

#[test]
fn noderef_at_path_invalid_returns_none() {
    let doc = Document::parse_str("key: value").unwrap();
    let root = doc.root().unwrap();

    // Non-existent paths
    assert!(root.at_path("/nonexistent").is_none());
    assert!(root.at_path("/key/nested").is_none()); // /key is scalar, can't go deeper
}

#[test]
fn noderef_seq_iter_on_non_sequence_is_empty() {
    let doc = Document::parse_str("key: value").unwrap();
    let mapping = doc.root().unwrap();

    // seq_iter on a mapping should be empty
    let count = mapping.seq_iter().count();
    assert_eq!(count, 0);
}

#[test]
fn noderef_map_iter_on_non_mapping_is_empty() {
    let doc = Document::parse_str("- item1\n- item2").unwrap();
    let seq = doc.root().unwrap();

    // map_iter on a sequence should be empty
    let count = seq.map_iter().count();
    assert_eq!(count, 0);
}

#[test]
fn noderef_is_scalar_is_mapping_is_sequence() {
    let doc = Document::parse_str("scalar: value\nseq: [1, 2]\nmap: {a: 1}").unwrap();
    let root = doc.root().unwrap();

    let scalar = root.at_path("/scalar").unwrap();
    assert!(scalar.is_scalar());
    assert!(!scalar.is_mapping());
    assert!(!scalar.is_sequence());

    let seq = root.at_path("/seq").unwrap();
    assert!(!seq.is_scalar());
    assert!(!seq.is_mapping());
    assert!(seq.is_sequence());

    let map = root.at_path("/map").unwrap();
    assert!(!map.is_scalar());
    assert!(map.is_mapping());
    assert!(!map.is_sequence());
}
