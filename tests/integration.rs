//! Integration tests covering the API surface used by shyaml-rs.
//!
//! These tests ensure compatibility with the shyaml-rs consumer.

use fyaml::document::Document;
use fyaml::node::{Node, NodeType};
use std::os::unix::io::AsRawFd;
use std::rc::Rc;
use std::str::FromStr;

// =============================================================================
// Document Parsing and Root Node Access
// =============================================================================

#[test]
fn parse_document_from_string() {
    let yaml = "foo: bar";
    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node();
    assert!(root.is_some());
}

#[test]
fn parse_document_root_node_is_mapping() {
    let yaml = "foo: bar\nbaz: qux";
    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();
    assert!(root.is_mapping());
    assert_eq!(root.get_type(), NodeType::Mapping);
}

#[test]
fn parse_empty_document_returns_error() {
    // libfyaml rejects empty input and prints "[ERR]: fy_parse_load_document() failed"
    // directly to stderr (fd 2). Since this is a C library, Rust's test output capture
    // doesn't intercept it, causing confusing error messages in test output.
    // We temporarily redirect stderr to /dev/null during this test.
    let devnull = std::fs::File::open("/dev/null").unwrap();
    let old_stderr = unsafe { libc::dup(2) };
    unsafe { libc::dup2(devnull.as_raw_fd(), 2) };

    let result = "".parse::<Document>();

    unsafe {
        libc::dup2(old_stderr, 2);
        libc::close(old_stderr);
    }

    assert!(result.is_err());
}

#[test]
fn document_to_string() {
    let yaml = "foo: bar";
    let doc = yaml.parse::<Document>().unwrap();
    let output = doc.to_string();
    assert!(output.contains("foo"));
    assert!(output.contains("bar"));
}

// =============================================================================
// Node Type Detection (as used by shyaml's nt2shyaml)
// =============================================================================

#[test]
fn node_type_scalar() {
    let yaml = "hello";
    let node = Node::from_str(yaml).unwrap();
    assert_eq!(node.get_type(), NodeType::Scalar);
    assert!(node.is_scalar());
    assert!(!node.is_mapping());
    assert!(!node.is_sequence());
}

#[test]
fn node_type_mapping() {
    let yaml = "foo: bar\nbaz: qux";
    let node = Node::from_str(yaml).unwrap();
    assert_eq!(node.get_type(), NodeType::Mapping);
    assert!(node.is_mapping());
    assert!(!node.is_scalar());
    assert!(!node.is_sequence());
}

#[test]
fn node_type_sequence() {
    let yaml = "- one\n- two\n- three";
    let node = Node::from_str(yaml).unwrap();
    assert_eq!(node.get_type(), NodeType::Sequence);
    assert!(node.is_sequence());
    assert!(!node.is_scalar());
    assert!(!node.is_mapping());
}

// =============================================================================
// Path Navigation (as used by shyaml's traverse)
// =============================================================================

#[test]
fn node_by_path_simple_key() {
    let yaml = "foo: bar";
    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();
    let node = root.node_by_path("/foo").unwrap();
    assert_eq!(node.to_raw_string().unwrap(), "bar");
}

#[test]
fn node_by_path_nested() {
    let yaml = "database:\n  host: localhost\n  port: 5432";
    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();

    let host = root.node_by_path("/database/host").unwrap();
    assert_eq!(host.to_raw_string().unwrap(), "localhost");

    let port = root.node_by_path("/database/port").unwrap();
    assert_eq!(port.to_raw_string().unwrap(), "5432");
}

#[test]
fn node_by_path_sequence_index() {
    let yaml = "items:\n  - first\n  - second\n  - third";
    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();

    let first = root.node_by_path("/items/0").unwrap();
    assert_eq!(first.to_raw_string().unwrap(), "first");

    let second = root.node_by_path("/items/1").unwrap();
    assert_eq!(second.to_raw_string().unwrap(), "second");
}

#[test]
fn node_by_path_not_found() {
    let yaml = "foo: bar";
    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();
    let result = root.node_by_path("/nonexistent");
    assert!(result.is_none());
}

