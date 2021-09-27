#[ockam_node_test_attribute::node]
async fn foo(mut c: ockam::Context) {
    c.stop().await.unwrap();
}
