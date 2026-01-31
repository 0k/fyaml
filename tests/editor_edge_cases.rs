//! Editor edge cases and error path tests.
//!
//! Tests for editor operations at boundaries and error conditions.

use fyaml::Document;
use fyaml::NodeStyle;

// =============================================================================
// Root Operations
// =============================================================================

#[test]
fn editor_set_yaml_at_slash_root() {
    let mut doc = Document::new().unwrap();
    {
        let mut ed = doc.edit();
        ed.set_yaml_at("/", "key: value").unwrap();
    }
    assert!(doc.root().is_some());
    assert!(doc.root().unwrap().is_mapping());
}

#[test]
fn editor_set_yaml_at_empty_path() {
    let mut doc = Document::new().unwrap();
    {
        let mut ed = doc.edit();
        ed.set_yaml_at("", "key: value").unwrap();
    }
    assert!(doc.root().is_some());
}

#[test]
fn editor_delete_at_root_fails() {
    let mut doc = Document::parse_str("key: value").unwrap();
    {
        let mut ed = doc.edit();
        let result = ed.delete_at("/");
        assert!(result.is_err());
    }
}

#[test]
fn editor_delete_at_empty_path_fails() {
    let mut doc = Document::parse_str("key: value").unwrap();
    {
        let mut ed = doc.edit();
        let result = ed.delete_at("");
        assert!(result.is_err());
    }
}

// =============================================================================
// Sequence Operations
// =============================================================================

#[test]
fn editor_delete_sequence_first_element() {
    let mut doc = Document::parse_str("items:\n  - a\n  - b\n  - c").unwrap();
    {
        let mut ed = doc.edit();
        let deleted = ed.delete_at("/items/0").unwrap();
        assert!(deleted);
    }
    let items = doc.at_path("/items").unwrap();
    assert_eq!(items.seq_len().unwrap(), 2);
    assert_eq!(items.seq_get(0).unwrap().scalar_str().unwrap(), "b");
}

#[test]
fn editor_delete_sequence_middle_element() {
    let mut doc = Document::parse_str("items:\n  - a\n  - b\n  - c").unwrap();
    {
        let mut ed = doc.edit();
        let deleted = ed.delete_at("/items/1").unwrap();
        assert!(deleted);
    }
    let items = doc.at_path("/items").unwrap();
    assert_eq!(items.seq_len().unwrap(), 2);
    assert_eq!(items.seq_get(0).unwrap().scalar_str().unwrap(), "a");
    assert_eq!(items.seq_get(1).unwrap().scalar_str().unwrap(), "c");
}

#[test]
fn editor_delete_sequence_last_element() {
    let mut doc = Document::parse_str("items:\n  - a\n  - b\n  - c").unwrap();
    {
        let mut ed = doc.edit();
        let deleted = ed.delete_at("/items/2").unwrap();
        assert!(deleted);
    }
    let items = doc.at_path("/items").unwrap();
    assert_eq!(items.seq_len().unwrap(), 2);
    assert_eq!(items.seq_get(1).unwrap().scalar_str().unwrap(), "b");
}

#[test]
fn editor_delete_sequence_out_of_bounds() {
    let mut doc = Document::parse_str("items:\n  - a\n  - b").unwrap();
    {
        let mut ed = doc.edit();
        let deleted = ed.delete_at("/items/10").unwrap();
        assert!(!deleted);
    }
}

#[test]
fn editor_seq_append_at() {
    let mut doc = Document::parse_str("items:\n  - a\n  - b").unwrap();
    {
        let mut ed = doc.edit();
        let new_item = ed.build_scalar("c").unwrap();
        ed.seq_append_at("/items", new_item).unwrap();
    }
    let items = doc.at_path("/items").unwrap();
    assert_eq!(items.seq_len().unwrap(), 3);
    assert_eq!(items.seq_get(2).unwrap().scalar_str().unwrap(), "c");
}

