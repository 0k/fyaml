//! Lifetime-bound iterators for sequences and mappings.

use crate::node_ref::NodeRef;
use fyaml_sys::*;
use libc::c_void;
use std::ptr::{self, NonNull};

/// Iterator over items in a sequence node.
///
/// Yields [`NodeRef`] items, all tied to the same document lifetime.
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
pub struct SeqIter<'doc> {
    node: NodeRef<'doc>,
    iter_ptr: *mut c_void,
}

impl<'doc> SeqIter<'doc> {
    /// Creates a new sequence iterator.
    ///
    /// If `node` is not a sequence, the iterator will be empty.
    pub(crate) fn new(node: NodeRef<'doc>) -> Self {
        SeqIter {
            node,
            iter_ptr: ptr::null_mut(),
        }
    }
}

impl<'doc> Iterator for SeqIter<'doc> {
    type Item = NodeRef<'doc>;

    fn next(&mut self) -> Option<Self::Item> {
        let node_ptr = unsafe { fy_node_sequence_iterate(self.node.as_ptr(), &mut self.iter_ptr) };
        NonNull::new(node_ptr).map(|nn| NodeRef::new(nn, self.node.document()))
    }
}

/// Iterator over key-value pairs in a mapping node.
///
/// Yields `(NodeRef, NodeRef)` pairs, all tied to the same document lifetime.
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
pub struct MapIter<'doc> {
    node: NodeRef<'doc>,
    iter_ptr: *mut c_void,
}

impl<'doc> MapIter<'doc> {
    /// Creates a new mapping iterator.
    ///
    /// If `node` is not a mapping, the iterator will be empty.
    pub(crate) fn new(node: NodeRef<'doc>) -> Self {
        MapIter {
            node,
            iter_ptr: ptr::null_mut(),
        }
    }
}

impl<'doc> Iterator for MapIter<'doc> {
    type Item = (NodeRef<'doc>, NodeRef<'doc>);

    fn next(&mut self) -> Option<Self::Item> {
        let pair_ptr = unsafe { fy_node_mapping_iterate(self.node.as_ptr(), &mut self.iter_ptr) };
        if pair_ptr.is_null() {
            return None;
        }

        let key_ptr = unsafe { fy_node_pair_key(pair_ptr) };
        let value_ptr = unsafe { fy_node_pair_value(pair_ptr) };

        // Both key and value should be non-null for a valid pair
        let key = NonNull::new(key_ptr)?;
        let value = NonNull::new(value_ptr)?;

        Some((
            NodeRef::new(key, self.node.document()),
            NodeRef::new(value, self.node.document()),
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::Document;

    #[test]
    fn test_seq_iter() {
        let doc = Document::parse_str("- a\n- b\n- c").unwrap();
        let root = doc.root().unwrap();
        let items: Vec<&str> = root.seq_iter().map(|n| n.scalar_str().unwrap()).collect();
        assert_eq!(items, vec!["a", "b", "c"]);
    }

    #[test]
    fn test_seq_iter_empty() {
        let doc = Document::parse_str("[]").unwrap();
        let root = doc.root().unwrap();
        let count = root.seq_iter().count();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_map_iter() {
        let doc = Document::parse_str("a: 1\nb: 2").unwrap();
        let root = doc.root().unwrap();
        let pairs: Vec<(&str, &str)> = root
            .map_iter()
            .map(|(k, v)| (k.scalar_str().unwrap(), v.scalar_str().unwrap()))
            .collect();
        assert_eq!(pairs, vec![("a", "1"), ("b", "2")]);
    }

    #[test]
    fn test_map_iter_empty() {
        let doc = Document::parse_str("{}").unwrap();
        let root = doc.root().unwrap();
        let count = root.map_iter().count();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_nested_iter() {
        let doc = Document::parse_str("users:\n  - name: Alice\n  - name: Bob").unwrap();
        let root = doc.root().unwrap();
        let users = root.at_path("/users").unwrap();

        let names: Vec<&str> = users
            .seq_iter()
            .map(|u| u.at_path("/name").unwrap().scalar_str().unwrap())
            .collect();
        assert_eq!(names, vec!["Alice", "Bob"]);
    }
}
