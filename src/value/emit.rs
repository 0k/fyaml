//! YAML emission for Value using libfyaml.
//!
//! Converts owned `Value` trees to YAML strings via the safe `Editor` API.
//! No direct FFI calls — all node building goes through `Editor` methods.

use super::{Number, TaggedValue, Value};
use crate::editor::{Editor, RawNodeHandle};
use crate::error::Result;
use crate::Document;

impl Value {
    /// Emits this value as a YAML string using libfyaml.
    ///
    /// This provides standards-compliant YAML output with proper quoting,
    /// escaping, and formatting. The output does **not** include a trailing
    /// newline — this is a value-level representation, not a document.
    ///
    /// # Example
    ///
    /// ```
    /// use fyaml::value::Value;
    /// use indexmap::IndexMap;
    ///
    /// let mut map = IndexMap::new();
    /// map.insert(Value::String("key".into()), Value::String("value".into()));
    /// let value = Value::Mapping(map);
    ///
    /// let yaml = value.to_yaml_string().unwrap();
    /// assert!(yaml.contains("key: value"));
    /// ```
    pub fn to_yaml_string(&self) -> Result<String> {
        let mut doc = Document::new()?;
        {
            let mut ed = doc.edit();
            let root = self.build_node(&mut ed)?;
            ed.set_root(root)?;
        }
        doc.root()
            .ok_or(crate::error::Error::Ffi("document has no root"))?
            .emit()
    }

    /// Recursively builds a libfyaml node tree from this Value using the Editor API.
    fn build_node(&self, ed: &mut Editor<'_>) -> Result<RawNodeHandle> {
        match self {
            Value::Null => ed.build_null(),
            Value::Bool(b) => {
                let s = if *b { "true" } else { "false" };
                ed.build_scalar(s)
            }
            Value::Number(n) => {
                let s = match n {
                    Number::Int(i) => i.to_string(),
                    Number::UInt(u) => u.to_string(),
                    Number::Float(f) => {
                        if f.is_nan() {
                            ".nan".to_string()
                        } else if f.is_infinite() {
                            if f.is_sign_positive() {
                                ".inf".to_string()
                            } else {
                                "-.inf".to_string()
                            }
                        } else {
                            format!("{}", f)
                        }
                    }
                };
                ed.build_scalar(&s)
            }
            Value::String(s) => {
                if crate::scalar_parse::needs_quoting(s) {
                    let mut node = ed.build_scalar(s)?;
                    ed.set_style(&mut node, crate::node::NodeStyle::SingleQuoted);
                    Ok(node)
                } else {
                    ed.build_scalar(s)
                }
            }
            Value::Sequence(items) => {
                let mut seq = ed.build_sequence()?;
                for item in items {
                    let child = item.build_node(ed)?;
                    ed.seq_append(&mut seq, child)?;
                }
                Ok(seq)
            }
            Value::Mapping(map) => {
                let mut m = ed.build_mapping()?;
                for (k, v) in map {
                    let key = k.build_node(ed)?;
                    let val = v.build_node(ed)?;
                    ed.map_insert(&mut m, key, val)?;
                }
                Ok(m)
            }
            Value::Tagged(tagged) => {
                let mut node = tagged.value.build_node(ed)?;
                ed.set_tag(&mut node, &tagged.tag)?;
                Ok(node)
            }
        }
    }
}

