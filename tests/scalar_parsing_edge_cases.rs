//! Scalar parsing edge cases tests.
//!
//! Tests for edge cases in YAML scalar type inference including
//! radix prefixes, special floats, and boundary values.

use fyaml::Document;

// =============================================================================
// Radix Prefixes (case variations)
// =============================================================================

#[test]
fn parse_lowercase_hex() {
    let doc = Document::parse_str("hex: 0xff").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("hex").unwrap().as_i64(), Some(255));
}

#[test]
fn parse_uppercase_hex_prefix() {
    let doc = Document::parse_str("hex: 0XFF").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("hex").unwrap().as_i64(), Some(255));
}

#[test]
fn parse_mixed_case_hex() {
    let doc = Document::parse_str("hex: 0xAbCdEf").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("hex").unwrap().as_i64(), Some(0xABCDEF));
}

#[test]
fn parse_lowercase_octal() {
    let doc = Document::parse_str("oct: 0o77").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("oct").unwrap().as_i64(), Some(63));
}

#[test]
fn parse_uppercase_octal_prefix() {
    let doc = Document::parse_str("oct: 0O77").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("oct").unwrap().as_i64(), Some(63));
}

#[test]
fn parse_lowercase_binary() {
    let doc = Document::parse_str("bin: 0b1010").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("bin").unwrap().as_i64(), Some(10));
}

#[test]
fn parse_uppercase_binary_prefix() {
    let doc = Document::parse_str("bin: 0B1010").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("bin").unwrap().as_i64(), Some(10));
}

// =============================================================================
// Negative Numbers with Radix
// =============================================================================

#[test]
fn parse_negative_hex() {
    let doc = Document::parse_str("hex: -0xFF").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("hex").unwrap().as_i64(), Some(-255));
}

#[test]
fn parse_negative_octal() {
    let doc = Document::parse_str("oct: -0o77").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("oct").unwrap().as_i64(), Some(-63));
}

#[test]
fn parse_negative_binary() {
    let doc = Document::parse_str("bin: -0b1010").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("bin").unwrap().as_i64(), Some(-10));
}

// =============================================================================
// Signed Numbers
// =============================================================================

#[test]
fn parse_positive_sign_integer() {
    let doc = Document::parse_str("pos: +42").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("pos").unwrap().as_i64(), Some(42));
}

#[test]
fn parse_positive_sign_unsigned() {
    let doc = Document::parse_str("pos: +42").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("pos").unwrap().as_u64(), Some(42));
}

#[test]
fn parse_positive_sign_hex() {
    let doc = Document::parse_str("hex: +0xFF").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("hex").unwrap().as_i64(), Some(255));
}

// =============================================================================
// Boundary Values
// =============================================================================

#[test]
fn parse_i64_max() {
    let yaml = format!("max: {}", i64::MAX);
    let doc = Document::parse_str(&yaml).unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("max").unwrap().as_i64(), Some(i64::MAX));
}

#[test]
fn parse_i64_min() {
    let yaml = format!("min: {}", i64::MIN);
    let doc = Document::parse_str(&yaml).unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("min").unwrap().as_i64(), Some(i64::MIN));
}

#[test]
fn parse_u64_max() {
    let yaml = format!("max: {}", u64::MAX);
    let doc = Document::parse_str(&yaml).unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("max").unwrap().as_u64(), Some(u64::MAX));
}

#[test]
fn parse_zero() {
    let doc = Document::parse_str("zero: 0").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("zero").unwrap().as_i64(), Some(0));
    assert_eq!(root.get("zero").unwrap().as_u64(), Some(0));
}

#[test]
fn parse_negative_one() {
    let doc = Document::parse_str("neg: -1").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("neg").unwrap().as_i64(), Some(-1));
    assert_eq!(root.get("neg").unwrap().as_u64(), None); // Can't represent as unsigned
}

// =============================================================================
// Invalid Numbers
// =============================================================================

#[test]
fn parse_just_minus_sign_not_number() {
    // Note: bare `-` can be interpreted as sequence indicator in YAML
    // Use quoted version to test
    let doc = Document::parse_str("dash: '-'").unwrap();
    let root = doc.root_value().unwrap();
    // Quoted, so not interpreted as number
    assert_eq!(root.get("dash").unwrap().as_i64(), None);
    assert_eq!(root.get("dash").unwrap().as_str(), Some("-"));
}

