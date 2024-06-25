//! Safe Rust bindings for libfyaml YAML parser.
//!
//! This crate provides a safe interface to the [libfyaml](https://github.com/pantoniou/libfyaml)
//! C library, enabling DOM-style navigation, path queries, multi-document parsing,
//! and a serde-compatible [`Value`] type.
//!
//! **Note:** This library is in early development and has not been widely used or
//! audited. The API may change. For production use cases requiring a mature library,
//! consider `serde_yml` or `serde-yaml-ng`.
//!
//! # Features
//!
//! - Parse YAML strings into document/node objects
//! - Navigate nodes using path-based queries (e.g., `/foo/bar`, `/list/0`)
//! - Support for all YAML node types: scalars, sequences, and mappings
//! - Iterate over mapping key-value pairs and sequence items
//! - Multi-document stream parsing from strings or stdin
//! - **[`Value`] type**: Pure Rust enum with serde support and libfyaml-powered emission
//!
//! # Quick Start with Value
//!
//! The [`Value`] type provides a convenient way to work with YAML data:
//!
//! ```
//! use fyaml::Value;
//!
//! // Parse YAML
//! let value: Value = "name: Alice\nage: 30".parse().unwrap();
//!
//! // Access values with indexing
//! assert_eq!(value["name"].as_str(), Some("Alice"));
//!
//! // Emit back to YAML
//! let yaml = value.to_yaml_string().unwrap();
//! assert!(yaml.contains("name: Alice"));
//! ```
//!
//! # Low-level Node API
//!
//! For more control, use the [`Node`](node::Node) API directly:
//!
//! ```
//! use fyaml::node::Node;
//! use std::str::FromStr;
//!
//! let yaml = "database:\n  host: localhost\n  port: 5432";
//! let root = Node::from_str(yaml).unwrap();
//!
//! let host = root.node_by_path("/database/host").unwrap();
//! assert_eq!(host.to_raw_string().unwrap(), "localhost");
//! ```
//!
//! # Path Syntax
//!
//! Paths use `/` as the separator (following JSON Pointer conventions):
//! - `/key` - access a mapping key
//! - `/0` - access a sequence index
//! - `/parent/child/0` - nested access
//!
//! # Serde Integration
//!
//! [`Value`] implements `Serialize` and `Deserialize`, enabling interoperability
//! with other serde-compatible formats:
//!
//! ```
//! use fyaml::Value;
//!
//! let value: Value = "key: value".parse().unwrap();
//!
//! // Serialize to JSON
//! let json = serde_json::to_string(&value).unwrap();
//! assert_eq!(json, r#"{"key":"value"}"#);
//!
//! // Deserialize from JSON
//! let from_json: Value = serde_json::from_str(&json).unwrap();
//! assert_eq!(from_json["key"].as_str(), Some("value"));
//! ```

pub mod document;
pub mod node;
pub mod value;

// Re-export commonly used types
pub use value::{Number, TaggedValue, Value};

/// Returns the version string of the underlying libfyaml C library.
pub fn get_c_version() -> Result<String, String> {
    log::trace!("get_c_version()");
    let cstr_ptr = unsafe { fyaml_sys::fy_library_version() };
    if cstr_ptr.is_null() {
        log::error!("Null pointer received from fy_library_version");
        return Err("Unknown version".to_string());
    }
    log::trace!("convert to string");
    let str = unsafe { std::ffi::CStr::from_ptr(cstr_ptr) };
    log::trace!("done !");
    Ok(str.to_string_lossy().to_string())
}

#[cfg(test)]
mod tests {
    use crate::node::Node;
    use std::str::FromStr;

    fn path(yaml: &str, path: &str) -> String {
        let dom = Node::from_str(yaml).unwrap();
        let node = dom.node_by_path(path).unwrap();
        node.to_string()
    }

    #[test]
    fn test_simple_hash() {
        assert_eq!(
            path(
                r#"
        foo: bar
        "#,
                "/foo"
            ),
            "bar"
        );
    }

    #[test]
    fn test_no_path() {
        assert_eq!(
            path(
                r#"
        foo: bar
        "#,
                ""
            ),
            "foo: bar"
        );
    }

    #[test]
    fn test_trap() {
        assert_eq!(
            path(
                r#"
        foo: "bar: wiz"
        "#,
                "foo"
            ),
            "\"bar: wiz\""
        );
    }
}
