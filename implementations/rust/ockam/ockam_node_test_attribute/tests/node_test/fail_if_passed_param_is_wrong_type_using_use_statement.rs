// This test checks that #[ockam_node_test_attribute::node_test] causes a compile time error
// if the function is passed a param that is not of type `ockam::Context`
// The param is not fully qualified (ie. using `use` statement).

#[ockam_node_test_attribute::node_test]
async fn my_test(ctx: String) -> ockam::Result<()> {}
