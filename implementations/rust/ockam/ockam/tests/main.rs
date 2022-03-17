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
    #[cfg(feature = "std")]
    {
        let t = trybuild::TestCases::new();
        t.compile_fail("tests/node/std/fail*.rs");
        t.pass("tests/node/std/pass*.rs");
        // Untested cases:
        //  - Empty body or unused context: this two cases can't be tested because
        //    they would run indefinitely. The unused context variant also includes
        //    tests where no `Context` argument is passed in the input function.
    };
}

#[test]
fn node_test() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/node_test/fail*.rs");
    t.pass("tests/node_test/pass*.rs");
}
