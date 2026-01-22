//! Parser and emitter configuration utilities.
//!
//! This module centralizes the construction of libfyaml configuration structures.

use fyaml_sys::*;
use std::ptr;

/// Creates a parse configuration for single-document parsing.
///
/// Enables:
/// - `FYPCF_PARSE_COMMENTS`: Preserve comments for roundtrip
#[inline]
pub fn document_parse_cfg() -> fy_parse_cfg {
    fy_parse_cfg {
        search_path: ptr::null_mut(),
        userdata: ptr::null_mut(),
        diag: ptr::null_mut(),
        flags: FYPCF_PARSE_COMMENTS,
    }
}

/// Creates a parse configuration for stream/multi-document parsing.
///
/// Enables:
/// - `FYPCF_DISABLE_BUFFERING`: Don't buffer input
/// - `FYPCF_QUIET`: Suppress diagnostic output
/// - `FYPCF_RESOLVE_DOCUMENT`: Resolve document after parsing
/// - `FYPCF_PARSE_COMMENTS`: Preserve comments for roundtrip
#[inline]
pub fn stream_parse_cfg() -> fy_parse_cfg {
    fy_parse_cfg {
        search_path: ptr::null_mut(),
        userdata: ptr::null_mut(),
        diag: ptr::null_mut(),
        flags: FYPCF_DISABLE_BUFFERING
            | FYPCF_QUIET
            | FYPCF_RESOLVE_DOCUMENT
            | FYPCF_PARSE_COMMENTS,
    }
}

/// Returns emitter flags that preserve original formatting and comments.
#[inline]
pub fn emit_flags() -> u32 {
    FYECF_MODE_ORIGINAL | FYECF_OUTPUT_COMMENTS
}
