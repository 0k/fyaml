//! Conversions between NodeRef and Value types.

use super::{TaggedValue, Value};
use crate::error::Result;
use crate::node::NodeType;
use crate::scalar_parse;
use crate::NodeRef;
use indexmap::IndexMap;

impl Value {
    /// Creates a Value from a NodeRef.
    ///
    /// This walks the NodeRef tree recursively and converts it to a pure Rust Value.
    /// Uses capacity pre-allocation for sequences and mappings based on their known lengths.
    /// Scalar type inference (null, bool, number, string) is performed during conversion.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::Document;
    /// use fyaml::Value;
    ///
    /// let doc = Document::parse_str("foo: 42").unwrap();
    /// let root = doc.root().unwrap();
    /// let value = Value::from_node_ref(root).unwrap();
    /// assert!(value.is_mapping());
    /// ```
    pub fn from_node_ref(node: NodeRef<'_>) -> Result<Value> {
        Self::from_node_ref_inner(node)
    }

    fn from_node_ref_inner(node: NodeRef<'_>) -> Result<Value> {
        let tag = node.tag_str()?;

        let value = match node.kind() {
            NodeType::Scalar => {
                let raw = node.scalar_str()?;
                // Non-plain scalars (quoted, literal, folded) should not be type-inferred
                if node.is_non_plain() {
                    Value::String(raw.to_string())
                } else {
                    infer_scalar_type(raw)
                }
            }
            NodeType::Sequence => {
                // Pre-allocate with known capacity
                let len = node.seq_len().unwrap_or(0);
                let mut items = Vec::with_capacity(len);
                for item in node.seq_iter() {
                    items.push(Self::from_node_ref_inner(item)?);
                }
                Value::Sequence(items)
            }
            NodeType::Mapping => {
                // Pre-allocate with known capacity
                let len = node.map_len().unwrap_or(0);
                let mut map = IndexMap::with_capacity(len);
                for (key_node, value_node) in node.map_iter() {
                    let key = Self::from_node_ref_inner(key_node)?;
                    let value = Self::from_node_ref_inner(value_node)?;
                    map.insert(key, value);
                }
                Value::Mapping(map)
            }
        };

        // Wrap with tag if present
        match tag {
            Some(t) => Ok(Value::Tagged(Box::new(TaggedValue {
                tag: t.to_string(),
                value,
            }))),
            None => Ok(value),
        }
    }
}

