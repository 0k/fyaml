//! Zero-copy typed value reference.
//!
//! This module provides [`ValueRef`], a lightweight wrapper around [`NodeRef`]
//! that provides typed accessors without allocation.
//!
//! # Comparison with [`Value`](crate::value::Value)
//!
//! | Feature | `Value` | `ValueRef<'doc>` |
//! |---------|---------|------------------|
//! | Allocation | Yes (owns data) | No (borrows from document) |
//! | Lifetime | `'static` | Tied to document `'doc` |
//! | Serde | Yes | No |
//! | Modification | Yes | No (read-only) |
//!
//! Use `ValueRef` when you need to read data without allocation.
//! Use `Value` when you need ownership or serde compatibility.
//!
//! # Semantics
//!
//! `ValueRef` typed accessors follow the same type inference rules as
//! [`Value::from_node_ref`](crate::Value::from_node_ref), ensuring consistent
//! behavior regardless of which API you use.
//!
//! # Example
//!
//! ```
//! use fyaml::Document;
//!
//! let doc = Document::parse_str("name: Alice\nage: 30\nactive: true").unwrap();
//! let root = doc.root_value().unwrap();
//!
//! // Zero-copy string access
//! assert_eq!(root.get("name").unwrap().as_str(), Some("Alice"));
//!
//! // Type parsing (no allocation)
//! assert_eq!(root.get("age").unwrap().as_i64(), Some(30));
//! assert_eq!(root.get("active").unwrap().as_bool(), Some(true));
//! ```

use crate::node_ref::NodeRef;
use crate::scalar_parse;
use std::fmt;

/// A zero-copy typed view of a YAML node.
///
/// `ValueRef` wraps a [`NodeRef`] and provides typed accessor methods that
/// interpret YAML scalars as specific types without allocation.
///
/// # Type Interpretation
///
/// YAML scalars are stored as strings, but can represent various types.
/// `ValueRef` provides methods to interpret these on demand:
///
/// - `as_str()` - Returns the raw string content (zero-copy)
/// - `as_bool()` - Interprets as boolean (`true`, `false`, `yes`, `no`, etc.)
/// - `as_i64()` / `as_u64()` - Interprets as integer (supports hex, octal, binary)
/// - `as_f64()` - Interprets as floating point (supports `.inf`, `.nan`)
/// - `is_null()` - Checks for null (`null`, `~`, empty)
///
/// # Non-Plain Scalar Awareness
///
/// Scalars with non-plain styles (single-quoted `'...'`, double-quoted `"..."`,
/// literal block `|`, folded block `>`) are treated as strings and will **not**
/// be type-interpreted. This matches YAML semantics where `'true'` is a string,
/// not a boolean.
///
/// ```
/// use fyaml::Document;
///
/// let doc = Document::parse_str("quoted: 'true'\nunquoted: true").unwrap();
/// let root = doc.root_value().unwrap();
///
/// // Quoted: treated as string, not interpreted
/// assert_eq!(root.get("quoted").unwrap().as_bool(), None);
/// assert_eq!(root.get("quoted").unwrap().as_str(), Some("true"));
///
/// // Unquoted (plain): interpreted as boolean
/// assert_eq!(root.get("unquoted").unwrap().as_bool(), Some(true));
/// ```
///
/// # YAML 1.1 Boolean Compatibility
///
/// Boolean interpretation accepts YAML 1.1-style values (`yes`/`no`, `on`/`off`)
/// in addition to YAML 1.2 core schema values (`true`/`false`). This matches
/// the behavior of many YAML parsers and configuration files.
#[derive(Clone, Copy)]
pub struct ValueRef<'doc> {
    node: NodeRef<'doc>,
}

impl<'doc> ValueRef<'doc> {
    /// Creates a new `ValueRef` from a `NodeRef`.
    #[inline]
    pub fn new(node: NodeRef<'doc>) -> Self {
        ValueRef { node }
    }

