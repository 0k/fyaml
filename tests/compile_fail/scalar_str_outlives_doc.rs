//! Test: Scalar string slice cannot outlive Document
//!
//! This should fail to compile because the returned &str has lifetime 'doc,
//! tied to the Document, preventing use-after-free of the string data.

use fyaml::Document;

fn main() {
    let scalar_str;
    {
        let doc = Document::parse_str("key: value").unwrap();
        let node = doc.at_path("/key").unwrap();
        scalar_str = node.scalar_str().unwrap();
        // doc is dropped here
    }
    // This line would use a dangling reference - should not compile!
    println!("{}", scalar_str);
}
