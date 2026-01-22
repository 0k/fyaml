//! Document type that owns parsed YAML data.

use crate::config;
use crate::editor::Editor;
use crate::error::{Error, Result};
use crate::ffi_util::{malloc_copy, take_c_string};
use crate::node_ref::NodeRef;
use crate::value_ref::ValueRef;
use fyaml_sys::*;
use libc::c_void;
use std::fmt;
use std::marker::PhantomData;
use std::ptr::{self, NonNull};
use std::str::FromStr;

// =============================================================================
// Input Ownership
// =============================================================================

// Forward declaration for parser inner type
pub(crate) use crate::parser::ParserInner;

/// How the input buffer is owned/kept alive.
///
/// This enum exists purely for memory safety - it keeps the input buffer
/// alive as long as the document exists. The document's nodes may reference
/// memory owned by this input source.
#[allow(dead_code)]
pub enum InputOwnership {
    /// libfyaml owns the input via malloc'd buffer.
    /// Used for string parsing with `fy_document_build_from_malloc_string`.
    LibfyamlOwned,
    /// Document owns the input string directly.
    /// Used with `fy_document_build_from_string` for zero-extra-copy parsing.
    OwnedString(String),
    /// Document owns the input bytes directly.
    /// Used with `fy_document_build_from_string` for zero-extra-copy parsing of raw bytes.
    OwnedBytes(Vec<u8>),
    /// Parser owns the input buffer (stream parsing).
    /// The parser must outlive the document to prevent use-after-free.
    Parser(std::rc::Rc<ParserInner>),
    /// Empty document or constructed document (no external input).
    None,
}

// =============================================================================
// Document
// =============================================================================

/// A parsed YAML document with exclusive ownership.
///
/// `Document` owns the underlying libfyaml document and all its nodes.
/// Use [`root()`](Self::root) to get a [`NodeRef`] for reading, or
/// [`edit()`](Self::edit) to get an [`Editor`] for modifications.
///
/// # Memory Safety
///
/// - `NodeRef<'doc>` borrows `&Document`, preventing use-after-free
/// - `Editor<'doc>` borrows `&mut Document`, preventing concurrent access
/// - All safety is enforced at compile time via Rust's borrow checker
///
/// # Thread Safety
///
/// `Document` is `!Send` and `!Sync` because libfyaml is not thread-safe.
///
/// # Example
///
/// ```
/// use fyaml::Document;
///
/// let doc = Document::parse_str("name: Alice").unwrap();
/// let root = doc.root().unwrap();
/// assert_eq!(root.at_path("/name").unwrap().scalar_str().unwrap(), "Alice");
/// ```
pub struct Document {
    /// Raw pointer to libfyaml document. Owned by this struct.
    pub(crate) doc_ptr: NonNull<fy_document>,
    /// Keeps input buffer alive if needed.
    #[allow(dead_code)]
    input: InputOwnership,
    /// Marker to ensure !Send + !Sync
    _marker: PhantomData<*mut ()>,
}

impl Document {
    /// Creates a Document from a raw libfyaml pointer.
    ///
    /// # Safety
    ///
    /// - `doc_ptr` must be a valid pointer to a libfyaml document
    /// - The caller transfers ownership of the document to this struct
    /// - The document will be destroyed when this struct is dropped
    #[inline]
    pub(crate) fn from_raw_ptr(doc_ptr: NonNull<fy_document>, input: InputOwnership) -> Self {
        Document {
            doc_ptr,
            input,
            _marker: PhantomData,
        }
    }

    /// Creates a new empty YAML document.
    ///
    /// Use [`edit()`](Self::edit) to add content to the document.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let mut doc = Document::new().unwrap();
    /// // Use editor to add content...
    /// ```
    pub fn new() -> Result<Self> {
        let doc_ptr = unsafe { fy_document_create(ptr::null_mut()) };
        let nn = NonNull::new(doc_ptr).ok_or(Error::Ffi("fy_document_create returned null"))?;
        Ok(Document {
            doc_ptr: nn,
            input: InputOwnership::None,
            _marker: PhantomData,
        })
    }

