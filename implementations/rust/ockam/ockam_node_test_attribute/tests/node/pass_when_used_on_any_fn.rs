#[ockam_node_test_attribute::node]
async fn main(mut c: ockam::Context) {
    c.stop().await.unwrap();
}
