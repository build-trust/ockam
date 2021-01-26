#[test]
fn trybuild() {
    let t = trybuild::TestCases::new();
    t.pass("tests/node_attribute/can_be_used_on_main.rs");
    t.compile_fail("tests/node_attribute/fails_if_item_is_not_a_function.rs");
    t.compile_fail("tests/node_attribute/fails_if_function_is_not_async.rs");
}
