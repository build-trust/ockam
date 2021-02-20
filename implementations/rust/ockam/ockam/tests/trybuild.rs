#[test]
fn trybuild() {
    let t = trybuild::TestCases::new();
    t.pass("tests/node_attribute/can_be_used_on_main.rs");
    t.pass("tests/node_attribute/can_be_used_on_any_fn.rs");
    t.pass("tests/node_attribute/can_be_used_on_any_fn_ockam_use_as_o.rs");
    t.compile_fail("tests/node_attribute/fails_if_item_is_not_a_function.rs");
    t.compile_fail("tests/node_attribute/fails_if_function_is_not_async.rs");
    t.compile_fail("tests/node_attribute/fails_if_passed_param_is_self.rs");
    t.compile_fail("tests/node_attribute/fails_if_passed_param_is_wrong_type.rs");
    t.compile_fail(
        "tests/node_attribute/fails_if_passed_param_is_wrong_type_using_use_statement.rs",
    );
    t.compile_fail("tests/node_attribute/fails_if_unused_context.rs");
    t.compile_fail("tests/node_attribute/fails_if_unused_context_empty_fnbody.rs");
    t.compile_fail("tests/node_attribute/fails_if_more_than_one_arg.rs");
}
