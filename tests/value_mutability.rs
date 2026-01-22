//! Value type mutability and ordering tests.
//!
//! Tests for mutable accessors, ordering comparisons, and hash implementations.

use fyaml::value::{Number, TaggedValue, Value};
use indexmap::IndexMap;
use std::collections::HashSet;

// =============================================================================
// Mutable Accessors
// =============================================================================

#[test]
fn value_as_str_mut() {
    let mut value = Value::String("hello".into());
    if let Some(s) = value.as_str_mut() {
        s.push_str(" world");
    }
    assert_eq!(value.as_str(), Some("hello world"));
}

#[test]
fn value_as_str_mut_on_non_string() {
    let mut value = Value::Bool(true);
    assert!(value.as_str_mut().is_none());
}

#[test]
fn value_as_sequence_mut() {
    let mut value = Value::Sequence(vec![Value::from(1)]);
    if let Some(seq) = value.as_sequence_mut() {
        seq.push(Value::from(2));
        seq.push(Value::from(3));
    }
    assert_eq!(value.as_sequence().unwrap().len(), 3);
}

#[test]
fn value_as_sequence_mut_on_non_sequence() {
    let mut value = Value::String("not a sequence".into());
    assert!(value.as_sequence_mut().is_none());
}

#[test]
fn value_as_mapping_mut() {
    let mut map = IndexMap::new();
    map.insert(Value::String("a".into()), Value::from(1));
    let mut value = Value::Mapping(map);

    if let Some(m) = value.as_mapping_mut() {
        m.insert(Value::String("b".into()), Value::from(2));
    }
    assert_eq!(value.as_mapping().unwrap().len(), 2);
}

#[test]
fn value_as_mapping_mut_on_non_mapping() {
    let mut value = Value::Sequence(vec![]);
    assert!(value.as_mapping_mut().is_none());
}

#[test]
fn value_as_tagged_mut() {
    let mut value = Value::Tagged(Box::new(TaggedValue {
        tag: "!old".into(),
        value: Value::String("data".into()),
    }));

    if let Some(t) = value.as_tagged_mut() {
        t.tag = "!new".into();
    }

    assert_eq!(value.as_tagged().unwrap().tag, "!new");
}

#[test]
fn value_as_tagged_mut_on_non_tagged() {
    let mut value = Value::Null;
    assert!(value.as_tagged_mut().is_none());
}

#[test]
fn value_get_mut() {
    let mut map = IndexMap::new();
    map.insert(Value::String("key".into()), Value::from(1));
    let mut value = Value::Mapping(map);

    if let Some(v) = value.get_mut("key") {
        *v = Value::from(42);
    }
    assert_eq!(value["key"].as_i64(), Some(42));
}

#[test]
fn value_get_mut_nonexistent_key() {
    let mut map = IndexMap::new();
    map.insert(Value::String("key".into()), Value::from(1));
    let mut value = Value::Mapping(map);

    assert!(value.get_mut("nonexistent").is_none());
}

#[test]
fn value_get_mut_on_non_mapping() {
    let mut value = Value::Sequence(vec![]);
    assert!(value.get_mut("key").is_none());
}

// =============================================================================
// Ordering Tests
// =============================================================================

#[test]
fn value_ordering_by_type() {
    // Null < Bool < Number < String < Sequence < Mapping < Tagged
    assert!(Value::Null < Value::Bool(false));
    assert!(Value::Bool(true) < Value::Number(Number::Int(0)));
    assert!(Value::Number(Number::Int(0)) < Value::String("".into()));
    assert!(Value::String("".into()) < Value::Sequence(vec![]));
    assert!(Value::Sequence(vec![]) < Value::Mapping(IndexMap::new()));
    assert!(
        Value::Mapping(IndexMap::new())
            < Value::Tagged(Box::new(TaggedValue {
                tag: "!t".into(),
                value: Value::Null
            }))
    );
}

#[test]
fn value_ordering_bools() {
    assert!(Value::Bool(false) < Value::Bool(true));
    assert_eq!(
        Value::Bool(true).partial_cmp(&Value::Bool(true)),
        Some(std::cmp::Ordering::Equal)
    );
}

#[test]
fn value_ordering_numbers() {
    assert!(Value::Number(Number::Int(-10)) < Value::Number(Number::Int(0)));
    assert!(Value::Number(Number::Int(0)) < Value::Number(Number::Int(10)));
    assert!(Value::Number(Number::UInt(10)) < Value::Number(Number::UInt(20)));
    assert!(Value::Number(Number::Float(1.0)) < Value::Number(Number::Float(2.0)));
}

#[test]
fn value_ordering_strings() {
    assert!(Value::String("a".into()) < Value::String("b".into()));
    assert!(Value::String("aa".into()) < Value::String("ab".into()));
    assert!(Value::String("".into()) < Value::String("a".into()));
}

#[test]
fn value_ordering_sequences() {
    let seq1 = Value::Sequence(vec![Value::from(1)]);
    let seq2 = Value::Sequence(vec![Value::from(2)]);
    let seq3 = Value::Sequence(vec![Value::from(1), Value::from(2)]);

    assert!(seq1 < seq2);
    assert!(seq1 < seq3);
}

