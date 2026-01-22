//! YAML value types with serde support.
//!
//! This module provides a pure Rust [`Value`] type that can represent any YAML value,
//! with full serde compatibility and libfyaml-powered emission.
//!
//! # Memory Model
//!
//! **`Value` is a fully-owned type that allocates memory.** When you parse YAML into
//! a `Value`, all strings, sequences, and mappings are copied into Rust-owned memory.
//!
//! For zero-copy access to YAML data, use [`ValueRef`](crate::ValueRef) or
//! [`NodeRef`](crate::NodeRef) instead. These borrow data directly from libfyaml's
//! buffers without allocation.
//!
//! | Type | Allocation | Lifetime | Serde | Use Case |
//! |------|------------|----------|-------|----------|
//! | `Value` | Yes (owns data) | `'static` | Yes | Serialize, transform, long-lived data |
//! | `ValueRef<'doc>` | No (borrows) | Tied to Document | No | Read-only, performance-critical |
//! | `NodeRef<'doc>` | No (borrows) | Tied to Document | No | Low-level access, iteration |
//!
//! # Features
//!
//! - Parse YAML into `Value` using libfyaml
//! - Serialize/deserialize with serde
//! - Emit YAML using libfyaml for standards-compliant output
//! - Order-preserving mappings via `IndexMap`
//!
//! # Example
//!
//! ```
//! use fyaml::value::Value;
//!
//! // Parse YAML
//! let value: Value = "foo: bar".parse().unwrap();
//!
//! // Access values
//! assert_eq!(value["foo"], Value::String("bar".into()));
//!
//! // Emit back to YAML
//! let yaml = value.to_yaml_string().unwrap();
//! ```

mod convert;
mod de;
mod emit;
mod ser;

use indexmap::IndexMap;
use std::cmp::Ordering;
use std::fmt;
use std::hash::{Hash, Hasher};
use std::str::FromStr;

/// A YAML value that can represent any YAML data type.
///
/// This is a pure Rust enum, making it easy to construct, manipulate,
/// and serialize. For YAML emission, it converts to libfyaml nodes.
///
/// # Allocation
///
/// `Value` owns all its data. Parsing YAML into `Value` allocates memory for
/// all strings and nested structures. For zero-copy access, use
/// [`ValueRef`](crate::ValueRef) instead.
#[derive(Clone, Debug)]
pub enum Value {
    /// Null value (YAML `null`, `~`, or empty).
    Null,
    /// Boolean value.
    Bool(bool),
    /// Numeric value (integer or float).
    Number(Number),
    /// String value.
    String(String),
    /// Sequence (list/array) of values.
    Sequence(Vec<Value>),
    /// Mapping (dictionary/object) of key-value pairs.
    /// Uses `IndexMap` to preserve insertion order.
    Mapping(IndexMap<Value, Value>),
    /// Tagged value with a custom YAML tag.
    Tagged(Box<TaggedValue>),
}

/// Numeric value that can be an integer or float.
#[derive(Clone, Debug)]
pub enum Number {
    /// Signed 64-bit integer.
    Int(i64),
    /// Unsigned 64-bit integer.
    UInt(u64),
    /// 64-bit floating point.
    Float(f64),
}

/// A value with an associated YAML tag.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TaggedValue {
    /// The tag string (e.g., "!custom" or "tag:yaml.org,2002:str").
    pub tag: String,
    /// The tagged value.
    pub value: Value,
}

impl Value {
    /// Returns `true` if the value is `Null`.
    pub fn is_null(&self) -> bool {
        matches!(self, Value::Null)
    }

