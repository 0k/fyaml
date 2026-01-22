//! Integration tests for fyaml.
//!
//! These tests cover the lifetime-based API with NodeRef and Editor.

use fyaml::{Document, FyParser, NodeStyle, NodeType};

// =============================================================================
// Document Parsing and Root Node Access
// =============================================================================

#[test]
fn parse_document_from_string() {
    let doc = Document::parse_str("foo: bar").unwrap();
    let root = doc.root();
    assert!(root.is_some());
}

#[test]
fn parse_document_root_node_is_mapping() {
    let doc = Document::parse_str("foo: bar\nbaz: qux").unwrap();
    let root = doc.root().unwrap();
    assert!(root.is_mapping());
    assert_eq!(root.kind(), NodeType::Mapping);
}

#[test]
fn parse_empty_document_returns_error() {
    let result = Document::parse_str("");
    assert!(result.is_err());
}

#[test]
fn document_emit() {
    let doc = Document::parse_str("foo: bar").unwrap();
    let output = doc.emit().unwrap();
    assert!(output.contains("foo"));
    assert!(output.contains("bar"));
}

// =============================================================================
// Node Type Detection
// =============================================================================

#[test]
fn node_type_scalar() {
    let doc = Document::parse_str("hello").unwrap();
    let root = doc.root().unwrap();
    assert_eq!(root.kind(), NodeType::Scalar);
    assert!(root.is_scalar());
    assert!(!root.is_mapping());
    assert!(!root.is_sequence());
}

#[test]
fn node_type_mapping() {
    let doc = Document::parse_str("foo: bar\nbaz: qux").unwrap();
    let root = doc.root().unwrap();
    assert_eq!(root.kind(), NodeType::Mapping);
    assert!(root.is_mapping());
    assert!(!root.is_scalar());
    assert!(!root.is_sequence());
}

#[test]
fn node_type_sequence() {
    let doc = Document::parse_str("- one\n- two\n- three").unwrap();
    let root = doc.root().unwrap();
    assert_eq!(root.kind(), NodeType::Sequence);
    assert!(root.is_sequence());
    assert!(!root.is_scalar());
    assert!(!root.is_mapping());
}

// =============================================================================
// Path Navigation
// =============================================================================

#[test]
fn at_path_simple_key() {
    let doc = Document::parse_str("foo: bar").unwrap();
    let root = doc.root().unwrap();
    let node = root.at_path("/foo").unwrap();
    assert_eq!(node.scalar_str().unwrap(), "bar");
}

#[test]
fn at_path_nested() {
    let doc = Document::parse_str("database:\n  host: localhost\n  port: 5432").unwrap();
    let root = doc.root().unwrap();

    let host = root.at_path("/database/host").unwrap();
    assert_eq!(host.scalar_str().unwrap(), "localhost");

    let port = root.at_path("/database/port").unwrap();
    assert_eq!(port.scalar_str().unwrap(), "5432");
}

#[test]
fn at_path_sequence_index() {
    let doc = Document::parse_str("items:\n  - first\n  - second\n  - third").unwrap();
    let root = doc.root().unwrap();

    let first = root.at_path("/items/0").unwrap();
    assert_eq!(first.scalar_str().unwrap(), "first");

    let second = root.at_path("/items/1").unwrap();
    assert_eq!(second.scalar_str().unwrap(), "second");
}

#[test]
fn at_path_not_found() {
    let doc = Document::parse_str("foo: bar").unwrap();
    let root = doc.root().unwrap();
    let result = root.at_path("/nonexistent");
    assert!(result.is_none());
}

#[test]
fn at_path_empty_returns_self() {
    let doc = Document::parse_str("foo: bar").unwrap();
    let root = doc.root().unwrap();
    let node = root.at_path("").unwrap();
    // Empty path should return the node itself
    assert!(node.is_mapping());
}

// =============================================================================
// Zero-Copy Scalar Access
// =============================================================================

#[test]
fn scalar_str_zero_copy() {
    let doc = Document::parse_str("value: hello world").unwrap();
    let root = doc.root().unwrap();
    let node = root.at_path("/value").unwrap();
    // Returns &'doc str - zero allocation
    let s: &str = node.scalar_str().unwrap();
    assert_eq!(s, "hello world");
}

#[test]
fn scalar_bytes_zero_copy() {
    let doc = Document::parse_str("data: bytes").unwrap();
    let node = doc.at_path("/data").unwrap();
    let bytes: &[u8] = node.scalar_bytes().unwrap();
    assert_eq!(bytes, b"bytes");
}

#[test]
fn scalar_preserves_content() {
    let doc = Document::parse_str("value: \"bar: wiz\"").unwrap();
    let root = doc.root().unwrap();
    let node = root.at_path("/value").unwrap();
    // scalar_str returns content without quotes
    assert_eq!(node.scalar_str().unwrap(), "bar: wiz");
}

#[test]
fn emit_yaml_formatted() {
    let doc = Document::parse_str("value: \"bar: wiz\"").unwrap();
    let root = doc.root().unwrap();
    let node = root.at_path("/value").unwrap();
    // emit() returns YAML-formatted (quoted because colon)
    assert_eq!(node.emit().unwrap().trim(), "\"bar: wiz\"");
}

// =============================================================================
// Mapping Iteration
// =============================================================================

#[test]
fn map_iter_yields_key_value_pairs() {
    let doc = Document::parse_str("a: 1\nb: 2\nc: 3").unwrap();
    let root = doc.root().unwrap();
    let pairs: Vec<_> = root.map_iter().collect();

    assert_eq!(pairs.len(), 3);

    let (key, value) = pairs[0];
    assert_eq!(key.scalar_str().unwrap(), "a");
    assert_eq!(value.scalar_str().unwrap(), "1");
}

#[test]
fn map_iter_keys_extraction() {
    let doc = Document::parse_str("foo: 1\nbar: 2\nbaz: 3").unwrap();
    let root = doc.root().unwrap();
    let keys: Vec<&str> = root
        .map_iter()
        .map(|(k, _)| k.scalar_str().unwrap())
        .collect();

    assert_eq!(keys, vec!["foo", "bar", "baz"]);
}

#[test]
fn map_iter_values_extraction() {
    let doc = Document::parse_str("foo: one\nbar: two\nbaz: three").unwrap();
    let root = doc.root().unwrap();
    let values: Vec<&str> = root
        .map_iter()
        .map(|(_, v)| v.scalar_str().unwrap())
        .collect();

    assert_eq!(values, vec!["one", "two", "three"]);
}

#[test]
fn map_get_lookup() {
    let doc = Document::parse_str("name: Alice\nage: 30").unwrap();
    let root = doc.root().unwrap();

    let name = root.map_get("name").unwrap();
    assert_eq!(name.scalar_str().unwrap(), "Alice");

    let age = root.map_get("age").unwrap();
    assert_eq!(age.scalar_str().unwrap(), "30");

    assert!(root.map_get("nonexistent").is_none());
}

// =============================================================================
// Sequence Iteration
// =============================================================================

#[test]
fn seq_iter_yields_nodes() {
    let doc = Document::parse_str("- alpha\n- beta\n- gamma").unwrap();
    let root = doc.root().unwrap();
    let items: Vec<&str> = root.seq_iter().map(|n| n.scalar_str().unwrap()).collect();

    assert_eq!(items, vec!["alpha", "beta", "gamma"]);
}

