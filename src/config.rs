//! Parser and emitter configuration utilities.
//!
//! This module centralizes the construction of libfyaml configuration structures.

use fyaml_sys::*;
use std::ptr;

/// Creates a parse configuration for single-document parsing with diagnostic capture.
///
/// Enables:
/// - `FYPCF_QUIET`: Suppress stderr output
/// - `FYPCF_KEEP_COMMENTS`: Preserve comments for roundtrip
///
/// The diag pointer allows capturing parse errors with location information.
#[inline]
pub fn document_parse_cfg_with_diag(diag: *mut fy_diag) -> fy_parse_cfg {
    fy_parse_cfg {
        search_path: ptr::null_mut(),
        userdata: ptr::null_mut(),
        diag,
        flags: FYPCF_QUIET | FYPCF_KEEP_COMMENTS,
    }
}

/// Creates a parse configuration for stream/multi-document parsing with diagnostic capture.
///
/// Enables:
/// - `FYPCF_QUIET`: Suppress stderr output (always enabled for no-stderr guarantee)
/// - `FYPCF_DISABLE_BUFFERING`: Don't buffer input
/// - `FYPCF_RESOLVE_DOCUMENT`: Resolve document after parsing
/// - `FYPCF_KEEP_COMMENTS`: Preserve comments for roundtrip
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
        flags: FYPCF_QUIET | FYPCF_DISABLE_BUFFERING | FYPCF_RESOLVE_DOCUMENT | FYPCF_KEEP_COMMENTS,
    }
}

/// Returns emitter flags that preserve original formatting and comments.
///
/// # Upstream bug workaround: `FYECF_WIDTH_INF`
///
/// libfyaml's emitter wraps long lines at the configured width (default
/// 80) by inserting a trailing `\` line-continuation. That escape is
/// only valid in double-quoted style: in single-quoted (and plain)
/// scalars the `\` is a literal character, so the emitted YAML no
/// longer round-trips (re-parsing yields a spurious `\` + folded
/// space in the value).
///
/// Reproduction (fy-tool, both v1.0.0-alpha7 and master @ ac6c0fc):
/// ```text
/// $ python3 -c "print(\"key: '\" + 'x'*81 + \"'\")" | fy-tool --dump --width=80 -
/// key: 'xxx...xxx\
///   xxxxxxx'        <- re-parses as "...xxx\ xxxxxxx" (corrupted)
/// ```
///
/// We force infinite width, the same workaround fy-tool itself applies
/// when stdout is not a tty (see `src/tool/fy-tool.c`: "if we're
/// dumping to a non tty stdout width is infinite").
///
/// TODO(upstream): re-test against new libfyaml releases
/// (<https://github.com/pantoniou/libfyaml>) with the reproduction
/// above; once finite-width wrapping of single-quoted scalars
/// round-trips correctly, `FYECF_WIDTH_INF` can be dropped. The
/// regression test `emit_long_single_quoted_scalar_round_trips` in
/// `tests/integration.rs` guards this.
#[inline]
pub fn emit_flags() -> u32 {
    FYECF_MODE_ORIGINAL | FYECF_OUTPUT_COMMENTS | FYECF_WIDTH_INF
}
