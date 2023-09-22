#[ockam::node]
async fn main(c: ockam::Context, _x: u64) {
    c.stop().await.unwrap();
}