    /// Reads and parses a single YAML document from stdin.
    ///
    /// This is a convenience method for reading one document from standard input.
    /// For multi-document streams, use [`FyParser::from_stdin()`](crate::FyParser::from_stdin) instead.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use fyaml::Document;
    ///
    /// // Read a YAML document from stdin
    /// let doc = Document::from_stdin()?;
    /// println!("Root type: {:?}", doc.root().map(|r| r.kind()));
    /// ```
    pub fn from_stdin() -> Result<Self> {
        use crate::FyParser;
        let parser = FyParser::from_stdin()?;
        parser
            .doc_iter()
            .next()
            .ok_or(Error::Parse("no document in stdin stream"))?
    }

    /// Parses a YAML string into a Document.
    ///
    /// # Memory Safety
    ///
    /// The input string is copied to a malloc'd buffer that libfyaml takes
    /// ownership of. This ensures zero-copy node access is safe even after
    /// the original string is dropped.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let doc = Document::parse_str("foo: bar").unwrap();
    /// let root = doc.root().unwrap();
    /// assert!(root.is_mapping());
    /// ```
    pub fn parse_str(s: &str) -> Result<Self> {
        if s.is_empty() {
            return Err(Error::Parse("empty input"));
        }

        // Allocate buffer and copy input - libfyaml takes ownership
        let buf = unsafe { malloc_copy(s.as_bytes())? };

        // libfyaml takes ownership of buf on success
        let cfg = config::document_parse_cfg();
        let doc_ptr = unsafe { fy_document_build_from_malloc_string(&cfg, buf, s.len()) };
        if doc_ptr.is_null() {
            // On failure, libfyaml does NOT free the buffer
            unsafe { libc::free(buf as *mut c_void) };
            return Err(Error::Parse("fy_document_build_from_malloc_string failed"));
        }

        Ok(Document {
            doc_ptr: NonNull::new(doc_ptr).unwrap(),
            input: InputOwnership::LibfyamlOwned,
            _marker: PhantomData,
        })
    }

    /// Parses an owned YAML string into a Document (zero extra copy).
    ///
    /// Unlike [`parse_str`](Self::parse_str), this method takes ownership of the
    /// String and uses it directly as the input buffer, avoiding an extra memory
    /// copy. The String is kept alive by the Document.
    ///
    /// Use this when you already own the input and want to minimize allocations.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let yaml_string = String::from("name: Alice\nage: 30");
    /// let doc = Document::from_string(yaml_string).unwrap();
    /// let root = doc.root().unwrap();
    /// assert_eq!(root.at_path("/name").unwrap().scalar_str().unwrap(), "Alice");
    /// ```
    pub fn from_string(s: String) -> Result<Self> {
        if s.is_empty() {
            return Err(Error::Parse("empty input"));
        }

        let cfg = config::document_parse_cfg();
        // SAFETY: fy_document_build_from_string borrows the input - the String must
        // remain valid for the document's lifetime. We keep it in InputOwnership::OwnedString.
        let doc_ptr =
            unsafe { fy_document_build_from_string(&cfg, s.as_ptr() as *const i8, s.len()) };
        if doc_ptr.is_null() {
            return Err(Error::Parse("fy_document_build_from_string failed"));
        }

        Ok(Document {
            doc_ptr: NonNull::new(doc_ptr).unwrap(),
            input: InputOwnership::OwnedString(s),
            _marker: PhantomData,
        })
    }

