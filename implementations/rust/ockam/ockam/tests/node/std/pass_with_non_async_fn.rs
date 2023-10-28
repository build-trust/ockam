#[ockam::node]
fn main(ctx: ockam::Context) -> ockam_core::Result<()> {
    ctx.stop().await
}