/// Infers the type of a YAML scalar value.
///
/// YAML scalars can represent null, bool, numbers, or strings.
/// This follows YAML 1.1/1.2 core schema conventions.
fn infer_scalar_type(s: &str) -> Value {
    // Check for null
    if scalar_parse::is_null(s) {
        return Value::Null;
    }

    // Check for boolean
    if let Some(b) = scalar_parse::parse_bool(s) {
        return Value::Bool(b);
    }

    // Check for number (int or float)
    if let Some(n) = scalar_parse::parse_number(s) {
        return Value::Number(n);
    }

    // Default to string
    Value::String(s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::value::Number;
    use crate::Document;

    #[test]
    fn test_infer_null() {
        assert_eq!(infer_scalar_type(""), Value::Null);
        assert_eq!(infer_scalar_type("~"), Value::Null);
        assert_eq!(infer_scalar_type("null"), Value::Null);
        assert_eq!(infer_scalar_type("NULL"), Value::Null);
    }

    #[test]
    fn test_infer_bool() {
        assert_eq!(infer_scalar_type("true"), Value::Bool(true));
        assert_eq!(infer_scalar_type("True"), Value::Bool(true));
        assert_eq!(infer_scalar_type("yes"), Value::Bool(true));
        assert_eq!(infer_scalar_type("false"), Value::Bool(false));
        assert_eq!(infer_scalar_type("False"), Value::Bool(false));
        assert_eq!(infer_scalar_type("no"), Value::Bool(false));
    }

    #[test]
    fn test_infer_integer() {
        assert_eq!(infer_scalar_type("42"), Value::Number(Number::UInt(42)));
        assert_eq!(infer_scalar_type("-42"), Value::Number(Number::Int(-42)));
        assert_eq!(infer_scalar_type("0xFF"), Value::Number(Number::UInt(255)));
        assert_eq!(infer_scalar_type("0o77"), Value::Number(Number::UInt(63)));
    }

    #[test]
    fn test_infer_float() {
        assert_eq!(infer_scalar_type("2.5"), Value::Number(Number::Float(2.5)));
        assert_eq!(
            infer_scalar_type("1.0e10"),
            Value::Number(Number::Float(1.0e10))
        );
    }

    #[test]
    fn test_infer_special_floats() {
        match infer_scalar_type(".inf") {
            Value::Number(Number::Float(f)) => assert!(f.is_infinite() && f.is_sign_positive()),
            _ => panic!("Expected positive infinity"),
        }
        match infer_scalar_type("-.inf") {
            Value::Number(Number::Float(f)) => assert!(f.is_infinite() && f.is_sign_negative()),
            _ => panic!("Expected negative infinity"),
        }
        match infer_scalar_type(".nan") {
            Value::Number(Number::Float(f)) => assert!(f.is_nan()),
            _ => panic!("Expected NaN"),
        }
    }

    #[test]
    fn test_infer_string() {
        assert_eq!(infer_scalar_type("hello"), Value::String("hello".into()));
        assert_eq!(
            infer_scalar_type("hello world"),
            Value::String("hello world".into())
        );
    }

    #[test]
    fn test_from_node_ref_scalar() {
        let doc = Document::parse_str("42").unwrap();
        let root = doc.root().unwrap();
        let value = Value::from_node_ref(root).unwrap();
        assert_eq!(value, Value::Number(Number::UInt(42)));
    }

    #[test]
    fn test_from_node_ref_sequence() {
        let doc = Document::parse_str("[1, 2, 3]").unwrap();
        let root = doc.root().unwrap();
        let value = Value::from_node_ref(root).unwrap();
        assert!(value.is_sequence());
        let seq = value.as_sequence().unwrap();
        assert_eq!(seq.len(), 3);
    }

    #[test]
    fn test_from_node_ref_mapping() {
        let doc = Document::parse_str("foo: bar").unwrap();
        let root = doc.root().unwrap();
        let value = Value::from_node_ref(root).unwrap();
        assert!(value.is_mapping());
        assert_eq!(value["foo"], Value::String("bar".into()));
    }

    #[test]
    fn test_from_node_ref_nested() {
        let doc = Document::parse_str("users:\n  - name: Alice\n  - name: Bob").unwrap();
        let root = doc.root().unwrap();
        let value = Value::from_node_ref(root).unwrap();
        assert!(value.is_mapping());
        let users = value["users"].as_sequence().unwrap();
        assert_eq!(users.len(), 2);
        assert_eq!(users[0]["name"], Value::String("Alice".into()));
        assert_eq!(users[1]["name"], Value::String("Bob".into()));
    }

    #[test]
    fn test_from_node_ref_quoted_string() {
        let doc = Document::parse_str("quoted: 'true'").unwrap();
        let root = doc.root().unwrap();
        let value = Value::from_node_ref(root).unwrap();
        // Quoted 'true' should be a string, not a bool
        assert_eq!(value["quoted"], Value::String("true".into()));
    }

    #[test]
    fn test_from_node_ref_type_inference() {
        let doc = Document::parse_str("bool: true\nnum: 42\nfloat: 2.5\nnull: ~").unwrap();
        let root = doc.root().unwrap();
        let value = Value::from_node_ref(root).unwrap();
        assert_eq!(value["bool"], Value::Bool(true));
        assert_eq!(value["num"], Value::Number(Number::UInt(42)));
        assert_eq!(value["float"], Value::Number(Number::Float(2.5)));
        assert_eq!(value["null"], Value::Null);
    }

    #[test]
    fn test_value_parse() {
        let value: Value = "key: value".parse().unwrap();
        assert!(value.is_mapping());
        assert_eq!(value["key"], Value::String("value".into()));
    }
}
