#[ockam::node]
fn main(mut ctx: ockam::Context) {
    ctx.stop().await.unwrap();
}
