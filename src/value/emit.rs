//! YAML emission for Value using libfyaml.

use super::{Number, TaggedValue, Value};
use crate::error::{Error, Result};
use crate::Document;
use fyaml_sys::*;
use libc::c_void;
use std::ffi::CStr;
use std::ptr;

impl Value {
    /// Emits this value as a YAML string using libfyaml.
    ///
    /// This provides standards-compliant YAML output with proper quoting,
    /// escaping, and formatting.
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
        // Create a new document
        let doc = Document::new()?;

        // Convert value to libfyaml node
        let node_ptr = self.to_fy_node(doc.as_ptr())?;

        // Set as document root
        let ret = unsafe { fy_document_set_root(doc.as_ptr(), node_ptr) };
        if ret != 0 {
            return Err(Error::Ffi("fy_document_set_root failed"));
        }

        // Emit to string
        let ptr = unsafe { fy_emit_document_to_string(doc.as_ptr(), FYECF_MODE_DEJSON) };
        if ptr.is_null() {
            return Err(Error::Ffi("fy_emit_document_to_string returned null"));
        }

        let s = unsafe { CStr::from_ptr(ptr) }
            .to_string_lossy()
            .into_owned();
        unsafe { libc::free(ptr as *mut c_void) };

        Ok(s)
    }

    /// Converts this Value to a libfyaml node.
    ///
    /// The node is owned by the document and will be freed when the document is dropped.
    fn to_fy_node(&self, doc_ptr: *mut fy_document) -> Result<*mut fy_node> {
        match self {
            Value::Null => {
                let node_ptr = unsafe { fy_node_create_scalar_copy(doc_ptr, ptr::null(), 0) };
                if node_ptr.is_null() {
                    return Err(Error::Ffi("fy_node_create_scalar_copy returned null"));
                }
                Ok(node_ptr)
            }
            Value::Bool(b) => {
                let s = if *b { "true" } else { "false" };
                let node_ptr = unsafe {
                    fy_node_create_scalar_copy(doc_ptr, s.as_ptr() as *const i8, s.len())
                };
                if node_ptr.is_null() {
                    return Err(Error::Ffi("fy_node_create_scalar_copy returned null"));
                }
                Ok(node_ptr)
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
                let node_ptr = unsafe {
                    fy_node_create_scalar_copy(doc_ptr, s.as_ptr() as *const i8, s.len())
                };
                if node_ptr.is_null() {
                    return Err(Error::Ffi("fy_node_create_scalar_copy returned null"));
                }
                Ok(node_ptr)
            }
            Value::String(s) => {
                let node_ptr = unsafe {
                    fy_node_create_scalar_copy(doc_ptr, s.as_ptr() as *const i8, s.len())
                };
                if node_ptr.is_null() {
                    return Err(Error::Ffi("fy_node_create_scalar_copy returned null"));
                }
                Ok(node_ptr)
            }
            Value::Sequence(seq) => {
                let seq_ptr = unsafe { fy_node_create_sequence(doc_ptr) };
                if seq_ptr.is_null() {
                    return Err(Error::Ffi("fy_node_create_sequence returned null"));
                }
                for item in seq {
                    let item_ptr = item.to_fy_node(doc_ptr)?;
                    let ret = unsafe { fy_node_sequence_append(seq_ptr, item_ptr) };
                    if ret != 0 {
                        return Err(Error::Ffi("fy_node_sequence_append failed"));
                    }
                }
                Ok(seq_ptr)
            }
            Value::Mapping(map) => {
                let map_ptr = unsafe { fy_node_create_mapping(doc_ptr) };
                if map_ptr.is_null() {
                    return Err(Error::Ffi("fy_node_create_mapping returned null"));
                }
                for (key, value) in map {
                    let key_ptr = key.to_fy_node(doc_ptr)?;
                    let value_ptr = value.to_fy_node(doc_ptr)?;
                    let ret = unsafe { fy_node_mapping_append(map_ptr, key_ptr, value_ptr) };
                    if ret != 0 {
                        return Err(Error::Ffi("fy_node_mapping_append failed"));
                    }
                }
                Ok(map_ptr)
            }
            Value::Tagged(tagged) => {
                // First create the inner value node
                let value_ptr = tagged.value.to_fy_node(doc_ptr)?;
                // Then set the tag on it
                let tag = &tagged.tag;
                let ret =
                    unsafe { fy_node_set_tag(value_ptr, tag.as_ptr() as *const i8, tag.len()) };
                if ret != 0 {
                    return Err(Error::Ffi("fy_node_set_tag failed"));
                }
                Ok(value_ptr)
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
        assert!(yaml.trim().is_empty() || yaml.contains("null") || yaml.contains("~"));
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
