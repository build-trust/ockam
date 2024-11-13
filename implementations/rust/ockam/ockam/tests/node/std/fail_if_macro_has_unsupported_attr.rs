#[ockam::node(timeout = 100)]
async fn main(c: ockam::Context) {
    c.shutdown_node().await.unwrap();
}
