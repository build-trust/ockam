#[test]
fn async_try_clone() {
    let t = trybuild::TestCases::new();
    t.pass("tests/async_try_clone/test.rs");
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