#[test]
fn parse_just_plus_sign_not_number() {
    let doc = Document::parse_str("plus: +").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("plus").unwrap().as_i64(), None);
}

#[test]
fn parse_invalid_hex() {
    let doc = Document::parse_str("invalid: 0xGG").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("invalid").unwrap().as_i64(), None);
}

#[test]
fn parse_overflow_returns_none() {
    // Value larger than i64::MAX
    let yaml = format!("big: {}0", i64::MAX);
    let doc = Document::parse_str(&yaml).unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("big").unwrap().as_i64(), None);
}

// =============================================================================
// Scientific Notation
// =============================================================================

#[test]
fn parse_scientific_lowercase_e() {
    let doc = Document::parse_str("sci: 1e3").unwrap();
    let root = doc.root_value().unwrap();
    assert!((root.get("sci").unwrap().as_f64().unwrap() - 1000.0).abs() < 0.01);
}

#[test]
fn parse_scientific_uppercase_e() {
    let doc = Document::parse_str("sci: 1E3").unwrap();
    let root = doc.root_value().unwrap();
    assert!((root.get("sci").unwrap().as_f64().unwrap() - 1000.0).abs() < 0.01);
}

#[test]
fn parse_scientific_negative_exponent() {
    let doc = Document::parse_str("sci: 1.5e-2").unwrap();
    let root = doc.root_value().unwrap();
    assert!((root.get("sci").unwrap().as_f64().unwrap() - 0.015).abs() < 0.0001);
}

#[test]
fn parse_scientific_positive_exponent() {
    let doc = Document::parse_str("sci: 1.5e+2").unwrap();
    let root = doc.root_value().unwrap();
    assert!((root.get("sci").unwrap().as_f64().unwrap() - 150.0).abs() < 0.01);
}

#[test]
fn parse_scientific_negative_base() {
    let doc = Document::parse_str("sci: -1e3").unwrap();
    let root = doc.root_value().unwrap();
    assert!((root.get("sci").unwrap().as_f64().unwrap() - (-1000.0)).abs() < 0.01);
}

// =============================================================================
// Special Float Values
// =============================================================================

#[test]
fn parse_infinity_lowercase() {
    let doc = Document::parse_str("inf: .inf").unwrap();
    let root = doc.root_value().unwrap();
    let val = root.get("inf").unwrap().as_f64().unwrap();
    assert!(val.is_infinite());
    assert!(val.is_sign_positive());
}

#[test]
fn parse_infinity_mixed_case() {
    let doc = Document::parse_str("inf: .Inf").unwrap();
    let root = doc.root_value().unwrap();
    assert!(root.get("inf").unwrap().as_f64().unwrap().is_infinite());
}

#[test]
fn parse_infinity_uppercase() {
    let doc = Document::parse_str("inf: .INF").unwrap();
    let root = doc.root_value().unwrap();
    assert!(root.get("inf").unwrap().as_f64().unwrap().is_infinite());
}

#[test]
fn parse_positive_infinity() {
    let doc = Document::parse_str("inf: +.inf").unwrap();
    let root = doc.root_value().unwrap();
    let val = root.get("inf").unwrap().as_f64().unwrap();
    assert!(val.is_infinite());
    assert!(val.is_sign_positive());
}

#[test]
fn parse_negative_infinity() {
    let doc = Document::parse_str("inf: -.inf").unwrap();
    let root = doc.root_value().unwrap();
    let val = root.get("inf").unwrap().as_f64().unwrap();
    assert!(val.is_infinite());
    assert!(val.is_sign_negative());
}

#[test]
fn parse_nan_lowercase() {
    let doc = Document::parse_str("nan: .nan").unwrap();
    let root = doc.root_value().unwrap();
    assert!(root.get("nan").unwrap().as_f64().unwrap().is_nan());
}

#[test]
fn parse_nan_mixed_case() {
    let doc = Document::parse_str("nan: .NaN").unwrap();
    let root = doc.root_value().unwrap();
    assert!(root.get("nan").unwrap().as_f64().unwrap().is_nan());
}

#[test]
fn parse_nan_uppercase() {
    let doc = Document::parse_str("nan: .NAN").unwrap();
    let root = doc.root_value().unwrap();
    assert!(root.get("nan").unwrap().as_f64().unwrap().is_nan());
}