    /// Returns the underlying `NodeRef`.
    #[inline]
    pub fn as_node(&self) -> NodeRef<'doc> {
        self.node
    }

    // ==================== Type Checking ====================

    /// Returns `true` if this is a scalar node.
    #[inline]
    pub fn is_scalar(&self) -> bool {
        self.node.is_scalar()
    }

    /// Returns `true` if this is a sequence (array/list).
    #[inline]
    pub fn is_sequence(&self) -> bool {
        self.node.is_sequence()
    }

    /// Returns `true` if this is a mapping (object/dictionary).
    #[inline]
    pub fn is_mapping(&self) -> bool {
        self.node.is_mapping()
    }

    /// Returns `true` if this scalar represents a null value.
    ///
    /// Recognizes: `null` (case-insensitive), `~`, and empty scalars.
    /// Non-plain scalars (quoted, literal, folded) are never considered null.
    pub fn is_null(&self) -> bool {
        if !self.node.is_scalar() {
            return false;
        }
        // Non-plain scalars are never null
        if self.node.is_non_plain() {
            return false;
        }
        match self.node.scalar_str() {
            Ok(s) => scalar_parse::is_null(s),
            Err(_) => false,
        }
    }

    // ==================== Zero-Copy String Access ====================

    /// Returns the scalar value as a string slice (zero-copy).
    ///
    /// Returns `None` if this is not a scalar or if the content is not valid UTF-8.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let doc = Document::parse_str("name: Alice").unwrap();
    /// let root = doc.root_value().unwrap();
    /// assert_eq!(root.get("name").unwrap().as_str(), Some("Alice"));
    /// ```
    pub fn as_str(&self) -> Option<&'doc str> {
        self.node.scalar_str().ok()
    }

    /// Returns the scalar value as a byte slice (zero-copy).
    ///
    /// Returns `None` if this is not a scalar.
    pub fn as_bytes(&self) -> Option<&'doc [u8]> {
        self.node.scalar_bytes().ok()
    }

    // ==================== Type Interpretation ====================

    /// Interprets the scalar as a boolean.
    ///
    /// Recognizes YAML 1.1 boolean values (for compatibility with common configs):
    /// - True: `true`, `True`, `TRUE`, `yes`, `Yes`, `YES`, `on`, `On`, `ON`
    /// - False: `false`, `False`, `FALSE`, `no`, `No`, `NO`, `off`, `Off`, `OFF`
    ///
    /// Returns `None` if not a scalar, non-plain (quoted/literal/folded),
    /// or not a recognized boolean string.
    ///
    /// # Note
    ///
    /// YAML 1.2 core schema only recognizes `true`/`false`. This method also
    /// accepts `yes`/`no`/`on`/`off` for compatibility with YAML 1.1 and
    /// common configuration files.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let doc = Document::parse_str("active: yes\nenabled: false").unwrap();
    /// let root = doc.root_value().unwrap();
    /// assert_eq!(root.get("active").unwrap().as_bool(), Some(true));
    /// assert_eq!(root.get("enabled").unwrap().as_bool(), Some(false));
    /// ```
    pub fn as_bool(&self) -> Option<bool> {
        if !self.node.is_scalar() {
            return None;
        }
        // Non-plain scalars are strings, not booleans
        if self.node.is_non_plain() {
            return None;
        }
        let s = self.node.scalar_str().ok()?;
        scalar_parse::parse_bool(s)
    }

    /// Interprets the scalar as a signed 64-bit integer.
    ///
    /// Supports:
    /// - Decimal: `42`, `-10`, `+5`
    /// - Hexadecimal: `0xFF`, `-0xFF`
    /// - Octal: `0o77`, `-0o77`
    /// - Binary: `0b1010`, `-0b1010`
    ///
    /// Returns `None` if not a scalar, non-plain (quoted/literal/folded),
    /// not a valid integer, or overflows `i64`.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let doc = Document::parse_str("count: 42\nnegative: -10").unwrap();
    /// let root = doc.root_value().unwrap();
    /// assert_eq!(root.get("count").unwrap().as_i64(), Some(42));
    /// assert_eq!(root.get("negative").unwrap().as_i64(), Some(-10));
    /// ```
    pub fn as_i64(&self) -> Option<i64> {
        if !self.node.is_scalar() {
            return None;
        }
        // Non-plain scalars are strings, not numbers
        if self.node.is_non_plain() {
            return None;
        }
        let s = self.node.scalar_str().ok()?;
        scalar_parse::parse_i64(s)
    }

    /// Interprets the scalar as an unsigned 64-bit integer.
    ///
    /// Supports:
    /// - Decimal: `42`, `+5`
    /// - Hexadecimal: `0xFF`
    /// - Octal: `0o77`
    /// - Binary: `0b1010`
    ///
    /// Returns `None` if not a scalar, non-plain, negative, not a valid integer,
    /// or overflows `u64`.
    pub fn as_u64(&self) -> Option<u64> {
        if !self.node.is_scalar() {
            return None;
        }
        if self.node.is_non_plain() {
            return None;
        }
        let s = self.node.scalar_str().ok()?;
        scalar_parse::parse_u64(s)
    }

    /// Interprets the scalar as a 64-bit floating point number.
    ///
    /// Recognizes:
    /// - Standard floats: `3.14`, `1.0e10`, `-2.5`
    /// - Positive infinity: `.inf`, `+.inf` (case-insensitive)
    /// - Negative infinity: `-.inf` (case-insensitive)
    /// - Not a number: `.nan` (case-insensitive)
    ///
    /// Note: Unlike `as_i64()`, plain integers like `42` will also parse as
    /// floats (`42.0`). Use `as_i64()` first if you need to distinguish.
    ///
    /// Returns `None` if not a scalar, non-plain, or not a valid float.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let doc = Document::parse_str("pi: 3.14159\ninf: .inf").unwrap();
    /// let root = doc.root_value().unwrap();
    /// assert!((root.get("pi").unwrap().as_f64().unwrap() - 3.14159).abs() < 0.0001);
    /// assert!(root.get("inf").unwrap().as_f64().unwrap().is_infinite());
    /// ```
    pub fn as_f64(&self) -> Option<f64> {
        if !self.node.is_scalar() {
            return None;
        }
        if self.node.is_non_plain() {
            return None;
        }
        let s = self.node.scalar_str().ok()?;
        scalar_parse::parse_f64(s)
    }

    // ==================== Navigation ====================

    /// Navigates to a child node by path.
    ///
    /// See [`NodeRef::at_path`] for path format details.
    pub fn at_path(&self, path: &str) -> Option<ValueRef<'doc>> {
        self.node.at_path(path).map(ValueRef::new)
    }

    /// Gets a value from a mapping by string key.
    ///
    /// Returns `None` if this is not a mapping or the key is not found.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let doc = Document::parse_str("name: Alice\nage: 30").unwrap();
    /// let root = doc.root_value().unwrap();
    /// assert_eq!(root.get("name").unwrap().as_str(), Some("Alice"));
    /// assert!(root.get("missing").is_none());
    /// ```
    pub fn get(&self, key: &str) -> Option<ValueRef<'doc>> {
        self.node.map_get(key).map(ValueRef::new)
    }

    /// Gets a sequence item by index.
    ///
    /// Negative indices count from the end (-1 is the last element).
    ///
    /// Returns `None` if this is not a sequence or index is out of bounds.
    pub fn index(&self, i: i32) -> Option<ValueRef<'doc>> {
        self.node.seq_get(i).map(ValueRef::new)
    }

    // ==================== Length ====================

    /// Returns the number of items in a sequence.
    ///
    /// Returns `None` if this is not a sequence.
    pub fn seq_len(&self) -> Option<usize> {
        self.node.seq_len().ok()
    }

    /// Returns the number of key-value pairs in a mapping.
    ///
    /// Returns `None` if this is not a mapping.
    pub fn map_len(&self) -> Option<usize> {
        self.node.map_len().ok()
    }

    // ==================== Iteration ====================

    /// Returns an iterator over sequence items as `ValueRef`.
    ///
    /// If this is not a sequence, the iterator will be empty.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let doc = Document::parse_str("- 1\n- 2\n- 3").unwrap();
    /// let root = doc.root_value().unwrap();
    ///
    /// let sum: i64 = root.seq_iter()
    ///     .filter_map(|v| v.as_i64())
    ///     .sum();
    /// assert_eq!(sum, 6);
    /// ```
    pub fn seq_iter(&self) -> impl Iterator<Item = ValueRef<'doc>> {
        self.node.seq_iter().map(ValueRef::new)
    }

    /// Returns an iterator over mapping key-value pairs as `(ValueRef, ValueRef)`.
    ///
    /// If this is not a mapping, the iterator will be empty.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    ///
    /// let doc = Document::parse_str("a: 1\nb: 2").unwrap();
    /// let root = doc.root_value().unwrap();
    ///
    /// for (key, value) in root.map_iter() {
    ///     println!("{}: {}", key.as_str().unwrap(), value.as_i64().unwrap());
    /// }
    /// ```
    pub fn map_iter(&self) -> impl Iterator<Item = (ValueRef<'doc>, ValueRef<'doc>)> {
        self.node
            .map_iter()
            .map(|(k, v)| (ValueRef::new(k), ValueRef::new(v)))
    }

    // ==================== Tag Access ====================

    /// Returns the YAML tag as a string slice (zero-copy).
    ///
    /// Returns `None` if the node has no explicit tag.
    pub fn tag(&self) -> Option<&'doc str> {
        self.node.tag_str().ok().flatten()
    }
}

