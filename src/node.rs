//! YAML node types and navigation.
//!
//! This module provides types for working with individual YAML nodes,
//! including scalars, sequences, and mappings.

use crate::document::FyDocument;
use fyaml_sys::*;
use libc::{c_void, size_t};
use std::fmt;
use std::ptr;
use std::rc::Rc;
use std::slice;
use std::str::FromStr;

/// The type of a YAML node.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum NodeType {
    /// A scalar value (string, number, boolean, null).
    Scalar,
    /// A sequence (list/array) of nodes.
    Sequence,
    /// A mapping (dictionary/object) of key-value pairs.
    Mapping,
}

impl From<u32> for NodeType {
    fn from(value: u32) -> Self {
        match value {
            x if x == fyaml_sys::FYNT_SCALAR => NodeType::Scalar,
            x if x == fyaml_sys::FYNT_SEQUENCE => NodeType::Sequence,
            x if x == fyaml_sys::FYNT_MAPPING => NodeType::Mapping,
            // libfyaml should only return valid node types; default to Scalar
            // if we somehow get an unexpected value (defensive programming)
            _ => {
                log::warn!(
                    "Unknown fy_node_type value: {}, defaulting to Scalar",
                    value
                );
                NodeType::Scalar
            }
        }
    }
}

/// Low-level YAML node wrapping libfyaml's `fy_node`.
///
/// This is an internal type. Use [`Node`] for the safe public API.
pub struct FyNode {
    pub(crate) node_ptr: *mut fy_node,
}

/// A YAML node with shared ownership of its parent document.
///
/// Nodes can be scalars, sequences, or mappings. Use the `is_*` methods
/// or [`get_type`](Node::get_type) to determine the node type.
///
/// # Path Navigation
///
/// Use [`node_by_path`](Node::node_by_path) to navigate to child nodes:
///
/// ```
/// use fyaml::node::Node;
/// use std::str::FromStr;
///
/// let yaml = "database:\n  host: localhost\n  port: 5432";
/// let root = Node::from_str(yaml).unwrap();
/// let host = root.node_by_path("/database/host").unwrap();
/// assert_eq!(host.to_raw_string().unwrap(), "localhost");
/// ```
pub struct Node {
    pub(crate) fy_node: Rc<FyNode>,
    pub(crate) fy_doc: Rc<FyDocument>,
}

impl FyNode {
    fn node_by_path(&self, path: &str) -> Option<Rc<FyNode>> {
        log::trace!("FyNode.node_by_path: {}", path);
        let node_ptr =
            unsafe { fy_node_by_path(self.node_ptr, path.as_ptr() as *const i8, path.len(), 0) };
        if node_ptr.is_null() {
            return None;
        }
        // SAFETY: The returned node is owned by the document (not this node),
        // so we don't free it in Drop. The document outlives this FyNode
        // because Node holds Rc<FyDocument>.
        Some(Rc::new(FyNode { node_ptr }))
    }

    fn get_type(&self) -> NodeType {
        unsafe { NodeType::from(fy_node_get_type(self.node_ptr)) }
    }

    fn is_scalar(&self) -> bool {
        self.get_type() == NodeType::Scalar
    }

    fn is_mapping(&self) -> bool {
        self.get_type() == NodeType::Mapping
    }

    fn is_sequence(&self) -> bool {
        self.get_type() == NodeType::Sequence
    }

    fn to_raw_string(&self) -> Result<String, String> {
        let mut len: size_t = 0;
        let data_ptr = unsafe { fy_node_get_scalar(self.node_ptr, &mut len) };
        if data_ptr.is_null() {
            return Err("Failed to read value".to_string());
        }
        let bytes = unsafe { slice::from_raw_parts(data_ptr as *const u8, len) };
        log::trace!("bytes: {:?}", bytes);
        match std::str::from_utf8(bytes) {
            Ok(value) => Ok(value.to_string()),
            Err(e) => {
                log::trace!("bytes: {:?}", bytes);
                Err(format!("Failed to read value: {}", e))
            }
        }
    }

