#[ockam::node]
async fn main(context: ockam::Context) {
    context.node().stop().unwrap();
}
