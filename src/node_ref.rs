//! Zero-copy node reference type.

use crate::config;
use crate::document::Document;
use crate::error::{Error, Result};
use crate::ffi_util::take_c_string;
use crate::iter::{MapIter, SeqIter};
use crate::node::{NodeStyle, NodeType};
use fyaml_sys::*;
use libc::size_t;
use std::fmt;
use std::ptr::NonNull;
use std::slice;

/// A borrowed reference to a YAML node.
///
/// `NodeRef<'doc>` provides zero-copy access to node data. The lifetime `'doc`
/// ties the reference to its parent [`Document`], ensuring the node cannot
/// outlive the document.
///
/// # Zero-Copy Scalar Access
///
/// Scalar data is accessed directly from libfyaml's internal buffers:
///
/// ```
/// use fyaml::Document;
///
/// let doc = Document::parse_str("key: value").unwrap();
/// let root = doc.root().unwrap();
/// let value = root.at_path("/key").unwrap();
///
/// // This returns &'doc str - zero allocation!
/// let s: &str = value.scalar_str().unwrap();
/// assert_eq!(s, "value");
/// ```
///
/// # Navigation
///
/// Navigate the tree using paths or iteration:
///
/// ```
/// use fyaml::Document;
///
/// let doc = Document::parse_str("users:\n  - name: Alice\n  - name: Bob").unwrap();
/// let root = doc.root().unwrap();
///
/// // Path navigation
/// let first_user = root.at_path("/users/0/name").unwrap();
/// assert_eq!(first_user.scalar_str().unwrap(), "Alice");
///
/// // Iteration
/// let users = root.at_path("/users").unwrap();
/// for user in users.seq_iter() {
///     println!("{}", user.at_path("/name").unwrap().scalar_str().unwrap());
/// }
/// ```
///
/// # Thread Safety
///
/// `NodeRef` is `!Send` and `!Sync` because the underlying document is not thread-safe.
#[derive(Clone, Copy)]
pub struct NodeRef<'doc> {
    /// Reference to the owning document.
    doc: &'doc Document,
    /// Raw pointer to the libfyaml node.
    node_ptr: NonNull<fy_node>,
}

impl<'doc> NodeRef<'doc> {
    /// Creates a new NodeRef.
    ///
    /// # Safety (internal)
    ///
    /// - `node_ptr` must be a valid pointer to a node in `doc`
    /// - The node must remain valid for the lifetime `'doc`
    #[inline]
    pub(crate) fn new(node_ptr: NonNull<fy_node>, doc: &'doc Document) -> Self {
        NodeRef { doc, node_ptr }
    }

    /// Returns the raw node pointer.
    #[inline]
    pub(crate) fn as_ptr(&self) -> *mut fy_node {
        self.node_ptr.as_ptr()
    }

