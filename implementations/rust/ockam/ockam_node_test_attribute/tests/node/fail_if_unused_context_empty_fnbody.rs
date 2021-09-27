// This test checks that #[ockam_node_test_attribute::node] causes a compile time error
// if the function is passed a parameter of type `ockam::Context` but is unused.

#[ockam_node_test_attribute::node]
async fn main(_ctx: ockam::Context) {}
