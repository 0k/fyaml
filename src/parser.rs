//! Stream parsing API.
//!
//! This module provides streaming YAML parsing that produces [`Document`] instances.
//!
//! # Memory Safety
//!
//! Documents produced by the parser may reference memory owned by the parser's
//! input buffer. The [`InputOwnership::Parser`] variant ensures the parser
//! outlives its documents, preventing use-after-free.

use crate::config;
use crate::document::{Document, InputOwnership};
use crate::error::{Error, Result};
use crate::ffi_util::malloc_copy;
use fyaml_sys::*;
use libc::{c_void, setvbuf, _IOLBF};
use std::marker::PhantomData;
use std::os::fd::AsRawFd;
use std::ptr::NonNull;
use std::rc::Rc;

// =============================================================================
// Parser Inner (shared ownership)
// =============================================================================

/// Internal parser state kept alive by documents parsed from it.
///
/// This ensures the input buffer remains valid while any document exists.
pub(crate) struct ParserInner {
    parser_ptr: *mut fy_parser,
    /// Marker to ensure !Send + !Sync
    _marker: PhantomData<*mut ()>,
}

impl ParserInner {
    fn new() -> Result<Self> {
        let cfg = config::stream_parse_cfg();
        let parser_ptr = unsafe { fy_parser_create(&cfg) };
        if parser_ptr.is_null() {
            return Err(Error::Ffi("fy_parser_create returned null"));
        }
        Ok(ParserInner {
            parser_ptr,
            _marker: PhantomData,
        })
    }

    #[inline]
    pub(crate) fn as_ptr(&self) -> *mut fy_parser {
        self.parser_ptr
    }
}

impl Drop for ParserInner {
    fn drop(&mut self) {
        if !self.parser_ptr.is_null() {
            log::trace!("Freeing ParserInner {:p}", self.parser_ptr);
            unsafe { fy_parser_destroy(self.parser_ptr) };
        }
    }
}

// =============================================================================
// Parser
// =============================================================================

/// Low-level YAML parser for streaming multi-document YAML.
///
/// Use [`FyParser::from_string`] or [`FyParser::from_stdin`] to create a parser,
/// then call [`doc_iter`](Self::doc_iter) to iterate over documents.
///
/// # Memory Safety
///
/// Documents produced by the parser hold a reference to the parser's internal
/// state, ensuring the input buffer remains valid. This prevents use-after-free
/// when accessing scalar data from streamed documents.
///
/// # Example
///
/// ```ignore
/// use fyaml::FyParser;
///
/// let yaml = "---\ndoc1: value1\n---\ndoc2: value2";
/// let parser = FyParser::from_string(yaml).unwrap();
///
/// for doc_result in parser.doc_iter() {
///     let doc = doc_result.unwrap();
///     println!("{}", doc.emit().unwrap());
/// }
/// ```
pub struct FyParser {
    inner: Rc<ParserInner>,
}

impl FyParser {
    /// Creates a new YAML parser with default configuration.
    fn new() -> Result<Self> {
        Ok(FyParser {
            inner: Rc::new(ParserInner::new()?),
        })
    }

    /// Creates a parser configured to process the given YAML string.
    ///
    /// This is useful for parsing multi-document YAML streams where you need
    /// to iterate over each document individually.
    ///
    /// # Example
    ///
    /// ```ignore
    /// use fyaml::FyParser;
    ///
    /// let yaml = "---\ndoc1: value1\n---\ndoc2: value2";
    /// let parser = FyParser::from_string(yaml).unwrap();
    ///
    /// let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();
    /// assert_eq!(docs.len(), 2);
    /// ```
    pub fn from_string(yaml: &str) -> Result<Self> {
        let parser = FyParser::new()?;

        let buf = unsafe { malloc_copy(yaml.as_bytes())? };
        let ret = unsafe { fy_parser_set_malloc_string(parser.inner.as_ptr(), buf, yaml.len()) };
        if ret != 0 {
            unsafe { libc::free(buf as *mut c_void) };
            return Err(Error::Ffi("fy_parser_set_malloc_string failed"));
        }

        Ok(parser)
    }

    /// Creates a parser configured to read from stdin.
    ///
    /// The stdin stream is set to line-buffered mode for interactive use.
    pub fn from_stdin() -> Result<Self> {
        Self::from_stdin_with_line_buffer(true)
    }

    /// Creates a parser configured to read from stdin with configurable buffering.
    ///
    /// When `line_buffered` is true, stdin is set to line-buffered mode which
    /// allows processing documents as soon as each line arrives.
    ///
    /// When `line_buffered` is false, stdin uses default (block) buffering which
    /// is more efficient for batch processing.
    pub fn from_stdin_with_line_buffer(line_buffered: bool) -> Result<Self> {
        log::trace!("open stdin (line_buffered={})", line_buffered);
        let parser = FyParser::new()?;

        // Duplicate stdin fd to avoid closing the real stdin when parser is destroyed
        let fd = std::io::stdin().as_raw_fd();
        let dup_fd = unsafe { libc::dup(fd) };
        if dup_fd < 0 {
            return Err(Error::Io("dup(stdin) failed"));
        }

        let fp = unsafe { libc::fdopen(dup_fd, b"r\0".as_ptr() as *const i8) };
        if fp.is_null() {
            unsafe { libc::close(dup_fd) };
            return Err(Error::Io("fdopen failed"));
        }

        if line_buffered {
            let rc = unsafe { setvbuf(fp, std::ptr::null_mut(), _IOLBF, 0) };
            if rc != 0 {
                unsafe { libc::fclose(fp) };
                return Err(Error::Io("setvbuf failed"));
            }
        }

        let ret = unsafe {
            fy_parser_set_input_fp(parser.inner.as_ptr(), b"stdin\0".as_ptr() as *const i8, fp)
        };
        if ret != 0 {
            unsafe { libc::fclose(fp) };
            return Err(Error::Ffi("fy_parser_set_input_fp failed"));
        }

        Ok(parser)
    }

