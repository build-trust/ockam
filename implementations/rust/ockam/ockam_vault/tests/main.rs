#[test]
fn vault_test() {
    let t = trybuild::TestCases::new();
    t.pass("tests/vault_test/pass*.rs");
}
