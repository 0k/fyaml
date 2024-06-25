//! Conversions between Node and Value types.

use super::{Number, TaggedValue, Value};
use crate::node::{Node, NodeType};
use indexmap::IndexMap;

impl Value {
    /// Creates a Value from a Node.
    ///
    /// This walks the Node tree recursively and converts it to a pure Rust Value.
    /// Scalar type inference (null, bool, number, string) is performed during conversion.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::node::Node;
    /// use fyaml::value::Value;
    /// use std::str::FromStr;
    ///
    /// let node = Node::from_str("foo: 42").unwrap();
    /// let value = Value::from_node(&node).unwrap();
    /// assert!(value.is_mapping());
    /// ```
    pub fn from_node(node: &Node) -> Result<Value, String> {
        Self::from_node_inner(node)
    }

    fn from_node_inner(node: &Node) -> Result<Value, String> {
        // Check for tag first
        let tag = node.get_tag()?;

        let value = match node.get_type() {
            NodeType::Scalar => {
                let raw = node.to_raw_string()?;
                infer_scalar_type(&raw)
            }
            NodeType::Sequence => {
                let mut items = Vec::new();
                for item_result in node.seq_iter() {
                    let item = item_result?;
                    items.push(Self::from_node_inner(&item)?);
                }
                Value::Sequence(items)
            }
            NodeType::Mapping => {
                let mut map = IndexMap::new();
                for pair_result in node.map_iter() {
                    let (key_node, value_node) = pair_result?;
                    let key = Self::from_node_inner(&key_node)?;
                    let value = Self::from_node_inner(&value_node)?;
                    map.insert(key, value);
                }
                Value::Mapping(map)
            }
        };

        // Wrap with tag if present
        match tag {
            Some(t) => Ok(Value::Tagged(Box::new(TaggedValue { tag: t, value }))),
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
    if is_null(s) {
        return Value::Null;
    }

    // Check for boolean
    if let Some(b) = parse_bool(s) {
        return Value::Bool(b);
    }

    // Check for integer (including hex, octal, binary)
    if let Some(n) = parse_integer(s) {
        return Value::Number(n);
    }

    // Check for float (including special values)
    if let Some(n) = parse_float(s) {
        return Value::Number(n);
    }

    // Default to string
    Value::String(s.to_string())
}

fn is_null(s: &str) -> bool {
    matches!(
        s.to_lowercase().as_str(),
        "" | "~" | "null" | "Null" | "NULL"
    ) || s == "~"
}

fn parse_bool(s: &str) -> Option<bool> {
    match s {
        "true" | "True" | "TRUE" | "yes" | "Yes" | "YES" | "on" | "On" | "ON" => Some(true),
        "false" | "False" | "FALSE" | "no" | "No" | "NO" | "off" | "Off" | "OFF" => Some(false),
        _ => None,
    }
}

fn parse_integer(s: &str) -> Option<Number> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // Handle sign
    let (neg, s) = if let Some(rest) = s.strip_prefix('-') {
        (true, rest)
    } else if let Some(rest) = s.strip_prefix('+') {
        (false, rest)
    } else {
        (false, s)
    };

    // Try different bases
    let result = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        i64::from_str_radix(hex, 16).ok()
    } else if let Some(oct) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
        i64::from_str_radix(oct, 8).ok()
    } else if let Some(bin) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        i64::from_str_radix(bin, 2).ok()
    } else {
        s.parse::<i64>().ok()
    };

    result.map(|n| {
        let n = if neg { -n } else { n };
        if n >= 0 {
            Number::UInt(n as u64)
        } else {
            Number::Int(n)
        }
    })
}

fn parse_float(s: &str) -> Option<Number> {
    let s_lower = s.to_lowercase();

    // Special float values
    match s_lower.as_str() {
        ".inf" | "+.inf" => return Some(Number::Float(f64::INFINITY)),
        "-.inf" => return Some(Number::Float(f64::NEG_INFINITY)),
        ".nan" => return Some(Number::Float(f64::NAN)),
        _ => {}
    }

    // Regular float
    // Must contain a decimal point or exponent to be considered a float
    if s.contains('.') || s.to_lowercase().contains('e') {
        if let Ok(f) = s.parse::<f64>() {
            return Some(Number::Float(f));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node::Node;
    use std::str::FromStr;

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
        assert_eq!(
            infer_scalar_type("3.14"),
            Value::Number(Number::Float(3.14))
        );
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
    fn test_from_node_scalar() {
        let node = Node::from_str("42").unwrap();
        let value = Value::from_node(&node).unwrap();
        assert_eq!(value, Value::Number(Number::UInt(42)));
    }

    #[test]
    fn test_from_node_sequence() {
        let node = Node::from_str("[1, 2, 3]").unwrap();
        let value = Value::from_node(&node).unwrap();
        assert!(value.is_sequence());
        let seq = value.as_sequence().unwrap();
        assert_eq!(seq.len(), 3);
    }

    #[test]
    fn test_from_node_mapping() {
        let node = Node::from_str("foo: bar").unwrap();
        let value = Value::from_node(&node).unwrap();
        assert!(value.is_mapping());
        assert_eq!(value["foo"], Value::String("bar".into()));
    }

    #[test]
    fn test_value_parse() {
        let value: Value = "key: value".parse().unwrap();
        assert!(value.is_mapping());
        assert_eq!(value["key"], Value::String("value".into()));
    }
}
