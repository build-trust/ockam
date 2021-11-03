// This test checks that #[ockam_node_test_attribute::node_test] causes a compile time error
// if the function is passed a `self` param (thus making it a
// `Receiver` function.

#[ockam_node_test_attribute::node_test]
async fn my_test(self) -> ockam::Result<()> {}
