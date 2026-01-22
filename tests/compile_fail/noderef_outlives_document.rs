//! Test: NodeRef cannot outlive Document
//!
//! This should fail to compile because the NodeRef's lifetime 'doc
//! is tied to the Document, preventing use-after-free.

use fyaml::Document;

fn main() {
    let node_ref;
    {
        let doc = Document::parse_str("key: value").unwrap();
        let root = doc.root().unwrap();
        node_ref = root.at_path("/key").unwrap();
        // doc is dropped here
    }
    // This line would use a dangling reference - should not compile!
    let _ = node_ref.scalar_str();
}