#[test]
fn node_by_path_empty_returns_root() {
    let yaml = "foo: bar";
    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();
    let node = root.node_by_path("").unwrap();
    // Empty path should return the node itself
    assert!(node.is_mapping());
}

#[test]
fn node_by_path_returns_arc() {
    // Verify the return type is Rc<Node> for shared ownership
    let yaml = "foo: bar";
    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();
    let node: Rc<Node> = root.node_by_path("/foo").unwrap();
    let cloned = Rc::clone(&node);
    assert_eq!(
        node.to_raw_string().unwrap(),
        cloned.to_raw_string().unwrap()
    );
}

// =============================================================================
// Value Extraction (to_raw_string vs to_string)
// =============================================================================

#[test]
fn to_raw_string_unquoted() {
    let yaml = "value: hello world";
    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();
    let node = root.node_by_path("/value").unwrap();
    // to_raw_string returns unquoted value
    assert_eq!(node.to_raw_string().unwrap(), "hello world");
}

#[test]
fn to_raw_string_preserves_content() {
    let yaml = "value: \"bar: wiz\"";
    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();
    let node = root.node_by_path("/value").unwrap();
    // to_raw_string returns content without quotes
    assert_eq!(node.to_raw_string().unwrap(), "bar: wiz");
}

#[test]
fn to_string_yaml_formatted() {
    let yaml = "value: \"bar: wiz\"";
    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();
    let node = root.node_by_path("/value").unwrap();
    // to_string returns YAML-formatted (quoted because colon)
    assert_eq!(node.to_string(), "\"bar: wiz\"");
}

#[test]
fn to_string_on_mapping() {
    let yaml = "nested:\n  a: 1\n  b: 2";
    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();
    let node = root.node_by_path("/nested").unwrap();
    let output = node.to_string();
    assert!(output.contains("a:"));
    assert!(output.contains("b:"));
}

#[test]
fn to_string_on_sequence() {
    let yaml = "- one\n- two";
    let node = Node::from_str(yaml).unwrap();
    let output = node.to_string();
    assert!(output.contains("- one"));
    assert!(output.contains("- two"));
}

// =============================================================================
// Mapping Iteration (as used by shyaml's keys, values, key_values)
// =============================================================================

#[test]
fn map_iter_yields_key_value_pairs() {
    let yaml = "a: 1\nb: 2\nc: 3";
    let node = Node::from_str(yaml).unwrap();
    let pairs: Vec<_> = node.map_iter().collect();

    assert_eq!(pairs.len(), 3);

    let (key, value) = pairs[0].as_ref().unwrap();
    assert_eq!(key.to_raw_string().unwrap(), "a");
    assert_eq!(value.to_raw_string().unwrap(), "1");
}

#[test]
fn map_iter_keys_extraction() {
    // Pattern used by shyaml's MapKeyIterator
    let yaml = "foo: 1\nbar: 2\nbaz: 3";
    let node = Node::from_str(yaml).unwrap();
    let keys: Vec<String> = node
        .map_iter()
        .filter_map(|r| r.ok())
        .map(|(k, _)| k.to_raw_string().unwrap())
        .collect();

    assert_eq!(keys, vec!["foo", "bar", "baz"]);
}

#[test]
fn map_iter_values_extraction() {
    // Pattern used by shyaml's MapValueIterator
    let yaml = "foo: one\nbar: two\nbaz: three";
    let node = Node::from_str(yaml).unwrap();
    let values: Vec<String> = node
        .map_iter()
        .filter_map(|r| r.ok())
        .map(|(_, v)| v.to_raw_string().unwrap())
        .collect();

    assert_eq!(values, vec!["one", "two", "three"]);
}

#[test]
fn map_iter_with_nested_values() {
    let yaml = "simple: value\nnested:\n  inner: data";
    let node = Node::from_str(yaml).unwrap();

    for result in node.map_iter() {
        let (key, value) = result.unwrap();
        if key.to_raw_string().unwrap() == "nested" {
            assert!(value.is_mapping());
            let inner = value.node_by_path("/inner").unwrap();
            assert_eq!(inner.to_raw_string().unwrap(), "data");
        }
    }
}

// =============================================================================
// Sequence Iteration (as used by shyaml's get_values)
// =============================================================================