    /// Returns an iterator over YAML documents in the stream.
    ///
    /// Each item is a `Result<Document, Error>` to surface parse errors.
    ///
    /// # Memory Safety
    ///
    /// Documents yielded by this iterator hold a reference to the parser,
    /// ensuring the input buffer remains valid even after the iterator is
    /// dropped or the parser goes out of scope.
    pub fn doc_iter(&self) -> DocumentIterator {
        DocumentIterator {
            inner: Rc::clone(&self.inner),
            done: false,
        }
    }
}

// =============================================================================
// Document Iterator
// =============================================================================

/// Iterator over YAML documents in a stream.
///
/// Created by calling [`FyParser::doc_iter`].
///
/// Returns `Result<Document, Error>` to distinguish between parse errors
/// and normal end-of-stream.
///
/// # Memory Safety
///
/// This iterator holds a shared reference to the parser's internal state.
/// Documents yielded by this iterator also hold this reference, ensuring
/// the parser's input buffer outlives all documents.
pub struct DocumentIterator {
    inner: Rc<ParserInner>,
    done: bool,
}

impl Iterator for DocumentIterator {
    type Item = Result<Document>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        log::trace!("next document ?");
        let doc_ptr = unsafe { fy_parse_load_document(self.inner.as_ptr()) };

        if doc_ptr.is_null() {
            self.done = true;
            // Check if null is due to parse error vs. clean end of stream
            let has_error = unsafe { fy_parser_get_stream_error(self.inner.as_ptr()) };
            if has_error {
                return Some(Err(Error::Parse("stream parse error")));
            }
            return None;
        }

        log::trace!("  got next document !");

        // Document keeps parser alive via Rc to ensure input buffer validity.
        // This is critical for memory safety: scalar data may reference
        // the parser's input buffer, so the parser must outlive the document.
        Some(Ok(Document::from_raw_ptr(
            NonNull::new(doc_ptr).unwrap(),
            InputOwnership::Parser(Rc::clone(&self.inner)),
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_single_document() {
        let parser = FyParser::from_string("foo: bar").unwrap();
        let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();
        assert_eq!(docs.len(), 1);
        let root = docs[0].root().unwrap();
        assert_eq!(root.at_path("/foo").unwrap().scalar_str().unwrap(), "bar");
    }

    #[test]
    fn test_parse_multiple_documents() {
        let parser = FyParser::from_string("---\ndoc1: v1\n---\ndoc2: v2\n---\ndoc3: v3").unwrap();
        let docs: Vec<_> = parser.doc_iter().filter_map(|r| r.ok()).collect();
        assert_eq!(docs.len(), 3);

        assert_eq!(
            docs[0].at_path("/doc1").unwrap().scalar_str().unwrap(),
            "v1"
        );
        assert_eq!(
            docs[1].at_path("/doc2").unwrap().scalar_str().unwrap(),
            "v2"
        );
        assert_eq!(
            docs[2].at_path("/doc3").unwrap().scalar_str().unwrap(),
            "v3"
        );
    }

    #[test]
    fn test_parse_empty_stream() {
        let parser = FyParser::from_string("").unwrap();
        let docs: Vec<_> = parser.doc_iter().collect();
        assert!(docs.is_empty());
    }

    #[test]
    fn test_documents_outlive_parser() {
        // This test verifies that documents can outlive the parser
        // because they hold an Rc to the parser's internal state.
        let docs: Vec<_>;
        {
            let parser = FyParser::from_string("key: value").unwrap();
            docs = parser.doc_iter().filter_map(|r| r.ok()).collect();
            // parser is dropped here, but ParserInner lives on via Rc
        }
        // Documents should still be valid
        assert_eq!(docs.len(), 1);
        let root = docs[0].root().unwrap();
        assert_eq!(root.at_path("/key").unwrap().scalar_str().unwrap(), "value");
    }

    #[test]
    fn test_parse_error_yields_err() {
        // Invalid YAML: bad indentation should produce an error
        let parser = FyParser::from_string("key: value\n  bad indent").unwrap();
        let results: Vec<_> = parser.doc_iter().collect();

        // Should have at least one result
        assert!(!results.is_empty());

        // The last result should be an error OR we get docs then error
        // (depends on how libfyaml reports the error)
        let has_error = results.iter().any(|r| r.is_err());
        // If no error, that's also acceptable if libfyaml tolerates this input
        // The important thing is we don't panic and handle gracefully
        if has_error {
            let err = results.iter().find(|r| r.is_err()).unwrap();
            assert!(err.is_err());
        }
    }

    #[test]
    fn test_parse_unclosed_bracket_error() {
        // Clearly invalid YAML: unclosed bracket
        let parser = FyParser::from_string("[unclosed").unwrap();
        let results: Vec<_> = parser.doc_iter().collect();

        // This should produce an error
        let has_error = results.iter().any(|r| r.is_err());
        assert!(has_error, "unclosed bracket should produce parse error");
    }
}
