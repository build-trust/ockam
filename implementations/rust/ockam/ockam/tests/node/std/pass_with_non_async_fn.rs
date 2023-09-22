#[ockam::node]
fn main(ctx: ockam::Context) {
    ctx.stop().await.unwrap();
}
