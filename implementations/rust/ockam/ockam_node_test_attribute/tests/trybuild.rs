#[test]
fn trybuild() {
    let t = trybuild::TestCases::new();
    #[cfg(feature = "std")]
    {
        // `node` macro tests
        t.compile_fail("tests/node/fail*.rs");
        t.pass("tests/node/pass*.rs");

        // `node_test` macro tests
        t.compile_fail("tests/node_test/fail*.rs");
        // When the node_test macro is compiled successfully, it will return the error "the main function doesn't exist" because this macro
        // is only valid within a test context. Therefore, the "pass" tests is considered valid if it returns only that error.
        t.compile_fail("tests/node_test/pass*.rs");
    };
}
