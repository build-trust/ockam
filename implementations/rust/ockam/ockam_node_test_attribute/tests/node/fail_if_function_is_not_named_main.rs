#[ockam_node_test_attribute::node]
fn foo(mut c: ockam::Context) {
    c.stop().await.unwrap();
}