impl fmt::Debug for ValueRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_null() {
            write!(f, "ValueRef(null)")
        } else if let Some(b) = self.as_bool() {
            write!(f, "ValueRef({})", b)
        } else if let Some(n) = self.as_i64() {
            write!(f, "ValueRef({})", n)
        } else if let Some(n) = self.as_f64() {
            write!(f, "ValueRef({})", n)
        } else if let Some(s) = self.as_str() {
            write!(f, "ValueRef({:?})", s)
        } else if self.is_sequence() {
            write!(f, "ValueRef(sequence[{}])", self.seq_len().unwrap_or(0))
        } else if self.is_mapping() {
            write!(f, "ValueRef(mapping[{}])", self.map_len().unwrap_or(0))
        } else {
            write!(f, "ValueRef(?)")
        }
    }
}

impl fmt::Display for ValueRef<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.node)
    }
}

#[cfg(test)]
mod tests {
    use crate::Document;

    // ==================== Basic Access ====================

    #[test]
    fn test_as_str() {
        let doc = Document::parse_str("key: hello").unwrap();
        let root = doc.root_value().unwrap();
        assert_eq!(root.get("key").unwrap().as_str(), Some("hello"));
    }

    #[test]
    fn test_as_bytes() {
        let doc = Document::parse_str("key: hello").unwrap();
        let root = doc.root_value().unwrap();
        assert_eq!(
            root.get("key").unwrap().as_bytes(),
            Some(b"hello".as_slice())
        );
    }

