//! Memory safety and boundary condition tests.
//!
//! Tests for handling large inputs, deep nesting, and boundary conditions
//! to ensure memory safety and prevent security issues.

use fyaml::Document;

#[test]
fn from_bytes_with_valid_utf8() {
    let bytes = b"key: value".to_vec();
    let doc = Document::from_bytes(bytes).unwrap();
    assert_eq!(doc.at_path("/key").unwrap().scalar_str().unwrap(), "value");
}

#[test]
fn from_bytes_with_nested_structure() {
    let bytes = b"outer:\n  inner:\n    key: value".to_vec();
    let doc = Document::from_bytes(bytes).unwrap();
    assert_eq!(
        doc.at_path("/outer/inner/key")
            .unwrap()
            .scalar_str()
            .unwrap(),
        "value"
    );
}

#[test]
fn from_bytes_preserves_escape_sequences() {
    // YAML with escape sequences
    let bytes = b"data: \"hello\\nworld\"".to_vec();
    let doc = Document::from_bytes(bytes).unwrap();
    let content = doc.at_path("/data").unwrap().scalar_str().unwrap();
    assert!(content.contains('\n'));
}

#[test]
fn from_string_ownership() {
    let yaml_string = String::from("name: Alice\nage: 30");
    let doc = Document::from_string(yaml_string).unwrap();
    // Original string is consumed, document should work
    assert_eq!(doc.at_path("/name").unwrap().scalar_str().unwrap(), "Alice");
    assert_eq!(doc.at_path("/age").unwrap().scalar_str().unwrap(), "30");
}

#[test]
fn deeply_nested_structure_50_levels() {
    // Security: prevent stack overflow with deep nesting
    let mut yaml = String::new();
    for i in 0..50 {
        yaml.push_str(&format!("{}l{}:\n", "  ".repeat(i), i));
    }
    yaml.push_str(&format!("{}value: deep", "  ".repeat(50)));

    let doc = Document::parse_str(&yaml).unwrap();
    // Should parse without stack overflow
    assert!(doc.root().is_some());
}

#[test]
fn very_long_scalar_value() {
    // Memory: handle large scalars
    let long_value = "x".repeat(100_000);
    let yaml = format!("key: {}", long_value);
    let doc = Document::parse_str(&yaml).unwrap();
    let value = doc.at_path("/key").unwrap().scalar_str().unwrap();
    assert_eq!(value.len(), 100_000);
}

#[test]
fn very_long_key() {
    let long_key = "k".repeat(10_000);
    let yaml = format!("{}: value", long_key);
    let doc = Document::parse_str(&yaml).unwrap();
    let path = format!("/{}", long_key);
    assert!(doc.at_path(&path).is_some());
}

#[test]
fn large_sequence() {
    // Create a sequence with many items
    let items: Vec<String> = (0..1000).map(|i| format!("- item{}", i)).collect();
    let yaml = items.join("\n");
    let doc = Document::parse_str(&yaml).unwrap();
    let root = doc.root().unwrap();
    assert_eq!(root.seq_len().unwrap(), 1000);
}

#[test]
fn large_mapping() {
    // Create a mapping with many keys
    let items: Vec<String> = (0..1000).map(|i| format!("key{}: value{}", i, i)).collect();
    let yaml = items.join("\n");
    let doc = Document::parse_str(&yaml).unwrap();
    let root = doc.root().unwrap();
    assert_eq!(root.map_len().unwrap(), 1000);
}

#[test]
fn document_from_string_empty_fails() {
    let result = Document::from_string(String::new());
    assert!(result.is_err());
}

#[test]
fn document_from_bytes_empty_fails() {
    let result = Document::from_bytes(Vec::new());
    assert!(result.is_err());
}

#[test]
fn document_parse_str_empty_fails() {
    let result = Document::parse_str("");
    assert!(result.is_err());
}

#[test]
fn document_default_creates_empty() {
    let doc = Document::default();
    assert!(doc.root().is_none());
}

#[test]
fn document_new_creates_empty() {
    let doc = Document::new().unwrap();
    assert!(doc.root().is_none());
}

#[test]
fn document_from_str_trait() {
    use std::str::FromStr;
    let doc = Document::from_str("key: value").unwrap();
    assert!(doc.root().is_some());
}

#[test]
fn document_display_trait() {
    let doc = Document::parse_str("key: value").unwrap();
    let display = format!("{}", doc);
    assert!(display.contains("key"));
    assert!(display.contains("value"));
}

#[test]
fn document_debug_trait() {
    let doc = Document::parse_str("key: value").unwrap();
    let debug = format!("{:?}", doc);
    assert!(debug.contains("Document"));
}

#[test]
fn multiple_documents_independent() {
    // Create two documents and verify they're independent
    let doc1 = Document::parse_str("key: value1").unwrap();
    let doc2 = Document::parse_str("key: value2").unwrap();

    assert_eq!(
        doc1.at_path("/key").unwrap().scalar_str().unwrap(),
        "value1"
    );
    assert_eq!(
        doc2.at_path("/key").unwrap().scalar_str().unwrap(),
        "value2"
    );
}

#[test]
fn document_scalar_root() {
    let doc = Document::parse_str("just_a_scalar").unwrap();
    let root = doc.root().unwrap();
    assert!(root.is_scalar());
    assert_eq!(root.scalar_str().unwrap(), "just_a_scalar");
}

#[test]
fn document_sequence_root() {
    let doc = Document::parse_str("[a, b, c]").unwrap();
    let root = doc.root().unwrap();
    assert!(root.is_sequence());
    assert_eq!(root.seq_len().unwrap(), 3);
}

#[test]
fn unicode_in_keys_and_values() {
    let yaml = "japanese_key: 日本語の値\nemoji: 🎉🎊🎁";
    let doc = Document::parse_str(yaml).unwrap();

    // Test Unicode values
    let japanese_value = doc.at_path("/japanese_key").unwrap().scalar_str().unwrap();
    assert_eq!(japanese_value, "日本語の値");

    let emoji = doc.at_path("/emoji").unwrap().scalar_str().unwrap();
    assert_eq!(emoji, "🎉🎊🎁");

    // Test Unicode keys via iteration
    let yaml2 = "日本語キー: value";
    let doc2 = Document::parse_str(yaml2).unwrap();
    let root = doc2.root().unwrap();

    // Find the key by iteration (Unicode keys may not work via path)
    let mut found = false;
    for (key, value) in root.map_iter() {
        let key_str = key.scalar_str().unwrap();
        if key_str == "日本語キー" {
            assert_eq!(value.scalar_str().unwrap(), "value");
            found = true;
        }
    }
    assert!(found, "Should find Unicode key via iteration");
}

#[test]
fn multiline_literal_block() {
    let yaml = "script: |\n  line1\n  line2\n  line3";
    let doc = Document::parse_str(yaml).unwrap();
    let script = doc.at_path("/script").unwrap().scalar_str().unwrap();
    assert!(script.contains("line1"));
    assert!(script.contains("line2"));
    assert!(script.contains("line3"));
}

#[test]
fn multiline_folded_block() {
    let yaml = "text: >\n  folded\n  text\n  here";
    let doc = Document::parse_str(yaml).unwrap();
    let text = doc.at_path("/text").unwrap().scalar_str().unwrap();
    // Folded blocks convert newlines to spaces
    assert!(text.contains("folded"));
}