#[test]
fn value_ordering_mappings() {
    let mut map1 = IndexMap::new();
    map1.insert(Value::String("a".into()), Value::from(1));

    let mut map2 = IndexMap::new();
    map2.insert(Value::String("b".into()), Value::from(1));

    let val1 = Value::Mapping(map1);
    let val2 = Value::Mapping(map2);

    // Mappings are compared by their entries
    assert!(val1 < val2);
}

#[test]
fn value_ordering_tagged() {
    let t1 = Value::Tagged(Box::new(TaggedValue {
        tag: "!a".into(),
        value: Value::Null,
    }));
    let t2 = Value::Tagged(Box::new(TaggedValue {
        tag: "!b".into(),
        value: Value::Null,
    }));

    assert!(t1 < t2);
}

// =============================================================================
// Number Equality and Ordering
// =============================================================================

#[test]
fn number_cross_type_equality_int_uint() {
    // Positive Int should equal UInt
    assert_eq!(Number::Int(42), Number::UInt(42));
    assert_eq!(Number::UInt(42), Number::Int(42));

    // Negative Int should not equal any UInt
    assert_ne!(Number::Int(-1), Number::UInt(1));
}

#[test]
fn number_float_equality() {
    assert_eq!(Number::Float(2.5), Number::Float(2.5));
    // NaN comparison via bit equality
    let nan1 = Number::Float(f64::NAN);
    let nan2 = Number::Float(f64::NAN);
    assert_eq!(nan1, nan2); // Using to_bits comparison
}

#[test]
fn number_ordering() {
    assert!(Number::Int(-10) < Number::Int(0));
    assert!(Number::Int(0) < Number::UInt(10));
    assert!(Number::Float(1.5) < Number::Float(2.5));
}

// =============================================================================
// Hash Tests
// =============================================================================

#[test]
fn value_hash_consistency() {
    let mut set = HashSet::new();
    set.insert(Value::String("key".into()));

    assert!(set.contains(&Value::String("key".into())));
    assert!(!set.contains(&Value::String("other".into())));
}

#[test]
fn value_hash_different_types() {
    let mut set = HashSet::new();
    set.insert(Value::Null);
    set.insert(Value::Bool(true));
    set.insert(Value::Number(Number::Int(42)));
    set.insert(Value::String("test".into()));

    assert_eq!(set.len(), 4);
}

#[test]
fn number_hash() {
    let mut set = HashSet::new();
    set.insert(Number::Int(42));
    set.insert(Number::UInt(100));
    set.insert(Number::Float(2.5));

    assert_eq!(set.len(), 3);
}

// =============================================================================
// From Implementations
// =============================================================================

#[test]
fn value_from_i32() {
    let value = Value::from(42i32);
    assert_eq!(value.as_i64(), Some(42));
}

#[test]
fn value_from_u32() {
    let value = Value::from(42u32);
    assert_eq!(value.as_u64(), Some(42));
}

#[test]
fn value_from_f32() {
    let value = Value::from(2.5f32);
    assert!((value.as_f64().unwrap() - 2.5).abs() < 0.001);
}

#[test]
fn value_from_vec() {
    let value = Value::from(vec![1i64, 2, 3]);
    assert!(value.is_sequence());
    assert_eq!(value.as_sequence().unwrap().len(), 3);
}

#[test]
fn value_from_option_some() {
    let value = Value::from(Some(42i64));
    assert_eq!(value.as_i64(), Some(42));
}

#[test]
fn value_from_option_none() {
    let value = Value::from(None::<i64>);
    assert!(value.is_null());
}

// =============================================================================
// Indexing
// =============================================================================

#[test]
fn value_index_str_missing() {
    let map = IndexMap::new();
    let value = Value::Mapping(map);
    // Missing keys return Null
    assert_eq!(value["missing"], Value::Null);
}

#[test]
fn value_index_usize_missing() {
    let value = Value::Sequence(vec![Value::from(1)]);
    // Out of bounds returns Null
    assert_eq!(value[10], Value::Null);
}

#[test]
fn value_index_on_non_container() {
    let value = Value::String("not a container".into());
    assert_eq!(value["key"], Value::Null);
    assert_eq!(value[0], Value::Null);
}

// =============================================================================
// Display
// =============================================================================

#[test]
fn value_display() {
    let value = Value::String("hello".into());
    let display = format!("{}", value);
    assert!(display.contains("hello"));
}

#[test]
fn tagged_value_equality() {
    let t1 = TaggedValue {
        tag: "!test".into(),
        value: Value::String("data".into()),
    };
    let t2 = TaggedValue {
        tag: "!test".into(),
        value: Value::String("data".into()),
    };
    let t3 = TaggedValue {
        tag: "!other".into(),
        value: Value::String("data".into()),
    };

    assert_eq!(t1, t2);
    assert_ne!(t1, t3);
}

#[test]
fn tagged_value_ordering() {
    let t1 = TaggedValue {
        tag: "!a".into(),
        value: Value::from(1),
    };
    let t2 = TaggedValue {
        tag: "!a".into(),
        value: Value::from(2),
    };
    let t3 = TaggedValue {
        tag: "!b".into(),
        value: Value::from(1),
    };

    // Same tag, different value
    assert!(t1 < t2);
    // Different tag (tag compared first)
    assert!(t1 < t3);
}