#[test]
fn editor_seq_append_at_empty_sequence() {
    let mut doc = Document::parse_str("items: []").unwrap();
    {
        let mut ed = doc.edit();
        let item = ed.build_scalar("first").unwrap();
        ed.seq_append_at("/items", item).unwrap();
    }
    let items = doc.at_path("/items").unwrap();
    assert_eq!(items.seq_len().unwrap(), 1);
}

#[test]
fn editor_seq_append_at_non_sequence_fails() {
    let mut doc = Document::parse_str("mapping:\n  key: value").unwrap();
    {
        let mut ed = doc.edit();
        let item = ed.build_scalar("x").unwrap();
        let result = ed.seq_append_at("/mapping", item);
        assert!(result.is_err());
    }
}

#[test]
fn editor_seq_append_at_root_sequence() {
    let mut doc = Document::parse_str("[a, b]").unwrap();
    {
        let mut ed = doc.edit();
        let item = ed.build_scalar("c").unwrap();
        ed.seq_append_at("", item).unwrap();
    }
    let root = doc.root().unwrap();
    assert_eq!(root.seq_len().unwrap(), 3);
}

// =============================================================================
// Error Paths
// =============================================================================

#[test]
fn editor_set_yaml_at_non_mapping_parent_fails() {
    let mut doc = Document::parse_str("scalar_root").unwrap();
    {
        let mut ed = doc.edit();
        let result = ed.set_yaml_at("/child", "value");
        assert!(result.is_err());
    }
}

#[test]
fn editor_set_yaml_at_nonexistent_parent_fails() {
    let mut doc = Document::parse_str("existing: value").unwrap();
    {
        let mut ed = doc.edit();
        let result = ed.set_yaml_at("/nonexistent/child", "value");
        assert!(result.is_err());
    }
}

#[test]
fn editor_delete_at_nonexistent_parent() {
    let mut doc = Document::parse_str("key: value").unwrap();
    {
        let mut ed = doc.edit();
        let deleted = ed.delete_at("/nonexistent/child").unwrap();
        assert!(!deleted);
    }
}

#[test]
fn editor_build_from_yaml_invalid() {
    let mut doc = Document::new().unwrap();
    {
        let mut ed = doc.edit();
        let result = ed.build_from_yaml("[unclosed");
        assert!(result.is_err());
    }
}

#[test]
fn editor_build_from_yaml_multiple_docs() {
    // Multiple documents in one snippet - takes first
    let mut doc = Document::new().unwrap();
    {
        let mut ed = doc.edit();
        let node = ed.build_from_yaml("first").unwrap();
        ed.set_root(node).unwrap();
    }
    assert_eq!(doc.root().unwrap().scalar_str().unwrap(), "first");
}

// =============================================================================
// Reading During Edit
// =============================================================================

#[test]
fn editor_read_root_during_edit() {
    let mut doc = Document::parse_str("name: Alice").unwrap();
    {
        let ed = doc.edit();
        let root = ed.root().unwrap();
        assert_eq!(
            root.at_path("/name").unwrap().scalar_str().unwrap(),
            "Alice"
        );
    }
}

#[test]
fn editor_at_path_during_edit() {
    let mut doc = Document::parse_str("nested:\n  key: value").unwrap();
    {
        let ed = doc.edit();
        let node = ed.at_path("/nested/key").unwrap();
        assert_eq!(node.scalar_str().unwrap(), "value");
    }
}

#[test]
fn editor_at_path_nonexistent_during_edit() {
    let mut doc = Document::parse_str("key: value").unwrap();
    {
        let ed = doc.edit();
        assert!(ed.at_path("/nonexistent").is_none());
    }
}

#[test]
fn editor_root_on_empty_document() {
    let mut doc = Document::new().unwrap();
    {
        let ed = doc.edit();
        assert!(ed.root().is_none());
    }
}

