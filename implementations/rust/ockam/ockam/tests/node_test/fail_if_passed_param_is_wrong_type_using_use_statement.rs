// This test checks that #[ockam_test_macros::node_test] causes a compile time error
// if the function is passed a param that is not of type `ockam_node::Context`
// The param is not fully qualified (ie. using `use` statement).

#[ockam_test_macros::node_test]
async fn my_test(ctx: String) -> ockam_core::Result<()> {}