    /// Parses owned bytes into a Document (zero extra copy).
    ///
    /// This is similar to [`from_string`](Self::from_string) but accepts raw bytes,
    /// which is useful when:
    /// - Reading YAML from files or network without UTF-8 validation overhead
    /// - Working with YAML that may contain binary data in tags
    /// - Avoiding the cost of UTF-8 validation when you know the input is valid
    ///
    /// The bytes are kept alive by the Document, enabling zero-copy access
    /// to scalar data via [`NodeRef::scalar_bytes`](crate::NodeRef::scalar_bytes).
    ///
    /// # Note
    ///
    /// YAML is specified as UTF-8, but libfyaml will attempt to parse the input
    /// regardless. Invalid UTF-8 sequences may cause parsing errors or be
    /// preserved as raw bytes accessible via `scalar_bytes()`.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// // Read bytes from a file (hypothetically)
    /// let yaml_bytes = b"name: Alice\nage: 30".to_vec();
    /// let doc = Document::from_bytes(yaml_bytes).unwrap();
    /// let root = doc.root().unwrap();
    /// assert_eq!(root.at_path("/name").unwrap().scalar_str().unwrap(), "Alice");
    /// ```
    pub fn from_bytes(bytes: Vec<u8>) -> Result<Self> {
        if bytes.is_empty() {
            return Err(Error::Parse("empty input"));
        }

        let cfg = config::document_parse_cfg();
        // SAFETY: fy_document_build_from_string borrows the input - the Vec must
        // remain valid for the document's lifetime. We keep it in InputOwnership::OwnedBytes.
        let doc_ptr = unsafe {
            fy_document_build_from_string(&cfg, bytes.as_ptr() as *const i8, bytes.len())
        };
        if doc_ptr.is_null() {
            return Err(Error::Parse("fy_document_build_from_string failed"));
        }

        Ok(Document {
            doc_ptr: NonNull::new(doc_ptr).unwrap(),
            input: InputOwnership::OwnedBytes(bytes),
            _marker: PhantomData,
        })
    }

    /// Returns the root node of this document, if any.
    ///
    /// Returns `None` for empty documents.
    ///
    /// The returned [`NodeRef`] borrows this document, preventing any
    /// mutations while the reference exists.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let doc = Document::parse_str("key: value").unwrap();
    /// if let Some(root) = doc.root() {
    ///     println!("Root type: {:?}", root.kind());
    /// }
    /// ```
    #[inline]
    pub fn root(&self) -> Option<NodeRef<'_>> {
        let node_ptr = unsafe { fy_document_root(self.doc_ptr.as_ptr()) };
        NonNull::new(node_ptr).map(|nn| NodeRef::new(nn, self))
    }

    /// Navigates to a node by path from the document root.
    ///
    /// This is a convenience method equivalent to `doc.root()?.at_path(path)`.
    ///
    /// Path format uses `/` as separator:
    /// - `/key` - access a mapping key
    /// - `/0` - access a sequence index
    /// - `/parent/child/0` - nested access
    ///
    /// Returns `None` if the document is empty or path doesn't exist.
    #[inline]
    pub fn at_path(&self, path: &str) -> Option<NodeRef<'_>> {
        self.root()?.at_path(path)
    }

    /// Returns the root node as a typed [`ValueRef`].
    ///
    /// `ValueRef` provides typed accessors (`as_str()`, `as_i64()`, `as_bool()`, etc.)
    /// that interpret YAML scalars on demand without allocation.
    ///
    /// Returns `None` for empty documents.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let doc = Document::parse_str("name: Alice\nage: 30\nactive: true").unwrap();
    /// let root = doc.root_value().unwrap();
    ///
    /// // Zero-copy typed access
    /// assert_eq!(root.get("name").unwrap().as_str(), Some("Alice"));
    /// assert_eq!(root.get("age").unwrap().as_i64(), Some(30));
    /// assert_eq!(root.get("active").unwrap().as_bool(), Some(true));
    /// ```
    #[inline]
    pub fn root_value(&self) -> Option<ValueRef<'_>> {
        self.root().map(ValueRef::new)
    }

    /// Returns an exclusive editor for modifying this document.
    ///
    /// While the editor exists, no [`NodeRef`] can be held (enforced by borrow checker).
    /// This ensures mutations cannot invalidate existing node references.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let mut doc = Document::parse_str("name: Alice").unwrap();
    ///
    /// // Mutation phase
    /// {
    ///     let mut ed = doc.edit();
    ///     ed.set_yaml_at("/name", "'Bob'").unwrap();
    /// }
    ///
    /// // Read phase
    /// let root = doc.root().unwrap();
    /// assert_eq!(root.at_path("/name").unwrap().scalar_str().unwrap(), "Bob");
    /// ```
    #[inline]
    pub fn edit(&mut self) -> Editor<'_> {
        Editor::new(self)
    }

    /// Emits the document as a YAML string.
    ///
    /// This preserves the original formatting style and comments.
    /// This always allocates a new string.
    ///
    /// # Note
    ///
    /// If the emitted YAML contains invalid UTF-8 (rare), invalid bytes are
    /// replaced with the Unicode replacement character (U+FFFD). YAML is
    /// expected to be valid UTF-8 per the specification.
    pub fn emit(&self) -> Result<String> {
        let ptr =
            unsafe { fy_emit_document_to_string(self.doc_ptr.as_ptr(), config::emit_flags()) };
        if ptr.is_null() {
            return Err(Error::Ffi("fy_emit_document_to_string returned null"));
        }
        // SAFETY: ptr is a valid malloc'd C string from libfyaml
        Ok(unsafe { take_c_string(ptr) })
    }

    /// Returns the raw document pointer.
    ///
    /// # Safety
    ///
    /// The pointer is valid only while this Document exists.
    #[inline]
    pub(crate) fn as_ptr(&self) -> *mut fy_document {
        self.doc_ptr.as_ptr()
    }
}

