//! Exclusive mutation API for documents.

use crate::document::Document;
use crate::error::{Error, Result};
use crate::ffi_util::malloc_copy;
use crate::node_ref::NodeRef;
use fyaml_sys::*;

use std::ptr::NonNull;

// =============================================================================
// RawNodeHandle
// =============================================================================

/// An opaque handle to a freshly-built node not yet in the document tree.
///
/// This handle represents a node that has been created but not yet inserted.
/// It can only be used with the [`Editor`] that created it.
///
/// # RAII Safety
///
/// If the handle is dropped without being inserted (via `set_root`, `seq_append_at`, etc.),
/// the node will be automatically freed to prevent memory leaks. Once inserted into
/// the document tree, the document takes ownership and the node will be freed when
/// the document is destroyed.
///
/// # Example
///
/// ```
/// use fyaml::Document;
///
/// let mut doc = Document::new().unwrap();
/// {
///     let mut ed = doc.edit();
///     let node = ed.build_from_yaml("key: value").unwrap();
///     // If we don't call ed.set_root(node), the node is freed when `node` is dropped
///     ed.set_root(node).unwrap();
/// }
/// ```
pub struct RawNodeHandle {
    pub(crate) node_ptr: NonNull<fy_node>,
    /// Whether this node has been inserted into the document tree
    inserted: bool,
}

impl RawNodeHandle {
    /// Returns the raw node pointer.
    #[inline]
    pub(crate) fn as_ptr(&self) -> *mut fy_node {
        self.node_ptr.as_ptr()
    }

    /// Marks this handle as consumed (inserted into the document tree).
    ///
    /// After calling this, Drop will not free the node.
    #[inline]
    pub(crate) fn mark_inserted(&mut self) {
        self.inserted = true;
    }
}

impl Drop for RawNodeHandle {
    fn drop(&mut self) {
        if !self.inserted {
            // Node was never inserted, so we must free it to avoid memory leaks
            log::trace!(
                "Freeing orphaned RawNodeHandle {:p}",
                self.node_ptr.as_ptr()
            );
            unsafe { fy_node_free(self.node_ptr.as_ptr()) };
        }
    }
}

// =============================================================================
// Path Helpers
// =============================================================================

/// Splits a path into (parent_path, key).
///
/// Examples:
/// - "/foo/bar" -> ("/foo", "bar")
/// - "/key" -> ("", "key")
/// - "key" -> ("", "key")
#[inline]
fn split_path(path: &str) -> (&str, &str) {
    match path.rfind('/') {
        Some(0) => ("", &path[1..]), // "/key" -> parent is root, key is "key"
        Some(i) => (&path[..i], &path[i + 1..]),
        None => ("", path),
    }
}

// =============================================================================
// Editor
// =============================================================================

/// Exclusive editor for modifying a document.
///
/// `Editor<'doc>` borrows `&mut Document`, ensuring no [`NodeRef`] can exist
/// while mutations are in progress. This prevents use-after-free at compile time.
///
/// # Primary API: Path-Based Mutations
///
/// The recommended way to modify documents is through path-based operations:
///
/// ```
/// use fyaml::Document;
///
/// let mut doc = Document::parse_str("name: Alice").unwrap();
/// {
///     let mut ed = doc.edit();
///     ed.set_yaml_at("/name", "'Bob'").unwrap();
///     ed.set_yaml_at("/age", "25").unwrap();
/// }
/// let root = doc.root().unwrap();
/// assert_eq!(root.at_path("/name").unwrap().scalar_str().unwrap(), "Bob");
/// ```
///
/// # Node Building API
///
/// For more complex modifications, you can build nodes and insert them:
///
/// ```
/// use fyaml::Document;
///
/// let mut doc = Document::new().unwrap();
/// {
///     let mut ed = doc.edit();
///     let root = ed.build_from_yaml("name: Alice\nage: 30").unwrap();
///     ed.set_root(root).unwrap();
/// }
/// ```
pub struct Editor<'doc> {
    doc: &'doc mut Document,
}