    // ==================== Boolean Tests ====================

    #[test]
    fn test_as_bool() {
        let doc = Document::parse_str("yes_val: yes\nno_val: no\ntrue_val: true").unwrap();
        let root = doc.root_value().unwrap();
        assert_eq!(root.get("yes_val").unwrap().as_bool(), Some(true));
        assert_eq!(root.get("no_val").unwrap().as_bool(), Some(false));
        assert_eq!(root.get("true_val").unwrap().as_bool(), Some(true));
    }

    #[test]
    fn test_quoted_not_interpreted() {
        let doc = Document::parse_str("quoted: 'true'\nunquoted: true").unwrap();
        let root = doc.root_value().unwrap();
        // Quoted is string, not bool
        assert_eq!(root.get("quoted").unwrap().as_bool(), None);
        assert_eq!(root.get("quoted").unwrap().as_str(), Some("true"));
        // Unquoted is bool
        assert_eq!(root.get("unquoted").unwrap().as_bool(), Some(true));
    }

    // ==================== Integer Tests ====================

    #[test]
    fn test_as_i64() {
        let doc = Document::parse_str("num: 42\nneg: -10\nhex: 0xFF").unwrap();
        let root = doc.root_value().unwrap();
        assert_eq!(root.get("num").unwrap().as_i64(), Some(42));
        assert_eq!(root.get("neg").unwrap().as_i64(), Some(-10));
        assert_eq!(root.get("hex").unwrap().as_i64(), Some(255));
    }

