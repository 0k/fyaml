//! Compile-fail tests to verify borrow checker safety.
//!
//! These tests use trybuild to verify that code which would be unsafe
//! is rejected at compile time.

#[test]
fn compile_fail_tests() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/compile_fail/*.rs");
}
