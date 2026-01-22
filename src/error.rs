//! Error types for fyaml operations.
//!
//! This module provides a structured error type that replaces string-based errors
//! throughout the crate, enabling better error handling and pattern matching.

use std::fmt;

/// Error type for fyaml operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// FFI call returned an error or unexpected result.
    Ffi(&'static str),

    /// YAML parsing failed.
    Parse(&'static str),

    /// I/O operation failed.
    Io(&'static str),

    /// Memory allocation failed.
    Alloc(&'static str),

    /// UTF-8 conversion failed.
    Utf8(std::str::Utf8Error),

    /// Attempted to mutate document while iterators are active.
    MutationWhileIterating,

    /// Operation requires a different node type.
    TypeMismatch {
        expected: &'static str,
        got: &'static str,
    },

    /// Nodes must belong to the same document.
    DocumentMismatch,

    /// Scalar length exceeds sanity limit.
    ScalarTooLarge(usize),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Ffi(msg) => write!(f, "FFI error: {}", msg),
            Error::Parse(msg) => write!(f, "Parse error: {}", msg),
            Error::Io(msg) => write!(f, "I/O error: {}", msg),
            Error::Alloc(msg) => write!(f, "Allocation error: {}", msg),
            Error::Utf8(e) => write!(f, "UTF-8 error: {}", e),
            Error::MutationWhileIterating => {
                write!(f, "Cannot mutate document while iterating")
            }
            Error::TypeMismatch { expected, got } => {
                write!(f, "Type mismatch: expected {}, got {}", expected, got)
            }
            Error::DocumentMismatch => {
                write!(f, "Nodes must belong to the same document")
            }
            Error::ScalarTooLarge(len) => {
                write!(f, "Scalar length {} exceeds sanity limit", len)
            }
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Utf8(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(e: std::str::Utf8Error) -> Self {
        Error::Utf8(e)
    }
}

/// Result type alias using fyaml's Error.
pub type Result<T> = std::result::Result<T, Error>;
