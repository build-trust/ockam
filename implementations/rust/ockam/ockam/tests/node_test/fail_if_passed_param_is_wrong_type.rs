// This test checks that #[ockam_test_macros::node_test] causes a compile time error
// if the function is passed a param that is not of type `ockam_node::Context`

#[ockam_test_macros::node_test]
async fn my_test(ctx: std::string::String) -> ockam_core::Result<()> {}
