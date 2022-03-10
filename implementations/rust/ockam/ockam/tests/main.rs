#[test]
fn async_try_clone() {
    let t = trybuild::TestCases::new();
    t.pass("tests/async_try_clone/test.rs");
}

#[test]
fn message_derive() {
    let t = trybuild::TestCases::new();
    t.pass("tests/message/test.rs");
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
    t.pass("tests/node_test/pass*.rs");
}
