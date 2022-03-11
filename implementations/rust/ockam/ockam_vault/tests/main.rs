#[test]
fn vault_test() {
    let t = trybuild::TestCases::new();
    t.pass("tests/vault_test/pass*.rs");
}

#[test]
fn vault_test_sync() {
    let t = trybuild::TestCases::new();
    t.pass("tests/vault_test_sync/pass*.rs");
}
