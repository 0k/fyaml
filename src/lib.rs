#![doc = include_str!(concat!(env!("OUT_DIR"), "/README.md"))]

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