    #[test]
    fn test_integer_boundary_values() {
        // Note: i64::MIN (-9223372036854775808) is a special case that requires
        // direct parsing without sign-stripping. The sign-prefix parsing handles
        // most common cases but may not handle the absolute minimum.
        let doc = Document::parse_str(&format!(
            "max_i64: {}\nlarge_neg: {}\nmax_u64: {}",
            i64::MAX,
            i64::MIN + 1,
            u64::MAX
        ))
        .unwrap();
        let root = doc.root_value().unwrap();
        assert_eq!(root.get("max_i64").unwrap().as_i64(), Some(i64::MAX));
        assert_eq!(root.get("large_neg").unwrap().as_i64(), Some(i64::MIN + 1));
        assert_eq!(root.get("max_u64").unwrap().as_u64(), Some(u64::MAX));
    }

    #[test]
    fn test_integer_overflow_returns_none() {
        // Values that overflow should return None
        let doc = Document::parse_str(&format!(
            "too_big: {}0\ntoo_small: -{}0",
            i64::MAX,
            i64::MAX
        ))
        .unwrap();
        let root = doc.root_value().unwrap();
        assert_eq!(root.get("too_big").unwrap().as_i64(), None);
        assert_eq!(root.get("too_small").unwrap().as_i64(), None);
    }

    #[test]
    fn test_as_u64_rejects_negative() {
        let doc = Document::parse_str("neg: -10\npos: 42").unwrap();
        let root = doc.root_value().unwrap();
        assert_eq!(root.get("neg").unwrap().as_u64(), None);
        assert_eq!(root.get("pos").unwrap().as_u64(), Some(42));
    }

    // ==================== Float Tests ====================

    #[test]
    fn test_as_f64() {
        let doc = Document::parse_str("val: 2.5\ninf: .inf\nnan: .nan").unwrap();
        let root = doc.root_value().unwrap();
        assert!((root.get("val").unwrap().as_f64().unwrap() - 2.5).abs() < 0.01);
        assert!(root.get("inf").unwrap().as_f64().unwrap().is_infinite());
        assert!(root.get("nan").unwrap().as_f64().unwrap().is_nan());
    }

    #[test]
    fn test_as_f64_special_values() {
        let doc = Document::parse_str("pinf: +.inf\nninf: -.inf\nnan: .NaN").unwrap();
        let root = doc.root_value().unwrap();
        let pinf = root.get("pinf").unwrap().as_f64().unwrap();
        let ninf = root.get("ninf").unwrap().as_f64().unwrap();
        let nan = root.get("nan").unwrap().as_f64().unwrap();
        assert!(pinf.is_infinite() && pinf.is_sign_positive());
        assert!(ninf.is_infinite() && ninf.is_sign_negative());
        assert!(nan.is_nan());
    }

    #[test]
    fn test_as_f64_exponent() {
        let doc = Document::parse_str("exp: 1e3\nneg_exp: 1.5e-2").unwrap();
        let root = doc.root_value().unwrap();
        assert!((root.get("exp").unwrap().as_f64().unwrap() - 1000.0).abs() < 0.01);
        assert!((root.get("neg_exp").unwrap().as_f64().unwrap() - 0.015).abs() < 0.0001);
    }

    #[test]
    fn test_as_f64_from_integer() {
        // Plain integers should parse as floats too
        let doc = Document::parse_str("int: 42").unwrap();
        let root = doc.root_value().unwrap();
        assert_eq!(root.get("int").unwrap().as_f64(), Some(42.0));
    }

    // ==================== Null Tests ====================

