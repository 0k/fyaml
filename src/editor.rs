//! Exclusive mutation API for documents.

use crate::diag::{diag_error, Diag};
use crate::document::Document;
use crate::error::{Error, Result};
use crate::ffi_util::malloc_copy;
use crate::node_ref::NodeRef;
use fyaml_sys::*;

use std::ptr::{self, NonNull};

// =============================================================================
// DiagGuard - RAII guard for temporary diag swap
// =============================================================================

/// RAII guard that restores the original diag on drop.
///
/// This ensures the document's diag is restored even if there's a panic
/// or early return during node building.
struct DiagGuard {
    doc_ptr: *mut fy_document,
    original_diag: *mut fy_diag,
}

impl DiagGuard {
    /// Creates a guard that will restore `original_diag` to `doc_ptr` on drop.
    fn new(doc_ptr: *mut fy_document, original_diag: *mut fy_diag) -> Self {
        Self {
            doc_ptr,
            original_diag,
        }
    }
}

impl Drop for DiagGuard {
    fn drop(&mut self) {
        // Restore original diag - safe even if doc_ptr is valid (which it must be
        // since Editor holds &mut Document)
        unsafe { fy_document_set_diag(self.doc_ptr, self.original_diag) };
    }
}

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
    /// Creates a detached handle from a raw pointer, returning an error if null.
    ///
    /// The caller must ensure `ptr` was allocated by libfyaml for this document.
    pub(crate) fn try_from_ptr(ptr: *mut fy_node, err: &'static str) -> Result<Self> {
        let node_ptr = NonNull::new(ptr).ok_or(Error::Ffi(err))?;
        Ok(Self {
            node_ptr,
            inserted: false,
        })
    }

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
    /// If the path doesn't exist, it will be created (for mappings only).
    ///
    /// The YAML snippet is parsed and its formatting (including quotes) is preserved.
    ///
    /// # Supported Parent Types
    ///
    /// - **Mappings**: Use string keys (e.g., `/config/host`). New keys are created if missing.
    /// - **Sequences**: Use integer indices (e.g., `/items/0`, `/items/-1`).
    ///   Negative indices count from the end (Python-style).
    ///   Index must be within bounds (cannot append via `set_yaml_at`).
    ///
    /// # Examples
    ///
    /// Setting a mapping value:
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
    ///
    /// Setting a sequence element:
    /// ```
    /// use fyaml::Document;
    ///
    /// let mut doc = Document::parse_str("items:\n  - a\n  - b\n  - c").unwrap();
    /// {
    ///     let mut ed = doc.edit();
    ///     ed.set_yaml_at("/items/1", "replaced").unwrap();
    ///     ed.set_yaml_at("/items/-1", "last").unwrap();  // negative index
    /// }
    /// assert_eq!(doc.at_path("/items/1").unwrap().scalar_str().unwrap(), "replaced");
    /// assert_eq!(doc.at_path("/items/2").unwrap().scalar_str().unwrap(), "last");
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

        // Check parent type and handle accordingly
        let parent_type = unsafe { fy_node_get_type(parent_ptr) };

        if parent_type == FYNT_MAPPING {
            // Look up existing pair
            let pair_ptr = unsafe {
                fy_node_mapping_lookup_pair_by_string(
                    parent_ptr,
                    key.as_ptr() as *const i8,
                    key.len(),
                )
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
        } else if parent_type == FYNT_SEQUENCE {
            // Parse key as index (supports negative indices like Python)
            let index: i32 = key
                .parse()
                .map_err(|_| Error::Ffi("invalid sequence index"))?;

            let count = unsafe { fy_node_sequence_item_count(parent_ptr) };

            // Resolve negative index
            let resolved_index = if index < 0 { count + index } else { index };

            if resolved_index < 0 || resolved_index >= count {
                return Err(Error::Ffi("sequence index out of bounds"));
            }

            // Get the item at the target index
            let old_item = unsafe { fy_node_sequence_get_by_index(parent_ptr, resolved_index) };
            if old_item.is_null() {
                return Err(Error::Ffi("sequence element not found"));
            }

            // Get the next item BEFORE removing (indices will shift after removal)
            let next_item =
                unsafe { fy_node_sequence_get_by_index(parent_ptr, resolved_index + 1) };

            // Remove the old item
            let removed = unsafe { fy_node_sequence_remove(parent_ptr, old_item) };
            if removed.is_null() {
                return Err(Error::Ffi("fy_node_sequence_remove failed"));
            }
            // Free the detached node
            unsafe { fy_node_free(removed) };

            // Insert new item at the same position
            if next_item.is_null() {
                // Was the last item, append
                let ret = unsafe { fy_node_sequence_append(parent_ptr, new_node.as_ptr()) };
                if ret != 0 {
                    return Err(Error::Ffi("fy_node_sequence_append failed"));
                }
            } else {
                // Insert before the next item
                let ret = unsafe {
                    fy_node_sequence_insert_before(parent_ptr, next_item, new_node.as_ptr())
                };
                if ret != 0 {
                    return Err(Error::Ffi("fy_node_sequence_insert_before failed"));
                }
            }
        } else {
            return Err(Error::TypeMismatch {
                expected: "mapping or sequence",
                got: "scalar",
            });
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
    ///
    /// # Errors
    ///
    /// Returns [`Error::ParseError`] with line and column information if parsing fails.
    pub fn build_from_yaml(&mut self, yaml: &str) -> Result<RawNodeHandle> {
        let buffer = unsafe { malloc_copy(yaml.as_bytes())? };

        // Create diagnostic handler to capture errors
        let diag = Diag::new();
        let diag_ptr = diag.as_ref().map(|d| d.as_ptr()).unwrap_or(ptr::null_mut());

        // Save original diag and set our capture diag with RAII guard for restoration
        let original_diag = unsafe { fy_document_get_diag(self.doc_ptr()) };
        let _guard = DiagGuard::new(self.doc_ptr(), original_diag);
        if !diag_ptr.is_null() {
            unsafe { fy_document_set_diag(self.doc_ptr(), diag_ptr) };
        }

        let node_ptr =
            unsafe { fy_node_build_from_malloc_string(self.doc_ptr(), buffer, yaml.len()) };

        // Guard will restore original diag on drop (including early returns and panics)

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
            return Err(diag_error(diag, "fy_node_build_from_malloc_string failed"));
        }
        // On success, libfyaml takes ownership of buffer (freed when document is destroyed).
        // node_ptr is non-null here (null case handled above).
        Ok(RawNodeHandle {
            node_ptr: NonNull::new(node_ptr).unwrap(),
            inserted: false,
        })
    }

    /// Creates a scalar node from raw pointer and length.
    ///
    /// Pass `(ptr::null(), 0)` for YAML null (distinct from empty string `("", 0)`).
    fn build_scalar_raw(&mut self, ptr: *const i8, len: usize) -> Result<RawNodeHandle> {
        let node_ptr = unsafe { fy_node_create_scalar_copy(self.doc_ptr(), ptr, len) };
        RawNodeHandle::try_from_ptr(node_ptr, "fy_node_create_scalar_copy failed")
    }

    /// Builds a plain scalar node.
    ///
    /// The scalar style is automatically determined based on content.
    /// Use [`build_from_yaml`](Self::build_from_yaml) for explicit quoting.
    pub fn build_scalar(&mut self, value: &str) -> Result<RawNodeHandle> {
        self.build_scalar_raw(value.as_ptr() as *const i8, value.len())
    }

    /// Builds an empty sequence node.
    pub fn build_sequence(&mut self) -> Result<RawNodeHandle> {
        let ptr = unsafe { fy_node_create_sequence(self.doc_ptr()) };
        RawNodeHandle::try_from_ptr(ptr, "fy_node_create_sequence failed")
    }

    /// Builds an empty mapping node.
    pub fn build_mapping(&mut self) -> Result<RawNodeHandle> {
        let ptr = unsafe { fy_node_create_mapping(self.doc_ptr()) };
        RawNodeHandle::try_from_ptr(ptr, "fy_node_create_mapping failed")
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
        let ptr = unsafe { fy_node_copy(self.doc_ptr(), source.as_ptr()) };
        RawNodeHandle::try_from_ptr(ptr, "fy_node_copy failed")
    }

    // ==================== Handle-Level Node Assembly ====================

    /// Appends an item to a detached sequence handle.
    ///
    /// The `item` handle is consumed (the sequence takes ownership).
    /// The `seq` handle must have been created with [`build_sequence`](Self::build_sequence).
    ///
    /// # Safety contract
    ///
    /// libfyaml's `fy_node_sequence_append` validates preconditions (type, attachment,
    /// document match) before linking. On error, the node is not consumed — ownership
    /// remains with the caller (verified in libfyaml source: `fy-doc.c`).
    pub fn seq_append(&mut self, seq: &mut RawNodeHandle, mut item: RawNodeHandle) -> Result<()> {
        let ret = unsafe { fy_node_sequence_append(seq.as_ptr(), item.as_ptr()) };
        if ret != 0 {
            return Err(Error::Ffi("fy_node_sequence_append failed"));
        }
        item.mark_inserted();
        Ok(())
    }

    /// Inserts a key-value pair into a detached mapping handle.
    ///
    /// Both `key` and `value` handles are consumed (the mapping takes ownership).
    /// The `map` handle must have been created with [`build_mapping`](Self::build_mapping).
    ///
    /// # Safety contract
    ///
    /// libfyaml's `fy_node_mapping_append` validates preconditions (type, attachment,
    /// document match, duplicate keys) before linking. On error, neither key nor value
    /// is consumed — ownership remains with the caller (verified in libfyaml source: `fy-doc.c`).
    pub fn map_insert(
        &mut self,
        map: &mut RawNodeHandle,
        mut key: RawNodeHandle,
        mut value: RawNodeHandle,
    ) -> Result<()> {
        let ret = unsafe { fy_node_mapping_append(map.as_ptr(), key.as_ptr(), value.as_ptr()) };
        if ret != 0 {
            return Err(Error::Ffi("fy_node_mapping_append failed"));
        }
        key.mark_inserted();
        value.mark_inserted();
        Ok(())
    }

    /// Sets the style of a detached node handle.
    ///
    /// libfyaml validates the requested style against the node content.
    /// If the style is not valid for this content (e.g., plain for a scalar
    /// that would be ambiguous), libfyaml may keep the current style.
    ///
    /// Returns the style that was actually set.
    pub fn set_style(
        &mut self,
        node: &mut RawNodeHandle,
        style: crate::node::NodeStyle,
    ) -> crate::node::NodeStyle {
        let raw_style = match style {
            crate::node::NodeStyle::Any => FYNS_ANY,
            crate::node::NodeStyle::Flow => FYNS_FLOW,
            crate::node::NodeStyle::Block => FYNS_BLOCK,
            crate::node::NodeStyle::Plain => FYNS_PLAIN,
            crate::node::NodeStyle::SingleQuoted => FYNS_SINGLE_QUOTED,
            crate::node::NodeStyle::DoubleQuoted => FYNS_DOUBLE_QUOTED,
            crate::node::NodeStyle::Literal => FYNS_LITERAL,
            crate::node::NodeStyle::Folded => FYNS_FOLDED,
            crate::node::NodeStyle::Alias => FYNS_ALIAS,
        };
        let result = unsafe { fy_node_set_style(node.as_ptr(), raw_style) };
        crate::node::NodeStyle::from(result)
    }

    /// Sets a YAML tag on a detached node handle.
    ///
    /// For example, `set_tag(&mut node, "!custom")` produces `!custom value`.
    pub fn set_tag(&mut self, node: &mut RawNodeHandle, tag: &str) -> Result<()> {
        let ret = unsafe { fy_node_set_tag(node.as_ptr(), tag.as_ptr() as *const i8, tag.len()) };
        if ret != 0 {
            return Err(Error::Ffi("fy_node_set_tag failed"));
        }
        Ok(())
    }

    /// Builds a null scalar node.
    ///
    /// Uses `build_from_yaml` internally because libfyaml's
    /// `fy_node_create_scalar_copy(doc, NULL, 0)` produces an empty scalar
    /// without the `is_null` flag that the parser sets. Going through the
    /// parser ensures `fy_node_is_null()` returns true and the emitter
    /// produces `null` instead of empty string.
    pub fn build_null(&mut self) -> Result<RawNodeHandle> {
        self.build_from_yaml("null")
    }

    // ==================== Path-Based Sequence Operations ====================

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

    #[test]
    fn test_set_yaml_at_sequence_first() {
        let mut doc = Document::parse_str("items:\n  - a\n  - b\n  - c").unwrap();
        {
            let mut ed = doc.edit();
            ed.set_yaml_at("/items/0", "'replaced'").unwrap();
        }
        assert_eq!(
            doc.at_path("/items/0").unwrap().scalar_str().unwrap(),
            "replaced"
        );
        assert_eq!(doc.at_path("/items/1").unwrap().scalar_str().unwrap(), "b");
        assert_eq!(doc.at_path("/items/2").unwrap().scalar_str().unwrap(), "c");
    }

    #[test]
    fn test_set_yaml_at_sequence_middle() {
        let mut doc = Document::parse_str("items:\n  - a\n  - b\n  - c").unwrap();
        {
            let mut ed = doc.edit();
            ed.set_yaml_at("/items/1", "replaced").unwrap();
        }
        assert_eq!(doc.at_path("/items/0").unwrap().scalar_str().unwrap(), "a");
        assert_eq!(
            doc.at_path("/items/1").unwrap().scalar_str().unwrap(),
            "replaced"
        );
        assert_eq!(doc.at_path("/items/2").unwrap().scalar_str().unwrap(), "c");
    }

    #[test]
    fn test_set_yaml_at_sequence_last() {
        let mut doc = Document::parse_str("items:\n  - a\n  - b\n  - c").unwrap();
        {
            let mut ed = doc.edit();
            ed.set_yaml_at("/items/2", "replaced").unwrap();
        }
        assert_eq!(doc.at_path("/items/0").unwrap().scalar_str().unwrap(), "a");
        assert_eq!(doc.at_path("/items/1").unwrap().scalar_str().unwrap(), "b");
        assert_eq!(
            doc.at_path("/items/2").unwrap().scalar_str().unwrap(),
            "replaced"
        );
    }

    #[test]
    fn test_set_yaml_at_sequence_negative_index() {
        let mut doc = Document::parse_str("items:\n  - a\n  - b\n  - c").unwrap();
        {
            let mut ed = doc.edit();
            ed.set_yaml_at("/items/-1", "last").unwrap();
        }
        assert_eq!(doc.at_path("/items/0").unwrap().scalar_str().unwrap(), "a");
        assert_eq!(doc.at_path("/items/1").unwrap().scalar_str().unwrap(), "b");
        assert_eq!(
            doc.at_path("/items/2").unwrap().scalar_str().unwrap(),
            "last"
        );
    }

    #[test]
    fn test_set_yaml_at_sequence_negative_first() {
        let mut doc = Document::parse_str("items:\n  - a\n  - b\n  - c").unwrap();
        {
            let mut ed = doc.edit();
            ed.set_yaml_at("/items/-3", "first").unwrap();
        }
        assert_eq!(
            doc.at_path("/items/0").unwrap().scalar_str().unwrap(),
            "first"
        );
        assert_eq!(doc.at_path("/items/1").unwrap().scalar_str().unwrap(), "b");
        assert_eq!(doc.at_path("/items/2").unwrap().scalar_str().unwrap(), "c");
    }

    #[test]
    fn test_set_yaml_at_sequence_out_of_bounds() {
        let mut doc = Document::parse_str("items:\n  - a\n  - b").unwrap();
        {
            let mut ed = doc.edit();
            let result = ed.set_yaml_at("/items/5", "oob");
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_set_yaml_at_sequence_negative_out_of_bounds() {
        let mut doc = Document::parse_str("items:\n  - a\n  - b").unwrap();
        {
            let mut ed = doc.edit();
            let result = ed.set_yaml_at("/items/-5", "oob");
            assert!(result.is_err());
        }
    }

    #[test]
    fn test_set_yaml_at_sequence_complex_value() {
        let mut doc = Document::parse_str("items:\n  - simple").unwrap();
        {
            let mut ed = doc.edit();
            ed.set_yaml_at("/items/0", "key: value").unwrap();
        }
        let item = doc.at_path("/items/0").unwrap();
        assert!(item.is_mapping());
        assert_eq!(item.map_get("key").unwrap().scalar_str().unwrap(), "value");
    }

    #[test]
    fn test_set_yaml_at_nested_in_sequence() {
        let mut doc = Document::parse_str("items:\n  - name: alice\n  - name: bob").unwrap();
        {
            let mut ed = doc.edit();
            ed.set_yaml_at("/items/0/name", "charlie").unwrap();
        }
        assert_eq!(
            doc.at_path("/items/0/name").unwrap().scalar_str().unwrap(),
            "charlie"
        );
        assert_eq!(
            doc.at_path("/items/1/name").unwrap().scalar_str().unwrap(),
            "bob"
        );
    }

    #[test]
    fn test_seq_append() {
        let mut doc = Document::new().unwrap();
        {
            let mut ed = doc.edit();
            let mut seq = ed.build_sequence().unwrap();
            let a = ed.build_scalar("a").unwrap();
            let b = ed.build_scalar("b").unwrap();
            ed.seq_append(&mut seq, a).unwrap();
            ed.seq_append(&mut seq, b).unwrap();
            ed.set_root(seq).unwrap();
        }
        let root = doc.root().unwrap();
        assert!(root.is_sequence());
        assert_eq!(root.seq_get(0).unwrap().scalar_str().unwrap(), "a");
        assert_eq!(root.seq_get(1).unwrap().scalar_str().unwrap(), "b");
    }

    #[test]
    fn test_map_insert() {
        let mut doc = Document::new().unwrap();
        {
            let mut ed = doc.edit();
            let mut map = ed.build_mapping().unwrap();
            let k = ed.build_scalar("name").unwrap();
            let v = ed.build_scalar("Alice").unwrap();
            ed.map_insert(&mut map, k, v).unwrap();
            ed.set_root(map).unwrap();
        }
        assert_eq!(doc.at_path("/name").unwrap().scalar_str().unwrap(), "Alice");
    }

    #[test]
    fn test_set_tag() {
        let mut doc = Document::new().unwrap();
        {
            let mut ed = doc.edit();
            let mut node = ed.build_scalar("42").unwrap();
            ed.set_tag(&mut node, "!custom").unwrap();
            ed.set_root(node).unwrap();
        }
        let root = doc.root().unwrap();
        assert_eq!(root.tag_str().unwrap().unwrap(), "!custom");
        assert_eq!(root.scalar_str().unwrap(), "42");
    }

    #[test]
    fn test_build_null() {
        // Note: build_null() creates a zero-length scalar via NULL ptr.
        // libfyaml does NOT distinguish this from build_scalar("") — both
        // emit as empty string. For YAML null semantics, use build_scalar("null").
        let mut doc = Document::new().unwrap();
        {
            let mut ed = doc.edit();
            let node = ed.build_null().unwrap();
            ed.set_root(node).unwrap();
        }
        let root = doc.root().unwrap();
        assert!(root.is_scalar());
        let emitted = root.emit().unwrap();
        assert!(emitted.is_empty() || emitted == "null");
    }
}