// =============================================================================
// Node Building
// =============================================================================

#[test]
fn editor_build_scalar() {
    let mut doc = Document::new().unwrap();
    {
        let mut ed = doc.edit();
        let scalar = ed.build_scalar("test value").unwrap();
        ed.set_root(scalar).unwrap();
    }
    assert_eq!(doc.root().unwrap().scalar_str().unwrap(), "test value");
}

#[test]
fn editor_build_sequence() {
    let mut doc = Document::new().unwrap();
    {
        let mut ed = doc.edit();
        let seq = ed.build_sequence().unwrap();
        ed.set_root(seq).unwrap();
    }
    let root = doc.root().unwrap();
    assert!(root.is_sequence());
    assert_eq!(root.seq_len().unwrap(), 0);
}

#[test]
fn editor_build_mapping() {
    let mut doc = Document::new().unwrap();
    {
        let mut ed = doc.edit();
        let map = ed.build_mapping().unwrap();
        ed.set_root(map).unwrap();
    }
    let root = doc.root().unwrap();
    assert!(root.is_mapping());
    assert_eq!(root.map_len().unwrap(), 0);
}

#[test]
fn editor_build_complex_yaml() {
    let mut doc = Document::new().unwrap();
    {
        let mut ed = doc.edit();
        let node = ed
            .build_from_yaml("users:\n  - name: Alice\n    age: 30\n  - name: Bob\n    age: 25")
            .unwrap();
        ed.set_root(node).unwrap();
    }

    let root = doc.root().unwrap();
    assert!(root.is_mapping());
    assert_eq!(
        root.at_path("/users/0/name").unwrap().scalar_str().unwrap(),
        "Alice"
    );
    assert_eq!(
        root.at_path("/users/1/name").unwrap().scalar_str().unwrap(),
        "Bob"
    );
}

// =============================================================================
// Style Preservation
// =============================================================================

#[test]
fn editor_preserves_single_quotes() {
    let mut doc = Document::parse_str("name: plain").unwrap();
    {
        let mut ed = doc.edit();
        ed.set_yaml_at("/name", "'single quoted'").unwrap();
    }

    let node = doc.at_path("/name").unwrap();
    assert_eq!(node.style(), NodeStyle::SingleQuoted);
    assert_eq!(node.scalar_str().unwrap(), "single quoted");
}

#[test]
fn editor_preserves_double_quotes() {
    let mut doc = Document::parse_str("name: plain").unwrap();
    {
        let mut ed = doc.edit();
        ed.set_yaml_at("/name", "\"double quoted\"").unwrap();
    }

    let node = doc.at_path("/name").unwrap();
    assert_eq!(node.style(), NodeStyle::DoubleQuoted);
}

#[test]
fn editor_set_nested_value() {
    let mut doc = Document::parse_str("a:\n  b:\n    c: old").unwrap();
    {
        let mut ed = doc.edit();
        ed.set_yaml_at("/a/b/c", "new").unwrap();
    }
    assert_eq!(doc.at_path("/a/b/c").unwrap().scalar_str().unwrap(), "new");
}

#[test]
fn editor_add_new_nested_key() {
    let mut doc = Document::parse_str("existing:\n  key: value").unwrap();
    {
        let mut ed = doc.edit();
        ed.set_yaml_at("/existing/new_key", "new_value").unwrap();
    }
    assert_eq!(
        doc.at_path("/existing/new_key")
            .unwrap()
            .scalar_str()
            .unwrap(),
        "new_value"
    );
    // Original key unchanged
    assert_eq!(
        doc.at_path("/existing/key").unwrap().scalar_str().unwrap(),
        "value"
    );
}

// =============================================================================
// Replace Root
// =============================================================================