    fn to_string_safe(&self) -> Result<String, String> {
        let ptr = unsafe { fy_emit_node_to_string(self.node_ptr, FYECF_MODE_DEJSON) };
        if ptr.is_null() {
            return Err("Failed to dump YAML node".to_string());
        }
        let s = unsafe { std::ffi::CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned();
        unsafe { libc::free(ptr as *mut c_void) };
        Ok(s)
    }

    fn seq_len(&self) -> Result<i32, String> {
        let len: i32 = unsafe { fy_node_sequence_item_count(self.node_ptr) };
        if len < 0 {
            return Err("Failed to get sequence length".to_string());
        }
        Ok(len)
    }

    fn map_len(&self) -> Result<i32, String> {
        let len: i32 = unsafe { fy_node_mapping_item_count(self.node_ptr) };
        if len < 0 {
            return Err("Failed to get mapping length".to_string());
        }
        Ok(len)
    }

    fn get_tag(&self) -> Result<Option<String>, String> {
        let mut len: size_t = 0;
        let tag_ptr = unsafe { fy_node_get_tag(self.node_ptr, &mut len) };
        if tag_ptr.is_null() {
            return Ok(None);
        }
        let bytes = unsafe { slice::from_raw_parts(tag_ptr as *const u8, len) };
        match std::str::from_utf8(bytes) {
            Ok(value) => Ok(Some(value.to_string())),
            Err(e) => Err(format!("Failed to read tag: {}", e)),
        }
    }
}

struct FyMappingIterator<'a> {
    fy_node: &'a FyNode,
    prevp: *mut c_void,
}

impl<'a> FyMappingIterator<'a> {
    fn new(node: &FyNode) -> FyMappingIterator<'_> {
        FyMappingIterator {
            fy_node: node,
            prevp: ptr::null_mut(),
        }
    }
}

impl<'a> Iterator for FyMappingIterator<'a> {
    type Item = Result<(FyNode, FyNode), String>;

    fn next(&mut self) -> Option<Self::Item> {
        log::trace!("FyMappingIterator.next");
        let node_pair_ptr =
            unsafe { fy_node_mapping_iterate(self.fy_node.node_ptr, &mut self.prevp) };
        if node_pair_ptr.is_null() {
            log::trace!("FyMappingIterator: end");
            return None;
        }
        let node_key_ptr = unsafe { fy_node_pair_key(node_pair_ptr) };
        if node_key_ptr.is_null() {
            return Some(Err("Failed to get mapping key".to_string()));
        }
        let node_value_ptr = unsafe { fy_node_pair_value(node_pair_ptr) };
        if node_value_ptr.is_null() {
            return Some(Err("Failed to get mapping value".to_string()));
        }

        let key = FyNode {
            node_ptr: node_key_ptr,
        };
        let value = FyNode {
            node_ptr: node_value_ptr,
        };
        Some(Ok((key, value)))
    }
}

struct FySequenceIterator<'a> {
    fy_node: &'a FyNode,
    prevp: *mut c_void,
}

impl<'a> FySequenceIterator<'a> {
    fn new(node: &FyNode) -> FySequenceIterator<'_> {
        FySequenceIterator {
            fy_node: node,
            prevp: ptr::null_mut(),
        }
    }
}

impl<'a> Iterator for FySequenceIterator<'a> {
    type Item = Result<FyNode, String>;

    fn next(&mut self) -> Option<Self::Item> {
        log::trace!("FySequenceIterator.next");
        let node_ptr = unsafe { fy_node_sequence_iterate(self.fy_node.node_ptr, &mut self.prevp) };
        if node_ptr.is_null() {
            log::trace!("FySequenceIterator: end");
            return None;
        }
        Some(Ok(FyNode { node_ptr }))
    }
}

impl Node {
    /// Navigates to a child node by path.
    ///
    /// Path format uses `/` as separator:
    /// - `/foo` - access key "foo" in a mapping
    /// - `/0` - access index 0 in a sequence
    /// - `/foo/bar/0` - nested access
    ///
    /// Returns `None` if the path doesn't exist.
    pub fn node_by_path(&self, path: &str) -> Option<Rc<Node>> {
        log::trace!("Node.node_by_path: {}", path);
        Some(Rc::new(Node {
            fy_node: self.fy_node.node_by_path(path)?,
            fy_doc: Rc::clone(&self.fy_doc),
        }))
    }

    /// Returns the YAML tag of this node, if any.
    ///
    /// Standard tags like `!str` may not be returned for implicitly typed values.
    pub fn get_tag(&self) -> Result<Option<String>, String> {
        self.fy_node.get_tag()
    }

    /// Returns the type of this node.
    pub fn get_type(&self) -> NodeType {
        self.fy_node.get_type()
    }

