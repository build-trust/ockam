#[test]
fn trybuild() {
    let t = trybuild::TestCases::new();
    t.pass("tests/node/pass*.rs");
    t.compile_fail("tests/node/fail*.rs");
    // The node_test tests check that a specific compile error occurs: "the main function doesn't exist",
    // which means that the macro was compiled without errors but it can't be used outside of a test context.
    t.compile_fail("tests/node_test/*.rs");
}