    /// Returns a reference to the parent document.
    #[inline]
    pub fn document(&self) -> &'doc Document {
        self.doc
    }

    // ==================== Type Information ====================

    /// Returns the type of this node.
    #[inline]
    pub fn kind(&self) -> NodeType {
        unsafe { NodeType::from(fy_node_get_type(self.as_ptr())) }
    }

    /// Returns `true` if this node is a scalar value.
    #[inline]
    pub fn is_scalar(&self) -> bool {
        self.kind() == NodeType::Scalar
    }

    /// Returns `true` if this node is a mapping (dictionary).
    #[inline]
    pub fn is_mapping(&self) -> bool {
        self.kind() == NodeType::Mapping
    }

    /// Returns `true` if this node is a sequence (list).
    #[inline]
    pub fn is_sequence(&self) -> bool {
        self.kind() == NodeType::Sequence
    }

    // ==================== Style Information ====================

    /// Returns the style of this node.
    ///
    /// For scalar nodes, this indicates quoting style (plain, single-quoted, etc.).
    /// For sequences/mappings, this indicates flow vs block style.
    #[inline]
    pub fn style(&self) -> NodeStyle {
        NodeStyle::from(unsafe { fy_node_get_style(self.as_ptr()) })
    }

    /// Returns `true` if this scalar was quoted (single or double quotes).
    #[inline]
    pub fn is_quoted(&self) -> bool {
        let style = unsafe { fy_node_get_style(self.as_ptr()) };
        style == FYNS_SINGLE_QUOTED || style == FYNS_DOUBLE_QUOTED
    }

    /// Returns `true` if this scalar has a non-plain style.
    ///
    /// Non-plain styles include single-quoted, double-quoted, literal (`|`),
    /// and folded (`>`). These styles should prevent type inference (the value
    /// should be treated as a string, not inferred as bool/int/null).
    #[inline]
    pub fn is_non_plain(&self) -> bool {
        let style = unsafe { fy_node_get_style(self.as_ptr()) };
        style == FYNS_SINGLE_QUOTED
            || style == FYNS_DOUBLE_QUOTED
            || style == FYNS_LITERAL
            || style == FYNS_FOLDED
    }

    // ==================== Zero-Copy Scalar Access ====================

    /// Returns the scalar value as a byte slice (zero-copy).
    ///
    /// The returned slice points directly into libfyaml's internal buffer.
    /// It is valid for the lifetime `'doc` of the document.
    ///
    /// # Errors
    ///
    /// Returns an error if this is not a scalar node.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let doc = Document::parse_str("data: hello").unwrap();
    /// let node = doc.at_path("/data").unwrap();
    /// assert_eq!(node.scalar_bytes().unwrap(), b"hello");
    /// ```
    pub fn scalar_bytes(&self) -> Result<&'doc [u8]> {
        let mut len: size_t = 0;
        let data_ptr = unsafe { fy_node_get_scalar(self.as_ptr(), &mut len) };
        if data_ptr.is_null() {
            return Err(Error::TypeMismatch {
                expected: "scalar",
                got: "non-scalar or null",
            });
        }
        // Sanity check
        if len > isize::MAX as usize {
            return Err(Error::ScalarTooLarge(len));
        }
        // SAFETY: data_ptr points into libfyaml's storage, kept alive by 'doc
        Ok(unsafe { slice::from_raw_parts(data_ptr as *const u8, len) })
    }

    /// Returns the scalar value as a string slice (zero-copy).
    ///
    /// The returned string points directly into libfyaml's internal buffer.
    /// It is valid for the lifetime `'doc` of the document.
    ///
    /// # Errors
    ///
    /// Returns an error if this is not a scalar node or if the content is not valid UTF-8.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let doc = Document::parse_str("name: Alice").unwrap();
    /// let name = doc.at_path("/name").unwrap().scalar_str().unwrap();
    /// assert_eq!(name, "Alice");
    /// ```
    pub fn scalar_str(&self) -> Result<&'doc str> {
        let bytes = self.scalar_bytes()?;
        std::str::from_utf8(bytes).map_err(Error::from)
    }

    // ==================== Zero-Copy Tag Access ====================

    /// Returns the YAML tag as a byte slice (zero-copy).
    ///
    /// Returns `Ok(None)` if the node has no explicit tag.
    pub fn tag_bytes(&self) -> Result<Option<&'doc [u8]>> {
        let mut len: size_t = 0;
        let tag_ptr = unsafe { fy_node_get_tag(self.as_ptr(), &mut len) };
        if tag_ptr.is_null() {
            return Ok(None);
        }
        if len > isize::MAX as usize {
            return Err(Error::ScalarTooLarge(len));
        }
        Ok(Some(unsafe {
            slice::from_raw_parts(tag_ptr as *const u8, len)
        }))
    }

    /// Returns the YAML tag as a string slice (zero-copy).
    ///
    /// Returns `Ok(None)` if the node has no explicit tag.
    pub fn tag_str(&self) -> Result<Option<&'doc str>> {
        match self.tag_bytes()? {
            Some(bytes) => std::str::from_utf8(bytes).map(Some).map_err(Error::from),
            None => Ok(None),
        }
    }

    // ==================== Navigation ====================

    /// Navigates to a child node by path.
    ///
    /// Path format uses `/` as separator:
    /// - `/foo` - access key "foo" in a mapping
    /// - `/0` - access index 0 in a sequence
    /// - `/foo/bar/0` - nested access
    /// - `` (empty) - returns self
    ///
    /// Returns `None` if the path doesn't exist.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let doc = Document::parse_str("a:\n  b:\n    c: deep").unwrap();
    /// let deep = doc.root().unwrap().at_path("/a/b/c").unwrap();
    /// assert_eq!(deep.scalar_str().unwrap(), "deep");
    /// ```
    pub fn at_path(&self, path: &str) -> Option<NodeRef<'doc>> {
        let node_ptr =
            unsafe { fy_node_by_path(self.as_ptr(), path.as_ptr() as *const i8, path.len(), 0) };
        NonNull::new(node_ptr).map(|nn| NodeRef::new(nn, self.doc))
    }

    // ==================== Length Operations ====================

    /// Returns the number of items in a sequence node.
    ///
    /// # Errors
    ///
    /// Returns an error if this is not a sequence.
    pub fn seq_len(&self) -> Result<usize> {
        let len: i32 = unsafe { fy_node_sequence_item_count(self.as_ptr()) };
        if len < 0 {
            return Err(Error::TypeMismatch {
                expected: "sequence",
                got: "non-sequence",
            });
        }
        Ok(len as usize)
    }

    /// Returns the number of key-value pairs in a mapping node.
    ///
    /// # Errors
    ///
    /// Returns an error if this is not a mapping.
    pub fn map_len(&self) -> Result<usize> {
        let len: i32 = unsafe { fy_node_mapping_item_count(self.as_ptr()) };
        if len < 0 {
            return Err(Error::TypeMismatch {
                expected: "mapping",
                got: "non-mapping",
            });
        }
        Ok(len as usize)
    }

    // ==================== Sequence Access ====================

    /// Gets a sequence item by index.
    ///
    /// Returns `None` if the index is out of bounds or this is not a sequence.
    /// Negative indices count from the end (-1 is the last element).
    pub fn seq_get(&self, index: i32) -> Option<NodeRef<'doc>> {
        if !self.is_sequence() {
            return None;
        }
        let node_ptr = unsafe { fy_node_sequence_get_by_index(self.as_ptr(), index) };
        NonNull::new(node_ptr).map(|nn| NodeRef::new(nn, self.doc))
    }

    /// Returns an iterator over items in a sequence node.
    ///
    /// If this is not a sequence, the iterator will be empty.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let doc = Document::parse_str("- a\n- b\n- c").unwrap();
    /// let root = doc.root().unwrap();
    ///
    /// let items: Vec<&str> = root.seq_iter()
    ///     .map(|n| n.scalar_str().unwrap())
    ///     .collect();
    /// assert_eq!(items, vec!["a", "b", "c"]);
    /// ```
    #[inline]
    pub fn seq_iter(&self) -> SeqIter<'doc> {
        SeqIter::new(*self)
    }

    // ==================== Mapping Access ====================

    /// Looks up a value in this mapping by string key.
    ///
    /// Returns `None` if the key is not found or this is not a mapping.
    pub fn map_get(&self, key: &str) -> Option<NodeRef<'doc>> {
        if !self.is_mapping() {
            return None;
        }
        let node_ptr = unsafe {
            fy_node_mapping_lookup_by_string(self.as_ptr(), key.as_ptr() as *const i8, key.len())
        };
        NonNull::new(node_ptr).map(|nn| NodeRef::new(nn, self.doc))
    }

    /// Returns an iterator over key-value pairs in a mapping node.
    ///
    /// If this is not a mapping, the iterator will be empty.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let doc = Document::parse_str("a: 1\nb: 2").unwrap();
    /// let root = doc.root().unwrap();
    ///
    /// for (key, value) in root.map_iter() {
    ///     println!("{}: {}", key.scalar_str().unwrap(), value.scalar_str().unwrap());
    /// }
    /// ```
    #[inline]
    pub fn map_iter(&self) -> MapIter<'doc> {
        MapIter::new(*self)
    }

    // ==================== Emission ====================

    /// Emits this node as a YAML string.
    ///
    /// For scalar nodes, this includes any quoting.
    /// For complex nodes, this returns properly formatted YAML.
    ///
    /// This always allocates a new string. If the emitted content contains
    /// invalid UTF-8 (rare), invalid bytes are replaced with U+FFFD.
    pub fn emit(&self) -> Result<String> {
        let ptr = unsafe { fy_emit_node_to_string(self.as_ptr(), config::emit_flags()) };
        if ptr.is_null() {
            return Err(Error::Ffi("fy_emit_node_to_string returned null"));
        }
        // SAFETY: ptr is a valid malloc'd C string from libfyaml
        Ok(unsafe { take_c_string(ptr) })
    }
}

