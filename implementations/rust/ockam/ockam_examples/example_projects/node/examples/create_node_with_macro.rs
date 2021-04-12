#[ockam::node]
async fn main(mut context: ockam::Context) {
    context.stop().await.unwrap();
}
