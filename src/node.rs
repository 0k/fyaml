//! YAML node type definitions.
//!
//! This module provides the core type enums shared across the API.

use fyaml_sys::*;

/// The type of a YAML node.
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum NodeType {
    /// A scalar value (string, number, boolean, null).
    Scalar,
    /// A sequence (list/array) of nodes.
    Sequence,
    /// A mapping (dictionary/object) of key-value pairs.
    Mapping,
}

/// The style of a YAML node (how it was/should be represented).
#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum NodeStyle {
    /// No specific style hint, let the emitter decide.
    Any,
    /// Flow style (inline) for sequences/mappings.
    Flow,
    /// Block style (indented) for sequences/mappings.
    Block,
    /// Plain scalar (no quotes).
    Plain,
    /// Single-quoted scalar.
    SingleQuoted,
    /// Double-quoted scalar.
    DoubleQuoted,
    /// Literal block scalar (|).
    Literal,
    /// Folded block scalar (>).
    Folded,
    /// Alias reference.
    Alias,
}

impl From<i32> for NodeStyle {
    fn from(value: i32) -> Self {
        match value {
            x if x == FYNS_ANY => NodeStyle::Any,
            x if x == FYNS_FLOW => NodeStyle::Flow,
            x if x == FYNS_BLOCK => NodeStyle::Block,
            x if x == FYNS_PLAIN => NodeStyle::Plain,
            x if x == FYNS_SINGLE_QUOTED => NodeStyle::SingleQuoted,
            x if x == FYNS_DOUBLE_QUOTED => NodeStyle::DoubleQuoted,
            x if x == FYNS_LITERAL => NodeStyle::Literal,
            x if x == FYNS_FOLDED => NodeStyle::Folded,
            x if x == FYNS_ALIAS => NodeStyle::Alias,
            _ => NodeStyle::Any,
        }
    }
}

impl From<u32> for NodeType {
    fn from(value: u32) -> Self {
        match value {
            x if x == FYNT_SCALAR => NodeType::Scalar,
            x if x == FYNT_SEQUENCE => NodeType::Sequence,
            x if x == FYNT_MAPPING => NodeType::Mapping,
            _ => {
                log::warn!(
                    "Unknown fy_node_type value: {}, defaulting to Scalar",
                    value
                );
                NodeType::Scalar
            }
        }
    }
}
