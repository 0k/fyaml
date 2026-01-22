//! Test: Editor blocks NodeRef access
//!
//! This should fail to compile because doc.edit() takes &mut doc,
//! which prevents any existing NodeRef from being used.

use fyaml::Document;

fn main() {
    let mut doc = Document::parse_str("key: value").unwrap();
    let root = doc.root().unwrap(); // immutable borrow of doc

    // This should fail: edit() needs &mut doc, but root holds &doc
    let _ed = doc.edit();

    // Even accessing root should fail
    let _ = root.scalar_str();
}