#[test]
fn seq_get_by_index() {
    let doc = Document::parse_str("[a, b, c]").unwrap();
    let root = doc.root().unwrap();

    assert_eq!(root.seq_get(0).unwrap().scalar_str().unwrap(), "a");
    assert_eq!(root.seq_get(1).unwrap().scalar_str().unwrap(), "b");
    assert_eq!(root.seq_get(2).unwrap().scalar_str().unwrap(), "c");
    assert!(root.seq_get(3).is_none());
}

#[test]
fn seq_get_negative_index() {
    let doc = Document::parse_str("[a, b, c]").unwrap();
    let root = doc.root().unwrap();

    // -1 should be the last element
    let last = root.seq_get(-1).unwrap();
    assert_eq!(last.scalar_str().unwrap(), "c");
}

// =============================================================================
// Length Operations
// =============================================================================

#[test]
fn map_len_returns_count() {
    let doc = Document::parse_str("a: 1\nb: 2\nc: 3\nd: 4").unwrap();
    let root = doc.root().unwrap();
    assert_eq!(root.map_len().unwrap(), 4);
}

#[test]
fn seq_len_returns_count() {
    let doc = Document::parse_str("- one\n- two\n- three").unwrap();
    let root = doc.root().unwrap();
    assert_eq!(root.seq_len().unwrap(), 3);
}

#[test]
fn map_len_empty() {
    let doc = Document::parse_str("{}").unwrap();
    let root = doc.root().unwrap();
    assert_eq!(root.map_len().unwrap(), 0);
}

#[test]
fn seq_len_empty() {
    let doc = Document::parse_str("[]").unwrap();
    let root = doc.root().unwrap();
    assert_eq!(root.seq_len().unwrap(), 0);
}

// =============================================================================
// Tag Handling
// =============================================================================

#[test]
fn get_tag_none_for_untagged() {
    let doc = Document::parse_str("value: hello").unwrap();
    let root = doc.root().unwrap();
    let node = root.at_path("/value").unwrap();
    assert!(node.tag_str().unwrap().is_none());
}

#[test]
fn get_tag_returns_tag() {
    let doc = Document::parse_str("value: !custom tagged_value").unwrap();
    let root = doc.root().unwrap();
    let node = root.at_path("/value").unwrap();
    let tag = node.tag_str().unwrap();
    assert!(tag.is_some());
    assert!(tag.unwrap().contains("custom"));
}

// =============================================================================
// Node Style and Quote Preservation
// =============================================================================

#[test]
fn node_style_detects_single_quotes() {
    let doc = Document::parse_str("key: 'quoted value'").unwrap();
    let root = doc.root().unwrap();
    let value = root.at_path("/key").unwrap();

    assert_eq!(value.style(), NodeStyle::SingleQuoted);
    assert!(value.is_quoted());
}

