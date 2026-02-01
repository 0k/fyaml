//! Shared scalar parsing utilities for YAML type inference.
//!
//! This module provides consistent parsing logic used by both `ValueRef` (zero-copy)
//! and `Value::from_node_ref()` (owned conversion).

use crate::value::Number;

/// Checks if a plain scalar represents null.
///
/// Recognizes: empty string, `~`, `null` (case-insensitive)
#[inline]
pub fn is_null(s: &str) -> bool {
    s.is_empty() || s == "~" || s.eq_ignore_ascii_case("null")
}

/// Parses a plain scalar as a boolean.
///
/// Recognizes YAML 1.1 boolean values:
/// - True: `true`, `True`, `TRUE`, `yes`, `Yes`, `YES`, `on`, `On`, `ON`
/// - False: `false`, `False`, `FALSE`, `no`, `No`, `NO`, `off`, `Off`, `OFF`
#[inline]
pub fn parse_bool(s: &str) -> Option<bool> {
    match s {
        "true" | "True" | "TRUE" | "yes" | "Yes" | "YES" | "on" | "On" | "ON" => Some(true),
        "false" | "False" | "FALSE" | "no" | "No" | "NO" | "off" | "Off" | "OFF" => Some(false),
        _ => None,
    }
}

/// Parses a plain scalar as a signed 64-bit integer.
///
/// Supports decimal, hexadecimal (`0x`), octal (`0o`), and binary (`0b`) prefixes.
/// Handles signs correctly, including edge case `i64::MIN`.
pub fn parse_i64(s: &str) -> Option<i64> {
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

    // Parse magnitude as i128 to handle i64::MIN correctly
    // (i64::MIN's absolute value overflows i64)
    let magnitude: i128 = if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        i128::from_str_radix(hex, 16).ok()?
    } else if let Some(oct) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
        i128::from_str_radix(oct, 8).ok()?
    } else if let Some(bin) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        i128::from_str_radix(bin, 2).ok()?
    } else {
        s.parse().ok()?
    };

    // Apply sign and check range
    let value = if neg { -magnitude } else { magnitude };
    i64::try_from(value).ok()
}

/// Parses a plain scalar as an unsigned 64-bit integer.
///
/// Supports decimal, hexadecimal (`0x`), octal (`0o`), and binary (`0b`) prefixes.
/// Returns `None` for negative values.
pub fn parse_u64(s: &str) -> Option<u64> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // Handle sign - negative values return None for unsigned
    let s = if let Some(rest) = s.strip_prefix('-') {
        // Negative values cannot be unsigned (unless it's just "-" which is invalid anyway)
        if !rest.is_empty() {
            return None;
        }
        rest
    } else if let Some(rest) = s.strip_prefix('+') {
        rest
    } else {
        s
    };

    // Parse with different bases
    if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).ok()
    } else if let Some(oct) = s.strip_prefix("0o").or_else(|| s.strip_prefix("0O")) {
        u64::from_str_radix(oct, 8).ok()
    } else if let Some(bin) = s.strip_prefix("0b").or_else(|| s.strip_prefix("0B")) {
        u64::from_str_radix(bin, 2).ok()
    } else {
        s.parse().ok()
    }
}

/// Parses a plain scalar as a 64-bit float.
///
/// Recognizes special values: `.inf`, `+.inf`, `-.inf`, `.nan` (case-insensitive)
pub fn parse_f64(s: &str) -> Option<f64> {
    // Special float values (case-insensitive)
    if s.eq_ignore_ascii_case(".inf") || s.eq_ignore_ascii_case("+.inf") {
        return Some(f64::INFINITY);
    }
    if s.eq_ignore_ascii_case("-.inf") {
        return Some(f64::NEG_INFINITY);
    }
    if s.eq_ignore_ascii_case(".nan") {
        return Some(f64::NAN);
    }

    // Standard float parsing
    s.parse().ok()
}

/// Checks if a plain scalar string would be ambiguous with another YAML type.
///
/// Returns `true` if the string content, when emitted as a plain scalar,
/// could be misinterpreted as null, boolean, or numeric. Such strings
/// need quoting to roundtrip correctly as `Value::String`.
#[inline]
pub fn needs_quoting(s: &str) -> bool {
    is_null(s) || parse_bool(s).is_some() || parse_number(s).is_some()
}

