#[test]
fn trybuild() {
    let t = trybuild::TestCases::new();
    t.pass("tests/node/pass*.rs");
    t.compile_fail("tests/node/fail*.rs");
    t.compile_fail("tests/node_test/*.rs");
}
