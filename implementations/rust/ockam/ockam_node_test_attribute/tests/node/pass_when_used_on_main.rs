// This test checks that an attribute macro #[ockam_node_test_attribute::node] exists
// and can be used with an async main function

#[ockam_node_test_attribute::node]
async fn main(mut context: ockam::Context) {
    context.stop().await.unwrap();
}
