//! Tests for FyParser edge cases.
//!
//! These tests target parser edge cases:
//! - Document end markers (`---` and `...`)
//! - Single document without markers
//! - Whitespace-only streams
//! - Comment-only documents
//! - Iterator exhaustion behavior

use fyaml::FyParser;

// =============================================================================
// Document end marker tests
// =============================================================================

#[test]
fn parser_multiple_documents_with_start_markers() {
    let yaml = "---\ndoc1: v1\n---\ndoc2: v2\n---\ndoc3: v3";
    let parser = FyParser::from_string(yaml).unwrap();

    let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();
    assert_eq!(docs.len(), 3);

    assert_eq!(
        docs[0].at_path("/doc1").unwrap().scalar_str().unwrap(),
        "v1"
    );
    assert_eq!(
        docs[1].at_path("/doc2").unwrap().scalar_str().unwrap(),
        "v2"
    );
    assert_eq!(
        docs[2].at_path("/doc3").unwrap().scalar_str().unwrap(),
        "v3"
    );
}

#[test]
fn parser_documents_with_end_markers() {
    let yaml = "doc1: v1\n...\ndoc2: v2\n...";
    let parser = FyParser::from_string(yaml).unwrap();

    let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();
    // End markers (`...`) separate documents
    assert!(!docs.is_empty());

    // First document should be accessible
    assert_eq!(
        docs[0].at_path("/doc1").unwrap().scalar_str().unwrap(),
        "v1"
    );
}

#[test]
fn parser_documents_with_both_markers() {
    let yaml = "---\ndoc1: v1\n...\n---\ndoc2: v2\n...";
    let parser = FyParser::from_string(yaml).unwrap();

    let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();
    assert_eq!(docs.len(), 2);

    assert_eq!(
        docs[0].at_path("/doc1").unwrap().scalar_str().unwrap(),
        "v1"
    );
    assert_eq!(
        docs[1].at_path("/doc2").unwrap().scalar_str().unwrap(),
        "v2"
    );
}

// =============================================================================
// Single document tests
// =============================================================================

#[test]
fn parser_single_document_no_markers() {
    let yaml = "key: value";
    let parser = FyParser::from_string(yaml).unwrap();

    let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();
    assert_eq!(docs.len(), 1);

    assert_eq!(
        docs[0].at_path("/key").unwrap().scalar_str().unwrap(),
        "value"
    );
}

#[test]
fn parser_single_document_with_start_marker() {
    let yaml = "---\nkey: value";
    let parser = FyParser::from_string(yaml).unwrap();

    let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();
    assert_eq!(docs.len(), 1);

    assert_eq!(
        docs[0].at_path("/key").unwrap().scalar_str().unwrap(),
        "value"
    );
}

#[test]
fn parser_single_document_with_end_marker() {
    let yaml = "key: value\n...";
    let parser = FyParser::from_string(yaml).unwrap();

    let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();
    assert_eq!(docs.len(), 1);

    assert_eq!(
        docs[0].at_path("/key").unwrap().scalar_str().unwrap(),
        "value"
    );
}

#[test]
fn parser_single_document_with_both_markers() {
    let yaml = "---\nkey: value\n...";
    let parser = FyParser::from_string(yaml).unwrap();

    let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();
    assert_eq!(docs.len(), 1);

    assert_eq!(
        docs[0].at_path("/key").unwrap().scalar_str().unwrap(),
        "value"
    );
}

// =============================================================================
// Empty and whitespace tests
// =============================================================================

#[test]
fn parser_empty_string() {
    let parser = FyParser::from_string("").unwrap();

    let docs: Vec<_> = parser.doc_iter().collect();
    assert!(docs.is_empty());
}

#[test]
fn parser_whitespace_only_stream() {
    let parser = FyParser::from_string("   \n\n   \n  ").unwrap();

    let docs: Vec<_> = parser.doc_iter().collect();
    // Whitespace-only should produce no documents or possibly an empty document
    // depending on libfyaml's behavior
    assert!(docs.is_empty() || docs.iter().all(|r| r.is_ok()));
}

#[test]
fn parser_tabs_only() {
    let parser = FyParser::from_string("\t\t\t").unwrap();

    let docs: Vec<_> = parser.doc_iter().collect();
    // Should handle gracefully (either empty or error)
    // Tabs at line start are not valid YAML indentation
    for doc in docs.iter().flatten() {
        // Document exists, which is fine
        let _ = doc.root();
    }
}