#[test]
fn seq_iter_yields_nodes() {
    let yaml = "- alpha\n- beta\n- gamma";
    let node = Node::from_str(yaml).unwrap();
    let items: Vec<String> = node
        .seq_iter()
        .filter_map(|r| r.ok())
        .map(|n| n.to_raw_string().unwrap())
        .collect();

    assert_eq!(items, vec!["alpha", "beta", "gamma"]);
}

#[test]
fn seq_iter_with_nested_mappings() {
    let yaml = "- name: first\n  value: 1\n- name: second\n  value: 2";
    let node = Node::from_str(yaml).unwrap();

    let items: Vec<_> = node.seq_iter().collect();
    assert_eq!(items.len(), 2);

    let first = items[0].as_ref().unwrap();
    assert!(first.is_mapping());
    let name = first.node_by_path("/name").unwrap();
    assert_eq!(name.to_raw_string().unwrap(), "first");
}

// =============================================================================
// Length Operations (as used by shyaml's get_length)
// =============================================================================

#[test]
fn map_len_returns_count() {
    let yaml = "a: 1\nb: 2\nc: 3\nd: 4";
    let node = Node::from_str(yaml).unwrap();
    assert_eq!(node.map_len().unwrap(), 4);
}

#[test]
fn seq_len_returns_count() {
    let yaml = "- one\n- two\n- three";
    let node = Node::from_str(yaml).unwrap();
    assert_eq!(node.seq_len().unwrap(), 3);
}

#[test]
fn map_len_empty() {
    let yaml = "{}";
    let node = Node::from_str(yaml).unwrap();
    assert_eq!(node.map_len().unwrap(), 0);
}

#[test]
fn seq_len_empty() {
    let yaml = "[]";
    let node = Node::from_str(yaml).unwrap();
    assert_eq!(node.seq_len().unwrap(), 0);
}

// =============================================================================
// Tag Handling (as used by shyaml's nt2shyaml for custom types)
// =============================================================================

#[test]
fn get_tag_none_for_untagged() {
    let yaml = "value: hello";
    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();
    let node = root.node_by_path("/value").unwrap();
    assert!(node.get_tag().unwrap().is_none());
}

#[test]
fn get_tag_returns_tag() {
    let yaml = "value: !custom tagged_value";
    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();
    let node = root.node_by_path("/value").unwrap();
    let tag = node.get_tag().unwrap();
    assert!(tag.is_some());
    assert!(tag.unwrap().contains("custom"));
}

// =============================================================================
// Library Version (as used by shyaml's --version)
// =============================================================================

#[test]
fn get_c_version_returns_string() {
    let version = fyaml::get_c_version().unwrap();
    assert!(!version.is_empty());
    // Version should contain digits (e.g., "0.9.1")
    assert!(version.chars().any(|c| c.is_ascii_digit()));
}

// =============================================================================
// Complex Scenarios (real-world patterns from shyaml)
// =============================================================================

