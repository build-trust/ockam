#[ockam::node(timeout = 100)]
async fn main(c: ockam::Context) {
    c.stop().await.unwrap();
}