    /// Returns `true` if this node is a scalar value.
    pub fn is_scalar(&self) -> bool {
        self.fy_node.is_scalar()
    }

    /// Returns `true` if this node is a mapping (dictionary).
    pub fn is_mapping(&self) -> bool {
        self.fy_node.is_mapping()
    }

    /// Returns `true` if this node is a sequence (list).
    pub fn is_sequence(&self) -> bool {
        self.fy_node.is_sequence()
    }

    /// Returns the raw string value of a scalar node.
    ///
    /// This returns the unquoted, unescaped value. For non-scalar nodes,
    /// use [`to_string_safe`](Self::to_string_safe) or the [`Display`](std::fmt::Display) implementation instead.
    pub fn to_raw_string(&self) -> Result<String, String> {
        self.fy_node.to_raw_string()
    }

    /// Returns the YAML string representation of this node.
    ///
    /// For complex nodes (mappings, sequences), this returns valid YAML.
    pub fn to_string_safe(&self) -> Result<String, String> {
        self.fy_node.to_string_safe()
    }

    /// Returns the number of items in a sequence node.
    pub fn seq_len(&self) -> Result<i32, String> {
        self.fy_node.seq_len()
    }

    /// Returns the number of key-value pairs in a mapping node.
    pub fn map_len(&self) -> Result<i32, String> {
        self.fy_node.map_len()
    }

    /// Returns an iterator over key-value pairs in a mapping node.
    pub fn map_iter(&self) -> MappingIterator<'_> {
        MappingIterator::new(self)
    }

    /// Returns an iterator over items in a sequence node.
    pub fn seq_iter(&self) -> SequenceIterator<'_> {
        SequenceIterator::new(self)
    }
}

/// Iterator over key-value pairs in a mapping node.
pub struct MappingIterator<'a> {
    fy_iter: FyMappingIterator<'a>,
    node: &'a Node,
}

impl<'a> MappingIterator<'a> {
    fn new(node: &'a Node) -> MappingIterator<'a> {
        MappingIterator {
            fy_iter: FyMappingIterator::new(&node.fy_node),
            node,
        }
    }
}

impl<'a> Iterator for MappingIterator<'a> {
    type Item = Result<(Node, Node), String>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.fy_iter.next() {
            Some(Ok((key, value))) => Some(Ok((
                Node {
                    fy_node: Rc::new(key),
                    fy_doc: Rc::clone(&self.node.fy_doc),
                },
                Node {
                    fy_node: Rc::new(value),
                    fy_doc: Rc::clone(&self.node.fy_doc),
                },
            ))),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}

/// Iterator over items in a sequence node.
pub struct SequenceIterator<'a> {
    fy_iter: FySequenceIterator<'a>,
    node: &'a Node,
}

impl<'a> SequenceIterator<'a> {
    fn new(node: &'a Node) -> SequenceIterator<'a> {
        SequenceIterator {
            fy_iter: FySequenceIterator::new(&node.fy_node),
            node,
        }
    }
}

impl<'a> Iterator for SequenceIterator<'a> {
    type Item = Result<Node, String>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.fy_iter.next() {
            Some(Ok(node)) => Some(Ok(Node {
                fy_node: Rc::new(node),
                fy_doc: Rc::clone(&self.node.fy_doc),
            })),
            Some(Err(e)) => Some(Err(e)),
            None => None,
        }
    }
}

impl Drop for FyNode {
    fn drop(&mut self) {
        if !self.node_ptr.is_null() {
            log::trace!("dropping FyNode {:?}", self.node_ptr);
            // SAFETY: We don't call fy_node_free here because nodes are owned
            // by the document. The document will free all nodes when it's dropped.
        }
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        log::trace!("dropping Node {:p}", self);
    }
}

impl FromStr for Node {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let doc = FyDocument::new()?;
        let node_ptr =
            unsafe { fy_node_build_from_string(doc.doc_ptr, s.as_ptr() as *const i8, s.len()) };
        if node_ptr.is_null() {
            return Err("Failed to parse string YAML node".to_string());
        }
        Ok(Node {
            fy_node: Rc::new(FyNode { node_ptr }),
            fy_doc: Rc::new(doc),
        })
    }
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_string_safe() {
            Ok(s) => write!(f, "{}", s),
            Err(_) => Ok(()),
        }
    }
}
