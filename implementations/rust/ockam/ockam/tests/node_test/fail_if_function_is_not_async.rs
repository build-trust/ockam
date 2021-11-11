// This test checks that #[ockam_test_macros::node_test] causes a compile time error
// if the item it is defined on is not an async function.

#[ockam_test_macros::node_test]
fn my_test(c: &mut ockam_node::Context) -> ockam_core::Result<()> {
    c.stop().await.unwrap();
}