    /// Returns `true` if the value is a `Bool`.
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }

    /// Returns `true` if the value is a `Number`.
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }

    /// Returns `true` if the value is a `String`.
    pub fn is_string(&self) -> bool {
        matches!(self, Value::String(_))
    }

    /// Returns `true` if the value is a `Sequence`.
    pub fn is_sequence(&self) -> bool {
        matches!(self, Value::Sequence(_))
    }

    /// Returns `true` if the value is a `Mapping`.
    pub fn is_mapping(&self) -> bool {
        matches!(self, Value::Mapping(_))
    }

    /// Returns `true` if the value is `Tagged`.
    pub fn is_tagged(&self) -> bool {
        matches!(self, Value::Tagged(_))
    }

    /// Returns the value as a `bool`, if it is one.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Value::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// Returns the value as an `i64`, if it can be represented as one.
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Number(Number::Int(n)) => Some(*n),
            Value::Number(Number::UInt(n)) => (*n).try_into().ok(),
            _ => None,
        }
    }

    /// Returns the value as a `u64`, if it can be represented as one.
    pub fn as_u64(&self) -> Option<u64> {
        match self {
            Value::Number(Number::UInt(n)) => Some(*n),
            Value::Number(Number::Int(n)) => (*n).try_into().ok(),
            _ => None,
        }
    }

    /// Returns the value as an `f64`, if it is a number.
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Number(Number::Float(f)) => Some(*f),
            Value::Number(Number::Int(n)) => Some(*n as f64),
            Value::Number(Number::UInt(n)) => Some(*n as f64),
            _ => None,
        }
    }

    /// Returns the value as a `&str`, if it is a string.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the value as a mutable `&mut String`, if it is a string.
    pub fn as_str_mut(&mut self) -> Option<&mut String> {
        match self {
            Value::String(s) => Some(s),
            _ => None,
        }
    }

    /// Returns the value as a `&[Value]`, if it is a sequence.
    pub fn as_sequence(&self) -> Option<&[Value]> {
        match self {
            Value::Sequence(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the value as a mutable `&mut Vec<Value>`, if it is a sequence.
    pub fn as_sequence_mut(&mut self) -> Option<&mut Vec<Value>> {
        match self {
            Value::Sequence(v) => Some(v),
            _ => None,
        }
    }

    /// Returns the value as a `&IndexMap<Value, Value>`, if it is a mapping.
    pub fn as_mapping(&self) -> Option<&IndexMap<Value, Value>> {
        match self {
            Value::Mapping(m) => Some(m),
            _ => None,
        }
    }

    /// Returns the value as a mutable `&mut IndexMap<Value, Value>`, if it is a mapping.
    pub fn as_mapping_mut(&mut self) -> Option<&mut IndexMap<Value, Value>> {
        match self {
            Value::Mapping(m) => Some(m),
            _ => None,
        }
    }

    /// Returns the tagged value, if this is a tagged value.
    pub fn as_tagged(&self) -> Option<&TaggedValue> {
        match self {
            Value::Tagged(t) => Some(t),
            _ => None,
        }
    }

    /// Returns a mutable reference to the tagged value, if this is a tagged value.
    pub fn as_tagged_mut(&mut self) -> Option<&mut TaggedValue> {
        match self {
            Value::Tagged(t) => Some(t),
            _ => None,
        }
    }

    /// Gets a value from a mapping by key.
    pub fn get<Q>(&self, key: &Q) -> Option<&Value>
    where
        Q: ?Sized + Hash + Eq + AsValueKey,
    {
        match self {
            Value::Mapping(m) => key.get_from_map(m),
            _ => None,
        }
    }

    /// Gets a mutable value from a mapping by key.
    pub fn get_mut<Q>(&mut self, key: &Q) -> Option<&mut Value>
    where
        Q: ?Sized + Hash + Eq + AsValueKey,
    {
        match self {
            Value::Mapping(m) => key.get_from_map_mut(m),
            _ => None,
        }
    }
}

/// Trait for types that can be used as keys to look up values in a mapping.
pub trait AsValueKey {
    fn get_from_map<'a>(&self, map: &'a IndexMap<Value, Value>) -> Option<&'a Value>;
    fn get_from_map_mut<'a>(&self, map: &'a mut IndexMap<Value, Value>) -> Option<&'a mut Value>;
}

impl AsValueKey for str {
    fn get_from_map<'a>(&self, map: &'a IndexMap<Value, Value>) -> Option<&'a Value> {
        // Zero-copy lookup: iterate and compare without allocating
        for (k, v) in map {
            if let Value::String(s) = k {
                if s == self {
                    return Some(v);
                }
            }
        }
        None
    }
    fn get_from_map_mut<'a>(&self, map: &'a mut IndexMap<Value, Value>) -> Option<&'a mut Value> {
        // Zero-copy lookup: iterate and compare without allocating
        for (k, v) in map {
            if let Value::String(s) = k {
                if s == self {
                    return Some(v);
                }
            }
        }
        None
    }
}

impl AsValueKey for String {
    fn get_from_map<'a>(&self, map: &'a IndexMap<Value, Value>) -> Option<&'a Value> {
        // Delegate to str implementation (zero-copy)
        self.as_str().get_from_map(map)
    }
    fn get_from_map_mut<'a>(&self, map: &'a mut IndexMap<Value, Value>) -> Option<&'a mut Value> {
        // Delegate to str implementation (zero-copy)
        self.as_str().get_from_map_mut(map)
    }
}