/// Parses a plain scalar as a Number (for Value type inference).
///
/// Tries i64 first, then u64, then f64 (only if contains `.` or exponent).
pub fn parse_number(s: &str) -> Option<Number> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    // Try signed integer first
    if let Some(n) = parse_i64(s) {
        // Prefer UInt for non-negative values
        if n >= 0 {
            return Some(Number::UInt(n as u64));
        }
        return Some(Number::Int(n));
    }

    // Try unsigned integer for large positive values (> i64::MAX)
    if let Some(n) = parse_u64(s) {
        return Some(Number::UInt(n));
    }

    // Try float (special values or decimal/exponent notation)
    // Special values
    if s.eq_ignore_ascii_case(".inf") || s.eq_ignore_ascii_case("+.inf") {
        return Some(Number::Float(f64::INFINITY));
    }
    if s.eq_ignore_ascii_case("-.inf") {
        return Some(Number::Float(f64::NEG_INFINITY));
    }
    if s.eq_ignore_ascii_case(".nan") {
        return Some(Number::Float(f64::NAN));
    }

    // Regular float - must contain decimal point or exponent
    let has_decimal = s.contains('.');
    let has_exponent = s.bytes().any(|b| b == b'e' || b == b'E');
    if has_decimal || has_exponent {
        if let Ok(f) = s.parse::<f64>() {
            return Some(Number::Float(f));
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_null() {
        assert!(is_null(""));
        assert!(is_null("~"));
        assert!(is_null("null"));
        assert!(is_null("NULL"));
        assert!(is_null("Null"));
        assert!(!is_null("nil"));
        assert!(!is_null("none"));
    }

    #[test]
    fn test_parse_bool() {
        assert_eq!(parse_bool("true"), Some(true));
        assert_eq!(parse_bool("True"), Some(true));
        assert_eq!(parse_bool("TRUE"), Some(true));
        assert_eq!(parse_bool("yes"), Some(true));
        assert_eq!(parse_bool("on"), Some(true));
        assert_eq!(parse_bool("false"), Some(false));
        assert_eq!(parse_bool("False"), Some(false));
        assert_eq!(parse_bool("no"), Some(false));
        assert_eq!(parse_bool("off"), Some(false));
        assert_eq!(parse_bool("maybe"), None);
    }

    #[test]
    fn test_parse_i64_decimal() {
        assert_eq!(parse_i64("42"), Some(42));
        assert_eq!(parse_i64("-10"), Some(-10));
        assert_eq!(parse_i64("+5"), Some(5));
        assert_eq!(parse_i64("0"), Some(0));
    }

    #[test]
    fn test_parse_i64_radix() {
        assert_eq!(parse_i64("0xFF"), Some(255));
        assert_eq!(parse_i64("0xff"), Some(255));
        assert_eq!(parse_i64("-0xFF"), Some(-255));
        assert_eq!(parse_i64("0o77"), Some(63));
        assert_eq!(parse_i64("-0o77"), Some(-63));
        assert_eq!(parse_i64("0b1010"), Some(10));
        assert_eq!(parse_i64("-0b1010"), Some(-10));
    }

    #[test]
    fn test_parse_i64_boundaries() {
        assert_eq!(parse_i64(&i64::MAX.to_string()), Some(i64::MAX));
        assert_eq!(parse_i64(&i64::MIN.to_string()), Some(i64::MIN));
        // Overflow returns None
        assert_eq!(parse_i64("9223372036854775808"), None); // i64::MAX + 1
    }

    #[test]
    fn test_parse_u64() {
        assert_eq!(parse_u64("42"), Some(42));
        assert_eq!(parse_u64("+5"), Some(5));
        assert_eq!(parse_u64("-10"), None); // Negative returns None
        assert_eq!(parse_u64("0xFF"), Some(255));
    }

    #[test]
    fn test_parse_u64_large() {
        assert_eq!(parse_u64(&u64::MAX.to_string()), Some(u64::MAX));
        // Values > i64::MAX but <= u64::MAX
        let large = (i64::MAX as u64) + 1;
        assert_eq!(parse_u64(&large.to_string()), Some(large));
    }

    #[test]
    fn test_parse_f64() {
        assert_eq!(parse_f64("2.5"), Some(2.5));
        assert_eq!(parse_f64("1e10"), Some(1e10));
        assert!(parse_f64(".inf").unwrap().is_infinite());
        assert!(parse_f64("+.inf").unwrap().is_infinite());
        assert!(parse_f64("-.inf").unwrap().is_infinite());
        assert!(parse_f64("-.inf").unwrap().is_sign_negative());
        assert!(parse_f64(".nan").unwrap().is_nan());
        assert!(parse_f64(".NaN").unwrap().is_nan());
    }

    #[test]
    fn test_parse_number() {
        assert_eq!(parse_number("42"), Some(Number::UInt(42)));
        assert_eq!(parse_number("-10"), Some(Number::Int(-10)));
        assert_eq!(parse_number("2.5"), Some(Number::Float(2.5)));
        // Large unsigned values that don't fit in i64
        let large = u64::MAX;
        assert_eq!(parse_number(&large.to_string()), Some(Number::UInt(large)));
    }
}
