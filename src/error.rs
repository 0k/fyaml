//! Error types for fyaml operations.
//!
//! This module provides structured error types for YAML parsing and manipulation,
//! with rich diagnostic information including line and column numbers.
//!
//! # Parse Errors
//!
//! When parsing fails, [`Error::ParseError`] provides detailed location information:
//!
//! ```
//! use fyaml::Document;
//!
//! let result = Document::parse_str("[unclosed");
//! if let Err(e) = result {
//!     // Access structured error info
//!     if let fyaml::Error::ParseError(parse_err) = &e {
//!         println!("Error: {}", parse_err.message());
//!         if let Some((line, col)) = parse_err.location() {
//!             println!("At line {}, column {}", line, col);
//!         }
//!     }
//!     // Or just display it nicely
//!     println!("{}", e);  // "Parse error at 2:1: flow sequence without a closing bracket"
//! }
//! ```

use std::fmt;

/// Detailed parse error with location information.
///
/// Contains the error message from libfyaml along with optional line and column
/// numbers indicating where the error occurred in the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// The error message from libfyaml.
    pub(crate) message: String,
    /// Line number (1-based), if available.
    pub(crate) line: Option<u32>,
    /// Column number (1-based), if available.
    pub(crate) column: Option<u32>,
}

impl ParseError {
    /// Creates a new parse error with just a message.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            line: None,
            column: None,
        }
    }

    /// Creates a new parse error with location information.
    pub fn with_location(message: impl Into<String>, line: u32, column: u32) -> Self {
        Self {
            message: message.into(),
            line: Some(line),
            column: Some(column),
        }
    }

    /// Returns the error message.
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Returns the line number (1-based), if available.
    pub fn line(&self) -> Option<u32> {
        self.line
    }

    /// Returns the column number (1-based), if available.
    pub fn column(&self) -> Option<u32> {
        self.column
    }

    /// Returns the location as (line, column), if both are available.
    pub fn location(&self) -> Option<(u32, u32)> {
        match (self.line, self.column) {
            (Some(l), Some(c)) => Some((l, c)),
            _ => None,
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match (self.line, self.column) {
            (Some(line), Some(col)) => write!(f, "at {}:{}: {}", line, col, self.message),
            (Some(line), None) => write!(f, "at line {}: {}", line, self.message),
            _ => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for ParseError {}

/// Error type for fyaml operations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// FFI call returned an error or unexpected result.
    Ffi(&'static str),

    /// YAML parsing failed (simple message, no location info).
    Parse(&'static str),

    /// YAML parsing failed with detailed location information.
    ParseError(ParseError),

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

impl Error {
    /// Returns the parse error details if this is a parse error.
    pub fn as_parse_error(&self) -> Option<&ParseError> {
        match self {
            Error::ParseError(e) => Some(e),
            _ => None,
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Ffi(msg) => write!(f, "FFI error: {}", msg),
            Error::Parse(msg) => write!(f, "Parse error: {}", msg),
            Error::ParseError(e) => write!(f, "Parse error {}", e),
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
            Error::ParseError(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::str::Utf8Error> for Error {
    fn from(e: std::str::Utf8Error) -> Self {
        Error::Utf8(e)
    }
}

impl From<ParseError> for Error {
    fn from(e: ParseError) -> Self {
        Error::ParseError(e)
    }
}

/// Result type alias using fyaml's Error.
pub type Result<T> = std::result::Result<T, Error>;