impl AsValueKey for Value {
    fn get_from_map<'a>(&self, map: &'a IndexMap<Value, Value>) -> Option<&'a Value> {
        map.get(self)
    }
    fn get_from_map_mut<'a>(&self, map: &'a mut IndexMap<Value, Value>) -> Option<&'a mut Value> {
        map.get_mut(self)
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Sequence(a), Value::Sequence(b)) => a == b,
            (Value::Mapping(a), Value::Mapping(b)) => a == b,
            (Value::Tagged(a), Value::Tagged(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Value {}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Value {
    fn cmp(&self, other: &Self) -> Ordering {
        // Order by type first, then by value
        fn type_order(v: &Value) -> u8 {
            match v {
                Value::Null => 0,
                Value::Bool(_) => 1,
                Value::Number(_) => 2,
                Value::String(_) => 3,
                Value::Sequence(_) => 4,
                Value::Mapping(_) => 5,
                Value::Tagged(_) => 6,
            }
        }

        let type_cmp = type_order(self).cmp(&type_order(other));
        if type_cmp != Ordering::Equal {
            return type_cmp;
        }

        match (self, other) {
            (Value::Null, Value::Null) => Ordering::Equal,
            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            (Value::Number(a), Value::Number(b)) => a.cmp(b),
            (Value::String(a), Value::String(b)) => a.cmp(b),
            (Value::Sequence(a), Value::Sequence(b)) => a.cmp(b),
            (Value::Mapping(a), Value::Mapping(b)) => {
                // Compare mappings by their entries
                let a_entries: Vec<_> = a.iter().collect();
                let b_entries: Vec<_> = b.iter().collect();
                a_entries.cmp(&b_entries)
            }
            (Value::Tagged(a), Value::Tagged(b)) => a.cmp(b),
            _ => Ordering::Equal, // Same type_order but different types shouldn't happen
        }
    }
}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Value::Null => {}
            Value::Bool(b) => b.hash(state),
            Value::Number(n) => n.hash(state),
            Value::String(s) => s.hash(state),
            Value::Sequence(v) => v.hash(state),
            Value::Mapping(m) => {
                // Hash the entries in order (IndexMap preserves order)
                for (k, v) in m {
                    k.hash(state);
                    v.hash(state);
                }
            }
            Value::Tagged(t) => t.hash(state),
        }
    }
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Number::Int(a), Number::Int(b)) => a == b,
            (Number::UInt(a), Number::UInt(b)) => a == b,
            (Number::Float(a), Number::Float(b)) => a.to_bits() == b.to_bits(),
            (Number::Int(a), Number::UInt(b)) => {
                if *a >= 0 {
                    (*a as u64) == *b
                } else {
                    false
                }
            }
            (Number::UInt(a), Number::Int(b)) => {
                if *b >= 0 {
                    *a == (*b as u64)
                } else {
                    false
                }
            }
            (Number::Int(a), Number::Float(b)) => (*a as f64).to_bits() == b.to_bits(),
            (Number::Float(a), Number::Int(b)) => a.to_bits() == (*b as f64).to_bits(),
            (Number::UInt(a), Number::Float(b)) => (*a as f64).to_bits() == b.to_bits(),
            (Number::Float(a), Number::UInt(b)) => a.to_bits() == (*b as f64).to_bits(),
        }
    }
}

impl Eq for Number {}

impl PartialOrd for Number {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Number {
    fn cmp(&self, other: &Self) -> Ordering {
        // Convert to f64 for comparison, using total_cmp for proper NaN handling
        let a = match self {
            Number::Int(n) => *n as f64,
            Number::UInt(n) => *n as f64,
            Number::Float(f) => *f,
        };
        let b = match other {
            Number::Int(n) => *n as f64,
            Number::UInt(n) => *n as f64,
            Number::Float(f) => *f,
        };
        a.total_cmp(&b)
    }
}

impl Hash for Number {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash based on the numeric value, normalized to bits for consistency
        match self {
            Number::Int(n) => {
                0u8.hash(state);
                n.hash(state);
            }
            Number::UInt(n) => {
                1u8.hash(state);
                n.hash(state);
            }
            Number::Float(f) => {
                2u8.hash(state);
                f.to_bits().hash(state);
            }
        }
    }
}

impl PartialOrd for TaggedValue {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TaggedValue {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.tag.cmp(&other.tag) {
            Ordering::Equal => self.value.cmp(&other.value),
            other => other,
        }
    }
}

// Indexing support
impl std::ops::Index<&str> for Value {
    type Output = Value;

    fn index(&self, key: &str) -> &Self::Output {
        static NULL: Value = Value::Null;
        self.get(key).unwrap_or(&NULL)
    }
}

impl std::ops::Index<usize> for Value {
    type Output = Value;