impl Drop for Document {
    fn drop(&mut self) {
        log::trace!("Dropping Document {:p}", self.doc_ptr.as_ptr());
        // Documents created by fy_parse_load_document() must be destroyed with
        // fy_parse_document_destroy(), while documents created by fy_document_create()
        // or fy_document_build_from_* must be destroyed with fy_document_destroy().
        match &self.input {
            InputOwnership::Parser(parser_inner) => {
                // Stream-parsed document: use fy_parse_document_destroy
                unsafe { fy_parse_document_destroy(parser_inner.as_ptr(), self.doc_ptr.as_ptr()) };
            }
            _ => {
                // All other documents: use fy_document_destroy
                unsafe { fy_document_destroy(self.doc_ptr.as_ptr()) };
            }
        }
    }
}

impl Default for Document {
    /// Creates an empty document.
    ///
    /// # Panics
    ///
    /// Panics if libfyaml fails to allocate the document (extremely rare,
    /// only on out-of-memory conditions).
    fn default() -> Self {
        Self::new().expect("Failed to create default document")
    }
}

impl FromStr for Document {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        Self::parse_str(s)
    }
}

impl fmt::Display for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.emit() {
            Ok(s) => write!(f, "{}", s),
            Err(_) => write!(f, "<Document emit error>"),
        }
    }
}

impl fmt::Debug for Document {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Document")
            .field("ptr", &self.doc_ptr)
            .finish()
    }
}

// Document is !Send and !Sync due to PhantomData<*mut ()>.
// This is intentional - libfyaml is not thread-safe.

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple() {
        let doc = Document::parse_str("foo: bar").unwrap();
        assert!(doc.root().is_some());
    }

    #[test]
    fn test_parse_empty_fails() {
        let result = Document::parse_str("");
        assert!(result.is_err());
    }

    #[test]
    fn test_new_empty_document() {
        let doc = Document::new().unwrap();
        assert!(doc.root().is_none());
    }

    #[test]
    fn test_at_path() {
        let doc = Document::parse_str("foo:\n  bar: baz").unwrap();
        let node = doc.at_path("/foo/bar").unwrap();
        assert_eq!(node.scalar_str().unwrap(), "baz");
    }

    #[test]
    fn test_emit() {
        let doc = Document::parse_str("foo: bar").unwrap();
        let yaml = doc.emit().unwrap();
        assert!(yaml.contains("foo"));
        assert!(yaml.contains("bar"));
    }
}
