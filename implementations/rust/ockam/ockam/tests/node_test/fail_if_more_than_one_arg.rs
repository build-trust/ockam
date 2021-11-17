// Test case to verify that only one argument is passed.

#[ockam_test_macros::node_test]
async fn my_test(mut c: ockam_node::Context, _x: u64) -> ockam_core::Result<()> {
    c.stop().await.unwrap();
}
