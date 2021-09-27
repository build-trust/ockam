// This test checks that #[ockam_node_test_attribute::node] causes a compile time error
// if the function is passed a param that is not of type `ockam::Context`

#[ockam_node_test_attribute::node]
async fn main(ctx: std::string::String) {}