#[test]
fn editor_replace_existing_root() {
    let mut doc = Document::parse_str("old: root").unwrap();
    {
        let mut ed = doc.edit();
        let new_root = ed.build_from_yaml("new: root").unwrap();
        ed.set_root(new_root).unwrap();
    }
    assert!(doc.at_path("/old").is_none());
    assert_eq!(doc.at_path("/new").unwrap().scalar_str().unwrap(), "root");
}

#[test]
fn editor_set_root_scalar() {
    let mut doc = Document::new().unwrap();
    {
        let mut ed = doc.edit();
        let scalar = ed.build_from_yaml("just_a_scalar").unwrap();
        ed.set_root(scalar).unwrap();
    }
    let root = doc.root().unwrap();
    assert!(root.is_scalar());
    assert_eq!(root.scalar_str().unwrap(), "just_a_scalar");
}

#[test]
fn editor_set_root_sequence() {
    let mut doc = Document::new().unwrap();
    {
        let mut ed = doc.edit();
        let seq = ed.build_from_yaml("[1, 2, 3]").unwrap();
        ed.set_root(seq).unwrap();
    }
    let root = doc.root().unwrap();
    assert!(root.is_sequence());
    assert_eq!(root.seq_len().unwrap(), 3);
}

// =============================================================================
// Handle-Level Assembly
// =============================================================================

#[test]
fn editor_build_structure_then_mutate_with_paths() {
    // Build a structure with handle-level API, then modify it with path-based API
    let mut doc = Document::new().unwrap();
    {
        let mut ed = doc.edit();
        let mut root = ed.build_mapping().unwrap();
        let k = ed.build_scalar("name").unwrap();
        let v = ed.build_scalar("Alice").unwrap();
        ed.map_insert(&mut root, k, v).unwrap();
        let k2 = ed.build_scalar("age").unwrap();
        let v2 = ed.build_scalar("30").unwrap();
        ed.map_insert(&mut root, k2, v2).unwrap();
        ed.set_root(root).unwrap();
    }
    // Now use path-based mutation on the programmatically built structure
    {
        let mut ed = doc.edit();
        ed.set_yaml_at("/name", "'Bob'").unwrap();
        ed.delete_at("/age").unwrap();
        ed.set_yaml_at("/role", "admin").unwrap();
    }
    assert_eq!(doc.at_path("/name").unwrap().scalar_str().unwrap(), "Bob");
    assert!(doc.at_path("/age").is_none());
    assert_eq!(doc.at_path("/role").unwrap().scalar_str().unwrap(), "admin");
}

#[test]
fn editor_seq_append_then_append_at() {
    // Build a sequence with handle-level API, then extend it with path-based API
    let mut doc = Document::new().unwrap();
    {
        let mut ed = doc.edit();
        let mut root = ed.build_mapping().unwrap();
        let k = ed.build_scalar("items").unwrap();
        let mut seq = ed.build_sequence().unwrap();
        let a = ed.build_scalar("a").unwrap();
        let b = ed.build_scalar("b").unwrap();
        ed.seq_append(&mut seq, a).unwrap();
        ed.seq_append(&mut seq, b).unwrap();
        ed.map_insert(&mut root, k, seq).unwrap();
        ed.set_root(root).unwrap();
    }
    {
        let mut ed = doc.edit();
        let c = ed.build_scalar("c").unwrap();
        ed.seq_append_at("/items", c).unwrap();
        ed.set_yaml_at("/items/0", "replaced").unwrap();
    }
    let items = doc.at_path("/items").unwrap();
    assert_eq!(items.seq_len().unwrap(), 3);
    assert_eq!(items.seq_get(0).unwrap().scalar_str().unwrap(), "replaced");
    assert_eq!(items.seq_get(1).unwrap().scalar_str().unwrap(), "b");
    assert_eq!(items.seq_get(2).unwrap().scalar_str().unwrap(), "c");
}