    fn index(&self, index: usize) -> &Self::Output {
        static NULL: Value = Value::Null;
        match self {
            Value::Sequence(v) => v.get(index).unwrap_or(&NULL),
            _ => &NULL,
        }
    }
}

impl FromStr for Value {
    type Err = crate::error::Error;

    fn from_str(s: &str) -> crate::error::Result<Self> {
        use crate::Document;
        let doc = Document::parse_str(s)?;
        let root = doc
            .root()
            .ok_or(crate::error::Error::Parse("empty document"))?;
        Value::from_node_ref(root)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.to_yaml_string() {
            Ok(s) => write!(f, "{}", s),
            Err(e) => write!(f, "<error: {}>", e),
        }
    }
}

// Convenient From implementations
impl From<bool> for Value {
    fn from(b: bool) -> Self {
        Value::Bool(b)
    }
}

impl From<i64> for Value {
    fn from(n: i64) -> Self {
        Value::Number(Number::Int(n))
    }
}

impl From<i32> for Value {
    fn from(n: i32) -> Self {
        Value::Number(Number::Int(n as i64))
    }
}

impl From<u64> for Value {
    fn from(n: u64) -> Self {
        Value::Number(Number::UInt(n))
    }
}

impl From<u32> for Value {
    fn from(n: u32) -> Self {
        Value::Number(Number::UInt(n as u64))
    }
}

impl From<f64> for Value {
    fn from(f: f64) -> Self {
        Value::Number(Number::Float(f))
    }
}

impl From<f32> for Value {
    fn from(f: f32) -> Self {
        Value::Number(Number::Float(f as f64))
    }
}

impl From<String> for Value {
    fn from(s: String) -> Self {
        Value::String(s)
    }
}

impl From<&str> for Value {
    fn from(s: &str) -> Self {
        Value::String(s.to_string())
    }
}

impl<T: Into<Value>> From<Vec<T>> for Value {
    fn from(v: Vec<T>) -> Self {
        Value::Sequence(v.into_iter().map(Into::into).collect())
    }
}

impl<T: Into<Value>> From<Option<T>> for Value {
    fn from(opt: Option<T>) -> Self {
        match opt {
            Some(v) => v.into(),
            None => Value::Null,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_value_type_checks() {
        assert!(Value::Null.is_null());
        assert!(Value::Bool(true).is_bool());
        assert!(Value::Number(Number::Int(42)).is_number());
        assert!(Value::String("hello".into()).is_string());
        assert!(Value::Sequence(vec![]).is_sequence());
        assert!(Value::Mapping(IndexMap::new()).is_mapping());
    }

    #[test]
    fn test_value_accessors() {
        assert_eq!(Value::Bool(true).as_bool(), Some(true));
        assert_eq!(Value::Number(Number::Int(42)).as_i64(), Some(42));
        assert_eq!(Value::Number(Number::UInt(42)).as_u64(), Some(42));
        assert_eq!(Value::Number(Number::Float(2.5)).as_f64(), Some(2.5));
        assert_eq!(Value::String("hello".into()).as_str(), Some("hello"));
    }

    #[test]
    fn test_value_equality() {
        assert_eq!(Value::Null, Value::Null);
        assert_eq!(Value::Bool(true), Value::Bool(true));
        assert_ne!(Value::Bool(true), Value::Bool(false));
        assert_eq!(Value::String("a".into()), Value::String("a".into()));
    }

    #[test]
    fn test_value_hash() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Value::String("key".into()));
        assert!(set.contains(&Value::String("key".into())));
    }

    #[test]
    fn test_value_indexing() {
        let mut map = IndexMap::new();
        map.insert(Value::String("foo".into()), Value::String("bar".into()));
        let value = Value::Mapping(map);
        assert_eq!(value["foo"], Value::String("bar".into()));
        assert_eq!(value["missing"], Value::Null);
    }

    #[test]
    fn test_sequence_indexing() {
        let value = Value::Sequence(vec![Value::from(1), Value::from(2), Value::from(3)]);
        assert_eq!(value[0], Value::Number(Number::Int(1)));
        assert_eq!(value[1], Value::Number(Number::Int(2)));
        assert_eq!(value[10], Value::Null);
    }

    #[test]
    fn test_from_impls() {
        assert_eq!(Value::from(true), Value::Bool(true));
        assert_eq!(Value::from(42i64), Value::Number(Number::Int(42)));
        assert_eq!(Value::from(42u64), Value::Number(Number::UInt(42)));
        assert_eq!(Value::from(2.5f64), Value::Number(Number::Float(2.5)));
        assert_eq!(Value::from("hello"), Value::String("hello".into()));
    }
}