// =============================================================================
// Boolean Variants
// =============================================================================

#[test]
fn parse_bool_true_variants() {
    let doc = Document::parse_str("t1: true\nt2: True\nt3: TRUE").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("t1").unwrap().as_bool(), Some(true));
    assert_eq!(root.get("t2").unwrap().as_bool(), Some(true));
    assert_eq!(root.get("t3").unwrap().as_bool(), Some(true));
}

#[test]
fn parse_bool_false_variants() {
    let doc = Document::parse_str("f1: false\nf2: False\nf3: FALSE").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("f1").unwrap().as_bool(), Some(false));
    assert_eq!(root.get("f2").unwrap().as_bool(), Some(false));
    assert_eq!(root.get("f3").unwrap().as_bool(), Some(false));
}

#[test]
fn parse_bool_yes_no() {
    let doc = Document::parse_str("y1: yes\ny2: Yes\ny3: YES\nn1: no\nn2: No\nn3: NO").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("y1").unwrap().as_bool(), Some(true));
    assert_eq!(root.get("y2").unwrap().as_bool(), Some(true));
    assert_eq!(root.get("y3").unwrap().as_bool(), Some(true));
    assert_eq!(root.get("n1").unwrap().as_bool(), Some(false));
    assert_eq!(root.get("n2").unwrap().as_bool(), Some(false));
    assert_eq!(root.get("n3").unwrap().as_bool(), Some(false));
}

#[test]
fn parse_bool_on_off() {
    let doc =
        Document::parse_str("on1: on\non2: On\non3: ON\noff1: off\noff2: Off\noff3: OFF").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("on1").unwrap().as_bool(), Some(true));
    assert_eq!(root.get("on2").unwrap().as_bool(), Some(true));
    assert_eq!(root.get("on3").unwrap().as_bool(), Some(true));
    assert_eq!(root.get("off1").unwrap().as_bool(), Some(false));
    assert_eq!(root.get("off2").unwrap().as_bool(), Some(false));
    assert_eq!(root.get("off3").unwrap().as_bool(), Some(false));
}

// =============================================================================
// Null Variants
// =============================================================================

#[test]
fn parse_null_variants() {
    let doc = Document::parse_str("n1: null\nn2: Null\nn3: NULL\nn4: ~").unwrap();
    let root = doc.root_value().unwrap();
    assert!(root.get("n1").unwrap().is_null());
    assert!(root.get("n2").unwrap().is_null());
    assert!(root.get("n3").unwrap().is_null());
    assert!(root.get("n4").unwrap().is_null());
}

#[test]
fn parse_empty_value_is_null() {
    let doc = Document::parse_str("empty:").unwrap();
    let root = doc.root_value().unwrap();
    assert!(root.get("empty").unwrap().is_null());
}

// =============================================================================
// Quoted Values Not Interpreted
// =============================================================================

#[test]
fn quoted_true_not_bool() {
    let doc = Document::parse_str("quoted: 'true'").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("quoted").unwrap().as_bool(), None);
    assert_eq!(root.get("quoted").unwrap().as_str(), Some("true"));
}

#[test]
fn quoted_number_not_int() {
    let doc = Document::parse_str("quoted: '42'").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("quoted").unwrap().as_i64(), None);
    assert_eq!(root.get("quoted").unwrap().as_str(), Some("42"));
}

#[test]
fn quoted_null_not_null() {
    let doc = Document::parse_str("quoted: 'null'").unwrap();
    let root = doc.root_value().unwrap();
    assert!(!root.get("quoted").unwrap().is_null());
    assert_eq!(root.get("quoted").unwrap().as_str(), Some("null"));
}

#[test]
fn double_quoted_true_not_bool() {
    let doc = Document::parse_str("quoted: \"true\"").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("quoted").unwrap().as_bool(), None);
}

#[test]
fn literal_block_not_interpreted() {
    let doc = Document::parse_str("literal: |\n  42").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("literal").unwrap().as_i64(), None);
}

#[test]
fn folded_block_not_interpreted() {
    let doc = Document::parse_str("folded: >\n  true").unwrap();
    let root = doc.root_value().unwrap();
    assert_eq!(root.get("folded").unwrap().as_bool(), None);
}