    #[test]
    fn test_is_null() {
        let doc = Document::parse_str("null_val: null\ntilde: ~\nempty:\nstr: 'null'").unwrap();
        let root = doc.root_value().unwrap();
        assert!(root.get("null_val").unwrap().is_null());
        assert!(root.get("tilde").unwrap().is_null());
        assert!(root.get("empty").unwrap().is_null());
        // Quoted 'null' is not null
        assert!(!root.get("str").unwrap().is_null());
    }

    #[test]
    fn test_is_null_case_insensitive() {
        let doc = Document::parse_str("n1: NULL\nn2: Null\nn3: NuLl").unwrap();
        let root = doc.root_value().unwrap();
        assert!(root.get("n1").unwrap().is_null());
        assert!(root.get("n2").unwrap().is_null());
        assert!(root.get("n3").unwrap().is_null());
    }

    // ==================== Non-Plain Scalar Tests ====================

    #[test]
    fn test_literal_block_not_interpreted() {
        let doc = Document::parse_str("literal: |\n  true").unwrap();
        let root = doc.root_value().unwrap();
        // Literal block should not be interpreted as bool
        assert_eq!(root.get("literal").unwrap().as_bool(), None);
        // But should still be accessible as string
        assert!(root.get("literal").unwrap().as_str().is_some());
    }

    #[test]
    fn test_folded_block_not_interpreted() {
        let doc = Document::parse_str("folded: >\n  42").unwrap();
        let root = doc.root_value().unwrap();
        // Folded block should not be interpreted as number
        assert_eq!(root.get("folded").unwrap().as_i64(), None);
        // But should still be accessible as string
        assert!(root.get("folded").unwrap().as_str().is_some());
    }

    #[test]
    fn test_double_quoted_not_interpreted() {
        let doc = Document::parse_str("quoted: \"42\"").unwrap();
        let root = doc.root_value().unwrap();
        assert_eq!(root.get("quoted").unwrap().as_i64(), None);
        assert_eq!(root.get("quoted").unwrap().as_str(), Some("42"));
    }

    // ==================== Navigation Tests ====================

    #[test]
    fn test_seq_iter() {
        let doc = Document::parse_str("- 1\n- 2\n- 3").unwrap();
        let root = doc.root_value().unwrap();
        let sum: i64 = root.seq_iter().filter_map(|v| v.as_i64()).sum();
        assert_eq!(sum, 6);
    }

    #[test]
    fn test_map_iter() {
        let doc = Document::parse_str("a: 1\nb: 2").unwrap();
        let root = doc.root_value().unwrap();
        let keys: Vec<&str> = root.map_iter().filter_map(|(k, _)| k.as_str()).collect();
        assert_eq!(keys, vec!["a", "b"]);
    }

    #[test]
    fn test_nested_access() {
        let doc = Document::parse_str("outer:\n  inner:\n    value: 42").unwrap();
        let root = doc.root_value().unwrap();
        let value = root
            .get("outer")
            .unwrap()
            .get("inner")
            .unwrap()
            .get("value")
            .unwrap();
        assert_eq!(value.as_i64(), Some(42));
    }

    // ==================== Type Checking Tests ====================

    #[test]
    fn test_type_checks() {
        let doc = Document::parse_str("scalar: hello\nseq: [1]\nmap: {a: 1}").unwrap();
        let root = doc.root_value().unwrap();

        assert!(root.get("scalar").unwrap().is_scalar());
        assert!(!root.get("scalar").unwrap().is_sequence());
        assert!(!root.get("scalar").unwrap().is_mapping());

        assert!(root.get("seq").unwrap().is_sequence());
        assert!(!root.get("seq").unwrap().is_scalar());

        assert!(root.get("map").unwrap().is_mapping());
        assert!(!root.get("map").unwrap().is_scalar());
    }

    // ==================== Tag Tests ====================

    #[test]
    fn test_tag_access() {
        let doc = Document::parse_str("!custom tagged").unwrap();
        let root = doc.root_value().unwrap();
        assert!(root.tag().is_some());
        assert!(root.tag().unwrap().contains("custom"));
    }

    #[test]
    fn test_no_tag() {
        let doc = Document::parse_str("untagged").unwrap();
        let root = doc.root_value().unwrap();
        assert!(root.tag().is_none());
    }
}