impl fmt::Display for NodeRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.emit() {
            Ok(s) => write!(f, "{}", s),
            Err(_) => write!(f, "<NodeRef emit error>"),
        }
    }
}

impl fmt::Debug for NodeRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeRef")
            .field("kind", &self.kind())
            .field("style", &self.style())
            .field("ptr", &self.node_ptr)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scalar_str() {
        let doc = Document::parse_str("key: value").unwrap();
        let node = doc.at_path("/key").unwrap();
        assert_eq!(node.scalar_str().unwrap(), "value");
    }

    #[test]
    fn test_is_quoted() {
        let doc = Document::parse_str("plain: value\nquoted: 'value'").unwrap();
        let plain = doc.at_path("/plain").unwrap();
        let quoted = doc.at_path("/quoted").unwrap();
        assert!(!plain.is_quoted());
        assert!(quoted.is_quoted());
    }

    #[test]
    fn test_navigation() {
        let doc = Document::parse_str("a:\n  b:\n    c: deep").unwrap();
        let node = doc.root().unwrap().at_path("/a/b/c").unwrap();
        assert_eq!(node.scalar_str().unwrap(), "deep");
    }

    #[test]
    fn test_seq_len() {
        let doc = Document::parse_str("[1, 2, 3]").unwrap();
        assert_eq!(doc.root().unwrap().seq_len().unwrap(), 3);
    }

    #[test]
    fn test_map_len() {
        let doc = Document::parse_str("a: 1\nb: 2").unwrap();
        assert_eq!(doc.root().unwrap().map_len().unwrap(), 2);
    }
}