impl TaggedValue {
    /// Emits this tagged value as a YAML string.
    pub fn to_yaml_string(&self) -> Result<String> {
        Value::Tagged(Box::new(self.clone())).to_yaml_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use indexmap::IndexMap;

    #[test]
    fn test_emit_null() {
        let value = Value::Null;
        let yaml = value.to_yaml_string().unwrap();
        assert_eq!(yaml, "null");
    }

    #[test]
    fn test_emit_bool() {
        let value = Value::Bool(true);
        let yaml = value.to_yaml_string().unwrap();
        assert!(yaml.contains("true"));

        let value = Value::Bool(false);
        let yaml = value.to_yaml_string().unwrap();
        assert!(yaml.contains("false"));
    }

    #[test]
    fn test_emit_number() {
        let value = Value::Number(Number::Int(42));
        let yaml = value.to_yaml_string().unwrap();
        assert!(yaml.contains("42"));

        let value = Value::Number(Number::Float(2.5));
        let yaml = value.to_yaml_string().unwrap();
        assert!(yaml.contains("2.5"));
    }

    #[test]
    fn test_emit_string() {
        let value = Value::String("hello world".into());
        let yaml = value.to_yaml_string().unwrap();
        assert!(yaml.contains("hello world"));
    }

    #[test]
    fn test_emit_string_ambiguous_bool() {
        assert_eq!(
            Value::String("true".into()).to_yaml_string().unwrap(),
            "'true'"
        );
        assert_eq!(
            Value::String("false".into()).to_yaml_string().unwrap(),
            "'false'"
        );
        assert_eq!(
            Value::String("yes".into()).to_yaml_string().unwrap(),
            "'yes'"
        );
        assert_eq!(Value::String("no".into()).to_yaml_string().unwrap(), "'no'");
        assert_eq!(Value::String("on".into()).to_yaml_string().unwrap(), "'on'");
        assert_eq!(
            Value::String("off".into()).to_yaml_string().unwrap(),
            "'off'"
        );
        assert_eq!(
            Value::String("True".into()).to_yaml_string().unwrap(),
            "'True'"
        );
        assert_eq!(
            Value::String("FALSE".into()).to_yaml_string().unwrap(),
            "'FALSE'"
        );
    }

    #[test]
    fn test_emit_string_ambiguous_null() {
        assert_eq!(
            Value::String("null".into()).to_yaml_string().unwrap(),
            "'null'"
        );
        assert_eq!(Value::String("~".into()).to_yaml_string().unwrap(), "'~'");
        assert_eq!(
            Value::String("Null".into()).to_yaml_string().unwrap(),
            "'Null'"
        );
        assert_eq!(
            Value::String("NULL".into()).to_yaml_string().unwrap(),
            "'NULL'"
        );
    }

    #[test]
    fn test_emit_string_ambiguous_number() {
        assert_eq!(Value::String("42".into()).to_yaml_string().unwrap(), "'42'");
        assert_eq!(
            Value::String("3.14".into()).to_yaml_string().unwrap(),
            "'3.14'"
        );
        assert_eq!(
            Value::String("0xFF".into()).to_yaml_string().unwrap(),
            "'0xFF'"
        );
        assert_eq!(
            Value::String(".inf".into()).to_yaml_string().unwrap(),
            "'.inf'"
        );
        assert_eq!(
            Value::String(".nan".into()).to_yaml_string().unwrap(),
            "'.nan'"
        );
    }

    #[test]
    fn test_emit_string_not_ambiguous() {
        assert_eq!(
            Value::String("hello".into()).to_yaml_string().unwrap(),
            "hello"
        );
        assert_eq!(
            Value::String("hello world".into())
                .to_yaml_string()
                .unwrap(),
            "hello world"
        );
    }

    #[test]
    fn test_emit_sequence() {
        let value = Value::Sequence(vec![
            Value::Number(Number::Int(1)),
            Value::Number(Number::Int(2)),
            Value::Number(Number::Int(3)),
        ]);
        let yaml = value.to_yaml_string().unwrap();
        assert!(yaml.contains("1"));
        assert!(yaml.contains("2"));
        assert!(yaml.contains("3"));
    }

    #[test]
    fn test_emit_mapping() {
        let mut map = IndexMap::new();
        map.insert(Value::String("key".into()), Value::String("value".into()));
        let value = Value::Mapping(map);
        let yaml = value.to_yaml_string().unwrap();
        assert!(yaml.contains("key"));
        assert!(yaml.contains("value"));
    }

    #[test]
    fn test_emit_nested() {
        let mut inner = IndexMap::new();
        inner.insert(Value::String("name".into()), Value::String("test".into()));
        inner.insert(Value::String("count".into()), Value::Number(Number::Int(5)));

        let mut outer = IndexMap::new();
        outer.insert(Value::String("item".into()), Value::Mapping(inner));

        let value = Value::Mapping(outer);
        let yaml = value.to_yaml_string().unwrap();
        assert!(yaml.contains("item"));
        assert!(yaml.contains("name"));
        assert!(yaml.contains("test"));
        assert!(yaml.contains("count"));
        assert!(yaml.contains("5"));
    }
}