#[test]
fn editor_build_with_nulls_and_tags() {
    let mut doc = Document::new().unwrap();
    {
        let mut ed = doc.edit();
        let mut root = ed.build_mapping().unwrap();

        let k1 = ed.build_scalar("present").unwrap();
        let v1 = ed.build_scalar("yes").unwrap();
        ed.map_insert(&mut root, k1, v1).unwrap();

        let k2 = ed.build_scalar("missing").unwrap();
        let v2 = ed.build_null().unwrap();
        ed.map_insert(&mut root, k2, v2).unwrap();

        ed.set_tag(&mut root, "!mytype").unwrap();
        ed.set_root(root).unwrap();
    }
    let root = doc.root().unwrap();
    assert_eq!(root.tag_str().unwrap().unwrap(), "!mytype");
    assert_eq!(
        root.at_path("/present").unwrap().scalar_str().unwrap(),
        "yes"
    );
    // Null node has empty scalar_str
    let missing = root.at_path("/missing").unwrap();
    assert!(missing.is_scalar());
    // Replace the null with a real value via path API
    {
        let mut ed = doc.edit();
        ed.set_yaml_at("/missing", "now here").unwrap();
    }
    assert_eq!(
        doc.at_path("/missing").unwrap().scalar_str().unwrap(),
        "now here"
    );
}

#[test]
fn editor_nested_handle_assembly() {
    // Build a nested structure entirely with handle-level API
    let mut doc = Document::new().unwrap();
    {
        let mut ed = doc.edit();

        // Inner mapping: {host: localhost, port: 5432}
        let mut db = ed.build_mapping().unwrap();
        let k = ed.build_scalar("host").unwrap();
        let v = ed.build_scalar("localhost").unwrap();
        ed.map_insert(&mut db, k, v).unwrap();
        let k = ed.build_scalar("port").unwrap();
        let v = ed.build_scalar("5432").unwrap();
        ed.map_insert(&mut db, k, v).unwrap();

        // Sequence: [web, api]
        let mut services = ed.build_sequence().unwrap();
        let s1 = ed.build_scalar("web").unwrap();
        let s2 = ed.build_scalar("api").unwrap();
        ed.seq_append(&mut services, s1).unwrap();
        ed.seq_append(&mut services, s2).unwrap();

        // Root mapping
        let mut root = ed.build_mapping().unwrap();
        let k = ed.build_scalar("database").unwrap();
        ed.map_insert(&mut root, k, db).unwrap();
        let k = ed.build_scalar("services").unwrap();
        ed.map_insert(&mut root, k, services).unwrap();

        ed.set_root(root).unwrap();
    }
    assert_eq!(
        doc.at_path("/database/host").unwrap().scalar_str().unwrap(),
        "localhost"
    );
    assert_eq!(
        doc.at_path("/services/1").unwrap().scalar_str().unwrap(),
        "api"
    );

    // Emit and reparse to verify structural integrity
    let yaml = doc.emit().unwrap();
    let reparsed = Document::parse_str(&yaml).unwrap();
    assert_eq!(
        reparsed
            .at_path("/database/port")
            .unwrap()
            .scalar_str()
            .unwrap(),
        "5432"
    );
    assert_eq!(
        reparsed
            .at_path("/services/0")
            .unwrap()
            .scalar_str()
            .unwrap(),
        "web"
    );
}

#[test]
fn editor_copy_then_tag_and_insert() {
    // Copy a node from one document, tag it, and insert into another
    let src = Document::parse_str("template:\n  x: 1\n  y: 2").unwrap();
    let mut dest = Document::new().unwrap();
    {
        let mut ed = dest.edit();
        let mut copied = ed.copy_node(src.at_path("/template").unwrap()).unwrap();
        ed.set_tag(&mut copied, "!copied").unwrap();
        ed.set_root(copied).unwrap();
    }
    let root = dest.root().unwrap();
    assert_eq!(root.tag_str().unwrap().unwrap(), "!copied");
    assert_eq!(root.at_path("/x").unwrap().scalar_str().unwrap(), "1");
}
