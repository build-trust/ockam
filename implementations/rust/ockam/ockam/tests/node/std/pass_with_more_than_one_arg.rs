#[ockam::node]
async fn main(mut c: ockam::Context, _x: u64) {
    c.stop().await.unwrap();
}
