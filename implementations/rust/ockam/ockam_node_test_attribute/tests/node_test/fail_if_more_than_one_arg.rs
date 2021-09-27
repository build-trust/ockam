// Test case to verify that only one argument is passed.

#[ockam_node_test_attribute::node_test]
async fn my_test(mut c: ockam::Context, _x: u64) {
    c.stop().await.unwrap();
}
