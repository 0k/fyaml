//! Internal FFI utilities for libfyaml bindings.
//!
//! This module provides shared helper functions for interfacing with
//! libfyaml's C API, ensuring consistent memory allocation and error handling.

use crate::error::{Error, Result};
use libc::{c_char, c_void};
use std::ffi::CStr;

/// Allocates a buffer via libc malloc and copies bytes into it.
///
/// Returns the buffer pointer. On empty input, allocates 1 byte to avoid
/// `malloc(0)` returning null (which is implementation-defined behavior).
///
/// # Safety
///
/// - Caller must ensure the returned buffer is eventually freed with `libc::free`,
///   OR passed to a libfyaml function that takes ownership (e.g.,
///   `fy_document_build_from_malloc_string`, `fy_parser_set_malloc_string`).
/// - If the libfyaml function fails, the caller is responsible for freeing the buffer.
///
/// # Example Usage Pattern
///
/// ```ignore
/// let buf = unsafe { malloc_copy(yaml.as_bytes())? };
/// let ret = unsafe { fy_some_function(buf, yaml.len()) };
/// if ret != 0 {
///     // On failure, libfyaml did NOT take ownership
///     unsafe { libc::free(buf as *mut c_void) };
///     return Err(Error::Ffi("function failed"));
/// }
/// // On success, libfyaml owns the buffer
/// ```
pub(crate) unsafe fn malloc_copy(bytes: &[u8]) -> Result<*mut c_char> {
    // malloc(0) may return null, so always allocate at least 1 byte
    let alloc_len = if bytes.is_empty() { 1 } else { bytes.len() };
    let buf = libc::malloc(alloc_len) as *mut c_char;
    if buf.is_null() {
        return Err(Error::Alloc("malloc failed"));
    }
    if !bytes.is_empty() {
        libc::memcpy(
            buf as *mut c_void,
            bytes.as_ptr() as *const c_void,
            bytes.len(),
        );
    }
    Ok(buf)
}

/// Converts a malloc'd C string to a Rust String and frees the original.
///
/// This is the inverse of `malloc_copy`: it takes a null-terminated C string
/// that was allocated by libfyaml via malloc, converts it to a Rust String,
/// and frees the C memory.
///
/// If the C string contains invalid UTF-8 (rare for YAML), invalid bytes
/// are replaced with the Unicode replacement character (U+FFFD).
///
/// # Safety
///
/// - `ptr` must be a valid pointer to a null-terminated C string
/// - `ptr` must have been allocated by malloc (or equivalent)
/// - Caller transfers ownership - ptr will be freed after conversion
/// - Do NOT use ptr after calling this function
pub(crate) unsafe fn take_c_string(ptr: *mut c_char) -> String {
    let c_str = CStr::from_ptr(ptr);
    let s = c_str.to_string_lossy().into_owned();
    libc::free(ptr as *mut c_void);
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_malloc_copy_normal() {
        unsafe {
            let data = b"hello world";
            let buf = malloc_copy(data).unwrap();
            assert!(!buf.is_null());
            // Verify content
            let slice = std::slice::from_raw_parts(buf as *const u8, data.len());
            assert_eq!(slice, data);
            libc::free(buf as *mut c_void);
        }
    }

    #[test]
    fn test_malloc_copy_empty() {
        unsafe {
            let data = b"";
            let buf = malloc_copy(data).unwrap();
            // Should still allocate (1 byte minimum)
            assert!(!buf.is_null());
            libc::free(buf as *mut c_void);
        }
    }
}
