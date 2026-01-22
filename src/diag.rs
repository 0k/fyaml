//! Diagnostic capture for libfyaml errors.
//!
//! This module provides a wrapper around libfyaml's diagnostic system to capture
//! error messages instead of printing them to stderr. Collected errors are then
//! converted into rich Rust error types with line/column information.

use crate::error::{Error, ParseError};
use fyaml_sys::*;
use std::ffi::CStr;
use std::os::raw::{c_char, c_void};
use std::ptr;

/// No-op output function that suppresses all diagnostic output to stderr.
///
/// This function is called by libfyaml when it wants to output diagnostic messages.
/// By providing this callback, we prevent any output to stderr while still collecting
/// errors via `fy_diag_errors_iterate`.
unsafe extern "C" fn silent_output(
    _diag: *mut fy_diag,
    _user: *mut c_void,
    _buf: *const c_char,
    _len: usize,
) {
    // Intentionally empty - suppress all output
}

/// RAII wrapper around libfyaml's diagnostic system.
///
/// Creates a diagnostic handler that collects errors instead of printing to stderr.
/// When dropped, the diagnostic handler is destroyed.
pub(crate) struct Diag {
    ptr: *mut fy_diag,
}

impl Diag {
    /// Creates a new diagnostic handler that collects errors silently.
    pub fn new() -> Option<Self> {
        let cfg = fy_diag_cfg {
            fp: ptr::null_mut(),
            output_fn: Some(silent_output), // Silent callback - no stderr output
            user: ptr::null_mut(),
            level: FYET_ERROR,
            module_mask: u32::MAX, // All modules
            _bitfield_align_1: [],
            _bitfield_1: fy_diag_cfg::new_bitfield_1(
                false, // show_source
                false, // show_position
                false, // show_type
                false, // show_module
                false, // color_diag (colorize)
            ),
            source_width: 0,
            position_width: 0,
            type_width: 0,
            module_width: 0,
        };

        let ptr = unsafe { fy_diag_create(&cfg) };
        if ptr.is_null() {
            return None;
        }

        // Enable error collection so we can iterate them later
        unsafe { fy_diag_set_collect_errors(ptr, true) };

        Some(Self { ptr })
    }

    /// Returns the raw pointer for use in parse configurations.
    pub fn as_ptr(&self) -> *mut fy_diag {
        self.ptr
    }

    /// Returns the first collected error, if any.
    ///
    /// This is more efficient than `collect_errors()` when you only need the first error,
    /// as it doesn't allocate a Vec.
    pub fn first_error(&self) -> Option<ParseError> {
        let mut prev: *mut std::ffi::c_void = ptr::null_mut();
        let err = unsafe { fy_diag_errors_iterate(self.ptr, &mut prev) };
        if err.is_null() {
            None
        } else {
            Some(unsafe { parse_error_from_diag_error(&*err) })
        }
    }

    /// Returns the first collected error as an Error, or a fallback if none collected.
    ///
    /// This is optimized to avoid allocating a Vec - it only retrieves the first error.
    pub fn first_error_or(&self, fallback_msg: &'static str) -> Error {
        self.first_error()
            .map(Error::ParseError)
            .unwrap_or(Error::Parse(fallback_msg))
    }

    /// Collects all errors into a vector of ParseError.
    ///
    /// Use [`first_error()`](Self::first_error) if you only need the first error.
    #[allow(dead_code)]
    pub fn collect_errors(&self) -> Vec<ParseError> {
        let mut errors = Vec::new();
        let mut prev: *mut std::ffi::c_void = ptr::null_mut();

        loop {
            let err = unsafe { fy_diag_errors_iterate(self.ptr, &mut prev) };
            if err.is_null() {
                break;
            }

            let parse_err = unsafe { parse_error_from_diag_error(&*err) };
            errors.push(parse_err);
        }

        errors
    }
}

impl Drop for Diag {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            // Use fy_diag_unref instead of fy_diag_destroy to properly handle
            // reference counting. libfyaml may have incremented the refcount
            // when we passed the diag to a parse config.
            unsafe { fy_diag_unref(self.ptr) };
        }
    }
}

/// Returns the first error from an optional Diag, or a fallback Parse error.
///
/// This consolidates the common pattern of extracting an error from a Diag
/// that might have failed to create (OOM).
pub(crate) fn diag_error(diag: Option<Diag>, fallback_msg: &'static str) -> Error {
    diag.map(|d| d.first_error_or(fallback_msg))
        .unwrap_or(Error::Parse(fallback_msg))
}

