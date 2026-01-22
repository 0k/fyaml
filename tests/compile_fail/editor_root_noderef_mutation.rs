//! Test: NodeRef from Editor::root() cannot survive across mutation
//!
//! This should fail to compile because the NodeRef from ed.root() borrows &ed,
//! which prevents calling mutation methods like ed.set_yaml_at().

use fyaml::Document;

fn main() {
    let mut doc = Document::parse_str("key: value").unwrap();
    let mut ed = doc.edit();

    // Get a NodeRef through the editor
    let root = ed.root().unwrap();

    // This should fail: set_yaml_at needs &mut ed, but root borrows &ed
    ed.set_yaml_at("/key", "'new_value'").unwrap();

    // Even accessing root should fail
    let _ = root.scalar_str();
}
