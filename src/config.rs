//! Parser and emitter configuration utilities.
//!
//! This module centralizes the construction of libfyaml configuration structures.

use fyaml_sys::*;
use std::ptr;

/// Creates a parse configuration for single-document parsing with diagnostic capture.
///
/// Enables:
/// - `FYPCF_QUIET`: Suppress stderr output
/// - `FYPCF_PARSE_COMMENTS`: Preserve comments for roundtrip
///
/// The diag pointer allows capturing parse errors with location information.
#[inline]
pub fn document_parse_cfg_with_diag(diag: *mut fy_diag) -> fy_parse_cfg {
    fy_parse_cfg {
        search_path: ptr::null_mut(),
        userdata: ptr::null_mut(),
        diag,
        flags: FYPCF_QUIET | FYPCF_PARSE_COMMENTS,
    }
}

/// Creates a parse configuration for stream/multi-document parsing with diagnostic capture.
///
/// Enables:
/// - `FYPCF_QUIET`: Suppress stderr output (always enabled for no-stderr guarantee)
/// - `FYPCF_DISABLE_BUFFERING`: Don't buffer input
/// - `FYPCF_RESOLVE_DOCUMENT`: Resolve document after parsing
/// - `FYPCF_PARSE_COMMENTS`: Preserve comments for roundtrip
///
/// The diag pointer allows capturing parse errors with location information.
/// FYPCF_QUIET is always enabled to guarantee no stderr output, regardless of
/// whether a custom diag is provided.
#[inline]
pub fn stream_parse_cfg_with_diag(diag: *mut fy_diag) -> fy_parse_cfg {
    fy_parse_cfg {
        search_path: ptr::null_mut(),
        userdata: ptr::null_mut(),
        diag,
        flags: FYPCF_QUIET
            | FYPCF_DISABLE_BUFFERING
            | FYPCF_RESOLVE_DOCUMENT
            | FYPCF_PARSE_COMMENTS,
    }
}

/// Returns emitter flags that preserve original formatting and comments.
#[inline]
pub fn emit_flags() -> u32 {
    FYECF_MODE_ORIGINAL | FYECF_OUTPUT_COMMENTS
}