impl<'doc> Editor<'doc> {
    /// Creates a new editor for the document.
    #[inline]
    pub(crate) fn new(doc: &'doc mut Document) -> Self {
        Editor { doc }
    }

    /// Returns the raw document pointer.
    #[inline]
    fn doc_ptr(&self) -> *mut fy_document {
        self.doc.as_ptr()
    }

    // ==================== Read Access During Edit ====================

    /// Returns the root node for reading during the edit session.
    ///
    /// Note: The returned `NodeRef` has a shorter lifetime than `'doc` - it
    /// borrows from `&self`, so it cannot outlive this editor call.
    #[inline]
    pub fn root(&self) -> Option<NodeRef<'_>> {
        let node_ptr = unsafe { fy_document_root(self.doc_ptr()) };
        // Create a NodeRef that borrows from self.doc via proper reborrow.
        // The borrow checker ensures no mutation while this NodeRef exists.
        NonNull::new(node_ptr).map(|nn| NodeRef::new(nn, &*self.doc))
    }

    /// Navigates to a node by path for reading.
    #[inline]
    pub fn at_path(&self, path: &str) -> Option<NodeRef<'_>> {
        self.root()?.at_path(path)
    }

    // ==================== Internal Helpers ====================

    /// Resolves a parent path to a node pointer.
    ///
    /// If `parent_path` is empty, returns the document root.
    fn resolve_parent(&self, parent_path: &str) -> Result<*mut fy_node> {
        if parent_path.is_empty() {
            let root_ptr = unsafe { fy_document_root(self.doc_ptr()) };
            if root_ptr.is_null() {
                return Err(Error::Ffi("document has no root"));
            }
            Ok(root_ptr)
        } else {
            let root_ptr = unsafe { fy_document_root(self.doc_ptr()) };
            if root_ptr.is_null() {
                return Err(Error::Ffi("document has no root"));
            }
            let parent_ptr = unsafe {
                fy_node_by_path(
                    root_ptr,
                    parent_path.as_ptr() as *const i8,
                    parent_path.len(),
                    0,
                )
            };
            if parent_ptr.is_null() {
                return Err(Error::Ffi("parent path not found"));
            }
            Ok(parent_ptr)
        }
    }

    // ==================== Path-Based Mutations ====================

    /// Sets a value at the given path from a YAML snippet.
    ///
    /// If the path exists, the value is replaced.
    /// If the path doesn't exist, it will be created (for simple cases).
    ///
    /// The YAML snippet is parsed and its formatting (including quotes) is preserved.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let mut doc = Document::parse_str("name: Alice").unwrap();
    /// {
    ///     let mut ed = doc.edit();
    ///     // Preserve single quotes
    ///     ed.set_yaml_at("/name", "'Bob'").unwrap();
    /// }
    /// let output = doc.emit().unwrap();
    /// assert!(output.contains("'Bob'"));
    /// ```
    pub fn set_yaml_at(&mut self, path: &str, yaml: &str) -> Result<()> {
        // Build the new node
        let mut new_node = self.build_from_yaml(yaml)?;

        // Find the parent path and key
        if path.is_empty() || path == "/" {
            // Setting the root
            return self.set_root(new_node);
        }

        // For paths like "/foo/bar", we need to:
        // 1. Navigate to parent ("/foo")
        // 2. Set the key ("bar") to the new value

        let (parent_path, key) = split_path(path);

        // Get or navigate to parent
        let parent_ptr = self.resolve_parent(parent_path)?;

        // Check if it's a mapping
        let parent_type = unsafe { fy_node_get_type(parent_ptr) };
        if parent_type != FYNT_MAPPING {
            return Err(Error::TypeMismatch {
                expected: "mapping",
                got: "non-mapping parent",
            });
        }

        // Look up existing pair
        let pair_ptr = unsafe {
            fy_node_mapping_lookup_pair_by_string(parent_ptr, key.as_ptr() as *const i8, key.len())
        };

        if !pair_ptr.is_null() {
            // Update existing pair's value
            let ret = unsafe { fy_node_pair_set_value(pair_ptr, new_node.as_ptr()) };
            if ret != 0 {
                return Err(Error::Ffi("fy_node_pair_set_value failed"));
            }
        } else {
            // Create new key and append
            let key_ptr = unsafe {
                fy_node_create_scalar_copy(self.doc_ptr(), key.as_ptr() as *const i8, key.len())
            };
            if key_ptr.is_null() {
                return Err(Error::Ffi("fy_node_create_scalar_copy failed"));
            }
            let ret = unsafe { fy_node_mapping_append(parent_ptr, key_ptr, new_node.as_ptr()) };
            if ret != 0 {
                unsafe { fy_node_free(key_ptr) };
                return Err(Error::Ffi("fy_node_mapping_append failed"));
            }
        }

        // Mark as inserted so Drop doesn't free it
        new_node.mark_inserted();
        Ok(())
    }

    /// Deletes the node at the given path.
    ///
    /// Returns `Ok(true)` if the node was deleted, `Ok(false)` if the path didn't exist.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let mut doc = Document::parse_str("name: Alice\nage: 30").unwrap();
    /// {
    ///     let mut ed = doc.edit();
    ///     ed.delete_at("/age").unwrap();
    /// }
    /// assert!(doc.at_path("/age").is_none());
    /// ```
    pub fn delete_at(&mut self, path: &str) -> Result<bool> {
        if path.is_empty() || path == "/" {
            // Can't delete root this way
            return Err(Error::Ffi("cannot delete root via delete_at"));
        }

        // Find parent and key using helper
        let (parent_path, key) = split_path(path);

        let parent_ptr = match self.resolve_parent(parent_path) {
            Ok(ptr) => ptr,
            Err(_) => return Ok(false), // Parent not found = nothing to delete
        };

        let parent_type = unsafe { fy_node_get_type(parent_ptr) };

        if parent_type == FYNT_MAPPING {
            // Remove by key string
            let pair_ptr = unsafe {
                fy_node_mapping_lookup_pair_by_string(
                    parent_ptr,
                    key.as_ptr() as *const i8,
                    key.len(),
                )
            };
            if pair_ptr.is_null() {
                return Ok(false);
            }
            let key_ptr = unsafe { fy_node_pair_key(pair_ptr) };
            if key_ptr.is_null() {
                return Ok(false);
            }
            let removed = unsafe { fy_node_mapping_remove_by_key(parent_ptr, key_ptr) };
            if removed.is_null() {
                return Ok(false);
            }
            // Free the detached node to avoid memory leak
            unsafe { fy_node_free(removed) };
            Ok(true)
        } else if parent_type == FYNT_SEQUENCE {
            // Try to parse key as index
            let index: i32 = key
                .parse()
                .map_err(|_| Error::Ffi("invalid sequence index"))?;
            let item_ptr = unsafe { fy_node_sequence_get_by_index(parent_ptr, index) };
            if item_ptr.is_null() {
                return Ok(false);
            }
            let removed = unsafe { fy_node_sequence_remove(parent_ptr, item_ptr) };
            if removed.is_null() {
                return Ok(false);
            }
            // Free the detached node to avoid memory leak
            unsafe { fy_node_free(removed) };
            Ok(true)
        } else {
            Err(Error::TypeMismatch {
                expected: "mapping or sequence",
                got: "scalar",
            })
        }
    }

    // ==================== Node Building ====================

    /// Builds a node from a YAML snippet.
    ///
    /// The node is created but not inserted into the document tree.
    /// Use [`set_root`](Self::set_root) or other methods to insert it.
    ///
    /// Original formatting (including quotes) is preserved.
    pub fn build_from_yaml(&mut self, yaml: &str) -> Result<RawNodeHandle> {
        let buffer = unsafe { malloc_copy(yaml.as_bytes())? };
        let node_ptr =
            unsafe { fy_node_build_from_malloc_string(self.doc_ptr(), buffer, yaml.len()) };
        if node_ptr.is_null() {
            // Note: fy_node_build_from_malloc_string creates an internal parser that takes
            // ownership of the buffer via fy_parser_set_malloc_string. Once registered,
            // the buffer is freed by fy_parse_cleanup when the internal parser is destroyed,
            // regardless of whether parsing succeeded or failed.
            //
            // The docs for fy_parser_set_malloc_string say "In case of an error the string
            // is not freed" - but this refers to errors in the registration call itself,
            // NOT to parse errors later. In practice, registration rarely fails (only on
            // allocation errors), so for parse failures the buffer is already registered
            // and WILL be freed by libfyaml.
            //
            // VERIFIED: Freeing here causes double-free (detected in tests).
            return Err(Error::Parse("fy_node_build_from_malloc_string failed"));
        }
        // On success, libfyaml takes ownership of buffer (freed when document is destroyed)
        Ok(RawNodeHandle {
            node_ptr: NonNull::new(node_ptr).unwrap(),
            inserted: false,
        })
    }

    /// Builds a plain scalar node.
    ///
    /// The scalar style is automatically determined based on content.
    /// Use [`build_from_yaml`](Self::build_from_yaml) for explicit quoting.
    pub fn build_scalar(&mut self, value: &str) -> Result<RawNodeHandle> {
        let node_ptr = unsafe {
            fy_node_create_scalar_copy(self.doc_ptr(), value.as_ptr() as *const i8, value.len())
        };
        let nn = NonNull::new(node_ptr).ok_or(Error::Ffi("fy_node_create_scalar_copy failed"))?;
        Ok(RawNodeHandle {
            node_ptr: nn,

            inserted: false,
        })
    }

    /// Builds an empty sequence node.
    pub fn build_sequence(&mut self) -> Result<RawNodeHandle> {
        let node_ptr = unsafe { fy_node_create_sequence(self.doc_ptr()) };
        let nn = NonNull::new(node_ptr).ok_or(Error::Ffi("fy_node_create_sequence failed"))?;
        Ok(RawNodeHandle {
            node_ptr: nn,

            inserted: false,
        })
    }

    /// Builds an empty mapping node.
    pub fn build_mapping(&mut self) -> Result<RawNodeHandle> {
        let node_ptr = unsafe { fy_node_create_mapping(self.doc_ptr()) };
        let nn = NonNull::new(node_ptr).ok_or(Error::Ffi("fy_node_create_mapping failed"))?;
        Ok(RawNodeHandle {
            node_ptr: nn,

            inserted: false,
        })
    }

    /// Sets the document root to the given node.
    ///
    /// The node handle is consumed and the document takes ownership.
    ///
    /// # Warning
    ///
    /// If the document already has a root, it will be replaced and freed.
    pub fn set_root(&mut self, mut node: RawNodeHandle) -> Result<()> {
        let ret = unsafe { fy_document_set_root(self.doc_ptr(), node.as_ptr()) };
        if ret != 0 {
            return Err(Error::Ffi("fy_document_set_root failed"));
        }
        // Mark as inserted so Drop doesn't free it
        node.mark_inserted();
        Ok(())
    }

    // ==================== Cross-Document Operations ====================

    /// Copies a node from another document (or this document) into this document.
    ///
    /// Returns a handle to the copied node that can be inserted.
    pub fn copy_node(&mut self, source: NodeRef<'_>) -> Result<RawNodeHandle> {
        let node_ptr = unsafe { fy_node_copy(self.doc_ptr(), source.as_ptr()) };
        let nn = NonNull::new(node_ptr).ok_or(Error::Ffi("fy_node_copy failed"))?;
        Ok(RawNodeHandle {
            node_ptr: nn,

            inserted: false,
        })
    }

    // ==================== Low-Level Sequence Operations ====================

    /// Appends a node to a sequence at the given path.
    ///
    /// The node handle is consumed and the document takes ownership.
    pub fn seq_append_at(&mut self, path: &str, mut item: RawNodeHandle) -> Result<()> {
        let seq_ptr = self.get_node_ptr_at(path)?;
        let seq_type = unsafe { fy_node_get_type(seq_ptr) };
        if seq_type != FYNT_SEQUENCE {
            return Err(Error::TypeMismatch {
                expected: "sequence",
                got: "non-sequence",
            });
        }
        let ret = unsafe { fy_node_sequence_append(seq_ptr, item.as_ptr()) };
        if ret != 0 {
            return Err(Error::Ffi("fy_node_sequence_append failed"));
        }
        // Mark as inserted so Drop doesn't free it
        item.mark_inserted();
        Ok(())
    }

    // ==================== Internal Helpers ====================

    fn get_node_ptr_at(&self, path: &str) -> Result<*mut fy_node> {
        let root_ptr = unsafe { fy_document_root(self.doc_ptr()) };
        if root_ptr.is_null() {
            return Err(Error::Ffi("document has no root"));
        }
        if path.is_empty() {
            return Ok(root_ptr);
        }
        let node_ptr =
            unsafe { fy_node_by_path(root_ptr, path.as_ptr() as *const i8, path.len(), 0) };
        if node_ptr.is_null() {
            return Err(Error::Ffi("path not found"));
        }
        Ok(node_ptr)
    }
}

