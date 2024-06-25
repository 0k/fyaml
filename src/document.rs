//! YAML document parsing and manipulation.
//!
//! This module provides types for parsing YAML documents from strings or stdin,
//! and iterating over multi-document streams.

use crate::node::{FyNode, Node};
use fyaml_sys::*;
use libc::{c_void, fdopen, setvbuf, _IOLBF};
use std::fmt;
use std::os::fd::AsRawFd;
use std::ptr;
use std::rc::Rc;
use std::str::FromStr;

/// Low-level YAML parser wrapping libfyaml's `fy_parser`.
///
/// Use [`FyParser::new`] to create a parser, then call [`Parse::doc_iter`]
/// to iterate over documents.
#[repr(C)]
pub struct FyParser {
    pub(crate) parser_ptr: *mut fy_parser,
}

impl FyParser {
    /// Creates a new YAML parser with default configuration.
    ///
    /// The parser is configured with:
    /// - Buffering disabled for streaming
    /// - Quiet mode (no stderr output)
    /// - Document resolution enabled
    // TODO: provide flags as arguments for customization
    pub fn new() -> Result<Rc<Self>, String> {
        let cfg = fy_parse_cfg {
            search_path: ptr::null_mut(),
            userdata: ptr::null_mut(),
            diag: ptr::null_mut(),
            flags: FYPCF_DISABLE_BUFFERING | FYPCF_QUIET | FYPCF_RESOLVE_DOCUMENT,
        };
        let parser_ptr = unsafe { fy_parser_create(&cfg) };
        if parser_ptr.is_null() {
            return Err("Failed to create libfyaml parser".to_string());
        }
        Ok(Rc::new(FyParser { parser_ptr }))
    }

    /// Creates a parser configured to read from stdin.
    ///
    /// The stdin stream is set to line-buffered mode for interactive use.
    pub fn from_stdin() -> Result<Rc<Self>, String> {
        log::trace!("open stdin");
        let parser = FyParser::new()?;
        let stdin = std::io::stdin();
        let fd = stdin.as_raw_fd();
        // Note: We don't close the FILE* as it wraps stdin which should remain open
        let fp = unsafe {
            fdopen(fd, "r".as_ptr() as *const i8) // Convert to *mut FILE
        };
        // Set the buffering mode to line-buffered
        let setvbuf_result = unsafe { setvbuf(fp, std::ptr::null_mut(), _IOLBF, 0) };
        if setvbuf_result != 0 {
            return Err("Failed to set line-buffered mode".to_string());
        }
        let ret =
            unsafe { fy_parser_set_input_fp(parser.parser_ptr, "stdin".as_ptr() as *const i8, fp) };
        if ret != 0 {
            return Err("Failed to set input file pointer".to_string());
        }
        Ok(Rc::clone(&parser))
    }
}

/// Trait for types that can produce a document iterator.
pub trait Parse {
    /// Returns an iterator over YAML documents in the stream.
    fn doc_iter(&self) -> DocumentIterator;
}

impl Parse for Rc<FyParser> {
    fn doc_iter(&self) -> DocumentIterator {
        log::trace!("get doc iter");
        DocumentIterator {
            fy_parser: Rc::clone(self),
        }
    }
}

impl Drop for FyParser {
    fn drop(&mut self) {
        if !self.parser_ptr.is_null() {
            log::trace!("Freeing FyParser {:p}", self.parser_ptr);
            unsafe { fy_parser_destroy(self.parser_ptr) };
        }
    }
}

/// Iterator over YAML documents in a stream.
///
/// Created by calling [`Parse::doc_iter`] on a parser.
pub struct DocumentIterator {
    fy_parser: Rc<FyParser>,
}

impl Iterator for DocumentIterator {
    type Item = Document;

    fn next(&mut self) -> Option<Self::Item> {
        log::trace!("next document ?");

        let doc_ptr = unsafe { fy_parse_load_document(self.fy_parser.parser_ptr) };
        if doc_ptr.is_null() {
            return None;
        }
        log::trace!("  got next document !");
        Some(Document {
            fy_doc: Rc::new(FyDocument { doc_ptr }),
        })
    }
}

/// Low-level YAML document wrapping libfyaml's `fy_document`.
///
/// This is an internal type. Use [`Document`] for the safe public API.
pub struct FyDocument {
    pub(crate) doc_ptr: *mut fy_document,
}

impl FyDocument {
    /// Creates a new empty YAML document.
    pub fn new() -> Result<Self, String> {
        let doc_ptr = unsafe { fy_document_create(ptr::null_mut()) };
        if doc_ptr.is_null() {
            return Err("Failed to create libfyaml document".to_string());
        }
        Ok(FyDocument { doc_ptr })
    }

    /// Returns the root node of this document, if any.
    pub fn root_node(&self) -> Option<FyNode> {
        let node_ptr = unsafe { fy_document_root(self.doc_ptr) };
        if node_ptr.is_null() {
            return None;
        }
        Some(FyNode { node_ptr })
    }
}

impl Drop for FyDocument {
    fn drop(&mut self) {
        if !self.doc_ptr.is_null() {
            unsafe { fy_document_destroy(self.doc_ptr) };
        }
    }
}

impl FromStr for FyDocument {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let doc_ptr = unsafe {
            fy_document_build_from_string(ptr::null_mut(), s.as_ptr() as *const i8, s.len())
        };
        if doc_ptr.is_null() {
            return Err("Failed to parse string as YAML document".to_string());
        }
        Ok(FyDocument { doc_ptr })
    }
}

impl fmt::Display for FyDocument {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let ptr = unsafe { fy_emit_document_to_string(self.doc_ptr, 0) };
        if ptr.is_null() {
            return Ok(());
        }
        let c_str = unsafe { std::ffi::CStr::from_ptr(ptr) };
        let result = write!(f, "{}", c_str.to_string_lossy());
        unsafe {
            libc::free(ptr as *mut c_void);
        }
        result
    }
}

/// A parsed YAML document.
///
/// Use [`Document::from_str`] to parse a YAML string, or iterate over
/// documents from a parser using [`Parse::doc_iter`].
///
/// # Example
///
/// ```
/// use fyaml::document::Document;
/// use std::str::FromStr;
///
/// let doc = Document::from_str("foo: bar").unwrap();
/// let root = doc.root_node().unwrap();
/// assert!(root.is_mapping());
/// ```
pub struct Document {
    pub(crate) fy_doc: Rc<FyDocument>,
}

impl Document {
    /// Creates a new empty YAML document.
    pub fn new() -> Result<Self, String> {
        Ok(Document {
            fy_doc: Rc::new(FyDocument::new()?),
        })
    }

    /// Returns the root node of this document, if any.
    ///
    /// Returns `None` for empty documents.
    pub fn root_node(&self) -> Option<Node> {
        Some(Node {
            fy_node: Rc::new(self.fy_doc.root_node()?),
            fy_doc: Rc::clone(&self.fy_doc),
        })
    }
}

impl FromStr for Document {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Document {
            fy_doc: Rc::new(FyDocument::from_str(s)?),
        })
    }
}

impl fmt::Display for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.fy_doc)
    }
}
