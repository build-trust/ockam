#[ockam_node_test_attribute::node(timeout = 100)]
async fn main(mut c: ockam::Context) {
    c.stop().await.unwrap();
}