#[test]
fn traverse_and_extract_pattern() {
    // Pattern: parse -> root_node -> node_by_path -> to_raw_string/to_string
    let yaml = indoc::indoc! {"
        database:
          host: localhost
          port: 5432
          credentials:
            user: admin
            pass: secret
    "};

    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();

    // Navigate to nested path
    let user = root.node_by_path("/database/credentials/user").unwrap();
    assert_eq!(user.to_raw_string().unwrap(), "admin");
}

#[test]
fn iterate_mapping_keys_values_pattern() {
    // Pattern used by shyaml keys/values commands
    let yaml = indoc::indoc! {"
        config:
          debug: true
          timeout: 30
          name: myapp
    "};

    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();
    let config = root.node_by_path("/config").unwrap();

    assert!(config.is_mapping());
    assert_eq!(config.map_len().unwrap(), 3);

    let mut keys = Vec::new();
    let mut values = Vec::new();
    for result in config.map_iter() {
        let (k, v) = result.unwrap();
        keys.push(k.to_raw_string().unwrap());
        if v.is_scalar() {
            values.push(v.to_raw_string().unwrap());
        } else {
            values.push(v.to_string());
        }
    }

    assert_eq!(keys, vec!["debug", "timeout", "name"]);
    assert_eq!(values, vec!["true", "30", "myapp"]);
}

#[test]
fn iterate_sequence_pattern() {
    // Pattern used by shyaml get-values on sequences
    let yaml = indoc::indoc! {"
        servers:
          - host: server1
            port: 8080
          - host: server2
            port: 8081
    "};

    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();
    let servers = root.node_by_path("/servers").unwrap();

    assert!(servers.is_sequence());
    assert_eq!(servers.seq_len().unwrap(), 2);

    let hosts: Vec<String> = servers
        .seq_iter()
        .filter_map(|r| r.ok())
        .filter_map(|n| n.node_by_path("/host"))
        .map(|n| n.to_raw_string().unwrap())
        .collect();

    assert_eq!(hosts, vec!["server1", "server2"]);
}

#[test]
fn type_detection_for_output_format() {
    // Pattern: check type to decide raw vs yaml output
    let yaml = indoc::indoc! {"
        scalar: simple
        mapping:
          key: value
        sequence:
          - item
    "};

    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();

    let scalar = root.node_by_path("/scalar").unwrap();
    let mapping = root.node_by_path("/mapping").unwrap();
    let sequence = root.node_by_path("/sequence").unwrap();

    // shyaml uses is_scalar() to decide output format
    assert!(scalar.is_scalar());
    assert!(!mapping.is_scalar());
    assert!(!sequence.is_scalar());

    // For scalar: to_raw_string, for others: to_string
    assert_eq!(scalar.to_raw_string().unwrap(), "simple");
    assert!(mapping.to_string().contains("key:"));
    assert!(sequence.to_string().contains("- item"));
}

#[test]
fn special_characters_in_values() {
    // Values with colons, quotes, etc.
    let yaml = indoc::indoc! {r#"
        url: "https://example.com:8080/path"
        message: "Hello: World"
        quoted: 'single quoted'
    "#};

    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();

    let url = root.node_by_path("/url").unwrap();
    assert_eq!(
        url.to_raw_string().unwrap(),
        "https://example.com:8080/path"
    );

    let message = root.node_by_path("/message").unwrap();
    assert_eq!(message.to_raw_string().unwrap(), "Hello: World");
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

    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();

    let literal = root.node_by_path("/literal").unwrap();
    let content = literal.to_raw_string().unwrap();
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

    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();

    // All are scalars
    assert!(root.node_by_path("/integer").unwrap().is_scalar());
    assert!(root.node_by_path("/float").unwrap().is_scalar());
    assert!(root.node_by_path("/negative").unwrap().is_scalar());
    assert!(root.node_by_path("/string_num").unwrap().is_scalar());

    // Raw string extraction
    assert_eq!(
        root.node_by_path("/integer")
            .unwrap()
            .to_raw_string()
            .unwrap(),
        "42"
    );
    assert_eq!(
        root.node_by_path("/float")
            .unwrap()
            .to_raw_string()
            .unwrap(),
        "3.14"
    );
}

#[test]
fn boolean_and_null_values() {
    let yaml = indoc::indoc! {"
        enabled: true
        disabled: false
        empty: null
        tilde_null: ~
    "};

    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();

    assert_eq!(
        root.node_by_path("/enabled")
            .unwrap()
            .to_raw_string()
            .unwrap(),
        "true"
    );
    assert_eq!(
        root.node_by_path("/disabled")
            .unwrap()
            .to_raw_string()
            .unwrap(),
        "false"
    );
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

    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();

    let deep = root
        .node_by_path("/level1/level2/level3/level4/value")
        .unwrap();
    assert_eq!(deep.to_raw_string().unwrap(), "deep");
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

    let doc = yaml.parse::<Document>().unwrap();
    let root = doc.root_node().unwrap();

    // Navigate to alice's first role
    let alice_admin = root.node_by_path("/users/0/roles/0").unwrap();
    assert_eq!(alice_admin.to_raw_string().unwrap(), "admin");

    // Navigate to bob's roles and check length
    let bob_roles = root.node_by_path("/users/1/roles").unwrap();
    assert_eq!(bob_roles.seq_len().unwrap(), 1);
}
