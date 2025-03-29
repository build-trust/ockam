#[ockam::node]
fn foo(c: ockam::Context) {
    c.shutdown_node().await.unwrap();
}
