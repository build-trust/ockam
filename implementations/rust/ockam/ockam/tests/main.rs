#[test]
fn async_try_clone() {
    let t = trybuild::TestCases::new();
    t.pass("tests/async_try_clone/test.rs");
}

#[test]
fn message_derive() {
    let t = trybuild::TestCases::new();
    t.pass("tests/message_derive/test.rs");
}

#[test]
fn node() {
    #[cfg(not(feature = "no_main"))]
    {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/node/fail*.rs");
        t.pass("tests/node/pass*.rs");
    };
}

#[test]
fn node_test() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/node_test/fail*.rs");
    // When the node_test macro is compiled successfully, it will return the error "the main function doesn't exist" because this macro
    // is only valid within a test context. Therefore, the "pass" tests is considered valid only if it returns that specific error.
    t.compile_fail("tests/node_test/pass*.rs");
}
