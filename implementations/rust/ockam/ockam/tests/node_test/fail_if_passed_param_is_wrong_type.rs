// This test checks that #[ockam::test] causes a compile time error
// if the function is passed a param that is not of type `ockam_node::Context`

#[ockam::test]
async fn my_test(ctx: std::string::String) -> ockam_core::Result<()> {}
