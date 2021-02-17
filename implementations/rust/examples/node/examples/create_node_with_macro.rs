#[ockam::node]
async fn main(context: ockam::Context) {
    context.stop().unwrap();
}
