// This test checks that #[ockam_node_test_attribute::node_test] causes a compile time error
// if the function is passed a param that is not of type `ockam::Context`

#[ockam_node_test_attribute::node_test]
async fn my_test(ctx: std::string::String) -> ockam::Result<()> {}
