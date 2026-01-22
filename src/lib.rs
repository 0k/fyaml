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
//! - Zero-copy scalar access via lifetime-bound [`NodeRef`] and [`ValueRef`]
//!
//! # Memory Model: Choosing the Right Type
//!
//! libfyaml is a zero-copy parser - it avoids copying string data during parsing.
//! This crate exposes that efficiency through different types:
//!
//! | Type | Allocates? | Use When |
//! |------|------------|----------|
//! | [`Value`] | Yes | You need serde, transformation, or owned data |
//! | [`ValueRef<'doc>`] | No | Reading typed values (i64, bool, str) without copies |
//! | [`NodeRef<'doc>`] | No | Low-level access, iteration, raw byte access |
//!
//! **For best performance**, use `NodeRef` or `ValueRef` when you only need to read
//! data. Convert to `Value` only when you need ownership or serde compatibility.
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
//! # Low-level Document API
//!
//! For more control and zero-copy access, use the [`Document`] API directly:
//!
//! ```
//! use fyaml::Document;
//!
//! let yaml = "database:\n  host: localhost\n  port: 5432";
//! let doc = Document::parse_str(yaml).unwrap();
//! let root = doc.root().unwrap();
//!
//! // Zero-copy: returns &str pointing into document memory
//! let host = root.at_path("/database/host").unwrap();
//! assert_eq!(host.scalar_str().unwrap(), "localhost");
//! ```
//!
//! # Path Syntax
//!
//! Paths use `/` as the separator (following JSON Pointer conventions):
//! - `/key` - access a mapping key
//! - `/0` - access a sequence index
//! - `/parent/child/0` - nested access
//!
//! # Mutation via Editor
//!
//! Use [`Document::edit()`] to get an exclusive [`Editor`] for modifications:
//!
//! ```
//! use fyaml::Document;
//!
//! let mut doc = Document::parse_str("name: Alice").unwrap();
//!
//! // Mutation phase - NodeRef cannot exist during this
//! {
//!     let mut ed = doc.edit();
//!     ed.set_yaml_at("/name", "'Bob'").unwrap();
//!     ed.set_yaml_at("/age", "30").unwrap();
//! }
//!
//! // Read phase
//! let root = doc.root().unwrap();
//! assert_eq!(root.at_path("/name").unwrap().scalar_str().unwrap(), "Bob");
//! ```
//!
//! # Multi-Document Streams
//!
//! Use [`FyParser`] for parsing YAML streams with multiple documents:
//!
//! ```
//! use fyaml::FyParser;
//!
//! let yaml = "---\ndoc1: value1\n---\ndoc2: value2";
//! let parser = FyParser::from_string(yaml).unwrap();
//!
//! let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();
//! assert_eq!(docs.len(), 2);
//! ```
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

mod config;
pub mod error;
mod ffi_util;
mod node;
mod scalar_parse;
pub mod value;

// Core modules (formerly v2)
mod document;
mod editor;
mod iter;
mod node_ref;
mod parser;
mod value_ref;

// Re-export main API
pub use document::Document;
pub use editor::{Editor, RawNodeHandle};
pub use iter::{MapIter, SeqIter};
pub use node::{NodeStyle, NodeType};
pub use node_ref::NodeRef;
pub use parser::{DocumentIterator, FyParser};
pub use value_ref::ValueRef;

// Re-export error and value types
pub use error::{Error, Result};
pub use value::{Number, TaggedValue, Value};

/// Returns the version string of the underlying libfyaml C library.
pub fn get_c_version() -> Result<String> {
    log::trace!("get_c_version()");
    let cstr_ptr = unsafe { fyaml_sys::fy_library_version() };
    if cstr_ptr.is_null() {
        log::error!("Null pointer received from fy_library_version");
        return Err(Error::Ffi("fy_library_version returned null"));
    }
    log::trace!("convert to string");
    let str = unsafe { std::ffi::CStr::from_ptr(cstr_ptr) };
    log::trace!("done !");
    Ok(str.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use crate::Document;

    fn path(yaml: &str, path: &str) -> String {
        let doc = Document::parse_str(yaml).unwrap();
        let root = doc.root().unwrap();
        if path.is_empty() {
            root.emit().unwrap()
        } else {
            root.at_path(path).unwrap().emit().unwrap()
        }
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
        let result = path(
            r#"
        foo: bar
        "#,
            "",
        );
        // emit() may or may not include trailing newline
        assert!(result.trim() == "foo: bar");
    }

    #[test]
    fn test_trap() {
        assert_eq!(
            path(
                r#"
        foo: "bar: wiz"
        "#,
                "/foo"
            ),
            "\"bar: wiz\""
        );
    }
}