#[test]
fn parser_newlines_only() {
    let parser = FyParser::from_string("\n\n\n\n").unwrap();

    let docs: Vec<_> = parser.doc_iter().collect();
    // Should produce no documents
    assert!(docs.is_empty() || docs.iter().all(|r| r.is_ok()));
}

// =============================================================================
// Comment tests
// =============================================================================

#[test]
fn parser_comment_only_document() {
    let yaml = "# This is a comment";
    let parser = FyParser::from_string(yaml).unwrap();

    let docs: Vec<_> = parser.doc_iter().collect();
    // Comment-only should produce no documents or an empty document
    assert!(docs.is_empty() || docs.iter().all(|r| r.is_ok()));
}

#[test]
fn parser_multiple_comments_only() {
    let yaml = "# Comment 1\n# Comment 2\n# Comment 3";
    let parser = FyParser::from_string(yaml).unwrap();

    let docs: Vec<_> = parser.doc_iter().collect();
    // Should produce no documents
    assert!(docs.is_empty() || docs.iter().all(|r| r.is_ok()));
}

#[test]
fn parser_document_with_comments() {
    let yaml = "# Header comment\nkey: value  # inline comment\n# Footer comment";
    let parser = FyParser::from_string(yaml).unwrap();

    let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();
    assert_eq!(docs.len(), 1);

    assert_eq!(
        docs[0].at_path("/key").unwrap().scalar_str().unwrap(),
        "value"
    );
}

#[test]
fn parser_comment_between_documents() {
    let yaml = "---\ndoc1: v1\n# Comment between\n---\ndoc2: v2";
    let parser = FyParser::from_string(yaml).unwrap();

    let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();
    assert_eq!(docs.len(), 2);
}

// =============================================================================
// Iterator exhaustion tests
// =============================================================================

#[test]
fn parser_iterator_exhausted_returns_none() {
    let yaml = "key: value";
    let parser = FyParser::from_string(yaml).unwrap();
    let mut iter = parser.doc_iter();

    // First call should return a document
    let first = iter.next();
    assert!(first.is_some());
    assert!(first.unwrap().is_ok());

    // Subsequent calls should return None
    assert!(iter.next().is_none());
    assert!(iter.next().is_none());
    assert!(iter.next().is_none());
}

#[test]
fn parser_iterator_exhausted_after_multiple_docs() {
    let yaml = "---\ndoc1: v1\n---\ndoc2: v2";
    let parser = FyParser::from_string(yaml).unwrap();
    let mut iter = parser.doc_iter();

    // Get all documents
    assert!(iter.next().is_some());
    assert!(iter.next().is_some());

    // Should be exhausted now
    assert!(iter.next().is_none());
    assert!(iter.next().is_none());
}

#[test]
fn parser_iterator_exhausted_empty_stream() {
    let parser = FyParser::from_string("").unwrap();
    let mut iter = parser.doc_iter();

    // Should immediately return None
    assert!(iter.next().is_none());
    assert!(iter.next().is_none());
}

// =============================================================================
// Documents outlive parser tests
// =============================================================================

#[test]
fn parser_documents_outlive_parser_scope() {
    let docs: Vec<_>;
    {
        let parser = FyParser::from_string("key: value").unwrap();
        docs = parser.doc_iter().filter_map(|r| r.ok()).collect();
        // Parser is dropped here
    }

    // Documents should still be valid
    assert_eq!(docs.len(), 1);
    let root = docs[0].root().unwrap();
    assert_eq!(root.at_path("/key").unwrap().scalar_str().unwrap(), "value");
}

#[test]
fn parser_documents_outlive_iterator() {
    let parser = FyParser::from_string("a: 1\n---\nb: 2").unwrap();
    let docs: Vec<_>;
    {
        let iter = parser.doc_iter();
        docs = iter.filter_map(|r| r.ok()).collect();
        // Iterator is dropped here
    }

    // Documents should still be valid
    assert_eq!(docs.len(), 2);
    assert_eq!(docs[0].at_path("/a").unwrap().scalar_str().unwrap(), "1");
    assert_eq!(docs[1].at_path("/b").unwrap().scalar_str().unwrap(), "2");
}

// =============================================================================
// Error handling tests
// =============================================================================

#[test]
fn parser_invalid_yaml_unclosed_bracket() {
    let parser = FyParser::from_string("[unclosed").unwrap();
    let results: Vec<_> = parser.doc_iter().collect();

    // Should have an error
    let has_error = results.iter().any(|r| r.is_err());
    assert!(has_error);
}