#[cfg(test)]
mod tests {
    use crate::Document;

    #[test]
    fn test_set_yaml_at_replace() {
        let mut doc = Document::parse_str("name: Alice").unwrap();
        {
            let mut ed = doc.edit();
            ed.set_yaml_at("/name", "'Bob'").unwrap();
        }
        let name = doc.at_path("/name").unwrap().scalar_str().unwrap();
        assert_eq!(name, "Bob");
    }

    #[test]
    fn test_set_yaml_at_new_key() {
        let mut doc = Document::parse_str("name: Alice").unwrap();
        {
            let mut ed = doc.edit();
            ed.set_yaml_at("/age", "30").unwrap();
        }
        assert_eq!(doc.at_path("/age").unwrap().scalar_str().unwrap(), "30");
        assert_eq!(doc.at_path("/name").unwrap().scalar_str().unwrap(), "Alice");
    }

    #[test]
    fn test_delete_at() {
        let mut doc = Document::parse_str("name: Alice\nage: 30").unwrap();
        {
            let mut ed = doc.edit();
            let deleted = ed.delete_at("/age").unwrap();
            assert!(deleted);
        }
        assert!(doc.at_path("/age").is_none());
        assert!(doc.at_path("/name").is_some());
    }

    #[test]
    fn test_delete_nonexistent() {
        let mut doc = Document::parse_str("name: Alice").unwrap();
        {
            let mut ed = doc.edit();
            let deleted = ed.delete_at("/nonexistent").unwrap();
            assert!(!deleted);
        }
    }

    #[test]
    fn test_build_and_set_root() {
        let mut doc = Document::new().unwrap();
        {
            let mut ed = doc.edit();
            let root = ed.build_from_yaml("name: Alice").unwrap();
            ed.set_root(root).unwrap();
        }
        assert_eq!(doc.at_path("/name").unwrap().scalar_str().unwrap(), "Alice");
    }

    #[test]
    fn test_copy_node() {
        let src = Document::parse_str("key: value").unwrap();
        let src_node = src.root().unwrap();

        let mut dest = Document::new().unwrap();
        {
            let mut ed = dest.edit();
            let copied = ed.copy_node(src_node).unwrap();
            ed.set_root(copied).unwrap();
        }
        assert!(dest.root().is_some());
    }

    #[test]
    fn test_preserves_quotes() {
        let mut doc = Document::parse_str("name: plain").unwrap();
        {
            let mut ed = doc.edit();
            ed.set_yaml_at("/name", "'quoted'").unwrap();
        }
        let output = doc.emit().unwrap();
        assert!(output.contains("'quoted'"));
    }
}
