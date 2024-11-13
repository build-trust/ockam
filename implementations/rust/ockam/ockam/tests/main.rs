#[test]
fn try_clone() {
    let t = trybuild::TestCases::new();
    // see the other use of `NIGHTLY_CI` for explanation.
    if std::env::var_os("NIGHTLY_CI").is_none() {
        t.compile_fail("tests/try_clone/fail*.rs");
    }
    t.pass("tests/try_clone/pass.rs");
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
        // The compile_fail tests include exact compiler error output. This can
        // differ between rust versions (for example, nightly rustc could have
        // improvements to error reporting which have not yet made it to stable).
        //
        // In this case, we'd have no way for both stable and nightly to have
        // passing CI. There are a few ways this could be approached, but the one we
        // take is just to only run the compile_fail trybuild tests against one Rust
        // version in CI.
        //
        // This means we must pick between stable and nightly for these tests. While
        // I don't think we have reason to believe the choice makes much of a
        // difference in practice, we choose stable:
        // - There are more users who use the stable version of Rust than on a
        //   nightly build, so we care more what the error messages look like on
        //   that version.
        // - Stable is updated less frequently, and is hopefully aptly named, so
        //   this should let us avoid churn in cases where the `rustc` developers
        //   are iterating on some aspect of error messaging.
        // - Choosing stable avoids us needing contributors to install nightly rust
        //   if they need to rebuild the trybuild output files, which would be an
        //   unnecessarily contribution roadblock.

        if std::env::var_os("NIGHTLY_CI").is_none() {
            t.compile_fail("tests/node/std/fail*.rs");
        }
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
    // see the other use of `NIGHTLY_CI` for explanation.
    if std::env::var_os("NIGHTLY_CI").is_none() {
        t.compile_fail("tests/node_test/fail*.rs");
    }
    t.pass("tests/node_test/pass*.rs");
}