#[test]
fn node_style_detects_double_quotes() {
    let doc = Document::parse_str(r#"key: "quoted value""#).unwrap();
    let root = doc.root().unwrap();
    let value = root.at_path("/key").unwrap();

    assert_eq!(value.style(), NodeStyle::DoubleQuoted);
    assert!(value.is_quoted());
}

#[test]
fn node_style_detects_plain_scalar() {
    let doc = Document::parse_str("key: plain value").unwrap();
    let root = doc.root().unwrap();
    let value = root.at_path("/key").unwrap();

    assert_eq!(value.style(), NodeStyle::Plain);
    assert!(!value.is_quoted());
}

#[test]
fn is_non_plain_detects_quoted() {
    let doc = Document::parse_str("single: 'value'\ndouble: \"value\"\nplain: value").unwrap();
    let root = doc.root().unwrap();

    assert!(root.at_path("/single").unwrap().is_non_plain());
    assert!(root.at_path("/double").unwrap().is_non_plain());
    assert!(!root.at_path("/plain").unwrap().is_non_plain());
}

// =============================================================================
// Editor Mutations
// =============================================================================

#[test]
fn editor_set_yaml_at() {
    let mut doc = Document::parse_str("name: Alice").unwrap();

    {
        let mut ed = doc.edit();
        ed.set_yaml_at("/name", "'Bob'").unwrap();
    }

    let root = doc.root().unwrap();
    assert_eq!(root.at_path("/name").unwrap().scalar_str().unwrap(), "Bob");
}

#[test]
fn editor_set_yaml_preserves_quotes() {
    let mut doc = Document::parse_str("name: Alice").unwrap();

    {
        let mut ed = doc.edit();
        ed.set_yaml_at("/name", "'true'").unwrap();
    }

    let root = doc.root().unwrap();
    let name = root.at_path("/name").unwrap();
    assert_eq!(name.scalar_str().unwrap(), "true");
    assert!(name.is_quoted());
    assert_eq!(name.style(), NodeStyle::SingleQuoted);
}

#[test]
fn editor_set_yaml_nested() {
    let mut doc = Document::parse_str("user:\n  name: Alice\n  age: 30").unwrap();

    {
        let mut ed = doc.edit();
        ed.set_yaml_at("/user/name", "Bob").unwrap();
    }

    let root = doc.root().unwrap();
    assert_eq!(
        root.at_path("/user/name").unwrap().scalar_str().unwrap(),
        "Bob"
    );
    // Other fields unchanged
    assert_eq!(
        root.at_path("/user/age").unwrap().scalar_str().unwrap(),
        "30"
    );
}

#[test]
fn editor_delete_at() {
    let mut doc = Document::parse_str("a: 1\nb: 2\nc: 3").unwrap();

    {
        let mut ed = doc.edit();
        ed.delete_at("/b").unwrap();
    }

    let root = doc.root().unwrap();
    assert_eq!(root.map_len().unwrap(), 2);
    assert!(root.at_path("/a").is_some());
    assert!(root.at_path("/b").is_none());
    assert!(root.at_path("/c").is_some());
}

#[test]
fn editor_build_and_set_root() {
    let mut doc = Document::new().unwrap();

    {
        let mut ed = doc.edit();
        let mapping = ed.build_from_yaml("name: Alice\nage: 30").unwrap();
        ed.set_root(mapping).unwrap();
    }

    let root = doc.root().unwrap();
    assert!(root.is_mapping());
    assert_eq!(
        root.at_path("/name").unwrap().scalar_str().unwrap(),
        "Alice"
    );
}

// =============================================================================
// Complex Scenarios
// =============================================================================

#[test]
fn traverse_and_extract_pattern() {
    let yaml = indoc::indoc! {"
        database:
          host: localhost
          port: 5432
          credentials:
            user: admin
            pass: secret
    "};

    let doc = Document::parse_str(yaml).unwrap();
    let root = doc.root().unwrap();

    // Navigate to nested path
    let user = root.at_path("/database/credentials/user").unwrap();
    assert_eq!(user.scalar_str().unwrap(), "admin");
}

#[test]
fn iterate_mapping_keys_values_pattern() {
    let yaml = indoc::indoc! {"
        config:
          debug: true
          timeout: 30
          name: myapp
    "};

    let doc = Document::parse_str(yaml).unwrap();
    let root = doc.root().unwrap();
    let config = root.at_path("/config").unwrap();

    assert!(config.is_mapping());
    assert_eq!(config.map_len().unwrap(), 3);

    let mut keys = Vec::new();
    for (k, _) in config.map_iter() {
        keys.push(k.scalar_str().unwrap());
    }

    assert_eq!(keys, vec!["debug", "timeout", "name"]);
}

#[test]
fn iterate_sequence_pattern() {
    let yaml = indoc::indoc! {"
        servers:
          - host: server1
            port: 8080
          - host: server2
            port: 8081
    "};

    let doc = Document::parse_str(yaml).unwrap();
    let root = doc.root().unwrap();
    let servers = root.at_path("/servers").unwrap();

    assert!(servers.is_sequence());
    assert_eq!(servers.seq_len().unwrap(), 2);

    let hosts: Vec<&str> = servers
        .seq_iter()
        .filter_map(|n| n.at_path("/host"))
        .map(|n| n.scalar_str().unwrap())
        .collect();

    assert_eq!(hosts, vec!["server1", "server2"]);
}

#[test]
fn deeply_nested_structure() {
    let yaml = indoc::indoc! {"
        level1:
          level2:
            level3:
              level4:
                value: deep
    "};

    let doc = Document::parse_str(yaml).unwrap();
    let root = doc.root().unwrap();

    let deep = root.at_path("/level1/level2/level3/level4/value").unwrap();
    assert_eq!(deep.scalar_str().unwrap(), "deep");
}

#[test]
fn mixed_sequence_and_mapping() {
    let yaml = indoc::indoc! {"
        users:
          - name: alice
            roles:
              - admin
              - user
          - name: bob
            roles:
              - user
    "};

    let doc = Document::parse_str(yaml).unwrap();
    let root = doc.root().unwrap();

    // Navigate to alice's first role
    let alice_admin = root.at_path("/users/0/roles/0").unwrap();
    assert_eq!(alice_admin.scalar_str().unwrap(), "admin");

    // Navigate to bob's roles and check length
    let bob_roles = root.at_path("/users/1/roles").unwrap();
    assert_eq!(bob_roles.seq_len().unwrap(), 1);
}

#[test]
fn special_characters_in_values() {
    let yaml = indoc::indoc! {r#"
        url: "https://example.com:8080/path"
        message: "Hello: World"
        quoted: 'single quoted'
    "#};

    let doc = Document::parse_str(yaml).unwrap();
    let root = doc.root().unwrap();

    let url = root.at_path("/url").unwrap();
    assert_eq!(url.scalar_str().unwrap(), "https://example.com:8080/path");

    let message = root.at_path("/message").unwrap();
    assert_eq!(message.scalar_str().unwrap(), "Hello: World");
}

#[test]
fn multiline_strings() {
    let yaml = indoc::indoc! {"
        literal: |
          line one
          line two
        folded: >
          folded
          text
    "};

    let doc = Document::parse_str(yaml).unwrap();
    let root = doc.root().unwrap();

    let literal = root.at_path("/literal").unwrap();
    let content = literal.scalar_str().unwrap();
    assert!(content.contains("line one"));
    assert!(content.contains("line two"));
}

#[test]
fn numeric_values() {
    let yaml = indoc::indoc! {"
        integer: 42
        float: 3.14
        negative: -10
        string_num: \"42\"
    "};

    let doc = Document::parse_str(yaml).unwrap();
    let root = doc.root().unwrap();

    // All are scalars
    assert!(root.at_path("/integer").unwrap().is_scalar());
    assert!(root.at_path("/float").unwrap().is_scalar());
    assert!(root.at_path("/negative").unwrap().is_scalar());
    assert!(root.at_path("/string_num").unwrap().is_scalar());

    // Raw string extraction
    assert_eq!(
        root.at_path("/integer").unwrap().scalar_str().unwrap(),
        "42"
    );
    assert_eq!(
        root.at_path("/float").unwrap().scalar_str().unwrap(),
        "3.14"
    );
}

// =============================================================================
// Lifetime Safety (runtime tests)
// =============================================================================

#[test]
fn read_then_edit_then_read() {
    let mut doc = Document::parse_str("name: Alice\nage: 30").unwrap();

    // Read phase 1
    {
        let root = doc.root().unwrap();
        assert_eq!(
            root.at_path("/name").unwrap().scalar_str().unwrap(),
            "Alice"
        );
    }

    // Edit phase
    {
        let mut ed = doc.edit();
        ed.set_yaml_at("/name", "Bob").unwrap();
    }

    // Read phase 2
    {
        let root = doc.root().unwrap();
        assert_eq!(root.at_path("/name").unwrap().scalar_str().unwrap(), "Bob");
        // age unchanged
        assert_eq!(root.at_path("/age").unwrap().scalar_str().unwrap(), "30");
    }
}

#[test]
fn multiple_noderef_same_time() {
    let doc = Document::parse_str("a: 1\nb: 2\nc: 3").unwrap();
    let root = doc.root().unwrap();

    // Multiple NodeRefs can coexist
    let a = root.at_path("/a").unwrap();
    let b = root.at_path("/b").unwrap();
    let c = root.at_path("/c").unwrap();

    assert_eq!(a.scalar_str().unwrap(), "1");
    assert_eq!(b.scalar_str().unwrap(), "2");
    assert_eq!(c.scalar_str().unwrap(), "3");
}

// =============================================================================
// Edge Cases
// =============================================================================

#[test]
fn scalar_str_on_mapping_returns_error() {
    let doc = Document::parse_str("a: 1").unwrap();
    let root = doc.root().unwrap();
    assert!(root.scalar_str().is_err());
}

#[test]
fn seq_len_on_mapping_returns_error() {
    let doc = Document::parse_str("a: 1").unwrap();
    let root = doc.root().unwrap();
    assert!(root.seq_len().is_err());
}

#[test]
fn map_len_on_sequence_returns_error() {
    let doc = Document::parse_str("[1, 2, 3]").unwrap();
    let root = doc.root().unwrap();
    assert!(root.map_len().is_err());
}

#[test]
fn empty_document_root_is_none() {
    let doc = Document::new().unwrap();
    assert!(doc.root().is_none());
}

#[test]
fn document_at_path_on_empty_is_none() {
    let doc = Document::new().unwrap();
    assert!(doc.at_path("/anything").is_none());
}

// =============================================================================
// Cross-Document Copy Tests
// =============================================================================

#[test]
fn copy_node_between_documents() {
    let src = Document::parse_str("name: Alice\nage: 30").unwrap();
    let src_root = src.root().unwrap();

    let mut dest = Document::new().unwrap();
    {
        let mut ed = dest.edit();
        let copied = ed.copy_node(src_root).unwrap();
        ed.set_root(copied).unwrap();
    }

    // Destination should have the copied content
    let dest_root = dest.root().unwrap();
    assert!(dest_root.is_mapping());
    assert_eq!(
        dest_root.at_path("/name").unwrap().scalar_str().unwrap(),
        "Alice"
    );
    assert_eq!(
        dest_root.at_path("/age").unwrap().scalar_str().unwrap(),
        "30"
    );

    // Source should be unchanged
    let src_root = src.root().unwrap();
    assert_eq!(
        src_root.at_path("/name").unwrap().scalar_str().unwrap(),
        "Alice"
    );
}

#[test]
fn copy_node_preserves_structure() {
    let src_yaml = indoc::indoc! {"
        nested:
          key: value
          list:
            - item1
            - item2
    "};
    let src = Document::parse_str(src_yaml).unwrap();
    let src_root = src.root().unwrap();

    let mut dest = Document::new().unwrap();
    {
        let mut ed = dest.edit();
        let copied = ed.copy_node(src_root).unwrap();
        ed.set_root(copied).unwrap();
    }

    // Verify nested structure
    let dest_root = dest.root().unwrap();
    assert_eq!(
        dest_root
            .at_path("/nested/key")
            .unwrap()
            .scalar_str()
            .unwrap(),
        "value"
    );
    assert_eq!(
        dest_root
            .at_path("/nested/list")
            .unwrap()
            .seq_len()
            .unwrap(),
        2
    );
    assert_eq!(
        dest_root
            .at_path("/nested/list/1")
            .unwrap()
            .scalar_str()
            .unwrap(),
        "item2"
    );
}

#[test]
fn copy_node_preserves_style() {
    let src = Document::parse_str("key: 'single quoted'").unwrap();
    let src_value = src.root().unwrap().at_path("/key").unwrap();

    let mut dest = Document::new().unwrap();
    {
        let mut ed = dest.edit();
        let copied = ed.copy_node(src_value).unwrap();
        ed.set_root(copied).unwrap();
    }

    let dest_root = dest.root().unwrap();
    assert_eq!(dest_root.style(), NodeStyle::SingleQuoted);
}

#[test]
fn copy_scalar_between_documents() {
    let src = Document::parse_str("just a scalar").unwrap();
    let src_root = src.root().unwrap();

    let mut dest = Document::new().unwrap();
    {
        let mut ed = dest.edit();
        let copied = ed.copy_node(src_root).unwrap();
        ed.set_root(copied).unwrap();
    }

    let dest_root = dest.root().unwrap();
    assert!(dest_root.is_scalar());
    assert_eq!(dest_root.scalar_str().unwrap(), "just a scalar");
}

#[test]
fn copy_sequence_between_documents() {
    let src = Document::parse_str("[a, b, c]").unwrap();
    let src_root = src.root().unwrap();

    let mut dest = Document::new().unwrap();
    {
        let mut ed = dest.edit();
        let copied = ed.copy_node(src_root).unwrap();
        ed.set_root(copied).unwrap();
    }

    let dest_root = dest.root().unwrap();
    assert!(dest_root.is_sequence());
    assert_eq!(dest_root.seq_len().unwrap(), 3);
    assert_eq!(dest_root.seq_get(0).unwrap().scalar_str().unwrap(), "a");
}

#[test]
fn copy_subnode_to_other_document() {
    let src = Document::parse_str("outer:\n  inner:\n    deep: value").unwrap();
    let inner = src.root().unwrap().at_path("/outer/inner").unwrap();

    let mut dest = Document::new().unwrap();
    {
        let mut ed = dest.edit();
        let copied = ed.copy_node(inner).unwrap();
        ed.set_root(copied).unwrap();
    }

    // Destination should only have the inner part
    let dest_root = dest.root().unwrap();
    assert!(dest_root.is_mapping());
    assert_eq!(
        dest_root.at_path("/deep").unwrap().scalar_str().unwrap(),
        "value"
    );
    assert!(dest_root.at_path("/outer").is_none()); // outer not present
}

#[test]
fn copy_within_same_document_requires_separate_phases() {
    // This test demonstrates that within a single document,
    // you cannot copy while holding a NodeRef - you must either:
    // 1. Use a separate source document, or
    // 2. Use the Editor's internal root() method
    let mut doc = Document::parse_str("original: value").unwrap();

    {
        let ed = doc.edit();
        // The Editor provides read access via ed.root() which has
        // a shorter lifetime, allowing read-then-mutate in one session
        let root = ed.root().unwrap();
        let value = root.at_path("/original").unwrap().scalar_str().unwrap();
        assert_eq!(value, "value");
        // But we can't copy root to somewhere else easily within same doc
        // because NodeRef is tied to &Editor, not &mut Editor
    }

    // Original unchanged
    assert_eq!(
        doc.at_path("/original").unwrap().scalar_str().unwrap(),
        "value"
    );
}

// =============================================================================
// Zero-Copy Verification Tests
// =============================================================================

/// Verifies that scalar_str() returns a slice pointing into the document's memory.
/// This test checks pointer provenance to confirm zero-copy behavior.
#[test]
fn zero_copy_scalar_str_pointer_provenance() {
    // Create a document with known content
    let yaml_content = "key: hello_world_test_value";
    let doc = Document::parse_str(yaml_content).unwrap();
    let node = doc.at_path("/key").unwrap();

    // Get the scalar as a str slice
    let scalar = node.scalar_str().unwrap();

    // Verify the content is correct
    assert_eq!(scalar, "hello_world_test_value");

    // The scalar pointer should NOT be the same as the input string pointer
    // because libfyaml makes a copy of the input, but the scalar should point
    // into libfyaml's internal buffer (not a newly allocated Rust String).
    // We can verify this indirectly: the slice should be stable across calls.
    let scalar2 = node.scalar_str().unwrap();
    assert_eq!(scalar.as_ptr(), scalar2.as_ptr());
}

/// Verifies that scalar_bytes() returns a slice pointing into the document's memory.
#[test]
fn zero_copy_scalar_bytes_pointer_provenance() {
    let doc = Document::parse_str("data: binary_like_content").unwrap();
    let node = doc.at_path("/data").unwrap();

    let bytes1 = node.scalar_bytes().unwrap();
    let bytes2 = node.scalar_bytes().unwrap();

    // Consecutive calls should return the same pointer
    assert_eq!(bytes1.as_ptr(), bytes2.as_ptr());
    assert_eq!(bytes1, b"binary_like_content");
}

/// Verifies that tag_str() returns a zero-copy slice when tag is present.
#[test]
fn zero_copy_tag_str_pointer_provenance() {
    let doc = Document::parse_str("!custom tagged_value").unwrap();
    let root = doc.root().unwrap();

    let tag1 = root.tag_str().unwrap();
    let tag2 = root.tag_str().unwrap();

    assert!(tag1.is_some());
    assert_eq!(tag1.unwrap(), "!custom");
    assert_eq!(tag1.unwrap().as_ptr(), tag2.unwrap().as_ptr());
}

/// Verifies that multiple scalar accesses within an iteration don't allocate new strings.
#[test]
fn zero_copy_iteration_no_string_allocation() {
    let doc = Document::parse_str(
        r#"
        items:
          - first
          - second
          - third
        "#,
    )
    .unwrap();
    let items = doc.at_path("/items").unwrap();

    // Collect pointers from first iteration
    let pointers1: Vec<*const u8> = items
        .seq_iter()
        .map(|n| n.scalar_str().unwrap().as_ptr())
        .collect();

    // Collect pointers from second iteration
    let pointers2: Vec<*const u8> = items
        .seq_iter()
        .map(|n| n.scalar_str().unwrap().as_ptr())
        .collect();

    // Pointers should be identical across iterations (zero-copy)
    assert_eq!(pointers1, pointers2);
}

/// Verifies that map iteration key/value access is zero-copy.
#[test]
fn zero_copy_map_iteration() {
    let doc = Document::parse_str(
        r#"
        key1: value1
        key2: value2
        "#,
    )
    .unwrap();
    let root = doc.root().unwrap();

    // First iteration
    let pairs1: Vec<(*const u8, *const u8)> = root
        .map_iter()
        .map(|(k, v)| {
            (
                k.scalar_str().unwrap().as_ptr(),
                v.scalar_str().unwrap().as_ptr(),
            )
        })
        .collect();

    // Second iteration
    let pairs2: Vec<(*const u8, *const u8)> = root
        .map_iter()
        .map(|(k, v)| {
            (
                k.scalar_str().unwrap().as_ptr(),
                v.scalar_str().unwrap().as_ptr(),
            )
        })
        .collect();

    // Pointers should be identical (zero-copy)
    assert_eq!(pairs1, pairs2);
}

// =============================================================================
// Stream Parsing Safety Tests
// =============================================================================

/// Documents from stream remain valid after parser is dropped.
/// This verifies the InputOwnership::Parser mechanism keeps the parser alive.
#[test]
fn stream_documents_outlive_parser() {
    let docs: Vec<Document>;
    {
        let parser = FyParser::from_string("---\ndoc1: v1\n---\ndoc2: v2\n---\ndoc3: v3").unwrap();
        docs = parser.doc_iter().filter_map(|r| r.ok()).collect();
        // parser is dropped here, but documents should still be valid
    }

    // Verify all documents are accessible after parser drop
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

/// Documents from stream remain valid after iterator is dropped.
#[test]
fn stream_documents_outlive_iterator() {
    let parser = FyParser::from_string("---\nkey: value1\n---\nkey: value2").unwrap();
    let docs: Vec<Document>;
    {
        let iter = parser.doc_iter();
        docs = iter.filter_map(|r| r.ok()).collect();
        // iterator is dropped here
    }

    // Documents should still be valid
    assert_eq!(docs.len(), 2);
    assert_eq!(
        docs[0].at_path("/key").unwrap().scalar_str().unwrap(),
        "value1"
    );
    assert_eq!(
        docs[1].at_path("/key").unwrap().scalar_str().unwrap(),
        "value2"
    );
}

/// Multiple documents from stream can be accessed concurrently.
#[test]
fn stream_documents_concurrent_access() {
    let parser = FyParser::from_string("---\na: 1\n---\nb: 2\n---\nc: 3").unwrap();
    let docs: Vec<Document> = parser.doc_iter().filter_map(|r| r.ok()).collect();

    // Access all documents simultaneously
    let values: Vec<&str> = vec![
        docs[0].at_path("/a").unwrap().scalar_str().unwrap(),
        docs[1].at_path("/b").unwrap().scalar_str().unwrap(),
        docs[2].at_path("/c").unwrap().scalar_str().unwrap(),
    ];

    assert_eq!(values, vec!["1", "2", "3"]);
}

/// Stream-parsed documents maintain zero-copy semantics.
#[test]
fn stream_documents_zero_copy() {
    let parser = FyParser::from_string("---\nkey: stream_value").unwrap();
    let docs: Vec<Document> = parser.doc_iter().filter_map(|r| r.ok()).collect();
    let node = docs[0].at_path("/key").unwrap();

    // Verify zero-copy: same pointer across calls
    let ptr1 = node.scalar_str().unwrap().as_ptr();
    let ptr2 = node.scalar_str().unwrap().as_ptr();
    assert_eq!(ptr1, ptr2);
}

/// Documents from different stream positions are independent.
#[test]
fn stream_documents_are_independent() {
    let parser = FyParser::from_string("---\nshared: doc1_value\n---\nshared: doc2_value").unwrap();
    let mut iter = parser.doc_iter();

    let mut doc1 = iter.next().unwrap().unwrap();
    let doc2 = iter.next().unwrap().unwrap();

    // Both documents should have their own independent values
    assert_eq!(
        doc1.at_path("/shared").unwrap().scalar_str().unwrap(),
        "doc1_value"
    );
    assert_eq!(
        doc2.at_path("/shared").unwrap().scalar_str().unwrap(),
        "doc2_value"
    );

    // Modifying one document shouldn't affect the other
    // (We can verify this by editing doc1 and checking doc2)
    {
        let mut ed = doc1.edit();
        ed.set_yaml_at("/shared", "'modified'").unwrap();
    }

    // doc2 should be unchanged
    assert_eq!(
        doc2.at_path("/shared").unwrap().scalar_str().unwrap(),
        "doc2_value"
    );
}

/// Parser can create multiple iterators (though typically only one is used).
#[test]
fn stream_parser_reusable() {
    let parser = FyParser::from_string("single: document").unwrap();

    // First iteration consumes documents
    let docs1: Vec<Document> = parser.doc_iter().filter_map(|r| r.ok()).collect();
    assert_eq!(docs1.len(), 1);

    // Second iteration should be empty (stream exhausted)
    let docs2: Vec<Document> = parser.doc_iter().filter_map(|r| r.ok()).collect();
    assert_eq!(docs2.len(), 0);
}

// =============================================================================
// Editor Memory Safety Tests
// =============================================================================

/// Tests that build_from_yaml with invalid YAML doesn't leak memory.
/// The buffer should be freed when parsing fails.
#[test]
fn editor_build_from_yaml_invalid_yaml_no_leak() {
    let mut doc = Document::new().unwrap();

    // Try building invalid YAML multiple times - if there's a leak, this would
    // eventually cause issues (though we can't easily detect leaks in tests,
    // this at least exercises the error path)
    for _ in 0..100 {
        let mut ed = doc.edit();
        let result = ed.build_from_yaml("[unclosed bracket");
        assert!(result.is_err());
    }
}

/// Tests that build_from_yaml restores document state after failure.
/// This verifies the diag swap pattern doesn't corrupt the document.
#[test]
fn editor_build_from_yaml_restores_state_after_failure() {
    let mut doc = Document::parse_str("existing: value").unwrap();

    // Build something invalid - should fail but not corrupt document
    {
        let mut ed = doc.edit();
        let result = ed.build_from_yaml("[invalid");
        assert!(result.is_err());
    }

    // Document should still be usable
    let root = doc.root().unwrap();
    assert_eq!(
        root.at_path("/existing").unwrap().scalar_str().unwrap(),
        "value"
    );

    // Should be able to do a successful edit after failed one
    {
        let mut ed = doc.edit();
        let node = ed.build_from_yaml("new: data").unwrap();
        ed.set_yaml_at("/added", "works").unwrap();
        // Don't insert `node` - just verify we can build again
        drop(node);
    }

    let root = doc.root().unwrap();
    assert_eq!(
        root.at_path("/added").unwrap().scalar_str().unwrap(),
        "works"
    );
}

/// Tests that build_from_yaml error includes location info.
#[test]
fn editor_build_from_yaml_error_has_location() {
    let mut doc = Document::new().unwrap();

    let mut ed = doc.edit();
    let result = ed.build_from_yaml("key: value\n[invalid");

    match result {
        Err(fyaml::Error::ParseError(pe)) => {
            // Should have location info (error is on line 2)
            assert!(pe.line().is_some(), "Editor parse error should have line");
        }
        Err(other) => panic!("Expected ParseError, got {:?}", other),
        Ok(_) => panic!("Expected parse error for invalid YAML"),
    }
}

/// Tests that RawNodeHandle properly frees nodes when dropped without insertion.
#[test]
fn editor_raw_node_handle_dropped_without_insert() {
    let mut doc = Document::new().unwrap();

    // Create nodes but don't insert them - they should be freed by RAII
    for _ in 0..100 {
        let mut ed = doc.edit();
        let _node = ed.build_from_yaml("key: value").unwrap();
        // node is dropped here without being inserted
    }
    // If there was a memory leak, we'd have problems, but this test at least
    // exercises the Drop path
}

/// Tests that RawNodeHandle properly frees scalars when dropped without insertion.
#[test]
fn editor_raw_node_handle_scalar_dropped() {
    let mut doc = Document::new().unwrap();

    for _ in 0..100 {
        let mut ed = doc.edit();
        let _scalar = ed.build_scalar("test value").unwrap();
        // scalar is dropped without insertion
    }
}

/// Tests that RawNodeHandle properly frees sequences when dropped without insertion.
#[test]
fn editor_raw_node_handle_sequence_dropped() {
    let mut doc = Document::new().unwrap();

    for _ in 0..100 {
        let mut ed = doc.edit();
        let _seq = ed.build_sequence().unwrap();
        // seq is dropped without insertion
    }
}

/// Tests that RawNodeHandle properly frees mappings when dropped without insertion.
#[test]
fn editor_raw_node_handle_mapping_dropped() {
    let mut doc = Document::new().unwrap();

    for _ in 0..100 {
        let mut ed = doc.edit();
        let _map = ed.build_mapping().unwrap();
        // map is dropped without insertion
    }
}

// =============================================================================
// Stream Parse Error Tests
// =============================================================================

/// Tests that invalid YAML in a stream produces an error, not just None.
#[test]
fn stream_parse_error_returns_err() {
    let parser = FyParser::from_string("[unclosed").unwrap();
    let results: Vec<_> = parser.doc_iter().collect();

    // Should have at least one result that is an error
    assert!(!results.is_empty());
    assert!(results.iter().any(|r| r.is_err()));
}

/// Tests that stream parse errors contain rich location info (line/column).
#[test]
fn stream_parse_error_has_location() {
    let parser = FyParser::from_string("[unclosed").unwrap();
    let results: Vec<_> = parser.doc_iter().collect();

    // Find the error
    let err = results
        .into_iter()
        .find(|r| r.is_err())
        .unwrap()
        .unwrap_err();

    // Should be a ParseError variant with location info
    match err {
        fyaml::Error::ParseError(pe) => {
            // Should have line/column information
            assert!(
                pe.line().is_some(),
                "Stream parse error should have line number"
            );
            assert!(
                pe.column().is_some(),
                "Stream parse error should have column number"
            );
            // Message should be descriptive
            assert!(
                !pe.message().is_empty(),
                "Stream parse error should have a message"
            );
        }
        other => panic!("Expected ParseError variant, got: {:?}", other),
    }
}

/// Tests that stream parse error on later lines reports correct location.
#[test]
fn stream_parse_error_multiline_location() {
    // The error is on line 3
    let yaml = "---\nkey: value\n[unclosed";
    let parser = FyParser::from_string(yaml).unwrap();
    let results: Vec<_> = parser.doc_iter().collect();

    // Find the error
    let err = results.into_iter().find(|r| r.is_err());
    assert!(err.is_some(), "Should have produced a parse error");

    let err = err.unwrap().unwrap_err();
    if let fyaml::Error::ParseError(pe) = err {
        // The error should be on line 3 (or possibly 2 depending on how libfyaml counts)
        let line = pe.line().expect("Should have line number");
        assert!(line >= 2, "Error line should be >= 2, got: {}", line);
    }
}

/// Tests that parse errors can be distinguished from clean EOF.
#[test]
fn stream_empty_yields_none_not_error() {
    let parser = FyParser::from_string("").unwrap();
    let results: Vec<_> = parser.doc_iter().collect();

    // Empty stream should yield no results (not an error)
    assert!(results.is_empty());
}

/// Tests that valid documents followed by invalid YAML produce docs then error.
#[test]
fn stream_valid_then_invalid() {
    // This tests the sequence: valid doc, then parse error
    let parser = FyParser::from_string("---\nvalid: doc\n---\n[unclosed").unwrap();
    let results: Vec<_> = parser.doc_iter().collect();

    // Should have at least one valid doc
    let valid_count = results.iter().filter(|r| r.is_ok()).count();
    let error_count = results.iter().filter(|r| r.is_err()).count();

    assert!(valid_count >= 1, "Should have at least one valid document");
    assert!(error_count >= 1, "Should have at least one error");
}

// =============================================================================
// YAML Tag Tests
// =============================================================================

/// Tests standard YAML tags.
#[test]
fn tag_yaml_standard_types() {
    let yaml = "
int: !!int 42
str: !!str 42
bool: !!bool true
float: !!float 3.14
null: !!null
";
    let doc = Document::parse_str(yaml).unwrap();
    let root = doc.root().unwrap();

    // Verify tags are preserved
    let int_node = root.at_path("/int").unwrap();
    let tag = int_node.tag_str().unwrap();
    assert!(tag.is_some());
    assert!(tag.unwrap().contains("int"));

    let str_node = root.at_path("/str").unwrap();
    let tag = str_node.tag_str().unwrap();
    assert!(tag.is_some());
    assert!(tag.unwrap().contains("str"));
}

/// Tests custom/local tags.
#[test]
fn tag_custom_local() {
    let doc = Document::parse_str("!mytag custom_value").unwrap();
    let root = doc.root().unwrap();

    let tag = root.tag_str().unwrap();
    assert!(tag.is_some());
    assert!(tag.unwrap().contains("mytag"));
    assert_eq!(root.scalar_str().unwrap(), "custom_value");
}

/// Tests tags on complex nodes.
#[test]
fn tag_on_mapping() {
    let doc = Document::parse_str("!person\nname: Alice\nage: 30").unwrap();
    let root = doc.root().unwrap();

    let tag = root.tag_str().unwrap();
    assert!(tag.is_some());
    assert!(tag.unwrap().contains("person"));
    assert!(root.is_mapping());
}

/// Tests tags on sequence nodes.
#[test]
fn tag_on_sequence() {
    let doc = Document::parse_str("!list\n- item1\n- item2").unwrap();
    let root = doc.root().unwrap();

    let tag = root.tag_str().unwrap();
    assert!(tag.is_some());
    assert!(tag.unwrap().contains("list"));
    assert!(root.is_sequence());
}

// =============================================================================
// UTF-8 and Special Content Tests
// =============================================================================

/// Tests handling of UTF-8 content.
#[test]
fn utf8_content_preserved() {
    let doc = Document::parse_str("greeting: こんにちは").unwrap();
    let root = doc.root().unwrap();

    let greeting = root.at_path("/greeting").unwrap();
    assert_eq!(greeting.scalar_str().unwrap(), "こんにちは");
}

/// Tests handling of emoji in content.
#[test]
fn emoji_content_preserved() {
    let doc = Document::parse_str("emoji: 🎉🎊🎁").unwrap();
    let root = doc.root().unwrap();

    let emoji = root.at_path("/emoji").unwrap();
    assert_eq!(emoji.scalar_str().unwrap(), "🎉🎊🎁");
}

/// Tests handling of mixed UTF-8 content.
#[test]
fn mixed_utf8_content() {
    let doc = Document::parse_str("message: \"Hello 世界! Привет мир! مرحبا بالعالم\"").unwrap();
    let root = doc.root().unwrap();

    let message = root.at_path("/message").unwrap();
    assert!(message.scalar_str().unwrap().contains("世界"));
    assert!(message.scalar_str().unwrap().contains("Привет"));
    assert!(message.scalar_str().unwrap().contains("مرحبا"));
}

/// Tests handling of null byte in content (should be escaped or rejected).
#[test]
fn special_characters_escaped() {
    // Tab and newline in double-quoted string should be preserved
    let doc = Document::parse_str("text: \"line1\\nline2\\ttabbed\"").unwrap();
    let root = doc.root().unwrap();

    let text = root.at_path("/text").unwrap();
    let content = text.scalar_str().unwrap();
    assert!(content.contains('\n'));
    assert!(content.contains('\t'));
}

/// Tests that very long UTF-8 strings are handled correctly.
#[test]
fn long_utf8_string() {
    let long_string = "日本語".repeat(1000);
    let yaml = format!("content: \"{}\"", long_string);
    let doc = Document::parse_str(&yaml).unwrap();
    let root = doc.root().unwrap();

    let content = root.at_path("/content").unwrap();
    assert_eq!(content.scalar_str().unwrap(), long_string);
}

// =============================================================================
// ValueRef Zero-Copy Typed Access Tests
// =============================================================================

#[test]
fn value_ref_basic_access() {
    let doc = Document::parse_str("name: Alice\nage: 30\nactive: true").unwrap();
    let root = doc.root_value().unwrap();

    assert_eq!(root.get("name").unwrap().as_str(), Some("Alice"));
    assert_eq!(root.get("age").unwrap().as_i64(), Some(30));
    assert_eq!(root.get("active").unwrap().as_bool(), Some(true));
}

#[test]
fn value_ref_quoted_not_interpreted() {
    let doc = Document::parse_str("quoted: 'true'\nquoted_num: '42'").unwrap();
    let root = doc.root_value().unwrap();

    // Quoted values should NOT be interpreted as bool/number
    assert_eq!(root.get("quoted").unwrap().as_bool(), None);
    assert_eq!(root.get("quoted").unwrap().as_str(), Some("true"));

    assert_eq!(root.get("quoted_num").unwrap().as_i64(), None);
    assert_eq!(root.get("quoted_num").unwrap().as_str(), Some("42"));
}

#[test]
fn value_ref_null_detection() {
    let doc = Document::parse_str("null_val: null\ntilde: ~\nempty:\nstr: 'null'").unwrap();
    let root = doc.root_value().unwrap();

    assert!(root.get("null_val").unwrap().is_null());
    assert!(root.get("tilde").unwrap().is_null());
    assert!(root.get("empty").unwrap().is_null());
    // Quoted 'null' is NOT null
    assert!(!root.get("str").unwrap().is_null());
}

#[test]
fn value_ref_number_formats() {
    let doc = Document::parse_str("dec: 255\nhex: 0xFF\noct: 0o377\nbin: 0b11111111").unwrap();
    let root = doc.root_value().unwrap();

    assert_eq!(root.get("dec").unwrap().as_i64(), Some(255));
    assert_eq!(root.get("hex").unwrap().as_i64(), Some(255));
    assert_eq!(root.get("oct").unwrap().as_i64(), Some(255));
    assert_eq!(root.get("bin").unwrap().as_i64(), Some(255));
}

#[test]
fn value_ref_special_floats() {
    let doc = Document::parse_str("inf: .inf\nneginf: -.inf\nnan: .nan").unwrap();
    let root = doc.root_value().unwrap();

    assert!(root.get("inf").unwrap().as_f64().unwrap().is_infinite());
    assert!(root
        .get("inf")
        .unwrap()
        .as_f64()
        .unwrap()
        .is_sign_positive());
    assert!(root.get("neginf").unwrap().as_f64().unwrap().is_infinite());
    assert!(root
        .get("neginf")
        .unwrap()
        .as_f64()
        .unwrap()
        .is_sign_negative());
    assert!(root.get("nan").unwrap().as_f64().unwrap().is_nan());
}

#[test]
fn value_ref_bool_variants() {
    let doc =
        Document::parse_str("t1: true\nt2: True\nt3: yes\nt4: on\nf1: false\nf2: no\nf3: off")
            .unwrap();
    let root = doc.root_value().unwrap();

    assert_eq!(root.get("t1").unwrap().as_bool(), Some(true));
    assert_eq!(root.get("t2").unwrap().as_bool(), Some(true));
    assert_eq!(root.get("t3").unwrap().as_bool(), Some(true));
    assert_eq!(root.get("t4").unwrap().as_bool(), Some(true));
    assert_eq!(root.get("f1").unwrap().as_bool(), Some(false));
    assert_eq!(root.get("f2").unwrap().as_bool(), Some(false));
    assert_eq!(root.get("f3").unwrap().as_bool(), Some(false));
}

#[test]
fn value_ref_sequence_iteration() {
    let doc = Document::parse_str("- 10\n- 20\n- 30").unwrap();
    let root = doc.root_value().unwrap();

    let sum: i64 = root.seq_iter().filter_map(|v| v.as_i64()).sum();
    assert_eq!(sum, 60);
}

#[test]
fn value_ref_mapping_iteration() {
    let doc = Document::parse_str("a: 1\nb: 2\nc: 3").unwrap();
    let root = doc.root_value().unwrap();

    let sum: i64 = root.map_iter().filter_map(|(_, v)| v.as_i64()).sum();
    assert_eq!(sum, 6);
}

#[test]
fn value_ref_nested_navigation() {
    let doc = Document::parse_str("level1:\n  level2:\n    level3: 42").unwrap();
    let root = doc.root_value().unwrap();

    // Method chaining
    let value = root
        .get("level1")
        .unwrap()
        .get("level2")
        .unwrap()
        .get("level3")
        .unwrap();
    assert_eq!(value.as_i64(), Some(42));

    // Path navigation
    let value2 = root.at_path("/level1/level2/level3").unwrap();
    assert_eq!(value2.as_i64(), Some(42));
}

#[test]
fn value_ref_index_access() {
    let doc = Document::parse_str("- first\n- second\n- third").unwrap();
    let root = doc.root_value().unwrap();

    assert_eq!(root.index(0).unwrap().as_str(), Some("first"));
    assert_eq!(root.index(1).unwrap().as_str(), Some("second"));
    assert_eq!(root.index(-1).unwrap().as_str(), Some("third")); // Negative index
}

#[test]
fn value_ref_tag_access() {
    let doc = Document::parse_str("!custom tagged_value").unwrap();
    let root = doc.root_value().unwrap();

    assert!(root.tag().is_some());
    assert!(root.tag().unwrap().contains("custom"));
}

#[test]
fn value_ref_type_checking() {
    let doc = Document::parse_str("scalar: hello\nseq:\n  - item\nmap:\n  key: value").unwrap();
    let root = doc.root_value().unwrap();

    assert!(root.get("scalar").unwrap().is_scalar());
    assert!(root.get("seq").unwrap().is_sequence());
    assert!(root.get("map").unwrap().is_mapping());

    assert!(!root.get("scalar").unwrap().is_sequence());
    assert!(!root.get("seq").unwrap().is_mapping());
    assert!(!root.get("map").unwrap().is_scalar());
}

// Note: Comment preservation requires FYPCF_PARSE_COMMENTS and FYECF_OUTPUT_COMMENTS
// flags which are not yet exposed in the Rust API. libfyaml supports it, but we don't yet.

#[test]
fn test_quote_style_preservation() {
    let yaml = "single: 'quoted value'
double: \"another value\"
plain: unquoted
";

    let mut doc = Document::parse_str(yaml).unwrap();
    {
        let mut ed = doc.edit();
        ed.set_yaml_at("/plain", "'now single quoted'").unwrap();
    }

    let output = doc.emit().unwrap();
    println!("Output:\n{}", output);

    // Original quote styles should be preserved
    assert!(
        output.contains("'quoted value'"),
        "Single quotes should be preserved"
    );
    assert!(
        output.contains("\"another value\""),
        "Double quotes should be preserved"
    );
}

#[test]
fn test_block_style_preservation() {
    let yaml = "literal: |
  line one
  line two
folded: >
  folded
  text
plain: value
";

    let mut doc = Document::parse_str(yaml).unwrap();
    {
        let mut ed = doc.edit();
        ed.set_yaml_at("/plain", "modified").unwrap();
    }

    let output = doc.emit().unwrap();
    println!("Output:\n{}", output);

    // Block styles should be preserved
    assert!(
        output.contains("literal: |"),
        "Literal block style should be preserved"
    );
    assert!(
        output.contains("folded: >"),
        "Folded block style should be preserved"
    );
}

#[test]
fn test_flow_style_preservation() {
    let yaml = "flow_seq: [1, 2, 3]
flow_map: {a: 1, b: 2}
other: value
";

    let mut doc = Document::parse_str(yaml).unwrap();
    {
        let mut ed = doc.edit();
        ed.set_yaml_at("/other", "modified").unwrap();
    }

    let output = doc.emit().unwrap();
    println!("Output:\n{}", output);

    // Flow styles should be preserved for untouched nodes
    assert!(
        output.contains("[") && output.contains("]"),
        "Flow sequence style should be preserved"
    );
    assert!(
        output.contains("{") && output.contains("}"),
        "Flow mapping style should be preserved"
    );
}

#[test]
fn test_comment_preservation() {
    let yaml = "# Top comment
name: Alice  # inline comment
age: 30
";

    let mut doc = Document::parse_str(yaml).unwrap();
    {
        let mut ed = doc.edit();
        ed.set_yaml_at("/age", "31").unwrap();
    }

    let output = doc.emit().unwrap();
    println!("Output:\n{}", output);

    // Check if comments are preserved
    assert!(
        output.contains("# Top comment"),
        "Top comment should be preserved"
    );
    assert!(
        output.contains("# inline comment"),
        "Inline comment should be preserved"
    );
}

// =============================================================================
// Destructor Stress Tests
// =============================================================================
// These tests exercise destructor paths to catch memory issues.
// Run with: RUSTFLAGS="-Zsanitizer=address" cargo +nightly test

/// Stress test for stream-parsed document destruction.
/// This exercises fy_parse_document_destroy vs fy_document_destroy paths.
#[test]
fn destructor_stress_stream_documents() {
    for _ in 0..100 {
        let parser =
            FyParser::from_string("---\nkey: value\n---\nfoo: bar\n---\nbaz: qux").unwrap();
        let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();
        assert_eq!(docs.len(), 3);
        // All docs dropped here via fy_parse_document_destroy
    }
    // Parser dropped here via fy_parser_destroy
}

/// Stress test for standalone document destruction.
/// This exercises fy_document_destroy path.
#[test]
fn destructor_stress_standalone_documents() {
    for i in 0..100 {
        let doc = Document::parse_str(&format!("key{}: value{}", i, i)).unwrap();
        let root = doc.root().unwrap();
        assert_eq!(
            root.at_path(&format!("/key{}", i))
                .unwrap()
                .scalar_str()
                .unwrap(),
            format!("value{}", i)
        );
        // Doc dropped here via fy_document_destroy
    }
}

/// Stress test for Document::from_string (zero-copy owned path).
#[test]
fn destructor_stress_owned_string() {
    for i in 0..100 {
        let yaml = format!("owned_key{}: owned_value{}", i, i);
        let doc = Document::from_string(yaml).unwrap();
        let root = doc.root().unwrap();
        assert!(root.is_mapping());
        // Doc dropped with OwnedString input
    }
}

/// Stress test for Editor::build_from_yaml error path.
/// Verifies no memory leak when build_from_yaml fails.
#[test]
fn destructor_stress_build_from_yaml_error() {
    let mut doc = Document::parse_str("key: value").unwrap();
    for _ in 0..100 {
        let mut ed = doc.edit();
        // Invalid YAML should fail but not leak
        let result = ed.build_from_yaml("[unclosed");
        assert!(result.is_err());
    }
}

/// Stress test for delete_at (tests fy_node_free on removed nodes).
#[test]
fn destructor_stress_delete_at() {
    for _ in 0..50 {
        let mut doc = Document::parse_str("a: 1\nb: 2\nc: 3\nd: 4").unwrap();
        {
            let mut ed = doc.edit();
            ed.delete_at("/a").unwrap();
            ed.delete_at("/b").unwrap();
            ed.delete_at("/c").unwrap();
            ed.delete_at("/d").unwrap();
        }
        assert!(doc.root().unwrap().map_iter().next().is_none());
    }
}

// =============================================================================
// Document::from_bytes Tests
// =============================================================================

#[test]
fn from_bytes_basic() {
    let yaml_bytes = b"name: Alice\nage: 30".to_vec();
    let doc = Document::from_bytes(yaml_bytes).unwrap();
    let root = doc.root().unwrap();
    assert_eq!(
        root.at_path("/name").unwrap().scalar_str().unwrap(),
        "Alice"
    );
    assert_eq!(root.at_path("/age").unwrap().scalar_str().unwrap(), "30");
}

#[test]
fn from_bytes_empty_fails() {
    let result = Document::from_bytes(vec![]);
    assert!(result.is_err());
}

#[test]
fn from_bytes_zero_copy_access() {
    let yaml_bytes = b"key: value".to_vec();
    let doc = Document::from_bytes(yaml_bytes).unwrap();
    let root = doc.root().unwrap();

    // Zero-copy bytes access
    let bytes = root.at_path("/key").unwrap().scalar_bytes().unwrap();
    assert_eq!(bytes, b"value");
}

#[test]
fn from_bytes_with_utf8() {
    let yaml_bytes = "emoji: 🎉\nunicode: café".as_bytes().to_vec();
    let doc = Document::from_bytes(yaml_bytes).unwrap();
    let root = doc.root().unwrap();
    assert_eq!(root.at_path("/emoji").unwrap().scalar_str().unwrap(), "🎉");
    assert_eq!(
        root.at_path("/unicode").unwrap().scalar_str().unwrap(),
        "café"
    );
}

#[test]
fn from_bytes_complex_structure() {
    let yaml = r#"
database:
  host: localhost
  port: 5432
  users:
    - admin
    - guest
"#;
    let doc = Document::from_bytes(yaml.as_bytes().to_vec()).unwrap();
    let root = doc.root().unwrap();

    assert_eq!(
        root.at_path("/database/host")
            .unwrap()
            .scalar_str()
            .unwrap(),
        "localhost"
    );
    assert_eq!(
        root.at_path("/database/port")
            .unwrap()
            .scalar_str()
            .unwrap(),
        "5432"
    );
    assert_eq!(
        root.at_path("/database/users/0")
            .unwrap()
            .scalar_str()
            .unwrap(),
        "admin"
    );
    assert_eq!(
        root.at_path("/database/users/1")
            .unwrap()
            .scalar_str()
            .unwrap(),
        "guest"
    );
}

/// Stress test for Document::from_bytes destruction.
#[test]
fn destructor_stress_from_bytes() {
    for i in 0..100 {
        let yaml = format!("key{}: value{}", i, i);
        let doc = Document::from_bytes(yaml.into_bytes()).unwrap();
        let root = doc.root().unwrap();
        assert!(root.is_mapping());
        // Doc dropped with OwnedBytes input
    }
}