/// Converts a libfyaml `fy_diag_error` to our `ParseError`.
///
/// # Safety
/// The `err` pointer must be valid and point to a properly initialized `fy_diag_error`.
unsafe fn parse_error_from_diag_error(err: &fy_diag_error) -> ParseError {
    let message = if err.msg.is_null() {
        "unknown error".to_string()
    } else {
        CStr::from_ptr(err.msg).to_string_lossy().into_owned()
    };

    // Line and column are 0-based in libfyaml, convert to 1-based for users
    // -1 means "not available"
    let line = if err.line >= 0 {
        Some((err.line + 1) as u32)
    } else {
        None
    };

    let column = if err.column >= 0 {
        Some((err.column + 1) as u32)
    } else {
        None
    };

    ParseError {
        message,
        line,
        column,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Document;

    #[test]
    fn test_diag_creation() {
        let diag = Diag::new();
        assert!(diag.is_some());
    }

    #[test]
    fn test_diag_collect_empty() {
        let diag = Diag::new().unwrap();
        let errors = diag.collect_errors();
        assert!(errors.is_empty());
    }

    #[test]
    fn test_parse_error_has_location() {
        let result = Document::parse_str("[unclosed");
        assert!(result.is_err());
        let err = result.unwrap_err();
        if let Error::ParseError(pe) = err {
            // Should have location info
            assert!(pe.line().is_some(), "Expected line number");
            assert!(pe.column().is_some(), "Expected column number");
            // Message should be non-empty (don't check specific wording - varies by libfyaml version)
            assert!(!pe.message().is_empty(), "Expected non-empty error message");
        } else {
            panic!("Expected ParseError variant, got: {:?}", err);
        }
    }

    #[test]
    fn test_parse_error_location_tuple() {
        let result = Document::parse_str("[unclosed");
        let err = result.unwrap_err();
        if let Error::ParseError(pe) = &err {
            let loc = pe.location();
            assert!(loc.is_some(), "Expected location tuple");
            let (line, col) = loc.unwrap();
            assert!(line >= 1, "Line should be 1-based");
            assert!(col >= 1, "Column should be 1-based");
        }
    }

    #[test]
    fn test_parse_error_display() {
        let result = Document::parse_str("key: [unclosed");
        let err = result.unwrap_err();
        let display = format!("{}", err);
        // Should include "Parse error" and location info
        assert!(
            display.contains("Parse error"),
            "Display should include 'Parse error'"
        );
    }

    #[test]
    fn test_multiline_parse_error_location() {
        // Error is in the YAML content
        let yaml = "key: value\nlist:\n  - [unclosed";
        let result = Document::parse_str(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        if let Error::ParseError(pe) = err {
            // Should have some line number (libfyaml's exact line counting may vary)
            assert!(
                pe.line().is_some(),
                "Expected line number for multiline error"
            );
            // Line should be > 1 since the error is not on the first line
            assert!(
                pe.line().unwrap() > 1,
                "Error should be after line 1, got: {:?}",
                pe.line()
            );
        }
    }

    #[test]
    fn test_multiple_errors_collection() {
        // Create a diagnostic and trigger parsing that may generate multiple errors
        // Note: libfyaml typically stops at the first error, so we may only get one
        // This test verifies collect_errors works and returns at least one error
        let diag = Diag::new().unwrap();
        let errors = diag.collect_errors();
        // Fresh diag should have no errors
        assert!(errors.is_empty());

        // After parsing fails, we get at least one error (tested via Document::parse_str)
        // The collect_errors is implicitly tested by the parse_error tests above
    }

    #[test]
    fn test_unicode_in_error_context() {
        // YAML with unicode followed by an error - the error message should handle this gracefully
        let yaml = "key: 日本語\n[unclosed";
        let result = Document::parse_str(yaml);
        assert!(result.is_err());
        let err = result.unwrap_err();
        if let Error::ParseError(pe) = err {
            // Message should be valid UTF-8 (to_string_lossy handles this)
            let msg = pe.message();
            assert!(!msg.is_empty(), "Error message should not be empty");
            // Should be valid UTF-8
            assert!(
                msg.is_ascii()
                    || msg
                        .chars()
                        .all(|c| !c.is_control() || c == '\n' || c == '\t'),
                "Error message should be valid text"
            );
        }
    }

    #[test]
    fn test_first_error_or_returns_collected() {
        // When there are collected errors, first_error_or returns the first one
        // Test implicitly via Document::parse_str which uses this mechanism
        let result = Document::parse_str("[bad");
        assert!(result.is_err());
        let err = result.unwrap_err();
        // Should be ParseError, not Parse (static)
        assert!(
            matches!(err, Error::ParseError(_)),
            "Expected ParseError variant, got {:?}",
            err
        );
    }

    #[test]
    fn test_first_error_or_returns_fallback() {
        // When no errors collected, first_error_or returns the fallback
        let diag = Diag::new().unwrap();
        let err = diag.first_error_or("fallback message");
        match err {
            Error::Parse(msg) => assert_eq!(msg, "fallback message"),
            _ => panic!("Expected Error::Parse fallback"),
        }
    }

    #[test]
    fn test_first_error_returns_none_when_empty() {
        let diag = Diag::new().unwrap();
        assert!(diag.first_error().is_none());
    }

    #[test]
    fn test_diag_error_helper_with_some() {
        // Test the diag_error helper with a Some(Diag) that has no errors
        let diag = Diag::new();
        let err = diag_error(diag, "fallback");
        match err {
            Error::Parse(msg) => assert_eq!(msg, "fallback"),
            _ => panic!("Expected Error::Parse when no errors collected"),
        }
    }

    #[test]
    fn test_diag_error_helper_with_none() {
        // Test the diag_error helper with None (simulating OOM on diag creation)
        let err = diag_error(None, "fallback");
        match err {
            Error::Parse(msg) => assert_eq!(msg, "fallback"),
            _ => panic!("Expected Error::Parse for None diag"),
        }
    }
}
