// This test checks that #[ockam_node_test_attribute::node_test] causes a compile time error
// if the item it is defined on is not an async function.

#[ockam_node_test_attribute::node_test]
fn my_test(c: &mut ockam::Context) -> ockam::Result<()> {
    c.stop().await.unwrap();
}
