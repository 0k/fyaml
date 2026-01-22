//! Error handling coverage tests.
//!
//! Tests for error types, display formatting, and the Error::source() implementation.

use fyaml::{Document, Error, ParseError};
use std::error::Error as StdError;

#[test]
fn error_display_type_mismatch() {
    let err = Error::TypeMismatch {
        expected: "scalar",
        got: "mapping",
    };
    let display = format!("{}", err);
    assert!(display.contains("scalar"));
    assert!(display.contains("mapping"));
}

#[test]
fn error_display_ffi() {
    let err = Error::Ffi("test ffi error");
    let display = format!("{}", err);
    assert!(display.contains("FFI"));
    assert!(display.contains("test ffi error"));
}

#[test]
fn error_display_parse() {
    let err = Error::Parse("invalid yaml");
    let display = format!("{}", err);
    assert!(display.contains("Parse"));
    assert!(display.contains("invalid yaml"));
}

#[test]
fn error_display_io() {
    let err = Error::Io("file not found");
    let display = format!("{}", err);
    assert!(display.contains("I/O"));
}

#[test]
fn error_display_alloc() {
    let err = Error::Alloc("out of memory");
    let display = format!("{}", err);
    assert!(display.contains("Allocation"));
}

#[test]
fn error_display_document_mismatch() {
    let err = Error::DocumentMismatch;
    let display = format!("{}", err);
    assert!(display.contains("document"));
}

#[test]
fn error_display_scalar_too_large() {
    let err = Error::ScalarTooLarge(999999);
    let display = format!("{}", err);
    assert!(display.contains("999999"));
    assert!(display.contains("limit"));
}

#[test]
fn error_display_mutation_while_iterating() {
    let err = Error::MutationWhileIterating;
    let display = format!("{}", err);
    assert!(display.contains("mutate"));
    assert!(display.contains("iterating"));
}

#[test]
#[allow(invalid_from_utf8)]
fn error_source_returns_underlying_utf8_error() {
    let utf8_err = std::str::from_utf8(&[0xff]).unwrap_err();
    let err = Error::from(utf8_err);
    assert!(err.source().is_some());
}

#[test]
fn error_source_returns_parse_error() {
    let pe = ParseError::with_location("test", 1, 1);
    let err = Error::from(pe);
    assert!(err.source().is_some());
}

#[test]
fn error_source_returns_none_for_ffi() {
    let err = Error::Ffi("test");
    assert!(err.source().is_none());
}

#[test]
fn parse_error_without_location() {
    let pe = ParseError::new("test message");
    assert!(pe.location().is_none());
    assert!(pe.line().is_none());
    assert!(pe.column().is_none());
    let display = format!("{}", pe);
    assert_eq!(display, "test message");
}

#[test]
fn parse_error_with_full_location() {
    let pe = ParseError::with_location("syntax error", 5, 10);
    assert_eq!(pe.location(), Some((5, 10)));
    assert_eq!(pe.line(), Some(5));
    assert_eq!(pe.column(), Some(10));
    let display = format!("{}", pe);
    assert!(display.contains("5:10"));
    assert!(display.contains("syntax error"));
}

#[test]
fn parse_error_message_accessor() {
    let pe = ParseError::new("my error message");
    assert_eq!(pe.message(), "my error message");
}

#[test]
fn as_parse_error_returns_some_for_parse_error() {
    let pe = ParseError::with_location("test", 1, 1);
    let err = Error::from(pe);
    assert!(err.as_parse_error().is_some());
    assert_eq!(err.as_parse_error().unwrap().line(), Some(1));
}

#[test]
fn as_parse_error_returns_none_for_other_errors() {
    let err = Error::Ffi("test");
    assert!(err.as_parse_error().is_none());

    let err = Error::Parse("test");
    assert!(err.as_parse_error().is_none());

    let err = Error::Io("test");
    assert!(err.as_parse_error().is_none());
}

#[test]
fn parse_error_equality() {
    let pe1 = ParseError::with_location("error", 1, 2);
    let pe2 = ParseError::with_location("error", 1, 2);
    let pe3 = ParseError::with_location("error", 1, 3);

    assert_eq!(pe1, pe2);
    assert_ne!(pe1, pe3);
}

#[test]
fn parse_error_clone() {
    let pe1 = ParseError::with_location("error", 5, 10);
    let pe2 = pe1.clone();
    assert_eq!(pe1, pe2);
}

#[test]
fn document_parse_error_has_location() {
    let result = Document::parse_str("[unclosed");
    assert!(result.is_err());

    if let Err(Error::ParseError(pe)) = result {
        // Should have location info
        assert!(pe.line().is_some());
    }
}