#[test]
fn parser_invalid_yaml_bad_indentation() {
    let parser = FyParser::from_string("key: value\n  bad: indent").unwrap();
    let results: Vec<_> = parser.doc_iter().collect();

    // May or may not error depending on libfyaml tolerance
    // Just verify we don't panic
    for result in results {
        match result {
            Ok(doc) => {
                let _ = doc.root();
            }
            Err(_) => {
                // Error is acceptable
            }
        }
    }
}

#[test]
fn parser_invalid_yaml_duplicate_key() {
    let parser = FyParser::from_string("key: value1\nkey: value2").unwrap();
    let results: Vec<_> = parser.doc_iter().collect();

    // Duplicate keys may be accepted by libfyaml (YAML allows it, just warns)
    // Verify we get at least one result
    assert!(!results.is_empty());
}

// =============================================================================
// Complex document tests
// =============================================================================

#[test]
fn parser_complex_multi_document_stream() {
    let yaml = r#"---
# First document
name: first
items:
  - a
  - b
---
# Second document
name: second
nested:
  key: value
---
# Third document - just a scalar
simple value
"#;

    let parser = FyParser::from_string(yaml).unwrap();
    let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();

    assert_eq!(docs.len(), 3);

    // First doc
    assert_eq!(
        docs[0].at_path("/name").unwrap().scalar_str().unwrap(),
        "first"
    );
    assert_eq!(
        docs[0].at_path("/items/0").unwrap().scalar_str().unwrap(),
        "a"
    );

    // Second doc
    assert_eq!(
        docs[1].at_path("/name").unwrap().scalar_str().unwrap(),
        "second"
    );
    assert_eq!(
        docs[1]
            .at_path("/nested/key")
            .unwrap()
            .scalar_str()
            .unwrap(),
        "value"
    );

    // Third doc - scalar root
    let third_root = docs[2].root().unwrap();
    assert!(third_root.is_scalar());
    assert!(third_root.scalar_str().unwrap().contains("simple value"));
}

#[test]
fn parser_documents_with_anchors_and_aliases() {
    let yaml = r#"---
anchor: &myanchor
  key: value
alias: *myanchor
"#;

    let parser = FyParser::from_string(yaml).unwrap();
    let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();

    assert_eq!(docs.len(), 1);

    // Both anchor and alias should resolve to same value
    let anchor_val = docs[0]
        .at_path("/anchor/key")
        .unwrap()
        .scalar_str()
        .unwrap();
    let alias_val = docs[0].at_path("/alias/key").unwrap().scalar_str().unwrap();
    assert_eq!(anchor_val, alias_val);
    assert_eq!(anchor_val, "value");
}

#[test]
fn parser_documents_with_tags() {
    let yaml = "---\ntagged: !custom value";
    let parser = FyParser::from_string(yaml).unwrap();

    let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();
    assert_eq!(docs.len(), 1);

    let tagged = docs[0].at_path("/tagged").unwrap();
    let tag = tagged.tag_str().unwrap();
    assert!(tag.is_some());
    assert!(tag.unwrap().contains("custom"));
}

// =============================================================================
// Scalar-only document tests
// =============================================================================

#[test]
fn parser_scalar_only_document() {
    let yaml = "just a scalar";
    let parser = FyParser::from_string(yaml).unwrap();

    let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();
    assert_eq!(docs.len(), 1);

    let root = docs[0].root().unwrap();
    assert!(root.is_scalar());
    assert_eq!(root.scalar_str().unwrap(), "just a scalar");
}

#[test]
fn parser_multiple_scalar_documents() {
    let yaml = "---\nscalar1\n---\nscalar2\n---\nscalar3";
    let parser = FyParser::from_string(yaml).unwrap();

    let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();
    assert_eq!(docs.len(), 3);

    assert_eq!(docs[0].root().unwrap().scalar_str().unwrap(), "scalar1");
    assert_eq!(docs[1].root().unwrap().scalar_str().unwrap(), "scalar2");
    assert_eq!(docs[2].root().unwrap().scalar_str().unwrap(), "scalar3");
}

#[test]
fn parser_sequence_only_document() {
    let yaml = "- a\n- b\n- c";
    let parser = FyParser::from_string(yaml).unwrap();

    let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();
    assert_eq!(docs.len(), 1);

    let root = docs[0].root().unwrap();
    assert!(root.is_sequence());
    assert_eq!(root.seq_len().unwrap(), 3);
}
